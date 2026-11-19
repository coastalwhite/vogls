use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::SCALAR_VSIZE;
use vogls_utils::OrderedSet;

use super::gate_instantiation::net_lvalue_to_dsts;
use crate::ast::AstId;
use crate::ast::udp::{UdpInstance, UdpInstantiation};
use crate::lower::drivers::ExprSource;
use crate::lower::expression::get_used_signals;
use crate::lower::udp::sequential_udp_state;
use crate::lower::{LowerContext, MutLowerContext};

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
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

        let mut reads = OrderedSet::new();
        for input in input_terminals.iter() {
            get_used_signals(ctx, mctx, scope, &mut reads, input)?;
        }

        let state = sequential_udp_state(ctx, mctx, scope, *udp)?;
        net_lvalue_to_dsts(ctx, mctx, scope, *output_terminal)?;
        mctx.driver_resolution.push_primitive(
            &mut mctx.gl,
            ctx.logic_mode,
            ExprSource::Udp {
                udp: *udp,
                scope,
                instance,
                state,
            },
            &mctx.fuse_dsts_scratch,
            SCALAR_VSIZE,
            None,
            reads.items,
        );
    }

    Ok(())
}
