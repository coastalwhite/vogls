use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, ProcessBuilder};

use crate::ast::AstId;
use crate::ast::statement::SeqBlock;
use crate::lower::statement::lower_stmts;
use crate::lower::{LowerContext, MutLowerContext, try_resolve_hident};

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    proc_builder: &mut ProcessBuilder,
    mut builder: BasicBlockBuilder,
    seq_block: AstId<'a, SeqBlock<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let SeqBlock { block, statements } = *seq_block;

    let scope_key = match block {
        Some(blk) => try_resolve_hident(
            scope,
            &ctx.table,
            ctx.arenas,
            blk.block_identifier,
            &mut mctx.diagnostics,
        )?,
        None => scope,
    };

    builder = lower_stmts(ctx, mctx, scope_key, proc_builder, builder, statements)?;
    Ok(builder)
}
