use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::SignalSlice;

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::module::{ModuleOrGenerateItemDeclaration, NetDeclAssignment, NetDeclarationNets};
use crate::ast::statement::Delay3;
use crate::elaborate::{NetResolution, SignalKind, VSymbol};
use crate::lower::fuse::{
    DestinationSite, DestinationSiteContent, RequiredSignalKind, SiteTerminal, SiteType,
    expr_to_src_sites,
};
use crate::lower::resolve_hident;
use crate::lower::{LowerContext, MutLowerContext};

/// Lower a Verilog net assignment construct to Vogls IR.
///
/// This only lowers the assignment part (which is equivalent to a continous assignment), as the nets/symbols are already defined during
/// elaboration.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
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
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    net: SymbolId,
    rvalue: AstId<'a, Expr<'a>>,
    delay3: Option<AstId<'a, Delay3<'a>>>,
) -> Result<(), ()> {
    let VSymbol::Signal(net_symbol) = &ctx.table[net].content else {
        unreachable!("A net assignment should always be directly on a net");
    };

    if !net_symbol.dims.is_empty() {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_span(rvalue), "assignment to an array");
        return Err(());
    }

    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 71
    //
    // """
    // When there is a continuous assignment in a declaration, the delay is part of the continuous
    // assignment and is not a net delay.
    // """
    mctx.fuse_dsts_scratch.clear();
    mctx.fuse_srcs_scratch.clear();

    let bit_length = net_symbol.ty.bit_length();

    // Destinations, from least to most significant.
    mctx.fuse_dsts_scratch.push(DestinationSite {
        content: DestinationSiteContent::Signal {
            signal: net_symbol.terminals.blocking_drive_signal(),
            slice: SignalSlice::with_end(bit_length),
            kind: net_symbol.kind,
        },
        ty: net_symbol.ty.into(),
    });

    // A delayed assignment assigns the input of a delay in front of the destinations.
    if let Some(delay3) = delay3 {
        let input = mctx.driver_resolution.push_delay(
            &mut mctx.gl,
            ctx.logic_mode,
            delay3,
            scope,
            &mctx.fuse_dsts_scratch,
            bit_length,
        );
        mctx.fuse_dsts_scratch.clear();
        mctx.fuse_dsts_scratch.push(DestinationSite {
            content: DestinationSiteContent::Signal {
                signal: input,
                slice: SignalSlice::with_end(bit_length),
                // The input of a delay is only driven by the assignment.
                kind: SignalKind::Net(NetResolution::Wire),
            },
            ty: SiteType::from(net_symbol.ty),
        });
    }

    // Sources, from least to most significant.
    expr_to_src_sites(
        ctx,
        &mctx.gl,
        &mut mctx.fuse_srcs_scratch,
        scope,
        &mut mctx.diagnostics,
        rvalue,
        SiteTerminal::Probe,
        RequiredSignalKind::None,
    )?;

    // The right-hand side is context-determined by the destinations, unless it is a concatenation
    // or only a part of it is assigned.
    let is_context_determined = !matches!(&*rvalue, Expr::Concatenation(_) | Expr::Replication(_));
    let context = is_context_determined.then_some((rvalue, scope, net_symbol.ty));
    if mctx
        .driver_resolution
        .push_sites(
            &mctx.gl,
            &mctx.fuse_srcs_scratch,
            &mctx.fuse_dsts_scratch,
            context,
        )
        .is_err()
    {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_span(rvalue),
            "assigning a real as part of a concatenation",
        );
        return Err(());
    }
    Ok(())
}
