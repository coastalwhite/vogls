use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use slotmap::SlotMap;
use vogls_codegen::{HeapBuilder, HeapRef};
use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_ir::{Bits, GlobalContext, LogicMode, SignalKey};
use vogls_runtime::SimulationIo;
use vogls_runtime::plugins::RuntimePluginState;
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_sim::{Event, Regions, Simulation, VmProcess, VmProcessKey, lower_process_to_vm};
use vogls_utils::{NonMaxU32, TimerStack, VgHashMap};
use vogls_verilog::arena::Arena;
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{Description, Module, ModuleItem, NonPortModuleItem};
use vogls_verilog::elaborate::{SymbolAstRefs, VSymbol, VSymbolTable};
use vogls_verilog::lower::{
    Diagnostics as LowerDiagnostics, LowerContext, MutLowerContext, lower_module_to_ir,
};
use vogls_verilog::parser::{
    AstArenas, Diagnostics as ParserDiagnostics, ParseContext, ParserScratches, TokenWalker,
    parse_file, report, report_error,
};
use vogls_verilog::tokenizer::{Macro, Tokenized};

use crate::fuse_signals::FuseSignalsContext;
use crate::{
    ExecutionContext, append_referenced_modules, fuse_signals, generate_signals_heap,
    lower_to_shared_object,
};

pub enum DesignBackend {
    Interpretted {
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
    pub rt_signal_map: VgHashMap<SignalKey, RtSignalKey>,
    pub signal_to_heap: Arc<[HeapRef]>,
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
        timers: &mut TimerStack,
        top_level_module: Option<&str>,
        ectx: &mut ExecutionContext,
        mut plugins: Vec<RuntimePluginState>,
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

        let mut tkw = timers.timed("tokenization", |_| TokenWalker::new(&token_buffer));
        let mut diagnostics = ParserDiagnostics::default();
        let ast = Arena::new();
        let mut arenas = AstArenas::default();

        let f = match timers.timed("parsing", |_| {
            parse_file(
                &mut tkw,
                &mut ParserScratches::default(),
                Some(&mut diagnostics),
                &mut arenas,
                &ast,
                &mut ParseContext::default(),
            )
        }) {
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

        let module_lut = VgHashMap::<IdentId, AstId<Module>>::from_iter(
            f.descriptions.iter().filter_map(|id| match &*id {
                Description::Module(id) => Some((id.module_identifier.item.0, *id)),
                Description::Udp(_) | Description::Config => None,
            }),
        );

        let tl_module_name = match top_level_module {
            Some(v) => arenas.ident_table.get_or_insert(v),
            None => {
                let mut referenced = HashSet::new();
                for id in f.descriptions {
                    let Description::Module(module_id) = &*id else {
                        continue;
                    };

                    let Module {
                        attribute_instances: _,
                        module_identifier: _,
                        module_parameter_port_list: _,
                        module_items,
                        ports: _,
                        default_nettype: _,
                    } = &**module_id;

                    for module_item in module_items.iter() {
                        let ModuleItem::NonPortModuleItem(p) = &*module_item else {
                            continue;
                        };

                        if let NonPortModuleItem::ModuleOrGenerateItem(module_item) = &**p {
                            append_referenced_modules(&arenas, *module_item, &mut referenced);
                        }
                    }
                }

                let mut top_level_modules = Vec::new();
                for id in f.descriptions {
                    let Description::Module(module_id) = &*id else {
                        continue;
                    };
                    let Module {
                        attribute_instances: _,
                        module_identifier,
                        module_parameter_port_list: _,
                        module_items: _,
                        ports: _,
                        default_nettype: _,
                    } = &**module_id;
                    let module_name = module_identifier.item.0;
                    if referenced.contains(&module_name) {
                        continue;
                    }
                    top_level_modules.push((*module_id, module_name));
                }

                if top_level_modules.len() == 0 {
                    return Err(<Box<dyn std::error::Error>>::from(
                        "no top-level module found".to_string(),
                    ));
                } else if top_level_modules.len() > 1 {
                    let names = top_level_modules
                        .iter()
                        .map(|(_, n)| &arenas.ident_table[*n])
                        .collect::<Vec<&str>>();
                    writeln!(
                        ectx.stderr,
                        "[ERR]: Found {} possible top-level modules: {names:?}",
                        top_level_modules.len()
                    )?;
                    let mut out = String::new();
                    for (m, _) in top_level_modules {
                        out.clear();
                        let span = arenas.get_item_span(m.module_identifier);
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

        let mut ctx = LowerContext {
            table: VSymbolTable::default(),
            table_ast_refs: SymbolAstRefs::default(),
            udps: VgHashMap::default(),
            arenas,
            tokenized: &token_buffer,
        };
        let mut mctx = MutLowerContext {
            gl,
            diagnostics: LowerDiagnostics::default(),
            connections: Vec::new(),
            fuse_scratch: Vec::new(),
            has_vcd: false,
        };
        let result = timers.timed("elaboration", |_| {
            vogls_verilog::elaborate::next::elaborate(
                &mut mctx.gl,
                &mut ctx,
                *tl_module,
                &module_lut,
                &mut mctx.diagnostics,
            )
        });

        if !mctx.diagnostics.warnings.is_empty() {
            for (location, warning) in &mctx.diagnostics.warnings {
                writeln!(ectx.stderr, "[WARN]: {warning}")?;
                let mut out = String::new();
                report(&token_buffer, *location, &mut out)?;
                writeln!(ectx.stderr, "{out}")?;
            }
        }

        if result.is_err() {
            for (location, err, context) in &mctx.diagnostics.errors {
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
            for root in ctx.table.roots() {
                writeln!(
                    ectx.stdout,
                    "{}",
                    ctx.table.display(*root, &ctx.arenas.ident_table, |s, f| {
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

        for description in f.descriptions.iter() {
            let Description::Udp(udp_id) = &*description else {
                continue;
            };

            let udp_id = *udp_id;
            let ident = udp_id.identifier.item.0;

            ctx.udps.insert(ident, udp_id);
        }

        let mut error = false;
        let mut outs_lut = VgHashMap::default();
        let mut outs = Vec::new();

        // @TODO: Iterate over the modules instead.
        timers.start("lower_global_items");
        for key in ctx.table.symbol_id_iter() {
            match &ctx.table[key].content {
                VSymbol::Module(i) => {
                    let module = module_lut[&i.module];
                    if i.contains_specify {
                        for item in module.module_items.iter() {
                            let ModuleItem::NonPortModuleItem(id) = &*item else {
                                continue;
                            };
                            let NonPortModuleItem::SpecifyBlock(specify_block) = **id else {
                                continue;
                            };

                            error |= vogls_verilog::lower::specify::lower_specify(
                                &mut ctx,
                                &mut mctx,
                                key,
                                specify_block.items,
                                &mut outs_lut,
                                &mut outs,
                            )
                            .is_err();
                        }
                    }

                    error |= vogls_verilog::lower::instantiate_nba_signals(
                        &mut mctx.gl,
                        &mut ctx,
                        key,
                        module,
                        &mut mctx.diagnostics,
                    )
                    .is_err();
                }
                VSymbol::Function(i) => {
                    let fn_decl = ctx.table_ast_refs.fns[i.ast_id];
                    error |= vogls_verilog::lower::module_or_generate_item::function::lower(
                        &mut ctx, &mut mctx, key, fn_decl,
                    )
                    .is_err();
                }
                VSymbol::Task(i) => {
                    let task_decl = ctx.table_ast_refs.tasks[i.ast_id];
                    error |= vogls_verilog::lower::module_or_generate_item::function::lower_task(
                        &mut ctx, &mut mctx, key, task_decl,
                    )
                    .is_err();
                }
                _ => {}
            }
        }
        timers.stop();

        if error {
            for (location, err, context) in &mctx.diagnostics.errors {
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
        // @TODO: Iterate over the modules instead.
        timers.start("lower");
        for key in ctx.table.symbol_id_iter() {
            let VSymbol::Module(m) = &ctx.table[key].content else {
                continue;
            };
            let module_id = module_lut[&m.module];
            let module_key = timers.timed(
                ctx.arenas.ident_table[module_id.module_identifier.item.0].to_string(),
                |_| lower_module_to_ir(module_id, &ctx, &mut mctx, key),
            );
            error |= module_key.is_err();
        }
        timers.stop();

        if !mctx.diagnostics.warnings.is_empty() {
            for (location, warning) in &mctx.diagnostics.warnings {
                writeln!(ectx.stderr, "[WARN]: {warning}")?;
                let mut out = String::new();
                report(&token_buffer, *location, &mut out)?;
                writeln!(ectx.stderr, "{out}")?;
            }
        }

        if error {
            for (location, err, context) in &mctx.diagnostics.errors {
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

        let fused = timers.timed("fuse_signals", |_| {
            fuse_signals::fuse_signals(
                &mut mctx.gl,
                &mctx.connections,
                &FuseSignalsContext {
                    print_unoptimized_fuse_signals: ectx.print_unoptimized_fuse_signals,
                    print_round_fuse_signals: ectx.print_round_fuse_signals,
                    print_optimized_fuse_signals: ectx.print_optimized_fuse_signals,
                },
            )
        });
        for symbol in ctx.table.symbol_id_iter() {
            if let VSymbol::Net(net) = &mut ctx.table[symbol].content {
                net.net.replace_signals(|s| {
                    if fused.contains_key(&s) {
                        mctx.gl.signals.remove(s);
                    }
                    fused.get(&s).copied().map_or((s, None), |(r, slice)| {
                        (r, slice.map(|s| NonMaxU32::new(s.lsb()).unwrap()))
                    })
                });
            }
        }

        if ectx.emit_unoptimized_ir {
            for signal in mctx.gl.signals.values() {
                writeln!(ectx.stdout, "{}", signal.display())?;
            }
            writeln!(ectx.stdout)?;
            for process in mctx.gl.processes.values() {
                writeln!(ectx.stdout, "{}", process.display(&mctx.gl))?;
            }
        }

        timers.timed("optimization", |_| {
            let processes = mctx.gl.processes.keys().collect::<Vec<_>>();
            vogls_ir::optimize::optimize_processes(mctx.gl(), &processes, ectx.opt)
        });

        if ectx.emit_ir {
            for signal in mctx.gl.signals.values() {
                writeln!(ectx.stdout, "{}", signal.display())?;
            }
            writeln!(ectx.stdout)?;
            for process in mctx.gl.processes.values() {
                writeln!(ectx.stdout, "{}", process.display(&mctx.gl))?;
            }
        }

        timers.start("build heap");
        let mut heap_builder = HeapBuilder::new();
        let mut signal_to_heap = Vec::new();
        let mut rt_signal_map = VgHashMap::default();
        generate_signals_heap(
            &mut heap_builder,
            &mut rt_signal_map,
            &mctx.gl.signals,
            &mut signal_to_heap,
            mctx.gl.logic_mode,
        );
        timers.stop();

        let signal_to_heap: Arc<[HeapRef]> = signal_to_heap.into();
        if mctx.has_vcd || ectx.vcd.is_some() {
            let tlm = ctx.table.roots()[0];
            let scope = ctx.vcd_scope(tlm, &ctx.arenas.ident_table);
            let (children, map) = vogls_vcd::VcdScope::lower(&scope, &rt_signal_map);
            let rtvcdoutput = match &ectx.vcd {
                Some(path) => {
                    vogls_vcd::RtVcdOutput::new_path(path, signal_to_heap.clone(), children, map)
                }
                None => vogls_vcd::RtVcdOutput::new(
                    Box::new(Vec::new()),
                    signal_to_heap.clone(),
                    children,
                    map,
                ),
            };
            plugins.push(Box::new(rtvcdoutput));
        }

        let num_regions = 3; // inactive, non-blocking, monitor
        if ectx.compile {
            Self::from_gl_compiled(
                mctx.gl,
                heap_builder,
                timers,
                ectx.itrace,
                ectx.stats,
                ectx.debug_symbols,
                ectx.output_source.as_deref(),
                rt_signal_map,
                signal_to_heap,
                num_regions,
                plugins,
                ctx.arenas.ident_table,
                ctx.table,
            )
        } else {
            Self::from_gl_interpretted(
                mctx.gl,
                heap_builder,
                timers,
                ectx.itrace,
                ectx.emit_vm,
                rt_signal_map,
                signal_to_heap,
                num_regions,
                plugins,
                ctx.arenas.ident_table,
                ctx.table,
            )
        }
    }

    pub fn new_vir(
        content: &str,
        timers: &mut TimerStack,
        ectx: &mut ExecutionContext,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut gl = GlobalContext::default();
        gl.logic_mode = ectx.logic_mode;
        vogls_ir::parse::parse(&content, &mut gl)?;

        if ectx.emit_unoptimized_ir {
            for signal in gl.signals.values() {
                println!("{}", signal.display());
            }
            println!();
            for process in gl.processes.values() {
                println!("{}", process.display(&gl));
            }
        }

        timers.timed("optimization", |_| {
            let processes = gl.processes.keys().collect::<Vec<_>>();
            vogls_ir::optimize::optimize_processes(&mut gl, &processes, ectx.opt);
        });

        if ectx.emit_ir {
            for signal in gl.signals.values() {
                println!("{}", signal.display());
            }
            println!();
            for process in gl.processes.values() {
                println!("{}", process.display(&gl));
            }
        }

        let mut heap_builder = HeapBuilder::new();
        let mut signal_to_heap = Vec::new();
        let mut rt_signal_map = VgHashMap::default();
        timers.timed("generate heap", |_| {
            generate_signals_heap(
                &mut heap_builder,
                &mut rt_signal_map,
                &gl.signals,
                &mut signal_to_heap,
                gl.logic_mode,
            )
        });

        if ectx.compile {
            Self::from_gl_compiled(
                gl,
                heap_builder,
                timers,
                ectx.itrace,
                ectx.stats,
                ectx.debug_symbols,
                None,
                rt_signal_map,
                signal_to_heap.into(),
                3,
                Vec::new(),
                IdentTable::default(),
                VSymbolTable::default(),
            )
        } else {
            Self::from_gl_interpretted(
                gl,
                heap_builder,
                timers,
                ectx.itrace,
                ectx.emit_vm,
                rt_signal_map,
                signal_to_heap.into(),
                3,
                Vec::new(),
                IdentTable::default(),
                VSymbolTable::default(),
            )
        }
    }

    pub fn from_gl_compiled(
        gl: GlobalContext,
        heap_builder: HeapBuilder,
        timers: &mut TimerStack,
        itrace: bool,
        stats: bool,
        debug_symbols: bool,
        output_source: Option<&Path>,
        rt_signal_map: VgHashMap<SignalKey, RtSignalKey>,
        signal_to_heap: Arc<[HeapRef]>,
        num_regions: u8,
        plugins: Vec<RuntimePluginState>,
        ident_table: IdentTable,
        elab_table: VSymbolTable,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (initial_state, design) = lower_to_shared_object(
            &gl,
            &rt_signal_map,
            heap_builder,
            &signal_to_heap,
            timers,
            itrace,
            stats,
            debug_symbols,
            output_source.as_deref(),
            plugins,
            num_regions,
        )?;

        return Ok(Self {
            gl,
            ident_table,
            elab_table,
            backend: DesignBackend::Compiled { design },
            rt_signal_map,
            signal_to_heap,
            initial_state: DesignState::Compiled(initial_state),
        });
    }

    pub fn from_gl_interpretted(
        gl: GlobalContext,
        mut heap_builder: HeapBuilder,
        timers: &mut TimerStack,
        itrace: bool,
        emit_vm: bool,
        mut rt_signal_map: VgHashMap<SignalKey, RtSignalKey>,
        signal_to_heap: Arc<[HeapRef]>,
        num_regions: u8,
        plugins: Vec<RuntimePluginState>,
        ident_table: IdentTable,
        elab_table: VSymbolTable,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut processes = Vec::<VmProcess>::default();
        let mut regions = Regions::new(num_regions as usize);

        let listeners = SlotMap::default();
        let watches = vec![Vec::new(); gl.signals.len()];

        timers.timed("lower to VM", |_| {
            for process in gl.processes.keys() {
                let vm_process = lower_process_to_vm(
                    process,
                    &gl,
                    &mut heap_builder,
                    &signal_to_heap,
                    &mut rt_signal_map,
                );
                let vm_process_key = VmProcessKey(processes.len() as u64);
                processes.push(vm_process);
                regions.active.push(Event {
                    process: vm_process_key,
                    ip: 0,
                });
            }
        });

        if emit_vm {
            for process in &processes {
                print!("{}", &process);
            }
        }
        let mut heap = heap_builder.finish();

        for (key, signal) in &gl.signals {
            if let Some(initialize) = &signal.initialize {
                assert_eq!(initialize.size(), signal.size);
                heap.store_bits(
                    signal_to_heap[rt_signal_map[&key].as_usize()],
                    gl.logic_mode,
                    initialize,
                );
            }
        }

        let mut simulation = Simulation::new(processes, signal_to_heap.clone(), gl.logic_mode);
        simulation.itrace = itrace;
        let mut initial_state = simulation.new_state(regions, listeners, watches, heap);
        initial_state.plugins = plugins;

        Ok(Self {
            gl,
            ident_table,
            elab_table,
            backend: DesignBackend::Interpretted { simulation },
            rt_signal_map,
            signal_to_heap,
            initial_state: DesignState::Interpretted(initial_state),
        })
    }

    pub fn run(
        &mut self,
        io: &mut SimulationIo,
        time: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match (&mut self.backend, &mut self.initial_state) {
            (
                DesignBackend::Interpretted { simulation },
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
            (DesignBackend::Interpretted { simulation }, DesignState::Interpretted(state)) => {
                simulation
                    .run(state, io, time)
                    .map_err(|_| "execution failed.".into())
            }
            (DesignBackend::Compiled { design }, DesignState::Compiled(state)) => design
                .run(state, io, time)
                .map_err(|_| "execution failed.".into()),
            _ => unreachable!(),
        }
    }

    pub fn get_rt_signal(&self, signal: SignalKey) -> RtSignalKey {
        self.rt_signal_map[&signal]
    }

    pub fn get_heap_ref(&self, signal: RtSignalKey) -> HeapRef {
        self.signal_to_heap[signal.as_usize()]
    }

    pub fn set_signal(&self, state: &mut DesignState, signal: RtSignalKey, bits: &Bits) {
        let heap_ref = self.get_heap_ref(signal);
        let updated = &state.runtime().heap.load_bits(heap_ref, self.gl.logic_mode) != bits;

        if updated {
            state
                .runtime_mut()
                .heap
                .store_bits(heap_ref, self.gl.logic_mode, bits);

            match (&self.backend, state) {
                (DesignBackend::Interpretted { simulation }, DesignState::Interpretted(state)) => {
                    simulation.poke_signal(state, signal)
                }
                (DesignBackend::Compiled { design }, DesignState::Compiled(state)) => {
                    design.poke_signal(state, signal)
                }
                _ => unreachable!(),
            }
        }
    }

    pub fn get_signal(&self, state: &DesignState, signal: RtSignalKey) -> Bits {
        let heap_ref = self.get_heap_ref(signal);
        state.runtime().heap.load_bits(heap_ref, self.gl.logic_mode)
    }

    pub fn emit_ir(&self) -> String {
        let mut s = String::new();
        for process in self.gl.processes.values() {
            use std::fmt::Write;
            writeln!(&mut s, "{}", process.display(&self.gl)).unwrap();
        }
        s
    }
}
