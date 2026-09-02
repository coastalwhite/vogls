use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::BasicBlockBuilder;

use crate::ast::AstId;
use crate::ast::statement::NonBlockingAssignment;
use crate::lower::{LowerContext, MutLowerContext, assign, expression};

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    mut builder: BasicBlockBuilder,
    nba: AstId<'a, NonBlockingAssignment<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let NonBlockingAssignment {
        variable_lvalue,
        delay_or_event_control,
        expression,
    } = &*nba;
    if let Some(delay_or_event_control) = delay_or_event_control {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_span(*delay_or_event_control),
            "delay or event control on non-blocking assignment",
        );
        return Err(());
    }

    let context_width = assign::variable_lvalue_size(ctx, mctx, scope, *variable_lvalue)?;
    let (value, value_ty) = expression::lower_expr(
        ctx,
        mctx,
        scope,
        &mut builder,
        *expression,
        Some(context_width),
    )?;
    assign::assign_variable_lvalue(
        ctx,
        mctx,
        scope,
        &mut builder,
        *variable_lvalue,
        value,
        value_ty,
        true,
    )?;
    Ok(builder)
}
