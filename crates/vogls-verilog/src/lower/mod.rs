mod addressing;
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

pub struct LowerContext<'a, 'b> {
    pub logic_mode: LogicMode,
    pub table: VSymbolTable,
    pub table_ast_refs: SymbolAstRefs<'a>,
    pub udps: VgHashMap<IdentId, AstId<'a, UdpDeclaration<'a>>>,
    pub arenas: &'b AstArenas,
    pub tokenized: &'b Tokenized,
    pub time_scale: TimeScale,
    pub time_resolution: TimeResolution,
}

pub struct MutLowerContext {
    pub gl: GlobalContext,
    pub nbas: IndexMap<SignalKey, (ProcessKey, SignalKey, Option<SignalKey>)>,
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
            S::Module(_)
            | S::NamedBlock
            | S::GenerateBlock(_)
            | S::GenerateBlocks
            | S::ModuleRange(_) => {
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
                let msb = i.ty.bit_length().get() - 1;
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

impl<'a> LowerContext<'a, '_> {
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

pub fn resolve_hier_symbol_id(
    scope: SymbolId,
    table: &VSymbolTable,
    ident: IdentId,
) -> Option<SymbolId> {
    let mut scope = scope;
    loop {
        if let Some(k) = table.resolve(scope, ident) {
            return Some(k);
        }

        let item = &table[scope];
        let parent = item.parent()?;
        scope = parent;
    }
}

fn resolve_hident_impl<'a>(
    scope: SymbolId,
    table: &VSymbolTable,
    ident: impl Into<HIdent<'a>>,
) -> Result<SymbolId, AstItem<Identifier>> {
    let ident = ident.into();

    let mut scope = scope;

    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 196
    //
    // """
    // If an identifier is referenced with a hierarchical name, the path can start with a module
    // name, instance name, task, function, named block, or named generate block. The names shall
    // be searched first at the current level and then in higher level modules until found. Because
    // both module names and instance names can be used, precedence is given to instance names if
    // there is a module named the same as an instance name.
    // """

    let Some(fst) = ident.components.first() else {
        return resolve_symbol_id(scope, table, ident.ident.item.0).ok_or(ident.ident);
    };

    // @TODO: Allow module names as well if the next line fails.
    scope = resolve_hier_symbol_id(scope, table, fst.ident.item.0).ok_or(fst.ident)?;
    for component in ident.components.iter().skip(1) {
        scope = table
            .resolve(scope, component.ident.item.0)
            .ok_or(component.ident)?;
    }
    table.resolve(scope, ident.ident.item.0).ok_or(ident.ident)
}

pub fn resolve_hident<'a>(
    scope: SymbolId,
    table: &VSymbolTable,
    ident: impl Into<HIdent<'a>>,
) -> Option<SymbolId> {
    resolve_hident_impl(scope, table, ident).ok()
}

pub fn try_resolve_hident<'a>(
    scope: SymbolId,
    table: &VSymbolTable,
    arenas: &AstArenas,
    ident: impl Into<HIdent<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    resolve_hident_impl(scope, table, ident).map_err(|ident| {
        diagnostics.var_not_found(arenas, ident);
    })
}

pub fn try_resolve_module<'a, 's>(
    scope: SymbolId,
    table: &'s VSymbolTable,
    arenas: &AstArenas,
    ident: impl Into<HIdent<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<&'s ModuleSymbol, ()> {
    let ident = ident.into();
    let sid = try_resolve_hident(scope, table, arenas, ident, diagnostics)?;
    let VSymbol::Module(n) = &table[sid].content else {
        diagnostics.not_yet_implemented(hident_span(arenas, ident), "symbol is not a module");
        return Err(());
    };
    Ok(n)
}

pub fn try_resolve_net<'a, 's>(
    scope: SymbolId,
    table: &'s VSymbolTable,
    arenas: &AstArenas,
    ident: impl Into<HIdent<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<&'s NetSymbol, ()> {
    let ident = ident.into();
    let sid = try_resolve_hident(scope, table, arenas, ident, diagnostics)?;
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
    let sid = try_resolve_hident(scope, table, arenas, ident, diagnostics)?;
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
    let sid = try_resolve_hident(scope, table, arenas, ident, diagnostics)?;
    let VSymbol::Net(n) = &table[sid].content else {
        diagnostics.not_yet_implemented(hident_span(arenas, ident), "cannot be used as net");
        return Err(());
    };
    Ok((sid, n))
}

pub fn unwrap_get_fn_mut(table: &mut VSymbolTable, sid: SymbolId) -> &mut FunctionSymbol {
    let VSymbol::Function(n) = &mut table[sid].content else {
        panic!()
    };
    n
}
pub fn unwrap_get_task_mut(table: &mut VSymbolTable, sid: SymbolId) -> &mut TaskSymbol {
    let VSymbol::Task(n) = &mut table[sid].content else {
        panic!()
    };
    n
}

pub fn unwrap_get_net(table: &VSymbolTable, sid: SymbolId) -> &NetSymbol {
    let VSymbol::Net(n) = &table[sid].content else {
        panic!()
    };
    n
}
pub fn unwrap_get_net_mut(table: &mut VSymbolTable, sid: SymbolId) -> &mut NetSymbol {
    let VSymbol::Net(n) = &mut table[sid].content else {
        panic!()
    };
    n
}

pub fn unwrap_resolve_net(scope: SymbolId, table: &VSymbolTable, ident: IdentId) -> &NetSymbol {
    let sid = resolve_symbol_id(scope, table, ident).unwrap();
    unwrap_get_net(table, sid)
}
pub fn unwrap_resolve_net_mut(
    scope: SymbolId,
    table: &mut VSymbolTable,
    ident: IdentId,
) -> &mut NetSymbol {
    let sid = resolve_symbol_id(scope, table, ident).unwrap();
    unwrap_get_net_mut(table, sid)
}

pub fn unwrap_get_module(table: &VSymbolTable, sid: SymbolId) -> &ModuleSymbol {
    let VSymbol::Module(n) = &table[sid].content else {
        panic!()
    };
    n
}
pub fn unwrap_get_module_mut(table: &mut VSymbolTable, sid: SymbolId) -> &mut ModuleSymbol {
    let VSymbol::Module(n) = &mut table[sid].content else {
        panic!()
    };
    n
}

pub fn unwrap_get_param_mut(table: &mut VSymbolTable, sid: SymbolId) -> &mut VValue {
    let VSymbol::Parameter(n) = &mut table[sid].content else {
        panic!()
    };
    n
}

fn hident_span(arenas: &AstArenas, ident: HIdent) -> TokenRange {
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
    let sid = try_resolve_hident(scope, table, arenas, ident, diagnostics)?;
    let VSymbol::Parameter(value) = &table[sid].content else {
        diagnostics.not_yet_implemented(hident_span(arenas, ident), "cannot be used as constant");
        return Err(());
    };
    Ok(value)
}

use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::time::TimeResolution;
use vogls_ir::token_range::TokenRange;
use vogls_ir::vcd::{VcdScope, VcdValue, VcdVariable, VcdVariableKey};
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, LogicMode, ProcessBuilder, ProcessKey, ProcessKind,
    SignalFlags, SignalKey, VariableKey, VectorSize,
};
use vogls_utils::{IndexMap, Table, VgHashMap};

use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::Expr;
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

use self::addressing::{Address, LValueAddressingContext, lower_addressing};
pub use self::expression::eval_constant_expr;
use self::expression::lower_expr;
pub use self::vtype::VType;
pub use self::vvalue::VValue;
pub use diagnostics::{Diagnostics, LowerErrorReason};

pub fn lower_module_to_ir<'a>(
    root: AstId<'a, Module<'a>>,
    ctx: &LowerContext<'a, '_>,
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

fn assign_task_output<'a>(
    ctx: &LowerContext<'a, '_>,
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
                let symbol_key = try_resolve_hident(
                    scope,
                    &ctx.table,
                    ctx.arenas,
                    *ast_ident,
                    &mut mctx.diagnostics,
                )?;
                let VSymbol::Net(s) = &ctx.table[symbol_key].content else {
                    mctx.diagnostics
                        .output_expr_not_allowed(ctx.arenas.get_span(expr));
                    error = true;
                    continue;
                };

                let mut actx = LValueAddressingContext {
                    ctx,
                    mctx,
                    builder,
                    loc: expr.loc,
                    scope,
                };

                let Address {
                    elem_offset,
                    output_width,
                    array,
                    is_unsigned: _,
                } = lower_addressing(
                    &mut actx,
                    s.ty.bit_length(),
                    &s.dims,
                    s.transform,
                    exprs.iter(),
                    range_expression.map(|r| r.into()),
                )?;

                let partial = match (elem_offset, array) {
                    (Some(elem_offset), Some((array_offset, _array_overflow))) => {
                        Some(builder.plus(mctx.gl(), elem_offset, array_offset))
                    }
                    (Some(elem_offset), None) => Some(elem_offset),
                    (None, Some((array_offset, _array_overflow))) => Some(array_offset),
                    (None, None) => None,
                };

                let src = expression::coerce_to(
                    mctx.gl(),
                    builder,
                    variable,
                    ty,
                    s.ty.resize_net_to(output_width),
                );
                s.net.drive_blocking(mctx.gl(), builder, src, partial);
            }

            Expr::Replication(_) => {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(expr), "repetition in net assign");
                error = true;
            }

            Expr::FunctionCall(..)
            | Expr::SystemFunctionCall(..)
            | Expr::Real(..)
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

fn msb_lsb_to_width(
    gl: &GlobalContext,
    arenas: &AstArenas,
    table: &VSymbolTable,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    ast_msb: AstId<ConstantExpr>,
    ast_lsb: AstId<ConstantExpr>,
) -> Result<(i64, i64, VectorSize), ()> {
    let msb = eval_constant_expr(gl, arenas, table, scope, diagnostics, ast_msb, None);
    let lsb = eval_constant_expr(gl, arenas, table, scope, diagnostics, ast_lsb, None);

    let (Ok(msb), Ok(lsb)) = (msb, lsb) else {
        return Err(());
    };
    let (Some(msb), Some(lsb)) = (msb.as_integer(), lsb.as_integer()) else {
        let tr = arenas.get_span(ast_msb) | arenas.get_span(ast_lsb);
        diagnostics.not_yet_implemented(tr, "Did not receive signed nets");
        return Err(());
    };
    let width = u32::try_from(msb.abs_diff(lsb)).ok();
    let width = width.and_then(|w| w.checked_add(1));
    let width = width.and_then(VectorSize::new);
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
    _needs_mask: bool,
) -> (ProcessKey, SignalKey, Option<SignalKey>) {
    let needs_mask = true;
    let vogls_ir::Signal {
        name,
        origin,
        size,
        mode,
        ..
    } = &gl.signals[signal];

    let mask_name = format!("{name}::NBA_MASK");
    let value_name = format!("{name}::NBA_VALUE");
    let (size, mode, origin) = (*size, *mode, *origin);
    let (process, mut builder) =
        ProcessBuilder::new(gl, ProcessKind::NonBlockingAssignment, origin);
    let process_key = process.key().unwrap();

    let mask = needs_mask.then(|| {
        gl.signals.insert(vogls_ir::Signal {
            name: mask_name,
            size,
            initialize: None,
            flags: SignalFlags::EMPTY,
            mode: LogicMode::TwoValue,
            origin,
        })
    });
    let value = gl.signals.insert(vogls_ir::Signal {
        name: value_name,
        size,
        initialize: None,
        flags: SignalFlags::EMPTY,
        mode,
        origin,
    });

    match mask {
        None => {
            process.set_standing(gl, [value].into());
            let init_bb = builder.key();
            let value_v = builder.probe(gl, value);
            builder = builder.wait_region(gl, Region::NonBlocking as u8);
            builder.drive(gl, signal, value_v);
            builder.watch_to(gl, [value].into(), init_bb);
        }
        Some(mask) => {
            process.set_standing(gl, [mask].into());
            // We need to conditionally branch here as it might have already been assigned before.
            let init_bb = builder.key();
            builder = builder.wait_region(gl, Region::NonBlocking as u8);

            let mask_v = builder.probe(gl, mask);
            let value_v = builder.probe(gl, value);
            let inv_mask = builder.binary_not(gl, mask_v);
            let old = builder.probe(gl, signal);
            let old = builder.and(gl, old, inv_mask);
            let value_v = builder.and(gl, value_v, mask_v);
            let result = builder.or(gl, old, value_v);
            builder.drive(gl, signal, result);
            let zero_mask = builder.constant(gl, Bits::new_zeroed(size));
            builder.drive(gl, mask, zero_mask);
            builder.watch_to(gl, [mask].into(), init_bb);
        }
    }
    process.finalize(gl);

    (process_key, value, mask)
}

pub fn instantiate_nba_signals<'a>(
    gl: &mut GlobalContext,
    ctx: &mut LowerContext<'a, '_>,
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
    ctx: &mut LowerContext<'a, '_>,
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
    ctx: &mut LowerContext<'a, '_>,
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
                        &ctx.table,
                        ctx.arenas,
                        vlvalue.ident,
                        diagnostics,
                    )?;
                    let needs_mask =
                        !vlvalue.exprs.is_empty() && vlvalue.range_expression.is_some();

                    match nba_signals.entry(sid) {
                        vogls_utils::Entry::Occupied(mut entry) => entry.get().1 |= needs_mask,
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
    ctx: &mut LowerContext<'a, '_>,
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
