mod assign;
mod diagnostics;
pub mod expression;
pub mod module_or_generate_item;
mod statement;
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

pub struct Scope<'a> {
    pub table: &'a mut VSymbolTable,
    pub key: SymbolId,
    pub signal_map: &'a mut HashMap<SignalKey, SignalKey>,
}

#[derive(Clone, Copy)]
pub struct EvalScope<'a> {
    pub table: &'a VSymbolTable,
    pub key: SymbolId,
}

impl<'a> Scope<'a> {
    pub fn eval<'b>(&'b self) -> EvalScope<'b> {
        EvalScope {
            table: &self.table,
            key: self.key,
        }
    }

    fn vcd_scope(&self) -> vogls_ir::vcd::VcdScope {
        todo!()
        // self.hierarchy.vcd_scope(self.key, 0)
    }
}

pub fn resolve_symbol_id(scope: SymbolId, table: &VSymbolTable, ident: IdentId) -> Option<SymbolId> {
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

pub fn try_resolve_symbol_id(
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

pub fn try_resolve_net<'a>(
    scope: SymbolId,
    table: &'a VSymbolTable,
    arenas: &AstArenas,
    ident: AstItem<Identifier>,
    diagnostics: &mut Diagnostics,
) -> Result<&'a NetSymbol, ()> {
    let sid = try_resolve_symbol_id(scope, table, arenas, ident, diagnostics)?;
    let VSymbol::Net(n) = &table[sid].content else {
        diagnostics.not_yet_implemented(arenas.get_item_span(ident), "cannot be used as net");
        return Err(());
    };
    Ok(n)
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
pub fn unwrap_get_module_mut<'a>(table: &'a mut VSymbolTable, sid: SymbolId) -> &'a mut ModuleSymbol {
    let VSymbol::Module(n) = &mut table[sid].content else {
        panic!()
    };
    n
}

pub fn try_resolve_constant<'a>(
    scope: SymbolId,
    table: &'a VSymbolTable,
    arenas: &AstArenas,
    ident: AstItem<Identifier>,
    diagnostics: &mut Diagnostics,
) -> Result<&'a VValue, ()> {
    let sid = try_resolve_symbol_id(scope, table, arenas, ident, diagnostics)?;
    let VSymbol::Parameter(value) = &table[sid].content else {
        diagnostics.not_yet_implemented(arenas.get_item_span(ident), "cannot be used as net");
        return Err(());
    };
    Ok(value)
}

use std::collections::HashMap;

use vogls_frontend::ident_table::IdentId;
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, GlobalContext, SCALAR_VSIZE, Signal, SignalKey, VariableKey, VectorSize,
    new_process,
};

use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::expr::{BitSlice, Expr};
use crate::ast::module::{
    GenerateRegion, Module, ModuleItem, NonPortModuleItem, ParamAssignment, ParameterDeclaration,
    Range,
};
use crate::ast::{AstId, AstItem, Identifier};
use crate::elaborate::{FunctionSymbol, ModuleSymbol, NetSymbol, VSymbol, VSymbolTable};
use crate::parser::AstArenas;

pub use self::expression::eval_constant_expr;
use self::expression::{lower_expr, truncate_or_extend};
pub use self::vtype::VType;
pub use self::vvalue::VValue;
pub use diagnostics::{Diagnostics, LowerErrorReason};
pub use module_or_generate_item::dims_to_array;

pub fn lower_module_to_ir<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    root: AstId<Module>,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let Module {
        attribute_instances: _,
        module_identifier: _,
        module_parameter_port_list: _,
        ports: _,
        module_items,
        default_nettype: _,
    } = arenas.get(root);

    for i in 0..scope.table[scope.key].children().len() {
        let child = scope.table[scope.key].children()[i];
        let VSymbol::GenerateBlock(ast_ids) = &scope.table[child].content else {
            continue;
        };

        for id in ast_ids.iter() {
            let mut scope = Scope {
                table: &mut scope.table,
                key: child,
                signal_map: scope.signal_map,
            };
            module_or_generate_item::lower(gl, arenas, &mut scope, id, diagnostics)?;
        }
    }

    for module_item in module_items.iter() {
        match arenas.get(module_item) {
            ModuleItem::PortDeclaration(_) => {}
            ModuleItem::NonPortModuleItem(p) => match arenas.get(*p) {
                NonPortModuleItem::ModuleOrGenerateItem(id) => {
                    module_or_generate_item::lower(gl, arenas, scope, *id, diagnostics)?
                }
                NonPortModuleItem::GenerateRegion(region) => {
                    let GenerateRegion {
                        module_or_generate_item,
                    } = region;
                    for id in module_or_generate_item.iter() {
                        module_or_generate_item::lower(gl, arenas, scope, id, diagnostics)?;
                    }
                }
                NonPortModuleItem::SpecifyBlock => todo!(),
                NonPortModuleItem::ParameterDeclaration(id) => {
                    let ParameterDeclaration {
                        typing: _,
                        assignments,
                    } = arenas.get(*id);
                    for assignment in assignments.iter() {
                        let ParamAssignment { param: _, constant } = arenas.get(assignment);
                        let ConstantMinTypMaxExpression::Single(constant) = arenas.get(*constant)
                        else {
                            todo!();
                        };

                        let _value =
                            eval_constant_expr(arenas, scope.eval(), diagnostics, *constant)?;
                        todo!()
                        // scope.push(arenas.get_ident(param.item.0), ScopeItem::Constant(v));
                    }
                }
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

fn lower_to_signal<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    ty: VType,
) -> Result<SignalKey, ()> {
    if let Expr::Ident(ast_ident, exprs, range_expression) = arenas.get(expr)
        && exprs.is_empty()
        && range_expression.is_none()
    {
        let Ok(symbol_key) =
            try_resolve_symbol_id(scope.key, scope.table, arenas, *ast_ident, diagnostics)
        else {
            diagnostics.var_not_found(arenas, *ast_ident);
            return Err(());
        };
        if let VSymbol::Net(s) = &scope.table[symbol_key].content
            && s.ty == ty
        {
            return Ok(s.signal);
        }
    }

    let signal = gl.signals.insert(Signal {
        name: "anon_port_assignment".to_string(),
        size: ty.force_net_width(),
        initialize: None,
        origin: arenas.get_span(expr),
    });

    let mut bb_builder = new_process(gl, "port_assignment".into(), arenas.get_span(expr));
    let bb_key = bb_builder.key();
    let (v, v_ty) = lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, expr)?;
    let v = expression::sign_or_zero_extend(gl, &mut bb_builder, v, v_ty, ty.force_net_width());

    bb_builder.drive(gl, signal, v);

    bb_builder.watch_for_ins_to(gl, bb_key);
    Ok(signal)
}

fn assign_port_output<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    output_net: SymbolId,
    ty: VType,
) -> Result<(), ()> {
    if let Expr::Ident(ast_ident, exprs, range_expression) = arenas.get(expr)
        && exprs.is_empty()
        && range_expression.is_none()
    {
        let symbol_key =
            try_resolve_symbol_id(scope.key, scope.table, arenas, *ast_ident, diagnostics)?;
        if let VSymbol::Net(s) = &scope.table[symbol_key].content
            && s.ty == ty
        {
            let signal = s.signal;
            let old_signal = std::mem::replace(
                &mut unwrap_get_net_mut(scope.table, output_net).signal,
                signal,
            );
            scope.signal_map.insert(old_signal, signal);
            gl.signals.remove(old_signal);
            return Ok(());
        }
    }

    let mut driving: Vec<AstId<Expr>> = Vec::new();
    driving.push(expr);

    let signal = unwrap_get_net(scope.table, output_net).signal;

    let mut bb_builder = new_process(gl, "port_assignment".into(), arenas.get_span(expr));
    let bb_key = bb_builder.key();

    let probed = bb_builder.probe(gl, signal);

    let mut error = false;
    while let Some(expr) = driving.pop() {
        match arenas.get(expr) {
            Expr::Concatenation(_) => {
                todo!()
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                let symbol_key =
                    try_resolve_symbol_id(scope.key, scope.table, arenas, *ast_ident, diagnostics)?;
                let VSymbol::Net(s) = &scope.table[symbol_key].content else {
                    diagnostics.output_expr_not_allowed(arenas.get_span(expr));
                    error = true;
                    continue;
                };

                let (offset_dst, length_dst) = if range_expression.is_none() && exprs.is_empty() {
                    (bb_builder.constant_u32(gl, 0), Some(s.ty.force_net_width()))
                } else if range_expression.is_none() && exprs.len() == 1 {
                    (
                        lower_expr(
                            gl,
                            arenas,
                            scope,
                            diagnostics,
                            &mut bb_builder,
                            exprs.first().unwrap(),
                        )?
                        .0,
                        None,
                    )
                } else if let Some(slice) = range_expression
                    && exprs.is_empty()
                {
                    match slice {
                        BitSlice::MsbLsb(msb, lsb) => {
                            let (_, lsb, width) =
                                msb_lsb_to_width(arenas, scope.eval(), diagnostics, *msb, *lsb)?;
                            let offset = bb_builder.constant_u32(gl, lsb as u32);
                            (offset, Some(width as VectorSize))
                        }
                        BitSlice::PlusWidth(base, width) => {
                            let offset =
                                lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, *base);
                            let width =
                                eval_constant_expr(arenas, scope.eval(), diagnostics, *width);
                            let width = width?.as_integer().unwrap();
                            (offset?.0, Some(VectorSize::new(width as u32).unwrap()))
                        }
                        BitSlice::MinusWidth(base, width) => {
                            let offset =
                                lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, *base);
                            let width =
                                eval_constant_expr(arenas, scope.eval(), diagnostics, *width)?;
                            let width =
                                VectorSize::new(width.as_integer().unwrap() as u32).unwrap();
                            let width_v =
                                bb_builder.constant_u32(gl, width.checked_add(1).unwrap().get());
                            let offset = bb_builder.minus(gl, offset?.0, width_v);
                            (offset, Some(width))
                        }
                    }
                } else {
                    diagnostics.not_yet_implemented(arenas.get_span(expr), "multiple braced");
                    error = true;
                    continue;
                };

                let length_dst = length_dst.unwrap_or(SCALAR_VSIZE);
                let src = probed;
                let src = truncate_or_extend(gl, &mut bb_builder, src, ty, length_dst);
                bb_builder.drive_partial(gl, s.signal, src, offset_dst, length_dst);
            }

            Expr::Replication(_) => {
                diagnostics.not_yet_implemented(arenas.get_span(expr), "repetition in net assign");
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
                diagnostics.output_expr_not_allowed(arenas.get_span(expr));
                error = true;
            }
        }
    }

    bb_builder.watch_for_ins_to(gl, bb_key);

    if error {
        return Err(());
    }

    Ok(())
}

fn assign_task_output<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    variable: VariableKey,
    expr: AstId<Expr>,
    ty: VType,
) -> Result<(), ()> {
    let mut driving: Vec<AstId<Expr>> = Vec::new();
    driving.push(expr);

    let mut error = false;
    while let Some(expr) = driving.pop() {
        match arenas.get(expr) {
            Expr::Concatenation(_) => {
                todo!()
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                let symbol_key =
                    try_resolve_symbol_id(scope.key, scope.table, arenas, *ast_ident, diagnostics)?;
                let VSymbol::Net(s) = &scope.table[symbol_key].content else {
                    diagnostics.output_expr_not_allowed(arenas.get_span(expr));
                    error = true;
                    continue;
                };

                let (offset_dst, length_dst) = if range_expression.is_none() && exprs.is_empty() {
                    (builder.constant_u32(gl, 0), Some(s.ty.force_net_width()))
                } else if range_expression.is_none() && exprs.len() == 1 {
                    (
                        lower_expr(
                            gl,
                            arenas,
                            scope,
                            diagnostics,
                            builder,
                            exprs.first().unwrap(),
                        )?
                        .0,
                        None,
                    )
                } else if let Some(slice) = range_expression
                    && exprs.is_empty()
                {
                    match slice {
                        BitSlice::MsbLsb(msb, lsb) => {
                            let (_, lsb, width) =
                                msb_lsb_to_width(arenas, scope.eval(), diagnostics, *msb, *lsb)?;
                            let offset = builder.constant_u32(gl, lsb as u32);
                            (offset, Some(width as VectorSize))
                        }
                        BitSlice::PlusWidth(base, width) => {
                            let offset = lower_expr(gl, arenas, scope, diagnostics, builder, *base);
                            let width =
                                eval_constant_expr(arenas, scope.eval(), diagnostics, *width);
                            let width = width?.as_integer().unwrap();
                            (offset?.0, Some(VectorSize::new(width as u32).unwrap()))
                        }
                        BitSlice::MinusWidth(base, width) => {
                            let offset = lower_expr(gl, arenas, scope, diagnostics, builder, *base);
                            let width =
                                eval_constant_expr(arenas, scope.eval(), diagnostics, *width)?;
                            let width =
                                VectorSize::new(width.as_integer().unwrap() as u32).unwrap();
                            let width_v =
                                builder.constant_u32(gl, width.checked_add(1).unwrap().get());
                            let offset = builder.minus(gl, offset?.0, width_v);
                            (offset, Some(width))
                        }
                    }
                } else {
                    diagnostics.not_yet_implemented(arenas.get_span(expr), "multiple braced");
                    error = true;
                    continue;
                };

                let length_dst = length_dst.unwrap_or(SCALAR_VSIZE);
                let src = truncate_or_extend(gl, builder, variable, ty, length_dst);
                builder.drive_partial(gl, s.signal, src, offset_dst, length_dst);
            }

            Expr::Replication(_) => {
                diagnostics.not_yet_implemented(arenas.get_span(expr), "repetition in net assign");
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
                diagnostics.output_expr_not_allowed(arenas.get_span(expr));
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
    arenas: &'a AstArenas,
    scope: EvalScope<'a>,
    diagnostics: &mut Diagnostics,
    ast_msb: AstId<ConstantExpr>,
    ast_lsb: AstId<ConstantExpr>,
) -> Result<(i64, i64, VectorSize), ()> {
    let msb = eval_constant_expr(arenas, scope, diagnostics, ast_msb);
    let lsb = eval_constant_expr(arenas, scope, diagnostics, ast_lsb);

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
    arenas: &'a AstArenas,
    scope: EvalScope<'a>,
    diagnostics: &mut Diagnostics,
    range: AstId<Range>,
) -> Result<(i64, i64, VectorSize), ()> {
    let range = arenas.get(range);
    msb_lsb_to_width(arenas, scope, diagnostics, range.msb, range.lsb)
}
