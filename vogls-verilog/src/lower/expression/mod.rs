use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, INTEGER_VSIZE, SignalKey, VariableKey, VectorSize,
};
use vogls_utils::OrderedSet;

use crate::ast::expr::{BinaryOperator, BitSlice, Expr, Replication, UnaryOperator};
use crate::ast::{AstId, HIdent};
use crate::elaborate::VSymbol;
use crate::lower::{VType, hident_span, msb_lsb_to_width, try_resolve_hident};
use crate::number::Sign;
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
    ctx: &LowerContext<'a>,
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
                        item.context_width.filter(|_| op.is_self_determined());
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
                    && !op.is_self_determined()
                    && context_width > ty.force_net_width()
                {
                    child = zero_or_sign_extend(mctx.gl(), builder, child, ty, context_width);
                    ty = ty.zero_or_sign_extend(context_width);
                }

                let (variable, ty) = match op {
                    O::LogicalNegation => {
                        (builder.logical_neg(&mut mctx.gl, child), VType::SCALAR_NET)
                    }
                    O::BitwiseNegation => (builder.binary_neg(&mut mctx.gl, child), ty),
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
                    O::SignMinus => (
                        builder.minus_revconstant(
                            mctx.gl(),
                            child,
                            Bits::new_zeroed(ty.force_net_width()),
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
                            &ctx.arenas,
                            &ctx.table,
                            scope,
                            &mut mctx.diagnostics,
                            l,
                        ),
                        get_expr_type(
                            &mctx.gl,
                            &ctx.arenas,
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
                        op.output_width(l_ty.force_net_width(), r_ty.force_net_width());
                    let (l_is_self_det, r_is_self_det) = op.is_self_determined();
                    if let Some(context_width) = item.context_width {
                        child_context_width = child_context_width.max(context_width);
                    }

                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new(
                        r,
                        (!r_is_self_det).then_some(child_context_width),
                    ));
                    dispatch_stack.push(StackItem::new(
                        l,
                        (!l_is_self_det).then_some(child_context_width),
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
                    if !l_is_self_det && context_width > l_ty.force_net_width() {
                        l = zero_or_sign_extend(mctx.gl(), builder, l, l_ty, context_width);
                        l_ty = l_ty.zero_or_sign_extend(context_width);
                    }
                    if !r_is_self_det && context_width > r_ty.force_net_width() {
                        r = zero_or_sign_extend(mctx.gl(), builder, r, r_ty, context_width);
                        r_ty = r_ty.zero_or_sign_extend(context_width);
                    }
                }

                let op = match op {
                    O::Power => bin_power,
                    O::Multiply => bin_multiply,
                    O::Divide => bin_divide,
                    O::Modulus => bin_modulus,
                    O::BinaryPlus => bin_plus,
                    O::BinaryMinus => bin_minus,
                    O::ShiftLeft => bin_logical_shift_left,
                    O::ShiftRight => bin_logical_shift_right,
                    O::GreaterThan => bin_greater_than,
                    O::GreaterThanEqual => bin_greater_than_equal,
                    O::LessThan => bin_less_than,
                    O::LessThanEqual => bin_less_than_equal,
                    O::ArithmeticLeftShift => bin_logical_shift_left,
                    O::ArithmeticRightShift => bin_arithmetic_shift_right,
                    O::LogicalEquality => bin_logical_equality,
                    O::LogicalInequality => bin_logical_inequality,
                    O::CaseEquality => bin_case_equality,
                    O::CaseInequality => bin_case_inequality,
                    O::BitwiseAnd => bin_bitwise_and,
                    O::BitwiseXor => bin_bitwise_xor,
                    O::BitwiseXnor => bin_bitwise_xnor,
                    O::BitwiseOr => bin_bitwise_or,
                    O::LogicalAnd => bin_logical_and,
                    O::LogicalOr => bin_logical_or,
                };
                let result = (op)(&mut mctx.gl, builder, l, l_ty, r, r_ty);
                result_stack.push(Some(result));
            }
            Expr::Concatenation(exprs) => {
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
                let Some((mut output, ty)) = result_stack.pop().unwrap() else {
                    result_stack.truncate(end_stack_size);
                    result_stack.push(None);
                    continue;
                };
                let mut width = ty.force_net_width().get();
                for _ in 1..exprs.len() {
                    let Some((next, next_ty)) = result_stack.pop().unwrap() else {
                        result_stack.truncate(end_stack_size);
                        result_stack.push(None);
                        continue;
                    };
                    let next_width = next_ty.force_net_width();
                    output = builder.concat(&mut mctx.gl, next, output);
                    width += next_width.get();
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
                    &ctx.arenas,
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
                let mut width = ty.force_net_width().get();
                for _ in 1..exprs.len() {
                    let Some((next, next_ty)) = result_stack.pop().unwrap() else {
                        result_stack.truncate(end_stack_size);
                        result_stack.push(None);
                        continue;
                    };
                    let next_width = next_ty.force_net_width();
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
                            &ctx.arenas,
                            &ctx.table,
                            scope,
                            &mut mctx.diagnostics,
                            truthy,
                        ),
                        get_expr_type(
                            &mctx.gl,
                            &ctx.arenas,
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
                        VectorSize::max(l_ty.force_net_width(), r_ty.force_net_width());
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

                let (Some((c, _)), Some((mut t, mut t_ty)), Some((mut f, mut f_ty))) =
                    (condition, truthy, falsy)
                else {
                    result_stack.push(None);
                    continue;
                };

                let size = t_ty.force_net_width().max(f_ty.force_net_width());
                if size > t_ty.force_net_width() {
                    t = zero_or_sign_extend(mctx.gl(), builder, t, t_ty, size);
                    t_ty = t_ty.zero_or_sign_extend(size);
                }
                if size > f_ty.force_net_width() {
                    f = zero_or_sign_extend(mctx.gl(), builder, f, f_ty, size);
                    f_ty = f_ty.zero_or_sign_extend(size);
                }

                if let Some(context_width) = item.context_width {
                    if context_width > t_ty.force_net_width() {
                        t = zero_or_sign_extend(mctx.gl(), builder, t, t_ty, context_width);
                        t_ty = t_ty.zero_or_sign_extend(context_width);
                    }
                    if context_width > f_ty.force_net_width() {
                        f = zero_or_sign_extend(mctx.gl(), builder, f, f_ty, context_width);
                    }
                }

                let c = builder.reduce_or(mctx.gl(), c);
                let result = builder.select(mctx.gl(), c, t, f);
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
                let mut exprs = exprs;
                let symbol_key = try_resolve_hident(
                    scope,
                    &ctx.table,
                    &ctx.arenas,
                    ast_ident,
                    &mut mctx.diagnostics,
                )?;
                let symbol = &ctx.table[symbol_key].content;
                let (mut ty, mut var) = match &symbol {
                    VSymbol::Parameter(value) => {
                        let value = value.clone();
                        (value.ty(), builder.constant(mctx.gl(), value.into_bits()))
                    }
                    VSymbol::Task(_)
                    | VSymbol::GenVar
                    | VSymbol::Function(_)
                    | VSymbol::Module(_)
                    | VSymbol::NamedBlock
                    | VSymbol::GenerateBlock(_)
                    | VSymbol::GenerateBlocks => {
                        mctx.diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(expr),
                            "cannot use this symbol",
                        );
                        error = true;
                        result_stack.truncate(end_result_stack_len);
                        result_stack.push(None);
                        continue 'dispatch_loop;
                    }
                    VSymbol::Net(s) => {
                        let mut dims = &s.dims[..];
                        if !dims.is_empty() {
                            if exprs.pop_front().is_none() {
                                mctx.diagnostics.not_yet_implemented(
                                    ctx.arenas.get_span(expr),
                                    "variable array",
                                );
                                error = true;
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue 'dispatch_loop;
                            }

                            let Some((idx, idx_ty)) = result_stack.pop().unwrap() else {
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue 'dispatch_loop;
                            };

                            dims = &dims[..dims.len() - 1];
                            let mut leaf_arr_items = dims.iter().product::<u32>();
                            let idx = truncate_or_extend(
                                &mut mctx.gl,
                                builder,
                                idx,
                                idx_ty,
                                INTEGER_VSIZE,
                            );
                            let mut offset = builder.multiply_constant(
                                &mut mctx.gl,
                                idx,
                                Bits::new_u32(leaf_arr_items),
                            );

                            while let Some(dim) = dims.last()
                                && exprs.pop_front().is_some()
                            {
                                let Some((expr, expr_ty)) = result_stack.pop().unwrap() else {
                                    result_stack.truncate(end_result_stack_len);
                                    result_stack.push(None);
                                    continue 'dispatch_loop;
                                };

                                leaf_arr_items /= *dim;
                                let expr = truncate_or_extend(
                                    mctx.gl(),
                                    builder,
                                    expr,
                                    expr_ty,
                                    INTEGER_VSIZE,
                                );
                                let expr = builder.multiply_constant(
                                    mctx.gl(),
                                    expr,
                                    Bits::new_u32(leaf_arr_items),
                                );
                                offset = builder.plus(mctx.gl(), offset, expr);
                                dims = &dims[..dims.len() - 1];
                            }

                            if !dims.is_empty() {
                                mctx.diagnostics.not_yet_implemented(
                                    ctx.arenas.get_span(expr),
                                    "variable array",
                                );
                                error = true;
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue 'dispatch_loop;
                            }

                            let size = s.ty.force_net_width();
                            let variable = s.net.probe(mctx.gl(), builder);
                            let offset = builder.multiply_constant(
                                mctx.gl(),
                                offset,
                                Bits::new_u32(size.get()),
                            );
                            let variable = builder.slice(mctx.gl(), variable, offset, size);

                            (s.ty, variable)
                        } else {
                            (s.ty, s.net.probe(mctx.gl(), builder))
                        }
                    }
                };

                for _ in 0..exprs.len() {
                    let Some((expr, expr_ty)) = result_stack.pop().unwrap() else {
                        result_stack.truncate(end_result_stack_len);
                        result_stack.push(None);
                        continue 'dispatch_loop;
                    };
                    ty = VType::SCALAR_NET;
                    let expr =
                        truncate_or_extend(&mut mctx.gl, builder, expr, expr_ty, INTEGER_VSIZE);
                    var = builder.select_bit(&mut mctx.gl, var, expr);
                }

                if let Some(slice) = range_expression {
                    let (lsb, width) = match slice {
                        BitSlice::MsbLsb(msb, lsb) => {
                            let Ok((_msb, lsb, width)) = msb_lsb_to_width(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                msb,
                                lsb,
                            ) else {
                                result_stack.push(None);
                                continue;
                            };
                            let lsb_v = builder.constant_u32(&mut mctx.gl, lsb as u32);
                            (lsb_v, width)
                        }
                        BitSlice::PlusWidth(_, width) => {
                            let Some((lsb, lsb_ty)) = result_stack.pop().unwrap() else {
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue;
                            };
                            let Ok(width) = eval_constant_expr(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                width,
                                None,
                            ) else {
                                result_stack.push(None);
                                continue;
                            };
                            let width =
                                VectorSize::new(width.as_integer().unwrap() as u32).unwrap();
                            let lsb = truncate_or_extend(
                                &mut mctx.gl,
                                builder,
                                lsb,
                                lsb_ty,
                                INTEGER_VSIZE,
                            );
                            (lsb, width)
                        }
                        BitSlice::MinusWidth(_, width) => {
                            let Some((lsb, lsb_ty)) = result_stack.pop().unwrap() else {
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue;
                            };

                            let Ok(width) = eval_constant_expr(
                                &mctx.gl,
                                &ctx.arenas,
                                &ctx.table,
                                scope,
                                &mut mctx.diagnostics,
                                width,
                                None,
                            ) else {
                                result_stack.push(None);
                                continue;
                            };
                            let lsb = truncate_or_extend(
                                &mut mctx.gl,
                                builder,
                                lsb,
                                lsb_ty,
                                INTEGER_VSIZE,
                            );
                            let width = width.as_integer().unwrap() as u32;
                            let width_v = builder.constant_u32(&mut mctx.gl, width - 1);
                            let lsb = builder.minus(&mut mctx.gl, lsb, width_v);
                            (lsb, VectorSize::new(width).unwrap())
                        }
                    };

                    ty = VType::UnsignedNet(width);
                    var = builder.slice(&mut mctx.gl, var, lsb, width as VectorSize);
                }

                result_stack.push(Some((var, ty)));
            }
            Expr::FunctionCall(ident, exprs) => {
                if !item.dispatched {
                    item.dispatched = true;
                    let Ok(fn_symbol) = try_resolve_hident(
                        scope,
                        &ctx.table,
                        &ctx.arenas,
                        ident,
                        &mut mctx.diagnostics,
                    ) else {
                        error = true;
                        result_stack.push(None);
                        continue;
                    };
                    let VSymbol::Function(fn_symbol) = &ctx.table[fn_symbol].content else {
                        mctx.diagnostics.not_yet_implemented(
                            hident_span(&ctx.arenas, ident),
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
                            .map(|(e, (_, ty))| StackItem::new(e, Some(ty.force_net_width()))),
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
                    &ctx.arenas,
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
                let s = s
                    .as_bytes()
                    .iter()
                    .copied()
                    .chain(std::iter::once(b'\0'))
                    .collect::<Box<[u8]>>();
                let value =
                    Bits::load_from_slice(&s, VectorSize::new((s.len() * 8) as u32).unwrap());
                let var = builder.constant(&mut mctx.gl, value);
                result_stack.push(Some((var, VType::String(s.len() as u32))));
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

// i op j, where op is: + - * / % & | ^ ^~ ~^
pub fn coerce_bin_arithmetic<'a>(
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

    let ty = coerce_to_max_size_ty(l_ty, r_ty);

    let l = sign_or_zero_extend(gl, builder, l, l_ty, ty.force_net_width());
    let r = sign_or_zero_extend(gl, builder, r, r_ty, ty.force_net_width());

    (l, ty, r, ty)
}

pub fn coerce_to_max_size_ty<'a>(l_ty: VType, r_ty: VType) -> VType {
    // max(L(i),L(j))

    if l_ty == r_ty {
        return l_ty;
    }

    let l_size = l_ty.net_size();
    let r_size = r_ty.net_size();

    let size = match (l_size, r_size) {
        (Some(l), Some(r)) => l.max(r),
        (Some(s), _) | (_, Some(s)) => s,
        (None, None) => unreachable!(),
    };

    VType::net(size, l_ty.is_signed() & r_ty.is_signed())
}

macro_rules! impl_bin_arithmetic {
    ($($f:ident => $builder_f:ident),+ $(,)?) => {
        $(
        fn $f<'a>(
            gl: &mut GlobalContext,
            builder: &mut BasicBlockBuilder,
            l: VariableKey,
            l_ty: VType,
            r: VariableKey,
            r_ty: VType,
        ) -> (VariableKey, VType) {
            let (l, l_ty, r, _) = coerce_bin_arithmetic(
                gl,
                builder,
                l,
                l_ty,
                r,
                r_ty,
            );
            (builder.$builder_f(gl, l, r), l_ty)
        }
        )+
    };
}

macro_rules! impl_bin_eq_ineq {
    ($($f:ident => $builder_f:ident),+ $(,)?) => {
        $(
        fn $f<'a>(
            gl: &mut GlobalContext,
            builder: &mut BasicBlockBuilder,
            l: VariableKey,
            l_ty: VType,
            r: VariableKey,
            r_ty: VType,
        ) -> (VariableKey, VType) {
            let (l, _, r, _) = coerce_bin_arithmetic(
                gl,
                builder,
                l,
                l_ty,
                r,
                r_ty,
            );
            (builder.$builder_f(gl, l, r), VType::SCALAR_NET)
        }
        )+
    };
}

macro_rules! impl_shift {
    ($($f:ident => $builder_f:ident),+ $(,)?) => {
        $(
        fn $f<'a>(
            gl: &mut GlobalContext,
            builder: &mut BasicBlockBuilder,
            l: VariableKey,
            l_ty: VType,
            r: VariableKey,
            r_ty: VType,
        ) -> (VariableKey, VType) {
            let r = sign_or_zero_extend(gl, builder, r, r_ty, INTEGER_VSIZE);
            (builder.$builder_f(gl, l, r), l_ty)
        }
        )+
    };
}

impl_bin_arithmetic! {
    bin_power => power,
    bin_multiply => multiply,
    bin_divide => divide,
    bin_modulus => modulus,
    bin_plus => plus,
    bin_minus => minus,
    bin_bitwise_and => and,
    bin_bitwise_xor => xor,
    bin_bitwise_xnor => xnor,
    bin_bitwise_or => or,
}

impl_bin_eq_ineq! {
    bin_greater_than => unsigned_gt,
    bin_greater_than_equal => unsigned_ge,
    bin_less_than => unsigned_lt,
    bin_less_than_equal => unsigned_le,
    bin_logical_equality => equals,
    bin_logical_inequality => not_equals,
    bin_case_equality => case_equals,
    bin_case_inequality => not_case_equals,
    bin_logical_and => logical_and,
    bin_logical_or => logical_or,
}

impl_shift! {
    bin_logical_shift_left => logical_shift_left,
    bin_logical_shift_right => logical_shift_right,
}

fn bin_arithmetic_shift_right<'a>(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    l: VariableKey,
    l_ty: VType,
    r: VariableKey,
    r_ty: VType,
) -> (VariableKey, VType) {
    let r = sign_or_zero_extend(gl, builder, r, r_ty, INTEGER_VSIZE);
    if l_ty.is_signed() {
        (builder.arithmetic_shift_right(gl, l, r), l_ty)
    } else {
        (builder.logical_shift_right(gl, l, r), l_ty)
    }
}

pub fn sign_or_zero_extend(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    from: VType,
    to: VectorSize,
) -> VariableKey {
    let from_width = from.force_net_width();
    if from_width == to {
        src
    } else if from_width > to {
        builder.truncate(gl, src, to)
    } else {
        if from.is_signed() {
            builder.sign_extend(gl, src, to)
        } else {
            builder.zero_extend(gl, src, to)
        }
    }
}

pub fn truncate_or_extend(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    from: VType,
    to: VectorSize,
) -> VariableKey {
    let from_width = from.force_net_width();
    if from_width == to {
        src
    } else if from_width > to {
        builder.truncate(gl, src, to)
    } else {
        if from.is_signed() {
            builder.sign_extend(gl, src, to)
        } else {
            builder.zero_extend(gl, src, to)
        }
    }
}

pub fn zero_or_sign_extend(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    from: VType,
    to: VectorSize,
) -> VariableKey {
    let from_width = from.force_net_width();
    assert!(from.force_net_width() <= to);
    if from_width == to {
        src
    } else if from.is_signed() {
        builder.sign_extend(gl, src, to)
    } else {
        builder.zero_extend(gl, src, to)
    }
}

pub fn get_used_signals<'a>(
    ctx: &LowerContext<'a>,
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
            Expr::Decimal(_) | Expr::Sized(_) | Expr::String(_) => {}
        }
    }

    if error {
        return Err(());
    }

    Ok(())
}

pub fn get_used_ident_signals<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    signals: &mut OrderedSet<SignalKey>,
    ident: impl Into<HIdent<'a>>,
) -> Result<(), ()> {
    let Ok(symbol_key) =
        try_resolve_hident(scope, &ctx.table, &ctx.arenas, ident, &mut mctx.diagnostics)
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
        | VSymbol::GenerateBlocks => {}
    }
    Ok(())
}
