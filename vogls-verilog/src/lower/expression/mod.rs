use std::collections::HashSet;

use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, Bits, GlobalContext, INTEGER_VSIZE, SignalKey,
    VariableKey, VectorSize,
};

use crate::ast::AstId;
use crate::ast::expr::{BinaryOperator, BitSlice, Expr, Replication, UnaryOperator};
use crate::lower::constant_expr::eval_constant_expr;
use crate::lower::scope::SymbolVariant;
use crate::lower::{VType, msb_lsb_to_width};
use crate::number::Sign;
use crate::parser::AstArenas;

use super::Diagnostics;
use super::scope::Scope;

mod system_function_call;
mod function_call;

#[deny(clippy::question_mark_used)] // Needs to be handled explicitly in the recursion.
pub fn lower_expr<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    expr: AstId<Expr>,
) -> Result<(VariableKey, VType), ()> {
    struct StackItem {
        expr: AstId<Expr>,
        dispatched: bool,
    }
    impl StackItem {
        pub fn new(expr: AstId<Expr>) -> Self {
            Self {
                expr,
                dispatched: false,
            }
        }
    }

    let mut error = false;
    let mut dispatch_stack: Vec<StackItem> = Vec::new();
    let mut result_stack: Vec<Option<(VariableKey, VType)>> = Vec::new();

    dispatch_stack.push(StackItem::new(expr));

    'dispatch_loop: while let Some(mut item) = dispatch_stack.pop() {
        match arenas.get(item.expr) {
            Expr::Unary(op, child) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new(*child));
                    continue;
                }

                let child = result_stack.pop().unwrap();

                let Some((child, ty)) = child else {
                    result_stack.push(None);
                    continue;
                };

                use UnaryOperator as O;
                let (variable, ty) = match op {
                    O::LogicalNegation => (builder.logical_neg(gl, child), VType::SCALAR_NET),
                    O::BitwiseNegation => (builder.binary_neg(gl, child), ty),
                    O::ReductionAnd => (builder.reduce_and(gl, child), VType::SCALAR_NET),
                    O::ReductionOr => (builder.reduce_or(gl, child), VType::SCALAR_NET),
                    O::ReductionNand => (builder.reduce_nand(gl, child), VType::SCALAR_NET),
                    O::ReductionNor => (builder.reduce_nor(gl, child), VType::SCALAR_NET),
                    O::ReductionXor => (builder.reduce_xor(gl, child), VType::SCALAR_NET),
                    O::ReductionXnor => (builder.reduce_xnor(gl, child), VType::SCALAR_NET),
                    O::SignPlus => todo!(),
                    O::SignMinus => todo!(),
                };
                result_stack.push(Some((variable, ty)));
            }
            Expr::Binary(op, l, r) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend([*r, *l].into_iter().map(StackItem::new));
                    continue;
                }

                let r = result_stack.pop().unwrap();
                let l = result_stack.pop().unwrap();

                let (Some((l, l_ty)), Some((r, r_ty))) = (l, r) else {
                    result_stack.push(None);
                    continue;
                };

                macro_rules! nyi {
                    ($t:literal) => {{
                        diagnostics.not_yet_implemented(
                            arenas.get_span(item.expr),
                            concat!("binexpr not implemented: ", $t),
                        );
                        result_stack.push(None);
                        error |= true;
                        continue;
                    }};
                }

                use BinaryOperator as O;
                let op = match op {
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
                    O::CaseEquality => nyi!("case equality"),
                    O::CaseInequality => nyi!("case inequality"),
                    O::BitwiseAnd => bin_bitwise_and,
                    O::BitwiseXor => bin_bitwise_xor,
                    O::BitwiseXnor => bin_bitwise_xnor,
                    O::BitwiseOr => bin_bitwise_or,
                    O::LogicalAnd => bin_logical_and,
                    O::LogicalOr => bin_logical_or,
                };
                let result = (op)(gl, builder, l, l_ty, r, r_ty);
                result_stack.push(Some(result));
            }
            Expr::Concatenation(exprs) => {
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
                    dispatch_stack.extend(exprs.iter().rev().map(StackItem::new));
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
                    output = builder.concat(gl, next, output);
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
                } = *replication;

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
                    dispatch_stack.extend(exprs.iter().rev().map(StackItem::new));
                    continue;
                }

                let end_stack_size = result_stack.len() - exprs.len();
                let Ok(repeat_n) =
                    eval_constant_expr(gl, arenas, scope, diagnostics, constant_expr)
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
                    output = builder.concat(gl, next, output);
                    width += next_width.get();
                }

                let Some(output_width) = width.checked_mul(repeat_n as u32) else {
                    diagnostics
                        .not_yet_implemented(arenas.get_span(item.expr), "replication overflow");
                    error = true;
                    result_stack.push(None);
                    continue;
                };

                let output_single = output;
                for _ in 1..repeat_n {
                    output = builder.concat(gl, output_single, output);
                }
                result_stack.push(Some((
                    output,
                    VType::UnsignedNet(VectorSize::new(output_width).unwrap()),
                )));
            }
            Expr::Ternary(condition, truthy, falsy) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new(*condition));
                    continue;
                }

                // @TODO: Make properly non-recursive

                let condition = result_stack.pop().unwrap();

                let Some((c, _)) = condition else {
                    result_stack.push(None);
                    continue;
                };

                let c = builder.reduce_or(gl, c);
                let condition_bb = builder.key();

                *builder = builder.next_terminate_later(gl);
                let truthy_start_bb = builder.key();
                let Ok((t, t_ty)) = lower_expr(gl, arenas, scope, diagnostics, builder, *truthy)
                else {
                    result_stack.push(None);
                    error = true;
                    continue;
                };
                let truthy_end_bb = builder.key();

                *builder = builder.next_terminate_later(gl);
                let falsy_start_bb = builder.key();
                let Ok((f, f_ty)) = lower_expr(gl, arenas, scope, diagnostics, builder, *falsy)
                else {
                    result_stack.push(None);
                    error = true;
                    continue;
                };
                let falsy_end_bb = builder.key();

                let ty = coerce_to_max_size_ty(t_ty, f_ty);
                let ty_size = ty.force_net_width();

                *builder = builder.continue_with(gl, truthy_end_bb);
                let t = sign_or_zero_extend(gl, builder, t, t_ty, ty_size);

                *builder = builder.continue_with(gl, falsy_end_bb);
                let f = sign_or_zero_extend(gl, builder, f, f_ty, ty_size);

                *builder = builder.next_terminate_later(gl);
                let (outcome, _) = builder.phi(gl, [(truthy_end_bb, t), (falsy_end_bb, f)].into());

                gl.bbs[condition_bb].terminator =
                    BasicBlockTerminator::Branch(c, truthy_start_bb, falsy_start_bb);
                gl.bbs[truthy_end_bb].terminator = BasicBlockTerminator::Jump(builder.key());
                gl.bbs[falsy_end_bb].terminator = BasicBlockTerminator::Jump(builder.key());

                result_stack.push(Some((outcome, ty)));
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
                            dispatch_stack.push(StackItem::new(*base))
                        }
                    }
                    dispatch_stack.extend(exprs.iter().map(StackItem::new));
                    continue;
                }

                let end_result_stack_len = result_stack.len()
                    - exprs.len()
                    - usize::from(matches!(
                        range_expression,
                        Some(BitSlice::PlusWidth(..) | BitSlice::MinusWidth(..))
                    ));
                let mut exprs = *exprs;
                let ident = arenas.get_ident(ast_ident.item.0);
                let Some(symbol_key) = scope.get(&ident) else {
                    diagnostics.var_not_found(arenas, *ast_ident);
                    result_stack.push(None);
                    error = true;
                    continue;
                };
                let symbol = &scope.symbols[symbol_key];
                let (mut ty, mut var) = match &symbol.variant {
                    SymbolVariant::Constant(value) => {
                        let value = value.clone();
                        (value.ty(), builder.constant(gl, value.into_bits()))
                    }
                    SymbolVariant::Genvar(value) => (
                        VType::SignedNet(INTEGER_VSIZE),
                        builder
                            .constant(gl, Bits::from_i64_truncated(value.unwrap(), INTEGER_VSIZE)),
                    ),
                    SymbolVariant::Task(_) => todo!(),
                    SymbolVariant::Signal(s) => {
                        let mut dims = &s.dims[..];
                        if !dims.is_empty() {
                            if exprs.pop_front().is_none() {
                                diagnostics
                                    .not_yet_implemented(arenas.get_span(expr), "variable array");
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
                            let idx = truncate_or_extend(gl, builder, idx, idx_ty, INTEGER_VSIZE);
                            let mut offset = builder.multiply_constant(gl, idx, leaf_arr_items);

                            while let Some(dim) = dims.last()
                                && exprs.pop_front().is_some()
                            {
                                let Some((expr, expr_ty)) = result_stack.pop().unwrap() else {
                                    result_stack.truncate(end_result_stack_len);
                                    result_stack.push(None);
                                    continue 'dispatch_loop;
                                };

                                leaf_arr_items /= *dim;
                                let expr =
                                    truncate_or_extend(gl, builder, expr, expr_ty, INTEGER_VSIZE);
                                let expr = builder.multiply_constant(gl, expr, leaf_arr_items);
                                offset = builder.plus(gl, offset, expr);
                                dims = &dims[..dims.len() - 1];
                            }

                            if !dims.is_empty() {
                                diagnostics
                                    .not_yet_implemented(arenas.get_span(expr), "variable array");
                                error = true;
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue 'dispatch_loop;
                            }

                            let size = s.ty.force_net_width();
                            let variable = builder.probe(gl, s.key);
                            let offset = builder.multiply_constant(gl, offset, size.get());
                            let variable = builder.logical_shift_right(gl, variable, offset);
                            let variable = builder.slice(gl, variable, size);

                            (s.ty, variable)
                        } else {
                            (s.ty, builder.probe(gl, s.key))
                        }
                    }
                };

                for _ in 0..exprs.len() {
                    let Some((expr, _)) = result_stack.pop().unwrap() else {
                        result_stack.truncate(end_result_stack_len);
                        result_stack.push(None);
                        continue 'dispatch_loop;
                    };
                    ty = VType::SCALAR_NET;
                    var = builder.select_bit(gl, var, expr);
                }

                if let Some(slice) = range_expression {
                    let (lsb, width) = match slice {
                        BitSlice::MsbLsb(msb, lsb) => {
                            let Ok((_msb, lsb, width)) =
                                msb_lsb_to_width(gl, arenas, scope, diagnostics, *msb, *lsb)
                            else {
                                result_stack.push(None);
                                continue;
                            };
                            let lsb_v = builder.constant_u32(gl, lsb as u32);
                            (lsb_v, width)
                        }
                        BitSlice::PlusWidth(_, width) => {
                            let Some((lsb, _)) = result_stack.pop().unwrap() else {
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue;
                            };
                            let Ok(width) =
                                eval_constant_expr(gl, arenas, scope, diagnostics, *width)
                            else {
                                result_stack.push(None);
                                continue;
                            };
                            let width =
                                VectorSize::new(width.as_integer().unwrap() as u32).unwrap();
                            (lsb, width)
                        }
                        BitSlice::MinusWidth(_, width) => {
                            let Some((lsb, _)) = result_stack.pop().unwrap() else {
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue;
                            };

                            let Ok(width) =
                                eval_constant_expr(gl, arenas, scope, diagnostics, *width)
                            else {
                                result_stack.push(None);
                                continue;
                            };
                            let width = width.as_integer().unwrap() as u32;
                            let width_v = builder.constant_u32(gl, width - 1);
                            let lsb = builder.minus(gl, lsb, width_v);
                            (lsb, VectorSize::new(width).unwrap())
                        }
                    };

                    ty = VType::UnsignedNet(width);
                    let shifted = builder.logical_shift_right(gl, var, lsb);
                    var = builder.slice(gl, shifted, width as VectorSize);
                }

                result_stack.push(Some((var, ty)));
            }
            Expr::FunctionCall(ident, exprs) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend(exprs.iter().map(StackItem::new));
                    continue;
                }

                let num_args = exprs.len();
                let result = function_call::lower_function_call(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    builder,
                    expr,
                    *ident,
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
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    if let Some(exprs) = exprs {
                        dispatch_stack.extend(exprs.iter().map(StackItem::new));
                    }
                    continue;
                }

                let num_args = exprs.map_or(0, |e| e.len());
                let result = system_function_call::lower_system_function_call(
                    gl,
                    arenas,
                    diagnostics,
                    builder,
                    expr,
                    *ident,
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
                let decimal = &arenas.decimals[decimal.at];
                result_stack.push(Some((
                    builder.constant(gl, decimal.clone()),
                    VType::SignedNet(INTEGER_VSIZE),
                )));
            }
            Expr::Sized(sized) => {
                let sized = &arenas.sized_numbers[sized.item.at];
                let signed = matches!(sized.sign, Sign::Signed);
                let size = sized.value.size();
                let var = builder.constant(gl, sized.value.clone());
                result_stack.push(Some((var, VType::net(size, signed))));
            }
            Expr::String(string_ref) => {
                let s = arenas.get_ident(string_ref.0);
                let s = s
                    .as_bytes()
                    .iter()
                    .copied()
                    .chain(std::iter::once(b'\0'))
                    .collect::<Box<[u8]>>();
                let value =
                    Bits::load_from_slice(&s, VectorSize::new((s.len() * 8) as u32).unwrap());
                let var = builder.constant(gl, value);
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
    bin_logical_and => logical_and,
    bin_logical_or => logical_or,
}

impl_shift! {
    bin_logical_shift_left => logical_shift_left,
    bin_logical_shift_right => logical_shift_right,
    bin_arithmetic_shift_right => arithmetic_shift_right,
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
        builder.slice(gl, src, to)
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
        builder.slice(gl, src, to)
    } else {
        if from.is_signed() {
            builder.sign_extend(gl, src, to)
        } else {
            builder.zero_extend(gl, src, to)
        }
    }
}

pub fn get_used_signals<'a>(
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
) -> Result<Vec<SignalKey>, ()> {
    struct StackItem {
        expr: AstId<Expr>,
        _dispatched: bool,
    }
    impl StackItem {
        pub fn new(expr: AstId<Expr>) -> Self {
            Self {
                expr,
                _dispatched: false,
            }
        }
    }

    let mut error = false;
    let mut dispatch_stack: Vec<StackItem> = Vec::new();
    let mut signals_seen: HashSet<SignalKey> = HashSet::new();
    let mut signals: Vec<SignalKey> = Vec::new();

    dispatch_stack.push(StackItem::new(expr));

    while let Some(item) = dispatch_stack.pop() {
        match arenas.get(item.expr) {
            Expr::Unary(_, c) => dispatch_stack.push(StackItem::new(*c)),
            Expr::Binary(_, l, r) => {
                dispatch_stack.extend([*l, *r].into_iter().map(StackItem::new))
            }
            Expr::Concatenation(exprs)
            | Expr::Replication(Replication {
                constant_expr: _,
                exprs,
            })
            | Expr::FunctionCall(_, exprs) => {
                dispatch_stack.extend(exprs.iter().map(StackItem::new))
            }
            Expr::SystemFunctionCall(_, exprs) => {
                if let Some(exprs) = exprs {
                    dispatch_stack.extend(exprs.iter().map(StackItem::new))
                }
            }
            Expr::Ternary(c, t, f) => {
                dispatch_stack.extend([*c, *t, *f].into_iter().map(StackItem::new))
            }
            Expr::Ident(ident, exprs, range_expression) => {
                let name = arenas.get_ident(ident.item.0);
                let Some(symbol_key) = scope.get(name) else {
                    diagnostics.var_not_found(arenas, *ident);
                    error = true;
                    continue;
                };
                let symbol = &scope.symbols[symbol_key];
                match &symbol.variant {
                    SymbolVariant::Signal(s) => {
                        if signals_seen.insert(s.key) {
                            signals.push(s.key);
                        }
                    }
                    SymbolVariant::Genvar(_)
                    | SymbolVariant::Constant(_)
                    | SymbolVariant::Task(_) => {}
                }

                dispatch_stack.extend(exprs.iter().map(StackItem::new));
                if let Some(range_expression) = range_expression {
                    match range_expression {
                        BitSlice::MsbLsb(_, _) => {}
                        BitSlice::PlusWidth(base, _) | BitSlice::MinusWidth(base, _) => {
                            dispatch_stack.push(StackItem::new(*base))
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

    Ok(signals)
}
