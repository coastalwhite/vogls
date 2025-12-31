use vogls_ir::{Bits, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, VectorSize};

use crate::ast::AstId;
use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{BinaryOperator, Expr};
use crate::lower::scope::SymbolVariant;
use crate::number::{Decimal, Sign};
use crate::parser::AstArenas;

use super::Diagnostics;
use super::scope::Scope;
use super::vvalue::VValue;

pub fn eval_constant_expr<'a>(
    _gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
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

    while let Some(mut item) = dispatch_stack.pop() {
        match arenas.get(item.expr) {
            Expr::Decimal(decimal) => {
                let decimal = &arenas.decimals[decimal.at];
                let Decimal::Small(v) = decimal else {
                    result_stack.push(None);
                    diagnostics.not_yet_implemented(
                        arenas.get_span(item.expr),
                        "constant expression of this kind not yet implemented",
                    );
                    error = true;
                    continue;
                };

                result_stack.push(Some(VValue::SignedNet(Bits::from_i64_truncated(
                    *v as i64,
                    INTEGER_VSIZE,
                ))));
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

                let ident = arenas.get_ident(ast_ident.item.0);
                let Some(symbol_key) = scope.get(ident) else {
                    result_stack.push(None);
                    diagnostics.var_not_found(arenas, *ast_ident);
                    error = true;
                    continue;
                };
                let value = match &scope.symbols[symbol_key].variant {
                    SymbolVariant::Genvar(n) => {
                        VValue::SignedNet(Bits::from_i64_truncated(n.unwrap(), INTEGER_VSIZE))
                    }
                    SymbolVariant::Constant(n) => n.clone(),
                    SymbolVariant::Task(_) => todo!(),
                    SymbolVariant::Signal(..) => {
                        result_stack.push(None);
                        diagnostics.not_yet_implemented(
                            arenas.get_item_span(*ast_ident),
                            "non-constant symbol in constant-expr",
                        );
                        error = true;
                        continue;
                    }
                };
                result_stack.push(Some(value));
            }
            Expr::Sized(sized) => {
                let sized = &arenas.sized_numbers[sized.item.at];
                let signed = matches!(sized.sign, Sign::Signed);
                let crate::number::Bits::Small(v) = sized.value else {
                    todo!()
                };
                let width = match sized.size {
                    None => (64 - v.leading_zeros()).max(1),
                    Some(size) => size.as_u32(),
                };
                assert!(width <= 64);
                let width = VectorSize::new(width).unwrap();
                result_stack.push(Some(VValue::net(Bits::Small(v, width), signed)));
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

                if condition.logical_equal(VValue::UnsignedNet(Bits::Small(0, SCALAR_VSIZE))) {
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
            Expr::FunctionCall(..)
            | Expr::SystemFunctionCall(..)
            | Expr::String(..)
            | Expr::Unary(..)
            | Expr::Replication(..) => {
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
