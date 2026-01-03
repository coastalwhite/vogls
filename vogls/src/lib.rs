use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use slotmap::SlotMap;
use vogls_ir::{Bits, ContextFormat, GlobalContext, Instruction, Signal};
use vogls_sim::{
    Context, EvaluationEvent, Event, Regions, SignalInfo, TracingLevel, VmProcess, VmProcessKey,
    lower_process_to_vm,
};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{
    CaseGenerateConstruct, CaseGenerateItem, GenerateBlock, IfGenerateConstruct,
    LoopGenerateConstruct, Module, ModuleItem, ModuleOrGenerateItem, NonPortModuleItem,
};
use vogls_verilog::hierarchy::{Hierarchy, ModuleBuilder};
use vogls_verilog::lower::{
    Diagnostics as LowerDiagnostics, ModuleArgs, ModuleContext, ModuleInitialization, ModuleQuery,
    fetch_module_interface, lower_module_to_ir,
};
use vogls_verilog::parser::{
    AstArenas, Diagnostics as ParserDiagnostics, ParseContext, ParserScratches, TokenWalker,
    parse_file, report, report_error,
};
use vogls_verilog::tokenizer::Tokenized;

pub struct ExecutionContext {
    pub stdout: Box<dyn std::io::Write>,
    pub stderr: Box<dyn std::io::Write>,
    pub emit_hierarchy: bool,
    pub emit_unoptimized_ir: bool,
    pub emit_ir: bool,
    pub emit_vm: bool,
    pub trace: TracingLevel,
    pub time: u64,
    pub opt_rounds: u8,
}

fn append_referenced_modules_generate_block<'a>(
    arenas: &'a AstArenas,
    generate_block: AstId<GenerateBlock>,
    referenced: &mut HashSet<&'a str>,
) {
    match arenas.get(generate_block) {
        GenerateBlock::ModuleOrGenerateItem(id) => {
            append_referenced_modules(arenas, *id, referenced)
        }
        GenerateBlock::BeginEnd(_, ids) => {
            for id in ids.iter() {
                append_referenced_modules(arenas, id, referenced);
            }
        }
    }
}

fn append_referenced_modules_opt_generate_block<'a>(
    arenas: &'a AstArenas,
    generate_block: AstId<Option<GenerateBlock>>,
    referenced: &mut HashSet<&'a str>,
) {
    match arenas.get(generate_block) {
        None => {}
        Some(GenerateBlock::ModuleOrGenerateItem(id)) => {
            append_referenced_modules(arenas, *id, referenced)
        }
        Some(GenerateBlock::BeginEnd(_, ids)) => {
            for id in ids.iter() {
                append_referenced_modules(arenas, id, referenced);
            }
        }
    }
}

fn append_referenced_modules<'a>(
    arenas: &'a AstArenas,
    module_or_generate_item: AstId<ModuleOrGenerateItem>,
    referenced: &mut HashSet<&'a str>,
) {
    match arenas.get(module_or_generate_item) {
        ModuleOrGenerateItem::ModuleInstantiation(module_instantiation) => {
            let module_instantiation = arenas.get(*module_instantiation);
            let module_name = arenas.get_ident(module_instantiation.module_identifier.item.0);
            referenced.insert(module_name);
        }
        ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(_) => {}
        ModuleOrGenerateItem::LocalParameterDeclaration(_) => {}
        ModuleOrGenerateItem::ParameterOverride => {}
        ModuleOrGenerateItem::ContinuousAssign(_) => {}
        ModuleOrGenerateItem::GateInstantiation(_) => {}
        ModuleOrGenerateItem::UdpInstantiation => {}
        ModuleOrGenerateItem::InitialConstruct(_) => {}
        ModuleOrGenerateItem::AlwaysConstruct(_) => {}
        ModuleOrGenerateItem::LoopGenerateConstruct(loop_generate_construct) => {
            let LoopGenerateConstruct {
                initialization: _,
                condition: _,
                iteration: _,
                block,
            } = arenas.get(*loop_generate_construct);
            append_referenced_modules_generate_block(arenas, *block, referenced);
        }
        ModuleOrGenerateItem::IfGenerateConstruct(if_generate_construct) => {
            let IfGenerateConstruct {
                condition: _,
                truthy,
                falsy,
            } = arenas.get(*if_generate_construct);
            append_referenced_modules_opt_generate_block(arenas, *truthy, referenced);
            if let Some(falsy) = falsy {
                append_referenced_modules_opt_generate_block(arenas, *falsy, referenced);
            }
        }
        ModuleOrGenerateItem::CaseGenerateConstruct(case_generate_construct) => {
            let CaseGenerateConstruct { value: _, items } = arenas.get(*case_generate_construct);
            for item in items.iter() {
                let CaseGenerateItem { pattern: _, block } = arenas.get(item);
                append_referenced_modules_opt_generate_block(arenas, *block, referenced);
            }
        }
    }
}

pub fn run(
    path: &Path,
    top_level_module: Option<&str>,
    ectx: &mut ExecutionContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let content: Rc<str> = std::fs::read_to_string(&path)?.into();
    let token_buffer = Tokenized::tokenize(content.clone(), Some(path.into()));
    let mut tkw = TokenWalker::new(&token_buffer);
    let mut diagnostics = ParserDiagnostics::default();

    let ast = match parse_file(
        &mut tkw,
        &mut ParserScratches::default(),
        Some(&mut diagnostics),
        &mut ParseContext::default(),
    ) {
        Ok(ast) => ast,
        Err(_) => {
            for (location, err) in &diagnostics.errors {
                let mut out = String::new();
                report_error(&token_buffer, err.clone(), *location, &mut out)?;
                write!(ectx.stderr, "{out}")?;
            }
            return Err("failed to parse".into());
        }
    };
    let mut gl = GlobalContext::default();

    let module_lut =
        HashMap::<&str, usize>::from_iter(ast.modules.iter().enumerate().map(|(i, module_id)| {
            let module = ast.arenas.get(module_id);
            (ast.arenas.get_ident(module.module_identifier.item.0), i)
        }));
    let module_to_ast_lut =
        HashMap::<&str, AstId<Module>>::from_iter(ast.modules.iter().map(|module_id| {
            let module = ast.arenas.get(module_id);
            (
                ast.arenas.get_ident(module.module_identifier.item.0),
                module_id,
            )
        }));

    let tl_module_name = match top_level_module {
        Some(v) => v,
        None => {
            let mut referenced = HashSet::new();
            for module_id in ast.modules {
                let Module {
                    attribute_instances: _,
                    module_identifier: _,
                    module_parameter_port_list: _,
                    module_items,
                    ports: _,
                } = ast.arenas.get(module_id);
                for module_item in module_items.iter() {
                    let ModuleItem::NonPortModuleItem(p) = ast.arenas.get(module_item) else {
                        continue;
                    };

                    if let NonPortModuleItem::ModuleOrGenerateItem(module_item) = ast.arenas.get(*p)
                    {
                        append_referenced_modules(&ast.arenas, *module_item, &mut referenced);
                    }
                }
            }

            let mut top_level_modules = Vec::new();
            for module_id in ast.modules {
                let Module {
                    attribute_instances: _,
                    module_identifier,
                    module_parameter_port_list: _,
                    module_items: _,
                    ports: _,
                } = ast.arenas.get(module_id);
                let module_name = ast.arenas.get_ident(module_identifier.item.0);
                if referenced.contains(module_name) {
                    continue;
                }
                top_level_modules.push((module_id, module_name));
            }

            if top_level_modules.len() == 0 {
                return Err(<Box<dyn std::error::Error>>::from(
                    "no top-level module found".to_string(),
                ));
            } else if top_level_modules.len() > 1 {
                let names = top_level_modules
                    .iter()
                    .map(|(_, n)| *n)
                    .collect::<Vec<&str>>();
                writeln!(
                    ectx.stderr,
                    "[ERR]: Found {} possible top-level modules: {names:?}",
                    top_level_modules.len()
                )?;
                let mut out = String::new();
                for (m, _) in top_level_modules {
                    out.clear();
                    let m = ast.arenas.get(m);
                    let span = ast.arenas.get_item_span(m.module_identifier);
                    report(&token_buffer, span, &mut out)?;
                    writeln!(ectx.stderr, "{out}").unwrap();
                }
                return Err("ambiguous top-level module".into());
            } else {
                top_level_modules[0].1
            }
        }
    };
    let Some(tl_module) = module_lut.get(tl_module_name) else {
        return Err(<Box<dyn std::error::Error>>::from(
            "cannot find top-level module".to_string(),
        ));
    };

    // Walk the modules in depth-first order and lower to IR.
    let mut error = false;
    let mut diagnostics = LowerDiagnostics::default();
    let mut next_modules = Vec::<ModuleInitialization>::new();
    let Ok((top_level_params, top_level_io, parameters)) = fetch_module_interface(
        &mut gl,
        &ast.arenas,
        ast.modules.get(*tl_module),
        &[],
        &mut diagnostics,
    ) else {
        for (location, warning) in &diagnostics.warnings {
            writeln!(ectx.stderr, "[WARN]: {warning}")?;
            let mut out = String::new();
            report(&token_buffer, *location, &mut out)?;
            writeln!(ectx.stderr, "{out}")?;
        }

        for (location, err, context) in &diagnostics.errors {
            let mut out = String::new();
            report_error(&token_buffer, err.clone(), *location, &mut out)?;
            write!(ectx.stderr, "{out}")?;
            if !context.is_empty() {
                writeln!(ectx.stderr, "context:")?;
                for c in context {
                    writeln!(ectx.stderr, "- {c}")?;
                }
            }
            writeln!(ectx.stderr)?;
        }
        return Err("top_level fetch_module error".into());
    };
    assert!(top_level_io.ports.is_empty());
    let mut hierarchy = Hierarchy::new(tl_module_name.to_string());
    let top_level_key = hierarchy.top_level_key();
    next_modules.push(ModuleInitialization {
        name: tl_module_name,
        parameters: top_level_params,
        io: top_level_io,
        args: ModuleArgs {
            parameters,
            signals: Vec::new(),
        },
        hierarchy_key: top_level_key,
    });
    let mut mc = ModuleContext {
        named_lookup: module_to_ast_lut,
        module_builder: ModuleBuilder::new(&mut hierarchy, top_level_key),
        next_modules,
        queries_to_resolve: Vec::new(),
    };
    while let Some(init) = mc.next_modules.pop() {
        let ModuleInitialization {
            name,
            parameters,
            io,
            args,
            hierarchy_key,
        } = &init;
        mc.module_builder.move_to(*hierarchy_key);
        let module_id = ast.modules.get(module_lut[name]);
        let module_key = lower_module_to_ir(
            &mut gl,
            &ast.arenas,
            module_id,
            &parameters,
            &io,
            &args,
            &mut mc,
            &mut diagnostics,
        );
        error |= module_key.is_err();
    }

    if !diagnostics.warnings.is_empty() {
        for (location, warning) in &diagnostics.warnings {
            writeln!(ectx.stderr, "[WARN]: {warning}")?;
            let mut out = String::new();
            report(&token_buffer, *location, &mut out)?;
            writeln!(ectx.stderr, "{out}")?;
        }
    }

    if error {
        for (location, err, context) in &diagnostics.errors {
            let mut out = String::new();
            report_error(&token_buffer, err.clone(), *location, &mut out)?;
            write!(ectx.stderr, "{out}")?;
            if !context.is_empty() {
                writeln!(ectx.stderr, "context:")?;
                for c in context {
                    writeln!(ectx.stderr, "- {c}")?;
                }
            }
            writeln!(ectx.stderr)?;
        }
        return Err("failed to lower".into());
    }

    for query in mc.queries_to_resolve {
        let ModuleQuery {
            bb,
            instruction,
            query,
        } = query;

        assert!(query.is_none());
        let scope = hierarchy.vcd_scope(u32::MAX);
        let i = &mut gl.bbs[bb].instrs[instruction];
        let dst = i.get_destination_variable().unwrap();
        *i = Instruction::Intrinsic(
            dst,
            Box::new(vogls_ir::IntrinsicOp::VcdAppendModule(scope)),
            [].into(),
        )
    }
    if ectx.emit_hierarchy {
        writeln!(
            ectx.stdout,
            "{}",
            hierarchy.display(hierarchy.top_level_key())
        )?;
    }

    if ectx.emit_unoptimized_ir {
        for process in gl.processes.values() {
            writeln!(ectx.stdout, "{}", process.display(&gl))?;
        }
    }

    let mut scratch_stack = Vec::new();
    let mut scratch_mfr = Vec::new();
    let mut scratch_bb_to_bb_map = HashMap::new();
    let mut scratch_bb_to_u32_map = HashMap::new();
    let mut scratch_bb_to_bits_map = HashMap::new();
    let mut scratch_var_to_var_map = HashMap::new();
    let mut scratch_var_seen = HashSet::new();
    let mut scratch_seen = HashSet::new();
    let mut scratch_removed = HashSet::new();
    for process in gl.processes.values_mut() {
        for _ in 0..ectx.opt_rounds {
            process.entry = vogls_ir::optimize::remove_needless_jumps(
                &mut gl.bbs,
                process.entry,
                &mut scratch_stack,
                &mut scratch_bb_to_bb_map,
                &mut scratch_bb_to_u32_map,
                &mut scratch_seen,
            );
            vogls_ir::optimize::remove_needles_branches(
                &mut gl.bbs,
                process.entry,
                &mut scratch_stack,
                &mut scratch_seen,
            );
            vogls_ir::optimize::propagate_constants(
                &mut gl.bbs,
                &gl.vars,
                process.entry,
                &mut scratch_stack,
                &mut scratch_mfr,
                &mut scratch_seen,
                &mut scratch_removed,
                &mut scratch_bb_to_bits_map,
                &mut scratch_var_to_var_map,
            );
            vogls_ir::optimize::deadcode_elimination(
                &mut gl.bbs,
                &mut gl.vars,
                process.entry,
                &mut scratch_stack,
                &mut scratch_seen,
                &mut scratch_var_seen,
            );
        }
    }

    if ectx.emit_ir && !ectx.emit_vm {
        for process in gl.processes.values() {
            writeln!(ectx.stdout, "{}", process.display(&gl))?;
        }
    }

    let mut processes = SlotMap::<VmProcessKey, VmProcess>::default();
    let mut regions = Regions::new(3); // inactive, non-blocking, monitor
    let mut signals = HashMap::default();
    let mut listeners = SlotMap::default();
    let mut watches = HashMap::default();

    // // Find the entity for the Top-Level Module.
    // let mut elab_processes = Vec::new();
    // elaborate::elaborate(tl_module_key, &mut gl, &mut elab_processes);

    let mut io_signals = HashMap::new();
    let mut stack_top = 0usize;
    for process in gl.processes.keys() {
        if ectx.emit_vm && ectx.emit_ir {
            println!();
            println!("{}", gl.processes[process].display(&gl));
        }
        let vm_process = lower_process_to_vm(process, &gl, &mut stack_top, &mut io_signals);

        if ectx.emit_vm {
            print!("{}", &vm_process);
        }

        let vm_process_key = processes.insert(vm_process);

        if ectx.emit_vm {
            println!(": {vm_process_key:?}");
        }

        regions.active.push(Event::Evaluation(EvaluationEvent {
            process: vm_process_key,
            ip: 0,
        }));
    }
    let mut stack = vec![0u8; stack_top];

    let mut signal_info = vec![
        SignalInfo {
            name: String::new(),
        };
        io_signals.len()
    ];
    for (ir_signal, signal) in io_signals {
        let Signal {
            name: _,
            size,
            initialize,
        } = &gl.signals[ir_signal];
        let value = match initialize {
            None => Bits::new_zeroed(*size),
            Some(initialize) => {
                assert_eq!(initialize.size(), *size);
                initialize.clone()
            }
        };
        signals.insert(signal, value);
        signal_info[signal.0 as usize].name = gl.signals[ir_signal].name.clone();
    }

    let stdout = std::mem::replace(&mut ectx.stdout, Box::new(Vec::new()) as _);
    let stderr = std::mem::replace(&mut ectx.stderr, Box::new(Vec::new()) as _);

    let mut ctx = Context::new(stdout, stderr);
    ctx.tracing_level = ectx.trace;
    let fail = vogls_sim::run(
        &mut ctx,
        &processes,
        &mut regions,
        &mut signals,
        &signal_info,
        &mut listeners,
        &mut watches,
        &mut stack,
        ectx.time,
    )
    .is_err();

    ectx.stdout = ctx.stdout;
    ectx.stderr = ctx.stderr;

    if fail {
        return Err("execution failed.".into());
    }

    Ok(())
}
