use std::fmt;
use std::path::Path;

use vogls_frontend::ident_table::IdentId;
use vogls_frontend::symbol_table::{FrozenSymbolTable, SymbolId, SymbolTable};
use vogls_fuse_signals::{FuseGraph, FuseGraphOptimizer, FuseTarget};
use vogls_ir::{GlobalContext, SignalFlags};
use vogls_utils::{IndexMap, NonMaxU32, VgHashMap};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{Module, ModuleItem, NonPortModuleItem, TimeScale};
use vogls_verilog::ast::udp::UdpDeclaration;
use vogls_verilog::elaborate::{SymbolAstRefs, VSymbol, VSymbolTable, determine_module_context};
use vogls_verilog::lower::{
    Diagnostics, LowerContext, MutLowerContext, create_nba_process, lower_module_to_ir,
};
use vogls_verilog::parser::{Ast, AstArenas, report};
use vogls_verilog::tokenizer::Tokenized;

use crate::lowered_design::LowerErrorStage;
use crate::plugin::VoglsPlugin;
use crate::symbol::{NetValue, Symbol};
use crate::{LowerError, LoweredDesign, ParsedDesign};

pub struct ElaboratedDesign<'a> {
    pub(crate) ast: Ast<'a>,
    pub(crate) token_buffer: Tokenized,
    pub(crate) arenas: AstArenas,

    pub(crate) module_lut: VgHashMap<IdentId, AstId<'a, Module<'a>>>,
    pub(crate) table: SymbolTable<VSymbol>,
    pub(crate) table_ast_refs: SymbolAstRefs<'a>,
    pub(crate) udps: VgHashMap<IdentId, AstId<'a, UdpDeclaration<'a>>>,
    pub(crate) gl: GlobalContext,

    pub(crate) unoptimized_fgs: Option<Box<dyn std::io::Write>>,
    pub(crate) optimized_fgs: Option<Box<dyn std::io::Write>>,
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
                let mut out = String::new();
                for (m, _) in modules {
                    out.clear();
                    let span = self.design.arenas.get_item_span(m.module_identifier);
                    report(&self.design.token_buffer, span, &mut out)?;
                    eprintln!("{out}");
                }
                Ok(())
            }
            ElaborationErrorKind::Diagnostics(diagnostics) => {
                diagnostics.report(&self.design.token_buffer).fmt(f)
            }
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
        self.diagnostics.report(&self.design.token_buffer).fmt(f)
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
        writer: Box<dyn std::io::Write>,
    ) -> &mut Self {
        self.unoptimized_fgs = Some(writer);
        self
    }
    #[cfg(feature = "unstable")]
    pub fn set_optimized_fuse_graphs_emit(&mut self, writer: Box<dyn std::io::Write>) -> &mut Self {
        self.optimized_fgs = Some(writer);
        self
    }

    pub fn get_signal_handle(&mut self, symbol: SymbolId) -> Option<SignalHandle> {
        let VSymbol::Net(net) = &self.table[symbol].content else {
            return None;
        };

        let signal = net.net.probe_signal();
        self.gl.signals[signal].flags |= SignalFlags::EXT_DRIVE | SignalFlags::EXT_PROBE;
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
        DisplayHierarchy(self)
    }

    fn with_context<T>(
        &mut self,
        mut f: impl FnMut(
            &mut LowerContext<'a, '_>,
            &mut MutLowerContext,
            &VgHashMap<IdentId, AstId<'a, Module<'a>>>,
        ) -> T,
    ) -> T {
        // This is not panic safe, so maybe we should add a unwind catch here?

        let mut ctx = LowerContext {
            table: std::mem::take(&mut self.table),
            table_ast_refs: std::mem::take(&mut self.table_ast_refs),
            udps: std::mem::take(&mut self.udps),
            arenas: &self.arenas,
            tokenized: &self.token_buffer,
            time_scale: TimeScale::default(),
        };
        let mut mctx = MutLowerContext {
            gl: std::mem::take(&mut self.gl),
            diagnostics: Diagnostics::default(),
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

    pub fn annotate_sdf<'b>(
        &'b mut self,
        path: impl AsRef<Path>,
    ) -> Result<&'b mut Self, AnnotationError<'a, 'b>> {
        match self.with_context(|ctx, mctx, _| {
            match crate::timing::lower_sdf(ctx, mctx, path.as_ref()) {
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
        });
        match result {
            Ok(_) => Ok(self),
            Err(diagnostics) => Err(AnnotationError {
                design: self,
                diagnostics,
            }),
        }
    }

    pub fn lower(
        self,
        plugins: Vec<Box<dyn VoglsPlugin>>,
    ) -> Result<LoweredDesign, LowerError<'a>> {
        let Self {
            module_lut,
            table,
            table_ast_refs,
            udps,
            gl,

            ast,
            token_buffer,
            arenas,

            mut unoptimized_fgs,
            mut optimized_fgs,
        } = self;

        let mut ctx = LowerContext {
            table,
            table_ast_refs,
            udps,
            arenas: &arenas,
            tokenized: &token_buffer,
            time_scale: TimeScale::default(),
        };
        let mut mctx = MutLowerContext {
            gl,
            diagnostics: Diagnostics::default(),
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
            let LowerContext {
                table,
                table_ast_refs,
                udps,
                arenas: _,
                tokenized: _,
                time_scale: _,
            } = ctx;
            return Err(LowerError {
                design: Self {
                    ast,
                    token_buffer,
                    arenas,
                    module_lut,
                    table,
                    table_ast_refs,
                    udps,
                    gl: mctx.gl,

                    unoptimized_fgs,
                    optimized_fgs,
                },
                diagnostics: mctx.diagnostics,
                stage: LowerErrorStage::GlobalItems,
            });
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
            let LowerContext {
                table,
                table_ast_refs,
                udps,
                arenas: _,
                tokenized: _,
                time_scale: _,
            } = ctx;
            return Err(LowerError {
                design: Self {
                    ast,
                    token_buffer,
                    arenas,
                    module_lut,
                    table,
                    table_ast_refs,
                    udps,
                    gl: mctx.gl,

                    unoptimized_fgs,
                    optimized_fgs,
                },
                diagnostics: mctx.diagnostics,
                stage: LowerErrorStage::Modules,
            });
        }

        let mut opt =
            FuseGraphOptimizer::new(FuseGraph::from_connections(&mut mctx.gl, &mctx.connections));
        if let Some(unoptimized_fgs) = unoptimized_fgs.as_mut() {
            writeln!(
                unoptimized_fgs,
                "{}",
                opt.graph().display_dot(&mctx.gl.signals)
            ).unwrap();
        }
        opt.optimize();
        if let Some(optimized_fgs) = optimized_fgs.as_mut() {
            writeln!(
                optimized_fgs,
                "{}",
                opt.graph().display_dot(&mctx.gl.signals)
            ).unwrap();
        }
        let (prb_fuse, drv_fuse) = opt.finalize(mctx.gl());

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
            ident_table: arenas.ident_table,
            token_buffer: token_buffer,
            itrace: false,
            emit_vm: false,
            stats: false,
            debug_symbols: false,
            output_source: None,
            print_vm_map: false,
        })
    }
}

#[derive(Clone)]
pub struct SignalHandle {
    pub(crate) symbol: SymbolId,
}
