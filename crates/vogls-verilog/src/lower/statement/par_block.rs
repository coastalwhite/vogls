use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, LogicMode, ProcessBuilder, ProcessKind, SignalFlags, VectorSize, SCALAR_VSIZE};

use crate::ast::statement::ParBlock;
use crate::ast::{AstId, AstIdRange};
use crate::lower::statement::lower_stmts;
use crate::lower::{try_resolve_hident, LowerContext, MutLowerContext};

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    mut builder: BasicBlockBuilder,
    par_block: AstId<'a, ParBlock<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let ParBlock { block, statements } = &*par_block;

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

    let origin = ctx.arenas.get_span(par_block);
    let Some(num_processes) = statements.len().try_into().ok().and_then(VectorSize::new) else {
        mctx.diagnostics
            .not_yet_implemented(origin, "overflow num processes");
        return Err(());
    };

    let fork_trigger = mctx.gl.signals.insert(vogls_ir::Signal {
        name: "::fork_trigger".to_string(),
        size: num_processes,
        initialize: Some(vogls_ir::Bits::new_zeroed(num_processes)),
        flags: SignalFlags::EMPTY,
        origin,
        mode: LogicMode::TwoValue,
    });

    for (i, stmt) in statements.iter().enumerate() {
        let (process, mut fork_builder) = ProcessBuilder::new(mctx.gl(), ProcessKind::Fork, origin);
        let fork_entry_bb = fork_builder.key();
        let condition =
            fork_builder.probe_slice_constant(mctx.gl(), fork_trigger, i as u32, SCALAR_VSIZE);
        let watch_builder;
        (fork_builder, watch_builder) = fork_builder.double_branch(mctx.gl(), condition);
        watch_builder.watch_to(mctx.gl(), vec![fork_trigger], fork_entry_bb);

        fork_builder =
            lower_stmts(ctx, mctx, scope_key, fork_builder, AstIdRange::single(stmt))?;
        let l0 = fork_builder.constant(mctx.gl(), vogls_ir::Bits::from(false));
        fork_builder.drive_partial_constant(mctx.gl(), fork_trigger, l0, i as u32);
        fork_builder.jump_to(mctx.gl(), fork_entry_bb);
        process.finalize(mctx.gl());
    }

    let l1 = builder.constant(mctx.gl(), vogls_ir::Bits::new_ones(num_processes));
    builder.drive(mctx.gl(), fork_trigger, l1);

    builder = builder.jump(mctx.gl());

    let start_bb = builder.key();
    let condition = builder.probe(mctx.gl(), fork_trigger);
    let condition = builder.reduce_or(mctx.gl(), condition);
    let watch_builder;
    (watch_builder, builder) = builder.double_branch(mctx.gl(), condition);
    watch_builder.watch_to(mctx.gl(), vec![fork_trigger], start_bb);
    Ok(builder)
}
