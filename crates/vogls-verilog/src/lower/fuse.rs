use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::Driver;
use vogls_ir::SignalSlice;

use crate::ast::expr::{BitSlice, Expr};
use crate::ast::{AstId, AstIdRange, HIdent};
use crate::elaborate::VSymbol;
use crate::lower::addressing::lower_addressing;
use crate::lower::{hident_span, try_resolve_hident};

use super::addressing::ConstantAddressingContext;
use super::{Diagnostics, LowerContext, MutLowerContext};

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
        _ => Ok(false),
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
        try_resolve_hident(scope, &ctx.table, ctx.arenas, ident, &mut mctx.diagnostics)?;
    let net = match &ctx.table[symbol_id].content {
        VSymbol::Parameter(_) => return Ok(false),
        VSymbol::Net(n) => n,

        _ => {
            mctx.diagnostics
                .not_yet_implemented(hident_span(ctx.arenas, ident), "cannot assign net to this.");
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
