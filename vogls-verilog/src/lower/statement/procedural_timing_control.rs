use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, BasicBlockTerminator, Bits, SCALAR_VSIZE, Time, VariableKey};
use vogls_utils::OrderedSet;

use crate::ast::expr::Expr;
use crate::ast::statement::{
    DelayControl, DelayValue, EventControl, EventExpressionPrimary, ProceduralTimingControl,
    StatementOrNull,
};
use crate::ast::{AstId, AstItem};
use crate::lower::expression::lower_expr;
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
                DelayControl::DelayValue(ast_value) => {
                    let value = match &**ast_value {
                        DelayValue::UnsignedNumber(value) => {
                            let value = &ctx.arenas.decimals[value.at];
                            value.as_u64().unwrap()
                        }
                        DelayValue::Identifier(ident) => {
                            let ident = AstItem {
                                item: *ident,
                                loc: ast_value.loc,
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

                    builder = if value == 0 {
                        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 159
                        //
                        // """
                        // An explicit zero delay (#0) requires that the process be
                        // suspended and added as an inactive event for the current time so
                        // that the process is resumed in the next simulation cycle in the
                        // current time.
                        // """
                        builder.wait_region(mctx.gl(), Region::Inactive as u8)
                    } else {
                        builder.wait(mctx.gl(), Time(value as u64))
                    };
                    builder = super::lower_statement_or_null(ctx, mctx, scope, builder, statement)?;
                }
                DelayControl::MinTypMax(_) => todo!(),
            }
        }
        ProceduralTimingControl::EventControl(event_control) => match &**event_control {
            EventControl::Star => {
                let start_bb = builder.key();
                builder = builder.jump(mctx.gl());

                let mut ins = OrderedSet::new();
                match &*statement {
                    StatementOrNull::Attribute(_) => {}
                    StatementOrNull::Statement(stmt) => {
                        super::get_used_signals(ctx, mctx, scope, &mut ins, *stmt)?
                    }
                }

                let statement_start_bb = builder.key();
                builder = super::lower_statement_or_null(ctx, mctx, scope, builder, statement)?;
                let statement_end_bb = builder.key();

                builder = builder.jump(mctx.gl());
                let watch_bb = builder.key();
                mctx.gl.bbs[start_bb].terminator = BasicBlockTerminator::Jump(watch_bb);

                let before = ins
                    .items
                    .iter()
                    .map(|s| builder.probe(mctx.gl(), *s))
                    .collect::<Vec<_>>();
                builder = builder.watch(mctx.gl(), ins.items.clone());

                let mut acc = builder.constant(mctx.gl(), Bits::from(false));
                for (before, signal) in before.into_iter().zip(ins.items) {
                    let after = builder.probe(mctx.gl(), signal);
                    let cond = builder.not_case_equals(mctx.gl(), before, after);
                    acc = builder.or(mctx.gl(), acc, cond);
                }

                builder = builder.branch_false_to(mctx.gl(), acc, watch_bb);
                let next_builder = builder.next_builder(mctx.gl());
                builder.jump_to(mctx.gl(), statement_start_bb);
                builder = next_builder;
                mctx.gl.bbs[statement_end_bb].terminator =
                    BasicBlockTerminator::Jump(builder.key());
            }
            EventControl::EventExpression(event_expression) => {
                builder = builder.jump(mctx.gl());
                let start_key = builder.key();

                let mut conditions: Vec<(WatchCondition, VariableKey, AstId<Expr>)> = Vec::new();
                let mut signals = Vec::new();
                for event_expression in event_expression.0.iter() {
                    let (expr, condition) = match &*event_expression {
                        EventExpressionPrimary::Expression(expr) => (expr, WatchCondition::None),
                        EventExpressionPrimary::Posedge(expr) => (expr, WatchCondition::Posedge),
                        EventExpressionPrimary::Negedge(expr) => (expr, WatchCondition::Negedge),
                    };

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
                    let key = net.net.probe_signal();

                    let (variable, _) = lower_expr(ctx, mctx, scope, &mut builder, *expr)?;
                    conditions.push((condition, variable, *expr));
                    signals.push(key);
                }
                builder = builder.watch(mctx.gl(), signals);

                let mut acc = builder.constant(mctx.gl(), Bits::new_zeroed(SCALAR_VSIZE));
                for (condition, before, expr) in conditions.into_iter() {
                    use WatchCondition as C;

                    let (after, _) = lower_expr(ctx, mctx, scope, &mut builder, expr)?;
                    let cond = match condition {
                        C::Posedge => builder.posedge(mctx.gl(), before, after),
                        C::Negedge => builder.negedge(mctx.gl(), before, after),
                        C::None => builder.not_case_equals(mctx.gl(), before, after),
                    };
                    let cond = builder.reduce_or(mctx.gl(), cond);
                    acc = builder.or(mctx.gl(), acc, cond);
                }

                builder = builder.branch_false_to(mctx.gl(), acc, start_key);
                builder = super::lower_statement_or_null(ctx, mctx, scope, builder, statement)?;
            }
        },
    }

    Ok(builder)
}
