use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use slotmap::SlotMap;
use vogls_codegen::{HeapBuilder, HeapRef};
use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::{FrozenSymbolTable, SymbolId};
use vogls_ir::vcd::{VcdScope, VcdValue, VcdVariableKey};
use vogls_ir::{Bits, GlobalContext, LogicMode, ProcessKind, SignalKey};
use vogls_runtime::SimulationIo;
use vogls_runtime::plugins::RuntimePluginState;
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_sim::{Event, Regions, Simulation, VmProcess, VmProcessKey, lower_process_to_vm};
use vogls_utils::{IndexMap, NonMaxU32, Table, TimerStack, VgHashMap};
use vogls_verilog::arena::Arena;
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{Description, Module, ModuleItem, NonPortModuleItem, TimeScale};
use vogls_verilog::elaborate::{SymbolAstRefs, VSymbol, VSymbolTable, determine_module_context};
use vogls_verilog::lower::{
    Diagnostics as LowerDiagnostics, LowerContext, MutLowerContext, create_nba_process,
    lower_module_to_ir,
};
use vogls_verilog::parser::{
    AstArenas, Diagnostics as ParserDiagnostics, ParseContext, ParserScratches, TokenWalker,
    parse_file, report, report_error,
};
use vogls_verilog::tokenizer::{Macro, Tokenized};

use crate::symbol::{NetValue, Symbol};
use crate::{
    ExecutionContext, append_referenced_modules, find_lupdt_signals, generate_signals_heap,
    lower_to_shared_object,
};
use vogls_fuse_signals::FuseTarget;

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
    pub elab_table: FrozenSymbolTable<Symbol>,
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
    pub fn plugins_mut(&mut self) -> &mut [RuntimePluginState] {
        match self {
            DesignState::Interpretted(s) => &mut s.plugins,
            DesignState::Compiled(s) => &mut s.plugins,
        }
    }
    pub fn plugins(&self) -> &[RuntimePluginState] {
        match self {
            DesignState::Interpretted(s) => &s.plugins,
            DesignState::Compiled(s) => &s.plugins,
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
                &mut ParseContext::new(),
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
                        time_scale: _,
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
                        time_scale: _,
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
            time_scale: TimeScale::default(),
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

        if let Some(sdf_path) = ectx.sdf.as_deref() {
            timers.timed("sdf", |_| {
                let mut diagnostics = LowerDiagnostics::default();
                let error =
                    crate::timing::lower_sdf(&mut ctx, &mut mctx, sdf_path, &mut diagnostics)
                        .is_err();

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
                Result::<(), Box<dyn std::error::Error>>::Ok(())
            })?;
        }

        let mut error = false;
        let mut outs_lut = VgHashMap::default();
        let mut outs = Vec::new();
        let mut nba_signals = IndexMap::new();

        timers.start("lower_specify_blocks");
        for key in ctx.table.symbol_id_iter() {
            match &ctx.table[key].content {
                VSymbol::Module(i) => {
                    let module = module_lut[&i.module];
                    ctx.time_scale = module.time_scale;
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
                }
                _ => {}
            }
        }
        timers.stop();

        // @TODO: Iterate over the modules instead.
        timers.start("lower_global_items");
        for key in ctx.table.symbol_id_iter() {
            match &ctx.table[key].content {
                VSymbol::Module(i) => {
                    let module = module_lut[&i.module];
                    ctx.time_scale = module.time_scale;

                    error |= vogls_verilog::lower::instantiate_nba_signals(
                        &mut mctx.gl,
                        &mut ctx,
                        key,
                        module,
                        &mut mctx.diagnostics,
                        &mut nba_signals,
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
                    let (_, ms) = determine_module_context(key, &ctx.table);
                    ctx.time_scale = ms.time_scale;
                    let task_decl = ctx.table_ast_refs.tasks[i.ast_id];
                    error |= vogls_verilog::lower::module_or_generate_item::function::lower_task(
                        &mut ctx, &mut mctx, key, task_decl,
                    )
                    .is_err();
                }
                _ => {}
            }
        }
        for (sid, (signal, needs_mask)) in nba_signals.into_iter() {
            let (process, nba, mask) = create_nba_process(mctx.gl(), signal, needs_mask);
            let VSymbol::Net(net) = &mut ctx.table[sid].content else {
                unreachable!();
            };
            net.net.nba = Some((process, nba, mask));
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

        timers.start("lower");
        for key in ctx.table.symbol_id_iter() {
            let VSymbol::Module(m) = &ctx.table[key].content else {
                continue;
            };
            let module_id = module_lut[&m.module];
            ctx.time_scale = module_id.time_scale;
            let module_key = lower_module_to_ir(module_id, &ctx, &mut mctx, key);
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

        let (prb_fuse, drv_fuse) = timers.timed("fuse_signals", |_| {
            vogls_fuse_signals::fuse_signals(&mut mctx.gl, &mctx.connections)
        });

        let mut table: FrozenSymbolTable<Symbol> = ctx.table.into();
        for symbol in table.symbol_id_iter() {
            if let Symbol::Net(net) = &mut table[symbol].content {
                match &mut net.net {
                    NetValue::Signal(s) => {
                        let prb = s.probe_signal().0;
                        if let Some(FuseTarget::Constant(value)) = prb_fuse.get(&prb) {
                            if prb_fuse.contains_key(&prb) {
                                mctx.gl.signals.remove(prb);
                            }
                            net.net = NetValue::Constant(value.clone());
                        } else {
                            s.map_prb(|s| match prb_fuse.get(&s) {
                                None => (s, None),
                                Some(FuseTarget::Constant(_)) => unreachable!(),
                                Some(FuseTarget::Signal(r, slice)) => {
                                    mctx.gl.signals.remove(s);
                                    (*r, slice.map(|s| NonMaxU32::new(s.lsb()).unwrap()))
                                }
                            });
                            s.map_drv(|s| match drv_fuse.get(&s) {
                                None => (s, None),
                                Some((r, slice)) => {
                                    (*r, slice.map(|s| NonMaxU32::new(s.lsb()).unwrap()))
                                }
                            });
                        }
                    }
                    NetValue::Constant(_) => unreachable!(),
                }
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

        if ectx.emit_process_stats {
            let mut counts = [0u64; ProcessKind::NUM_KINDS];
            for process in mctx.gl.processes.values() {
                counts[process.kind as usize] += 1;
            }

            writeln!(ectx.stdout, "Process Kind Counts:")?;
            for (kind, count) in ProcessKind::KINDS.into_iter().zip(counts) {
                if count == 0 {
                    continue;
                }
                writeln!(ectx.stdout, "  {}: {}", kind.into_static_str(), count)?;
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

        timers.start("find lupdt signals");
        let mut lupdt_indexes = VgHashMap::<RtSignalKey, u64>::default();
        find_lupdt_signals(&mctx.gl, &rt_signal_map, &mut lupdt_indexes);
        timers.stop();

        let signal_to_heap: Arc<[HeapRef]> = signal_to_heap.into();
        if mctx.has_vcd || ectx.vcd.is_some() {
            let tlm = table.roots()[0];
            let scope = vcd_scope(&table, tlm, &ctx.arenas.ident_table);
            let (children, map) = vogls_vcd::VcdScope::lower(&scope, &rt_signal_map);
            let rtvcdoutput = match &ectx.vcd {
                Some(path) => {
                    vogls_vcd::RtVcdOutput::new_path(path, signal_to_heap.clone(), children, map)
                }
                None => vogls_vcd::RtVcdOutput::new(
                    Box::new(Vec::new()),
                    signal_to_heap.clone(),
                    Vec::new(),
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
                lupdt_indexes,
                num_regions,
                plugins,
                ctx.arenas.ident_table,
                table,
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
                lupdt_indexes,
                num_regions,
                plugins,
                ctx.arenas.ident_table,
                table,
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
        let mut lupdt_indexes = VgHashMap::default();
        timers.timed("generate heap", |_| {
            generate_signals_heap(
                &mut heap_builder,
                &mut rt_signal_map,
                &gl.signals,
                &mut signal_to_heap,
                gl.logic_mode,
            )
        });

        timers.timed("find lupdt signals", |_| {
            find_lupdt_signals(&gl, &rt_signal_map, &mut lupdt_indexes)
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
                lupdt_indexes,
                3,
                Vec::new(),
                IdentTable::default(),
                FrozenSymbolTable::default(),
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
                lupdt_indexes,
                3,
                Vec::new(),
                IdentTable::default(),
                FrozenSymbolTable::default(),
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
        lupdt_indexes: VgHashMap<RtSignalKey, u64>,
        num_regions: u8,
        plugins: Vec<RuntimePluginState>,
        ident_table: IdentTable,
        elab_table: FrozenSymbolTable<Symbol>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (initial_state, design) = lower_to_shared_object(
            &gl,
            &rt_signal_map,
            heap_builder,
            &signal_to_heap,
            lupdt_indexes,
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
        lupdt_indexes: VgHashMap<RtSignalKey, u64>,
        num_regions: u8,
        plugins: Vec<RuntimePluginState>,
        ident_table: IdentTable,
        elab_table: FrozenSymbolTable<Symbol>,
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
        let mut lupdt_updated = vec![false; lupdt_indexes.len()];

        for (key, signal) in &gl.signals {
            if let Some(initialize) = &signal.initialize {
                let rt_key = rt_signal_map[&key];
                assert_eq!(initialize.size(), signal.size);
                heap.store_bits(signal_to_heap[rt_key.as_usize()], gl.logic_mode, initialize);
                let is_unchanged = match gl.logic_mode {
                    LogicMode::TwoValue => initialize.count_zeros() == initialize.size().get(),
                    LogicMode::FourValue => initialize.count_unknown() == initialize.size().get(),
                };
                if !is_unchanged && let Some(lupdt_idx) = lupdt_indexes.get(&rt_key) {
                    lupdt_updated[*lupdt_idx as usize] = true;
                }
            }
        }

        let mut simulation = Simulation::new(
            processes,
            signal_to_heap.clone(),
            lupdt_indexes,
            gl.logic_mode,
        );
        simulation.itrace = itrace;
        let mut initial_state =
            simulation.new_state(regions, listeners, watches, heap, &lupdt_updated);
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

pub fn vcd_scope(
    symtable: &FrozenSymbolTable<Symbol>,
    scope: SymbolId,
    ident_table: &IdentTable,
) -> vogls_ir::vcd::VcdOutput {
    let mut key = scope;
    while let Some(parent) = symtable[key].parent() {
        key = parent;
    }

    let mut table = Table::new();
    let mut signal_map = VgHashMap::default();
    let mut scope = VcdScope {
        name: "".to_string(),
        items: Vec::new(),
    };
    extend_symbol_table_to_vcd_scope(
        &mut scope,
        &[key],
        symtable,
        ident_table,
        &mut table,
        &mut signal_map,
    );
    vogls_ir::vcd::VcdOutput {
        table,
        signal_map,
        children: scope.items,
    }
}

fn extend_symbol_table_to_vcd_scope(
    scope: &mut VcdScope,
    symbols: &[SymbolId],
    table: &FrozenSymbolTable<Symbol>,
    ident_table: &IdentTable,
    variable_table: &mut Table<VcdVariableKey, vogls_ir::vcd::VcdVariable>,
    signal_map: &mut VgHashMap<SignalKey, Vec<VcdVariableKey>>,
) {
    for sid in symbols.iter() {
        let name = &ident_table[table[*sid].name()];
        match &table[*sid].content {
            Symbol::Module | Symbol::Block | Symbol::GenerateBlocks => {
                let mut subscope = VcdScope {
                    name: name.to_string(),
                    items: Vec::new(),
                };
                extend_symbol_table_to_vcd_scope(
                    &mut subscope,
                    table[*sid].children(&table),
                    table,
                    ident_table,
                    variable_table,
                    signal_map,
                );
                scope
                    .items
                    .push(vogls_ir::vcd::VcdScopeItem::Scope(subscope));
            }
            Symbol::Net(i) => {
                let net = &i.net;

                // @TODO: Property implement this.
                let lsb = 0;
                let msb = i.ty.force_net_width().get() - 1;
                let msb_lsb = (msb > 0).then_some((msb, lsb));

                let (value, signal) = match net {
                    NetValue::Signal(net_signal) => {
                        let (signal, slice) = net_signal.probe_signal();
                        (VcdValue::Signal(signal, slice), Some(signal))
                    }
                    NetValue::Constant(bits) => (VcdValue::Constant(bits.clone()), None),
                };
                let variable_key = variable_table.insert(vogls_ir::vcd::VcdVariable {
                    name: ident_table[table[*sid].name()].to_string(),
                    value,
                    ty: vogls_ir::vcd::NetType::Wire,
                    msb_lsb,
                });
                scope
                    .items
                    .push(vogls_ir::vcd::VcdScopeItem::Variable(variable_key));
                if let Some(signal) = signal {
                    signal_map.entry(signal).or_default().push(variable_key);
                }
            }
            Symbol::Task | Symbol::Function | Symbol::Parameter(_) => {}
        }
    }
}
