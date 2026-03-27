use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, Bits, INTEGER_VSIZE, SCALAR_VSIZE, VariableKey, VectorSize};

use crate::ast::constant_expr::ConstantRangeExpression;
use crate::ast::statement::{NetLValue, NetLValueFlat, VariableLValue, VariableLValueFlat};
use crate::ast::{AstId, RangeExpression};
use crate::elaborate::VSymbol;
use crate::lower::expression::eval_constant_expr;
use crate::lower::expression::{self, lower_expr, sign_or_zero_extend, truncate_or_extend};
use crate::lower::{msb_lsb_to_width, try_resolve_symbol_id};

use super::{LowerContext, try_resolve_net};
use super::{MutLowerContext, VType};

pub fn assign_variable_lvalue<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<'a, VariableLValue<'a>>,
    variable: VariableKey,
    variable_ty: VType,
    nba: bool,
) -> Result<(), ()> {
    if ast_lvalue.0.len() == 1 {
        return assign_variable_lvalue_flat(
            ctx,
            mctx,
            scope,
            builder,
            ast_lvalue.0.get(0),
            variable,
            variable_ty,
            nba,
        );
    }

    assert!(!ast_lvalue.0.is_empty());
    let mut total_width = 0u32;
    for lvf in ast_lvalue.0.iter() {
        let ty = variable_lvalue_flat_ty(ctx, mctx, scope, lvf)?;
        total_width += ty.force_net_width().get();
    }
    let variable = truncate_or_extend(
        mctx.gl(),
        builder,
        variable,
        variable_ty,
        VectorSize::new(total_width).unwrap(),
    );

    let mut offset = 0u32;
    for lvf in ast_lvalue.0.iter().rev() {
        let ty = variable_lvalue_flat_ty(ctx, mctx, scope, lvf)?;
        let width = ty.force_net_width();
        let variable = builder.slice_constant(mctx.gl(), variable, offset, width);
        assign_variable_lvalue_flat(ctx, mctx, scope, builder, lvf, variable, ty, nba)?;
        offset += width.get();
    }
    Ok(())
}

pub fn variable_lvalue_flat_ty<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    ast_lvalue: AstId<'a, VariableLValueFlat<'a>>,
) -> Result<VType, ()> {
    let VariableLValueFlat {
        ident,
        exprs,
        range_expression,
    } = &*ast_lvalue;

    let symbol_key = try_resolve_symbol_id(
        scope,
        &ctx.table,
        &ctx.arenas,
        *ident,
        &mut mctx.diagnostics,
    )?;

    let exprs = *exprs;
    let (mut ty, mut n_dims) = match &ctx.table[symbol_key].content {
        VSymbol::Parameter(v) => (v.ty(), 0),
        VSymbol::Net(s) => (s.ty, s.dims.len()),
        _ => todo!(),
    };

    if exprs.len() > n_dims {
        ty = VType::SCALAR_NET;
    }
    n_dims = n_dims.saturating_sub(exprs.len());

    match range_expression {
        None if n_dims > 0 => {
            mctx.diagnostics.not_yet_implemented(
                ctx.arenas.get_range_span(exprs),
                "driving array without indices",
            );
            return Err(());
        }
        None => return Ok(ty),
        Some(range_expression) => match &**range_expression {
            RangeExpression::Expr(_) => {
                if n_dims > 1 {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_range_span(exprs),
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
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_range_span(exprs),
                    "driving array without indices",
                );
                Err(())
            }
            RangeExpression::MsbLsb(msb, lsb) => {
                let (_, _, width) = msb_lsb_to_width(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *msb,
                    *lsb,
                )?;
                Ok(VType::UnsignedNet(width))
            }
            RangeExpression::BasePlus(_, width) | RangeExpression::BaseMinus(_, width) => {
                let width = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *width,
                )?;
                Ok(VType::UnsignedNet(width.ty().force_net_width()))
            }
        },
    }
}

pub fn assign_variable_lvalue_flat<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<'a, VariableLValueFlat<'a>>,
    variable: VariableKey,
    variable_ty: VType,
    nba: bool,
) -> Result<(), ()> {
    let VariableLValueFlat {
        ident,
        exprs,
        range_expression,
    } = &*ast_lvalue;

    let symbol_key = try_resolve_symbol_id(
        scope,
        &ctx.table,
        &ctx.arenas,
        *ident,
        &mut mctx.diagnostics,
    )?;

    let mut exprs = *exprs;
    let (ty, dims) = match &ctx.table[symbol_key].content {
        VSymbol::Parameter(v) => (v.ty(), [].into()),
        VSymbol::Net(s) => (s.ty.clone(), s.dims.clone()),
        v => panic!("{v:?}"),
    };
    let mut dims = &dims[..];
    let mut arr_idx = if !dims.is_empty()
        && let Some(fst) = exprs.pop_front()
    {
        dims = &dims[..dims.len() - 1];
        let mut leaf_arr_items = dims.iter().product::<u32>();
        let (fst, fst_ty) = lower_expr(ctx, mctx, scope, builder, fst)?;
        let fst = sign_or_zero_extend(mctx.gl(), builder, fst, fst_ty, INTEGER_VSIZE);
        let mut offset = builder.multiply_constant(mctx.gl(), fst, Bits::new_u32(leaf_arr_items));

        while let Some(dim) = dims.last()
            && let Some(expr) = exprs.pop_front()
        {
            leaf_arr_items /= *dim;
            let (expr, expr_ty) = lower_expr(ctx, mctx, scope, builder, expr)?;
            let expr = sign_or_zero_extend(mctx.gl(), builder, expr, expr_ty, INTEGER_VSIZE);
            let expr = builder.multiply_constant(mctx.gl(), expr, Bits::new_u32(leaf_arr_items));
            offset = builder.plus(mctx.gl(), offset, expr);
            dims = &dims[1..];
        }

        Some(offset)
    } else {
        None
    };
    if !exprs.is_empty() {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_range_span(exprs), "variable_lvalue::exprs");
        return Err(());
    }

    let mut range_expression = *range_expression;
    if !dims.is_empty()
        && let Some(RangeExpression::Expr(expr)) = range_expression.map(|e| *e)
    {
        _ = range_expression.take();

        dims = &dims[..dims.len() - 1];
        let leaf_arr_items = dims.iter().product::<u32>();
        let (fst, fst_ty) = lower_expr(ctx, mctx, scope, builder, expr)?;
        let fst = sign_or_zero_extend(mctx.gl(), builder, fst, fst_ty, INTEGER_VSIZE);
        let offset = builder.multiply_constant(mctx.gl(), fst, Bits::new_u32(leaf_arr_items));

        arr_idx = Some(match arr_idx {
            None => offset,
            Some(arr_idx) => builder.plus(mctx.gl(), arr_idx, offset),
        });
    }

    if !dims.is_empty() {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_range_span(exprs),
            "driving array without indices",
        );
        return Err(());
    }

    match &ctx.table[symbol_key].content {
        VSymbol::Net(s) => {
            let net = &s.net;
            let size = ty.force_net_width();
            let partial = match range_expression {
                None => match arr_idx {
                    None => None,
                    Some(idx) => {
                        // @TODO: Verify size.
                        let idx =
                            builder.multiply_constant(mctx.gl(), idx, Bits::new_u32(size.get()));
                        Some((idx, size))
                    }
                },
                Some(range_expression) => {
                    let (offset, length) = match &*range_expression {
                        RangeExpression::Expr(expr) => {
                            let (expr, expr_ty) = lower_expr(ctx, mctx, scope, builder, *expr)?;
                            let expr = sign_or_zero_extend(
                                mctx.gl(),
                                builder,
                                expr,
                                expr_ty,
                                INTEGER_VSIZE,
                            );
                            (expr, SCALAR_VSIZE)
                        }
                        RangeExpression::MsbLsb(msb, lsb) => {
                            let (_, lsb, width) = msb_lsb_to_width(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                *msb,
                                *lsb,
                            )?;
                            (
                                builder.constant(
                                    mctx.gl(),
                                    Bits::new_u64(lsb as u64).truncate(INTEGER_VSIZE),
                                ),
                                width,
                            )
                        }
                        RangeExpression::BasePlus(expr, ast_width) => {
                            let (expr, expr_ty) = lower_expr(ctx, mctx, scope, builder, *expr)?;
                            let expr = sign_or_zero_extend(
                                mctx.gl(),
                                builder,
                                expr,
                                expr_ty,
                                INTEGER_VSIZE,
                            );
                            let width = eval_constant_expr(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                *ast_width,
                            )?;
                            let width = width
                                .truncate_or_extend(INTEGER_VSIZE)
                                .as_integer()
                                .unwrap() as u32;
                            let Some(width) = VectorSize::new(width) else {
                                mctx.diagnostics.not_yet_implemented(
                                    ctx.arenas.get_span(*ast_width),
                                    "zero width",
                                );
                                return Err(());
                            };
                            (expr, width)
                        }
                        RangeExpression::BaseMinus(expr, ast_width) => {
                            let (expr, expr_ty) = lower_expr(ctx, mctx, scope, builder, *expr)?;
                            let expr = sign_or_zero_extend(
                                mctx.gl(),
                                builder,
                                expr,
                                expr_ty,
                                INTEGER_VSIZE,
                            );
                            let width = eval_constant_expr(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                *ast_width,
                            )?;
                            let width = width
                                .truncate_or_extend(INTEGER_VSIZE)
                                .as_integer()
                                .unwrap() as u32;
                            let Some(width) = VectorSize::new(width) else {
                                mctx.diagnostics.not_yet_implemented(
                                    ctx.arenas.get_span(*ast_width),
                                    "zero width",
                                );
                                return Err(());
                            };
                            let width_v = builder.constant_u32(mctx.gl(), width.get() - 1);
                            let lsb = builder.minus(mctx.gl(), expr, width_v);
                            (lsb, width)
                        }
                    };

                    match arr_idx {
                        None => Some((offset, length)),
                        Some(idx) => {
                            // @TODO: Verify size.
                            let idx = builder.multiply_constant(
                                mctx.gl(),
                                idx,
                                Bits::new_u32(size.get()),
                            );
                            let offset = builder.plus(mctx.gl(), offset, idx);
                            Some((offset, length))
                        }
                    }
                }
            };
            let size = partial.map_or(size, |(_, s)| s);
            let variable =
                expression::truncate_or_extend(mctx.gl(), builder, variable, variable_ty, size);

            if nba {
                net.drive_non_blocking(mctx.gl(), builder, variable, partial);
            } else {
                net.drive_blocking(mctx.gl(), builder, variable, partial);
            }
        }
        _ => todo!(),
    }
    Ok(())
}

pub fn assign_net_lvalue<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<'a, NetLValue<'a>>,
    variable: VariableKey,
    variable_ty: VType,
) -> Result<(), ()> {
    let lvalue = &*ast_lvalue;
    if lvalue.0.len() == 1 {
        return assign_net_lvalue_flat(
            ctx,
            mctx,
            scope,
            builder,
            lvalue.0.get(0),
            variable,
            variable_ty,
        );
    }

    assert!(!lvalue.0.is_empty());
    let mut total_width = 0u32;
    for lvf in lvalue.0.iter() {
        let ty = net_lvalue_flat_ty(ctx, mctx, scope, lvf)?;
        total_width += ty.force_net_width().get();
    }
    let variable = truncate_or_extend(
        mctx.gl(),
        builder,
        variable,
        variable_ty,
        VectorSize::new(total_width).unwrap(),
    );

    let mut offset = 0u32;
    for lvf in lvalue.0.iter().rev() {
        let ty = net_lvalue_flat_ty(ctx, mctx, scope, lvf)?;
        let width = ty.force_net_width();
        let variable = builder.slice_constant(mctx.gl(), variable, offset, width);
        assign_net_lvalue_flat(ctx, mctx, scope, builder, lvf, variable, ty)?;
        offset += width.get();
    }
    Ok(())
}

pub fn net_lvalue_flat_ty<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    ast_lvalue: AstId<'a, NetLValueFlat<'a>>,
) -> Result<VType, ()> {
    let NetLValueFlat {
        ident,
        constant_exprs,
        constant_range_expression,
    } = &*ast_lvalue;

    let symbol_key = try_resolve_symbol_id(
        scope,
        &ctx.table,
        &ctx.arenas,
        *ident,
        &mut mctx.diagnostics,
    )?;

    let exprs = *constant_exprs;
    let (mut ty, mut n_dims) = match &ctx.table[symbol_key].content {
        VSymbol::Parameter(v) => (v.ty(), 0),
        VSymbol::Net(s) => (s.ty, s.dims.len()),
        _ => todo!(),
    };

    if exprs.len() > n_dims {
        ty = VType::SCALAR_NET;
    }
    n_dims = n_dims.saturating_sub(exprs.len());

    match constant_range_expression {
        None if n_dims > 0 => {
            mctx.diagnostics.not_yet_implemented(
                ctx.arenas.get_range_span(exprs),
                "driving array without indices",
            );
            return Err(());
        }
        None => return Ok(ty),
        Some(range_expression) => match &**range_expression {
            ConstantRangeExpression::Single(_) => {
                if n_dims > 1 {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_range_span(exprs),
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
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_range_span(exprs),
                    "driving array without indices",
                );
                Err(())
            }
            ConstantRangeExpression::MsbLsb { msb, lsb } => {
                let (_, _, width) = msb_lsb_to_width(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *msb,
                    *lsb,
                )?;
                Ok(VType::UnsignedNet(width))
            } // RangeExpression::BasePlus(_, width) | RangeExpression::BaseMinus(_, width) => {
              //     let width = eval_constant_expr(gl, arenas, scope, diagnostics, *width)?;
              //     Ok(VType::UnsignedNet(width.to_vector_size().unwrap()))
              // }
        },
    }
}

fn assign_net_lvalue_flat<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    lvalue: AstId<'a, NetLValueFlat<'a>>,
    variable: VariableKey,
    variable_ty: VType,
) -> Result<(), ()> {
    let NetLValueFlat {
        ident,
        constant_exprs,
        constant_range_expression,
    } = &*lvalue;

    let s = try_resolve_net(
        scope,
        &ctx.table,
        &ctx.arenas,
        *ident,
        &mut mctx.diagnostics,
    )?;
    let mut dims = &s.dims[..];

    let mut exprs = *constant_exprs;
    let mut arr_idx = if !dims.is_empty()
        && let Some(fst) = exprs.pop_front()
    {
        dims = &dims[1..];
        let mut leaf_arr_items = dims.iter().product::<u32>();
        let fst = eval_constant_expr(
            &mctx.gl,
            &ctx.arenas,
            &ctx.table,
            scope,
            &mut mctx.diagnostics,
            fst,
        )?;
        let fst = fst.as_integer().unwrap();
        let mut offset = fst as u32 * leaf_arr_items;

        while let Some(dim) = dims.first()
            && let Some(expr) = exprs.pop_front()
        {
            leaf_arr_items /= *dim;
            let expr = eval_constant_expr(
                &mctx.gl,
                &ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                expr,
            )?;
            let expr = expr.as_integer().unwrap();
            let expr = expr as u32 * leaf_arr_items;
            offset += expr;
            dims = &dims[1..];
        }

        Some(offset)
    } else {
        None
    };
    if !exprs.is_empty() {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_range_span(exprs), "variable_lvalue::exprs");
        return Err(());
    }

    let mut range_expression = *constant_range_expression;
    if !dims.is_empty()
        && let Some(ConstantRangeExpression::Single(expr)) = range_expression.map(|e| *e)
    {
        _ = range_expression.take();

        dims = &dims[1..];
        let leaf_arr_items = dims.iter().product::<u32>();
        let fst = eval_constant_expr(
            &mctx.gl,
            &ctx.arenas,
            &ctx.table,
            scope,
            &mut mctx.diagnostics,
            expr,
        )?;
        let fst = fst.as_integer().unwrap();
        let offset = fst as u32 * leaf_arr_items;

        arr_idx = Some(match arr_idx {
            None => offset,
            Some(arr_idx) => arr_idx + offset,
        });
    }

    if !dims.is_empty() {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_range_span(exprs),
            "driving array without indices",
        );
        return Err(());
    }

    let size = s.ty.force_net_width();
    let partial = match range_expression {
        None => match arr_idx {
            None => None,
            Some(idx) => Some((builder.constant_u32(mctx.gl(), idx * size.get()), size)),
        },
        Some(range_expression) => {
            let (offset, length) = match &*range_expression {
                ConstantRangeExpression::Single(expr) => (
                    eval_constant_expr(
                        &mctx.gl,
                        &ctx.arenas,
                        &ctx.table,
                        scope,
                        &mut mctx.diagnostics,
                        *expr,
                    )?
                    .as_integer()
                    .unwrap(),
                    VectorSize::new(1).unwrap(),
                ),
                ConstantRangeExpression::MsbLsb { msb, lsb } => {
                    let (_, lsb, size) = msb_lsb_to_width(
                        &mctx.gl,
                        &ctx.arenas,
                        &ctx.table,
                        scope,
                        &mut mctx.diagnostics,
                        *msb,
                        *lsb,
                    )?;
                    (lsb, size)
                }
            };

            Some(match arr_idx {
                None => (builder.constant_u32(mctx.gl(), offset as u32), length),
                Some(idx) => (
                    builder.constant_u32(mctx.gl(), idx * size.get() + offset as u32),
                    length,
                ),
            })
        }
    };
    let size = partial.map_or(s.ty.force_net_width(), |(_, s)| s);
    let variable = expression::sign_or_zero_extend(mctx.gl(), builder, variable, variable_ty, size);
    s.net.drive_blocking(mctx.gl(), builder, variable, partial);
    Ok(())
}

pub fn net_lvalue_width<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    output_terminal: AstId<'a, NetLValue<'a>>,
) -> Result<VectorSize, ()> {
    let lvalue = &*output_terminal;
    let lvalue_flat = lvalue
        .0
        .first()
        .expect("Concatenation should have at least one value");
    let mut size = net_lvalue_flat_ty(ctx, mctx, scope, lvalue_flat)?.force_net_width();

    for lvalue_flat in lvalue.0.iter().skip(1) {
        let lvalue_size = net_lvalue_flat_ty(ctx, mctx, scope, lvalue_flat)?.force_net_width();
        size = size.checked_add(lvalue_size.get()).ok_or_else(|| {
            mctx.diagnostics
                .net_width_overflow(ctx.arenas.get_span(output_terminal));
        })?;
    }
    Ok(size)
}
