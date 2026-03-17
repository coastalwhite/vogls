use vogls_ir::{GlobalContext, new_anonymous_builder};

use crate::ast::module::{FunctionDeclaration, TaskDeclaration};
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::{LoweredFunction, LoweredTask};
use crate::lower::Scope;
use crate::lower::{Diagnostics, unwrap_get_fn_mut, unwrap_get_task_mut};
use crate::parser::AstArenas;

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    scope: &mut Scope<'a>,
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

    let builder = new_anonymous_builder(gl, "function".into(), arenas.get_span(id));

    let dummy_process_key = builder.process();
    let entry_key = builder.key();

    let builder = crate::lower::statement::statements_to_process(
        gl,
        arenas,
        scope,
        diagnostics,
        builder,
        AstIdRange::single(*statement),
    )?;

    let terminate_key = builder.key();
    builder.halt(gl);

    gl.processes.remove(dummy_process_key);

    unwrap_get_fn_mut(scope.table, scope.key).lowered = Some(LoweredFunction {
        entry: entry_key,
        terminate: terminate_key,
    });

    Ok(())
}

pub fn lower_task<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    scope: &mut Scope<'a>,
    id: AstId<'a, TaskDeclaration<'a>>,
) -> Result<(), ()> {
    let TaskDeclaration {
        automatic: _,
        ident: _,
        task_ports: _,
        block_item_decls: _,
        statement_or_null,
    } = &*id;

    let builder = new_anonymous_builder(gl, "task".into(), arenas.get_span(id));

    let dummy_process_key = builder.process();
    let entry_key = builder.key();

    let builder = crate::lower::statement::lower_statement_or_null(
        gl,
        arenas,
        scope,
        diagnostics,
        builder,
        *statement_or_null,
    )?;

    let terminate_key = builder.key();
    builder.halt(gl);

    gl.processes.remove(dummy_process_key);
    unwrap_get_task_mut(scope.table, scope.key).lowered = Some(LoweredTask {
        entry: entry_key,
        terminate: terminate_key,
    });

    Ok(())
}
