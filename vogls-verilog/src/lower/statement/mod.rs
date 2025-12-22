use vogls_ir::{BasicBlockBuilder, GlobalContext};

use crate::ast::statement::StatementOrNull;
use crate::ast::AstId;
use crate::parser::AstArenas;

use super::scope::Scope;
use super::{statements_to_process, Diagnostics, VTypeTable};

pub mod loop_statement;
pub mod conditional;

pub fn lower_statement_or_null<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: BasicBlockBuilder,
    statement: AstId<StatementOrNull>,
) -> Result<BasicBlockBuilder, ()> {
    match arenas.get(statement) {
        StatementOrNull::Attribute(_) => Ok(builder),
        StatementOrNull::Statement(statement) => statements_to_process(
            gl,
            arenas,
            types,
            scope,
            diagnostics,
            builder,
            std::slice::from_ref(arenas.get(*statement)),
        ),
    }
}
