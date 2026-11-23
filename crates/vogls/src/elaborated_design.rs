use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::{FrozenSymbolTable, SymbolId, SymbolTable};
use vogls_ir::time::TimeResolution;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{GlobalContext, LogicMode, ProcessKey, SignalFlags, SignalKey};
use vogls_utils::{IndexMap, NonMaxU32, VgHashMap};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{Module, ModuleItem, NonPortModuleItem, TimeScale};
use vogls_verilog::ast::udp::UdpDeclaration;
use vogls_verilog::elaborate::{SymbolAstRefs, VSymbol, VSymbolTable};
use vogls_verilog::lower::drivers::{DriverResolution, FinalizeOptions, FuseTarget};
use vogls_verilog::lower::for_each_module_item;
use vogls_verilog::lower::module_or_generate_item::{self, function};
use vogls_verilog::lower::{
    Diagnostics, LowerContext, LowerErrorReason, MutLowerContext, WarningsReport, observed::Scan,
};
use vogls_verilog::parser::{Ast, AstArenas, ReportKind, SpanError};
use vogls_verilog::tokenizer::Tokenized;

use crate::lowered_design::LowerErrorStage;
use crate::plugin::{VcdPlugin, VoglsPlugin};
use crate::symbol::{SignalValue, Symbol};
use crate::{LowerError, LoweredDesign, ParsedDesign};

#[derive(Clone)]
pub struct ElaboratedDesign<'a> {
    pub(crate) ast: Ast<'a>,
    pub(crate) token_buffer: Tokenized,
    pub(crate) arenas: AstArenas,

    pub(crate) logic_mode: LogicMode,
    pub(crate) module_lut: VgHashMap<IdentId, AstId<'a, Module<'a>>>,
    pub(crate) table: SymbolTable<VSymbol>,
    pub(crate) table_ast_refs: SymbolAstRefs<'a>,
    pub(crate) udps: VgHashMap<IdentId, AstId<'a, UdpDeclaration<'a>>>,
    pub(crate) gl: GlobalContext,
    pub(crate) nbas: IndexMap<SignalKey, (ProcessKey, SignalKey, Option<SignalKey>)>,
    pub(crate) driver_resolution: DriverResolution<'a>,

    pub(crate) warnings: Vec<(TokenRange, String)>,

    pub(crate) unoptimized_fgs: Option<Arc<Mutex<dyn std::io::Write + Send + Sync>>>,
    pub(crate) optimized_fgs: Option<Arc<Mutex<dyn std::io::Write + Send + Sync>>>,
    /// Maximum number of rounds to simplify a subgraph of the drive graph.
    pub drive_graph_rounds: usize,
    /// Whether the drive graph is optimized at all, see [`FinalizeOptions::optimize`].
    pub drive_graph_opt: bool,
    pub time_resolution: TimeResolution,
    /// Path to dump a VCD trace of the whole design to, see [`ElaboratedDesign::trace_vcd`].
    pub(crate) vcd: Option<PathBuf>,
    /// Whether the design calls `$dumpfile` or `$dumpvars`, found during elaboration.
    pub(crate) has_vcd: bool,
}

pub struct ElaborationError<'a> {
    pub(crate) design: ParsedDesign<'a>,
    pub(crate) kind: ElaborationErrorKind<'a>,
}

pub enum ElaborationErrorKind<'a> {
    CannotFindTopLevelModule(String),
    AmbiguousTopLevelModule(Vec<(AstId<'a, Module<'a>>, IdentId)>),
    Diagnostics(Diagnostics),
}

impl<'a> fmt::Display for ElaborationError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // @TODO: Improve errors
        match &self.kind {
            ElaborationErrorKind::CannotFindTopLevelModule(module_name) => {
                write!(f, "cannot find top-level module '{module_name}'",)
            }
            ElaborationErrorKind::AmbiguousTopLevelModule(modules) => {
                let names = modules
                    .iter()
                    .map(|(_, n)| &self.design.ident_table()[*n])
                    .collect::<Vec<&str>>();
                writeln!(
                    f,
                    "[ERR]: Found {} possible top-level modules: {names:?}",
                    modules.len()
                )?;
                let t = &self.design.token_buffer;
                for (m, n) in modules {
                    let location = self.design.arenas.get_item_span(m.module_identifier);
                    SpanError {
                        spans: &t.spans,
                        file_idxs: &t.file_idxs,
                        paths: &t.paths,
                        contents: &t.contents,
                        error: format!(
                            "possible top-level module `{}`",
                            &self.design.arenas.ident_table[*n]
                        ),
                        kind: ReportKind::Info,
                        code: None,
                        location,
                    }
                    .fmt(f)?;
                    writeln!(f)?;
                }
                Ok(())
            }
            ElaborationErrorKind::Diagnostics(diagnostics) => diagnostics
                .report(&self.design.token_buffer, &self.design.arenas)
                .fmt(f),
        }
    }
}
impl<'a> fmt::Debug for ElaborationError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ElaborationErrorKind::CannotFindTopLevelModule(_) => {
                f.write_str("ElaborationError::CannotFindTopLevelModule")
            }
            ElaborationErrorKind::AmbiguousTopLevelModule(_) => {
                f.write_str("ElaborationError::AmbiguousTopLevelModule")
            }
            ElaborationErrorKind::Diagnostics(_) => f.write_str("ElaborationError::Diagnostics"),
        }
    }
}
impl<'a> std::error::Error for ElaborationError<'a> {}

pub struct AnnotationError<'a, 'b> {
    design: &'b ElaboratedDesign<'a>,
    diagnostics: Diagnostics,
}

impl<'a, 'b> fmt::Display for AnnotationError<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.diagnostics
            .report(&self.design.token_buffer, &self.design.arenas)
            .fmt(f)
    }
}
impl<'a, 'b> fmt::Debug for AnnotationError<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnnotationError")
            .field("num_errors", &self.diagnostics.errors.len())
            .field("num_warnings", &self.diagnostics.warnings.len())
            .finish_non_exhaustive()
    }
}
impl<'a, 'b> std::error::Error for AnnotationError<'a, 'b> {}

impl<'a> ElaboratedDesign<'a> {
    #[cfg(feature = "unstable")]
    pub fn set_unoptimized_fuse_graphs_emit(
        &mut self,
        writer: Arc<Mutex<dyn std::io::Write + Send + Sync>>,
    ) -> &mut Self {
        self.unoptimized_fgs = Some(writer);
        self
    }
    #[cfg(feature = "unstable")]
    pub fn set_optimized_fuse_graphs_emit(
        &mut self,
        writer: Arc<Mutex<dyn std::io::Write + Send + Sync>>,
    ) -> &mut Self {
        self.optimized_fgs = Some(writer);
        self
    }

    pub fn get_signal_handle(&mut self, symbol: SymbolId) -> Option<SignalHandle> {
        let VSymbol::Signal(net) = &self.table[symbol].content else {
            return None;
        };

        let signal = net.terminals.probe_signal();
        self.gl.signals[signal].flags |= SignalFlags::EXT_DRIVE | SignalFlags::EXT_PROBE;
        Some(SignalHandle { symbol })
    }

    /// Like [`ElaboratedDesign::get_signal_handle`], but for a signal that is only read and never
    /// written from outside the design, so the drive graph does not need to account for external
    /// drivers.
    pub fn get_probe_handle(&mut self, symbol: SymbolId) -> Option<SignalHandle> {
        let VSymbol::Signal(net) = &self.table[symbol].content else {
            return None;
        };

        let signal = net.terminals.probe_signal();
        self.gl.signals[signal].flags |= SignalFlags::EXT_PROBE;
        Some(SignalHandle { symbol })
    }

    pub fn table(&self) -> &VSymbolTable {
        &self.table
    }

    pub fn display_hierarchy(&self) -> impl fmt::Display {
        struct DisplayHierarchy<'a, 'b>(&'b ElaboratedDesign<'a>);
        impl<'a, 'b> fmt::Display for DisplayHierarchy<'a, 'b> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                for root in self.0.table().roots() {
                    writeln!(
                        f,
                        "{}",
                        self.0
                            .table()
                            .display(*root, &self.0.arenas.ident_table, |s, f| {
                                match s {
                                    VSymbol::Module(_) => f.write_str("mod"),
                                    VSymbol::Parameter(v) => {
                                        if v.ty().is_signed() {
                                            f.write_str("signed ")?;
                                        }
                                        write!(f, "{}", v.clone().into_bits())?;
                                        Ok(())
                                    }
                                    VSymbol::Signal(s) => {
                                        f.write_str("net")?;
                                        if s.ty.is_signed() {
                                            f.write_str(" signed")?;
                                        }
                                        if s.ty.bit_length().get() > 1 {
                                            write!(f, "[{}]", s.ty.bit_length().get())?;
                                        }
                                        Ok(())
                                    }
                                    VSymbol::NamedBlock => f.write_str("named block"),
                                    VSymbol::GenerateBlock(_) => f.write_str("generate block"),
                                    VSymbol::GenerateBlocks => f.write_str("generate blocks"),
                                    VSymbol::ModuleRange(_) => f.write_str("modules"),
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
        DisplayHierarchy(self)
    }

    fn with_context<T>(
        &mut self,
        mut f: impl FnMut(
            &mut LowerContext<'a, '_>,
            &mut MutLowerContext<'a>,
            &VgHashMap<IdentId, AstId<'a, Module<'a>>>,
        ) -> T,
    ) -> T {
        // This is not panic safe, so maybe we should add a unwind catch here?

        let mut ctx = LowerContext {
            logic_mode: self.logic_mode,
            table: std::mem::take(&mut self.table),
            table_ast_refs: std::mem::take(&mut self.table_ast_refs),
            udps: std::mem::take(&mut self.udps),
            arenas: &self.arenas,
            tokenized: &self.token_buffer,
            time_scale: TimeScale::default(),
            time_resolution: self.time_resolution,
        };
        let mut mctx = MutLowerContext {
            gl: std::mem::take(&mut self.gl),
            nbas: std::mem::take(&mut self.nbas),
            diagnostics: Diagnostics::default(),
            fuse_dsts_scratch: Vec::new(),
            fuse_srcs_scratch: Vec::new(),
            monitor: None,
            driver_resolution: std::mem::take(&mut self.driver_resolution),
        };

        let result = f(&mut ctx, &mut mctx, &self.module_lut);
        self.table = ctx.table;
        self.table_ast_refs = ctx.table_ast_refs;
        self.udps = ctx.udps;
        self.gl = mctx.gl;
        self.nbas = mctx.nbas;
        self.warnings = mctx.diagnostics.warnings;
        self.driver_resolution = mctx.driver_resolution;
        result
    }

    pub fn report_warnings(&self) -> WarningsReport<'_> {
        WarningsReport::new(&self.warnings, &self.token_buffer)
    }

    pub fn annotate_sdf<'b>(
        &'b mut self,
        path: impl AsRef<Path>,
        scope: Option<&str>,
    ) -> Result<&'b mut Self, AnnotationError<'a, 'b>> {
        let path = path.as_ref();
        let mut diagnostics = Diagnostics::default();
        let Ok(content) = std::fs::read_to_string(path) else {
            diagnostics.not_yet_implemented(Default::default(), "SDF file cannot be opened");
            return Err(AnnotationError {
                design: self,
                diagnostics,
            });
        };

        let mut tkw = vogls_sdf::TokenWalker::new(&content, &mut self.arenas.ident_table);
        let Ok(sdf) =
            <vogls_sdf::DelayFile as vogls_sdf::Consume>::consume(&mut tkw).map_err(|err| {
                diagnostics.errors.push((
                    TokenRange::default(),
                    LowerErrorReason::SdfParse(path.display().to_string(), err.to_string()),
                    Vec::new(),
                ));
            })
        else {
            return Err(AnnotationError {
                design: self,
                diagnostics,
            });
        };

        match self.with_context(|ctx, mctx, _| {
            let scope = match scope {
                None => None,
                Some(scope) => {
                    match crate::find_symbol_from_path(&ctx.table, &ctx.arenas.ident_table, scope) {
                        Some(sid) if matches!(ctx.table[sid].content, VSymbol::Module(_)) => {
                            Some(sid)
                        }
                        Some(_) | None => {
                            mctx.diagnostics.errors.push((
                                TokenRange::default(),
                                LowerErrorReason::SdfUnknownScope(scope.to_string()),
                                Vec::new(),
                            ));
                            return Err(std::mem::take(&mut mctx.diagnostics));
                        }
                    }
                }
            };
            match crate::timing::lower_sdf(ctx, mctx, &sdf, scope) {
                Ok(_) => Ok(()),
                Err(_) => Err(std::mem::take(&mut mctx.diagnostics)),
            }
        }) {
            Ok(_) => Ok(self),
            Err(diagnostics) => Err(AnnotationError {
                design: self,
                diagnostics,
            }),
        }
    }

    pub fn annotate_specify<'b>(&'b mut self) -> Result<&'b mut Self, AnnotationError<'a, 'b>> {
        let result = self.with_context(|ctx, mctx, module_lut| {
            let mut error = false;
            for key in ctx.table.symbol_id_iter() {
                if let VSymbol::Module(i) = &ctx.table[key].content {
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
                            )
                            .is_err();
                        }
                    }
                }
            }

            if error {
                return Err(std::mem::take(&mut mctx.diagnostics));
            }

            Ok(())
        });
        match result {
            Ok(_) => Ok(self),
            Err(diagnostics) => Err(AnnotationError {
                design: self,
                diagnostics,
            }),
        }
    }

    /// Dump a VCD trace of the whole design to `vcd`.
    ///
    /// The VCD output is added as a plugin at the start of [`ElaboratedDesign::lower`], after
    /// the user-supplied plugins. This also happens without a path when the design calls
    /// `$dumpfile` or `$dumpvars`.
    pub fn trace_vcd(&mut self, vcd: impl Into<PathBuf>) -> &mut Self {
        self.vcd = Some(vcd.into());
        self
    }

    pub fn lower(
        mut self,
        mut plugins: Vec<Box<dyn VoglsPlugin>>,
    ) -> Result<LoweredDesign, Box<LowerError<'a>>> {
        // Appended last so the indices of the user-supplied plugins are preserved.
        if self.vcd.is_some() || self.has_vcd {
            plugins.push(Box::new(VcdPlugin::new(self.vcd.clone())));
        }
        for plugin in &mut plugins {
            plugin.register_handles(&mut self);
        }

        let Self {
            module_lut,
            table,
            table_ast_refs,
            udps,
            gl,
            nbas,
            driver_resolution,

            logic_mode,

            ast,
            token_buffer,
            arenas,

            unoptimized_fgs,
            optimized_fgs,

            drive_graph_rounds,
            drive_graph_opt,
            time_resolution,
            vcd,
            has_vcd,

            warnings: _,
        } = self;

        let mut ctx = LowerContext {
            logic_mode,
            table,
            table_ast_refs,
            udps,
            arenas: &arenas,
            tokenized: &token_buffer,
            time_scale: TimeScale::default(),
            time_resolution,
        };
        let mut mctx = MutLowerContext {
            gl,
            nbas,
            diagnostics: Diagnostics::default(),
            fuse_dsts_scratch: Vec::new(),
            fuse_srcs_scratch: Vec::new(),
            monitor: None,
            driver_resolution,
        };

        let mut error = false;
        let modules = ctx
            .table
            .symbol_id_iter()
            .filter_map(|key| match &ctx.table[key].content {
                VSymbol::Module(m) => Some((key, module_lut[&m.module])),
                _ => None,
            })
            .collect::<Vec<_>>();

        let mut scan = Scan::default();
        for &(key, module_id) in &modules {
            ctx.time_scale = module_id.time_scale;
            error |= for_each_module_item(module_id, &ctx, key, |scope, id| {
                module_or_generate_item::lower_structural(&ctx, &mut mctx, scope, &mut scan, id)
            })
            .is_err();
        }
        function::observe_all(&ctx, &mut mctx);

        let mut resolved = None;
        if !error {
            let mut unoptimized_fgs = unoptimized_fgs.as_ref().map(|w| w.lock().unwrap());
            let mut optimized_fgs = optimized_fgs.as_ref().map(|w| w.lock().unwrap());
            let options = FinalizeOptions {
                max_rounds: drive_graph_rounds,
                optimize: drive_graph_opt,
                unoptimized_dot: unoptimized_fgs
                    .as_deref_mut()
                    .map(|w| w as &mut dyn std::io::Write),
                optimized_dot: optimized_fgs
                    .as_deref_mut()
                    .map(|w| w as &mut dyn std::io::Write),
            };
            match std::mem::take(&mut mctx.driver_resolution).finalize(&mut ctx, &mut mctx, options)
            {
                Ok(r) => resolved = Some(r),
                Err(()) => error = true,
            }
        }

        // Functions and tasks, and so the procedural code that calls them, are lowered also when
        // the drivers could not be resolved, so that their errors are reported too.
        function::discard_all(&mut ctx, &mut mctx.gl);
        if function::lower_all(&mut ctx, &mut mctx).is_err() {
            let LowerContext {
                logic_mode,
                table,
                table_ast_refs,
                udps,
                arenas: _,
                tokenized: _,
                time_scale: _,
                time_resolution,
            } = ctx;
            return Err(Box::new(LowerError {
                design: Self {
                    logic_mode,
                    ast,
                    token_buffer,
                    arenas,
                    module_lut,
                    table,
                    table_ast_refs,
                    udps,
                    gl: mctx.gl,
                    nbas: mctx.nbas,
                    driver_resolution: mctx.driver_resolution,

                    unoptimized_fgs,
                    optimized_fgs,

                    drive_graph_rounds,
                    drive_graph_opt,
                    time_resolution,
                    vcd,
                    has_vcd,

                    warnings: Vec::new(),
                },
                diagnostics: mctx.diagnostics,
                stage: LowerErrorStage::GlobalItems,
            }));
        }

        for &(key, module_id) in &modules {
            ctx.time_scale = module_id.time_scale;
            error |= for_each_module_item(module_id, &ctx, key, |scope, id| {
                module_or_generate_item::lower_procedural(&ctx, &mut mctx, scope, id)
            })
            .is_err();
        }

        if error {
            let LowerContext {
                logic_mode,
                table,
                table_ast_refs,
                udps,
                arenas: _,
                tokenized: _,
                time_scale: _,
                time_resolution,
            } = ctx;
            return Err(Box::new(LowerError {
                design: Self {
                    logic_mode,
                    ast,
                    token_buffer,
                    arenas,
                    module_lut,
                    table,
                    table_ast_refs,
                    udps,
                    gl: mctx.gl,
                    nbas: mctx.nbas,
                    driver_resolution: mctx.driver_resolution,

                    unoptimized_fgs,
                    optimized_fgs,

                    drive_graph_rounds,
                    drive_graph_opt,
                    time_resolution,
                    vcd,
                    has_vcd,

                    warnings: Vec::new(),
                },
                diagnostics: mctx.diagnostics,
                stage: LowerErrorStage::Modules,
            }));
        }

        let (prb_fuse, write_fuse) = resolved
            .expect("drivers are resolved without errors")
            .apply(&mut mctx.gl);

        let mut table: FrozenSymbolTable<Symbol> = ctx.table.into();
        for symbol in table.symbol_id_iter() {
            if let Symbol::Signal(sym) = &mut table[symbol].content {
                match &mut sym.value {
                    SignalValue::Signal(s) => {
                        let prb = s.probe_signal().0;
                        if let Some(FuseTarget::Constant(value)) = prb_fuse.get(&prb) {
                            if prb_fuse.contains_key(&prb) {
                                mctx.gl.signals.remove(prb);
                            }
                            sym.value = SignalValue::Constant(value.clone());
                        } else {
                            // Drives are mapped first: `map_drv` starts from the unmapped probe
                            // signal. The writes of a net connected by a `tran` go to the value
                            // that the net drives into it, see `WriteFuseMap`.
                            s.map_drv(|s| match write_fuse.get(&s) {
                                None => (s, None),
                                Some(&(r, slice)) => {
                                    (r, Some(NonMaxU32::new(slice.lsb()).unwrap()))
                                }
                            });
                            s.map_prb(|s| match prb_fuse.get(&s) {
                                None => (s, None),
                                Some(FuseTarget::Constant(_)) => unreachable!(),
                                Some(FuseTarget::Signal(r, slice)) => {
                                    mctx.gl.signals.remove(s);
                                    (*r, slice.map(|s| NonMaxU32::new(s.lsb()).unwrap()))
                                }
                            });
                        }
                    }
                    SignalValue::Constant(_) => unreachable!(),
                }
            }
        }

        Ok(LoweredDesign {
            table,
            gl: mctx.gl,
            warnings: mctx.diagnostics.warnings,
            plugins,
            ident_table: arenas.ident_table,
            token_buffer,
            itrace: false,
            emit_vm: false,
            stats: false,
            debug_symbols: false,
            output_source: None,
            print_vm_map: false,
            emit_clif: None,
            emit_disassembly: None,
            profile: None,
            time_resolution,
        })
    }

    pub fn ident_table(&self) -> &IdentTable {
        &self.arenas.ident_table
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalHandle {
    pub(crate) symbol: SymbolId,
}
