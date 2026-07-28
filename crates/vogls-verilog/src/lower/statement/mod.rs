use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::BasicBlockBuilder;

use crate::ast::AstIdRange;
use crate::ast::statement::{Statement, StatementContent};
use crate::lower::Region;
use crate::lower::expression::function_call::lower_task_enable;

use super::LowerContext;
use super::MutLowerContext;
pub use used_signals::{get_used_signals, get_used_signals_stmt_or_null};

mod blocking_assignment;
pub mod conditional;
pub mod loop_statement;
mod nonblocking_assignment;
mod par_block;
pub mod procedural_timing_control;
mod seq_block;
pub mod system_task_enable;
mod used_signals;
mod wait;

/// Lower several Verilog statements to Vogls IR.
pub fn lower_stmts<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    mut builder: BasicBlockBuilder,
    stmts: AstIdRange<'a, Statement<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    use StatementContent as S;
    for statement in stmts.iter() {
        match statement.content {
            S::BlockingAssignment(ba) => {
                builder = blocking_assignment::lower(ctx, mctx, scope, builder, ba)?
            }
            S::CaseStatement(case_statement) => {
                builder =
                    conditional::lower_case_statement(ctx, mctx, scope, builder, case_statement)?
            }
            S::ConditionalStatement(conditional) => {
                builder = conditional::lower(ctx, mctx, scope, builder, conditional)?
            }
            S::DisableStatement => todo!(),
            S::EventTrigger => todo!(),
            S::LoopStatement(ls) => {
                builder = loop_statement::lower_loop_statement(ctx, mctx, scope, builder, ls)?
            }
            S::NonBlockingAssignment(nba) => {
                builder = nonblocking_assignment::lower(ctx, mctx, scope, builder, nba)?
            }
            S::ProceduralContinuousAssignments => {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(statement),
                    "procedural continous assignments are not yet supported",
                );
            }
            S::ProceduralTimingControlStatement(ptc_stmt) => {
                builder = procedural_timing_control::lower(ctx, mctx, scope, builder, ptc_stmt)?
            }
            S::SeqBlock(seq_block) => {
                builder = seq_block::lower(ctx, mctx, scope, builder, seq_block)?
            }
            S::ParBlock(par_block) => {
                builder = par_block::lower(ctx, mctx, scope, builder, par_block)?
            }
            S::SystemTaskEnable(system_task_enable) => {
                builder = system_task_enable::lower_system_task_enable(
                    ctx,
                    mctx,
                    scope,
                    builder,
                    system_task_enable,
                )?;
            }
            S::TaskEnable(task_enable) => {
                builder = lower_task_enable(ctx, mctx, scope, builder, task_enable)?;
            }
            S::WaitStatement(wait_stmt) => {
                builder = wait::lower(ctx, mctx, scope, builder, wait_stmt)?
            }
        }
    }

    Ok(builder)
}
