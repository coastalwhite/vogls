use std::collections::HashMap;

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{Bits, GlobalContext, VectorSize};

use crate::ast::AstId;
use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{BinaryOperator, BitSlice, Expr, Replication, UnaryOperator};
use crate::elaborate::{VSymbol, VSymbolTable};
use crate::lower::expression::{StackItem, get_expr_type};
use crate::lower::vvalue::VValue;
use crate::lower::{hident_span, try_resolve_constant, try_resolve_hident};
use crate::number::Sign;
use crate::parser::AstArenas;

use super::Diagnostics;

pub fn eval_constant_expr<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    table: &VSymbolTable,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    expr: AstId<'a, ConstantExpr<'a>>,
    context_width: Option<VectorSize>,
) -> Result<VValue, ()> {
    let expr = expr.into_expr();

    let mut error = false;
    let mut dispatch_stack: Vec<StackItem<'a>> = Vec::new();
    let mut result_stack: Vec<Option<VValue>> = Vec::new();

    dispatch_stack.push(StackItem::new(expr, context_width));

    while let Some(mut item) = dispatch_stack.pop() {
        match *item.expr {
            Expr::Decimal(decimal) => {
                let decimal = &arenas.decimals[decimal.at];
                result_stack.push(Some(VValue::SignedNet(decimal.clone())));
            }
            Expr::Unary(op, child) => {
                if !item.dispatched {
                    item.dispatched = true;
                    let child_context_width =
                        item.context_width.filter(|_| op.is_self_determined());
                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new(child, child_context_width));
                    continue;
                }

                let Some(mut child) = result_stack.pop().unwrap() else {
                    result_stack.push(None);
                    continue;
                };

                if let Some(context_width) = item.context_width
                    && !op.is_self_determined()
                    && context_width > child.ty().force_net_width()
                {
                    child = child.zero_or_sign_extend(context_width);
                }

                use UnaryOperator as O;
                let result = match op {
                    O::LogicalNegation => VValue::scalar_from_bool(child.to_logical()),
                    O::BitwiseNegation => child.bitwise_invert(),
                    O::ReductionAnd
                    | O::ReductionOr
                    | O::ReductionNand
                    | O::ReductionNor
                    | O::ReductionXor
                    | O::ReductionXnor
                    | O::SignPlus
                    | O::SignMinus => {
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
            Expr::Binary(op, lhs, rhs) => {
                if !item.dispatched {
                    item.dispatched = true;

                    let (Ok(l_ty), Ok(r_ty)) = (
                        get_expr_type(gl, arenas, table, scope, diagnostics, lhs),
                        get_expr_type(gl, arenas, table, scope, diagnostics, rhs),
                    ) else {
                        result_stack.push(None);
                        error = true;
                        continue;
                    };

                    let mut child_context_width =
                        op.output_width(l_ty.force_net_width(), r_ty.force_net_width());
                    let (l_is_self_det, r_is_self_det) = op.is_self_determined();
                    if let Some(context_width) = item.context_width {
                        child_context_width = child_context_width.max(context_width);
                    }

                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new(
                        rhs,
                        (!r_is_self_det).then_some(child_context_width),
                    ));
                    dispatch_stack.push(StackItem::new(
                        lhs,
                        (!l_is_self_det).then_some(child_context_width),
                    ));
                    continue;
                }

                let rhs = result_stack.pop().unwrap();
                let lhs = result_stack.pop().unwrap();

                let (Some(mut lhs), Some(mut rhs)) = (lhs, rhs) else {
                    result_stack.push(None);
                    continue;
                };

                let (l_is_self_det, r_is_self_det) = op.is_self_determined();
                if let Some(context_width) = item.context_width {
                    if !l_is_self_det && context_width > lhs.ty().force_net_width() {
                        lhs = lhs.zero_or_sign_extend(context_width);
                    }
                    if !r_is_self_det && context_width > rhs.ty().force_net_width() {
                        rhs = rhs.zero_or_sign_extend(context_width);
                    }
                }

                use BinaryOperator as O;
                let result = match op {
                    O::Power => VValue::power(lhs, rhs),
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
                    O::LessThan => VValue::from(VValue::less_than(lhs, rhs)),
                    O::LessThanEqual => VValue::from(VValue::less_than_equal(lhs, rhs)),
                    O::GreaterThan => VValue::from(VValue::greater_than(lhs, rhs)),
                    O::GreaterThanEqual => VValue::from(VValue::greater_than_equal(lhs, rhs)),
                    O::LogicalAnd => VValue::scalar_from_bool(VValue::logical_and(lhs, rhs)),
                    O::LogicalOr => VValue::scalar_from_bool(VValue::logical_or(lhs, rhs)),
                    O::LogicalEquality => VValue::scalar_from_bool(lhs.logical_equal(rhs)),
                    O::LogicalInequality => VValue::scalar_from_bool(lhs.logical_not_equal(rhs)),
                    O::ArithmeticLeftShift
                    | O::ArithmeticRightShift
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
                if !exprs.is_empty() {
                    result_stack.push(None);
                    diagnostics.not_yet_implemented(
                        arenas.get_span(item.expr),
                        "constant expression of this kind not yet implemented",
                    );
                    error = true;
                    continue;
                }

                if !item.dispatched && range_expression.is_some() {
                    item.dispatched = true;

                    dispatch_stack.push(item);
                    if let Some(range_expression) = range_expression {
                        match range_expression {
                            BitSlice::MsbLsb(msb, lsb) => dispatch_stack.extend([
                                StackItem::new_no_ctx(msb.into_expr()),
                                StackItem::new_no_ctx(lsb.into_expr()),
                            ]),
                            BitSlice::PlusWidth(offset, width)
                            | BitSlice::MinusWidth(offset, width) => dispatch_stack.extend([
                                StackItem::new_no_ctx(offset),
                                StackItem::new_no_ctx(width.into_expr()),
                            ]),
                        }
                    }
                    continue;
                }

                let end_length =
                    result_stack.len() - range_expression.is_some().then_some(2).unwrap_or(0);
                let Ok(value) = try_resolve_constant(scope, &table, arenas, ast_ident, diagnostics)
                else {
                    result_stack.truncate(end_length);
                    result_stack.push(None);
                    error = true;
                    continue;
                };

                let mut value = value.clone();
                if let Some(range_expression) = range_expression {
                    let fst = result_stack.pop().unwrap();
                    let snd = result_stack.pop().unwrap();

                    let (Some(fst), Some(snd)) = (fst, snd) else {
                        result_stack.truncate(end_length);
                        result_stack.push(None);
                        continue;
                    };

                    let (lsb, width) = match range_expression {
                        BitSlice::MsbLsb(..) => {
                            // @TODO: Fallible.
                            let msb = fst.as_integer().unwrap();
                            let lsb = snd.as_integer().unwrap();
                            (
                                lsb as u32,
                                VectorSize::new((msb as u32 - lsb as u32) + 1).unwrap(),
                            )
                        }
                        BitSlice::PlusWidth(..) => {
                            // @TODO: Fallible.
                            let offset = fst.as_integer().unwrap();
                            let width = snd.as_integer().unwrap();
                            (offset as u32, VectorSize::new(width as u32).unwrap())
                        }
                        BitSlice::MinusWidth(..) => {
                            // @TODO: Fallible.
                            let offset = fst.as_integer().unwrap();
                            let width = snd.as_integer().unwrap();
                            (
                                offset as u32 - (width as u32 - 1),
                                VectorSize::new(width as u32).unwrap(),
                            )
                        }
                    };

                    value = VValue::UnsignedNet(value.into_bits().slicex(lsb, width));
                }

                result_stack.push(Some(value));
            }
            Expr::Sized(sized) => {
                let sized = &arenas.sized_numbers[sized.item.at];
                let signed = matches!(sized.sign, Sign::Signed);
                result_stack.push(Some(VValue::net(sized.value.clone(), signed)));
            }
            Expr::Ternary(condition, truthy, falsy) => {
                if !item.dispatched {
                    item.dispatched = true;

                    let (Ok(l_ty), Ok(r_ty)) = (
                        get_expr_type(gl, arenas, table, scope, diagnostics, truthy),
                        get_expr_type(gl, arenas, table, scope, diagnostics, falsy),
                    ) else {
                        result_stack.push(None);
                        error = true;
                        continue;
                    };
                    let mut child_context_width =
                        l_ty.force_net_width().max(r_ty.force_net_width());
                    if let Some(context_width) = item.context_width {
                        child_context_width = child_context_width.max(context_width);
                    }

                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new_no_ctx(condition));
                    dispatch_stack.extend(
                        [truthy, falsy]
                            .into_iter()
                            .map(|e| StackItem::new(e, Some(child_context_width))),
                    );
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
                    dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx));
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
                        dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx));
                    }
                    continue;
                }

                let num_args = exprs.map_or(0, |e| e.len());
                let result = super::system_function_call::eval_constant(
                    arenas,
                    diagnostics,
                    expr,
                    ident,
                    &result_stack[result_stack.len() - num_args..],
                );

                result_stack.truncate(result_stack.len() - num_args);
                error |= result.is_err();
                result_stack.push(result.ok());
            }
            Expr::FunctionCall(ident, arguments) => {
                if !item.dispatched {
                    item.dispatched = true;

                    let Ok(fn_sid) = try_resolve_hident(scope, &table, arenas, ident, diagnostics)
                    else {
                        result_stack.push(None);
                        error = true;
                        continue;
                    };
                    let VSymbol::Function(fn_symbol) = &table[fn_sid].content else {
                        result_stack.push(None);
                        diagnostics.not_yet_implemented(
                            hident_span(arenas, ident),
                            "not calling a function",
                        );
                        error = true;
                        continue;
                    };

                    dispatch_stack.push(item);
                    dispatch_stack.extend(
                        arguments
                            .iter()
                            .zip(&fn_symbol.inputs)
                            .map(|(expr, (_, ty))| {
                                StackItem::new(expr, Some(ty.force_net_width()))
                            }),
                    );
                    continue;
                }

                let return_stack_length = result_stack.len() - arguments.len();

                let Ok(fn_sid) = try_resolve_hident(scope, &table, arenas, ident, diagnostics)
                else {
                    result_stack.truncate(return_stack_length);
                    result_stack.push(None);
                    error = true;
                    continue;
                };

                let VSymbol::Function(fn_symbol) = &table[fn_sid].content else {
                    result_stack.truncate(return_stack_length);
                    result_stack.push(None);
                    diagnostics
                        .not_yet_implemented(hident_span(arenas, ident), "not calling a function");
                    error = true;
                    continue;
                };

                let Some(lowered) = fn_symbol.lowered.as_ref() else {
                    result_stack.truncate(return_stack_length);
                    diagnostics.not_yet_implemented(
                        hident_span(arenas, ident),
                        "function is not yet lowered",
                    );
                    error = true;
                    continue;
                };
                if fn_symbol.inputs.len() != arguments.len() {
                    result_stack.truncate(return_stack_length);
                    result_stack.push(None);
                    diagnostics
                        .not_yet_implemented(arenas.get_span(expr), "invalid number of arguments");
                    error = true;
                    continue;
                }

                let mut esignals = HashMap::new();
                let mut evars = HashMap::new();

                let mut inputs_error = false;
                for ((sig, ty), value) in fn_symbol
                    .inputs
                    .iter()
                    .zip(result_stack.drain(return_stack_length..))
                {
                    let Some(value) = value else {
                        inputs_error = true;
                        break;
                    };
                    esignals.insert(
                        *sig,
                        value.truncate_or_extend(ty.force_net_width()).into_bits(),
                    );
                }
                if inputs_error {
                    result_stack.push(None);
                    error = true;
                    continue;
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
            Expr::String(string_ref) => {
                let s = arenas.get_ident(string_ref.0);
                let s = s
                    .as_bytes()
                    .iter()
                    .copied()
                    .chain(std::iter::once(b'\0'))
                    .collect::<Box<[u8]>>();
                let size = VectorSize::new((s.len() * 8) as u32).unwrap();
                let value = Bits::load_from_slice(&s, size);
                result_stack.push(Some(VValue::UnsignedNet(value)));
            }
            Expr::Replication(id) => {
                let Replication {
                    constant_expr,
                    exprs,
                } = &id;
                assert!(exprs.len() > 0);
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new_no_ctx(constant_expr.into_expr()));
                    dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx));
                    continue;
                }

                // @Performance. Allocate once.
                let end_length = result_stack.len() - exprs.len();
                let Some(num_reps) = result_stack.pop().unwrap() else {
                    result_stack.truncate(end_length);
                    result_stack.push(None);
                    continue;
                };

                let Some(fst) = result_stack.pop().unwrap() else {
                    result_stack.truncate(end_length);
                    result_stack.push(None);
                    continue;
                };
                let mut acc = fst.into_bits();
                for _ in 1..exprs.len() {
                    let Some(value) = result_stack.pop().unwrap() else {
                        result_stack.truncate(end_length);
                        result_stack.push(None);
                        continue;
                    };
                    acc = Bits::concatenate(&acc, &value.into_bits());
                }

                // @TODO: Binary concatenations
                let mut final_acc = acc.clone();
                for _ in 1..num_reps.as_integer().unwrap() {
                    final_acc = Bits::concatenate(&final_acc, &acc);
                }
                result_stack.push(Some(VValue::UnsignedNet(final_acc)));
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
