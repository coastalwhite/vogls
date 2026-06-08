use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::{Driver, InputEdge};
use vogls_ir::{INTEGER_VSIZE, SCALAR_VSIZE, SignalSlice, VectorSize};

use crate::ast::expr::{BitSlice, Expr};
use crate::ast::module::NetAssignment;
use crate::ast::{AstId, AstIdRange, HIdent};
use crate::elaborate::VSymbol;
use crate::lower::addressing::lower_addressing;
use crate::lower::{hident_span, try_resolve_hident, try_resolve_net};

use super::addressing::{Address, ConstantAddressingContext};
use super::{Diagnostics, LowerContext, MutLowerContext, VType, VValue, eval_constant_expr};

fn try_constant_expr_no_ctx<'a>(
    ctx: &LowerContext<'a, '_>,
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
    ctx: &LowerContext<'a, '_>,
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
    ctx: &LowerContext<'a, '_>,
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
    ctx: &LowerContext<'a, '_>,
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

    let mut actx = ConstantAddressingContext {
        gl: &mctx.gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics: &mut Diagnostics::default(),
        loc: expr.loc,
        _pd: std::marker::PhantomData,
    };

    let Ok(address) = lower_addressing(
        &mut actx,
        net.ty.force_net_width(),
        &net.dims,
        net.transform,
        exprs.iter().map(|e| e.into_constant()),
        range_expr.map(|r| r.into()),
    ) else {
        return Ok(false);
    };

    let output_width = address.output_width;
    let Some(offset) = address.signal_offset_as_u32() else {
        return Ok(false);
    };

    // @TODO: Handle array overflow.
    let net_signal = net.net.probe_signal();
    let Some(output_slice) = SignalSlice::from_width(offset, output_width) else {
        return Ok(false);
    };

    mctx.fuse_scratch
        .push(Driver::Signal(net_signal, Some(output_slice)));
    Ok(true)
}

pub fn try_fuse_assign<'a>(
    ctx: &LowerContext<'a, '_>,
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

    let mut actx = ConstantAddressingContext {
        gl: &mctx.gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics: &mut Diagnostics::default(),
        loc: lvalue.loc,
        _pd: std::marker::PhantomData,
    };

    let Ok(address) = lower_addressing(
        &mut actx,
        to_net.ty.force_net_width(),
        &to_net.dims,
        to_net.transform,
        lvalue.constant_exprs.iter(),
        lvalue.constant_range_expression.map(|r| (*r).into()),
    ) else {
        return Ok(false);
    };

    let Some(offset) = address.signal_offset_as_u32() else {
        return Ok(false);
    };
    let drivee = to_net.net.blocking_drive_signal();

    // @TODO: sum(driver.size()) > output_width
    let drivee_signal_width = mctx.gl.signals[to_net.net.blocking_drive_signal()].size;
    let mut offset = offset;
    for driver in &mctx.fuse_scratch {
        let width = driver.size(&mctx.gl.signals);
        let Some(width) = VectorSize::new((drivee_signal_width.get() - offset).min(width.get()))
        else {
            break;
        };

        // If we are fusing things that don't exist. Just cancel the fuse.
        if drivee_signal_width.get() < offset + width.get() {
            return Ok(false);
        }

        mctx.connections.push(InputEdge {
            driver: driver.clone(),
            drivee,
            drivee_slice: Some(SignalSlice::from_width(offset, width).unwrap()),
        });
        offset += width.get();
    }
    Ok(true)
}
