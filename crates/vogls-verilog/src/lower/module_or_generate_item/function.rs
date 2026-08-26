use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::ProcessBuilder;

use crate::ast::module::{FunctionDeclaration, TaskDeclaration};
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::{LoweredFunction, LoweredTask};
use crate::lower::{LowerContext, MutLowerContext};
use crate::lower::{unwrap_get_fn_mut, unwrap_get_task_mut};

pub fn lower<'a>(
    ctx: &mut LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, FunctionDeclaration<'a>>,
) -> Result<(), ()> {
    let FunctionDeclaration {
        automatic: _,
        range_or_type: _,
        ident: _,
        tf_input_decls: _,
        block_item_decls: _,
        statement,
    } = &*id;

    let (mut proc_builder, builder) = ProcessBuilder::new_anonymous(mctx.gl());
    let entry_key = builder.key();

    let builder = crate::lower::statement::lower_stmts(
        ctx,
        mctx,
        scope,
        &mut proc_builder,
        builder,
        AstIdRange::single(*statement),
    )?;

    let terminate_key = builder.key();
    builder.halt(mctx.gl());
    proc_builder.finalize(mctx.gl());

    unwrap_get_fn_mut(&mut ctx.table, scope).lowered = Some(LoweredFunction {
        entry: entry_key,
        terminate: terminate_key,
    });

    Ok(())
}

pub fn lower_task<'a>(
    ctx: &mut LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, TaskDeclaration<'a>>,
) -> Result<(), ()> {
    let TaskDeclaration {
        automatic: _,
        ident: _,
        task_ports: _,
        block_item_decls: _,
        statement_or_null,
    } = &*id;

    let (mut proc_builder, builder) = ProcessBuilder::new_anonymous(mctx.gl());

    let entry_key = builder.key();

    let builder = crate::lower::statement::lower_stmts(
        ctx,
        mctx,
        scope,
        &mut proc_builder,
        builder,
        statement_or_null.as_id_range(),
    )?;

    let terminate_key = builder.key();
    builder.halt(mctx.gl());
    proc_builder.finalize(mctx.gl());

    unwrap_get_task_mut(&mut ctx.table, scope).lowered = Some(LoweredTask {
        entry: entry_key,
        terminate: terminate_key,
    });

    Ok(())
}
