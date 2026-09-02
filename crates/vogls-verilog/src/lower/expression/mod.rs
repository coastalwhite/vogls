use std::cmp;

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, SignalKey, VSIZE_8,
    VSIZE_64, VariableKey, VectorSize,
};
use vogls_utils::OrderedSet;

use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{BinaryOperator, BitSlice, Expr, Replication, UnaryOperator};
use crate::ast::{AstId, HIdent};
use crate::elaborate::{ArrayDim, VSymbol, VSymbolTable, VectorTransform};
use crate::lower::addressing::{Address, AddressingContext, RangeExpr, lower_addressing};
use crate::lower::{VType, hident_span, try_resolve_hident};
use crate::number::Sign;
use crate::parser::AstArenas;
pub use constant_expr::eval_constant_expr;
pub use ty::get_expr_type;

use super::LowerContext;
use super::{Diagnostics, MutLowerContext};

mod constant_expr;
pub mod function_call;
mod system_function_call;
mod ty;

struct StackItem<'a> {
    expr: AstId<'a, Expr<'a>>,
    /// Context determined width for this expression.
    ///
    /// Verilog has a concept of "self-determined expressions". This determines how an expression
    /// needs to operate on its inputs. For instance, if a shift has a context width higher than
    /// the left side width, it first needs to be extended to the context width.
    context_width: Option<VectorSize>,
    dispatched: bool,
}
impl<'a> StackItem<'a> {
    pub fn new(expr: AstId<'a, Expr<'a>>, context_width: Option<VectorSize>) -> Self {
        Self {
            expr,
            context_width,
            dispatched: false,
        }
    }

    pub fn new_no_ctx(expr: AstId<'a, Expr<'a>>) -> Self {
        Self::new(expr, None)
    }
}

#[deny(clippy::question_mark_used)] // Needs to be handled explicitly in the recursion.
pub fn lower_expr<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    expr: AstId<'a, Expr<'a>>,
    context_width: Option<VectorSize>,
) -> Result<(VariableKey, VType), ()> {
    let mut error = false;
    let mut dispatch_stack: Vec<StackItem<'a>> = Vec::new();
    let mut result_stack: Vec<Option<(VariableKey, VType)>> = Vec::new();

    dispatch_stack.push(StackItem::new(expr, context_width));

    'dispatch_loop: while let Some(mut item) = dispatch_stack.pop() {
        match *item.expr {
            Expr::Unary(op, child) => {
                use UnaryOperator as O;

                if !item.dispatched {
                    item.dispatched = true;

                    let child_context_width =
                        item.context_width.filter(|_| !op.is_self_determined());
                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new(child, child_context_width));
                    continue;
                }

                let child = result_stack.pop().unwrap();

                let Some((mut child, mut ty)) = child else {
                    result_stack.push(None);
                    continue;
                };

                if let Some(context_width) = item.context_width
                    && !ty.is_real()
                    && !op.is_self_determined()
                    && context_width > ty.bit_length()
                {
                    child = zero_or_sign_extend(mctx.gl(), builder, child, ty, context_width);
                    ty = ty.zero_or_sign_extend(context_width);
                }

                let (variable, ty) = match op {
                    O::LogicalNegation if ty.is_real() => {
                        let v = builder.real_to_logical(mctx.gl(), child);
                        let v = builder.logical_neg(mctx.gl(), v);
                        (v, VType::SCALAR_NET)
                    }
                    O::LogicalNegation => {
                        (builder.logical_neg(&mut mctx.gl, child), VType::SCALAR_NET)
                    }
                    O::BitwiseNegation
                    | O::ReductionAnd
                    | O::ReductionOr
                    | O::ReductionNand
                    | O::ReductionNor
                    | O::ReductionXor
                    | O::ReductionXnor
                        if ty.is_real() =>
                    {
                        error = true;
                        mctx.diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(item.expr),
                            "operation does not support reals",
                        );
                        result_stack.push(None);
                        continue;
                    }
                    O::BitwiseNegation => (builder.binary_not(&mut mctx.gl, child), ty),
                    O::ReductionAnd => (builder.reduce_and(&mut mctx.gl, child), VType::SCALAR_NET),
                    O::ReductionOr => (builder.reduce_or(&mut mctx.gl, child), VType::SCALAR_NET),
                    O::ReductionNand => {
                        (builder.reduce_nand(&mut mctx.gl, child), VType::SCALAR_NET)
                    }
                    O::ReductionNor => (builder.reduce_nor(&mut mctx.gl, child), VType::SCALAR_NET),
                    O::ReductionXor => (builder.reduce_xor(&mut mctx.gl, child), VType::SCALAR_NET),
                    O::ReductionXnor => {
                        (builder.reduce_xnor(&mut mctx.gl, child), VType::SCALAR_NET)
                    }

                    O::SignPlus => (child, ty),
                    O::SignMinus if ty.is_real() => {
                        (builder.real_neg(mctx.gl(), child), VType::Real)
                    }
                    O::SignMinus => (
                        builder.revminus_constant(
                            mctx.gl(),
                            child,
                            Bits::new_zeroed(ty.bit_length()),
                        ),
                        ty,
                    ),
                };
                result_stack.push(Some((variable, ty)));
            }
            Expr::Binary(op, l, r) => {
                use BinaryOperator as O;
                if !item.dispatched {
                    item.dispatched = true;

                    let (Ok(l_ty), Ok(r_ty)) = (
                        get_expr_type(
                            &mctx.gl,
                            ctx.arenas,
                            &ctx.table,
                            scope,
                            &mut mctx.diagnostics,
                            l,
                        ),
                        get_expr_type(
                            &mctx.gl,
                            ctx.arenas,
                            &ctx.table,
                            scope,
                            &mut mctx.diagnostics,
                            r,
                        ),
                    ) else {
                        result_stack.push(None);
                        error = true;
                        continue;
                    };

                    let mut child_context_width =
                        op.context_width(l_ty.bit_length(), r_ty.bit_length());
                    let (l_is_self_det, r_is_self_det) = op.is_self_determined();
                    if let Some(context_width) = item.context_width {
                        child_context_width = child_context_width.max(context_width);
                    }

                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new(
                        r,
                        (!r_is_self_det & !r_ty.is_real()).then_some(child_context_width),
                    ));
                    dispatch_stack.push(StackItem::new(
                        l,
                        (!l_is_self_det & !l_ty.is_real()).then_some(child_context_width),
                    ));
                    continue;
                }

                let r = result_stack.pop().unwrap();
                let l = result_stack.pop().unwrap();

                let (Some((mut l, mut l_ty)), Some((mut r, mut r_ty))) = (l, r) else {
                    result_stack.push(None);
                    continue;
                };

                let (l_is_self_det, r_is_self_det) = op.is_self_determined();
                if let Some(context_width) = item.context_width {
                    if !l_is_self_det && !l_ty.is_real() && context_width > l_ty.bit_length() {
                        l = zero_or_sign_extend(mctx.gl(), builder, l, l_ty, context_width);
                        l_ty = l_ty.zero_or_sign_extend(context_width);
                    }
                    if !r_is_self_det && !r_ty.is_real() && context_width > r_ty.bit_length() {
                        r = zero_or_sign_extend(mctx.gl(), builder, r, r_ty, context_width);
                        r_ty = r_ty.zero_or_sign_extend(context_width);
                    }
                }

                macro_rules! arithmetic_op {
                    ($integer:ident, $real:ident) => {{
                        let (l, l_ty, r, _) =
                            coerce_bin_arithmetic(mctx.gl(), builder, l, l_ty, r, r_ty);
                        if l_ty.is_real() {
                            (builder.$real(mctx.gl(), l, r), VType::Real)
                        } else {
                            (builder.$integer(mctx.gl(), l, r), l_ty)
                        }
                    }};
                }
                macro_rules! comparison_op {
                    ($signed:ident, $unsigned:ident, $real:ident) => {{
                        let (l, l_ty, r, r_ty) =
                            coerce_bin_arithmetic(mctx.gl(), builder, l, l_ty, r, r_ty);
                        if l_ty.is_real() {
                            (builder.$real(mctx.gl(), l, r), VType::SCALAR_NET)
                        } else if l_ty.is_signed() || r_ty.is_signed() {
                            (builder.$signed(mctx.gl(), l, r), VType::SCALAR_NET)
                        } else {
                            (builder.$unsigned(mctx.gl(), l, r), VType::SCALAR_NET)
                        }
                    }};
                }
                macro_rules! logical_op {
                    ($integer:ident, $real:ident) => {{
                        let (l, l_ty, r, _) =
                            coerce_bin_arithmetic(mctx.gl(), builder, l, l_ty, r, r_ty);
                        if l_ty.is_real() {
                            (builder.$real(mctx.gl(), l, r), VType::SCALAR_NET)
                        } else {
                            (builder.$integer(mctx.gl(), l, r), VType::SCALAR_NET)
                        }
                    }};
                }
                macro_rules! case_equality {
                    ($f:ident) => {{
                        let (l, l_ty, r, _) =
                            coerce_bin_arithmetic(mctx.gl(), builder, l, l_ty, r, r_ty);
                        if l_ty.is_real() {
                            error = true;
                            mctx.diagnostics.not_yet_implemented(
                                ctx.arenas.get_span(item.expr),
                                "operand does not support reals",
                            );
                            result_stack.push(None);
                            continue;
                        } else {
                            (builder.$f(mctx.gl(), l, r), VType::SCALAR_NET)
                        }
                    }};
                }
                macro_rules! bitwise_op {
                    ($f:ident) => {{
                        let (l, l_ty, r, _) =
                            coerce_bin_arithmetic(mctx.gl(), builder, l, l_ty, r, r_ty);
                        if l_ty.is_real() {
                            error = true;
                            mctx.diagnostics.not_yet_implemented(
                                ctx.arenas.get_span(item.expr),
                                "operand does not support reals",
                            );
                            result_stack.push(None);
                            continue;
                        } else {
                            (builder.$f(mctx.gl(), l, r), l_ty)
                        }
                    }};
                }
                macro_rules! shift_op {
                    ($f:ident) => {{
                        if l_ty.is_real() | r_ty.is_real() {
                            error = true;
                            mctx.diagnostics.not_yet_implemented(
                                ctx.arenas.get_span(item.expr),
                                "operand does not support reals",
                            );
                            result_stack.push(None);
                            continue;
                        }

                        // From LRM: 5.1.12 Shift operators
                        // > The right operand is always treated as an unsigned number and has no effect on the
                        // > signedness of the result.
                        let r_ty = r_ty.to_unsigned();

                        let r = sign_or_zero_extend(mctx.gl(), builder, r, r_ty, INTEGER_VSIZE);
                        (builder.$f(mctx.gl(), l, r), l_ty)
                    }};
                }

                let result = match op {
                    O::Power => {
                        let l_width = l_ty.bit_length();
                        let (l, l_ty, r, _) =
                            coerce_bin_arithmetic(mctx.gl(), builder, l, l_ty, r, r_ty);
                        if l_ty.is_real() {
                            (builder.real_pow(mctx.gl(), l, r), VType::Real)
                        } else {
                            let v = builder.power(mctx.gl(), l, r);
                            let l_ty = l_ty.truncate(l_width);
                            (builder.truncate(mctx.gl(), v, l_width), l_ty)
                        }
                    }
                    O::Multiply => arithmetic_op!(multiply, real_mul),
                    O::BinaryPlus => arithmetic_op!(plus, real_add),
                    O::BinaryMinus => arithmetic_op!(minus, real_sub),
                    O::Divide => {
                        let (l, l_ty, r, r_ty) =
                            coerce_bin_arithmetic(mctx.gl(), builder, l, l_ty, r, r_ty);
                        if l_ty.is_real() {
                            (builder.real_div(mctx.gl(), l, r), VType::Real)
                        } else if l_ty.is_signed() || r_ty.is_signed() {
                            (builder.signed_divide(mctx.gl(), l, r), l_ty)
                        } else {
                            (builder.divide(mctx.gl(), l, r), l_ty)
                        }
                    }
                    O::Modulus => {
                        let (l, l_ty, r, r_ty) =
                            coerce_bin_arithmetic(mctx.gl(), builder, l, l_ty, r, r_ty);
                        if l_ty.is_real() {
                            error = true;
                            mctx.diagnostics.not_yet_implemented(
                                ctx.arenas.get_span(item.expr),
                                "operand does not support reals",
                            );
                            result_stack.push(None);
                            continue;
                        } else if l_ty.is_signed() || r_ty.is_signed() {
                            (builder.signed_modulus(mctx.gl(), l, r), l_ty)
                        } else {
                            (builder.modulus(mctx.gl(), l, r), l_ty)
                        }
                    }
                    O::ShiftLeft => shift_op!(logical_shift_left),
                    O::ShiftRight => shift_op!(logical_shift_right),
                    O::GreaterThan => comparison_op!(signed_gt, unsigned_gt, real_gt),
                    O::GreaterThanEqual => comparison_op!(signed_ge, unsigned_ge, real_geq),
                    O::LessThan => comparison_op!(signed_lt, unsigned_lt, real_lt),
                    O::LessThanEqual => comparison_op!(signed_le, unsigned_le, real_leq),
                    O::ArithmeticLeftShift => shift_op!(logical_shift_left),
                    O::ArithmeticRightShift => {
                        if l_ty.is_real() | r_ty.is_real() {
                            error = true;
                            mctx.diagnostics.not_yet_implemented(
                                ctx.arenas.get_span(item.expr),
                                "operand does not support reals",
                            );
                            result_stack.push(None);
                            continue;
                        }

                        // From LRM: 5.1.12 Shift operators
                        // > The right operand is always treated as an unsigned number and has no effect on the
                        // > signedness of the result.
                        let r_ty = r_ty.to_unsigned();
                        let r = sign_or_zero_extend(mctx.gl(), builder, r, r_ty, INTEGER_VSIZE);
                        if l_ty.is_signed() {
                            (builder.arithmetic_shift_right(mctx.gl(), l, r), l_ty)
                        } else {
                            (builder.logical_shift_right(mctx.gl(), l, r), l_ty)
                        }
                    }
                    O::LogicalEquality => logical_op!(equals, real_eq),
                    O::LogicalInequality => logical_op!(not_equals, real_ne),
                    O::CaseEquality => case_equality!(case_equals),
                    O::CaseInequality => case_equality!(not_case_equals),
                    O::BitwiseAnd => bitwise_op!(and),
                    O::BitwiseXor => bitwise_op!(xor),
                    O::BitwiseXnor => bitwise_op!(xnor),
                    O::BitwiseOr => bitwise_op!(or),
                    O::LogicalAnd => logical_op!(logical_and, real_logical_and),
                    O::LogicalOr => logical_op!(logical_or, real_logical_or),
                };
                result_stack.push(Some(result));
            }
            Expr::Concatenation(exprs) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend(
                        exprs
                            .iter()
                            .rev()
                            .filter(|e| {
                                !is_zero_sized_replication(
                                    &mctx.gl, ctx.arenas, &ctx.table, scope, e,
                                )
                            })
                            .map(StackItem::new_no_ctx),
                    );
                    continue;
                }

                // @NOTE: Zero-sized replications are allowed in concatenations per 5.1.14.
                //
                // > A replication operation may have a replication constant with a value of zero.
                // > This is useful in parameterized code. A replication with a zero replication
                // > constant is considered to have a size of zero and is ignored. Such a
                // > replication shall appear only within a concatenation in which at least one of
                // > the operands of the concatenation has a positive size.
                let num_exprs = exprs
                    .iter()
                    .filter(|e| {
                        !is_zero_sized_replication(&mctx.gl, ctx.arenas, &ctx.table, scope, e)
                    })
                    .count();
                if num_exprs == 0 {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item.expr),
                        "concatenation without expressions",
                    );
                    error = true;
                    result_stack.push(None);
                    continue;
                }

                let mut has_real = false;
                let end_stack_size = result_stack.len() - num_exprs;
                let Some((mut output, ty)) = result_stack.pop().unwrap() else {
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                };
                has_real |= ty.is_real();
                let mut width = ty.bit_length().get();
                for _ in 1..num_exprs {
                    let Some((next, next_ty)) = result_stack.pop().unwrap() else {
                        result_stack.truncate(end_stack_size);
                        result_stack.push(None);
                        continue;
                    };
                    has_real |= next_ty.is_real();
                    let next_width = next_ty.bit_length();
                    output = builder.concat(&mut mctx.gl, next, output);
                    width += next_width.get();
                }

                if has_real {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item.expr),
                        "real is not allowed in concatenation.",
                    );
                    error = true;
                    result_stack.push(None);
                    continue;
                }

                result_stack.push(Some((
                    output,
                    VType::UnsignedNet(VectorSize::new(width).unwrap()),
                )));
            }
            Expr::Replication(replication) => {
                let Replication {
                    constant_expr,
                    exprs,
                } = replication;

                if exprs.is_empty() {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item.expr),
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
                let Ok(repeat_n) = eval_constant_expr(
                    &mctx.gl,
                    ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    constant_expr,
                    None,
                ) else {
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                };

                let Some(repeat_n) = repeat_n.as_integer() else {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(constant_expr),
                        "replication overflow",
                    );
                    error = true;
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                };
                if repeat_n == 0 {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(constant_expr),
                        "replication is 0",
                    );
                    error = true;
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                }
                let Some((mut output, ty)) = result_stack.pop().unwrap() else {
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                };

                if ty.is_real() {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item.expr),
                        "real is not allowed in replication",
                    );
                    error = true;
                    result_stack.push(None);
                    continue;
                }

                let mut width = ty.bit_length().get();
                for _ in 1..exprs.len() {
                    let Some((next, next_ty)) = result_stack.pop().unwrap() else {
                        result_stack.truncate(end_stack_size);
                        result_stack.push(None);
                        continue;
                    };
                    let next_width = next_ty.bit_length();
                    output = builder.concat(mctx.gl(), next, output);
                    width += next_width.get();
                }

                let Some(output_width) = width.checked_mul(repeat_n as u32) else {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item.expr),
                        "replication overflow",
                    );
                    error = true;
                    result_stack.push(None);
                    continue;
                };

                let output_single = output;
                for _ in 1..repeat_n {
                    output = builder.concat(mctx.gl(), output_single, output);
                }
                result_stack.push(Some((
                    output,
                    VType::UnsignedNet(VectorSize::new(output_width).unwrap()),
                )));
            }
            Expr::Ternary(condition, truthy, falsy) => {
                if !item.dispatched {
                    item.dispatched = true;

                    let (Ok(l_ty), Ok(r_ty)) = (
                        get_expr_type(
                            &mctx.gl,
                            ctx.arenas,
                            &ctx.table,
                            scope,
                            &mut mctx.diagnostics,
                            truthy,
                        ),
                        get_expr_type(
                            &mctx.gl,
                            ctx.arenas,
                            &ctx.table,
                            scope,
                            &mut mctx.diagnostics,
                            falsy,
                        ),
                    ) else {
                        result_stack.push(None);
                        error = true;
                        continue;
                    };

                    let mut child_context_width =
                        VectorSize::max(l_ty.bit_length(), r_ty.bit_length());
                    if let Some(context_width) = item.context_width {
                        child_context_width = child_context_width.max(context_width);
                    }

                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new_no_ctx(condition));
                    dispatch_stack.push(StackItem::new(truthy, Some(child_context_width)));
                    dispatch_stack.push(StackItem::new(falsy, Some(child_context_width)));
                    continue;
                }

                // @TODO: This is not 100% semantically correct. It should mix truthy and falsy
                // when the condition is `x` or `z`.

                let condition = result_stack.pop().unwrap();
                let truthy = result_stack.pop().unwrap();
                let falsy = result_stack.pop().unwrap();

                let (Some((c, c_ty)), Some((mut t, mut t_ty)), Some((mut f, mut f_ty))) =
                    (condition, truthy, falsy)
                else {
                    result_stack.push(None);
                    continue;
                };

                if t_ty.is_real() || f_ty.is_real() {
                    t = to_real(mctx.gl(), builder, t, t_ty);
                    f = to_real(mctx.gl(), builder, f, f_ty);
                    t_ty = VType::Real;
                } else {
                    let size = t_ty.bit_length().max(f_ty.bit_length());
                    if size > t_ty.bit_length() {
                        t = zero_or_sign_extend(mctx.gl(), builder, t, t_ty, size);
                        t_ty = t_ty.zero_or_sign_extend(size);
                    }
                    if size > f_ty.bit_length() {
                        f = zero_or_sign_extend(mctx.gl(), builder, f, f_ty, size);
                        f_ty = f_ty.zero_or_sign_extend(size);
                    }

                    if let Some(context_width) = item.context_width {
                        if context_width > t_ty.bit_length() {
                            t = zero_or_sign_extend(mctx.gl(), builder, t, t_ty, context_width);
                            t_ty = t_ty.zero_or_sign_extend(context_width);
                        }
                        if context_width > f_ty.bit_length() {
                            f = zero_or_sign_extend(mctx.gl(), builder, f, f_ty, context_width);
                        }
                    }
                }

                let c = to_logical(mctx.gl(), builder, c, c_ty);
                let result = builder.select_merge(mctx.gl(), c, t, f);
                result_stack.push(Some((result, t_ty)));
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                if (!exprs.is_empty()
                    || range_expression.is_some_and(|r| {
                        matches!(r, BitSlice::PlusWidth(..) | BitSlice::MinusWidth(..))
                    }))
                    && !item.dispatched
                {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    match range_expression {
                        None => {}
                        Some(BitSlice::MsbLsb(..)) => {}
                        Some(BitSlice::PlusWidth(base, _) | BitSlice::MinusWidth(base, _)) => {
                            dispatch_stack.push(StackItem::new_no_ctx(base))
                        }
                    }
                    dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx));
                    continue;
                }

                let end_result_stack_len = result_stack.len()
                    - exprs.len()
                    - usize::from(matches!(
                        range_expression,
                        Some(BitSlice::PlusWidth(..) | BitSlice::MinusWidth(..))
                    ));
                let Ok(symbol_key) = try_resolve_hident(
                    scope,
                    &ctx.table,
                    ctx.arenas,
                    ast_ident,
                    &mut mctx.diagnostics,
                ) else {
                    error = true;
                    result_stack.truncate(end_result_stack_len);
                    result_stack.push(None);
                    continue;
                };
                let symbol = &ctx.table[symbol_key].content;
                let (ty, dims, transform, var) = match &symbol {
                    VSymbol::Parameter(value) => {
                        let value = value.clone();
                        (
                            value.ty(),
                            &[] as &[ArrayDim],
                            VectorTransform::default(),
                            builder.constant(mctx.gl(), value.into_bits()),
                        )
                    }
                    VSymbol::Net(s) => (
                        s.ty,
                        &s.dims[..],
                        s.transform,
                        s.net.probe(mctx.gl(), builder),
                    ),

                    VSymbol::Task(_)
                    | VSymbol::GenVar
                    | VSymbol::Function(_)
                    | VSymbol::Module(_)
                    | VSymbol::NamedBlock
                    | VSymbol::GenerateBlock(_)
                    | VSymbol::GenerateBlocks
                    | VSymbol::ModuleRange(_) => {
                        mctx.diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(expr),
                            "cannot use this symbol",
                        );
                        error = true;
                        result_stack.truncate(end_result_stack_len);
                        result_stack.push(None);
                        continue 'dispatch_loop;
                    }
                };

                if ty.is_real() && (exprs.len() > dims.len() || range_expression.is_some()) {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item.expr),
                        "bit and part selects are not allowed on reals",
                    );
                    result_stack.push(None);
                    error = true;
                    continue;
                }

                struct LowerExprPartSelect<'a, 'b> {
                    builder: &'b mut BasicBlockBuilder,
                    result_stack: &'b [Option<(VariableKey, VType)>],
                    expr: AstId<'a, Expr<'a>>,
                    scope: SymbolId,
                    ctx: &'b LowerContext<'a, 'b>,
                    mctx: &'b mut MutLowerContext,
                }

                #[allow(clippy::question_mark_used)]
                impl<'a, 'b> AddressingContext for LowerExprPartSelect<'a, 'b> {
                    type ConstantExpr = AstId<'a, ConstantExpr<'a>>;
                    type Expr = usize;
                    type Var = VariableKey;
                    type Bool = VariableKey;

                    type Error = ();

                    fn too_many_selects(&mut self) -> Self::Error {
                        let tr = self.ctx.arenas.get_span(self.expr);
                        self.mctx.diagnostics.not_yet_implemented(
                            tr,
                            "cannot select from array or too many selects",
                        );
                    }

                    fn stride_overflow(&mut self) -> Self::Error {
                        let tr = self.ctx.arenas.get_span(self.expr);
                        self.mctx
                            .diagnostics
                            .not_yet_implemented(tr, "stride overflow");
                    }

                    fn not_yet_implemented(&mut self, reason: &'static str) -> Self::Error {
                        let tr = self.ctx.arenas.get_span(self.expr);
                        self.mctx.diagnostics.not_yet_implemented(tr, reason);
                    }

                    fn eval_constant(
                        &mut self,
                        operand: Self::ConstantExpr,
                    ) -> Result<i64, Self::Error> {
                        let result = eval_constant_expr(
                            &self.mctx.gl,
                            self.ctx.arenas,
                            &self.ctx.table,
                            self.scope,
                            &mut self.mctx.diagnostics,
                            operand,
                            None,
                        )?;
                        let Some(result) = result.as_integer() else {
                            let tr = self.ctx.arenas.get_span(operand);
                            self.mctx
                                .diagnostics
                                .not_yet_implemented(tr, "unable to use as operand");
                            return Err(());
                        };
                        Ok(result)
                    }
                    fn eval_var(&mut self, operand: Self::Expr) -> Result<Self::Var, Self::Error> {
                        let (var, ty) = self.result_stack[operand].unwrap();
                        let var = truncate_or_extend(
                            self.mctx.gl(),
                            self.builder,
                            var,
                            ty,
                            INTEGER_VSIZE,
                        );
                        Ok(var)
                    }

                    fn or_overflow(&mut self, lhs: Self::Bool, rhs: Self::Bool) -> Self::Bool {
                        self.builder.or(self.mctx.gl(), lhs, rhs)
                    }

                    fn var_from_i64(&mut self, v: i64) -> Result<Self::Var, Self::Error> {
                        Ok(self.builder.constant(
                            self.mctx.gl(),
                            Bits::new_u64(v as u64).truncate(INTEGER_VSIZE),
                        ))
                    }

                    fn var_geq_nonzerou32(
                        &mut self,
                        lhs: Self::Var,
                        rhs: std::num::NonZeroU32,
                    ) -> Result<Self::Bool, Self::Error> {
                        let rhs = self
                            .builder
                            .constant(self.mctx.gl(), Bits::new_u32(rhs.get()));
                        Ok(self.builder.unsigned_ge(self.mctx.gl(), lhs, rhs))
                    }

                    fn var_mul_nonzerou32(
                        &mut self,
                        lhs: Self::Var,
                        rhs: std::num::NonZeroU32,
                    ) -> Result<Self::Var, Self::Error> {
                        Ok(self.builder.multiply_constant(
                            self.mctx.gl(),
                            lhs,
                            Bits::new_u32(rhs.get()),
                        ))
                    }

                    fn var_add(
                        &mut self,
                        lhs: Self::Var,
                        rhs: Self::Var,
                    ) -> Result<Self::Var, Self::Error> {
                        Ok(self.builder.plus(self.mctx.gl(), lhs, rhs))
                    }

                    fn var_sub_i64(
                        &mut self,
                        lhs: Self::Var,
                        rhs: i64,
                    ) -> Result<Self::Var, Self::Error> {
                        Ok(self.builder.minus_constant(
                            self.mctx.gl(),
                            lhs,
                            Bits::new_u64(rhs as u64).truncate(INTEGER_VSIZE),
                        ))
                    }
                    fn var_revsub_u32(
                        &mut self,
                        lhs: Self::Var,
                        rhs: u32,
                    ) -> Result<Self::Var, Self::Error> {
                        Ok(self
                            .builder
                            .revminus_constant(self.mctx.gl(), lhs, Bits::new_u32(rhs)))
                    }
                }

                let range_base = end_result_stack_len + exprs.len();
                let range_expr = match range_expression {
                    None => None,
                    Some(BitSlice::MsbLsb(msb, lsb)) => Some(RangeExpr::MsbLsb(msb, lsb)),
                    Some(BitSlice::PlusWidth(_, width)) => {
                        Some(RangeExpr::PlusWidth(range_base, width))
                    }
                    Some(BitSlice::MinusWidth(_, width)) => {
                        Some(RangeExpr::MinusWidth(range_base, width))
                    }
                };

                let result = lower_addressing::<LowerExprPartSelect>(
                    &mut LowerExprPartSelect {
                        builder,
                        result_stack: &result_stack,
                        expr,
                        scope,
                        ctx,
                        mctx,
                    },
                    ty.bit_length(),
                    dims,
                    transform,
                    (end_result_stack_len..end_result_stack_len + exprs.len()).rev(),
                    range_expr,
                );

                let Ok(part_select) = result else {
                    result_stack.truncate(end_result_stack_len);
                    result_stack.push(None);
                    error = true;
                    continue;
                };

                let Address {
                    elem_offset,
                    output_width,
                    array,
                    is_unsigned,
                } = part_select;
                result_stack.truncate(end_result_stack_len);

                // @TODO:
                // 1. We should use the overflow value here.
                // 2. We should limit our write to `ty.force_net_width()` bits as in
                //    arrays, this will inherently be wrong.

                let var = if let Some(elem_offset) = elem_offset {
                    let offset = match array {
                        None => elem_offset,
                        Some((array_offset, _array_overflow)) => {
                            builder.plus(mctx.gl(), array_offset, elem_offset)
                        }
                    };
                    builder.slice(&mut mctx.gl, var, offset, output_width)
                } else if let Some((array_offset, _array_overflow)) = array {
                    builder.slice(&mut mctx.gl, var, array_offset, output_width)
                } else {
                    builder.truncate(&mut mctx.gl, var, output_width)
                };

                let output_ty = if ty.is_real() {
                    VType::Real
                } else if is_unsigned {
                    VType::UnsignedNet(output_width)
                } else {
                    ty.truncate(output_width)
                };
                result_stack.push(Some((var, output_ty)));
            }
            Expr::FunctionCall(ident, exprs) => {
                if !item.dispatched {
                    item.dispatched = true;
                    let Ok(fn_symbol) = try_resolve_hident(
                        scope,
                        &ctx.table,
                        ctx.arenas,
                        ident,
                        &mut mctx.diagnostics,
                    ) else {
                        error = true;
                        result_stack.push(None);
                        continue;
                    };
                    let VSymbol::Function(fn_symbol) = &ctx.table[fn_symbol].content else {
                        mctx.diagnostics.not_yet_implemented(
                            hident_span(ctx.arenas, ident),
                            "not calling a function",
                        );
                        error = true;
                        result_stack.push(None);
                        continue;
                    };

                    dispatch_stack.push(item);
                    dispatch_stack.extend(
                        exprs
                            .iter()
                            .zip(&fn_symbol.inputs)
                            .map(|(e, (_, ty))| StackItem::new(e, Some(ty.bit_length()))),
                    );
                    continue;
                }

                let num_args = exprs.len();
                let result = function_call::lower_function_call(
                    ctx,
                    mctx,
                    scope,
                    builder,
                    expr,
                    ident,
                    &result_stack[result_stack.len() - num_args..],
                );

                result_stack.truncate(result_stack.len() - num_args);
                match result {
                    Ok((v, t)) => result_stack.push(Some((v, t))),
                    Err(_) => {
                        result_stack.push(None);
                        error = true;
                    }
                }
            }
            Expr::SystemFunctionCall(ident, exprs) => {
                if !item.dispatched {
                    match system_function_call::lower_unevaluated_system_function_call(
                        ctx, mctx, scope, builder, ident, exprs,
                    ) {
                        Ok(Some(res)) => {
                            result_stack.push(Some(res));
                            continue;
                        }
                        Err(()) => {
                            error = true;
                            result_stack.push(None);
                            continue;
                        }
                        Ok(None) => {}
                    }

                    item.dispatched = true;
                    dispatch_stack.push(item);
                    if let Some(exprs) = exprs {
                        // @TODO: We would maybe pass some context here.
                        dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx));
                    }
                    continue;
                }

                let num_args = exprs.map_or(0, |e| e.len());
                let result = system_function_call::lower_system_function_call(
                    ctx,
                    mctx,
                    builder,
                    expr,
                    ident,
                    &result_stack[result_stack.len() - num_args..],
                );

                result_stack.truncate(result_stack.len() - num_args);
                match result {
                    Ok((v, t)) => result_stack.push(Some((v, t))),
                    Err(_) => {
                        result_stack.push(None);
                        error = true;
                    }
                }
            }
            Expr::Real(v) => {
                result_stack.push(Some((
                    builder.constant_u64(&mut mctx.gl, v.to_bits()),
                    VType::Real,
                )));
            }
            Expr::Decimal(decimal) => {
                let decimal = &ctx.arenas.decimals[decimal.at];
                result_stack.push(Some((
                    builder.constant(&mut mctx.gl, decimal.clone()),
                    VType::SignedNet(INTEGER_VSIZE),
                )));
            }
            Expr::Sized(sized) => {
                let sized = &ctx.arenas.sized_numbers[sized.item.at];
                let signed = matches!(sized.sign, Sign::Signed);
                let size = sized.value.size();
                let var = builder.constant(&mut mctx.gl, sized.value.clone());
                result_stack.push(Some((var, VType::net(size, signed))));
            }
            Expr::String(string_ref) => {
                let s = ctx.arenas.get_ident(string_ref.0);
                let Ok(num_chars) = u32::try_from(s.len()) else {
                    error = true;
                    mctx.diagnostics
                        .not_yet_implemented(ctx.arenas.get_span(expr), "string size overflow");
                    result_stack.push(None);
                    continue;
                };
                let Some(size) = VectorSize::new(num_chars)
                    .unwrap_or(SCALAR_VSIZE)
                    .checked_mul(VSIZE_8)
                else {
                    error = true;
                    mctx.diagnostics
                        .not_yet_implemented(ctx.arenas.get_span(expr), "string size overflow");
                    result_stack.push(None);
                    continue;
                };

                let value = if num_chars == 0 {
                    Bits::new_zeroed(VSIZE_8)
                } else {
                    let s = s.bytes().rev().collect::<Box<[u8]>>();
                    Bits::load_from_slice(&s, size)
                };
                let var = builder.constant(&mut mctx.gl, value);
                result_stack.push(Some((var, VType::UnsignedNet(size))));
            }
        }
    }

    if error {
        return Err(());
    }

    assert_eq!(result_stack.len(), 1);
    let Some((value, ty)) = result_stack.pop().unwrap() else {
        panic!();
    };
    Ok((value, ty))
}

fn to_logical(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    var: VariableKey,
    ty: VType,
) -> VariableKey {
    match ty {
        VType::SignedNet(_) | VType::UnsignedNet(_) => builder.reduce_or(gl, var),
        VType::Real => builder.real_to_logical(gl, var),
    }
}

// i op j, where op is: + - * / % & | ^ ^~ ~^
pub fn coerce_bin_arithmetic(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    l: VariableKey,
    l_ty: VType,
    r: VariableKey,
    r_ty: VType,
) -> (VariableKey, VType, VariableKey, VType) {
    // max(L(i),L(j))

    if l_ty == r_ty {
        return (l, l_ty, r, r_ty);
    }

    if l_ty.is_real() || r_ty.is_real() {
        let l = to_real(gl, builder, l, l_ty);
        let r = to_real(gl, builder, r, r_ty);
        return (l, VType::Real, r, VType::Real);
    }

    let ty = coerce_to_max_size_ty(l_ty, r_ty);

    let l = sign_or_zero_extend(gl, builder, l, l_ty, ty.bit_length());
    let r = sign_or_zero_extend(gl, builder, r, r_ty, ty.bit_length());

    (l, ty, r, ty)
}

pub fn coerce_to_max_size_ty(l_ty: VType, r_ty: VType) -> VType {
    // max(L(i),L(j))

    if l_ty == r_ty {
        return l_ty;
    }

    let l_size = l_ty.bit_length();
    let r_size = r_ty.bit_length();

    let size = cmp::max(l_size, r_size);

    VType::net(size, l_ty.is_signed() & r_ty.is_signed())
}

pub fn to_real(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    ty: VType,
) -> VariableKey {
    match ty {
        VType::SignedNet(_) => builder.real_from_signed_decimal(gl, src),
        VType::UnsignedNet(_) => builder.real_from_unsigned_decimal(gl, src),
        VType::Real => src,
    }
}

pub fn sign_or_zero_extend(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    from: VType,
    to: VectorSize,
) -> VariableKey {
    let from_width = from.bit_length();
    if from_width == to {
        src
    } else if from_width > to {
        builder.truncate(gl, src, to)
    } else if from.is_signed() {
        builder.sign_extend(gl, src, to)
    } else {
        builder.zero_extend(gl, src, to)
    }
}

pub fn truncate_or_extend(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    from: VType,
    to: VectorSize,
) -> VariableKey {
    let from_width = from.bit_length();
    if from_width == to {
        src
    } else if from_width > to {
        builder.truncate(gl, src, to)
    } else if from.is_signed() {
        builder.sign_extend(gl, src, to)
    } else {
        builder.zero_extend(gl, src, to)
    }
}

pub fn coerce_to(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    from: VType,
    to: VType,
) -> VariableKey {
    use VType as T;
    match (from, to) {
        (T::SignedNet(_) | T::UnsignedNet(_), T::SignedNet(_) | T::UnsignedNet(_)) => {
            truncate_or_extend(gl, builder, src, from, to.bit_length())
        }
        (T::Real, T::SignedNet(to)) => {
            let v = builder.real_to_i64(gl, src);
            truncate_or_extend(gl, builder, v, T::SignedNet(VSIZE_64), to)
        }
        (T::Real, T::UnsignedNet(to)) => {
            let v = builder.real_to_u64(gl, src);
            truncate_or_extend(gl, builder, v, T::UnsignedNet(VSIZE_64), to)
        }
        (_, T::Real) => to_real(gl, builder, src, from),
    }
}

pub fn zero_or_sign_extend(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    from: VType,
    to: VectorSize,
) -> VariableKey {
    let from_width = from.bit_length();
    assert!(from.bit_length() <= to);
    if from_width == to {
        src
    } else if from.is_signed() {
        builder.sign_extend(gl, src, to)
    } else {
        builder.zero_extend(gl, src, to)
    }
}

pub fn get_used_signals<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    signals: &mut OrderedSet<SignalKey>,
    expr: AstId<'a, Expr<'a>>,
) -> Result<(), ()> {
    let mut error = false;
    let mut dispatch_stack: Vec<StackItem<'a>> = Vec::new();

    dispatch_stack.push(StackItem::new_no_ctx(expr));

    while let Some(item) = dispatch_stack.pop() {
        match &*item.expr {
            Expr::Unary(_, c) => dispatch_stack.push(StackItem::new_no_ctx(*c)),
            Expr::Binary(_, l, r) => {
                dispatch_stack.extend([*l, *r].into_iter().map(StackItem::new_no_ctx))
            }
            Expr::Concatenation(exprs)
            | Expr::Replication(Replication {
                constant_expr: _,
                exprs,
            })
            | Expr::FunctionCall(_, exprs) => {
                dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx))
            }
            Expr::SystemFunctionCall(_, exprs) => {
                if let Some(exprs) = exprs {
                    dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx))
                }
            }
            Expr::Ternary(c, t, f) => {
                dispatch_stack.extend([*c, *t, *f].into_iter().map(StackItem::new_no_ctx))
            }
            Expr::Ident(ident, exprs, range_expression) => {
                if get_used_ident_signals(ctx, mctx, scope, signals, *ident).is_err() {
                    error = true;
                    continue;
                }

                dispatch_stack.extend(exprs.iter().map(StackItem::new_no_ctx));
                if let Some(range_expression) = range_expression {
                    match range_expression {
                        BitSlice::MsbLsb(_, _) => {}
                        BitSlice::PlusWidth(base, _) | BitSlice::MinusWidth(base, _) => {
                            dispatch_stack.push(StackItem::new_no_ctx(*base))
                        }
                    }
                }
            }
            Expr::Real(_) | Expr::Decimal(_) | Expr::Sized(_) | Expr::String(_) => {}
        }
    }

    if error {
        return Err(());
    }

    Ok(())
}

pub fn get_used_ident_signals<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    signals: &mut OrderedSet<SignalKey>,
    ident: impl Into<HIdent<'a>>,
) -> Result<(), ()> {
    let Ok(symbol_key) =
        try_resolve_hident(scope, &ctx.table, ctx.arenas, ident, &mut mctx.diagnostics)
    else {
        return Err(());
    };
    match &ctx.table[symbol_key].content {
        VSymbol::Net(s) => {
            _ = signals.insert(s.net.probe_signal());
        }
        VSymbol::Parameter(_)
        | VSymbol::GenVar
        | VSymbol::Task(_)
        | VSymbol::Module(_)
        | VSymbol::NamedBlock
        | VSymbol::Function(_)
        | VSymbol::GenerateBlock(_)
        | VSymbol::GenerateBlocks
        | VSymbol::ModuleRange(_) => {}
    }
    Ok(())
}

fn is_zero_sized_replication<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    table: &VSymbolTable,
    scope: SymbolId,
    expr: &Expr<'a>,
) -> bool {
    if let Expr::Replication(r) = expr
        && eval_constant_expr(
            gl,
            arenas,
            table,
            scope,
            &mut Diagnostics::default(),
            r.constant_expr,
            None,
        )
        .is_ok_and(|v| v.into_bits().is_equal_to_zero())
    {
        true
    } else {
        false
    }
}
