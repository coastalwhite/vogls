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
        instances,
    } = &*id;

    let Some(udp) = ctx.udps.get(&identifier.item.0) else {
        mctx.diagnostics.udp_not_found(ctx.arenas, *identifier);
        return Err(());
    };

    for instance in instances.iter() {
        let UdpInstance {
            name: _,
            output_terminal,
            input_terminals,
        } = &*instance;

        crate::lower::udp::lower_udp(ctx, mctx, scope, *udp, *output_terminal, *input_terminals)?;
    }

    Ok(())
}
