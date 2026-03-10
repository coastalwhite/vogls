use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

use slotmap::{SecondaryMap, SlotMap};
use vogls_codegen::{HeapBuilder, HeapOffset, HeapRef};
use vogls_codegen_c::runtime::{CDesign, CDesignState};
use vogls_codegen_c::{ListenerBuilder, lower_signal_drive_fn, lower_signal_drive_header};
use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_ir::{Bits, GlobalContext, LogicMode, Signal, SignalKey};
use vogls_runtime::RuntimeState;
use vogls_runtime::SimulationIo;
use vogls_sim::{
    Event, Regions, Simulation, VmProcess, VmProcessKey, VmSignalKey, lower_process_to_vm,
};
use vogls_utils::VgHashMap;
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{Description, Module, ModuleItem, NonPortModuleItem};
use vogls_verilog::elaborate::{VSymbol, VSymbolTable};
use vogls_verilog::lower::{Diagnostics as LowerDiagnostics, Scope, lower_module_to_ir};
use vogls_verilog::parser::{
    Diagnostics as ParserDiagnostics, ParseContext, ParserScratches, TokenWalker, parse_file,
    report, report_error,
};
use vogls_verilog::tokenizer::{Macro, Tokenized};

use crate::{ExecutionContext, append_referenced_modules, token_range_to_line_range};

pub enum DesignBackend {
    Interpretted {
        vm_signal_map: HashMap<SignalKey, VmSignalKey>,
        simulation: vogls_sim::Simulation,
    },
    Compiled {
        design: vogls_codegen_c::runtime::CDesign,
    },
}

pub struct Design {
    pub gl: GlobalContext,
    pub ident_table: IdentTable,
    pub elab_table: VSymbolTable,
    pub backend: DesignBackend,
    pub initial_state: DesignState,
}

#[derive(Clone)]
pub enum DesignState {
    Interpretted(vogls_sim::SimulationState),
    Compiled(vogls_codegen_c::runtime::CDesignState),
}

impl DesignState {
    pub fn runtime_mut(&mut self) -> &mut RuntimeState {
        match self {
            DesignState::Interpretted(s) => &mut s.runtime,
            DesignState::Compiled(s) => &mut s.runtime,
        }
    }
    pub fn runtime(&self) -> &RuntimeState {
        match self {
            DesignState::Interpretted(s) => &s.runtime,
            DesignState::Compiled(s) => &s.runtime,
        }
    }
}

impl Design {
    pub fn new(
        paths: &[&Path],
        top_level_module: Option<&str>,
        ectx: &mut ExecutionContext,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut token_buffer = Tokenized::default();
        let mut macros = HashMap::new();
        for define in &ectx.defines {
            macros.insert(define.clone(), Macro::default());
        }
        if ectx.logic_mode == LogicMode::TwoValue {
            macros.insert("__VOGLS__TWO_VALUE_LOGIC".to_string(), Macro::default());
        }
        for path in paths {
            let content: Rc<str> = std::fs::read_to_string(&path)?.into();
            token_buffer.append_tokenize_with_macros(content, Some((*path).into()), &mut macros);
        }

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

        let module_lut =
            VgHashMap::<IdentId, AstId<Module>>::from_iter(ast.descriptions.iter().filter_map(
                |id| match ast.arenas.get(id) {
                    Description::Module(id) => {
                        Some((ast.arenas.get(*id).module_identifier.item.0, *id))
                    }
                    Description::Udp(_) | Description::Config => None,
                },
            ));

        let tl_module_name = match top_level_module {
            Some(v) => ast.arenas.ident_table.get_or_insert(v),
            None => {
                let mut referenced = HashSet::new();
                for id in ast.descriptions {
                    let Description::Module(module_id) = *ast.arenas.get(id) else {
                        continue;
                    };

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

                        if let NonPortModuleItem::ModuleOrGenerateItem(module_item) =
                            ast.arenas.get(*p)
                        {
                            append_referenced_modules(&ast.arenas, *module_item, &mut referenced);
                        }
                    }
                }

                let mut top_level_modules = Vec::new();
                for id in ast.descriptions {
                    let Description::Module(module_id) = *ast.arenas.get(id) else {
                        continue;
                    };
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

        let mut diagnostics = LowerDiagnostics::default();
        let result = vogls_verilog::elaborate::next::elaborate(
            &mut gl,
            &ast.arenas,
            &token_buffer,
            *tl_module,
            &module_lut,
            &mut diagnostics,
        );

        if !diagnostics.warnings.is_empty() {
            for (location, warning) in &diagnostics.warnings {
                writeln!(ectx.stderr, "[WARN]: {warning}")?;
                let mut out = String::new();
                report(&token_buffer, *location, &mut out)?;
                writeln!(ectx.stderr, "{out}")?;
            }
        }

        let Ok(mut elab_table) = result else {
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
        };

        if ectx.emit_hierarchy {
            for root in elab_table.roots() {
                writeln!(
                    ectx.stdout,
                    "{}",
                    elab_table.display(*root, &ast.arenas.ident_table, |s, f| {
                        match s {
                            VSymbol::Module(_) => f.write_str("mod"),
                            VSymbol::Parameter(v) => {
                                if v.ty().is_signed() {
                                    f.write_str("signed ")?;
                                }
                                write!(f, "{}", v.clone().into_bits())?;
                                Ok(())
                            }
                            VSymbol::Net(s) => {
                                f.write_str("net")?;
                                if s.ty.is_signed() {
                                    f.write_str(" signed")?;
                                }
                                if s.ty.force_net_width().get() > 1 {
                                    write!(f, "[{}]", s.ty.force_net_width().get())?;
                                }
                                Ok(())
                            }
                            VSymbol::NamedBlock => f.write_str("named block"),
                            VSymbol::GenerateBlock(_) => f.write_str("generate block"),
                            VSymbol::GenerateBlocks => f.write_str("generate blocks"),
                            VSymbol::GenVar => f.write_str("genvar"),
                            VSymbol::Task(_) => f.write_str("task"),
                            VSymbol::Function(_) => f.write_str("function"),
                        }
                    })
                )?;
            }
        }

        let mut udps = VgHashMap::default();
        for description in ast.descriptions.iter() {
            let Description::Udp(udp_id) = ast.arenas.get(description) else {
                continue;
            };

            let udp_id = *udp_id;
            let ident = ast.arenas.get(udp_id).identifier.item.0;

            udps.insert(ident, udp_id);
        }

        let mut error = false;
        let mut signal_map = HashMap::new();
        let mut outs_lut = VgHashMap::default();
        let mut outs = Vec::new();

        // @TODO: Iterate over the modules instead.
        for key in elab_table.symbol_id_iter() {
            match &elab_table[key].content {
                VSymbol::Module(i) if i.contains_specify => {
                    let module = module_lut[&i.module];
                    for item in ast.arenas.get(module).module_items.iter() {
                        let ModuleItem::NonPortModuleItem(id) = ast.arenas.get(item) else {
                            continue;
                        };
                        let NonPortModuleItem::SpecifyBlock(specify_block) = ast.arenas.get(*id)
                        else {
                            continue;
                        };

                        error |= vogls_verilog::lower::specify::lower_specify(
                            &mut gl,
                            &ast.arenas,
                            &mut Scope {
                                table: &mut elab_table,
                                key,
                                udps: &udps,
                                signal_map: &mut signal_map,
                                tokenized: &token_buffer,
                            },
                            specify_block.items,
                            &mut outs_lut,
                            &mut outs,
                            &mut diagnostics,
                        )
                        .is_err();
                    }
                }
                VSymbol::Function(i) => {
                    let fn_decl = i.ast_id;
                    error |= vogls_verilog::lower::module_or_generate_item::function::lower(
                        &mut gl,
                        &ast.arenas,
                        &mut diagnostics,
                        &mut Scope {
                            table: &mut elab_table,
                            key,
                            udps: &udps,
                            signal_map: &mut signal_map,
                            tokenized: &token_buffer,
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
                            udps: &udps,
                            signal_map: &mut signal_map,
                            tokenized: &token_buffer,
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
        // @TODO: Iterate over the modules instead.
        for key in elab_table.symbol_id_iter() {
            let VSymbol::Module(m) = &elab_table[key].content else {
                continue;
            };
            let module_id = module_lut[&m.module];
            let module_key = lower_module_to_ir(
                &mut gl,
                &ast.arenas,
                module_id,
                &mut vogls_verilog::lower::Scope {
                    table: &mut elab_table,
                    key,
                    udps: &udps,
                    signal_map: &mut signal_map,
                    tokenized: &token_buffer,
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

        for symbol in elab_table.symbol_id_iter() {
            if let VSymbol::Net(n) = &mut elab_table[symbol].content {
                while let Some(s) = signal_map.get(&n.signal) {
                    n.signal = *s;
                }
            }
        }
        for bb in gl.bbs.values_mut() {
            bb.map_signals(|mut s| {
                while let Some(ns) = signal_map.get(&s) {
                    s = *ns;
                }
                s
            });
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
        let listeners = SlotMap::default();
        let mut watches = Vec::default();

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

        let mut heap_builder = HeapBuilder::new();
        let mut io_signals = HashMap::new();
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
            signals.push(HeapRef {
                offset: HeapOffset { bit_offset: 0 },
                size: vogls_ir::SCALAR_VSIZE,
            });
            watches.push(Vec::new());
        }

        for (min_bits, max_bits) in [
            // (1, u32::MAX)
            (33, u32::MAX),
            (17, 32),
            (9, 16),
            (5, 8),
            (3, 4),
            (2, 2),
            (1, 1),
        ] {
            for (i, signal) in gl.signals.values().enumerate() {
                let size = signal.size;
                let mut num_bits = size.get();
                if gl.logic_mode == LogicMode::FourValue {
                    num_bits = num_bits * 2;
                }

                if (min_bits..=max_bits).contains(&num_bits) {
                    signals[i] = heap_builder.claim(gl.logic_mode, size);
                }
            }
        }

        if ectx.compile {
            let mut listener_builder = ListenerBuilder::default();
            let io_signals = io_signals
                .iter()
                .map(|(k, v)| (*k, signals[v.0 as usize]))
                .collect();

            let mut out = Vec::new();

            for signal in gl.signals.keys() {
                lower_signal_drive_header(&mut out, signal, &io_signals)?;
            }

            for (i, process) in gl.processes.keys().enumerate() {
                vogls_codegen_c::lower_process(
                    &mut out,
                    process,
                    i,
                    &gl,
                    &mut heap_builder,
                    &mut listener_builder,
                    &io_signals,
                )?;
            }

            for signal in gl.signals.keys() {
                lower_signal_drive_fn(&mut out, &gl, signal, &listener_builder, &io_signals)?;
            }

            vogls_codegen_c::lower_startup_function(&mut out, &gl)?;

            let mut c_file = Vec::new();

            vogls_codegen_c::prologue(&mut c_file)?;
            c_file.extend(&out);
            vogls_codegen_c::epilogue(&mut c_file)?;
            // vogls_codegen_c::add_main(&mut c_file, &gl, &heap_builder, &listener_builder)?;

            std::fs::write("t2.c", &c_file)?;

            let mut cc = Command::new("cc")
                .args([
                    "-x",
                    "c",
                    "-g3",
                    "-fPIC",
                    "-O2",
                    // "t2.c",
                    "-",
                    "-shared",
                    "-o",
                    "/tmp/vogls-target.so",
                ])
                .stdin(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            cc.stdin.take().unwrap().write_all(&c_file)?;
            if !cc.wait().unwrap().success() {
                return Err("compilation failed!".into());
            }

            let initial_state = CDesignState::new(
                &gl,
                heap_builder.finish(),
                listener_builder.top,
                regions.num_additional_regions() as u8,
            );
            let design = CDesign::new(
                &Path::new("/tmp/vogls-target.so"),
                regions.num_additional_regions() as u8,
            );
            return Ok(Self {
                gl,
                ident_table: ast.arenas.ident_table,
                elab_table,
                backend: DesignBackend::Compiled { design },
                initial_state: DesignState::Compiled(initial_state),
            });
        }

        for process in gl.processes.keys() {
            if ectx.emit_vm && ectx.emit_ir {
                println!();
                println!("{}", gl.processes[process].display(&gl));
            }
            let vm_process = lower_process_to_vm(
                process,
                &gl,
                &mut heap_builder,
                &signals,
                &mut io_signals,
                &signal_map,
            );

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
        let mut heap = heap_builder.finish();

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
                heap.store_bits(
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

        let mut simulation = Simulation::new(processes, signals, gl.logic_mode);
        simulation.itrace = ectx.itrace;
        let mut initial_state = simulation.new_state(regions, listeners, watches, heap);

        if let Some(vcd_path) = &ectx.vcd {
            let tlm = elab_table.roots()[0];
            let scope = Scope {
                table: &mut elab_table,
                key: tlm,
                udps: &udps,
                signal_map: &mut signal_map,
                tokenized: &token_buffer,
            };
            let scope = scope.vcd_scope(&ast.arenas.ident_table);
            let scope = vogls_sim::VcdScope::lower(&scope, &io_signals, &signal_map);
            initial_state.start_vcd(vcd_path, scope);
        }

        Ok(Self {
            gl,
            ident_table: ast.arenas.ident_table,
            elab_table,
            backend: DesignBackend::Interpretted {
                vm_signal_map: io_signals,
                simulation,
            },
            initial_state: DesignState::Interpretted(initial_state),
        })
    }

    pub fn run(
        mut self,
        io: &mut SimulationIo,
        time: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match (&mut self.backend, &mut self.initial_state) {
            (
                DesignBackend::Interpretted {
                    vm_signal_map: _,
                    simulation,
                },
                DesignState::Interpretted(initial_state),
            ) => simulation
                .run(initial_state, io, time)
                .map_err(|_| "execution failed.".into()),
            (DesignBackend::Compiled { design }, DesignState::Compiled(initial_state)) => design
                .run(initial_state, io, time)
                .map_err(|_| "execution failed.".into()),
            _ => panic!(),
        }
    }

    pub fn run_from_state(
        &self,
        state: &mut DesignState,
        io: &mut SimulationIo,
        time: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match (&self.backend, state) {
            (
                DesignBackend::Interpretted {
                    vm_signal_map: _,
                    simulation,
                },
                DesignState::Interpretted(state),
            ) => simulation
                .run(state, io, time)
                .map_err(|_| "execution failed.".into()),
            (DesignBackend::Compiled { design }, DesignState::Compiled(state)) => design
                .run(state, io, time)
                .map_err(|_| "execution failed.".into()),
            _ => unreachable!(),
        }
    }
}
