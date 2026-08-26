use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, ProcessBuilder};
use vogls_utils::OrderedSet;

use crate::ast::AstId;
use crate::ast::statement::WaitStatement;
use crate::lower::{LowerContext, MutLowerContext, expression};

use super::lower_stmts;

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    proc_builder: &mut ProcessBuilder,
    mut builder: BasicBlockBuilder,
    wait_stmt: AstId<'a, WaitStatement<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let WaitStatement {
        expression,
        statement_or_null,
    } = *wait_stmt;

    let start_tr = proc_builder.next_temporal_region(mctx.gl());
    let stmt_tr = proc_builder.next_temporal_region(mctx.gl());

    builder.temporal_jump_to(mctx.gl(), start_tr);
    builder.finished_switch_to(mctx.gl(), start_tr.entry());

    let (condition, _) = expression::lower_expr(ctx, mctx, scope, &mut builder, expression, None)?;
    let condition = builder.reduce_or(mctx.gl(), condition);
    let mut ins = OrderedSet::new();
    expression::get_used_signals(ctx, mctx, scope, &mut ins, expression)?;

    let (mut stmt_builder, mut watch_builder) = builder.double_branch(mctx.gl(), condition);
    watch_builder.watch_to(mctx.gl(), ins.items, start_tr);

    stmt_builder.temporal_jump_to(mctx.gl(), stmt_tr);
    stmt_builder.finished_switch_to(mctx.gl(), stmt_tr.entry());
    builder = lower_stmts(
        ctx,
        mctx,
        scope,
        proc_builder,
        stmt_builder,
        statement_or_null.as_id_range(),
    )?;
    Ok(builder)
}
