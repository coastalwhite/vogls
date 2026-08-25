use vogls_frontend::symbol_table::SymbolId;

use crate::ast::AstId;
use crate::ast::udp::{UdpInstance, UdpInstantiation};
use crate::lower::{LowerContext, MutLowerContext};

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, UdpInstantiation<'a>>,
) -> Result<(), ()> {
    let UdpInstantiation {
        identifier,
        drive_strength,
        delay,
        instances,
    } = &*id;

    if let Some(drive_strength) = drive_strength {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_item_span(*drive_strength),
            "drive strength on UDPs",
        );
        return Err(());
    }
    if let Some(delay) = delay {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_span(*delay), "delay on UDPs");
        return Err(());
    }

    let Some(udp) = ctx.udps.get(&identifier.item.0) else {
        mctx.diagnostics.udp_not_found(ctx.arenas, *identifier);
        return Err(());
    };

    for instance in instances.iter() {
        let UdpInstance {
            name,
            output_terminal,
            input_terminals,
        } = &*instance;

        if let Some((_, range)) = name
            && let Some(range) = range
        {
            mctx.diagnostics
                .not_yet_implemented(ctx.arenas.get_span(*range), "range on UDPs");
            return Err(());
        }

        crate::lower::udp::lower_udp(ctx, mctx, scope, *udp, *output_terminal, *input_terminals)?;
    }

    Ok(())
}
