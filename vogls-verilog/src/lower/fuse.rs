use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::{Driver, InputEdge};
use vogls_ir::{INTEGER_VSIZE, SCALAR_VSIZE, SignalSlice, VectorSize};

use crate::ast::constant_expr::ConstantRangeExpression;
use crate::ast::expr::{BitSlice, Expr};
use crate::ast::module::NetAssignment;
use crate::ast::{AstId, AstIdRange, HIdent};
use crate::elaborate::VSymbol;
use crate::lower::{hident_span, try_resolve_hident, try_resolve_net};

use super::{Diagnostics, LowerContext, MutLowerContext, VType, VValue, eval_constant_expr};

fn try_constant_expr_no_ctx<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
) -> Option<VValue> {
    eval_constant_expr(
        &mctx.gl,
        &ctx.arenas,
        &ctx.table,
        scope,
        &mut Diagnostics::default(),
        expr.into_constant(),
        None,
    )
    .ok()
}

fn try_constant_bitslice<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    bitslice: BitSlice<'a>,
) -> Result<Option<SignalSlice>, ()> {
    match bitslice {
        BitSlice::MsbLsb(msb, lsb) => {
            let msb = eval_constant_expr(
                &mctx.gl,
                &ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                msb,
                None,
            )?;
            let lsb = eval_constant_expr(
                &mctx.gl,
                &ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                lsb,
                None,
            )?;
            // @TODO: Validate input.
            let Some(msb) = msb
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32()
            else {
                return Ok(None);
            };
            let Some(lsb) = lsb
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32()
            else {
                return Ok(None);
            };
            // @TODO: validate
            let slice = SignalSlice::new(msb, lsb).unwrap();
            Ok(Some(slice))
        }
        BitSlice::PlusWidth(offset, ast_width) => {
            let Some(offset) = try_constant_expr_no_ctx(ctx, mctx, scope, offset) else {
                return Ok(None);
            };
            let width = eval_constant_expr(
                &mctx.gl,
                &ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                ast_width,
                None,
            )?;
            let Some(offset) = offset
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32()
            else {
                return Ok(None);
            };
            let Some(width) = width
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32()
            else {
                return Ok(None);
            };
            let Some(width) = VectorSize::new(width) else {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(ast_width), "zero-width net");
                return Err(());
            };
            // @TODO: validate
            let slice = SignalSlice::from_width(offset, width).unwrap();
            Ok(Some(slice))
        }
        BitSlice::MinusWidth(offset, ast_width) => {
            let Some(offset) = try_constant_expr_no_ctx(ctx, mctx, scope, offset) else {
                return Ok(None);
            };
            let width = eval_constant_expr(
                &mctx.gl,
                &ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                ast_width,
                None,
            )?;
            let Some(offset) = offset
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32()
            else {
                return Ok(None);
            };
            let Some(width) = width
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32()
            else {
                return Ok(None);
            };
            let Some(width) = VectorSize::new(width) else {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(ast_width), "zero-width net");
                return Err(());
            };
            // @TODO: validate
            let lsb = offset.strict_sub(width.get() - 1);
            let slice = SignalSlice::from_width(lsb, width).unwrap();
            Ok(Some(slice))
        }
    }
}

pub fn try_lower_fuse_driver_expr<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
) -> Result<bool, ()> {
    // @TODO: This could also allow for `Concat` and `Replication`.

    match &*expr {
        Expr::Ident(ident, exprs, range_expr) => {
            try_lower_fuse_driver_ident(ctx, mctx, scope, expr, *ident, *exprs, *range_expr)
        }
        Expr::Sized(sized) => {
            // @TODO: Signed.
            let sized = &ctx.arenas.sized_numbers[sized.item.at];
            mctx.fuse_scratch
                .push(Driver::Constant(sized.value.clone()));
            Ok(true)
        }
        _ => return Ok(false),
    }
}

pub fn try_lower_fuse_driver_ident<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    ident: HIdent<'_>,
    exprs: AstIdRange<'_, Expr>,
    range_expr: Option<BitSlice<'_>>,
) -> Result<bool, ()> {
    let symbol_id =
        try_resolve_hident(scope, &ctx.table, &ctx.arenas, ident, &mut mctx.diagnostics)?;
    let net = match &ctx.table[symbol_id].content {
        VSymbol::Parameter(_) => return Ok(false),
        VSymbol::Net(n) => n,

        _ => {
            mctx.diagnostics.not_yet_implemented(
                hident_span(&ctx.arenas, ident),
                "cannot assign net to this.",
            );
            return Err(());
        }
    };

    if exprs.len() < net.dims.len() {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_span(expr), "cannot assign array");
        return Err(());
    }

    let net_signal = net.net.probe_signal();

    // Fast path. No slicing at all.
    if exprs.is_empty() && range_expr.is_none() {
        mctx.fuse_scratch.push(Driver::Signal(net_signal, None));
        return Ok(true);
    }

    let ty_size = net.ty.force_net_width();

    // Handle array indexing.
    let mut offset = 0;
    let mut current_size = ty_size;
    for (expr, &dim) in exprs.iter().rev().zip(net.dims.iter()) {
        let Some(idx) = try_constant_expr_no_ctx(ctx, mctx, scope, expr) else {
            return Ok(false);
        };
        let idx = idx.truncate_or_extend(INTEGER_VSIZE);
        let Some(idx) = idx.into_bits().extract_exact_u32() else {
            return Ok(false);
        };

        if idx >= dim {
            mctx.diagnostics
                .warnings
                .push((ctx.arenas.get_span(expr), "index out of range".into()));
            // @TODO: Handle this better somehow...
            return Ok(false);
        }

        // @TODO: Checked arithmetic
        offset += current_size.get() * idx;
        current_size = VectorSize::new(current_size.get() * dim).unwrap();
    }

    // Handle bit indexing indexing.
    let mut output_width = ty_size;
    for expr in exprs.truncate(exprs.len() - net.dims.len()).iter().rev() {
        let Some(idx) = try_constant_expr_no_ctx(ctx, mctx, scope, expr) else {
            return Ok(false);
        };
        let idx = idx.truncate_or_extend(INTEGER_VSIZE);
        let Some(idx) = idx.into_bits().extract_exact_u32() else {
            return Ok(false);
        };

        if idx >= ty_size.get() {
            mctx.diagnostics
                .warnings
                .push((ctx.arenas.get_span(expr), "index out of range".into()));
            // @TODO: Handle this better somehow...
            return Ok(false);
        }

        offset += idx;
        output_width = SCALAR_VSIZE;
    }

    // Handle range slicing.
    let Some(mut output_slice) = SignalSlice::from_width(offset, output_width) else {
        return Ok(false);
    };
    if let Some(range_expr) = range_expr {
        let Some(slice) = try_constant_bitslice(ctx, mctx, scope, range_expr)? else {
            return Ok(false);
        };

        let Some(relative_slice) = output_slice.relative_slice(slice) else {
            return Ok(false);
        };
        output_slice = relative_slice;
    }

    mctx.fuse_scratch
        .push(Driver::Signal(net_signal, Some(output_slice)));
    Ok(true)
}

pub fn try_fuse_assign<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    net_assignment: AstId<'a, NetAssignment<'a>>,
) -> Result<bool, ()> {
    // @TODO: Support concatenation
    if net_assignment.net_lvalue.0.len() != 1 {
        return Ok(false);
    }

    mctx.fuse_scratch.clear();
    if !try_lower_fuse_driver_expr(ctx, mctx, scope, net_assignment.expression)? {
        return Ok(false);
    }
    let lvalue = net_assignment.net_lvalue.0.get(0);
    let to_net = try_resolve_net(
        scope,
        &ctx.table,
        &ctx.arenas,
        lvalue.ident,
        &mut mctx.diagnostics,
    )?;
    let drivee = to_net.net.blocking_drive_signal();

    let single_expression = lvalue.constant_range_expression.and_then(|r| match *r {
        ConstantRangeExpression::Single(expr) => Some(expr),
        ConstantRangeExpression::MsbLsb { .. }
        | ConstantRangeExpression::BasePlus { .. }
        | ConstantRangeExpression::BaseMinus { .. } => None,
    });

    if lvalue.constant_exprs.len() + usize::from(single_expression.is_some()) < to_net.dims.len() {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_span(lvalue), "cannot assign array");
        return Err(());
    }

    let drivee_ty_size = to_net.ty.force_net_width();

    // Handle array indexing.
    let mut offset = 0;
    let mut current_size = drivee_ty_size;
    for (expr, &dim) in (single_expression
        .iter()
        .copied()
        .chain(lvalue.constant_exprs.iter().rev()))
    .zip(to_net.dims.iter())
    {
        let idx = eval_constant_expr(
            &mctx.gl,
            &ctx.arenas,
            &ctx.table,
            scope,
            &mut mctx.diagnostics,
            expr,
            None,
        )?;
        let idx = idx.truncate_or_extend(INTEGER_VSIZE);
        let Some(idx) = idx.into_bits().extract_exact_u32() else {
            return Ok(false);
        };

        if idx >= dim {
            mctx.diagnostics
                .warnings
                .push((ctx.arenas.get_span(expr), "index out of range".into()));
            return Err(());
        }

        // @TODO: Checked arithmetic
        offset += current_size.get() * idx;
        current_size = VectorSize::new(current_size.get() * dim).unwrap();
    }

    // Handle bit indexing indexing.
    let mut drivee_output_width = drivee_ty_size;
    for expr in lvalue
        .constant_exprs
        .truncate(
            lvalue.constant_exprs.len() + usize::from(single_expression.is_some())
                - to_net.dims.len(),
        )
        .iter()
        .rev()
    {
        let idx = eval_constant_expr(
            &mctx.gl,
            &ctx.arenas,
            &ctx.table,
            scope,
            &mut mctx.diagnostics,
            expr,
            None,
        )?;
        let idx = idx.truncate_or_extend(INTEGER_VSIZE);
        let Some(idx) = idx.into_bits().extract_exact_u32() else {
            return Ok(false);
        };

        if idx >= drivee_ty_size.get() {
            mctx.diagnostics
                .warnings
                .push((ctx.arenas.get_span(expr), "index out of range".into()));
            // @TODO: Handle this better somehow...
            return Err(());
        }

        offset += idx;
        drivee_output_width = SCALAR_VSIZE;
    }

    // Handle range slicing.
    let Some(mut drivee_output_slice) = SignalSlice::from_width(offset, drivee_output_width) else {
        return Ok(false);
    };
    if let Some(range_expr) = &lvalue.constant_range_expression {
        let slice = match &**range_expr {
            ConstantRangeExpression::Single(expr) => {
                let idx = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *expr,
                    None,
                )?;
                let idx = idx.truncate_or_extend(INTEGER_VSIZE);
                let Some(idx) = idx.into_bits().extract_exact_u32() else {
                    return Ok(false);
                };

                // @TODO: Remove unwrap
                SignalSlice::from_width(idx, SCALAR_VSIZE).unwrap()
            }
            ConstantRangeExpression::MsbLsb { msb, lsb } => {
                let msb = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *msb,
                    None,
                )?;
                let lsb = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *lsb,
                    None,
                )?;

                let msb = msb.truncate_or_extend(INTEGER_VSIZE);
                let Some(msb) = msb.into_bits().extract_exact_u32() else {
                    return Ok(false);
                };
                let lsb = lsb.truncate_or_extend(INTEGER_VSIZE);
                let Some(lsb) = lsb.into_bits().extract_exact_u32() else {
                    return Ok(false);
                };
                SignalSlice::new(msb, lsb).unwrap()
            }
            ConstantRangeExpression::BasePlus { base, width } => {
                let base = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *base,
                    None,
                )?;
                let width = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *width,
                    None,
                )?;

                let base = base.truncate_or_extend(INTEGER_VSIZE);
                let Some(base) = base.into_bits().extract_exact_u32() else {
                    return Ok(false);
                };
                let width = width.truncate_or_extend(INTEGER_VSIZE);
                let Some(width) = width.into_bits().extract_exact_u32() else {
                    return Ok(false);
                };
                let Some(width) = VectorSize::new(width) else {
                    return Ok(false);
                };
                SignalSlice::from_width(base, width).unwrap()
            }
            ConstantRangeExpression::BaseMinus { base, width } => {
                let base = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *base,
                    None,
                )?;
                let width = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *width,
                    None,
                )?;

                let base = base.truncate_or_extend(INTEGER_VSIZE);
                let Some(base) = base.into_bits().extract_exact_u32() else {
                    return Ok(false);
                };
                let width = width.truncate_or_extend(INTEGER_VSIZE);
                let Some(width) = width.into_bits().extract_exact_u32() else {
                    return Ok(false);
                };
                let Some(width) = VectorSize::new(width) else {
                    return Ok(false);
                };
                let Some(base) = base.checked_sub(width.get()).and_then(|v| v.checked_add(1)) else {
                    return Ok(false);
                };
                SignalSlice::from_width(base - width.get() + 1, width).unwrap()
            }
        };

        let Some(relative_slice) = drivee_output_slice.relative_slice(slice) else {
            return Ok(false);
        };
        drivee_output_slice = relative_slice;
    }

    let mut offset = drivee_output_slice.lsb();
    for driver in &mctx.fuse_scratch {
        let width = driver.size(&mctx.gl.signals);
        let Some(width) = VectorSize::new((drivee_ty_size.get() - offset).min(width.get())) else {
            break;
        };
        mctx.connections.push(InputEdge {
            driver: driver.clone(),
            drivee,
            drivee_slice: Some(SignalSlice::from_width(offset, width).unwrap()),
        });
        offset += width.get();
    }
    Ok(true)
}
