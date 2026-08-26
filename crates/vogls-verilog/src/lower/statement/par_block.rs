use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, LogicMode, ProcessBuilder, ProcessKind, SCALAR_VSIZE, SignalFlags,
    VectorSize,
};

use crate::ast::statement::ParBlock;
use crate::ast::{AstId, AstIdRange};
use crate::lower::statement::lower_stmts;
use crate::lower::{LowerContext, MutLowerContext, try_resolve_hident};

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    proc_builder: &mut ProcessBuilder,
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
        let (mut proc_builder, mut fork_builder) =
            ProcessBuilder::new(mctx.gl(), ProcessKind::Fork, origin);
        let stmt_tr = proc_builder.next_temporal_region(mctx.gl());
        let fork_entry_tr = proc_builder.entry();
        let condition =
            fork_builder.probe_slice_constant(mctx.gl(), fork_trigger, i as u32, SCALAR_VSIZE);
        let mut watch_builder;
        (fork_builder, watch_builder) = fork_builder.double_branch(mctx.gl(), condition);
        watch_builder.watch_to(mctx.gl(), vec![fork_trigger], fork_entry_tr);

        fork_builder.temporal_jump_to(mctx.gl(), stmt_tr);
        fork_builder.finished_switch_to(mctx.gl(), stmt_tr.entry());
        fork_builder = lower_stmts(
            ctx,
            mctx,
            scope_key,
            &mut proc_builder,
            fork_builder,
            AstIdRange::single(stmt),
        )?;
        let l0 = fork_builder.constant(mctx.gl(), vogls_ir::Bits::from(false));
        fork_builder.drive_partial_constant(mctx.gl(), fork_trigger, l0, i as u32);
        fork_builder.temporal_jump_to(mctx.gl(), fork_entry_tr);
        proc_builder.finalize(mctx.gl());
    }

    let l1 = builder.constant(mctx.gl(), vogls_ir::Bits::new_ones(num_processes));
    builder.drive(mctx.gl(), fork_trigger, l1);

    let start_tr = proc_builder.next_temporal_region(mctx.gl());
    let after_tr = proc_builder.next_temporal_region(mctx.gl());

    builder.temporal_jump_to(mctx.gl(), start_tr);
    builder.finished_switch_to(mctx.gl(), start_tr.entry());

    let condition = builder.probe(mctx.gl(), fork_trigger);
    let condition = builder.reduce_or(mctx.gl(), condition);
    let mut watch_builder;
    (watch_builder, builder) = builder.double_branch(mctx.gl(), condition);
    watch_builder.watch_to(mctx.gl(), vec![fork_trigger], start_tr);

    builder.temporal_jump_to(mctx.gl(), after_tr);
    builder.finished_switch_to(mctx.gl(), after_tr.entry());
    Ok(builder)
}
