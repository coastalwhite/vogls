use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, Bits, GlobalContext, IntrinsicArg, IntrinsicOp,
    VariableKey, VectorSize,
};

use crate::ast::AstId;
use crate::ast::expr::{BinaryOperator, BitSlice, Expr, UnaryOperator};
use crate::lower::constant_expr::eval_constant_expr;
use crate::lower::scope::SymbolVariant;
use crate::lower::{VType, msb_lsb_to_width};
use crate::number::Decimal;
use crate::parser::AstArenas;

use super::Diagnostics;
use super::scope::Scope;

pub fn lower_expr<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
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
                    O::ShiftLeft => nyi!("shift left"),
                    O::ShiftRight => nyi!("shift right"),
                    O::GreaterThan => bin_greater_than,
                    O::GreaterThanEqual => bin_greater_than_equal,
                    O::LessThan => bin_less_than,
                    O::LessThanEqual => bin_less_than_equal,
                    O::ArithmeticLeftShift => nyi!("arithmetic shift left"),
                    O::ArithmeticRightShift => nyi!("arithmetic shift right"),
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
                    panic!("empty concat");
                    // let var = builder.constant(gl, Value::Bits(Bits::Small(0, 0)));
                    // result_stack.push(Some((var, VType::VectorNet(0))));
                    // continue;
                }

                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend(exprs.iter().rev().map(StackItem::new));
                    continue;
                }

                let Some((mut output, ty)) = result_stack.pop().unwrap() else {
                    result_stack.push(None);
                    continue;
                };
                let Some(mut width) = ty.net_size() else {
                    diagnostics.not_yet_implemented(
                        arenas.get_span(exprs.last().unwrap()),
                        "non-net concatenation",
                    );
                    error = true;
                    result_stack.push(None);
                    continue;
                };
                for i in 1..exprs.len() {
                    let Some((next, next_ty)) = result_stack.pop().unwrap() else {
                        result_stack.push(None);
                        continue;
                    };
                    let Some(next_width) = next_ty.net_size() else {
                        diagnostics.not_yet_implemented(
                            arenas.get_span(exprs.get(exprs.len() - i - 1)),
                            "non-net concatenation",
                        );
                        error = true;
                        result_stack.push(None);
                        continue;
                    };
                    output = builder.concat(gl, next, output);
                    width += next_width;
                }
                result_stack.push(Some((output, VType::Net(width))));
            }
            Expr::Replication(_) => todo!(),
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

                let (Ok(t_ty), Ok(f_ty)) = (
                    expr_to_type(gl, arenas, scope, diagnostics, *truthy),
                    expr_to_type(gl, arenas, scope, diagnostics, *falsy),
                ) else {
                    result_stack.push(None);
                    continue;
                };

                let condition_bb = builder.key();
                *builder = builder.next_terminate_later(gl);

                let truthy_bb = builder.key();
                let Ok((t, _)) = lower_expr(gl, arenas, scope, diagnostics, builder, *truthy)
                else {
                    result_stack.push(None);
                    error |= true;
                    continue;
                };
                let (t, _) = coerce_to_max_size(gl, builder, t, t_ty, f_ty);

                *builder = builder.next_terminate_later(gl);
                let falsy_bb = builder.key();
                let Ok((f, _)) = lower_expr(gl, arenas, scope, diagnostics, builder, *falsy) else {
                    result_stack.push(None);
                    error |= true;
                    continue;
                };
                let (f, _) = coerce_to_max_size(gl, builder, f, t_ty, f_ty);

                *builder = builder.next_terminate_later(gl);
                let (outcome, _) = builder.phi(gl, [(truthy_bb, t), (falsy_bb, f)].into());

                gl.bbs[condition_bb].terminator =
                    BasicBlockTerminator::Branch(c, truthy_bb, falsy_bb);
                gl.bbs[truthy_bb].terminator = BasicBlockTerminator::Jump(builder.key());
                gl.bbs[falsy_bb].terminator = BasicBlockTerminator::Jump(builder.key());

                result_stack.push(Some((outcome, t_ty)));
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                if (!exprs.is_empty() || range_expression.is_some()) && !item.dispatched {
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
                        VType::Integer,
                        builder.constant(gl, Bits::from_i64_minimal(value.unwrap())),
                    ),
                    SymbolVariant::Task(_) => todo!(),
                    SymbolVariant::Signal(dims, ty, key) => {
                        let mut dims = &dims[..];
                        if !dims.is_empty() {
                            if exprs.pop_front().is_none() {
                                diagnostics
                                    .not_yet_implemented(arenas.get_span(expr), "variable array");
                                error = true;
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue 'dispatch_loop;
                            }

                            let Some((idx, _)) = result_stack.pop().unwrap() else {
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue 'dispatch_loop;
                            };

                            dims = &dims[..dims.len() - 1];
                            let mut leaf_arr_items = dims.iter().product::<u32>();
                            let mut offset = builder.multiply_constant(gl, idx, leaf_arr_items);

                            while let Some(dim) = dims.last()
                                && exprs.pop_front().is_some()
                            {
                                let Some((expr, _)) = result_stack.pop().unwrap() else {
                                    result_stack.truncate(end_result_stack_len);
                                    result_stack.push(None);
                                    continue 'dispatch_loop;
                                };

                                leaf_arr_items /= *dim;
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

                            (*ty, builder.arr_probe(gl, *key, offset))
                        } else {
                            (*ty, builder.probe(gl, *key))
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
                            let (_msb, lsb, width) =
                                msb_lsb_to_width(gl, arenas, scope, diagnostics, *msb, *lsb)?;
                            let lsb_v = builder.constant_u32(gl, lsb as u32);
                            (lsb_v, width)
                        }
                        BitSlice::PlusWidth(_, width) => {
                            let Some((lsb, _)) = result_stack.pop().unwrap() else {
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue;
                            };
                            let width = eval_constant_expr(gl, arenas, scope, diagnostics, *width)?;
                            let width = width.as_integer().unwrap() as VectorSize;
                            (lsb, width)
                        }
                        BitSlice::MinusWidth(_, width) => {
                            let Some((lsb, _)) = result_stack.pop().unwrap() else {
                                result_stack.truncate(end_result_stack_len);
                                result_stack.push(None);
                                continue;
                            };

                            let width = eval_constant_expr(gl, arenas, scope, diagnostics, *width)?;
                            let width = width.as_integer().unwrap() as u32;
                            let width_v = builder.constant_u32(gl, width - 1);
                            let lsb = builder.minus(gl, lsb, width_v);
                            (lsb, width as VectorSize)
                        }
                    };

                    ty = VType::Net(width);
                    let shifted = builder.lsr(gl, var, lsb);
                    var = builder.slice(gl, shifted, width as VectorSize);
                }

                result_stack.push(Some((var, ty)));
            }
            Expr::FunctionCall(..) => {
                diagnostics.not_yet_implemented(arenas.get_span(expr), "function calls");
                result_stack.push(None);
                error = true;
                continue;
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

                match arenas.get_ident(ident.item.0) {
                    "vogls_dbg" => {
                        if exprs.is_none_or(|e| e.len() != 1) {
                            diagnostics
                                .not_yet_implemented(arenas.get_span(expr), "vogls_dbg #args != 1");
                            result_stack.push(None);
                            error = true;
                            continue;
                        }

                        let Some((e, _)) = result_stack.last().unwrap() else {
                            result_stack.push(None);
                            continue;
                        };
                        builder.intrinsic(
                            gl,
                            IntrinsicOp::Display,
                            vec![IntrinsicArg::Variable(*e)],
                        );
                    }
                    _ => {
                        diagnostics.not_yet_implemented(arenas.get_span(expr), "function calls");
                        result_stack.push(None);
                        error = true;
                        continue;
                    }
                }
            }
            Expr::Decimal(decimal) => {
                let decimal = &arenas.decimals[decimal.at];
                let decimal = match decimal {
                    Decimal::Small(v) => *v as i64,
                    _ => todo!(),
                };

                result_stack.push(Some((
                    builder.constant(gl, Bits::from_i64_truncated(decimal, 32)),
                    VType::Integer,
                )));
            }
            Expr::Sized(sized) => {
                let sized = &arenas.sized_numbers[sized.item.at];
                let crate::number::Bits::Small(v) = sized.value else {
                    todo!()
                };
                let width = match sized.size {
                    None => (64 - v.leading_zeros()).max(1),
                    Some(size) => size.as_u32(),
                };
                let var = builder.constant(gl, Bits::Small(v, width));

                result_stack.push(Some((var, VType::Net(width))));
            }
            Expr::String(_) => todo!(),
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

    let l_size = l_ty.net_size();
    let r_size = r_ty.net_size();

    let size = match (l_size, r_size) {
        (Some(l), Some(r)) => l.max(r),
        (Some(s), _) | (_, Some(s)) => s,
        (None, None) => unreachable!(),
    };

    let l = sign_extend_or_truncate(gl, builder, l, l_ty, size);
    let r = sign_extend_or_truncate(gl, builder, r, r_ty, size);

    let ty = VType::Net(size);
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

    VType::Net(size)
}

pub fn coerce_to_max_size<'a>(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    e: VariableKey,
    l_ty: VType,
    r_ty: VType,
) -> (VariableKey, VType) {
    // max(L(i),L(j))

    if l_ty == r_ty {
        return (e, l_ty);
    }

    let l_size = l_ty.net_size();
    let r_size = r_ty.net_size();

    let size = match (l_size, r_size) {
        (Some(l), Some(r)) => l.max(r),
        (Some(s), _) | (_, Some(s)) => s,
        (None, None) => unreachable!(),
    };

    let e = builder.cast(gl, e, size);

    let ty = VType::Net(size);
    (e, ty)
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

pub fn sign_extend_or_truncate(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    from: VType,
    to: VectorSize,
) -> VariableKey {
    let from = from.force_net_width();
    if from == to {
        src
    } else if from > to {
        builder.slice(gl, src, to)
    } else {
        builder.cast(gl, src, to)
    }
}

fn expr_to_type<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
) -> Result<VType, ()> {
    Ok(match arenas.get(expr) {
        Expr::Unary(op, child) => {
            let child = expr_to_type(gl, arenas, scope, diagnostics, *child)?;
            use UnaryOperator as O;
            match op {
                O::LogicalNegation | O::BitwiseNegation => child,
                O::ReductionAnd
                | O::ReductionOr
                | O::ReductionNand
                | O::ReductionNor
                | O::ReductionXor
                | O::ReductionXnor => VType::SCALAR_NET,
                O::SignPlus => todo!(),
                O::SignMinus => todo!(),
            }
        }
        Expr::Binary(op, l, r) => {
            let l = expr_to_type(gl, arenas, scope, diagnostics, *l)?;
            let r = expr_to_type(gl, arenas, scope, diagnostics, *r)?;
            _ = (l, r);
            use BinaryOperator as O;
            match op {
                O::Multiply => todo!(),
                O::Divide => todo!(),
                O::Modulus => todo!(),
                O::BinaryPlus => coerce_to_max_size_ty(l, r),
                O::BinaryMinus => todo!(),
                O::ShiftLeft => todo!(),
                O::ShiftRight => todo!(),
                O::ArithmeticLeftShift => todo!(),
                O::ArithmeticRightShift => todo!(),
                O::GreaterThan => todo!(),
                O::GreaterThanEqual => todo!(),
                O::LessThan => todo!(),
                O::LessThanEqual => todo!(),
                O::LogicalEquality => todo!(),
                O::LogicalInequality => todo!(),
                O::CaseEquality => todo!(),
                O::CaseInequality => todo!(),
                O::BitwiseAnd => todo!(),
                O::BitwiseXor => todo!(),
                O::BitwiseXnor => todo!(),
                O::BitwiseOr => todo!(),
                O::LogicalAnd => todo!(),
                O::LogicalOr => todo!(),
            }
        }
        Expr::Concatenation(exprs) => {
            let mut width = 0;
            let mut error = false;
            for expr in exprs.iter() {
                match expr_to_type(gl, arenas, scope, diagnostics, expr)
                    .and_then(|t| t.net_size().ok_or(()))
                {
                    Ok(ew) => width += ew,
                    Err(_) => error = true,
                }
            }
            if error {
                return Err(());
            }
            VType::Net(width)
        }
        Expr::Replication(_) => todo!(),
        Expr::Ternary(_, _, _) => todo!(),
        Expr::Ident(ast_ident, exprs, range_expression) => {
            let ident = arenas.get_ident(ast_ident.item.0);
            let Some(symbol_key) = scope.get(&ident) else {
                diagnostics.var_not_found(arenas, *ast_ident);
                return Err(());
            };
            let (n_dims, ty) = match &scope.symbols[symbol_key].variant {
                SymbolVariant::Genvar(_) => (0 as _, VType::Integer),
                SymbolVariant::Constant(vvalue) => (0, vvalue.ty()),
                SymbolVariant::Signal(dims, ty, _signal_key) => (dims.len(), *ty),
                SymbolVariant::Task(_) => todo!(),
            };

            if n_dims > exprs.len() {
                diagnostics.not_yet_implemented(arenas.get_span(expr), "more dims than exprs");
                return Err(());
            }

            match range_expression {
                None if exprs.len() > n_dims => VType::SCALAR_NET,
                None => ty,
                Some(bit_slice) => {
                    let width = match bit_slice {
                        BitSlice::MsbLsb(msb, lsb) => {
                            let (_, _, width) =
                                msb_lsb_to_width(gl, arenas, scope, diagnostics, *msb, *lsb)?;
                            width
                        }
                        BitSlice::PlusWidth(_, width) | BitSlice::MinusWidth(_, width) => {
                            let width = eval_constant_expr(gl, arenas, scope, diagnostics, *width)?;
                            let width = width.as_integer().unwrap() as VectorSize;
                            width
                        }
                    };
                    VType::Net(width)
                }
            }
        }
        Expr::Decimal(_) => VType::Integer,
        Expr::Sized(sized) => {
            let sized = &arenas.sized_numbers[sized.item.at];
            let Some(size) = sized.size else { todo!() };
            VType::Net(size.as_u32())
        }
        Expr::String(_) => todo!(),
        Expr::FunctionCall(ast_item, ast_id_range) => todo!(),
        Expr::SystemFunctionCall(ast_item, ast_id_range) => todo!(),
    })
}
