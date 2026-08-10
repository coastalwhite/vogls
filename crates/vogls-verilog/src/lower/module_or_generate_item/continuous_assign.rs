use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::InputEdge;
use vogls_ir::{ProcessBuilder, ProcessKind, SignalSlice, VectorSize};
use vogls_utils::OrderedSet;

use crate::ast::constant_expr::{ConstantBitSlice, ConstantExpr};
use crate::ast::expr::Expr;
use crate::ast::module::{ContinousAssign, NetAssignment};
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::VSymbol;
use crate::lower::addressing::{ConstantAddressingContext, lower_addressing};
use crate::lower::assign::{assign_net_lvalue, net_lvalue_size};
use crate::lower::expression::{get_expr_type, get_used_signals, lower_expr};
use crate::lower::fuse::try_lower_fuse_driver_expr;
use crate::lower::{Diagnostics, LowerContext, MutLowerContext, try_resolve_hident};

/// Lower a Verilog `assign` construct to Vogls IR.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    assign: AstId<'a, ContinousAssign<'a>>,
) -> Result<(), ()> {
    for ast_net_assignment in assign.list_of_net_assignments {
        let net_assignment = &*ast_net_assignment;

        // Optimization: Try to alias the expression into the assignee. If this is successful,
        // future analysis may be able to completely remove this process.
        if try_fuse_assign(ctx, mctx, scope, ast_net_assignment)? {
            continue;
        }

        // See which signals are used in the expression.
        let mut watch_list = OrderedSet::new();
        get_used_signals(ctx, mctx, scope, &mut watch_list, net_assignment.expression)?;
        let watch_list = watch_list.items;

        // Create a process that
        // 1. Computes the expression output                       (RValue)
        // 2. Drives the net(s)                                    (LValue)
        // 3. Watches for updates to any used signal in the RValue
        let (process, mut bb_builder) =
            ProcessBuilder::new(mctx.gl(), ProcessKind::Assign, ctx.arenas.get_span(assign));
        let bb_key = bb_builder.key();

        let context_width = net_lvalue_size(ctx, mctx, scope, net_assignment.net_lvalue)?;
        let (rvalue, rvalue_ty) = lower_expr(
            ctx,
            mctx,
            scope,
            &mut bb_builder,
            net_assignment.expression,
            Some(context_width),
        )?;

        assign_net_lvalue(
            ctx,
            mctx,
            scope,
            &mut bb_builder,
            net_assignment.net_lvalue,
            rvalue,
            rvalue_ty,
        )?;

        bb_builder.watch_to(mctx.gl(), watch_list, bb_key);
        process.finalize(mctx.gl());
    }

    Ok(())
}

fn try_fuse_assign<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    net_assignment: AstId<'a, NetAssignment<'a>>,
) -> Result<bool, ()> {
    // @TODO: Support concatenation
    if net_assignment.net_lvalue.0.len() != 1 {
        return Ok(false);
    }

    let lvalue = net_assignment.net_lvalue.0.get(0);
    let to_net = try_resolve_hident(
        scope,
        &ctx.table,
        ctx.arenas,
        lvalue.ident,
        &mut mctx.diagnostics,
    )?;

    try_fuse_flat_assign(
        ctx,
        mctx,
        scope,
        to_net,
        net_assignment.expression,
        net_assignment.loc,
        lvalue.constant_exprs,
        lvalue.constant_range_expression,
    )
}

pub fn try_fuse_flat_assign<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    net: SymbolId,
    rvalue: AstId<'a, Expr<'a>>,
    loc: usize,
    constant_exprs: AstIdRange<'a, ConstantExpr<'a>>,
    constant_range_expression: Option<AstId<'a, ConstantBitSlice<'a>>>,
) -> Result<bool, ()> {
    mctx.fuse_scratch.clear();
    if !try_lower_fuse_driver_expr(ctx, mctx, scope, rvalue)? {
        return Ok(false);
    }

    let VSymbol::Net(n) = &ctx.table[net].content else {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.spans[loc], "cannot be used as net");
        return Err(());
    };

    // Fusing does not deal with sign-extension or conversion to/from reals. Therefore, we stop
    // fusing them here.
    if get_expr_type(
        &mut mctx.gl,
        ctx.arenas,
        &ctx.table,
        scope,
        &mut Diagnostics::default(),
        rvalue,
    )
    .map_or(true, |v| {
        v.bit_length() != n.ty.bit_length() && (!v.is_unsigned_net() || !n.ty.is_unsigned_net())
    }) {
        return Ok(false);
    }

    let mut actx = ConstantAddressingContext {
        gl: &mctx.gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics: &mut Diagnostics::default(),
        loc,
        _pd: std::marker::PhantomData,
    };

    let Ok(address) = lower_addressing(
        &mut actx,
        n.ty.bit_length(),
        &n.dims,
        n.transform,
        constant_exprs.iter(),
        constant_range_expression.map(|r| (*r).into()),
    ) else {
        return Ok(false);
    };

    let Some(offset) = address.signal_offset_as_u32() else {
        return Ok(false);
    };
    let drivee = n.net.blocking_drive_signal();

    // @TODO: sum(driver.size()) > output_width
    let drivee_signal_width = mctx.gl.signals[n.net.blocking_drive_signal()].size;
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
