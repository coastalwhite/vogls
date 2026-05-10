use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use slotmap::SlotMap;
use vogls_codegen::{HeapBuilder, HeapRef};
use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::{FrozenSymbolTable, SymbolId, SymbolTable};
use vogls_ir::optimize::OptFlags;
use vogls_ir::vcd::{VcdScope, VcdValue, VcdVariableKey};
use vogls_ir::{Bits, GlobalContext, LogicMode, ProcessKind, SignalKey};
use vogls_runtime::SimulationIo;
use vogls_runtime::plugins::RuntimePluginState;
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_sim::{Event, Regions, Simulation, VmProcess, VmProcessKey, lower_process_to_vm};
use vogls_utils::{IndexMap, NonMaxU32, Table, TimerStack, VgHashMap};
pub use vogls_verilog::arena::Arena;
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{Description, Module, ModuleItem, NonPortModuleItem, TimeScale};
use vogls_verilog::ast::udp::UdpDeclaration;
use vogls_verilog::elaborate::{SymbolAstRefs, VSymbol, VSymbolTable, determine_module_context};
use vogls_verilog::lower::{
    Diagnostics as LowerDiagnostics, LowerContext, MutLowerContext, create_nba_process,
    lower_module_to_ir,
};
use vogls_verilog::parser::{
    Ast, AstArenas, Diagnostics as ParserDiagnostics, ParseContext, ParserScratches, TokenWalker,
    parse_file, report, report_error,
};
use vogls_verilog::tokenizer::{Macro, Tokenized};

use crate::symbol::{NetValue, Symbol};
use crate::{
    ExecutionContext, append_referenced_modules, find_lupdt_signals, generate_signals_heap,
};
use vogls_fuse_signals::FuseTarget;

pub enum DesignBackend {
    Interpretted {
        simulation: vogls_sim::Simulation,
    },
    #[cfg(feature = "native")]
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
    #[cfg(feature = "native")]
    Compiled(vogls_codegen_c::runtime::CDesignState),
}

impl DesignState {
    pub fn runtime_mut(&mut self) -> &mut RuntimeState {
        match self {
            DesignState::Interpretted(s) => &mut s.runtime,
            #[cfg(feature = "native")]
            DesignState::Compiled(s) => &mut s.runtime,
        }
    }
    pub fn runtime(&self) -> &RuntimeState {
        match self {
            DesignState::Interpretted(s) => &s.runtime,
            #[cfg(feature = "native")]
            DesignState::Compiled(s) => &s.runtime,
        }
    }
    pub fn plugins_mut(&mut self) -> &mut [RuntimePluginState] {
        match self {
            DesignState::Interpretted(s) => &mut s.plugins,
            #[cfg(feature = "native")]
            DesignState::Compiled(s) => &mut s.plugins,
        }
    }
    pub fn plugins(&self) -> &[RuntimePluginState] {
        match self {
            DesignState::Interpretted(s) => &s.plugins,
            #[cfg(feature = "native")]
            DesignState::Compiled(s) => &s.plugins,
        }
    }
}

#[derive(Default)]
pub struct DesignBuilder {
    token_buffer: Tokenized,
    macros: HashMap<String, Macro>,
    timers: TimerStack,
}

pub struct ParsedDesign<'a> {
    ast: Ast<'a>,
    token_buffer: Tokenized,
    arenas: AstArenas,
    timers: TimerStack,
}

pub struct ElaboratedDesign<'a> {
    module_lut: VgHashMap<IdentId, AstId<'a, Module<'a>>>,
    table: SymbolTable<VSymbol>,
    table_ast_refs: SymbolAstRefs<'a>,
    udps: VgHashMap<IdentId, AstId<'a, UdpDeclaration<'a>>>,
    gl: GlobalContext,
}

pub struct LoweredDesign {
    table: FrozenSymbolTable<Symbol>,
    gl: GlobalContext,
    plugins: Vec<RuntimePluginState>,
    vcd: Option<PathBuf>,
    has_vcd: bool,

    itrace: bool,
    emit_vm: bool,
    stats: bool,
    debug_symbols: bool,
    output_source: Option<PathBuf>,
}

pub enum ElaborationError<'a> {
    CannotFindTopLevelModule,
    AmbiguousTopLevelModule(Vec<(AstId<'a, Module<'a>>, IdentId)>),
    Diagnostics(LowerDiagnostics),
}

pub enum LowerError {
    GlobalItems(LowerDiagnostics),
    Modules(LowerDiagnostics),
}

impl DesignBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_source(&mut self, path: impl AsRef<Path>) -> io::Result<&mut Self> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)?;
        self.add_source_str_with_name(source, path);
        Ok(self)
    }

    pub fn add_source_str(&mut self, source: impl Into<Rc<str>>) -> &mut Self {
        self.add_source_str_with_opt_name(source, <Option<Rc<Path>>>::None)
    }

    pub fn add_source_str_with_name(
        &mut self,
        source: impl Into<Rc<str>>,
        name: impl Into<Rc<Path>>,
    ) -> &mut Self {
        self.add_source_str_with_opt_name(source, Some(name));
        self
    }

    pub fn add_source_str_with_opt_name(
        &mut self,
        source: impl Into<Rc<str>>,
        name: Option<impl Into<Rc<Path>>>,
    ) -> &mut Self {
        self.token_buffer.append_tokenize_with_macros(
            source.into(),
            name.map(Into::into),
            &mut self.macros,
        );
        self
    }

    pub fn define_macro(&mut self, name: impl Into<String>, value: Macro) -> &mut Self {
        self.macros.insert(name.into(), value);
        self
    }

    pub fn parse<'a>(
        mut self,
        arena: &'a mut Arena,
    ) -> Result<ParsedDesign<'a>, (Self, ParserDiagnostics)> {
        let mut tkw = TokenWalker::new(&self.token_buffer);
        let mut arenas = AstArenas::default();

        let mut diagnostics = ParserDiagnostics::default();
        self.timers.start("parsing");
        let ast = parse_file(
            &mut tkw,
            &mut ParserScratches::default(),
            Some(&mut diagnostics),
            &mut arenas,
            arena,
            &mut ParseContext::new(),
        );
        self.timers.start("stop");
        let Ok(ast) = ast else {
            return Err((self, diagnostics));
        };

        Ok(ParsedDesign {
            ast,
            token_buffer: self.token_buffer,
            arenas,
            timers: self.timers,
        })
    }
}

impl<'a> ParsedDesign<'a> {
    pub fn infer_top_level_module(
        &'a self,
    ) -> Result<(AstId<'a, Module<'a>>, IdentId), Vec<(AstId<'a, Module<'a>>, IdentId)>> {
        let mut referenced = HashSet::new();
        for id in self.ast.descriptions {
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
                    append_referenced_modules(&self.arenas, *module_item, &mut referenced);
                }
            }
        }

        let mut top_level_modules = Vec::new();
        for id in self.ast.descriptions {
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

        if top_level_modules.len() == 1 {
            return Ok(top_level_modules[0]);
        }

        Err(top_level_modules)
    }

    pub fn elaborate(
        &'a self,
        mode: LogicMode,
        top_level_module: Option<&str>,
    ) -> Result<ElaboratedDesign<'a>, ElaborationError<'a>> {
        // @TODO: Verify that all modules are uniquely named.
        let module_lut = VgHashMap::<IdentId, AstId<Module>>::from_iter(
            self.ast.descriptions.iter().filter_map(|id| match &*id {
                Description::Module(id) => Some((id.module_identifier.item.0, *id)),
                Description::Udp(_) | Description::Config => None,
            }),
        );

        let top_level_module = match top_level_module {
            Some(name) => {
                let id = self
                    .arenas
                    .ident_table
                    .get(name)
                    .and_then(|name| module_lut.get(&name).copied());
                match id {
                    None => return Err(ElaborationError::CannotFindTopLevelModule),
                    Some(id) => id,
                }
            }
            None => match self.infer_top_level_module() {
                Ok((m, _)) => m,
                Err(top_level_modules) => {
                    return Err(ElaborationError::AmbiguousTopLevelModule(top_level_modules));
                }
            },
        };

        let mut ctx = LowerContext {
            table: VSymbolTable::default(),
            table_ast_refs: SymbolAstRefs::default(),
            udps: VgHashMap::default(),
            arenas: &self.arenas,
            tokenized: &self.token_buffer,
            time_scale: TimeScale::default(),
        };
        let mut mctx = MutLowerContext {
            gl: GlobalContext::default(),
            diagnostics: LowerDiagnostics::default(),
            connections: Vec::new(),
            fuse_scratch: Vec::new(),
            has_vcd: false,
        };
        mctx.gl.logic_mode = mode;
        let Ok(()) = vogls_verilog::elaborate::next::elaborate(
            &mut mctx.gl,
            &mut ctx,
            top_level_module,
            &module_lut,
            &mut mctx.diagnostics,
        ) else {
            return Err(ElaborationError::Diagnostics(mctx.diagnostics));
        };

        for description in self.ast.descriptions.iter() {
            let Description::Udp(udp_id) = &*description else {
                continue;
            };

            let udp_id = *udp_id;
            let ident = udp_id.identifier.item.0;

            ctx.udps.insert(ident, udp_id);
        }

        Ok(ElaboratedDesign {
            module_lut,
            table: ctx.table,
            table_ast_refs: ctx.table_ast_refs,
            udps: ctx.udps,
            gl: mctx.gl,
        })
    }
}

impl<'a> ElaboratedDesign<'a> {
    pub fn table(&self) -> &VSymbolTable {
        &self.table
    }

    pub fn display_hierarchy(&self, design: &'a ParsedDesign) -> impl fmt::Display {
        struct DisplayHierarchy<'a, 'b>(&'b ElaboratedDesign<'a>, &'b ParsedDesign<'a>);
        impl<'a, 'b> fmt::Display for DisplayHierarchy<'a, 'b> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                for root in self.0.table().roots() {
                    writeln!(
                        f,
                        "{}",
                        self.0
                            .table()
                            .display(*root, &self.1.arenas.ident_table, |s, f| {
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
                Ok(())
            }
        }
        DisplayHierarchy(self, design)
    }

    fn with_context<T>(
        &mut self,
        design: &'a ParsedDesign,
        mut f: impl FnMut(
            &mut LowerContext<'a>,
            &mut MutLowerContext,
            &VgHashMap<IdentId, AstId<'a, Module<'a>>>,
        ) -> T,
    ) -> T {
        // This is not panic safe, so maybe we should add a unwind catch here?

        let mut ctx = LowerContext {
            table: std::mem::take(&mut self.table),
            table_ast_refs: std::mem::take(&mut self.table_ast_refs),
            udps: std::mem::take(&mut self.udps),
            arenas: &design.arenas,
            tokenized: &design.token_buffer,
            time_scale: TimeScale::default(),
        };
        let mut mctx = MutLowerContext {
            gl: std::mem::take(&mut self.gl),
            diagnostics: LowerDiagnostics::default(),
            connections: Vec::new(),
            fuse_scratch: Vec::new(),
            has_vcd: false,
        };

        let result = f(&mut ctx, &mut mctx, &self.module_lut);
        self.table = ctx.table;
        self.table_ast_refs = ctx.table_ast_refs;
        self.udps = ctx.udps;
        self.gl = mctx.gl;
        result
    }

    pub fn annotate_sdf(
        &mut self,
        design: &'a ParsedDesign,
        path: impl AsRef<Path>,
    ) -> Result<&mut Self, LowerDiagnostics> {
        self.with_context(design, |ctx, mctx, _| {
            match crate::timing::lower_sdf(ctx, mctx, path.as_ref()) {
                Ok(_) => Ok(()),
                Err(_) => Err(std::mem::take(&mut mctx.diagnostics)),
            }
        })?;
        Ok(self)
    }

    pub fn annotate_specify(
        &mut self,
        design: &'a ParsedDesign,
    ) -> Result<&mut Self, LowerDiagnostics> {
        self.with_context(design, |ctx, mctx, module_lut| {
            let mut error = false;
            let mut outs_lut = VgHashMap::default();
            let mut outs = Vec::new();

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
                                    ctx,
                                    mctx,
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

            if error {
                return Err(std::mem::take(&mut mctx.diagnostics));
            }

            Ok(())
        })?;

        Ok(self)
    }

    pub fn lower(
        self,
        design: &'a ParsedDesign,
        plugins: Vec<RuntimePluginState>,
    ) -> Result<LoweredDesign, LowerError> {
        let Self {
            module_lut,
            table,
            table_ast_refs,
            udps,
            gl,
        } = self;

        let mut ctx = LowerContext {
            table,
            table_ast_refs,
            udps,
            arenas: &design.arenas,
            tokenized: &design.token_buffer,
            time_scale: TimeScale::default(),
        };
        let mut mctx = MutLowerContext {
            gl,
            diagnostics: LowerDiagnostics::default(),
            connections: Vec::new(),
            fuse_scratch: Vec::new(),
            has_vcd: false,
        };

        // @TODO: Iterate over the modules instead.
        let mut error = false;
        let mut nba_signals = IndexMap::new();
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

        if error {
            return Err(LowerError::GlobalItems(mctx.diagnostics));
        }

        for key in ctx.table.symbol_id_iter() {
            let VSymbol::Module(m) = &ctx.table[key].content else {
                continue;
            };
            let module_id = module_lut[&m.module];
            ctx.time_scale = module_id.time_scale;
            let module_key = lower_module_to_ir(module_id, &ctx, &mut mctx, key);
            error |= module_key.is_err();
        }

        if error {
            return Err(LowerError::Modules(mctx.diagnostics));
        }

        let (prb_fuse, drv_fuse) =
            vogls_fuse_signals::fuse_signals(&mut mctx.gl, &mctx.connections);

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

        Ok(LoweredDesign {
            table,
            gl: mctx.gl,
            plugins,
            vcd: None,
            has_vcd: mctx.has_vcd,
            itrace: false,
            emit_vm: false,
            stats: false,
            debug_symbols: false,
            output_source: None,
        })
    }
}

pub struct EmitDesignIr<'a>(&'a LoweredDesign);
struct CodegenPreparation {
    heap_builder: HeapBuilder,
    signal_to_heap: Arc<[HeapRef]>,
    rt_signal_map: VgHashMap<SignalKey, RtSignalKey>,
    lupdt_indexes: VgHashMap<RtSignalKey, u64>,
}

const NUM_REGIONS: u8 = 3;
impl LoweredDesign {
    pub fn optimize(&mut self, flags: OptFlags) -> &mut Self {
        let processes = self.gl.processes.keys().collect::<Vec<_>>();
        vogls_ir::optimize::optimize_processes(&mut self.gl, &processes, flags);
        self
    }

    pub fn emit_ir<'a>(&'a self) -> EmitDesignIr<'a> {
        EmitDesignIr(self)
    }

    fn prepare_codegen(&mut self, design: &ParsedDesign) -> CodegenPreparation {
        let mut heap_builder = HeapBuilder::new();
        let mut signal_to_heap = Vec::new();
        let mut rt_signal_map = VgHashMap::default();
        let mut lupdt_indexes = VgHashMap::<RtSignalKey, u64>::default();

        generate_signals_heap(
            &mut heap_builder,
            &mut rt_signal_map,
            &self.gl.signals,
            &mut signal_to_heap,
            self.gl.logic_mode,
        );
        find_lupdt_signals(&self.gl, &rt_signal_map, &mut lupdt_indexes);
        let signal_to_heap: Arc<[HeapRef]> = signal_to_heap.into();

        if self.has_vcd() || self.vcd.is_some() {
            let tlm = self.table.roots()[0];
            let scope = vcd_scope(&self.table, tlm, &design.arenas.ident_table);
            let (children, map) = vogls_vcd::VcdScope::lower(&scope, &rt_signal_map);
            let rtvcdoutput = match &self.vcd {
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
            self.plugins.push(Box::new(rtvcdoutput));
        }

        CodegenPreparation {
            heap_builder,
            signal_to_heap,
            rt_signal_map,
            lupdt_indexes,
        }
    }

    #[cfg(feature = "native")]
    pub fn compile(mut self, design: ParsedDesign) -> Result<Design, Box<dyn std::error::Error>> {
        let CodegenPreparation {
            heap_builder,
            signal_to_heap,
            rt_signal_map,
            lupdt_indexes,
        } = self.prepare_codegen(&design);

        Design::from_gl_compiled(
            self.gl,
            heap_builder,
            &mut TimerStack::default(),
            self.itrace,
            self.stats,
            self.debug_symbols,
            self.output_source.as_deref(),
            rt_signal_map,
            signal_to_heap,
            lupdt_indexes,
            NUM_REGIONS,
            self.plugins,
            design.arenas.ident_table,
            self.table,
        )
    }

    pub fn to_bytecode(
        mut self,
        design: ParsedDesign,
    ) -> Result<Design, Box<dyn std::error::Error>> {
        let CodegenPreparation {
            heap_builder,
            signal_to_heap,
            rt_signal_map,
            lupdt_indexes,
        } = self.prepare_codegen(&design);
        Design::from_gl_interpretted(
            self.gl,
            heap_builder,
            &mut TimerStack::default(),
            self.itrace,
            self.emit_vm,
            rt_signal_map,
            signal_to_heap,
            lupdt_indexes,
            NUM_REGIONS,
            self.plugins,
            design.arenas.ident_table,
            self.table,
        )
    }

    fn has_vcd(&self) -> bool {
        self.has_vcd
    }

    fn trace_vcd(&mut self, vcd: PathBuf) -> &mut Self {
        self.vcd = Some(vcd);
        self
    }
}

impl<'a> fmt::Display for EmitDesignIr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for signal in self.0.gl.signals.values() {
            signal.display().fmt(f)?;
            writeln!(f)?;
        }
        writeln!(f)?;
        for process in self.0.gl.processes.values() {
            process.display(&self.0.gl).fmt(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Design {
    pub fn new(
        paths: &[&Path],
        timers: &mut TimerStack,
        top_level_module: Option<&str>,
        ectx: &mut ExecutionContext,
        plugins: Vec<RuntimePluginState>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut builder = DesignBuilder::new();
        if ectx.logic_mode == LogicMode::TwoValue {
            builder.define_macro("__VOGLS__TWO_VALUE_LOGIC", Macro::default());
        }
        for name in &ectx.defines {
            builder.define_macro(name, Macro::default());
        }
        timers.timed("tokenization", |_| {
            for path in paths {
                builder.add_source(path)?;
            }
            io::Result::Ok(())
        })?;

        let mut arena = Arena::default();
        let design = match builder.parse(&mut arena) {
            Ok(design) => design,
            Err((builder, diagnostics)) => {
                for (location, err) in &diagnostics.errors {
                    let mut out = String::new();
                    report_error(&builder.token_buffer, err.clone(), *location, &mut out)?;
                    write!(ectx.stderr, "{out}")?;
                }
                return Err("failed to parse".into());
            }
        };

        let mut elaborate = match design.elaborate(ectx.logic_mode, top_level_module) {
            Ok(v) => v,
            Err(err) => match err {
                ElaborationError::CannotFindTopLevelModule => {
                    return Err("cannot find top level module.".into());
                }
                ElaborationError::AmbiguousTopLevelModule(top_level_modules) => {
                    let names = top_level_modules
                        .iter()
                        .map(|(_, n)| &design.arenas.ident_table[*n])
                        .collect::<Vec<&str>>();
                    writeln!(
                        ectx.stderr,
                        "[ERR]: Found {} possible top-level modules: {names:?}",
                        top_level_modules.len()
                    )?;
                    let mut out = String::new();
                    for (m, _) in top_level_modules {
                        out.clear();
                        let span = design.arenas.get_item_span(m.module_identifier);
                        report(&design.token_buffer, span, &mut out)?;
                        writeln!(ectx.stderr, "{out}").unwrap();
                    }
                    return Err("ambiguous top-level module".into());
                }
                ElaborationError::Diagnostics(diagnostics) => {
                    writeln!(ectx.stderr, "{}", diagnostics.report(&design.token_buffer))?;
                    return Err("failed to elaborate".into());
                }
            },
        };

        if ectx.emit_hierarchy {
            writeln!(ectx.stdout, "{}", elaborate.display_hierarchy(&design))?;
        }

        if let Some(sdf_path) = ectx.sdf.as_deref() {
            if let Err(diagnostics) = elaborate.annotate_sdf(&design, sdf_path) {
                writeln!(ectx.stderr, "{}", diagnostics.report(&design.token_buffer))?;
                return Err("failed to annotate sdf".into());
            }
        }

        timers.start("lower_specify_blocks");
        if let Err(diagnostics) = elaborate.annotate_specify(&design) {
            writeln!(ectx.stderr, "{}", diagnostics.report(&design.token_buffer))?;
            return Err("failed to annotate specify".into());
        }
        timers.stop();

        let mut lowered = match elaborate.lower(&design, plugins) {
            Ok(v) => v,
            Err(err) => match err {
                LowerError::GlobalItems(diagnostics) => {
                    writeln!(ectx.stderr, "{}", diagnostics.report(&design.token_buffer))?;
                    return Err("failed to lower globals".into());
                }
                LowerError::Modules(diagnostics) => {
                    writeln!(ectx.stderr, "{}", diagnostics.report(&design.token_buffer))?;
                    return Err("failed to lower modules".into());
                }
            },
        };

        if ectx.emit_unoptimized_ir {
            writeln!(ectx.stdout, "{}", lowered.emit_ir())?;
        }

        timers.timed("optimization", |_| {
            lowered.optimize(ectx.opt);
        });

        if ectx.emit_ir {
            writeln!(ectx.stdout, "{}", lowered.emit_ir())?;
        }

        if ectx.emit_process_stats {
            let mut counts = [0u64; ProcessKind::NUM_KINDS];
            for process in lowered.gl.processes.values() {
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

        if let Some(vcd) = &ectx.vcd {
            lowered.trace_vcd(vcd.clone());
        }
        lowered.itrace = ectx.itrace;
        lowered.emit_vm = ectx.emit_vm;
        lowered.stats = ectx.stats;
        lowered.debug_symbols = ectx.debug_symbols;
        lowered.output_source = ectx.output_source.clone();

        if ectx.compile {
            #[cfg(feature = "native")]
            {
                lowered.compile(design)
            }

            #[cfg(not(feature = "native"))]
            {
                unreachable!()
            }
        } else {
            lowered.to_bytecode(design)
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
            #[cfg(feature = "native")]
            {
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
            }

            #[cfg(not(feature = "native"))]
            {
                unreachable!()
            }
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

    #[cfg(feature = "native")]
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
        let (initial_state, design) = crate::lower_to_shared_object(
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
            #[cfg(feature = "native")]
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
            #[cfg(feature = "native")]
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
                #[cfg(feature = "native")]
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
