use std::collections::HashMap;

use vogls_ir::{BasicBlockBuilder, GlobalContext, PhiRef, Value};

use crate::ast::statement::{
    ConditionalStatement, LoopStatement, LoopStatementVariant, StatementOrNull,
};
use crate::ast::{AstId, AstIdRange};
use crate::lower::scope::{Scope, SymbolKey, SymbolVariant};
use crate::lower::{
    assign_variable_lvalue, get_intersect_symbols_generated, lower_expr, statements_to_process,
};
use crate::parser::AstArenas;

pub fn lower<'a>(
    mut builder: BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    conditional: AstId<ConditionalStatement>,
    arenas: &'a AstArenas,
) -> BasicBlockBuilder {
    let ConditionalStatement {
        if_branch,
        else_ifs,
        else_branch,
    } = arenas.get(conditional);

    if !else_ifs.is_empty() {
        todo!()
    }

    let start_bb = builder.key();

    let mut end_builder = builder.next_builder(gl);

    let condition = lower_expr(
        &mut builder,
        gl,
        scope,
        arenas.get(if_branch.condition),
        arenas,
    );
    match else_branch {
        None => {
            let mut if_true_builder = builder.branch_false_to(gl, condition, end_builder.key());
            scope.push_scope();
            if_true_builder =
                lower_statement_or_null(if_true_builder, gl, scope, if_branch.statement, arenas);
            let assigned = scope.scope_assigned_symbols().collect::<Vec<_>>();
            scope.pop_scope();
            let origin = if_true_builder.key();

            if_true_builder.jump_to(gl, end_builder.key());

            for (s, v) in assigned {
                let before = scope.scope_variables[s].last().unwrap().1;
                let v = end_builder.phi_full(gl, origin, v, start_bb, before);
                scope.assign(s, v);
            }
        }
        Some(statement) => todo!(),
    }

    end_builder

}

pub fn lower_statement_or_null<'a>(
    mut builder: BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    statement: AstId<StatementOrNull>,
    arenas: &'a AstArenas,
) -> BasicBlockBuilder {
    match arenas.get(statement) {
        StatementOrNull::Attribute(_) => builder,
        StatementOrNull::Statement(statement) => statements_to_process(
            builder,
            gl,
            scope,
            std::slice::from_ref(arenas.get(*statement)),
            arenas,
        ),
    }
}
