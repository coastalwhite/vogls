mod assign;
mod diagnostics;
pub mod expression;
mod fuse;
pub mod module_or_generate_item;
pub mod specify;
mod statement;
pub mod udp;
mod vtype;
mod vvalue;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    Active = 0,
    Inactive = 1,
    NonBlocking = 2,
    Monitor = 3,
}

pub struct LowerContext<'a> {
    pub table: VSymbolTable,
    pub table_ast_refs: SymbolAstRefs<'a>,
    pub udps: VgHashMap<IdentId, AstId<'a, UdpDeclaration<'a>>>,
    pub arenas: AstArenas,
    pub tokenized: &'a Tokenized,
    pub time_scale: TimeScale,
}

pub struct MutLowerContext {
    pub gl: GlobalContext,
    pub diagnostics: Diagnostics,
    pub connections: Vec<vogls_fuse_signals::InputEdge>,
    pub fuse_scratch: Vec<vogls_fuse_signals::Driver>,
    pub has_vcd: bool,
}
impl MutLowerContext {
    pub fn gl(&mut self) -> &mut GlobalContext {
        &mut self.gl
    }
}

fn extend_symbol_table_to_vcd_scope(
    scope: &mut VcdScope,
    symbols: &[SymbolId],
    table: &VSymbolTable,
    ident_table: &IdentTable,
    variable_table: &mut Table<VcdVariableKey, VcdVariable>,
    signal_map: &mut VgHashMap<SignalKey, Vec<VcdVariableKey>>,
) {
    use VSymbol as S;
    for sid in symbols.iter() {
        let name = &ident_table[table[*sid].name()];
        match &table[*sid].content {
            S::Module(_) | S::NamedBlock | S::GenerateBlock(_) | S::GenerateBlocks => {
                let mut subscope = VcdScope {
                    name: name.to_string(),
                    items: Vec::new(),
                };
                extend_symbol_table_to_vcd_scope(
                    &mut subscope,
                    table[*sid].children(),
                    table,
                    ident_table,
                    variable_table,
                    signal_map,
                );
                scope
                    .items
                    .push(vogls_ir::vcd::VcdScopeItem::Scope(subscope));
            }
            S::Net(i) => {
                let net = &i.net;

                // @TODO: Property implement this.
                let lsb = 0;
                let msb = i.ty.force_net_width().get() - 1;
                let msb_lsb = (msb > 0).then_some((msb, lsb));

                let signal = net.probe_signal();
                let variable_key = variable_table.insert(vogls_ir::vcd::VcdVariable {
                    name: ident_table[table[*sid].name()].to_string(),
                    value: VcdValue::Signal(signal, None),
                    ty: vogls_ir::vcd::NetType::Wire,
                    msb_lsb,
                });
                scope
                    .items
                    .push(vogls_ir::vcd::VcdScopeItem::Variable(variable_key));
                signal_map.entry(signal).or_default().push(variable_key);
            }
            S::Task(_) | S::Function(_) | S::Parameter(_) | S::GenVar => {}
        }
    }
}

impl<'a> LowerContext<'a> {
    pub fn vcd_scope(&self, scope: SymbolId, ident_table: &IdentTable) -> vogls_ir::vcd::VcdOutput {
        let mut key = scope;
        while let Some(parent) = self.table[key].parent() {
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
            &self.table,
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

    pub fn get_line_number(&self, token_idx: usize) -> usize {
        let file_idx = self.tokenized.file_idxs[token_idx];
        let start = self.tokenized.spans[token_idx].start();
        let idx = match self.tokenized.file_line_offsets[file_idx as usize].binary_search(&start) {
            Ok(l) => l,
            Err(l) => l - 1,
        };
        idx + 1
    }
}

pub fn resolve_symbol_id(
    scope: SymbolId,
    table: &VSymbolTable,
    ident: IdentId,
) -> Option<SymbolId> {
    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 196
    //
    // """
    // If it is declared locally, then the local item shall be used; if not, the search shall
    // continue upward until an item by that name is found or until a module boundary is
    // encountered.
    //
    // If the item is a variable, it shall stop at a module boundary; if the item is a task,
    // function, named block, or generate block, it continues to search higher level modules
    // until found. This fact means that tasks and functions can use and modify the variables
    // within the containing module by name, without going through their ports.
    // """

    let mut scope = scope;
    loop {
        if let Some(k) = table.resolve(scope, ident) {
            return Some(k);
        }

        let item = &table[scope];
        let parent = item.parent()?;
        if matches!(item.content, VSymbol::Module(_)) {
            return None;
        }

        scope = parent;
    }
}

pub fn resolve_symbol_id_hier<'a>(
    scope: SymbolId,
    table: &VSymbolTable,
    ident: impl Into<HIdent<'a>>,
) -> Option<SymbolId> {
    let ident = ident.into();

    let mut scope = scope;

    for component in ident.components {
        scope = resolve_symbol_id(scope, table, component.ident.item.0)?;
    }

    resolve_symbol_id(scope, table, ident.ident.item.0)
}

pub fn try_resolve_symbol_id<'a>(
    scope: SymbolId,
    table: &VSymbolTable,
    arenas: &AstArenas,
    ident: impl Into<HIdent<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    let ident = ident.into();

    let mut scope = scope;

    for component in ident.components {
        scope = try_resolve_symbol_id_nonhier(scope, table, arenas, component.ident, diagnostics)?;
    }

    try_resolve_symbol_id_nonhier(scope, table, arenas, ident.ident, diagnostics)
}
pub fn try_resolve_symbol_id_nonhier(
    scope: SymbolId,
    table: &VSymbolTable,
    arenas: &AstArenas,
    ident: AstItem<Identifier>,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    let Some(symid) = resolve_symbol_id(scope, table, ident.item.0) else {
        diagnostics.var_not_found(arenas, ident);
        return Err(());
    };
    Ok(symid)
}

pub fn try_resolve_net<'a, 's>(
    scope: SymbolId,
    table: &'s VSymbolTable,
    arenas: &AstArenas,
    ident: impl Into<HIdent<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<&'s NetSymbol, ()> {
    let ident = ident.into();
    let sid = try_resolve_symbol_id(scope, table, arenas, ident, diagnostics)?;
    let VSymbol::Net(n) = &table[sid].content else {
        diagnostics.not_yet_implemented(hident_span(arenas, ident), "cannot be used as net");
        return Err(());
    };
    Ok(n)
}

pub fn try_resolve_net_mut<'a, 's>(
    scope: SymbolId,
    table: &'s mut VSymbolTable,
    arenas: &AstArenas,
    ident: impl Into<HIdent<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<&'s mut NetSymbol, ()> {
    let ident = ident.into();
    let sid = try_resolve_symbol_id(scope, table, arenas, ident, diagnostics)?;
    let VSymbol::Net(n) = &mut table[sid].content else {
        diagnostics.not_yet_implemented(hident_span(arenas, ident), "cannot be used as net");
        return Err(());
    };
    Ok(n)
}
pub fn try_resolve_net_with_sid<'a, 's>(
    scope: SymbolId,
    table: &'s VSymbolTable,
    arenas: &AstArenas,
    ident: impl Into<HIdent<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<(SymbolId, &'s NetSymbol), ()> {
    let ident = ident.into();
    let sid = try_resolve_symbol_id(scope, table, arenas, ident, diagnostics)?;
    let VSymbol::Net(n) = &table[sid].content else {
        diagnostics.not_yet_implemented(hident_span(arenas, ident), "cannot be used as net");
        return Err(());
    };
    Ok((sid, n))
}

pub fn strict_resolve_module<'a>(
    scope: SymbolId,
    table: &'a VSymbolTable,
    ident: IdentId,
) -> &'a ModuleSymbol {
    let sid = resolve_symbol_id(scope, table, ident).unwrap();
    let VSymbol::Module(n) = &table[sid].content else {
        panic!()
    };
    n
}

pub fn unwrap_get_fn_mut<'a>(table: &'a mut VSymbolTable, sid: SymbolId) -> &'a mut FunctionSymbol {
    let VSymbol::Function(n) = &mut table[sid].content else {
        panic!()
    };
    n
}
pub fn unwrap_get_task_mut<'a>(table: &'a mut VSymbolTable, sid: SymbolId) -> &'a mut TaskSymbol {
    let VSymbol::Task(n) = &mut table[sid].content else {
        panic!()
    };
    n
}

pub fn unwrap_get_net<'a>(table: &'a VSymbolTable, sid: SymbolId) -> &'a NetSymbol {
    let VSymbol::Net(n) = &table[sid].content else {
        panic!()
    };
    n
}
pub fn unwrap_get_net_mut<'a>(table: &'a mut VSymbolTable, sid: SymbolId) -> &'a mut NetSymbol {
    let VSymbol::Net(n) = &mut table[sid].content else {
        panic!()
    };
    n
}

pub fn unwrap_resolve_net<'a>(
    scope: SymbolId,
    table: &'a VSymbolTable,
    ident: IdentId,
) -> &'a NetSymbol {
    let sid = resolve_symbol_id(scope, table, ident).unwrap();
    unwrap_get_net(table, sid)
}
pub fn unwrap_resolve_net_mut<'a>(
    scope: SymbolId,
    table: &'a mut VSymbolTable,
    ident: IdentId,
) -> &'a mut NetSymbol {
    let sid = resolve_symbol_id(scope, table, ident).unwrap();
    unwrap_get_net_mut(table, sid)
}

pub fn unwrap_get_module<'a>(table: &'a VSymbolTable, sid: SymbolId) -> &'a ModuleSymbol {
    let VSymbol::Module(n) = &table[sid].content else {
        panic!()
    };
    n
}
pub fn unwrap_get_module_mut<'a>(
    table: &'a mut VSymbolTable,
    sid: SymbolId,
) -> &'a mut ModuleSymbol {
    let VSymbol::Module(n) = &mut table[sid].content else {
        panic!()
    };
    n
}

pub fn unwrap_get_param_mut<'a>(table: &'a mut VSymbolTable, sid: SymbolId) -> &'a mut VValue {
    let VSymbol::Parameter(n) = &mut table[sid].content else {
        panic!()
    };
    n
}

fn hident_span<'a>(arenas: &AstArenas, ident: HIdent<'a>) -> TokenRange {
    let lst = arenas.get_item_span(ident.ident);
    match ident.components.first() {
        None => lst,
        Some(fst) => arenas.get_span(fst) | lst,
    }
}

pub fn try_resolve_constant<'a, 's>(
    scope: SymbolId,
    table: &'s VSymbolTable,
    arenas: &AstArenas,
    ident: impl Into<HIdent<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<&'s VValue, ()> {
    let ident = ident.into();
    let sid = try_resolve_symbol_id(scope, table, arenas, ident, diagnostics)?;
    let VSymbol::Parameter(value) = &table[sid].content else {
        diagnostics.not_yet_implemented(hident_span(arenas, ident), "cannot be used as constant");
        return Err(());
    };
    Ok(value)
}

use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::{Driver, InputEdge};
use vogls_ir::token_range::TokenRange;
use vogls_ir::vcd::{VcdScope, VcdValue, VcdVariable, VcdVariableKey};
use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, GlobalContext, ProcessKey, ProcessKind, SCALAR_VSIZE,
    SignalKey, SignalSlice, VariableKey, VectorSize, new_process,
};
use vogls_utils::{IndexMap, OrderedSet, Table, VgHashMap};

use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{BitSlice, Expr};
use crate::ast::module::{
    GenerateRegion, Module, ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemContent,
    NonPortModuleItem, Range, TimeScale,
};
use crate::ast::statement::{Statement, StatementContent, StatementOrNull};
use crate::ast::udp::UdpDeclaration;
use crate::ast::{AstId, AstIdRange, AstItem, HIdent, Identifier};
use crate::elaborate::{
    FunctionSymbol, ModuleSymbol, NetSymbol, SymbolAstRefs, TaskSymbol, VSymbol, VSymbolTable,
};
use crate::parser::AstArenas;
use crate::tokenizer::Tokenized;

pub use self::expression::eval_constant_expr;
use self::expression::{get_expr_type, get_used_signals, lower_expr, truncate_or_extend};
use self::fuse::try_lower_fuse_driver_expr;
pub use self::vtype::VType;
pub use self::vvalue::VValue;
pub use diagnostics::{Diagnostics, LowerErrorReason};
pub use module_or_generate_item::dims_to_array;

pub fn lower_module_to_ir<'a>(
    root: AstId<'a, Module<'a>>,
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
) -> Result<(), ()> {
    let Module {
        attribute_instances: _,
        module_identifier: _,
        module_parameter_port_list: _,
        ports: _,
        module_items,
        default_nettype: _,
        time_scale: _,
    } = &*root;

    let mut scopes = Vec::new();
    scopes.extend(ctx.table[scope].children().iter().filter(|c| {
        matches!(
            ctx.table[**c].content,
            VSymbol::GenerateBlock(_) | VSymbol::GenerateBlocks
        )
    }));
    while let Some(scope_key) = scopes.pop() {
        if let VSymbol::GenerateBlock(offset) = &ctx.table[scope_key].content {
            for id in ctx.table_ast_refs.gen_blocks[*offset].iter() {
                module_or_generate_item::lower(ctx, mctx, scope_key, id)?;
            }
        }
        scopes.extend(ctx.table[scope_key].children().iter().filter(|c| {
            matches!(
                ctx.table[**c].content,
                VSymbol::GenerateBlock(_) | VSymbol::GenerateBlocks
            )
        }));
    }

    for module_item in module_items.iter() {
        match &*module_item {
            ModuleItem::PortDeclaration(_) => {}
            ModuleItem::NonPortModuleItem(p) => match &**p {
                NonPortModuleItem::ModuleOrGenerateItem(id) => {
                    module_or_generate_item::lower(ctx, mctx, scope, *id)?
                }
                NonPortModuleItem::GenerateRegion(region) => {
                    let GenerateRegion {
                        module_or_generate_item,
                    } = region;
                    for id in module_or_generate_item.iter() {
                        module_or_generate_item::lower(ctx, mctx, scope, id)?;
                    }
                }
                NonPortModuleItem::SpecifyBlock(_) => {}
                NonPortModuleItem::ParameterDeclaration(_) => {}
                NonPortModuleItem::SpecParamDeclaration => todo!(),
            },
        }
    }
    Ok(())
}

enum WatchCondition {
    None,
    Posedge,
    Negedge,
}

fn assign_input_port<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    port: SymbolId,
) -> Result<(), ()> {
    let port = unwrap_get_net(&ctx.table, port);

    mctx.fuse_scratch.clear();
    if try_lower_fuse_driver_expr(ctx, mctx, scope, expr)? {
        let drivee = port.net.blocking_drive_signal();

        let mut offset = 0;
        let drivee_width = mctx.gl.signals[drivee].size;
        for driver in &mctx.fuse_scratch {
            let width = driver.size(&mctx.gl.signals);
            let Some(width) = VectorSize::new((drivee_width.get() - offset).min(width.get()))
            else {
                break;
            };
            mctx.connections.push(InputEdge {
                driver: driver.clone(),
                drivee,
                drivee_slice: Some(SignalSlice::from_width(offset, width).unwrap()),
            });
            offset += width.get();
        }
        return Ok(());
    }

    let (_, mut bb_builder) = new_process(mctx.gl(), ProcessKind::Port, ctx.arenas.get_span(expr));
    let bb_key = bb_builder.key();
    let context_width = port.ty.force_net_width();
    let (v, v_ty) = lower_expr(ctx, mctx, scope, &mut bb_builder, expr, Some(context_width))?;
    let v = expression::sign_or_zero_extend(
        mctx.gl(),
        &mut bb_builder,
        v,
        v_ty,
        port.ty.force_net_width(),
    );
    port.net.drive_blocking(mctx.gl(), &mut bb_builder, v, None);

    let mut signals = OrderedSet::new();
    expression::get_used_signals(ctx, mctx, scope, &mut signals, expr)?;
    if signals.is_empty() {
        bb_builder.halt(mctx.gl());
    } else {
        bb_builder.watch_to(mctx.gl(), signals.items, bb_key);
    }
    Ok(())
}

fn assign_port_output<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    output_net: SymbolId,
    _ty: VType,
) -> Result<(), ()> {
    let output = unwrap_get_net(&ctx.table, output_net);

    if let Expr::Ident(ident, exprs, range) = &*expr {
        let to_signal = try_resolve_net(
            scope,
            &ctx.table,
            &ctx.arenas,
            *ident,
            &mut mctx.diagnostics,
        )?;
        let driver = output.net.probe_signal();
        let drivee = to_signal.net.blocking_drive_signal();
        if exprs.is_empty() && range.is_none() {
            mctx.connections.push(InputEdge {
                driver: Driver::Signal(driver, None),
                drivee,
                drivee_slice: None,
            });
            return Ok(());
        }

        if range.is_none()
            && exprs.len() == 1
            && let Ok(v) = eval_constant_expr(
                &mctx.gl,
                &ctx.arenas,
                &ctx.table,
                scope,
                &mut Diagnostics::default(),
                exprs.get(0).into_constant(),
                None,
            )
        {
            let v = v.coerce(&VType::SignedNet(vogls_ir::INTEGER_VSIZE));
            let v = v.into_bits();
            if let Some(v) = v.extract_exact_u32() {
                mctx.connections.push(InputEdge {
                    driver: Driver::Signal(driver, None),
                    drivee,
                    drivee_slice: Some(SignalSlice::from_width(v, SCALAR_VSIZE).unwrap()),
                });
                return Ok(());
            }
        }
    }

    let (_, mut bb_builder) = new_process(mctx.gl(), ProcessKind::Port, ctx.arenas.get_span(expr));
    let bb_key = bb_builder.key();

    let signal = &output.net;
    let ty = output.ty;
    let probed = output.net.probe(mctx.gl(), &mut bb_builder);

    let mut driving: Vec<(VariableKey, VType, AstId<Expr>)> = Vec::new();
    driving.push((probed, ty, expr));

    let mut ins = OrderedSet::new();
    ins.insert(signal.probe_signal());

    let mut error = false;
    while let Some((var, var_ty, expr)) = driving.pop() {
        match &*expr {
            Expr::Concatenation(exprs) => {
                let mut shift = 0;
                for e in exprs.iter().rev() {
                    let e_ty = get_expr_type(
                        &mctx.gl,
                        &ctx.arenas,
                        &ctx.table,
                        scope,
                        &mut mctx.diagnostics,
                        e,
                    )?;
                    let e_width = e_ty.force_net_width();
                    let subvar = bb_builder.slice_constant(mctx.gl(), var, shift, e_width);
                    driving.push((subvar, e_ty, e));
                    shift += e_width.get();
                }
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                let symbol_key = try_resolve_symbol_id(
                    scope,
                    &ctx.table,
                    &ctx.arenas,
                    *ast_ident,
                    &mut mctx.diagnostics,
                )?;
                let VSymbol::Net(s) = &ctx.table[symbol_key].content else {
                    mctx.diagnostics
                        .output_expr_not_allowed(ctx.arenas.get_span(expr));
                    error = true;
                    continue;
                };

                if range_expression.is_none() && exprs.is_empty() {
                    s.net.drive_blocking(mctx.gl(), &mut bb_builder, var, None);
                    continue;
                }

                if let Some(range_expression) = range_expression {
                    match range_expression {
                        BitSlice::MsbLsb(_, _) => {}
                        BitSlice::PlusWidth(base, _) | BitSlice::MinusWidth(base, _) => {
                            get_used_signals(ctx, mctx, scope, &mut ins, *base)?;
                        }
                    }
                }
                for expr in exprs.iter() {
                    get_used_signals(ctx, mctx, scope, &mut ins, expr)?;
                }

                let (offset_dst, length_dst) = if range_expression.is_none() && exprs.len() == 1 {
                    (
                        lower_expr(
                            ctx,
                            mctx,
                            scope,
                            &mut bb_builder,
                            exprs.first().unwrap(),
                            None,
                        )?
                        .0,
                        None,
                    )
                } else if let Some(slice) = range_expression
                    && exprs.is_empty()
                {
                    match slice {
                        BitSlice::MsbLsb(msb, lsb) => {
                            let (_, lsb, width) = msb_lsb_to_width(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                *msb,
                                *lsb,
                            )?;
                            let offset = bb_builder.constant_u32(mctx.gl(), lsb as u32);
                            (offset, Some(width as VectorSize))
                        }
                        BitSlice::PlusWidth(base, width) => {
                            let offset = lower_expr(ctx, mctx, scope, &mut bb_builder, *base, None);
                            let width = eval_constant_expr(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                *width,
                                None,
                            );
                            let width = width?.as_integer().unwrap();
                            (offset?.0, Some(VectorSize::new(width as u32).unwrap()))
                        }
                        BitSlice::MinusWidth(base, width) => {
                            let offset = lower_expr(ctx, mctx, scope, &mut bb_builder, *base, None);
                            let width = eval_constant_expr(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                *width,
                                None,
                            )?;
                            let width =
                                VectorSize::new(width.as_integer().unwrap() as u32).unwrap();
                            let width_v = bb_builder
                                .constant_u32(mctx.gl(), width.checked_add(1).unwrap().get());
                            let offset = bb_builder.minus(mctx.gl(), offset?.0, width_v);
                            (offset, Some(width))
                        }
                    }
                } else {
                    mctx.diagnostics
                        .not_yet_implemented(ctx.arenas.get_span(expr), "multiple braced");
                    error = true;
                    continue;
                };

                let length_dst = length_dst.unwrap_or(SCALAR_VSIZE);
                let src = var;
                let src = truncate_or_extend(mctx.gl(), &mut bb_builder, src, var_ty, length_dst);
                s.net
                    .drive_blocking(mctx.gl(), &mut bb_builder, src, Some(offset_dst));
            }

            Expr::Replication(_) => {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(expr), "repetition in net assign");
                error = true;
            }

            Expr::FunctionCall(..)
            | Expr::SystemFunctionCall(..)
            | Expr::Decimal(..)
            | Expr::Sized(..)
            | Expr::Ternary(..)
            | Expr::String(..)
            | Expr::Unary(..)
            | Expr::Binary(..) => {
                mctx.diagnostics
                    .output_expr_not_allowed(ctx.arenas.get_span(expr));
                error = true;
            }
        }
    }

    bb_builder.watch_to(mctx.gl(), ins.items, bb_key);

    if error {
        return Err(());
    }

    Ok(())
}

fn assign_task_output<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    variable: VariableKey,
    expr: AstId<'a, Expr<'a>>,
    ty: VType,
) -> Result<(), ()> {
    let mut driving: Vec<AstId<'a, Expr<'a>>> = Vec::new();
    driving.push(expr);

    let mut error = false;
    while let Some(expr) = driving.pop() {
        match &*expr {
            Expr::Concatenation(_) => {
                todo!()
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                let symbol_key = try_resolve_symbol_id(
                    scope,
                    &ctx.table,
                    &ctx.arenas,
                    *ast_ident,
                    &mut mctx.diagnostics,
                )?;
                let VSymbol::Net(s) = &ctx.table[symbol_key].content else {
                    mctx.diagnostics
                        .output_expr_not_allowed(ctx.arenas.get_span(expr));
                    error = true;
                    continue;
                };

                let (offset_dst, length_dst) = if range_expression.is_none() && exprs.is_empty() {
                    (
                        builder.constant_u32(mctx.gl(), 0),
                        Some(s.ty.force_net_width()),
                    )
                } else if range_expression.is_none() && exprs.len() == 1 {
                    (
                        lower_expr(ctx, mctx, scope, builder, exprs.first().unwrap(), None)?.0,
                        None,
                    )
                } else if let Some(slice) = range_expression
                    && exprs.is_empty()
                {
                    match slice {
                        BitSlice::MsbLsb(msb, lsb) => {
                            let (_, lsb, width) = msb_lsb_to_width(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                *msb,
                                *lsb,
                            )?;
                            let offset = builder.constant_u32(mctx.gl(), lsb as u32);
                            (offset, Some(width as VectorSize))
                        }
                        BitSlice::PlusWidth(base, width) => {
                            let offset = lower_expr(ctx, mctx, scope, builder, *base, None);
                            let width = eval_constant_expr(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                *width,
                                None,
                            );
                            let width = width?.as_integer().unwrap();
                            (offset?.0, Some(VectorSize::new(width as u32).unwrap()))
                        }
                        BitSlice::MinusWidth(base, width) => {
                            let offset = lower_expr(ctx, mctx, scope, builder, *base, None);
                            let width = eval_constant_expr(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                *width,
                                None,
                            )?;
                            let width =
                                VectorSize::new(width.as_integer().unwrap() as u32).unwrap();
                            let width_v = builder
                                .constant_u32(mctx.gl(), width.checked_add(1).unwrap().get());
                            let offset = builder.minus(mctx.gl(), offset?.0, width_v);
                            (offset, Some(width))
                        }
                    }
                } else {
                    mctx.diagnostics
                        .not_yet_implemented(ctx.arenas.get_span(expr), "multiple braced");
                    error = true;
                    continue;
                };

                let length_dst = length_dst.unwrap_or(SCALAR_VSIZE);
                let src = truncate_or_extend(mctx.gl(), builder, variable, ty, length_dst);
                s.net
                    .drive_blocking(mctx.gl(), builder, src, Some(offset_dst));
            }

            Expr::Replication(_) => {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(expr), "repetition in net assign");
                error = true;
            }

            Expr::FunctionCall(..)
            | Expr::SystemFunctionCall(..)
            | Expr::Decimal(..)
            | Expr::Sized(..)
            | Expr::Ternary(..)
            | Expr::String(..)
            | Expr::Unary(..)
            | Expr::Binary(..) => {
                mctx.diagnostics
                    .output_expr_not_allowed(ctx.arenas.get_span(expr));
                error = true;
            }
        }
    }

    if error {
        return Err(());
    }

    Ok(())
}

fn msb_lsb_to_width<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    table: &VSymbolTable,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    ast_msb: AstId<ConstantExpr>,
    ast_lsb: AstId<ConstantExpr>,
) -> Result<(i64, i64, VectorSize), ()> {
    let msb = eval_constant_expr(gl, arenas, table, scope, diagnostics, ast_msb, None);
    let lsb = eval_constant_expr(gl, arenas, table, scope, diagnostics, ast_lsb, None);

    let (Ok(VValue::SignedNet(msb)), Ok(VValue::SignedNet(lsb))) = (msb, lsb) else {
        return Err(());
    };
    let msb = msb.as_i64().unwrap();
    let lsb = lsb.as_i64().unwrap();
    let width = u32::try_from(msb.abs_diff(lsb)).ok();
    let width = width.and_then(|w| w.checked_add(1));
    let width = width.and_then(|w| VectorSize::new(w));
    let Some(width) = width else {
        let tr = arenas.get_span(ast_msb) | arenas.get_span(ast_lsb);
        diagnostics.net_width_overflow(tr);
        return Err(());
    };
    Ok((msb, lsb, width))
}

pub fn evaluate_range<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    table: &VSymbolTable,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    range: AstId<'a, Range<'a>>,
) -> Result<(i64, i64, VectorSize), ()> {
    let range = &*range;
    msb_lsb_to_width(gl, arenas, table, scope, diagnostics, range.msb, range.lsb)
}

pub fn create_nba_process(
    gl: &mut GlobalContext,
    signal: SignalKey,
    needs_mask: bool,
) -> (ProcessKey, SignalKey, Option<SignalKey>) {
    let vogls_ir::Signal {
        name, origin, size, ..
    } = &gl.signals[signal];

    let mask_name = format!("{name}::NBA_MASK");
    let value_name = format!("{name}::NBA_VALUE");
    let (size, origin) = (*size, *origin);
    let (process_key, mut builder) = new_process(gl, ProcessKind::NonBlockingAssignment, origin);

    let mask = needs_mask.then(|| {
        gl.signals.insert(vogls_ir::Signal {
            name: mask_name,
            size,
            initialize: None,
            origin,
        })
    });
    let value = gl.signals.insert(vogls_ir::Signal {
        name: value_name,
        size,
        initialize: None,
        origin,
    });

    match mask {
        None => {
            let init_bb = builder.key();
            let value_v = builder.probe(gl, value);
            builder.drive(gl, signal, value_v);
            builder = builder.watch(gl, [value].into());
            builder.wait_region_to(gl, Region::NonBlocking as u8, init_bb);
        }
        Some(mask) => {
            // We need to conditionally branch here as it might have already been assigned before.
            let mask_v = builder.probe(gl, mask);
            let mask_ro = builder.reduce_or(gl, mask_v);

            let init_bb = builder.key();
            builder = builder.next_terminate_later(gl);

            let watch_bb = builder.key();
            builder = builder.watch(gl, [value].into());

            let waitregion_bb = builder.key();
            builder = builder.wait_region(gl, Region::NonBlocking as u8);

            let mask_v = builder.probe(gl, mask);
            let value_v = builder.probe(gl, value);
            let inv_mask = builder.binary_neg(gl, mask_v);
            let old = builder.probe(gl, signal);
            let old = builder.and(gl, old, inv_mask);
            let value_v = builder.and(gl, value_v, mask_v);
            let result = builder.or(gl, old, value_v);
            builder.drive(gl, signal, result);
            builder.jump_to(gl, watch_bb);

            gl.bbs[init_bb].terminator =
                BasicBlockTerminator::Branch(mask_ro, waitregion_bb, watch_bb);
        }
    }

    (process_key, value, mask)
}

pub fn instantiate_nba_signals<'a>(
    gl: &mut GlobalContext,
    ctx: &mut LowerContext<'a>,
    scope: SymbolId,
    module: AstId<'a, Module<'a>>,
    diagnostics: &mut Diagnostics,
    nba_signals: &mut IndexMap<SymbolId, (SignalKey, bool)>,
) -> Result<(), ()> {
    let mut scopes = Vec::new();
    scopes.extend(ctx.table[scope].children().iter().filter(|c| {
        matches!(
            ctx.table[**c].content,
            VSymbol::GenerateBlock(_) | VSymbol::GenerateBlocks
        )
    }));
    while let Some(scope_key) = scopes.pop() {
        if let VSymbol::GenerateBlock(offset) = &ctx.table[scope_key].content {
            for id in ctx.table_ast_refs.gen_blocks[*offset].iter() {
                instantiate_module_or_generate_item_nba_signals(
                    gl,
                    ctx,
                    scope_key,
                    id,
                    diagnostics,
                    nba_signals,
                )?;
            }
        }
        scopes.extend(ctx.table[scope_key].children().iter().filter(|c| {
            matches!(
                ctx.table[**c].content,
                VSymbol::GenerateBlock(_) | VSymbol::GenerateBlocks
            )
        }));
    }

    for module_item in module.module_items.iter() {
        match &*module_item {
            ModuleItem::PortDeclaration(_) => {}
            ModuleItem::NonPortModuleItem(p) => match &**p {
                NonPortModuleItem::ModuleOrGenerateItem(id) => {
                    instantiate_module_or_generate_item_nba_signals(
                        gl,
                        ctx,
                        scope,
                        *id,
                        diagnostics,
                        nba_signals,
                    )?;
                }
                NonPortModuleItem::GenerateRegion(_) => {}
                NonPortModuleItem::SpecifyBlock(_) => {}
                NonPortModuleItem::ParameterDeclaration(_) => {}
                NonPortModuleItem::SpecParamDeclaration => todo!(),
            },
        }
    }

    Ok(())
}

pub fn instantiate_module_or_generate_item_nba_signals<'a>(
    gl: &mut GlobalContext,
    ctx: &mut LowerContext<'a>,
    scope: SymbolId,
    item: AstId<'a, ModuleOrGenerateItem<'a>>,
    diagnostics: &mut Diagnostics,
    nba_signals: &mut IndexMap<SymbolId, (SignalKey, bool)>,
) -> Result<(), ()> {
    match item.content {
        ModuleOrGenerateItemContent::ModuleOrGenerateItemDeclaration(_)
        | ModuleOrGenerateItemContent::LocalParameterDeclaration(_)
        | ModuleOrGenerateItemContent::ParameterOverride
        | ModuleOrGenerateItemContent::ContinuousAssign(_)
        | ModuleOrGenerateItemContent::GateInstantiation(_)
        | ModuleOrGenerateItemContent::UdpInstantiation(_)
        | ModuleOrGenerateItemContent::ModuleInstantiation(_)
        | ModuleOrGenerateItemContent::LoopGenerateConstruct(_)
        | ModuleOrGenerateItemContent::IfGenerateConstruct(_)
        | ModuleOrGenerateItemContent::CaseGenerateConstruct(_) => Ok(()),
        ModuleOrGenerateItemContent::InitialConstruct(id) => instantiate_stmts_nba_signals(
            gl,
            ctx,
            scope,
            AstIdRange::single(id.0),
            diagnostics,
            nba_signals,
        ),
        ModuleOrGenerateItemContent::AlwaysConstruct(id) => instantiate_stmts_nba_signals(
            gl,
            ctx,
            scope,
            AstIdRange::single(id.0),
            diagnostics,
            nba_signals,
        ),
    }
}

pub fn instantiate_stmts_nba_signals<'a>(
    gl: &mut GlobalContext,
    ctx: &mut LowerContext<'a>,
    scope: SymbolId,
    stmts: AstIdRange<'a, Statement<'a>>,
    diagnostics: &mut Diagnostics,
    nba_signals: &mut IndexMap<SymbolId, (SignalKey, bool)>,
) -> Result<(), ()> {
    for stmt in stmts {
        match stmt.content {
            StatementContent::NonBlockingAssignment(nba) => {
                for vlvalue in nba.variable_lvalue.0 {
                    let (sid, net) = try_resolve_net_with_sid(
                        scope,
                        &mut ctx.table,
                        &ctx.arenas,
                        vlvalue.ident,
                        diagnostics,
                    )?;
                    let needs_mask =
                        !vlvalue.exprs.is_empty() && vlvalue.range_expression.is_some();

                    match nba_signals.entry(sid) {
                        vogls_utils::Entry::Occuppied(mut entry) => entry.get().1 |= needs_mask,
                        vogls_utils::Entry::Vacant(entry) => {
                            _ = entry.insert((net.net.blocking_drive_signal(), needs_mask))
                        }
                    }
                }
            }
            StatementContent::DisableStatement => todo!(),
            StatementContent::EventTrigger => todo!(),
            StatementContent::CaseStatement(id) => {
                for case_item in id.items {
                    instantiate_stmt_or_null_nba_signals(
                        gl,
                        ctx,
                        scope,
                        case_item.statement_or_null,
                        diagnostics,
                        nba_signals,
                    )?;
                }
            }
            StatementContent::ConditionalStatement(id) => {
                instantiate_stmt_or_null_nba_signals(
                    gl,
                    ctx,
                    scope,
                    id.if_branch.statement,
                    diagnostics,
                    nba_signals,
                )?;
                for else_if in id.else_ifs {
                    instantiate_stmt_or_null_nba_signals(
                        gl,
                        ctx,
                        scope,
                        else_if.statement,
                        diagnostics,
                        nba_signals,
                    )?;
                }
                if let Some(else_branch) = id.else_branch {
                    instantiate_stmt_or_null_nba_signals(
                        gl,
                        ctx,
                        scope,
                        else_branch,
                        diagnostics,
                        nba_signals,
                    )?;
                }
            }
            StatementContent::LoopStatement(id) => instantiate_stmts_nba_signals(
                gl,
                ctx,
                scope,
                AstIdRange::single(id.statement),
                diagnostics,
                nba_signals,
            )?,
            StatementContent::ProceduralContinuousAssignments => todo!(),

            StatementContent::ParBlock(id) => instantiate_stmts_nba_signals(
                gl,
                ctx,
                scope,
                id.statements,
                diagnostics,
                nba_signals,
            )?,
            StatementContent::SeqBlock(id) => instantiate_stmts_nba_signals(
                gl,
                ctx,
                scope,
                id.statements,
                diagnostics,
                nba_signals,
            )?,
            StatementContent::ProceduralTimingControlStatement(id) => {
                instantiate_stmt_or_null_nba_signals(
                    gl,
                    ctx,
                    scope,
                    id.statement_or_null,
                    diagnostics,
                    nba_signals,
                )?
            }
            StatementContent::WaitStatement(id) => instantiate_stmt_or_null_nba_signals(
                gl,
                ctx,
                scope,
                id.statement_or_null,
                diagnostics,
                nba_signals,
            )?,
            StatementContent::TaskEnable(_) => {
                // @TODO: this needs to instantiate something...
            }
            StatementContent::BlockingAssignment(_) | StatementContent::SystemTaskEnable(_) => {}
        }
    }
    Ok(())
}

pub fn instantiate_stmt_or_null_nba_signals<'a>(
    gl: &mut GlobalContext,
    ctx: &mut LowerContext<'a>,
    scope: SymbolId,
    stmt: AstId<'a, StatementOrNull<'a>>,
    diagnostics: &mut Diagnostics,
    nba_signals: &mut IndexMap<SymbolId, (SignalKey, bool)>,
) -> Result<(), ()> {
    match &*stmt {
        StatementOrNull::Attribute(_) => Ok(()),
        StatementOrNull::Statement(stmt) => instantiate_stmts_nba_signals(
            gl,
            ctx,
            scope,
            AstIdRange::single(*stmt),
            diagnostics,
            nba_signals,
        ),
    }
}
