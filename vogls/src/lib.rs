use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use slotmap::{SecondaryMap, SlotMap};
use vogls_ir::token_range::TokenRange;
use vogls_ir::{Bits, ContextFormat, GlobalContext, Instruction, Signal};
use vogls_sim::{
    Context, EvaluationEvent, Event, Regions, SignalInfo, VmProcess, VmProcessKey, VmSignalKey,
    lower_process_to_vm,
};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{
    CaseGenerateConstruct, CaseGenerateItem, GenerateBlock, IfGenerateConstruct,
    LoopGenerateConstruct, Module, ModuleItem, ModuleOrGenerateItem, NonPortModuleItem,
};
use vogls_verilog::elaborate::{elaborate_module, elaborate_module_or_generate_item};
use vogls_verilog::hierarchy::{
    Hierarchy, HierarchyGenerateBlock, HierarchyItem, HierarchyItemRange, HierarchyKey,
    HierarchyModule, HierarchyParameter, ScopeBuilder,
};
use vogls_verilog::lower::{Diagnostics as LowerDiagnostics, lower_module_to_ir};
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
    pub trace: bool,
    pub time: u64,
    pub opt_rounds: u8,
}

pub fn token_range_to_line_range(
    tokenized: &Tokenized,
    tr: TokenRange,
    line_luts: &[Vec<usize>],
) -> Option<vogls_trace::Span> {
    let file = tokenized.file_idxs[tr.start];
    if file == tokenized.file_idxs[tr.end - 1] {
        let span_start = tokenized.spans[tr.start].start();
        let span_end = tokenized.spans[tr.end - 1].end();

        let line_start = line_luts[file as usize]
            .binary_search(&span_start)
            .unwrap_or_else(|e| e - 1) as u64;
        let line_end = line_luts[file as usize]
            .binary_search(&span_end)
            .unwrap_or_else(|e| e) as u64;
        return Some(vogls_trace::Span {
            file: file as u64,
            line_range: line_start..line_end,
        });
    }

    None
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
                    default_nettype: _,
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
                    default_nettype: _,
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

    let mut hierarchy = Hierarchy::new(tl_module_name.to_string());
    let top_level_key = hierarchy.top_level_key();

    let mut diagnostics = LowerDiagnostics::default();
    let mut error = false;
    error |= elaborate_module(
        &mut gl.signals,
        &ast.arenas,
        ast.modules.get(*tl_module),
        &mut ScopeBuilder {
            hierarchy: &mut hierarchy,
            key: top_level_key,
        },
        &mut diagnostics,
    )
    .is_err();

    let mut offset = 1;
    while let Some(item) = hierarchy.symbols.get(offset) {
        use HierarchyItem as I;
        match item {
            I::Module(m) => {
                let m = *m;
                let items_len = hierarchy.symbols.len();
                let HierarchyModule {
                    name: _,
                    module_name,
                    children,
                    ast: _,
                    parent: _,
                    lut: _,
                    ports: _,
                    parameter_lut: _,
                    parameters: _,
                    parameter_overrides: _,
                } = &mut hierarchy.modules[m];

                *children = HierarchyItemRange {
                    start: items_len,
                    end: items_len,
                };
                let id = ast.modules.get(module_lut[module_name.as_str()]);

                error |= elaborate_module(
                    &mut gl.signals,
                    &ast.arenas,
                    id,
                    &mut ScopeBuilder {
                        hierarchy: &mut hierarchy,
                        key: HierarchyKey::new(offset),
                    },
                    &mut diagnostics,
                )
                .is_err();
            }
            I::NamedBlock(_) => todo!(),
            I::GenerateBlock(i) => {
                let HierarchyGenerateBlock {
                    ast: children,
                    genvar,
                    genvars,
                    ..
                } = &hierarchy.generate_blocks[*i];

                let children = *children;
                let genvar = genvar.clone();
                let mut genvars = genvars.clone();

                let mut builder = ScopeBuilder {
                    hierarchy: &mut hierarchy,
                    key: HierarchyKey::new(offset),
                };

                if let Some((name, value)) = genvar {
                    builder.insert_parameter(HierarchyParameter {
                        name,
                        parent: builder.key(),
                        value,
                    });
                };

                for id in children.iter() {
                    error |= elaborate_module_or_generate_item(
                        &mut gl.signals,
                        &ast.arenas,
                        id,
                        &mut builder,
                        &mut diagnostics,
                        &mut genvars
                    )
                    .is_err();
                }
            }
            I::Task(_) => todo!(),
            I::Function(_) => todo!(),

            I::Net(_) | I::Parameter(_) | I::GenVar(_) => {}
        }
        offset += 1;
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

    if ectx.emit_hierarchy {
        writeln!(
            ectx.stdout,
            "{}",
            hierarchy.display(hierarchy.top_level_key())
        )?;
    }

    // let Ok((top_level_params, top_level_io, parameters)) = fetch_module_interface(
    //     &mut gl,
    //     &ast.arenas,
    //     ast.modules.get(*tl_module),
    //     &[],
    //     &mut diagnostics,
    // ) else {
    //     for (location, warning) in &diagnostics.warnings {
    //         writeln!(ectx.stderr, "[WARN]: {warning}")?;
    //         let mut out = String::new();
    //         report(&token_buffer, *location, &mut out)?;
    //         writeln!(ectx.stderr, "{out}")?;
    //     }
    //
    //     for (location, err, context) in &diagnostics.errors {
    //         let mut out = String::new();
    //         report_error(&token_buffer, err.clone(), *location, &mut out)?;
    //         write!(ectx.stderr, "{out}")?;
    //         if !context.is_empty() {
    //             writeln!(ectx.stderr, "context:")?;
    //             for c in context {
    //                 writeln!(ectx.stderr, "- {c}")?;
    //             }
    //         }
    //         writeln!(ectx.stderr)?;
    //     }
    //     return Err("top_level fetch_module error".into());
    // };
    // if !top_level_io.ports.is_empty() {
    //     return Err("top_level has input and output ports".into());
    // }

    // Walk the modules in depth-first order and lower to IR.
    let mut error = false;
    let mut diagnostics = LowerDiagnostics::default();
    // @TODO: Iterate over the modules instead.
    for i in 0..hierarchy.items().len() {
        let HierarchyItem::Module(m) = hierarchy.items()[i] else {
            continue;
        };
        let key = HierarchyKey::new(i);
        let module = &hierarchy.modules()[m];
        let module_id = ast.modules.get(module_lut[module.module_name.as_str()]);
        let module_key = lower_module_to_ir(
            &mut gl,
            &ast.arenas,
            module_id,
            &mut vogls_verilog::lower::Scope {
                hierarchy: &mut hierarchy,
                key,
                signal_map: &mut HashMap::new(),
            },
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

    // for query in mc.queries_to_resolve {
    //     let ModuleQuery {
    //         bb,
    //         instruction,
    //         query,
    //     } = query;
    //
    //     assert!(query.is_none());
    //     let scope = hierarchy.vcd_scope(u32::MAX);
    //     let i = &mut gl.bbs[bb].instrs[instruction];
    //     let dst = i.get_destination_variable().unwrap();
    //     *i = Instruction::Intrinsic(
    //         dst,
    //         Box::new(vogls_ir::IntrinsicOp::VcdAppendModule(scope)),
    //         [].into(),
    //     )
    // }

    if ectx.emit_unoptimized_ir {
        for process in gl.processes.values() {
            writeln!(ectx.stdout, "{}", process.display(&gl))?;
        }
    }

    let mut scratch_stack = Vec::new();
    let mut scratch_mfr = Vec::new();
    let mut scratch_bb_to_bits_map = HashMap::new();
    let mut scratch_var_to_var_map = HashMap::new();
    let mut scratch_var_seen = HashSet::new();
    let mut scratch_seen = HashSet::new();
    let mut scratch_removed = HashSet::new();
    let mut scratch_fan_in = SecondaryMap::new();
    for process in gl.processes.values_mut() {
        if cfg!(debug_assertions) {
            vogls_ir::optimize::get_fan_in(
                &mut gl.bbs,
                process.entry,
                &mut scratch_stack,
                &mut scratch_seen,
                &mut scratch_fan_in,
            );
        }

        for _ in 0..ectx.opt_rounds {
            vogls_ir::optimize::remove_needless_jumps(
                &mut gl.bbs,
                process.entry,
                &mut scratch_stack,
                &mut scratch_seen,
                &mut scratch_fan_in,
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
                &mut scratch_fan_in,
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

        if cfg!(debug_assertions) {
            vogls_ir::optimize::get_fan_in(
                &mut gl.bbs,
                process.entry,
                &mut scratch_stack,
                &mut scratch_seen,
                &mut scratch_fan_in,
            );
        }
    }

    if ectx.emit_ir && !ectx.emit_vm {
        for process in gl.processes.values() {
            writeln!(ectx.stdout, "{}", process.display(&gl))?;
        }
    }

    let mut processes = Vec::<VmProcess>::default();
    let mut regions = Regions::new(3); // inactive, non-blocking, monitor
    let mut signals = HashMap::default();
    let mut listeners = SlotMap::default();
    let mut watches = HashMap::default();

    // // Find the entity for the Top-Level Module.
    // let mut elab_processes = Vec::new();
    // elaborate::elaborate(tl_module_key, &mut gl, &mut elab_processes);

    let mut trace_processes = Vec::new();
    let mut trace_signals = Vec::new();
    let mut line_luts = Vec::new();

    if ectx.trace {
        line_luts.extend(token_buffer.contents.iter().map(|c| {
            let mut s = c.as_ref();
            let original_length = s.len();
            let mut vs = Vec::new();
            while let Some(p) = s.find(['\n', '\r']) {
                if s.as_bytes()[p] == b'\r' {
                    todo!();
                }

                let offset = original_length - s.len();
                vs.push(offset);
                s = &s[p + 1..];
            }

            if !s.is_empty() {
                let offset = original_length - s.len();
                vs.push(offset);
            }

            vs
        }));
    }

    let mut io_signals = HashMap::new();
    let mut signal_info = vec![
        SignalInfo {
            name: String::new(),
        };
        io_signals.len()
    ];
    for (key, signal) in &gl.signals {
        let Signal {
            name,
            size,
            initialize,
            origin,
        } = signal;
        let value = match initialize {
            None => Bits::new_zeroed(*size),
            Some(initialize) => {
                assert_eq!(initialize.size(), *size);
                initialize.clone()
            }
        };

        if ectx.trace {
            trace_signals.push(vogls_trace::Signal {
                name: Some(name.clone()),
                location: token_range_to_line_range(&token_buffer, *origin, &line_luts),
                initial: value.clone(),
            });
        }

        let vm_signal_key = VmSignalKey(io_signals.len() as u64);
        io_signals.insert(key, vm_signal_key);
        signals.insert(vm_signal_key, value);
        signal_info.push(SignalInfo { name: name.clone() });
    }

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

        let vm_process_key = VmProcessKey(processes.len() as u64);
        processes.push(vm_process);

        if ectx.emit_vm {
            println!(": {vm_process_key:?}");
        }

        if ectx.trace {
            let vogls_ir::Process { name, origin, .. } = &gl.processes[process];

            trace_processes.push(vogls_trace::Process {
                name: Some(name.clone()),
                location: token_range_to_line_range(&token_buffer, *origin, &line_luts),
            });
        }

        regions.active.push(Event::Evaluation(EvaluationEvent {
            process: vm_process_key,
            ip: 0,
        }));
    }
    let mut stack = vec![0u8; stack_top];

    let stdout = std::mem::replace(&mut ectx.stdout, Box::new(Vec::new()) as _);
    let stderr = std::mem::replace(&mut ectx.stderr, Box::new(Vec::new()) as _);

    let mut trace = None;
    if ectx.trace {
        trace = Some(vogls_trace::Trace {
            files: token_buffer
                .contents
                .iter()
                .zip(&token_buffer.paths)
                .map(|(s, p)| vogls_trace::File {
                    name: p.as_ref().map(|p| p.display().to_string()),
                    content: s.to_string(),
                })
                .collect(),
            processes: trace_processes,
            signals: trace_signals,
            driven: Vec::new(),
            woken: Vec::new(),
            watches: Vec::new(),
            events: Vec::new(),
        });
    }
    let mut ctx = Context::new(stdout, stderr);
    let fail = vogls_sim::run(
        &mut ctx,
        &processes,
        &mut regions,
        &mut signals,
        &signal_info,
        &mut listeners,
        &mut watches,
        trace.as_mut(),
        &mut stack,
        ectx.time,
    )
    .is_err();

    ectx.stdout = ctx.stdout;
    ectx.stderr = ctx.stderr;

    if fail {
        return Err("execution failed.".into());
    }

    if let Some(trace) = trace {
        trace.dump(&mut std::io::BufWriter::new(std::fs::File::create(
            "dump.vgtd",
        )?))?;
    }

    Ok(())
}
