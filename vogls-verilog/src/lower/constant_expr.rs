use vogls_ir::GlobalContext;

use crate::ast::AstId;
use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{BinaryOperator, Expr};
use crate::lower::scope::SymbolVariant;
use crate::number::Decimal;
use crate::parser::AstArenas;

use super::scope::Scope;
use super::vvalue::VValue;
use super::{Diagnostics, VTypeTable};

pub fn eval_constant_expr<'a>(
    _gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
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

                result_stack.push(Some(VValue::Integer(*v as i64)));
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

                let (Some(VValue::Integer(lhs)), Some(VValue::Integer(rhs))) = (lhs, rhs) else {
                    result_stack.push(None);
                    continue;
                };

                use BinaryOperator as O;
                let result = match op {
                    O::Multiply => lhs * rhs,
                    O::Divide => lhs / rhs,
                    O::Modulus => lhs % rhs,
                    O::BinaryPlus => lhs + rhs,
                    O::BinaryMinus => lhs - rhs,
                    O::ShiftLeft => lhs << rhs,
                    O::ShiftRight => lhs >> rhs,
                    O::BitwiseAnd => lhs & rhs,
                    O::BitwiseXor => lhs ^ rhs,
                    O::BitwiseXnor => !(lhs ^ rhs),
                    O::BitwiseOr => lhs | rhs,
                    O::LessThan => i64::from(lhs < rhs),
                    O::GreaterThan
                    | O::GreaterThanEqual
                    | O::LessThanEqual
                    | O::LogicalEquality
                    | O::LogicalInequality
                    | O::CaseEquality
                    | O::CaseInequality
                    | O::LogicalAnd
                    | O::LogicalOr => {
                        result_stack.push(None);
                        diagnostics.not_yet_implemented(
                            arenas.get_span(item.expr),
                            "constant expression of this kind not yet implemented",
                        );
                        error = true;
                        continue;
                    }
                };
                result_stack.push(Some(VValue::Integer(result)));
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
                let n = match scope.symbols[symbol_key].variant {
                    SymbolVariant::Genvar(n) => n.unwrap(),
                    SymbolVariant::Constant(n) => n.unwrap(),
                    SymbolVariant::Variable(_) | SymbolVariant::Signal(_) => {
                        result_stack.push(None);
                        diagnostics.not_yet_implemented(
                            arenas.get_item_span(*ast_ident),
                            "non-constant symbol in constant-expr",
                        );
                        error = true;
                        continue;
                    }
                };
                result_stack.push(Some(VValue::Integer(n)));
            }
            Expr::Sized(..)
            | Expr::String(..)
            | Expr::Unary(..)
            | Expr::Concatenation(..)
            | Expr::Replication(..)
            | Expr::Ternary(..) => {
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
