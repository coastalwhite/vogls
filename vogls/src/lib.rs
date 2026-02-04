use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use slotmap::{SecondaryMap, SlotMap};
use vogls_frontend::VgHashMap;
use vogls_frontend::ident_table::IdentId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{Bits, GlobalContext, LogicMode, Signal};
use vogls_sim::{
    Context, Event, Regions, SignalInfo, StackBuilder, VmProcess, VmProcessKey, VmSignalKey,
    lower_process_to_vm,
};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{
    CaseGenerateConstruct, CaseGenerateItem, GenerateBlock, IfGenerateConstruct,
    LoopGenerateConstruct, Module, ModuleItem, ModuleOrGenerateItem, NonPortModuleItem,
};
use vogls_verilog::elaborate::{ModuleSymbol, VSymbol, VSymbolTable, elaborate_module};
use vogls_verilog::lower::{Diagnostics as LowerDiagnostics, Scope, lower_module_to_ir};
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
    pub itrace: bool,
    pub time: u64,
    pub opt_rounds: u8,
    pub logic_mode: LogicMode,
    pub no_run: bool,
    pub vcd: Option<PathBuf>,
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
    referenced: &mut HashSet<IdentId>,
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
    referenced: &mut HashSet<IdentId>,
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
    referenced: &mut HashSet<IdentId>,
) {
    match arenas.get(module_or_generate_item) {
        ModuleOrGenerateItem::ModuleInstantiation(module_instantiation) => {
            let module_instantiation = arenas.get(*module_instantiation);
            let module_name = module_instantiation.module_identifier.item.0;
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

    let mut ast = match parse_file(
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
    gl.logic_mode = ectx.logic_mode;

    let module_lut = HashMap::<IdentId, usize>::from_iter(ast.modules.iter().enumerate().map(
        |(i, module_id)| {
            let module = ast.arenas.get(module_id);
            (module.module_identifier.item.0, i)
        },
    ));

    let tl_module_name = match top_level_module {
        Some(v) => ast.arenas.ident_table.get_or_insert(v),
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
                let module_name = module_identifier.item.0;
                if referenced.contains(&module_name) {
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
                    .map(|(_, n)| &ast.arenas.ident_table[*n])
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
    let Some(tl_module) = module_lut.get(&tl_module_name) else {
        return Err(<Box<dyn std::error::Error>>::from(
            "cannot find top-level module".to_string(),
        ));
    };

    let mut elab_table = VSymbolTable::default();
    let mut module_instance_stack = Vec::new();

    let tl_module_symid = elab_table
        .insert_root(
            tl_module_name,
            ast.arenas.get_item_span(
                ast.arenas
                    .get(ast.modules.get(*tl_module))
                    .module_identifier,
            ),
            VSymbol::Module(ModuleSymbol {
                module: tl_module_name,
                ports: Vec::new(),
                parameters: Vec::new(),
                parameter_overrides: Arc::new(VgHashMap::default()),
                parameter_override_values: Arc::new(Vec::new()),
            }),
        )
        .expect("There are no symbols yet. This cannot fail");

    let mut diagnostics = LowerDiagnostics::default();
    let mut error = false;

    module_instance_stack.push(tl_module_symid);
    while let Some(module_symid) = module_instance_stack.pop() {
        let VSymbol::Module(m) = &elab_table[module_symid].content else {
            unreachable!()
        };
        error |= elaborate_module(
            &mut gl,
            &ast.arenas,
            ast.modules.get(module_lut[&m.module]),
            module_symid,
            &mut elab_table,
            &mut module_instance_stack,
            &mut diagnostics,
        )
        .is_err();
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
            elab_table.display(tl_module_symid, &ast.arenas.ident_table, |s, f| {
                match s {
                    VSymbol::Module(_) => f.write_str("mod"),
                    VSymbol::Parameter(v) => write!(f, "{v:?}"),
                    VSymbol::Net(_) => f.write_str("net"),
                    VSymbol::NamedBlock => f.write_str("named block"),
                    VSymbol::GenerateBlock(_) => f.write_str("generate block"),
                    VSymbol::GenVar => f.write_str("genvar"),
                    VSymbol::Task(_) => f.write_str("task"),
                    VSymbol::Function(_) => f.write_str("function"),
                }
            })
        )?;
    }

    let mut signal_map = HashMap::new();
    // @TODO: Iterate over the modules instead.
    for key in elab_table.symbol_id_iter() {
        match &elab_table[key].content {
            VSymbol::Function(i) => {
                let fn_decl = i.ast_id;
                error |= vogls_verilog::lower::module_or_generate_item::function::lower(
                    &mut gl,
                    &ast.arenas,
                    &mut diagnostics,
                    &mut Scope {
                        table: &mut elab_table,
                        key,
                        signal_map: &mut signal_map,
                    },
                    fn_decl,
                )
                .is_err();
            }
            VSymbol::Task(i) => {
                let task_decl = i.ast_id;
                error |= vogls_verilog::lower::module_or_generate_item::function::lower_task(
                    &mut gl,
                    &ast.arenas,
                    &mut diagnostics,
                    &mut Scope {
                        table: &mut elab_table,
                        key,
                        signal_map: &mut signal_map,
                    },
                    task_decl,
                )
                .is_err();
            }
            _ => {}
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

    // Walk the modules in depth-first order and lower to IR.
    let mut diagnostics = LowerDiagnostics::default();
    let mut signal_map = HashMap::new();
    // @TODO: Iterate over the modules instead.
    for key in elab_table.symbol_id_iter() {
        let VSymbol::Module(m) = &elab_table[key].content else {
            continue;
        };
        let module_id = ast.modules.get(module_lut[&m.module]);
        let module_key = lower_module_to_ir(
            &mut gl,
            &ast.arenas,
            module_id,
            &mut vogls_verilog::lower::Scope {
                table: &mut elab_table,
                key,
                signal_map: &mut signal_map,
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
    let mut signals = Vec::default();
    let mut listeners = SlotMap::default();
    let mut watches = Vec::default();

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

    let mut stack_builder = StackBuilder::new();
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
        signals.push(stack_builder.claim(gl.logic_mode, *size));
        watches.push(Vec::new());
        signal_info.push(SignalInfo { name: name.clone() });
    }

    for process in gl.processes.keys() {
        if ectx.emit_vm && ectx.emit_ir {
            println!();
            println!("{}", gl.processes[process].display(&gl));
        }
        let vm_process =
            lower_process_to_vm(process, &gl, &mut stack_builder, &signals, &mut io_signals);

        if ectx.emit_vm {
            print!("{}", &vm_process);
        }

        let vm_process_key = VmProcessKey(processes.len() as u64);
        processes.push(vm_process);

        if ectx.emit_vm {
            println!(": {vm_process_key:?}");
        }

        let vogls_ir::Process { name, origin, .. } = &gl.processes[process];
        if ectx.trace {
            trace_processes.push(vogls_trace::Process {
                name: Some(name.clone()),
                location: token_range_to_line_range(&token_buffer, *origin, &line_luts),
            });
        }

        regions.active.push(Event {
            process: vm_process_key,
            ip: 0,
        });
    }
    let mut stack = stack_builder.finish();

    for (key, signal) in &gl.signals {
        let Signal {
            name,
            size,
            initialize,
            origin,
        } = signal;
        let mut value = None;
        if let Some(initialize) = initialize {
            assert_eq!(initialize.size(), *size);
            stack.store_bits(
                signals[io_signals[&key].0 as usize],
                gl.logic_mode,
                initialize,
            );
            value = Some(initialize);
        }

        if ectx.trace {
            trace_signals.push(vogls_trace::Signal {
                name: Some(name.clone()),
                location: token_range_to_line_range(&token_buffer, *origin, &line_luts),
                initial: value.cloned().unwrap_or_else(|| Bits::new_zeroed(*size)),
            });
        }
    }

    if ectx.no_run {
        return Ok(());
    }

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

    let mut ctx = Context::new(gl.logic_mode, stdout, stderr);
    ctx.itrace = ectx.itrace;
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
        ectx.vcd.as_deref().map(|p| {
            let scope = Scope {
                table: &mut elab_table,
                key: tl_module_symid,
                signal_map: &mut signal_map,
            };
            let scope = scope.vcd_scope(&ast.arenas.ident_table);
            let scope = vogls_sim::VcdScope::lower(&scope, &io_signals);
            (p, scope)
        }),
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
