use std::collections::HashMap;
use std::marker::PhantomData;

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{Bits, GlobalContext, VectorSize};

use crate::ast::AstId;
use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{BinaryOperator, BitSlice, Expr, Replication, UnaryOperator};
use crate::elaborate::{VSymbol, VSymbolTable, VectorTransform};
use crate::lower::addressing::{Address, AddressingContext, RangeExpr, lower_addressing};
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
                    O::LogicalNegation => VValue::scalar_from_bool(!child.to_logical()),
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
                if !item.dispatched && (!exprs.is_empty() || range_expression.is_some()) {
                    item.dispatched = true;

                    dispatch_stack.push(item);
                    dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx));
                    if let Some(range_expression) = range_expression {
                        dispatch_stack.extend(
                            range_expression
                                .exprs()
                                .into_iter()
                                .map(StackItem::new_no_ctx),
                        );
                    }
                    continue;
                }

                let end_length = result_stack.len()
                    - exprs.len()
                    - if range_expression.is_some() { 2 } else { 0 };
                let Ok(value) = try_resolve_constant(scope, table, arenas, ast_ident, diagnostics)
                else {
                    result_stack.truncate(end_length);
                    result_stack.push(None);
                    error = true;
                    continue;
                };

                pub struct ConstantRValueAddressingContext<'a, 'b> {
                    pub arenas: &'b AstArenas,
                    pub result_stack: &'b [Option<VValue>],
                    pub diagnostics: &'b mut Diagnostics,
                    pub loc: usize,
                    pub _pd: PhantomData<&'a ()>,
                }

                impl<'a, 'b> AddressingContext for ConstantRValueAddressingContext<'a, 'b> {
                    type ConstantExpr = usize;
                    type Expr = usize;
                    type Var = i64;
                    type Bool = bool;

                    type Error = ();

                    fn too_many_selects(&mut self) -> Self::Error {
                        let tr = self.arenas.spans[self.loc];
                        self.diagnostics.not_yet_implemented(
                            tr,
                            "cannot select from array or too many selects",
                        );
                    }

                    fn stride_overflow(&mut self) -> Self::Error {
                        let tr = self.arenas.spans[self.loc];
                        self.diagnostics.not_yet_implemented(tr, "stride overflow");
                    }

                    fn not_yet_implemented(&mut self, reason: &'static str) -> Self::Error {
                        let tr = self.arenas.spans[self.loc];
                        self.diagnostics.not_yet_implemented(tr, reason);
                    }

                    fn eval_constant(
                        &mut self,
                        operand: Self::ConstantExpr,
                    ) -> Result<i64, Self::Error> {
                        let value = self.result_stack[operand].clone().ok_or(())?;
                        value.as_integer().ok_or_else(|| {
                            self.diagnostics.not_yet_implemented(
                                self.arenas.spans[self.loc],
                                "cannot convert to index",
                            )
                        })
                    }
                    fn eval_var(&mut self, operand: Self::Expr) -> Result<Self::Var, Self::Error> {
                        let value = self.result_stack[operand].clone().ok_or(())?;
                        value.as_integer().ok_or_else(|| {
                            self.diagnostics.not_yet_implemented(
                                self.arenas.spans[self.loc],
                                "cannot convert to index",
                            )
                        })
                    }

                    fn or_overflow(&mut self, lhs: Self::Bool, rhs: Self::Bool) -> Self::Bool {
                        lhs | rhs
                    }
                    fn var_from_i64(&mut self, v: i64) -> Result<Self::Var, Self::Error> {
                        Ok(v)
                    }
                    fn var_geq_nonzerou32(
                        &mut self,
                        lhs: Self::Var,
                        rhs: std::num::NonZeroU32,
                    ) -> Result<Self::Bool, Self::Error> {
                        Ok(lhs >= i64::from(rhs.get()))
                    }
                    fn var_mul_nonzerou32(
                        &mut self,
                        lhs: Self::Var,
                        rhs: std::num::NonZeroU32,
                    ) -> Result<Self::Var, Self::Error> {
                        lhs.checked_mul(i64::from(rhs.get())).ok_or(())
                    }

                    fn var_add(
                        &mut self,
                        lhs: Self::Var,
                        rhs: Self::Var,
                    ) -> Result<Self::Var, Self::Error> {
                        lhs.checked_add(rhs).ok_or(())
                    }

                    fn var_sub_i64(
                        &mut self,
                        lhs: Self::Var,
                        rhs: i64,
                    ) -> Result<Self::Var, Self::Error> {
                        lhs.checked_sub(rhs).ok_or(())
                    }
                    fn var_revsub_u32(
                        &mut self,
                        lhs: Self::Var,
                        rhs: u32,
                    ) -> Result<Self::Var, Self::Error> {
                        i64::from(rhs).checked_sub(lhs).ok_or(())
                    }
                }

                let exprs = (result_stack.len() - exprs.len()..result_stack.len()).rev();
                let range = range_expression.map(|r| {
                    let fst = end_length + 1;
                    let snd = end_length;
                    match r {
                        BitSlice::MsbLsb(..) => RangeExpr::MsbLsb(fst, snd),
                        BitSlice::PlusWidth(..) => RangeExpr::PlusWidth(fst, snd),
                        BitSlice::MinusWidth(..) => RangeExpr::MinusWidth(fst, snd),
                    }
                });

                let mut actx = ConstantRValueAddressingContext {
                    arenas,
                    result_stack: &result_stack,
                    diagnostics,
                    loc: expr.loc,
                    _pd: PhantomData,
                };

                let result = lower_addressing(
                    &mut actx,
                    value.ty().force_net_width(),
                    &[],
                    VectorTransform::default(),
                    exprs,
                    range,
                );

                let Ok(Address {
                    elem_offset,
                    output_width,
                    array,
                    is_unsigned,
                }) = result
                else {
                    result_stack.truncate(end_length);
                    result_stack.push(None);
                    error = true;
                    continue;
                };

                assert!(array.is_none());
                let is_signed = value.ty().is_signed();
                let value = value.clone().into_bits();
                let offset = match elem_offset {
                    None => 0,
                    Some(elem_offset) => u32::try_from(elem_offset).map_err(|_| {
                        diagnostics
                            .not_yet_implemented(arenas.get_span(expr), "out-of-range offset");
                    })?,
                };

                result_stack.truncate(end_length);
                let value = if is_unsigned {
                    VValue::UnsignedNet(value.slicex(offset, output_width))
                } else {
                    VValue::net(value.slicex(offset, output_width), is_signed)
                };
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

                    let Ok(fn_sid) = try_resolve_hident(scope, table, arenas, ident, diagnostics)
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

                let Ok(fn_sid) = try_resolve_hident(scope, table, arenas, ident, diagnostics)
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
                assert!(!exprs.is_empty());
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
