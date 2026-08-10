use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::InputEdge;
use vogls_ir::{ProcessBuilder, ProcessKind, SignalSlice, VectorSize};
use vogls_utils::OrderedSet;

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::module::{ModuleOrGenerateItemDeclaration, NetDeclAssignment, NetDeclarationNets};
use crate::elaborate::VSymbol;
use crate::lower::expression::{self, get_used_signals, lower_expr};
use crate::lower::fuse::try_lower_fuse_driver_expr;
use crate::lower::resolve_hident;
use crate::lower::{LowerContext, MutLowerContext};

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
            NetDeclarationNets::Idents(_) => {}
            NetDeclarationNets::Assignments(assignments) => {
                let mut error = false;
                for assignment in assignments.iter() {
                    let NetDeclAssignment { ident, expr } = &*assignment;
                    let Some(net_sid) = resolve_hident(scope, &ctx.table, *ident) else {
                        unreachable!(
                            "The net for net assignment should always be defined in elaboration"
                        );
                    };
                    error |= assign_net(ctx, mctx, scope, net_sid, *expr).is_err();
                }
                if error {
                    return Err(());
                }
            }
        },
        ModuleOrGenerateItemDeclaration::Reg(_) => {}
        ModuleOrGenerateItemDeclaration::Integer(_) => {}
        ModuleOrGenerateItemDeclaration::Real(_) => {}
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
    // marshalling process.
    mctx.fuse_scratch.clear();
    if try_lower_fuse_driver_expr(ctx, mctx, scope, rvalue)? {
        let drivee = net_symbol.net.blocking_drive_signal();

        let mut offset = 0;
        let drivee_width = mctx.gl.signals[drivee].size;
        for driver in &mctx.fuse_scratch {
            let bit_length = driver.size(&mctx.gl.signals);
            let Some(bit_length) =
                VectorSize::new((drivee_width.get() - offset).min(bit_length.get()))
            else {
                break;
            };
            mctx.connections.push(InputEdge {
                driver: driver.clone(),
                drivee,
                drivee_slice: Some(SignalSlice::from_width(offset, bit_length).unwrap()),
            });
            offset += bit_length.get();
        }
        return Ok(());
    }

    let mut sensitivity_list = OrderedSet::new();
    get_used_signals(ctx, mctx, scope, &mut sensitivity_list, rvalue)?;
    let sensitivity_list = sensitivity_list.items;

    let net_bit_length = net_symbol.ty.bit_length();

    let (process, mut bb_builder) = ProcessBuilder::new(
        &mut mctx.gl,
        ProcessKind::Assign,
        ctx.arenas.get_span(rvalue),
    );
    let bb_key = bb_builder.key();

    let (rvalue, rvalue_ty) = lower_expr(
        ctx,
        mctx,
        scope,
        &mut bb_builder,
        rvalue,
        Some(net_bit_length),
    )?;

    let v = expression::coerce_to(mctx.gl(), &mut bb_builder, rvalue, rvalue_ty, net_symbol.ty);
    net_symbol
        .net
        .drive_blocking(mctx.gl(), &mut bb_builder, v, None);
    bb_builder.watch_to(mctx.gl(), sensitivity_list, bb_key);

    process.finalize(mctx.gl());

    Ok(())
}
