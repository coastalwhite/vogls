use std::collections::HashMap;

use vogls_ir::{Bits, GlobalContext};

use crate::ast::AstId;
use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{BinaryOperator, Expr};
use crate::elaborate::VSymbol;
use crate::lower::vvalue::VValue;
use crate::lower::{EvalScope, hident_span, try_resolve_constant, try_resolve_symbol_id};
use crate::number::Sign;
use crate::parser::AstArenas;

use super::Diagnostics;

pub fn eval_constant_expr<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: EvalScope<'_>,
    diagnostics: &mut Diagnostics,
    expr: AstId<ConstantExpr>,
) -> Result<VValue, ()> {
    let expr = expr.into_expr();
    struct StackItem {
        expr: AstId<Expr>,
        dispatched: bool,
    }

    let mut error = false;
    let mut dispatch_stack: Vec<StackItem> = Vec::new();
    let mut result_stack: Vec<Option<VValue>> = Vec::new();

    dispatch_stack.push(StackItem {
        expr,
        dispatched: false,
    });

    'dispatch_loop: while let Some(mut item) = dispatch_stack.pop() {
        match arenas.get(item.expr) {
            Expr::Decimal(decimal) => {
                let decimal = &arenas.decimals[decimal.at];
                result_stack.push(Some(VValue::SignedNet(decimal.clone())));
            }
            Expr::Binary(op, lhs, rhs) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend([*rhs, *lhs].into_iter().map(|expr| StackItem {
                        expr,
                        dispatched: false,
                    }));
                    continue;
                }

                let rhs = result_stack.pop().unwrap();
                let lhs = result_stack.pop().unwrap();

                let (Some(lhs), Some(rhs)) = (lhs, rhs) else {
                    result_stack.push(None);
                    continue;
                };

                use BinaryOperator as O;
                let result = match op {
                    O::Power => todo!("nyi: power"),
                    O::Multiply => VValue::multiply(lhs, rhs),
                    O::Divide => VValue::divide(lhs, rhs),
                    O::Modulus => VValue::remainder(lhs, rhs),
                    O::BinaryPlus => VValue::add(lhs, rhs),
                    O::BinaryMinus => VValue::sub(lhs, rhs),
                    O::ShiftLeft => VValue::logical_shift_left(lhs, rhs),
                    O::ShiftRight => VValue::logical_shift_right(lhs, rhs),
                    O::BitwiseAnd => VValue::bitwise_and(lhs, rhs),
                    O::BitwiseXor => VValue::bitwise_xor(lhs, rhs),
                    O::BitwiseXnor => VValue::bitwise_xnor(lhs, rhs),
                    O::BitwiseOr => VValue::bitwise_or(lhs, rhs),
                    O::LessThan => VValue::scalar_from_bool(VValue::less_than(lhs, rhs)),
                    O::LessThanEqual => VValue::scalar_from_bool(VValue::less_than_equal(lhs, rhs)),
                    O::GreaterThan => VValue::scalar_from_bool(VValue::greater_than(lhs, rhs)),
                    O::GreaterThanEqual => {
                        VValue::scalar_from_bool(VValue::greater_than_equal(lhs, rhs))
                    }
                    O::LogicalAnd => VValue::scalar_from_bool(VValue::logical_and(lhs, rhs)),
                    O::LogicalOr => VValue::scalar_from_bool(VValue::logical_or(lhs, rhs)),
                    O::ArithmeticLeftShift
                    | O::ArithmeticRightShift
                    | O::LogicalEquality
                    | O::LogicalInequality
                    | O::CaseEquality
                    | O::CaseInequality => {
                        result_stack.push(None);
                        diagnostics.not_yet_implemented(
                            arenas.get_span(item.expr),
                            "constant expression of this kind not yet implemented",
                        );
                        error = true;
                        continue;
                    }
                };
                result_stack.push(Some(result));
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                if !exprs.is_empty() || range_expression.is_some() {
                    result_stack.push(None);
                    diagnostics.not_yet_implemented(
                        arenas.get_span(item.expr),
                        "constant expression of this kind not yet implemented",
                    );
                    error = true;
                    continue;
                }

                let Ok(value) =
                    try_resolve_constant(scope.key, scope.table, arenas, *ast_ident, diagnostics)
                else {
                    result_stack.push(None);
                    error = true;
                    continue;
                };
                result_stack.push(Some(value.clone()));
            }
            Expr::Sized(sized) => {
                let sized = &arenas.sized_numbers[sized.item.at];
                let signed = matches!(sized.sign, Sign::Signed);
                result_stack.push(Some(VValue::net(sized.value.clone(), signed)));
            }
            Expr::Ternary(condition, truthy, falsy) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend([*condition, *truthy, *falsy].into_iter().map(|expr| {
                        StackItem {
                            expr,
                            dispatched: false,
                        }
                    }));
                    continue;
                }

                let condition = result_stack.pop().unwrap();
                let truthy = result_stack.pop().unwrap();
                let falsy = result_stack.pop().unwrap();

                let (Some(condition), Some(truthy), Some(falsy)) = (condition, truthy, falsy)
                else {
                    result_stack.push(None);
                    continue;
                };

                let (truthy, falsy) = VValue::coerce_max_size(truthy, falsy);

                if condition.logical_equal(VValue::UnsignedNet(Bits::from(false))) {
                    result_stack.push(Some(falsy));
                } else {
                    result_stack.push(Some(truthy));
                }
            }
            Expr::Concatenation(exprs) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend(exprs.iter().map(|expr| StackItem {
                        expr,
                        dispatched: false,
                    }));
                    continue;
                }

                let end_length = result_stack.len() - exprs.len();
                let Some(mut value) = result_stack.pop().unwrap() else {
                    result_stack.truncate(end_length);
                    result_stack.push(None);
                    continue;
                };

                for _ in 1..exprs.len() {
                    let Some(next) = result_stack.pop().unwrap() else {
                        result_stack.truncate(end_length);
                        result_stack.push(None);
                        continue;
                    };
                    value = VValue::concatenate(value, next);
                }
                result_stack.push(Some(value));
            }
            Expr::SystemFunctionCall(ident, exprs) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    if let Some(exprs) = exprs {
                        dispatch_stack.extend(exprs.iter().map(|expr| StackItem {
                            expr,
                            dispatched: false,
                        }));
                    }
                    continue;
                }

                let num_args = exprs.map_or(0, |e| e.len());
                let result = super::system_function_call::eval_constant(
                    arenas,
                    diagnostics,
                    expr,
                    *ident,
                    &result_stack[result_stack.len() - num_args..],
                );

                result_stack.truncate(result_stack.len() - num_args);
                error |= result.is_err();
                result_stack.push(result.ok());
            }
            Expr::FunctionCall(ident, arguments) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend(arguments.iter().map(|expr| StackItem {
                        expr,
                        dispatched: false,
                    }));
                    continue;
                }

                let return_stack_length = result_stack.len() - arguments.len();

                let Ok(fn_sid) =
                    try_resolve_symbol_id(scope.key, scope.table, arenas, *ident, diagnostics)
                else {
                    result_stack.truncate(return_stack_length);
                    error = true;
                    continue;
                };

                let VSymbol::Function(fn_symbol) = &scope.table[fn_sid].content else {
                    result_stack.truncate(return_stack_length);
                    diagnostics
                        .not_yet_implemented(hident_span(arenas, *ident), "not calling a function");
                    error = true;
                    continue;
                };

                let Some(lowered) = fn_symbol.lowered.as_ref() else {
                    result_stack.truncate(return_stack_length);
                    diagnostics.not_yet_implemented(
                        hident_span(arenas, *ident),
                        "function is not yet lowered",
                    );
                    error = true;
                    continue;
                };
                if fn_symbol.inputs.len() != arguments.len() {
                    result_stack.truncate(return_stack_length);
                    diagnostics
                        .not_yet_implemented(arenas.get_span(expr), "invalid number of arguments");
                    error = true;
                    continue;
                }

                let mut esignals = HashMap::new();
                let mut evars = HashMap::new();

                for ((sig, ty), value) in fn_symbol
                    .inputs
                    .iter()
                    .zip(result_stack.drain(return_stack_length..))
                {
                    let Some(value) = value else {
                        error = true;
                        continue 'dispatch_loop;
                    };
                    esignals.insert(
                        *sig,
                        value.truncate_or_extend(ty.force_net_width()).into_bits(),
                    );
                }
                esignals.insert(
                    fn_symbol.output,
                    Bits::new_unknown(fn_symbol.output_ty.force_net_width()),
                );

                vogls_ir::evaluation::evaluate(gl, lowered.entry, &mut esignals, &mut evars);

                result_stack.push(Some(VValue::net(
                    esignals[&fn_symbol.output].clone(),
                    fn_symbol.output_ty.is_signed(),
                )));
            }
            Expr::String(..) | Expr::Unary(..) | Expr::Replication(..) => {
                result_stack.push(None);
                diagnostics.not_yet_implemented(
                    arenas.get_span(item.expr),
                    "constant expression of this kind not yet implemented",
                );
                error = true;
            }
        }
    }

    if error {
        return Err(());
    }

    assert_eq!(result_stack.len(), 1);
    let Some(value) = result_stack.pop().unwrap() else {
        panic!();
    };

    Ok(value)
}
