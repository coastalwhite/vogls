use vogls_ir::{BasicBlockBuilder, Bits, GlobalContext, VariableKey};

use crate::ast::statement::{VariableLValue, VariableLValueFlat};
use crate::ast::{AstId, RangeExpression};
use crate::lower::constant_expr::eval_constant_expr;
use crate::lower::expression::{self, lower_expr, sign_extend_or_truncate};
use crate::lower::msb_lsb_to_width;
use crate::lower::scope::SymbolVariant;
use crate::parser::AstArenas;

use super::scope::Scope;
use super::{Diagnostics, Region, VType};

pub fn assign_variable_lvalue<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<VariableLValue>,
    variable: VariableKey,
    variable_ty: VType,
    region: Region,
) -> Result<(), ()> {
    let lvalue = arenas.get(ast_lvalue);
    if lvalue.0.len() == 1 {
        return assign_variable_lvalue_flat(
            gl,
            arenas,
            scope,
            diagnostics,
            builder,
            lvalue.0.get(0),
            variable,
            variable_ty,
            region,
        );
    }

    assert!(!lvalue.0.is_empty());
    let mut offset = 0u32;
    for lvf in lvalue.0.iter().rev() {
        let ty = variable_lvalue_flat_ty(gl, arenas, scope, diagnostics, lvf)?;
        let width = ty.force_net_width();
        let voffset = builder.constant_u32(gl, offset);
        let variable = builder.logical_shift_right(gl, variable, voffset);
        let variable = sign_extend_or_truncate(gl, builder, variable, variable_ty, width);
        assign_variable_lvalue_flat(
            gl,
            arenas,
            scope,
            diagnostics,
            builder,
            lvf,
            variable,
            ty,
            region,
        )?;
        offset += width;
    }
    Ok(())
}

pub fn variable_lvalue_flat_ty<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    ast_lvalue: AstId<VariableLValueFlat>,
) -> Result<VType, ()> {
    let VariableLValueFlat {
        ident,
        exprs,
        range_expression,
    } = arenas.get(ast_lvalue);

    let lvalue_ident = arenas.get_ident(ident.item.0);
    let Some(symbol_key) = scope.get(&lvalue_ident) else {
        diagnostics.var_not_found(arenas, *ident);
        return Err(());
    };

    let exprs = *exprs;
    let (mut ty, mut n_dims) = match &scope.symbols[symbol_key].variant {
        SymbolVariant::Genvar(_) => (VType::Integer, 0),
        SymbolVariant::Constant(v) => (v.ty(), 0),
        SymbolVariant::Task(_) => todo!(),
        SymbolVariant::Signal(dims, ty, _key) => (*ty, dims.len()),
    };

    if exprs.len() > n_dims {
        ty = VType::SCALAR_NET;
    }
    n_dims = n_dims.saturating_sub(exprs.len());

    match range_expression {
        None if n_dims > 0 => {
            diagnostics.not_yet_implemented(
                arenas.get_range_span(exprs),
                "driving array without indices",
            );
            return Err(());
        }
        None => return Ok(ty),
        Some(range_expression) => match arenas.get(*range_expression) {
            RangeExpression::Expr(_) => {
                if n_dims > 1 {
                    diagnostics.not_yet_implemented(
                        arenas.get_range_span(exprs),
                        "driving array without indices",
                    );
                    Err(())
                } else if n_dims == 1 {
                    Ok(ty)
                } else {
                    Ok(VType::SCALAR_NET)
                }
            }
            _ if n_dims > 0 => {
                diagnostics.not_yet_implemented(
                    arenas.get_range_span(exprs),
                    "driving array without indices",
                );
                Err(())
            }
            RangeExpression::MsbLsb(msb, lsb) => {
                let (_, _, width) = msb_lsb_to_width(gl, arenas, scope, diagnostics, *msb, *lsb)?;
                Ok(VType::Net(width))
            }
            RangeExpression::BasePlus(_, width) | RangeExpression::BaseMinus(_, width) => {
                let width = eval_constant_expr(gl, arenas, scope, diagnostics, *width)?;
                Ok(VType::Net(width.to_vector_size().unwrap()))
            }
        },
    }
}

pub fn assign_variable_lvalue_flat<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<VariableLValueFlat>,
    variable: VariableKey,
    variable_ty: VType,
    region: Region,
) -> Result<(), ()> {
    let VariableLValueFlat {
        ident,
        exprs,
        range_expression,
    } = arenas.get(ast_lvalue);

    let lvalue_ident = arenas.get_ident(ident.item.0);
    let Some(symbol_key) = scope.get(&lvalue_ident) else {
        diagnostics.var_not_found(arenas, *ident);
        return Err(());
    };

    let mut exprs = *exprs;
    let (ty, dims) = match &scope.symbols[symbol_key].variant {
        SymbolVariant::Genvar(_) => (VType::Integer, Vec::new()),
        SymbolVariant::Constant(v) => (v.ty(), Vec::new()),
        SymbolVariant::Task(_) => todo!(),
        SymbolVariant::Signal(dims, ty, key) => (*ty, dims.clone()),
    };
    let mut dims = &dims[..];
    let mut arr_idx = if !dims.is_empty()
        && let Some(fst) = exprs.pop_front()
    {
        dims = &dims[..dims.len() - 1];
        let mut leaf_arr_items = dims.iter().product::<u32>();
        let (fst, _) = lower_expr(gl, arenas, scope, diagnostics, builder, fst)?;
        let mut offset = builder.multiply_constant(gl, fst, leaf_arr_items);

        while let Some(dim) = dims.last()
            && let Some(expr) = exprs.pop_front()
        {
            leaf_arr_items /= *dim;
            let (expr, _) = lower_expr(gl, arenas, scope, diagnostics, builder, expr)?;
            let expr = builder.multiply_constant(gl, expr, leaf_arr_items);
            offset = builder.plus(gl, offset, expr);
            dims = &dims[1..];
        }

        Some(offset)
    } else {
        None
    };
    if !exprs.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_range_span(exprs), "variable_lvalue::exprs");
        return Err(());
    }

    let mut range_expression = *range_expression;
    if !dims.is_empty()
        && let Some(RangeExpression::Expr(expr)) = range_expression.map(|e| arenas.get(e))
    {
        _ = range_expression.take();

        dims = &dims[..dims.len() - 1];
        let leaf_arr_items = dims.iter().product::<u32>();
        let (fst, _) = lower_expr(gl, arenas, scope, diagnostics, builder, *expr)?;
        let offset = builder.multiply_constant(gl, fst, leaf_arr_items);

        arr_idx = Some(match arr_idx {
            None => offset,
            Some(arr_idx) => builder.plus(gl, arr_idx, offset),
        });
    }

    if !dims.is_empty() {
        diagnostics.not_yet_implemented(
            arenas.get_range_span(exprs),
            "driving array without indices",
        );
        return Err(());
    }

    match &mut scope.symbols[symbol_key].variant {
        SymbolVariant::Constant(_) => todo!(),
        SymbolVariant::Genvar(_) => todo!(),
        SymbolVariant::Task(_) => todo!(),
        SymbolVariant::Signal(_dims, _ty, key) => {
            let key = *key;
            let size = ty.force_net_width();
            let partial = match range_expression {
                None => match arr_idx {
                    None => None,
                    Some(idx) => {
                        // @TODO: Verify size.
                        let idx = builder.cast(gl, idx, 32);
                        let idx = builder.multiply_constant(gl, idx, size);
                        Some((idx, size))
                    }
                },
                Some(range_expression) => {
                    let (offset, length) = match arenas.get(range_expression) {
                        RangeExpression::Expr(expr) => (
                            lower_expr(gl, arenas, scope, diagnostics, builder, *expr)?.0,
                            1,
                        ),
                        RangeExpression::MsbLsb(msb, lsb) => {
                            let (_, lsb, width) =
                                msb_lsb_to_width(gl, arenas, scope, diagnostics, *msb, *lsb)?;
                            (
                                builder.constant(gl, Bits::from_i64_truncated(lsb as i64, 32)),
                                width,
                            )
                        }
                        RangeExpression::BasePlus(_, _) => todo!("BasePlus"),
                        RangeExpression::BaseMinus(_, _) => todo!("BaseMinus"),
                    };

                    match arr_idx {
                        None => Some((offset, length)),
                        Some(idx) => {
                            // @TODO: Verify size.
                            let idx = builder.cast(gl, idx, 32);
                            let idx = builder.multiply_constant(gl, idx, size);
                            let offset = builder.cast(gl, offset, 32);
                            let offset = builder.plus(gl, offset, idx);
                            Some((offset, length))
                        }
                    }
                }
            };
            let size = partial.map_or(size, |(_, s)| s);
            let variable =
                expression::sign_extend_or_truncate(gl, builder, variable, variable_ty, size);
            builder.regioned_drive_opt_partial(gl, key, variable, region as u8, partial);
        }
    }
    Ok(())
}
