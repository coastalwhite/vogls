use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, VariableKey, VectorSize,
};

use crate::ast::constant_expr::ConstantRangeExpression;
use crate::ast::statement::{NetLValue, NetLValueFlat, VariableLValue, VariableLValueFlat};
use crate::ast::{AstId, RangeExpression};
use crate::hierarchy::{HierarchyItem, HierarchyNet, HierarchyParameter};
use crate::lower::expression::eval_constant_expr;
use crate::lower::expression::{self, lower_expr, sign_or_zero_extend, truncate_or_extend};
use crate::lower::msb_lsb_to_width;
use crate::parser::AstArenas;

use super::Scope;
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
    let mut total_width = 0u32;
    for lvf in lvalue.0.iter() {
        let ty = variable_lvalue_flat_ty(arenas, scope, diagnostics, lvf)?;
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
        let ty = variable_lvalue_flat_ty(arenas, scope, diagnostics, lvf)?;
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
            region,
        )?;
        offset += width.get();
    }
    Ok(())
}

pub fn variable_lvalue_flat_ty<'a>(
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
    let (mut ty, mut n_dims) = match &scope.hierarchy.items()[symbol_key.as_idx()] {
        HierarchyItem::GenVar(_) => todo!(),
        HierarchyItem::Parameter(v) => {
            let p = &scope.hierarchy.parameters()[*v];
            (p.value.ty(), 0)
        }
        HierarchyItem::Net(s) => {
            let n = &scope.hierarchy.net()[*s];
            (n.ty, n.dims.len())
        }
        HierarchyItem::Module(_)
        | HierarchyItem::NamedBlock(_)
        | HierarchyItem::Task(_)
        | HierarchyItem::Function(_) => todo!(),
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
                let (_, _, width) = msb_lsb_to_width(arenas, scope.eval(), diagnostics, *msb, *lsb)?;
                Ok(VType::UnsignedNet(width))
            }
            RangeExpression::BasePlus(_, width) | RangeExpression::BaseMinus(_, width) => {
                let width = eval_constant_expr(arenas, scope.eval(), diagnostics, *width)?;
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
    let (ty, dims) = match &scope.hierarchy.items()[symbol_key.as_idx()] {
        HierarchyItem::GenVar(_) => (VType::SignedNet(INTEGER_VSIZE), [].into()),
        HierarchyItem::Parameter(v) => {
            let HierarchyParameter { name: _, value } = &scope.hierarchy.parameters()[*v];
            (value.ty(), [].into())
        }
        HierarchyItem::Net(s) => {
            let HierarchyNet {
                name: _, ty, dims, ..
            } = &scope.hierarchy.net()[*s];
            (ty.clone(), dims.clone())
        }
        HierarchyItem::Module(_) => todo!(),
        HierarchyItem::NamedBlock(_) => todo!(),
        HierarchyItem::Task(_) => todo!(),
        HierarchyItem::Function(_) => todo!(),
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

    match &scope.hierarchy.items()[symbol_key.as_idx()] {
        HierarchyItem::Net(s) => {
            let s = &scope.hierarchy.net()[*s];
            let key = s.signal;
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
                            let (_, lsb, width) =
                                msb_lsb_to_width(arenas, scope.eval(), diagnostics, *msb, *lsb)?;
                            (
                                builder.constant(
                                    gl,
                                    Bits::from_i64_truncated(lsb as i64, INTEGER_VSIZE),
                                ),
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
                            let idx = builder.multiply_constant(gl, idx, size.get());
                            let offset = builder.plus(gl, offset, idx);
                            Some((offset, length))
                        }
                    }
                }
            };
            let size = partial.map_or(size, |(_, s)| s);
            let variable = expression::truncate_or_extend(gl, builder, variable, variable_ty, size);
            builder.regioned_drive_opt_partial(gl, key, variable, region as u8, partial);
        }
        HierarchyItem::Module(_) => todo!(),
        HierarchyItem::NamedBlock(_) => todo!(),
        HierarchyItem::Task(_) => todo!(),
        HierarchyItem::Function(_) => todo!(),
        HierarchyItem::Parameter(_) => todo!(),
        HierarchyItem::GenVar(_) => todo!(),
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
        let ty = net_lvalue_flat_ty(arenas, scope, diagnostics, lvf)?;
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
        let ty = net_lvalue_flat_ty(arenas, scope, diagnostics, lvf)?;
        let width = ty.force_net_width();
        let variable = builder.extract_constant(gl, variable, offset, width);
        assign_net_lvalue_flat(gl, arenas, scope, diagnostics, builder, lvf, variable, ty)?;
        offset += width.get();
    }
    Ok(())
}

pub fn net_lvalue_flat_ty<'a>(
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

    let lvalue_ident = arenas.get_ident(ident.item.0);
    let Some(symbol_key) = scope.get(&lvalue_ident) else {
        diagnostics.var_not_found(arenas, *ident);
        return Err(());
    };

    let exprs = *constant_exprs;
    let (mut ty, mut n_dims) = match &scope.hierarchy.items()[symbol_key.as_idx()] {
        HierarchyItem::GenVar(_) => todo!(),
        HierarchyItem::Parameter(v) => {
            let p = &scope.hierarchy.parameters()[*v];
            (p.value.ty(), 0)
        }
        HierarchyItem::Net(s) => {
            let n = &scope.hierarchy.net()[*s];
            (n.ty, n.dims.len())
        }
        HierarchyItem::Module(_)
        | HierarchyItem::NamedBlock(_)
        | HierarchyItem::Task(_)
        | HierarchyItem::Function(_) => todo!(),
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
                let (_, _, width) = msb_lsb_to_width(arenas, scope.eval(), diagnostics, *msb, *lsb)?;
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

    let Some(symbol_key) = scope.get(arenas.get_ident(ident.item.0)) else {
        diagnostics.var_not_found(arenas, *ident);
        return Err(());
    };

    let HierarchyItem::Net(s) = &scope.hierarchy.items()[symbol_key.as_idx()] else {
        panic!("not a signal");
    };
    let s = &scope.hierarchy.net()[*s];
    let key = s.signal;
    let mut dims = &s.dims[..];

    let mut exprs = *constant_exprs;
    let mut arr_idx = if !dims.is_empty()
        && let Some(fst) = exprs.pop_front()
    {
        dims = &dims[1..];
        let mut leaf_arr_items = dims.iter().product::<u32>();
        let fst = eval_constant_expr(arenas, scope.eval(), diagnostics, fst)?;
        let fst = fst.as_integer().unwrap();
        let mut offset = fst as u32 * leaf_arr_items;

        while let Some(dim) = dims.first()
            && let Some(expr) = exprs.pop_front()
        {
            leaf_arr_items /= *dim;
            let expr = eval_constant_expr(arenas, scope.eval(), diagnostics, expr)?;
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
        let fst = eval_constant_expr(arenas, scope.eval(), diagnostics, *expr)?;
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
                    eval_constant_expr(arenas, scope.eval(), diagnostics, *expr)?
                        .as_integer()
                        .unwrap(),
                    VectorSize::new(1).unwrap(),
                ),
                ConstantRangeExpression::MsbLsb { msb, lsb } => {
                    let (_, lsb, size) = msb_lsb_to_width(arenas, scope.eval(), diagnostics, *msb, *lsb)?;
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
    builder.drive_opt_partial(gl, key, variable, partial);
    Ok(())
}

pub fn net_lvalue_width<'a>(
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
    let mut size = net_lvalue_flat_ty(arenas, scope, diagnostics, lvalue_flat)?.force_net_width();

    for lvalue_flat in lvalue.0.iter().skip(1) {
        let lvalue_size =
            net_lvalue_flat_ty(arenas, scope, diagnostics, lvalue_flat)?.force_net_width();
        size = size.checked_add(lvalue_size.get()).ok_or_else(|| {
            diagnostics.net_width_overflow(arenas.get_span(output_terminal));
        })?;
    }
    Ok(size)
}
