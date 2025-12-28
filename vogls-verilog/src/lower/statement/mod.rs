use vogls_ir::{BasicBlockBuilder, GlobalContext};

use crate::ast::AstId;
use crate::ast::statement::{
    BlockingAssignment, NonBlockingAssignment, ProceduralTimingControlStatement, Statement,
    StatementContent, StatementOrNull, TaskEnable,
};
use crate::lower::expression::lower_expr;
use crate::lower::scope::SymbolVariant;
use crate::lower::{Region, assign};
use crate::parser::AstArenas;

use super::Diagnostics;
use super::scope::Scope;

pub mod conditional;
pub mod loop_statement;
pub mod procedural_timing_control;
pub mod system_task_enable;

pub fn lower_statement_or_null<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
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
            scope,
            diagnostics,
            builder,
            std::slice::from_ref(arenas.get(*statement)),
        ),
    }
}

pub fn statements_to_process<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    stmts: &[Statement],
) -> Result<BasicBlockBuilder, ()> {
    use StatementContent as S;
    for statement in stmts.iter() {
        match statement.content {
            S::BlockingAssignment(ba) => {
                let ba = arenas.get(ba);
                let BlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control,
                    expression,
                } = ba;
                assert!(delay_or_event_control.is_none());

                let (value, value_ty) =
                    lower_expr(gl, arenas, scope, diagnostics, &mut builder, *expression)?;
                assign::assign_variable_lvalue(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    &mut builder,
                    *variable_lvalue,
                    value,
                    value_ty,
                    Region::Active,
                )?;
            }
            S::CaseStatement(case_statement) => {
                builder = conditional::lower_case_statement(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    builder,
                    case_statement,
                )?
            }
            S::ConditionalStatement(conditional) => {
                builder = conditional::lower(gl, arenas, scope, diagnostics, builder, conditional)?
            }
            S::DisableStatement => todo!(),
            S::EventTrigger => todo!(),
            S::LoopStatement(ls) => {
                builder = loop_statement::lower_loop_statement(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    builder,
                    ls,
                )?
            }
            S::NonBlockingAssignment(nba) => {
                let NonBlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control,
                    expression,
                } = arenas.get(nba);
                assert!(delay_or_event_control.is_none());

                let (value, value_ty) =
                    lower_expr(gl, arenas, scope, diagnostics, &mut builder, *expression)?;
                assign::assign_variable_lvalue(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    &mut builder,
                    *variable_lvalue,
                    value,
                    value_ty,
                    Region::NonBlocking,
                )?;
            }
            S::ParBlock => todo!(),
            S::ProceduralContinuousAssignments => todo!(),
            S::ProceduralTimingControlStatement(id) => {
                let ProceduralTimingControlStatement {
                    procedural_timing_control,
                    statement_or_null,
                } = arenas.get(id);
                builder = procedural_timing_control::lower(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    builder,
                    *procedural_timing_control,
                    *statement_or_null,
                )?
            }
            S::SeqBlock(id) => {
                let seq_block = arenas.get(id);
                let statements = seq_block
                    .statements
                    .iter()
                    .map(|v| arenas.get(v).clone())
                    .collect::<Vec<_>>();
                builder =
                    statements_to_process(gl, arenas, scope, diagnostics, builder, &statements)?;
            }
            S::SystemTaskEnable(id) => {
                builder = system_task_enable::lower_system_task_enable(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    builder,
                    id,
                )?;
            }
            S::TaskEnable(id) => {
                let TaskEnable { ident } = arenas.get(id);
                let name = arenas.get_ident(ident.item.0);
                let Some(symbol_key) = scope.get(name) else {
                    diagnostics.var_not_found(arenas, *ident);
                    return Err(());
                };
                let SymbolVariant::Task(statement_or_null) = &scope.symbols[symbol_key].variant
                else {
                    diagnostics
                        .not_yet_implemented(arenas.get_item_span(*ident), "non-task enabled");
                    return Err(());
                };

                builder = lower_statement_or_null(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    builder,
                    *statement_or_null,
                )?;
            }
            S::WaitStatement => todo!(),
        }
    }

    Ok(builder)
}
