use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, Bits, LogicMode, ProcessBuilder, SCALAR_VSIZE, Signal, SignalFlags,
    TIME_VSIZE, Time, VariableKey,
};
use vogls_utils::OrderedSet;

use crate::ast::expr::Expr;
use crate::ast::statement::{
    DelayControl, DelayValue, EventControl, EventExpressionPrimary, ProceduralTimingControl,
    ProceduralTimingControlStatement, StatementOrNull,
};
use crate::ast::{AstId, AstItem};
use crate::elaborate::NetSymbol;
use crate::lower::expression::{lower_expr, truncate_or_extend};
use crate::lower::{LowerContext, MutLowerContext, try_resolve_net};
use crate::lower::{WatchCondition, try_resolve_constant};

use super::Region;

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    proc_builder: &mut ProcessBuilder,
    mut builder: BasicBlockBuilder,
    ptc_stmt: AstId<'a, ProceduralTimingControlStatement<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let next_tr = proc_builder.next_temporal_region(mctx.gl());
    match *ptc_stmt.procedural_timing_control {
        ProceduralTimingControl::DelayControl(ast_delay_control) => {
            let delay_control = &*ast_delay_control;
            match delay_control {
                DelayControl::DelayValue(ast_delay_value) => {
                    if let DelayValue::UnsignedNumber(value) = &**ast_delay_value
                        && ctx.arenas.decimals[value.at].as_u64() == Some(0)
                    {
                        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 159
                        //
                        // """
                        // An explicit zero delay (#0) requires that the process be
                        // suspended and added as an inactive event for the current time so
                        // that the process is resumed in the next simulation cycle in the
                        // current time.
                        // """
                        builder.wait_region_to(mctx.gl(), Region::Inactive as u8, next_tr)
                    } else {
                        let delay = match &**ast_delay_value {
                            DelayValue::UnsignedNumber(value) => {
                                let value = &ctx.arenas.decimals[value.at];
                                let delay = value.as_u64().unwrap_or(0);
                                ctx.time_scale
                                    .unit
                                    .truncate_or_multiply_to(delay, ctx.time_resolution)
                            }
                            DelayValue::RealNumber(value) => ctx
                                .time_scale
                                .unit
                                .real_to_ticks(
                                    *value,
                                    ctx.time_scale.precision,
                                    ctx.time_resolution,
                                )
                                .unwrap_or(u64::MAX),
                            DelayValue::Identifier(ident) => {
                                let ident = AstItem {
                                    item: *ident,
                                    loc: ast_delay_value.loc,
                                };
                                let value = try_resolve_constant(
                                    scope,
                                    &ctx.table,
                                    ctx.arenas,
                                    ident,
                                    &mut mctx.diagnostics,
                                )?;
                                let delay = value.as_integer().unwrap() as u64;
                                ctx.time_scale
                                    .unit
                                    .truncate_or_multiply_to(delay, ctx.time_resolution)
                            }
                        };
                        builder.wait_to(mctx.gl(), Time(delay), next_tr);
                    }
                }
                DelayControl::MinTypMax(min_typ_max) => {
                    let (value, value_ty) = lower_expr(
                        ctx,
                        mctx,
                        scope,
                        &mut builder,
                        min_typ_max.typical,
                        Some(TIME_VSIZE),
                    )?;
                    let delay =
                        truncate_or_extend(mctx.gl(), &mut builder, value, value_ty, TIME_VSIZE);
                    let delay = builder.multiply_constant(
                        mctx.gl(),
                        delay,
                        Bits::from_u64(
                            TIME_VSIZE,
                            ctx.time_scale
                                .unit
                                .truncate_or_multiply_to(1, ctx.time_resolution),
                        ),
                    );
                    builder.variable_wait_to(mctx.gl(), delay, next_tr);
                }
            }
        }
        ProceduralTimingControl::EventControl(event_control) => match &*event_control {
            EventControl::Star => {
                let mut ins = OrderedSet::new();
                match *ptc_stmt.statement_or_null {
                    StatementOrNull::Attribute(_) => {}
                    StatementOrNull::Statement(stmt) => {
                        super::get_used_signals(ctx, mctx, scope, &mut ins, stmt)?
                    }
                }

                builder.watch_to(mctx.gl(), ins.items, next_tr);
            }
            EventControl::EventExpression(event_expression) => {
                let start_tr = proc_builder.next_temporal_region(mctx.gl());
                builder.temporal_jump_to(mctx.gl(), start_tr);
                builder.finished_switch_to(mctx.gl(), start_tr.entry());

                let mut contains_edge = false;
                let mut conditions: Vec<(WatchCondition, VariableKey, &NetSymbol, AstId<Expr>)> =
                    Vec::new();
                let mut signals = Vec::new();
                for event_expression in event_expression.0.iter() {
                    let (expr, condition) = match &*event_expression {
                        EventExpressionPrimary::Expression(expr) => (expr, WatchCondition::None),
                        EventExpressionPrimary::Posedge(expr) => (expr, WatchCondition::Posedge),
                        EventExpressionPrimary::Negedge(expr) => (expr, WatchCondition::Negedge),
                    };

                    contains_edge |=
                        matches!(condition, WatchCondition::Posedge | WatchCondition::Negedge);

                    let Expr::Ident(ast_ident, exprs, range_expression) = &**expr else {
                        panic!("not an ident");
                    };
                    if !exprs.is_empty() || range_expression.is_some() {
                        mctx.diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(*expr),
                            "event expression of this kind",
                        );
                        return Err(());
                    }

                    let net = try_resolve_net(
                        scope,
                        &ctx.table,
                        ctx.arenas,
                        *ast_ident,
                        &mut mctx.diagnostics,
                    )?;
                    let signal = net.net.probe_signal();
                    let dummy = mctx.gl().vars.insert(LogicMode::FourValue, SCALAR_VSIZE);
                    conditions.push((condition, dummy, net, *expr));
                    signals.push(signal);
                }
                let mut before_signals = Vec::new();
                if contains_edge {
                    for (_, v, net, _) in conditions.iter_mut() {
                        *v = net.net.probe(mctx.gl(), &mut builder);
                        let before_signal = mctx.gl.signals.insert(Signal {
                            name: format!("EVENT_BEFORE/{}", mctx.gl.signals.len()),
                            size: mctx.gl.vars.size(*v),
                            initialize: None,
                            flags: SignalFlags::EMPTY,
                            origin: TokenRange::default(),
                            mode: v.mode(),
                        });
                        builder.drive(mctx.gl(), before_signal, *v);
                        before_signals.push(before_signal);
                    }
                }

                let middle_tr = proc_builder.next_temporal_region(mctx.gl());
                builder.watch_to(mctx.gl(), signals, middle_tr);
                builder.finished_switch_to(mctx.gl(), middle_tr.entry());
                if contains_edge {
                    let mut acc = builder.constant(mctx.gl(), Bits::new_zeroed(SCALAR_VSIZE));
                    for ((condition, _, _, expr), before_signal) in
                        conditions.into_iter().zip(before_signals)
                    {
                        use WatchCondition as C;

                        let before = builder.probe(mctx.gl(), before_signal);
                        let (after, _) = lower_expr(ctx, mctx, scope, &mut builder, expr, None)?;
                        let cond = match condition {
                            C::Posedge => builder.posedge(mctx.gl(), before, after),
                            C::Negedge => builder.negedge(mctx.gl(), before, after),
                            C::None => builder.not_case_equals(mctx.gl(), before, after),
                        };
                        let cond = builder.reduce_or(mctx.gl(), cond);
                        acc = builder.or(mctx.gl(), acc, cond);
                    }

                    let mut false_builder;
                    (builder, false_builder) = builder.double_branch(mctx.gl(), acc);
                    false_builder.temporal_jump_to(mctx.gl(), start_tr);
                }

                builder.temporal_jump_to(mctx.gl(), next_tr);
            }
        },
    }

    builder.finished_switch_to(mctx.gl(), next_tr.entry());
    builder = super::lower_stmts(
        ctx,
        mctx,
        scope,
        proc_builder,
        builder,
        ptc_stmt.statement_or_null.as_id_range(),
    )?;

    Ok(builder)
}
