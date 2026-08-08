use std::num::NonZeroU32;

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{GlobalContext, VectorSize, INTEGER_VSIZE, SCALAR_VSIZE, VSIZE_8};

use crate::ast::AstId;
use crate::ast::expr::{BinaryOperator, BitSlice, Expr, Replication, UnaryOperator};
use crate::elaborate::{VSymbol, VSymbolTable};
use crate::lower::expression::system_function_call::get_system_function_call_output_ty;
use crate::lower::expression::{
    coerce_to_max_size_ty, is_zero_sized_replication, system_function_call,
};
use crate::lower::{
    Diagnostics, VType, eval_constant_expr, hident_span, msb_lsb_to_width, try_resolve_hident,
};
use crate::number::Sign;
use crate::parser::AstArenas;

use super::StackItem;

#[deny(clippy::question_mark_used)] // Needs to be handled explicitly in the recursion.
pub fn get_expr_type<'a>(
    gl: &GlobalContext,
    arenas: &AstArenas,
    table: &VSymbolTable,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    expr: AstId<'a, Expr<'a>>,
) -> Result<VType, ()> {
    let mut error = false;
    let mut dispatch_stack: Vec<StackItem<'a>> = Vec::new();
    let mut result_stack: Vec<Option<VType>> = Vec::new();

    dispatch_stack.push(StackItem::new_no_ctx(expr));

    while let Some(mut item) = dispatch_stack.pop() {
        match *item.expr {
            Expr::Unary(op, child) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new_no_ctx(child));
                    continue;
                }

                let child = result_stack.pop().unwrap();
                let Some(child) = child else {
                    result_stack.push(None);
                    continue;
                };

                use UnaryOperator as O;
                let ty = VType::net(
                    op.output_size(child.force_net_width()),
                    child.is_signed() && matches!(op, O::SignPlus | O::SignMinus),
                );
                result_stack.push(Some(ty));
            }
            Expr::Binary(op, l, r) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend([r, l].into_iter().map(StackItem::new_no_ctx));
                    continue;
                }

                let r = result_stack.pop().unwrap();
                let l = result_stack.pop().unwrap();

                let (Some(l), Some(r)) = (l, r) else {
                    result_stack.push(None);
                    continue;
                };

                // @Performance. Don't traverse type for expressions that aren't influential to the
                // output type.
                use BinaryOperator as O;
                let ty = match op {
                    O::GreaterThan
                    | O::GreaterThanEqual
                    | O::LessThan
                    | O::LessThanEqual
                    | O::LogicalEquality
                    | O::LogicalInequality
                    | O::CaseEquality
                    | O::CaseInequality
                    | O::LogicalAnd
                    | O::LogicalOr => VType::SCALAR_NET,

                    O::ShiftLeft
                    | O::ShiftRight
                    | O::ArithmeticLeftShift
                    | O::ArithmeticRightShift => l,

                    O::Power
                    | O::Multiply
                    | O::Divide
                    | O::Modulus
                    | O::BinaryPlus
                    | O::BinaryMinus
                    | O::BitwiseAnd
                    | O::BitwiseXor
                    | O::BitwiseXnor
                    | O::BitwiseOr => coerce_to_max_size_ty(l, r),
                };
                result_stack.push(Some(ty));
            }
            Expr::Concatenation(exprs) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend(
                        exprs
                            .iter()
                            .rev()
                            .filter(|e| !is_zero_sized_replication(gl, arenas, table, scope, e))
                            .map(StackItem::new_no_ctx),
                    );
                    continue;
                }

                let num_exprs = exprs
                    .iter()
                    .filter(|e| !is_zero_sized_replication(gl, arenas, table, scope, e))
                    .count();
                let mut size = 0;
                let mut child_error = false;
                for ty in result_stack.drain(..num_exprs) {
                    let Some(ty) = ty else {
                        child_error = true;
                        break;
                    };
                    // @TODO: Overflow check.
                    size += ty.force_net_width().get();
                }
                if child_error {
                    result_stack.push(None);
                    continue;
                }

                let Some(size) = VectorSize::new(size) else {
                    diagnostics.not_yet_implemented(
                        arenas.get_span(item.expr),
                        "concatenation without expressions",
                    );
                    error = true;
                    result_stack.push(None);
                    continue;
                };
                result_stack.push(Some(VType::UnsignedNet(size)));
            }
            Expr::Replication(replication) => {
                let Replication {
                    constant_expr,
                    exprs,
                } = replication;

                if exprs.is_empty() {
                    diagnostics.not_yet_implemented(
                        arenas.get_span(item.expr),
                        "concatenation without expressions",
                    );
                    error = true;
                    result_stack.push(None);
                    continue;
                }

                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend(exprs.iter().rev().map(StackItem::new_no_ctx));
                    continue;
                }

                let end_stack_size = result_stack.len() - exprs.len();
                let Ok(repeat_n) =
                    eval_constant_expr(gl, arenas, table, scope, diagnostics, constant_expr, None)
                else {
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                };

                let Some(repeat_n) = repeat_n.as_integer() else {
                    diagnostics.not_yet_implemented(
                        arenas.get_span(constant_expr),
                        "replication overflow",
                    );
                    error = true;
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                };
                if repeat_n == 0 {
                    diagnostics
                        .not_yet_implemented(arenas.get_span(constant_expr), "replication is 0");
                    error = true;
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                }
                let Some(ty) = result_stack.pop().unwrap() else {
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                };
                let mut width = ty.force_net_width().get();
                for _ in 1..exprs.len() {
                    let Some(next_ty) = result_stack.pop().unwrap() else {
                        result_stack.truncate(end_stack_size);
                        result_stack.push(None);
                        continue;
                    };
                    let next_width = next_ty.force_net_width();
                    width += next_width.get();
                }

                let Some(output_width) = width.checked_mul(repeat_n as u32) else {
                    diagnostics
                        .not_yet_implemented(arenas.get_span(item.expr), "replication overflow");
                    error = true;
                    result_stack.push(None);
                    continue;
                };

                result_stack.push(Some(VType::UnsignedNet(
                    VectorSize::new(output_width).unwrap(),
                )));
            }
            Expr::Ternary(_, t, f) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend([f, t].into_iter().map(StackItem::new_no_ctx));
                    continue;
                }

                let f = result_stack.pop().unwrap();
                let t = result_stack.pop().unwrap();

                let (Some(t), Some(f)) = (t, f) else {
                    result_stack.push(None);
                    continue;
                };
                result_stack.push(Some(coerce_to_max_size_ty(t, f)));
            }
            Expr::Ident(ident, exprs, range_expr) => {
                let Ok(symbol_id) = try_resolve_hident(scope, table, arenas, ident, diagnostics)
                else {
                    error = true;
                    result_stack.push(None);
                    continue;
                };
                let (ty, dims) = match &table[symbol_id].content {
                    VSymbol::Parameter(vvalue) => (vvalue.ty(), &[] as &[NonZeroU32]),
                    VSymbol::Net(n) => (n.ty, n.dims.as_slice()),
                    _ => {
                        diagnostics
                            .not_yet_implemented(hident_span(arenas, ident), "not a valid net");
                        error = true;
                        result_stack.push(None);
                        continue;
                    }
                };

                if exprs.len() < dims.len() {
                    diagnostics
                        .not_yet_implemented(arenas.get_span(expr), "unable to get type of array");
                    error = true;
                    result_stack.push(None);
                    continue;
                }

                // Fast path. No slicing at all.
                if exprs.is_empty() && range_expr.is_none() {
                    result_stack.push(Some(ty));
                    continue;
                }

                let ty = match range_expr {
                    Some(range_expr) => match range_expr {
                        BitSlice::MsbLsb(msb, lsb) => {
                            let Ok((_, _, width)) =
                                msb_lsb_to_width(gl, arenas, table, scope, diagnostics, msb, lsb)
                            else {
                                result_stack.push(None);
                                continue;
                            };
                            VType::UnsignedNet(width)
                        }
                        BitSlice::PlusWidth(_, width) | BitSlice::MinusWidth(_, width) => {
                            let Ok(width) = eval_constant_expr(
                                gl,
                                arenas,
                                table,
                                scope,
                                diagnostics,
                                width,
                                None,
                            ) else {
                                result_stack.push(None);
                                continue;
                            };
                            let width = width.coerce(&VType::UnsignedNet(INTEGER_VSIZE));
                            let Some(width) = width.into_bits().extract_exact_u32() else {
                                diagnostics.not_yet_implemented(
                                    arenas.get_span(expr),
                                    "width cannot contain unknown or high-impedance values",
                                );
                                error = true;
                                result_stack.push(None);
                                continue;
                            };
                            let Some(width) = VectorSize::new(width) else {
                                diagnostics.not_yet_implemented(
                                    arenas.get_span(expr),
                                    "width has to be non-zero",
                                );
                                error = true;
                                result_stack.push(None);
                                continue;
                            };
                            VType::UnsignedNet(width)
                        }
                    },
                    None if exprs.len() > dims.len() => VType::SCALAR_NET,
                    None => ty,
                };

                result_stack.push(Some(ty));
            }
            Expr::FunctionCall(ident, _) => {
                let Ok(fn_symbol) = try_resolve_hident(scope, table, arenas, ident, diagnostics)
                else {
                    error = true;
                    result_stack.push(None);
                    continue;
                };
                let VSymbol::Function(fn_symbol) = &table[fn_symbol].content else {
                    diagnostics
                        .not_yet_implemented(hident_span(arenas, ident), "not calling a function");
                    error = true;
                    result_stack.push(None);
                    continue;
                };
                result_stack.push(Some(fn_symbol.output_ty));
            }
            Expr::SystemFunctionCall(ident, exprs) => {
                if !item.dispatched {
                    match system_function_call::lower_unevaluated_system_function_call_ty(
                        arenas,
                        diagnostics,
                        expr,
                        ident,
                        exprs,
                    ) {
                        Ok(Some(res)) => {
                            result_stack.push(Some(res));
                            continue;
                        }
                        Err(()) => {
                            result_stack.push(None);
                            continue;
                        }
                        Ok(None) => {}
                    }

                    item.dispatched = true;
                    dispatch_stack.push(item);
                    if let Some(exprs) = exprs {
                        dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx));
                    }
                    continue;
                }

                let num_args = exprs.map_or(0, |e| e.len());
                let result = get_system_function_call_output_ty(
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
            Expr::Decimal(_) => {
                result_stack.push(Some(VType::SignedNet(INTEGER_VSIZE)));
            }
            Expr::Sized(sized) => {
                let sized = &arenas.sized_numbers[sized.item.at];
                let signed = matches!(sized.sign, Sign::Signed);
                let size = sized.value.size();
                result_stack.push(Some(VType::net(size, signed)));
            }
            Expr::String(string_ref) => {
                let s = arenas.get_ident(string_ref.0);
                let Some(size) = u32::try_from(s.len())
                    .ok()
                    .map(|v| VectorSize::new(v).unwrap_or(SCALAR_VSIZE))
                    .and_then(|v| v.checked_mul(VSIZE_8))
                else {
                    error = true;
                    diagnostics.not_yet_implemented(arenas.get_span(expr), "string size overflow");
                    result_stack.push(None);
                    continue;
                };
                result_stack.push(Some(VType::UnsignedNet(size)));
            }
        }
    }

    if error {
        return Err(());
    }

    assert_eq!(result_stack.len(), 1);
    let Some(ty) = result_stack.pop().unwrap() else {
        panic!();
    };
    Ok(ty)
}
