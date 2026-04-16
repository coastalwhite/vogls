use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, Bits, SCALAR_VSIZE, TIME_VSIZE, Time, VariableKey};
use vogls_utils::OrderedSet;

use crate::ast::expr::Expr;
use crate::ast::statement::{
    DelayControl, DelayValue, EventControl, EventExpressionPrimary, ProceduralTimingControl,
    StatementOrNull,
};
use crate::ast::{AstId, AstItem};
use crate::elaborate::NetSymbol;
use crate::lower::expression::{lower_expr, truncate_or_extend};
use crate::lower::{LowerContext, MutLowerContext, try_resolve_net};
use crate::lower::{WatchCondition, try_resolve_constant};

use super::Region;

pub fn lower<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    mut builder: BasicBlockBuilder,
    ptc: AstId<'a, ProceduralTimingControl<'a>>,
    statement: AstId<'a, StatementOrNull<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    match &*ptc {
        ProceduralTimingControl::DelayControl(ast_delay_control) => {
            let delay_control = &**ast_delay_control;
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
                        builder = builder.wait_region(mctx.gl(), Region::Inactive as u8)
                    } else {
                        let delay = match &**ast_delay_value {
                            DelayValue::UnsignedNumber(value) => {
                                let value = &ctx.arenas.decimals[value.at];
                                value.as_u64().unwrap()
                            }
                            DelayValue::Identifier(ident) => {
                                let ident = AstItem {
                                    item: *ident,
                                    loc: ast_delay_value.loc,
                                };
                                let value = try_resolve_constant(
                                    scope,
                                    &ctx.table,
                                    &ctx.arenas,
                                    ident,
                                    &mut mctx.diagnostics,
                                )?;
                                value.as_integer().unwrap() as u64
                            }
                        };
                        let delay = delay * ctx.time_scale.time_unit;
                        builder = builder.wait(mctx.gl(), Time(delay));
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
                        Bits::from_u64(TIME_VSIZE, ctx.time_scale.time_unit),
                    );
                    builder = builder.variable_wait(mctx.gl(), delay);
                }
            }
            builder = super::lower_statement_or_null(ctx, mctx, scope, builder, statement)?;
        }
        ProceduralTimingControl::EventControl(event_control) => match &**event_control {
            EventControl::Star => {
                let mut ins = OrderedSet::new();
                match &*statement {
                    StatementOrNull::Attribute(_) => {}
                    StatementOrNull::Statement(stmt) => {
                        super::get_used_signals(ctx, mctx, scope, &mut ins, *stmt)?
                    }
                }

                builder = builder.watch(mctx.gl(), ins.items);
                builder = super::lower_statement_or_null(ctx, mctx, scope, builder, statement)?;
            }
            EventControl::EventExpression(event_expression) => {
                builder = builder.jump(mctx.gl());
                let start_key = builder.key();

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
                        &ctx.arenas,
                        *ast_ident,
                        &mut mctx.diagnostics,
                    )?;
                    let (signal, _slice) = net.net.probe_signal();
                    conditions.push((condition, VariableKey::default(), net, *expr));
                    signals.push(signal);
                }
                if contains_edge {
                    for (_, v, net, _) in conditions.iter_mut() {
                        *v = net.net.probe(mctx.gl(), &mut builder);
                    }
                }
                builder = builder.watch(mctx.gl(), signals);
                if contains_edge {
                    let mut acc = builder.constant(mctx.gl(), Bits::new_zeroed(SCALAR_VSIZE));
                    for (condition, before, _, expr) in conditions.into_iter() {
                        use WatchCondition as C;

                        let (after, _) = lower_expr(ctx, mctx, scope, &mut builder, expr, None)?;
                        let cond = match condition {
                            C::Posedge => builder.posedge(mctx.gl(), before, after),
                            C::Negedge => builder.negedge(mctx.gl(), before, after),
                            C::None => builder.not_case_equals(mctx.gl(), before, after),
                        };
                        let cond = builder.reduce_or(mctx.gl(), cond);
                        acc = builder.or(mctx.gl(), acc, cond);
                    }

                    builder = builder.branch_false_to(mctx.gl(), acc, start_key);
                }
                builder = super::lower_statement_or_null(ctx, mctx, scope, builder, statement)?;
            }
        },
    }

    Ok(builder)
}
