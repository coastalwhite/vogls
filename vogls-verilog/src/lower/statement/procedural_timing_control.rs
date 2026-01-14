use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, Bits, GlobalContext, SCALAR_VSIZE, Time, VariableKey,
};

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::statement::{
    DelayControl, DelayValue, EventControl, EventExpressionPrimary, ProceduralTimingControl,
    StatementOrNull,
};
use crate::hierarchy::{HierarchyItem, HierarchyParameter};
use crate::lower::Scope;
use crate::lower::WatchCondition;
use crate::lower::expression::lower_expr;
use crate::parser::AstArenas;

use super::{Diagnostics, Region};

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    ptc: AstId<ProceduralTimingControl>,
    statement: AstId<StatementOrNull>,
) -> Result<BasicBlockBuilder, ()> {
    match arenas.get(ptc) {
        ProceduralTimingControl::DelayControl(ast_delay_control) => {
            let delay_control = arenas.get(*ast_delay_control);
            match delay_control {
                DelayControl::DelayValue(value) => {
                    let value = match arenas.get(*value) {
                        DelayValue::UnsignedNumber(value) => {
                            let value = &arenas.decimals[value.at];
                            value.as_u64().unwrap()
                        }
                        DelayValue::Identifier(ast_ident) => {
                            let ident = arenas.get_ident(ast_ident.0);
                            let Some(symbol_key) = scope.get(ident) else {
                                diagnostics.not_yet_implemented(
                                    arenas.get_span(*ast_delay_control),
                                    "Ident not found",
                                );
                                return Err(());
                            };
                            let symbol = &scope.hierarchy.items()[symbol_key.as_idx()];
                            let value = match &symbol {
                                HierarchyItem::Parameter(s) => {
                                    let HierarchyParameter {
                                        name: _,
                                        parent: _,
                                        value,
                                    } = &scope.hierarchy.parameters()[*s];
                                    // @TODO: Remove unwrap
                                    value.as_integer().unwrap() as u64
                                }
                                _ => todo!(),
                            };
                            value
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
                        builder.wait_region(gl, Region::Inactive as u8)
                    } else {
                        builder.wait(gl, Time(value as u64))
                    };
                    builder = super::lower_statement_or_null(
                        gl,
                        arenas,
                        scope,
                        diagnostics,
                        builder,
                        statement,
                    )?;
                }
                DelayControl::MinTypMax(_) => todo!(),
            }
        }
        ProceduralTimingControl::EventControl(event_control) => match arenas.get(*event_control) {
            EventControl::Star => {
                let start_bb = builder.key();
                builder = builder.jump(gl);

                let process = builder.process();
                let start_ins = gl.processes[process].ins.len();

                let statement_start_bb = builder.key();
                builder = super::lower_statement_or_null(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    builder,
                    statement,
                )?;
                let statement_end_bb = builder.key();

                builder = builder.jump(gl);
                let watch_bb = builder.key();
                gl.bbs[start_bb].terminator = BasicBlockTerminator::Jump(watch_bb);
                let end_ins = gl.processes[process].ins.len();

                let signals = gl.processes[process]
                    .ins
                    .iter()
                    .skip(start_ins)
                    .take(end_ins - start_ins)
                    .copied()
                    .collect::<Vec<_>>();

                let before = signals
                    .iter()
                    .map(|s| builder.probe(gl, *s))
                    .collect::<Vec<_>>();
                builder = builder.watch(gl, signals.clone());

                let mut acc = builder.constant(gl, Bits::Small(0, SCALAR_VSIZE));
                for (before, signal) in before.into_iter().zip(signals) {
                    let after = builder.probe(gl, signal);
                    let cond = builder.not_equals(gl, before, after);
                    acc = builder.or(gl, acc, cond);
                }

                builder = builder.branch_false_to(gl, acc, watch_bb);
                let next_builder = builder.next_builder(gl);
                builder.jump_to(gl, statement_start_bb);
                builder = next_builder;
                gl.bbs[statement_end_bb].terminator = BasicBlockTerminator::Jump(builder.key());
            }
            EventControl::EventExpression(event_expression) => {
                builder = builder.jump(gl);
                let start_key = builder.key();

                let mut conditions: Vec<(WatchCondition, VariableKey, AstId<Expr>)> = Vec::new();
                let mut signals = Vec::new();
                for event_expression in event_expression.0.iter() {
                    let (expr, condition) = match arenas.get(event_expression) {
                        EventExpressionPrimary::Expression(expr) => (expr, WatchCondition::None),
                        EventExpressionPrimary::Posedge(expr) => (expr, WatchCondition::Posedge),
                        EventExpressionPrimary::Negedge(expr) => (expr, WatchCondition::Negedge),
                    };

                    let Expr::Ident(ast_ident, exprs, range_expression) = arenas.get(*expr) else {
                        panic!("not an ident");
                    };
                    if !exprs.is_empty() || range_expression.is_some() {
                        diagnostics.not_yet_implemented(
                            arenas.get_span(*expr),
                            "event expression of this kind",
                        );
                        return Err(());
                    }

                    let ident = arenas.get_ident(ast_ident.item.0);
                    let Some(symbol_key) = scope.get(ident) else {
                        diagnostics.var_not_found(arenas, *ast_ident);
                        return Err(());
                    };
                    let HierarchyItem::Net(s) = &scope.hierarchy.items()[symbol_key.as_idx()]
                    else {
                        panic!("not a signal");
                    };
                    let s = &scope.hierarchy.net()[*s];
                    let key = s.signal;

                    let (variable, _) =
                        lower_expr(gl, arenas, scope, diagnostics, &mut builder, *expr)?;
                    conditions.push((condition, variable, *expr));
                    signals.push(key);
                }
                builder = builder.watch(gl, signals);

                let mut acc = builder.constant(gl, Bits::new_zeroed(SCALAR_VSIZE));
                for (condition, before, expr) in conditions.into_iter() {
                    use WatchCondition as C;

                    let (after, _) =
                        lower_expr(gl, arenas, scope, diagnostics, &mut builder, expr)?;
                    let cond = match condition {
                        C::Posedge => {
                            let t = builder.binary_neg(gl, before);
                            builder.and(gl, t, after)
                        }
                        C::Negedge => {
                            let t = builder.binary_neg(gl, after);
                            builder.and(gl, before, t)
                        }
                        C::None => builder.not_equals(gl, before, after),
                    };
                    let cond = builder.reduce_or(gl, cond);
                    acc = builder.or(gl, acc, cond);
                }

                builder = builder.branch_false_to(gl, acc, start_key);
                builder = super::lower_statement_or_null(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    builder,
                    statement,
                )?;
            }
        },
    }

    Ok(builder)
}
