use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, ProcessBuilder, ProcessKind, VariableKey};
use vogls_utils::OrderedSet;

use crate::ast::expr::Expr;
use crate::ast::module::{ModuleOrGenerateItemDeclaration, NetDeclAssignment, NetDeclarationNets};
use crate::ast::statement::Delay3;
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::VSymbol;
use crate::lower::expression::{self, get_used_signals, lower_expr};
use crate::lower::resolve_hident;
use crate::lower::{LowerContext, MutLowerContext};

use super::continuous_assign::{lower_assign_delay3, try_fuse_flat_assign};
use super::gate_instantiation::{AssignOutputFn, GateDelayFn, lower_delayed_drive};

/// Lower a Verilog net assignment construct to Vogls IR.
///
/// This only lowers the assignment part (which is equivalent to a continous assignment), as the nets/symbols are already defined during
/// elaboration.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    item_decl: AstId<'a, ModuleOrGenerateItemDeclaration<'a>>,
) -> Result<(), ()> {
    match &*item_decl {
        ModuleOrGenerateItemDeclaration::Net(net_decl) => match net_decl.nets {
            NetDeclarationNets::Idents(_) => {
                // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 71
                //
                // """
                // This syntax, called a net delay, means that any value change that is to be
                // applied to wireA by some other statement shall be delayed for ten time units
                // before it takes effect.
                // """
                if let Some(delay3) = net_decl.delay3 {
                    mctx.diagnostics
                        .not_yet_implemented(ctx.arenas.get_span(delay3), "net delays");
                    return Err(());
                }
            }
            NetDeclarationNets::Assignments(assignments) => {
                if let Some(drive_strength) = net_decl.drive_strength {
                    mctx.diagnostics.warn_not_yet_implemented(
                        ctx.arenas.get_item_span(drive_strength),
                        "Drive strength is ignored.",
                    );
                }

                let mut error = false;
                for assignment in assignments.iter() {
                    let NetDeclAssignment { ident, expr } = &*assignment;
                    let Some(net_sid) = resolve_hident(scope, &ctx.table, *ident) else {
                        unreachable!(
                            "The net for net assignment should always be defined in elaboration"
                        );
                    };
                    error |= assign_net(ctx, mctx, scope, net_sid, *expr, net_decl.delay3).is_err();
                }
                if error {
                    return Err(());
                }
            }
        },
        ModuleOrGenerateItemDeclaration::Reg(_) => {}
        ModuleOrGenerateItemDeclaration::Integer(_) => {}
        ModuleOrGenerateItemDeclaration::Time(_) => {}
        ModuleOrGenerateItemDeclaration::Real(_) => {}
        ModuleOrGenerateItemDeclaration::Realtime(_) => {}
        ModuleOrGenerateItemDeclaration::Genvar(_) => {}
        ModuleOrGenerateItemDeclaration::Task(_) => {}
        ModuleOrGenerateItemDeclaration::Function(_) => {}
    }

    Ok(())
}

fn assign_net<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    net: SymbolId,
    rvalue: AstId<'a, Expr<'a>>,
    delay3: Option<AstId<'a, Delay3<'a>>>,
) -> Result<(), ()> {
    let VSymbol::Net(net_symbol) = &ctx.table[net].content else {
        unreachable!("A net assignment should always be directly on a net");
    };

    if !net_symbol.dims.is_empty() {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_span(rvalue), "assignment to an array");
        return Err(());
    }

    // Optimization: Try to alias LValue and RValue. This allows a future pass to eliminate the
    // marshalling process. An alias cannot be delayed.
    mctx.fuse_scratch.clear();
    if delay3.is_none()
        && try_fuse_flat_assign(
            ctx,
            mctx,
            scope,
            net,
            rvalue,
            rvalue.loc,
            AstIdRange::empty(),
            None,
        )?
    {
        return Ok(());
    }

    let mut sensitivity_list = OrderedSet::new();
    get_used_signals(ctx, mctx, scope, &mut sensitivity_list, rvalue)?;
    let sensitivity_list = sensitivity_list.items;

    let net_bit_length = net_symbol.ty.bit_length();

    let span = ctx.arenas.get_span(rvalue);
    let (proc_builder, mut bb_builder) =
        ProcessBuilder::new(&mut mctx.gl, ProcessKind::Assign, span);
    let entry_tr = proc_builder.entry();

    let (rvalue, rvalue_ty) = lower_expr(
        ctx,
        mctx,
        scope,
        &mut bb_builder,
        rvalue,
        Some(net_bit_length),
    )?;

    let v = expression::coerce_to(mctx.gl(), &mut bb_builder, rvalue, rvalue_ty, net_symbol.ty);

    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 71
    //
    // """
    // When there is a continuous assignment in a declaration, the delay is part of the continuous
    // assignment and is not a net delay.
    // """
    let delay = delay3.map(|delay3| {
        move |mctx: &mut MutLowerContext,
              builder: &mut BasicBlockBuilder,
              new_value: VariableKey,
              driven_value: VariableKey| {
            lower_assign_delay3(
                ctx,
                mctx,
                scope,
                builder,
                delay3,
                new_value,
                driven_value,
                net_bit_length,
            )
        }
    });
    let assign = |mctx: &mut MutLowerContext, builder: &mut BasicBlockBuilder, value| {
        net_symbol
            .net
            .drive_blocking(mctx.gl(), builder, value, None);
        Ok(())
    };
    lower_delayed_drive(
        mctx,
        &mut bb_builder,
        span,
        delay.as_ref().map(|delay| delay as &GateDelayFn),
        &assign as &AssignOutputFn,
        v,
        net_bit_length,
    )?;
    bb_builder.watch_to(mctx.gl(), sensitivity_list, entry_tr);

    proc_builder.finalize(mctx.gl());

    Ok(())
}
