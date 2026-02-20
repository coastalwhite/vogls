use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, Bits, GlobalContext, INTEGER_VSIZE, ProcessKey,
    SCALAR_VSIZE, SignalKey, VariableKey, VectorSize, new_process,
};

use crate::ast::constant_expr::ConstantRangeExpression;
use crate::ast::statement::{NetLValue, NetLValueFlat, VariableLValue, VariableLValueFlat};
use crate::ast::{AstId, RangeExpression};
use crate::elaborate::VSymbol;
use crate::lower::expression::eval_constant_expr;
use crate::lower::expression::{self, lower_expr, sign_or_zero_extend, truncate_or_extend};
use crate::lower::{msb_lsb_to_width, try_resolve_symbol_id, unwrap_get_net_mut};
use crate::parser::AstArenas;

use super::{Diagnostics, Region, VType};
use super::{Scope, try_resolve_net};

pub fn assign_variable_lvalue<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<VariableLValue>,
    variable: VariableKey,
    variable_ty: VType,
    nba: bool,
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
            nba,
        );
    }

    assert!(!lvalue.0.is_empty());
    let mut total_width = 0u32;
    for lvf in lvalue.0.iter() {
        let ty = variable_lvalue_flat_ty(gl, arenas, scope, diagnostics, lvf)?;
        total_width += ty.force_net_width().get();
    }
    let variable = truncate_or_extend(
        gl,
        builder,
        variable,
        variable_ty,
        VectorSize::new(total_width).unwrap(),
    );

    let mut offset = 0u32;
    for lvf in lvalue.0.iter().rev() {
        let ty = variable_lvalue_flat_ty(gl, arenas, scope, diagnostics, lvf)?;
        let width = ty.force_net_width();
        let variable = builder.extract_constant(gl, variable, offset, width);
        assign_variable_lvalue_flat(
            gl,
            arenas,
            scope,
            diagnostics,
            builder,
            lvf,
            variable,
            ty,
            nba,
        )?;
        offset += width.get();
    }
    Ok(())
}

pub fn variable_lvalue_flat_ty<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    ast_lvalue: AstId<VariableLValueFlat>,
) -> Result<VType, ()> {
    let VariableLValueFlat {
        ident,
        exprs,
        range_expression,
    } = arenas.get(ast_lvalue);

    let symbol_key = try_resolve_symbol_id(scope.key, scope.table, arenas, *ident, diagnostics)?;

    let exprs = *exprs;
    let (mut ty, mut n_dims) = match &scope.table[symbol_key].content {
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
                let (_, _, width) =
                    msb_lsb_to_width(gl, arenas, scope.eval(), diagnostics, *msb, *lsb)?;
                Ok(VType::UnsignedNet(width))
            }
            RangeExpression::BasePlus(_, width) | RangeExpression::BaseMinus(_, width) => {
                let width = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, *width)?;
                Ok(VType::UnsignedNet(width.to_vector_size().unwrap()))
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
    nba: bool,
) -> Result<(), ()> {
    let VariableLValueFlat {
        ident,
        exprs,
        range_expression,
    } = arenas.get(ast_lvalue);

    let symbol_key = try_resolve_symbol_id(scope.key, scope.table, arenas, *ident, diagnostics)?;

    let mut exprs = *exprs;
    let (ty, dims) = match &scope.table[symbol_key].content {
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
        let (fst, fst_ty) = lower_expr(gl, arenas, scope, diagnostics, builder, fst)?;
        let fst = sign_or_zero_extend(gl, builder, fst, fst_ty, INTEGER_VSIZE);
        let mut offset = builder.multiply_constant(gl, fst, leaf_arr_items);

        while let Some(dim) = dims.last()
            && let Some(expr) = exprs.pop_front()
        {
            leaf_arr_items /= *dim;
            let (expr, expr_ty) = lower_expr(gl, arenas, scope, diagnostics, builder, expr)?;
            let expr = sign_or_zero_extend(gl, builder, expr, expr_ty, INTEGER_VSIZE);
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
        let (fst, fst_ty) = lower_expr(gl, arenas, scope, diagnostics, builder, *expr)?;
        let fst = sign_or_zero_extend(gl, builder, fst, fst_ty, INTEGER_VSIZE);
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

    match &scope.table[symbol_key].content {
        VSymbol::Net(s) => {
            let key = s.signal;
            let specify_proxy = s.specify_proxy;
            let size = ty.force_net_width();
            let partial = match range_expression {
                None => match arr_idx {
                    None => None,
                    Some(idx) => {
                        // @TODO: Verify size.
                        let idx = builder.multiply_constant(gl, idx, size.get());
                        Some((idx, size))
                    }
                },
                Some(range_expression) => {
                    let (offset, length) = match arenas.get(range_expression) {
                        RangeExpression::Expr(expr) => {
                            let (expr, expr_ty) =
                                lower_expr(gl, arenas, scope, diagnostics, builder, *expr)?;
                            let expr =
                                sign_or_zero_extend(gl, builder, expr, expr_ty, INTEGER_VSIZE);
                            (expr, SCALAR_VSIZE)
                        }
                        RangeExpression::MsbLsb(msb, lsb) => {
                            let (_, lsb, width) = msb_lsb_to_width(
                                gl,
                                arenas,
                                scope.eval(),
                                diagnostics,
                                *msb,
                                *lsb,
                            )?;
                            (
                                builder.constant(
                                    gl,
                                    Bits::new_u64(lsb as u64).truncate(INTEGER_VSIZE),
                                ),
                                width,
                            )
                        }
                        RangeExpression::BasePlus(expr, ast_width) => {
                            let (expr, expr_ty) =
                                lower_expr(gl, arenas, scope, diagnostics, builder, *expr)?;
                            let expr =
                                sign_or_zero_extend(gl, builder, expr, expr_ty, INTEGER_VSIZE);
                            let width = eval_constant_expr(
                                gl,
                                arenas,
                                scope.eval(),
                                diagnostics,
                                *ast_width,
                            )?;
                            let width = width
                                .truncate_or_extend(INTEGER_VSIZE)
                                .as_integer()
                                .unwrap() as u32;
                            let Some(width) = VectorSize::new(width) else {
                                diagnostics
                                    .not_yet_implemented(arenas.get_span(*ast_width), "zero width");
                                return Err(());
                            };
                            (expr, width)
                        }
                        RangeExpression::BaseMinus(expr, ast_width) => {
                            let (expr, expr_ty) =
                                lower_expr(gl, arenas, scope, diagnostics, builder, *expr)?;
                            let expr =
                                sign_or_zero_extend(gl, builder, expr, expr_ty, INTEGER_VSIZE);
                            let width = eval_constant_expr(
                                gl,
                                arenas,
                                scope.eval(),
                                diagnostics,
                                *ast_width,
                            )?;
                            let width = width
                                .truncate_or_extend(INTEGER_VSIZE)
                                .as_integer()
                                .unwrap() as u32;
                            let Some(width) = VectorSize::new(width) else {
                                diagnostics
                                    .not_yet_implemented(arenas.get_span(*ast_width), "zero width");
                                return Err(());
                            };
                            let width_v = builder.constant_u32(gl, width.get() - 1);
                            let lsb = builder.minus(gl, expr, width_v);
                            (lsb, width)
                        }
                    };

                    match arr_idx {
                        None => Some((offset, length)),
                        Some(idx) => {
                            // @TODO: Verify size.
                            let idx = builder.multiply_constant(gl, idx, size.get());
                            let offset = builder.plus(gl, offset, idx);
                            Some((offset, length))
                        }
                    }
                }
            };
            let size = partial.map_or(size, |(_, s)| s);
            let variable = expression::truncate_or_extend(gl, builder, variable, variable_ty, size);

            if nba {
                let s = unwrap_get_net_mut(scope.table, symbol_key);
                let (_, mask, value) = s
                    .nba
                    .get_or_insert_with(|| create_nba_process(gl, specify_proxy.unwrap_or(key)));
                let mask_value = builder.constant(gl, Bits::new_ones(size));
                builder.drive_opt_partial(gl, *mask, mask_value, partial);
                builder.drive_opt_partial(gl, *value, variable, partial);
            } else {
                builder.drive_opt_partial(gl, specify_proxy.unwrap_or(key), variable, partial);
            }
        }
        _ => todo!(),
    }
    Ok(())
}

pub fn assign_net_lvalue<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<NetLValue>,
    variable: VariableKey,
    variable_ty: VType,
) -> Result<(), ()> {
    let lvalue = arenas.get(ast_lvalue);
    if lvalue.0.len() == 1 {
        return assign_net_lvalue_flat(
            gl,
            arenas,
            scope,
            diagnostics,
            builder,
            lvalue.0.get(0),
            variable,
            variable_ty,
        );
    }

    assert!(!lvalue.0.is_empty());
    let mut total_width = 0u32;
    for lvf in lvalue.0.iter() {
        let ty = net_lvalue_flat_ty(gl, arenas, scope, diagnostics, lvf)?;
        total_width += ty.force_net_width().get();
    }
    let variable = truncate_or_extend(
        gl,
        builder,
        variable,
        variable_ty,
        VectorSize::new(total_width).unwrap(),
    );

    let mut offset = 0u32;
    for lvf in lvalue.0.iter().rev() {
        let ty = net_lvalue_flat_ty(gl, arenas, scope, diagnostics, lvf)?;
        let width = ty.force_net_width();
        let variable = builder.extract_constant(gl, variable, offset, width);
        assign_net_lvalue_flat(gl, arenas, scope, diagnostics, builder, lvf, variable, ty)?;
        offset += width.get();
    }
    Ok(())
}

pub fn net_lvalue_flat_ty<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    ast_lvalue: AstId<NetLValueFlat>,
) -> Result<VType, ()> {
    let NetLValueFlat {
        ident,
        constant_exprs,
        constant_range_expression,
    } = arenas.get(ast_lvalue);

    let symbol_key = try_resolve_symbol_id(scope.key, scope.table, arenas, *ident, diagnostics)?;

    let exprs = *constant_exprs;
    let (mut ty, mut n_dims) = match &scope.table[symbol_key].content {
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
            diagnostics.not_yet_implemented(
                arenas.get_range_span(exprs),
                "driving array without indices",
            );
            return Err(());
        }
        None => return Ok(ty),
        Some(range_expression) => match arenas.get(*range_expression) {
            ConstantRangeExpression::Single(_) => {
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
            ConstantRangeExpression::MsbLsb { msb, lsb } => {
                let (_, _, width) =
                    msb_lsb_to_width(gl, arenas, scope.eval(), diagnostics, *msb, *lsb)?;
                Ok(VType::UnsignedNet(width))
            } // RangeExpression::BasePlus(_, width) | RangeExpression::BaseMinus(_, width) => {
              //     let width = eval_constant_expr(gl, arenas, scope, diagnostics, *width)?;
              //     Ok(VType::UnsignedNet(width.to_vector_size().unwrap()))
              // }
        },
    }
}

fn assign_net_lvalue_flat<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    lvalue: AstId<NetLValueFlat>,
    variable: VariableKey,
    variable_ty: VType,
) -> Result<(), ()> {
    let NetLValueFlat {
        ident,
        constant_exprs,
        constant_range_expression,
    } = arenas.get(lvalue);

    let s = try_resolve_net(scope.key, scope.table, arenas, *ident, diagnostics)?;
    let specify_proxy = s.specify_proxy;
    let key = s.signal;
    let mut dims = &s.dims[..];

    let mut exprs = *constant_exprs;
    let mut arr_idx = if !dims.is_empty()
        && let Some(fst) = exprs.pop_front()
    {
        dims = &dims[1..];
        let mut leaf_arr_items = dims.iter().product::<u32>();
        let fst = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, fst)?;
        let fst = fst.as_integer().unwrap();
        let mut offset = fst as u32 * leaf_arr_items;

        while let Some(dim) = dims.first()
            && let Some(expr) = exprs.pop_front()
        {
            leaf_arr_items /= *dim;
            let expr = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, expr)?;
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
        diagnostics.not_yet_implemented(arenas.get_range_span(exprs), "variable_lvalue::exprs");
        return Err(());
    }

    let mut range_expression = *constant_range_expression;
    if !dims.is_empty()
        && let Some(ConstantRangeExpression::Single(expr)) = range_expression.map(|e| arenas.get(e))
    {
        _ = range_expression.take();

        dims = &dims[1..];
        let leaf_arr_items = dims.iter().product::<u32>();
        let fst = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, *expr)?;
        let fst = fst.as_integer().unwrap();
        let offset = fst as u32 * leaf_arr_items;

        arr_idx = Some(match arr_idx {
            None => offset,
            Some(arr_idx) => arr_idx + offset,
        });
    }

    if !dims.is_empty() {
        diagnostics.not_yet_implemented(
            arenas.get_range_span(exprs),
            "driving array without indices",
        );
        return Err(());
    }

    let size = s.ty.force_net_width();
    let partial = match range_expression {
        None => match arr_idx {
            None => None,
            Some(idx) => Some((builder.constant_u32(gl, idx * size.get()), size)),
        },
        Some(range_expression) => {
            let (offset, length) = match arenas.get(range_expression) {
                ConstantRangeExpression::Single(expr) => (
                    eval_constant_expr(gl, arenas, scope.eval(), diagnostics, *expr)?
                        .as_integer()
                        .unwrap(),
                    VectorSize::new(1).unwrap(),
                ),
                ConstantRangeExpression::MsbLsb { msb, lsb } => {
                    let (_, lsb, size) =
                        msb_lsb_to_width(gl, arenas, scope.eval(), diagnostics, *msb, *lsb)?;
                    (lsb, size)
                }
            };

            Some(match arr_idx {
                None => (builder.constant_u32(gl, offset as u32), length),
                Some(idx) => (
                    builder.constant_u32(gl, idx * size.get() + offset as u32),
                    length,
                ),
            })
        }
    };
    let size = partial.map_or(s.ty.force_net_width(), |(_, s)| s);
    let variable = expression::sign_or_zero_extend(gl, builder, variable, variable_ty, size);
    builder.drive_opt_partial(gl, specify_proxy.unwrap_or(key), variable, partial);
    Ok(())
}

pub fn net_lvalue_width<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    output_terminal: AstId<NetLValue>,
) -> Result<VectorSize, ()> {
    let lvalue = arenas.get(output_terminal);
    let lvalue_flat = lvalue
        .0
        .first()
        .expect("Concatenation should have at least one value");
    let mut size =
        net_lvalue_flat_ty(gl, arenas, scope, diagnostics, lvalue_flat)?.force_net_width();

    for lvalue_flat in lvalue.0.iter().skip(1) {
        let lvalue_size =
            net_lvalue_flat_ty(gl, arenas, scope, diagnostics, lvalue_flat)?.force_net_width();
        size = size.checked_add(lvalue_size.get()).ok_or_else(|| {
            diagnostics.net_width_overflow(arenas.get_span(output_terminal));
        })?;
    }
    Ok(size)
}

pub fn create_nba_process(
    gl: &mut GlobalContext,
    signal: SignalKey,
) -> (ProcessKey, SignalKey, SignalKey) {
    let vogls_ir::Signal {
        name, origin, size, ..
    } = &gl.signals[signal];

    let process_name = format!("{name}::NBA_PROC");
    let mask_name = format!("{name}::NBA_MASK");
    let value_name = format!("{name}::NBA_VALUE");
    let (size, origin) = (*size, *origin);
    let mut builder = new_process(gl, process_name, origin);

    let process_key = builder.process();
    let mask = gl.signals.insert(vogls_ir::Signal {
        name: mask_name,
        size,
        initialize: None,
        origin,
    });
    let value = gl.signals.insert(vogls_ir::Signal {
        name: value_name,
        size,
        initialize: None,
        origin,
    });

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

    gl.bbs[init_bb].terminator = BasicBlockTerminator::Branch(mask_ro, waitregion_bb, watch_bb);

    (process_key, mask, value)
}
