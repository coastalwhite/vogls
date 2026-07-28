use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::BasicBlockBuilder;
use vogls_utils::OrderedSet;

use crate::ast::AstId;
use crate::ast::statement::WaitStatement;
use crate::lower::{LowerContext, MutLowerContext, expression};

use super::lower_stmts;

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    mut builder: BasicBlockBuilder,
    wait_stmt: AstId<'a, WaitStatement<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let WaitStatement {
        expression,
        statement_or_null,
    } = *wait_stmt;

    builder = builder.jump(mctx.gl());
    let start_bb = builder.key();

    let (condition, _) = expression::lower_expr(ctx, mctx, scope, &mut builder, expression, None)?;
    let condition = builder.reduce_or(mctx.gl(), condition);
    let mut ins = OrderedSet::new();
    expression::get_used_signals(ctx, mctx, scope, &mut ins, expression)?;

    let (stmt_builder, watch_builder) = builder.double_branch(mctx.gl(), condition);
    watch_builder.watch_to(mctx.gl(), ins.items, start_bb);
    builder = lower_stmts(
        ctx,
        mctx,
        scope,
        stmt_builder,
        statement_or_null.as_id_range(),
    )?;
    Ok(builder)
}
