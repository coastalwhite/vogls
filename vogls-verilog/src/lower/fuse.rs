use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{INTEGER_VSIZE, SCALAR_VSIZE, SignalSlice, VectorSize};

use crate::ast::AstId;
use crate::ast::expr::{BitSlice, Expr};
use crate::elaborate::VSymbol;
use crate::lower::{hident_span, try_resolve_symbol_id};

use super::{Diagnostics, LowerContext, MutLowerContext, VType, VValue, eval_constant_expr};

fn try_constant_expr<'a>(
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
            )?;
            let lsb = eval_constant_expr(
                &mctx.gl,
                &ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                lsb,
            )?;
            // @TODO: Validate input.
            let msb = msb
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32();
            let lsb = lsb
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32();
            // @TODO: validate
            let slice = SignalSlice::new(msb, lsb).unwrap();
            Ok(Some(slice))
        }
        BitSlice::PlusWidth(offset, ast_width) => {
            let Some(offset) = try_constant_expr(ctx, mctx, scope, offset) else {
                return Ok(None);
            };
            let width = eval_constant_expr(
                &mctx.gl,
                &ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                ast_width,
            )?;
            let offset = offset
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32();
            let width = width
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32();
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
            let Some(offset) = try_constant_expr(ctx, mctx, scope, offset) else {
                return Ok(None);
            };
            let width = eval_constant_expr(
                &mctx.gl,
                &ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                ast_width,
            )?;
            let offset = offset
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32();
            let width = width
                .coerce(&VType::UnsignedNet(INTEGER_VSIZE))
                .into_bits()
                .extract_exact_u32();
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

    let Expr::Ident(ident, exprs, range_expr) = &*expr else {
        return Ok(false);
    };

    let symbol_id = try_resolve_symbol_id(
        scope,
        &ctx.table,
        &ctx.arenas,
        *ident,
        &mut mctx.diagnostics,
    )?;
    let net = match &ctx.table[symbol_id].content {
        VSymbol::Parameter(_) => return Ok(false),
        VSymbol::Net(n) => n,

        _ => {
            mctx.diagnostics.not_yet_implemented(
                hident_span(&ctx.arenas, *ident),
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

    let (net_signal, net_slice) = net.net.probe_signal();
    assert!(net_slice.is_none(), "should not yet be set");

    // Fast path. No slicing at all.
    if exprs.is_empty() && range_expr.is_none() {
        mctx.fuse_scratch.push((net_signal, None));
        return Ok(true);
    }

    let ty_size = net.ty.force_net_width();

    // Handle array indexing.
    let mut offset = net_slice.map_or(0, |s| s.lsb());
    let mut current_size = ty_size;
    for (expr, &dim) in exprs.iter().rev().zip(net.dims.iter()) {
        let Some(idx) = try_constant_expr(ctx, mctx, scope, expr) else {
            return Ok(false);
        };
        let idx = idx.truncate_or_extend(INTEGER_VSIZE);
        let idx = idx.into_bits().extract_exact_u32();

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
        let Some(idx) = try_constant_expr(ctx, mctx, scope, expr) else {
            return Ok(false);
        };
        let idx = idx.truncate_or_extend(INTEGER_VSIZE);
        let idx = idx.into_bits().extract_exact_u32();

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
        let Some(slice) = try_constant_bitslice(ctx, mctx, scope, *range_expr)? else {
            return Ok(false);
        };

        let Some(relative_slice) = output_slice.relative_slice(slice) else {
            return Ok(false);
        };
        output_slice = relative_slice;
    }

    mctx.fuse_scratch.push((net_signal, Some(output_slice)));
    Ok(true)
}
