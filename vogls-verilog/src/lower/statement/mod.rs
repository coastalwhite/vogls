use vogls_ir::{BasicBlockBuilder, BasicBlockTerminator, GlobalContext};

use crate::ast::statement::{
    Block, BlockingAssignment, NonBlockingAssignment, ProceduralTimingControlStatement, SeqBlock,
    Statement, StatementContent, StatementOrNull, TaskEnable, WaitStatement,
};
use crate::ast::{AstId, AstIdRange};
use crate::lower::expression::function_call::lower_task_enable;
use crate::lower::expression::{self, lower_expr};
use crate::lower::{Region, assign, try_resolve_symbol_id};
use crate::parser::AstArenas;

use super::Diagnostics;
use super::Scope;

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
            AstIdRange::single(*statement),
        ),
    }
}

pub fn statements_to_process<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    stmts: AstIdRange<Statement>,
) -> Result<BasicBlockBuilder, ()> {
    use StatementContent as S;
    for statement in stmts.iter() {
        match arenas.get(statement).content {
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
                    false,
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
                    true,
                )?;
            }
            S::ParBlock => todo!(),
            S::ProceduralContinuousAssignments => {
                diagnostics.not_yet_implemented(
                    arenas.get_span(statement),
                    "procedural continous assignments are not yet supported",
                );
            }
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
                let SeqBlock { block, statements } = arenas.get(id);

                let scope_key = match block {
                    Some(blk) => try_resolve_symbol_id(
                        scope.key,
                        scope.table,
                        arenas,
                        arenas.get(*blk).block_identifier,
                        diagnostics,
                    )?,
                    None => scope.key,
                };

                let mut scope = Scope {
                    table: scope.table,
                    key: scope_key,
                    signal_map: scope.signal_map,
                };

                builder = statements_to_process(
                    gl,
                    arenas,
                    &mut scope,
                    diagnostics,
                    builder,
                    *statements,
                )?;
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
                let TaskEnable { ident, exprs } = arenas.get(id);
                builder =
                    lower_task_enable(gl, arenas, scope, diagnostics, builder, *ident, *exprs)?;
            }
            S::WaitStatement(id) => {
                let WaitStatement {
                    expression,
                    statement_or_null,
                } = arenas.get(id);

                builder = builder.jump(gl);
                let (condition, _) = expression::lower_expr(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    &mut builder,
                    *expression,
                )?;
                let condition = builder.reduce_or(gl, condition);
                let ins = expression::get_used_signals(arenas, scope, diagnostics, *expression)?;

                let start_bb = builder.key();
                builder = builder.next_terminate_later(gl);
                let ret_with_watch_bb = builder.key();
                builder = builder.next_terminate_later(gl);
                let statement_bb = builder.key();

                gl.bbs[start_bb].terminator =
                    BasicBlockTerminator::Branch(condition, statement_bb, ret_with_watch_bb);
                gl.bbs[ret_with_watch_bb].terminator = BasicBlockTerminator::Watch(start_bb, ins);

                builder = lower_statement_or_null(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    builder,
                    *statement_or_null,
                )?;
            }
        }
    }

    Ok(builder)
}
