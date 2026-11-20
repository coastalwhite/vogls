//! Finds the signals that procedural code observes, before it is lowered.
//!
//! Procedural code, including the bodies of functions and tasks, is lowered after the drive graph
//! is resolved, so their watches and last update times must be known beforehand.

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::SignalKey;
use vogls_utils::OrderedSet;

use crate::ast::expr::{BitSlice, Expr};
use crate::ast::statement::{
    BlockingAssignment, CaseItemPattern, DelayControl, EventControl, EventExpressionPrimary,
    LoopStatementVariant, MinTypMaxExpression, NonBlockingAssignment, ProceduralTimingControl,
    Statement, StatementContent, StatementOrNull, VariableLValue,
};
use crate::ast::{AstId, HIdent};
use crate::elaborate::VSymbol;

use super::statement::get_used_signals_stmt_or_null;
use super::{LowerContext, MutLowerContext, expression, resolve_hident};

#[derive(Default)]
pub struct Scan {
    pub watched: OrderedSet<SignalKey>,
    pub lupdt: OrderedSet<SignalKey>,
    pub calls: OrderedSet<SymbolId>,
}

impl Scan {
    pub fn clear(&mut self) {
        self.watched.clear();
        self.lupdt.clear();
        self.calls.clear();
    }

    /// Scans the body of the function or task `sid`.
    pub fn subroutine<'a>(
        &mut self,
        ctx: &LowerContext<'a, '_>,
        mctx: &mut MutLowerContext<'a>,
        sid: SymbolId,
    ) {
        match &ctx.table[sid].content {
            VSymbol::Function(f) => {
                let statement = ctx.table_ast_refs.fns[f.ast_id].statement;
                self.stmt(ctx, mctx, sid, statement);
            }
            VSymbol::Task(t) => {
                let statement_or_null = ctx.table_ast_refs.tasks[t.ast_id].statement_or_null;
                self.stmt_or_null(ctx, mctx, sid, statement_or_null);
            }
            _ => unreachable!("not a function or task"),
        }
    }

    /// Records the watched signals and the signals whose last update time is taken in the driver
    /// resolution.
    pub fn observe(&self, mctx: &mut MutLowerContext<'_>) {
        for &signal in &self.watched.items {
            mctx.driver_resolution.observe_watch(signal);
        }
        for &signal in &self.lupdt.items {
            mctx.driver_resolution.observe_lupdt(signal);
        }
    }

    fn stmt_or_null<'a>(
        &mut self,
        ctx: &LowerContext<'a, '_>,
        mctx: &mut MutLowerContext<'a>,
        scope: SymbolId,
        stmt: AstId<'a, StatementOrNull<'a>>,
    ) {
        match &*stmt {
            StatementOrNull::Attribute(_) => {}
            StatementOrNull::Statement(stmt) => self.stmt(ctx, mctx, scope, *stmt),
        }
    }

    pub fn stmt<'a>(
        &mut self,
        ctx: &LowerContext<'a, '_>,
        mctx: &mut MutLowerContext<'a>,
        scope: SymbolId,
        stmt: AstId<'a, Statement<'a>>,
    ) {
        match stmt.content {
            StatementContent::CaseStatement(id) => {
                self.expr(ctx, scope, id.expr);
                for item in id.items.iter() {
                    if let CaseItemPattern::Expressions(exprs) = item.pattern.item {
                        exprs.iter().for_each(|e| self.expr(ctx, scope, e));
                    }
                    self.stmt_or_null(ctx, mctx, scope, item.statement_or_null);
                }
            }
            StatementContent::ConditionalStatement(id) => {
                for branch in std::iter::once(id.if_branch).chain(id.else_ifs.iter().map(|b| *b)) {
                    self.expr(ctx, scope, branch.condition);
                    self.stmt_or_null(ctx, mctx, scope, branch.statement);
                }
                if let Some(else_branch) = id.else_branch {
                    self.stmt_or_null(ctx, mctx, scope, else_branch);
                }
            }
            StatementContent::LoopStatement(id) => {
                match id.variant {
                    LoopStatementVariant::Forever => {}
                    LoopStatementVariant::Repeat(expr) | LoopStatementVariant::While(expr) => {
                        self.expr(ctx, scope, expr)
                    }
                    LoopStatementVariant::For(initialization, condition, step) => {
                        for assignment in [initialization, step] {
                            self.lvalue(ctx, scope, assignment.lvalue);
                            self.expr(ctx, scope, assignment.expr);
                        }
                        self.expr(ctx, scope, condition);
                    }
                }
                self.stmt(ctx, mctx, scope, id.statement);
            }
            StatementContent::BlockingAssignment(id) => {
                // Intra-assignment timing controls are not lowered.
                let BlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control: _,
                    expression,
                } = *id;
                self.lvalue(ctx, scope, variable_lvalue);
                self.expr(ctx, scope, expression);
            }
            StatementContent::NonBlockingAssignment(id) => {
                let NonBlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control: _,
                    expression,
                } = *id;
                self.lvalue(ctx, scope, variable_lvalue);
                self.expr(ctx, scope, expression);
            }
            StatementContent::ProceduralTimingControlStatement(id) => {
                match &*id.procedural_timing_control {
                    ProceduralTimingControl::DelayControl(delay) => match &**delay {
                        DelayControl::DelayValue(_) => {}
                        DelayControl::MinTypMax(id) => {
                            let MinTypMaxExpression { min_max, typical } = **id;
                            self.expr(ctx, scope, typical);
                            if let Some((min, max)) = min_max {
                                self.expr(ctx, scope, min);
                                self.expr(ctx, scope, max);
                            }
                        }
                    },
                    ProceduralTimingControl::EventControl(event_control) => {
                        match &**event_control {
                            EventControl::Star => {
                                _ = get_used_signals_stmt_or_null(
                                    ctx,
                                    mctx,
                                    scope,
                                    &mut self.watched,
                                    id.statement_or_null,
                                );
                            }
                            EventControl::EventExpression(event_expression) => {
                                for primary in event_expression.0.iter() {
                                    let (EventExpressionPrimary::Expression(expr)
                                    | EventExpressionPrimary::Posedge(expr)
                                    | EventExpressionPrimary::Negedge(expr)) = *primary;
                                    // Only plain identifiers are lowered.
                                    if let Expr::Ident(ident, exprs, None) = &*expr
                                        && exprs.is_empty()
                                        && let Some(signal) = resolve_signal(ctx, scope, *ident)
                                    {
                                        self.watched.insert(signal);
                                    }
                                }
                            }
                        }
                    }
                }
                self.stmt_or_null(ctx, mctx, scope, id.statement_or_null);
            }
            StatementContent::SeqBlock(id) => {
                let scope = match id.block {
                    None => Some(scope),
                    Some(block) => resolve_hident(scope, &ctx.table, block.block_identifier),
                };
                if let Some(scope) = scope {
                    for s in id.statements.iter() {
                        self.stmt(ctx, mctx, scope, s);
                    }
                }
            }
            StatementContent::ParBlock(id) => {
                let scope = match id.block {
                    None => Some(scope),
                    Some(block) => resolve_hident(scope, &ctx.table, block.block_identifier),
                };
                if let Some(scope) = scope {
                    for s in id.statements.iter() {
                        self.stmt(ctx, mctx, scope, s);
                    }
                }
            }
            StatementContent::SystemTaskEnable(id) => {
                let name = &ctx.arenas.ident_table[id.system_task_identifier.item.0];
                let is_monitor = matches!(name, "monitor" | "monitorb" | "monitoro" | "monitorh");
                for expr in id.expressions.iter() {
                    if let Some(expr) = AstId::transpose_option(expr) {
                        self.expr(ctx, scope, expr);
                        if is_monitor {
                            _ = expression::get_used_signals(
                                ctx,
                                mctx,
                                scope,
                                &mut self.watched,
                                expr,
                            );
                        }
                    }
                }
            }
            StatementContent::TaskEnable(id) => {
                if let Some(sid) = resolve_hident(scope, &ctx.table, id.ident)
                    && matches!(ctx.table[sid].content, VSymbol::Task(_))
                {
                    self.calls.insert(sid);
                }
                id.exprs.iter().for_each(|e| self.expr(ctx, scope, e));
            }
            StatementContent::WaitStatement(id) => {
                self.expr(ctx, scope, id.expression);
                _ = expression::get_used_signals(
                    ctx,
                    mctx,
                    scope,
                    &mut self.watched,
                    id.expression,
                );
                self.stmt_or_null(ctx, mctx, scope, id.statement_or_null);
            }
            // Not lowered.
            StatementContent::DisableStatement
            | StatementContent::EventTrigger
            | StatementContent::ProceduralContinuousAssignments => {}
        }
    }

    /// Scans the expressions that select the bits of an lvalue.
    fn lvalue<'a>(
        &mut self,
        ctx: &LowerContext<'a, '_>,
        scope: SymbolId,
        lvalue: AstId<'a, VariableLValue<'a>>,
    ) {
        for flat in lvalue.0.iter() {
            flat.exprs.iter().for_each(|e| self.expr(ctx, scope, e));
            if let Some(range_expression) = flat.range_expression {
                match &*range_expression {
                    BitSlice::PlusWidth(base, width) | BitSlice::MinusWidth(base, width) => {
                        self.expr(ctx, scope, *base);
                        self.expr(ctx, scope, width.into_expr());
                    }
                    BitSlice::MsbLsb(msb, lsb) => {
                        self.expr(ctx, scope, msb.into_expr());
                        self.expr(ctx, scope, lsb.into_expr());
                    }
                }
            }
        }
    }

    /// Finds the signals of every `$vogls_lupdt` and the functions that are called in `expr`.
    fn expr<'a>(&mut self, ctx: &LowerContext<'a, '_>, scope: SymbolId, expr: AstId<'a, Expr<'a>>) {
        let mut stack = vec![expr];
        while let Some(expr) = stack.pop() {
            match &*expr {
                Expr::Unary(_, c) => stack.push(*c),
                Expr::Binary(_, l, r) => stack.extend([*l, *r]),
                Expr::Ternary(c, t, f) => stack.extend([*c, *t, *f]),
                Expr::Concatenation(exprs) => stack.extend(exprs.iter()),
                Expr::FunctionCall(ident, exprs) => {
                    if let Some(sid) = resolve_hident(scope, &ctx.table, *ident)
                        && matches!(ctx.table[sid].content, VSymbol::Function(_))
                    {
                        self.calls.insert(sid);
                    }
                    stack.extend(exprs.iter())
                }
                Expr::Replication(replication) => {
                    stack.push(replication.constant_expr.into_expr());
                    stack.extend(replication.exprs.iter())
                }
                Expr::SystemFunctionCall(ident, arguments) => {
                    let Some(arguments) = arguments else {
                        continue;
                    };
                    if &ctx.arenas.ident_table[ident.item.0] == "vogls_lupdt"
                        && arguments.len() == 1
                        && let Expr::Ident(arg, exprs, None) = &*arguments.get(0)
                        && exprs.is_empty()
                        && let Some(signal) = resolve_signal(ctx, scope, *arg)
                    {
                        self.lupdt.insert(signal);
                    }
                    stack.extend(arguments.iter());
                }
                Expr::Ident(_, exprs, range_expression) => {
                    stack.extend(exprs.iter());
                    match range_expression {
                        Some(
                            BitSlice::PlusWidth(base, width) | BitSlice::MinusWidth(base, width),
                        ) => stack.extend([*base, width.into_expr()]),
                        Some(BitSlice::MsbLsb(msb, lsb)) => {
                            stack.extend([msb.into_expr(), lsb.into_expr()])
                        }
                        None => {}
                    }
                }
                Expr::Real(_) | Expr::Decimal(_) | Expr::Sized(_) | Expr::String(_) => {}
            }
        }
    }
}

/// The signal that probing the identifier `ident` in `scope` reads, if it names a signal.
fn resolve_signal<'a>(
    ctx: &LowerContext<'a, '_>,
    scope: SymbolId,
    ident: HIdent<'a>,
) -> Option<SignalKey> {
    let sid = resolve_hident(scope, &ctx.table, ident)?;
    match &ctx.table[sid].content {
        VSymbol::Signal(symbol) => Some(symbol.terminals.probe_signal()),
        _ => None,
    }
}
