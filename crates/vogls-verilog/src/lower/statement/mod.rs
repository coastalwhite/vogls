use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, LogicMode, ProcessBuilder, ProcessKind, SCALAR_VSIZE, SignalFlags,
    SignalKey, VectorSize,
};
use vogls_utils::OrderedSet;

use crate::ast::expr::BitSlice;
use crate::ast::statement::{
    BlockingAssignment, CaseItem, CaseItemPattern, CaseStatement, ConditionalStatement,
    DelayControl, DelayOrEventControl, DelayValue, EventControl, EventExpressionPrimary,
    LoopStatement, LoopStatementVariant, MinTypMaxExpression, NonBlockingAssignment, ParBlock,
    ProceduralTimingControl, ProceduralTimingControlStatement, SeqBlock, Statement,
    StatementContent, StatementOrNull, SystemTaskEnable, TaskEnable, VariableAssignment,
    VariableLValue, VariableLValueFlat, WaitStatement,
};
use crate::ast::{AstId, AstIdRange, AstItem};
use crate::lower::expression::function_call::lower_task_enable;
use crate::lower::expression::{self, lower_expr};
use crate::lower::{Region, assign, try_resolve_hident};

use super::LowerContext;
use super::MutLowerContext;

pub mod conditional;
pub mod loop_statement;
pub mod procedural_timing_control;
pub mod system_task_enable;

pub fn lower_statement_or_null<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: BasicBlockBuilder,
    statement: AstId<'a, StatementOrNull<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    match &*statement {
        StatementOrNull::Attribute(_) => Ok(builder),
        StatementOrNull::Statement(statement) => {
            statements_to_process(ctx, mctx, scope, builder, AstIdRange::single(*statement))
        }
    }
}

pub fn statements_to_process<'a>(
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
                let BlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control,
                    expression,
                } = &*ba;
                assert!(delay_or_event_control.is_none());

                let context_width =
                    assign::variable_lvalue_size(ctx, mctx, scope, *variable_lvalue)?;
                let (value, value_ty) = lower_expr(
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
                    false,
                )?;
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
                let NonBlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control,
                    expression,
                } = &*nba;
                assert!(delay_or_event_control.is_none());

                let context_width =
                    assign::variable_lvalue_size(ctx, mctx, scope, *variable_lvalue)?;
                let (value, value_ty) = lower_expr(
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
            }
            S::ProceduralContinuousAssignments => {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(statement),
                    "procedural continous assignments are not yet supported",
                );
            }
            S::ProceduralTimingControlStatement(id) => {
                let ProceduralTimingControlStatement {
                    procedural_timing_control,
                    statement_or_null,
                } = &*id;
                builder = procedural_timing_control::lower(
                    ctx,
                    mctx,
                    scope,
                    builder,
                    *procedural_timing_control,
                    *statement_or_null,
                )?
            }
            S::SeqBlock(id) => {
                let SeqBlock { block, statements } = &*id;

                let scope_key = match block {
                    Some(blk) => try_resolve_hident(
                        scope,
                        &ctx.table,
                        ctx.arenas,
                        blk.block_identifier,
                        &mut mctx.diagnostics,
                    )?,
                    None => scope,
                };

                builder = statements_to_process(ctx, mctx, scope_key, builder, *statements)?;
            }
            S::ParBlock(id) => {
                let ParBlock { block, statements } = &*id;

                let scope_key = match block {
                    Some(blk) => try_resolve_hident(
                        scope,
                        &ctx.table,
                        ctx.arenas,
                        blk.block_identifier,
                        &mut mctx.diagnostics,
                    )?,
                    None => scope,
                };

                let origin = ctx.arenas.get_span(id);
                let Some(num_processes) =
                    statements.len().try_into().ok().and_then(VectorSize::new)
                else {
                    mctx.diagnostics
                        .not_yet_implemented(origin, "overflow num processes");
                    return Err(());
                };

                let fork_trigger = mctx.gl.signals.insert(vogls_ir::Signal {
                    name: "::fork_trigger".to_string(),
                    size: num_processes,
                    initialize: Some(vogls_ir::Bits::new_zeroed(num_processes)),
                    flags: SignalFlags::EMPTY,
                    origin,
                    mode: LogicMode::TwoValue,
                });

                for (i, stmt) in statements.iter().enumerate() {
                    let (process, mut fork_builder) =
                        ProcessBuilder::new(mctx.gl(), ProcessKind::Fork, origin);
                    let fork_entry_bb = fork_builder.key();
                    let condition = fork_builder.probe_slice_constant(
                        mctx.gl(),
                        fork_trigger,
                        i as u32,
                        SCALAR_VSIZE,
                    );
                    let watch_builder;
                    (fork_builder, watch_builder) =
                        fork_builder.double_branch(mctx.gl(), condition);
                    watch_builder.watch_to(mctx.gl(), vec![fork_trigger], fork_entry_bb);

                    fork_builder = statements_to_process(
                        ctx,
                        mctx,
                        scope_key,
                        fork_builder,
                        AstIdRange::single(stmt),
                    )?;
                    let l0 = fork_builder.constant(mctx.gl(), vogls_ir::Bits::from(false));
                    fork_builder.drive_partial_constant(mctx.gl(), fork_trigger, l0, i as u32);
                    fork_builder.jump_to(mctx.gl(), fork_entry_bb);
                    process.finalize(mctx.gl());
                }

                let l1 = builder.constant(mctx.gl(), vogls_ir::Bits::new_ones(num_processes));
                builder.drive(mctx.gl(), fork_trigger, l1);

                builder = builder.jump(mctx.gl());

                let start_bb = builder.key();
                let condition = builder.probe(mctx.gl(), fork_trigger);
                let condition = builder.reduce_or(mctx.gl(), condition);
                let watch_builder;
                (watch_builder, builder) = builder.double_branch(mctx.gl(), condition);
                watch_builder.watch_to(mctx.gl(), vec![fork_trigger], start_bb);
            }
            S::SystemTaskEnable(id) => {
                builder =
                    system_task_enable::lower_system_task_enable(ctx, mctx, scope, builder, id)?;
            }
            S::TaskEnable(id) => {
                let TaskEnable { ident, exprs } = &*id;
                builder = lower_task_enable(ctx, mctx, scope, builder, *ident, *exprs)?;
            }
            S::WaitStatement(id) => {
                let WaitStatement {
                    expression,
                    statement_or_null,
                } = &*id;

                builder = builder.jump(mctx.gl());
                let start_bb = builder.key();

                let (condition, _) =
                    expression::lower_expr(ctx, mctx, scope, &mut builder, *expression, None)?;
                let condition = builder.reduce_or(mctx.gl(), condition);
                let mut ins = OrderedSet::new();
                expression::get_used_signals(ctx, mctx, scope, &mut ins, *expression)?;

                let (stmt_builder, watch_builder) = builder.double_branch(mctx.gl(), condition);
                watch_builder.watch_to(mctx.gl(), ins.items, start_bb);
                builder =
                    lower_statement_or_null(ctx, mctx, scope, stmt_builder, *statement_or_null)?;
            }
        }
    }

    Ok(builder)
}

pub fn get_used_signals<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    signals: &mut OrderedSet<SignalKey>,
    stmt: AstId<'a, Statement<'a>>,
) -> Result<(), ()> {
    let mut error = false;
    match stmt.content {
        StatementContent::CaseStatement(id) => {
            let CaseStatement {
                variant: _,
                expr,
                items,
            } = &*id;
            error |= expression::get_used_signals(ctx, mctx, scope, signals, *expr).is_err();
            for item in items.iter() {
                let CaseItem {
                    pattern,
                    statement_or_null,
                } = &*item;
                match pattern.item {
                    CaseItemPattern::Default => {}
                    CaseItemPattern::Expressions(exprs) => {
                        for expr in exprs.iter() {
                            error |= expression::get_used_signals(ctx, mctx, scope, signals, expr)
                                .is_err();
                        }
                    }
                }

                error |=
                    get_used_signals_stmt_or_null(ctx, mctx, scope, signals, *statement_or_null)
                        .is_err();
            }
        }
        StatementContent::ConditionalStatement(id) => {
            let ConditionalStatement {
                if_branch,
                else_ifs,
                else_branch,
            } = &*id;
            error |= expression::get_used_signals(ctx, mctx, scope, signals, if_branch.condition)
                .is_err();
            error |= get_used_signals_stmt_or_null(ctx, mctx, scope, signals, if_branch.statement)
                .is_err();
            for else_if in else_ifs.iter() {
                error |= expression::get_used_signals(ctx, mctx, scope, signals, else_if.condition)
                    .is_err();
                error |=
                    get_used_signals_stmt_or_null(ctx, mctx, scope, signals, else_if.statement)
                        .is_err();
            }
            if let Some(else_branch) = else_branch {
                error |=
                    get_used_signals_stmt_or_null(ctx, mctx, scope, signals, *else_branch).is_err();
            }
        }
        StatementContent::DisableStatement => todo!(),
        StatementContent::EventTrigger => todo!(),
        StatementContent::LoopStatement(id) => {
            let LoopStatement { variant, statement } = &*id;
            match variant {
                LoopStatementVariant::Forever => {}
                LoopStatementVariant::Repeat(expr) | LoopStatementVariant::While(expr) => {
                    error |= expression::get_used_signals(ctx, mctx, scope, signals, *expr).is_err()
                }
                LoopStatementVariant::For(initialization, condition, step) => {
                    let VariableAssignment {
                        lvalue: initialization_lvalue,
                        expr: initialization,
                    } = &**initialization;
                    let VariableAssignment {
                        lvalue: step_lvalue,
                        expr: step,
                    } = &**step;

                    for lvalue in [*initialization_lvalue, *step_lvalue] {
                        error |=
                            get_variable_lvalue_used_signals(ctx, mctx, scope, signals, lvalue)
                                .is_err();
                    }

                    for expr in [*initialization, *condition, *step] {
                        error |=
                            expression::get_used_signals(ctx, mctx, scope, signals, expr).is_err()
                    }
                }
            }
            error |= get_used_signals(ctx, mctx, scope, signals, *statement).is_err();
        }

        StatementContent::BlockingAssignment(id) => {
            let BlockingAssignment {
                variable_lvalue,
                delay_or_event_control,
                expression,
            } = &*id;

            error |= get_variable_lvalue_used_signals(ctx, mctx, scope, signals, *variable_lvalue)
                .is_err();
            if let Some(delay_or_event_control) = delay_or_event_control {
                error |= get_delay_or_event_control_used_signals(
                    ctx,
                    mctx,
                    scope,
                    signals,
                    *delay_or_event_control,
                )
                .is_err();
            }
            error |= expression::get_used_signals(ctx, mctx, scope, signals, *expression).is_err();
        }
        StatementContent::NonBlockingAssignment(id) => {
            let NonBlockingAssignment {
                variable_lvalue,
                delay_or_event_control,
                expression,
            } = &*id;

            error |= get_variable_lvalue_used_signals(ctx, mctx, scope, signals, *variable_lvalue)
                .is_err();
            if let Some(delay_or_event_control) = delay_or_event_control {
                error |= get_delay_or_event_control_used_signals(
                    ctx,
                    mctx,
                    scope,
                    signals,
                    *delay_or_event_control,
                )
                .is_err();
            }
            error |= expression::get_used_signals(ctx, mctx, scope, signals, *expression).is_err();
        }

        StatementContent::ProceduralContinuousAssignments => todo!(),
        StatementContent::ProceduralTimingControlStatement(id) => {
            let ProceduralTimingControlStatement {
                procedural_timing_control,
                statement_or_null,
            } = &*id;

            match &**procedural_timing_control {
                ProceduralTimingControl::DelayControl(id) => {
                    error |= get_delay_control_used_signals(ctx, mctx, scope, signals, *id).is_err()
                }
                ProceduralTimingControl::EventControl(id) => {
                    error |= get_event_control_used_signals(ctx, mctx, scope, signals, *id).is_err()
                }
            }
            get_used_signals_stmt_or_null(ctx, mctx, scope, signals, *statement_or_null)?;
        }
        StatementContent::SeqBlock(id) => {
            let SeqBlock { block, statements } = &*id;
            let scope_key = match block {
                Some(blk) => try_resolve_hident(
                    scope,
                    &ctx.table,
                    ctx.arenas,
                    blk.block_identifier,
                    &mut mctx.diagnostics,
                )?,
                None => scope,
            };
            for s in statements.iter() {
                get_used_signals(ctx, mctx, scope_key, signals, s)?;
            }
        }
        StatementContent::ParBlock(id) => {
            let ParBlock { block, statements } = &*id;
            let scope_key = match block {
                Some(blk) => try_resolve_hident(
                    scope,
                    &ctx.table,
                    ctx.arenas,
                    blk.block_identifier,
                    &mut mctx.diagnostics,
                )?,
                None => scope,
            };
            for s in statements.iter() {
                get_used_signals(ctx, mctx, scope_key, signals, s)?;
            }
        }

        StatementContent::SystemTaskEnable(id) => {
            let SystemTaskEnable {
                system_task_identifier: _,
                expressions,
            } = &*id;
            for expr in expressions.iter() {
                expression::get_used_signals(ctx, mctx, scope, signals, expr)?;
            }
        }
        StatementContent::TaskEnable(id) => {
            let TaskEnable { ident: _, exprs } = &*id;
            for expr in exprs.iter() {
                expression::get_used_signals(ctx, mctx, scope, signals, expr)?;
            }
        }
        StatementContent::WaitStatement(id) => {
            let WaitStatement {
                expression,
                statement_or_null,
            } = &*id;
            expression::get_used_signals(ctx, mctx, scope, signals, *expression)?;
            get_used_signals_stmt_or_null(ctx, mctx, scope, signals, *statement_or_null)?;
        }
    }
    if error { Err(()) } else { Ok(()) }
}

pub fn get_used_signals_stmt_or_null<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    signals: &mut OrderedSet<SignalKey>,
    stmt: AstId<'a, StatementOrNull<'a>>,
) -> Result<(), ()> {
    match &*stmt {
        StatementOrNull::Attribute(_) => Ok(()),
        StatementOrNull::Statement(id) => get_used_signals(ctx, mctx, scope, signals, *id),
    }
}

pub fn get_variable_lvalue_used_signals<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    signals: &mut OrderedSet<SignalKey>,
    lvalue: AstId<'a, VariableLValue<'a>>,
) -> Result<(), ()> {
    let mut error = false;
    for flat_lvalue in lvalue.0.iter() {
        let VariableLValueFlat {
            ident: _,
            exprs,
            range_expression,
        } = &*flat_lvalue;

        for expr in exprs.iter() {
            error |= expression::get_used_signals(ctx, mctx, scope, signals, expr).is_err();
        }
        if let Some(range_expression) = range_expression {
            match &**range_expression {
                BitSlice::PlusWidth(expr, _) | BitSlice::MinusWidth(expr, _) => {
                    error |= expression::get_used_signals(ctx, mctx, scope, signals, *expr).is_err()
                }
                BitSlice::MsbLsb(_, _) => {}
            }
        }
    }
    if error { Err(()) } else { Ok(()) }
}

pub fn get_delay_or_event_control_used_signals<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    signals: &mut OrderedSet<SignalKey>,
    delay_or_event_control: AstId<'a, DelayOrEventControl<'a>>,
) -> Result<(), ()> {
    match &*delay_or_event_control {
        DelayOrEventControl::DelayControl(id) => {
            get_delay_control_used_signals(ctx, mctx, scope, signals, *id)
        }
        DelayOrEventControl::EventControl(id) => {
            get_event_control_used_signals(ctx, mctx, scope, signals, *id)
        }
    }
}

pub fn get_delay_control_used_signals<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    signals: &mut OrderedSet<SignalKey>,
    delay_control: AstId<'a, DelayControl<'a>>,
) -> Result<(), ()> {
    let mut error = false;
    match &*delay_control {
        DelayControl::DelayValue(id) => match &**id {
            DelayValue::UnsignedNumber(_) => {}
            DelayValue::Identifier(ident) => {
                error |= expression::get_used_ident_signals(
                    ctx,
                    mctx,
                    scope,
                    signals,
                    AstItem {
                        item: *ident,
                        loc: id.loc,
                    },
                )
                .is_err();
            }
        },
        DelayControl::MinTypMax(id) => {
            let MinTypMaxExpression { min_max, typical } = &**id;
            error |= expression::get_used_signals(ctx, mctx, scope, signals, *typical).is_err();
            if let Some((min, max)) = min_max {
                error |= expression::get_used_signals(ctx, mctx, scope, signals, *min).is_err();
                error |= expression::get_used_signals(ctx, mctx, scope, signals, *max).is_err();
            }
        }
    }
    if error { Err(()) } else { Ok(()) }
}

pub fn get_event_control_used_signals<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    signals: &mut OrderedSet<SignalKey>,
    event_control: AstId<'a, EventControl<'a>>,
) -> Result<(), ()> {
    let mut error = false;
    match &*event_control {
        EventControl::Star => {}
        EventControl::EventExpression(event_expr) => {
            for expr in event_expr.0 {
                let expr = match &*expr {
                    EventExpressionPrimary::Expression(expr) => *expr,
                    EventExpressionPrimary::Posedge(expr) => *expr,
                    EventExpressionPrimary::Negedge(expr) => *expr,
                };
                error |= expression::get_used_signals(ctx, mctx, scope, signals, expr).is_err();
            }
        }
    }
    if error { Err(()) } else { Ok(()) }
}
