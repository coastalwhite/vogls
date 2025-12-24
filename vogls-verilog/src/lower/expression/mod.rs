use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, IntrinsicArg, IntrinsicOp, Type, Value, VariableKey,
    VectorSize,
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
                    O::ReductionAnd => todo!(),
                    O::ReductionOr => todo!(),
                    O::ReductionNand => todo!(),
                    O::ReductionNor => todo!(),
                    O::ReductionXor => (builder.reduce_xor(gl, child), VType::SCALAR_NET),
                    O::ReductionXnor => todo!(),
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

                use BinaryOperator as O;
                let op = match op {
                    O::Multiply => bin_multiply,
                    O::Divide => bin_divide,
                    O::Modulus => bin_modulus,
                    O::BinaryPlus => bin_plus,
                    O::BinaryMinus => bin_minus,
                    O::ShiftLeft => todo!(),
                    O::ShiftRight => todo!(),
                    O::GreaterThan => bin_greater_than,
                    O::GreaterThanEqual => bin_greater_than_equal,
                    O::LessThan => bin_less_than,
                    O::LessThanEqual => bin_less_than_equal,
                    O::ArithmeticLeftShift => todo!(),
                    O::ArithmeticRightShift => todo!(),
                    O::LogicalEquality => bin_logical_equality,
                    O::LogicalInequality => bin_logical_inequality,
                    O::CaseEquality => todo!(),
                    O::CaseInequality => todo!(),
                    O::BitwiseAnd => bin_bitwise_and,
                    O::BitwiseXor => bin_bitwise_xor,
                    O::BitwiseXnor => bin_bitwise_xnor,
                    O::BitwiseOr => bin_bitwise_or,
                    O::LogicalAnd => todo!(),
                    O::LogicalOr => todo!(),
                };
                let result = (op)(gl, arenas, diagnostics, expr, builder, l, l_ty, r, r_ty);

                error |= result.is_err();
                result_stack.push(result.ok());
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
                let Some(mut width) = ty.net_width() else {
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
                    let Some(next_width) = next_ty.net_width() else {
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
            Expr::Ternary(_, _, _) => todo!(),
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
                        (value.ty(), builder.constant(gl, value.into_ir()))
                    }
                    SymbolVariant::Genvar(value) => (
                        VType::Integer,
                        builder.constant(gl, Value::Decimal(value.unwrap())),
                    ),
                    SymbolVariant::Task(_) => todo!(),
                    SymbolVariant::Signal(dims, key) => {
                        let ty = VType::from_ir(gl.signals[*key].ty);
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
                            let mut offset = builder.i64_multiply_constant(gl, idx, leaf_arr_items);

                            while let Some(dim) = dims.last()
                                && exprs.pop_front().is_some()
                            {
                                let Some((expr, _)) = result_stack.pop().unwrap() else {
                                    result_stack.truncate(end_result_stack_len);
                                    result_stack.push(None);
                                    continue 'dispatch_loop;
                                };

                                leaf_arr_items /= *dim;
                                let expr = builder.i64_multiply_constant(gl, expr, leaf_arr_items);
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

                            (ty, builder.arr_probe(gl, *key, offset))
                        } else {
                            (ty, builder.probe(gl, *key))
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
                            let lsb_v = builder.constant(gl, Value::Decimal(lsb as i64));
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
                            let width = width.as_integer().unwrap();
                            let width_v = builder.constant(gl, Value::Decimal(width - 1));
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
                    builder.constant(gl, Value::Decimal(decimal)),
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
                let var = builder.constant(gl, Value::Bits(Bits::Small(v, width)));

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
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    builder: &mut BasicBlockBuilder,
    l: VariableKey,
    l_ty: VType,
    r: VariableKey,
    r_ty: VType,
) -> Result<(VariableKey, VType, VariableKey, VType), ()> {
    // max(L(i),L(j))

    if l_ty == r_ty {
        return Ok((l, l_ty, r, r_ty));
    }

    let l_width = l_ty.net_width();
    let r_width = r_ty.net_width();

    let width = match (l_width, r_width) {
        (Some(l), Some(r)) => l.max(r),
        (Some(s), _) | (_, Some(s)) => s,
        (None, None) => unreachable!(),
    };

    let ty = vogls_ir::Type::Bits(width);
    let l = builder.cast(gl, l, ty);
    let r = builder.cast(gl, r, ty);

    let ty = VType::Net(width);
    Ok((l, ty, r, ty))
}

macro_rules! impl_bin_arithmetic {
    ($($f:ident => $builder_f:ident),+ $(,)?) => {
        $(
        fn $f<'a>(
            gl: &mut GlobalContext,
            arenas: &'a AstArenas,
            diagnostics: &mut Diagnostics,
            expr: AstId<Expr>,
            builder: &mut BasicBlockBuilder,
            l: VariableKey,
            l_ty: VType,
            r: VariableKey,
            r_ty: VType,
        ) -> Result<(VariableKey, VType), ()> {
            let (l, l_ty, r, _) = coerce_bin_arithmetic(
                gl,
                arenas,
                diagnostics,
                expr,
                builder,
                l,
                l_ty,
                r,
                r_ty,
            )?;
            Ok((builder.$builder_f(gl, l, r), l_ty))
        }
        )+
    };
}

macro_rules! impl_bin_eq_ineq {
    ($($f:ident => $builder_f:ident),+ $(,)?) => {
        $(
        fn $f<'a>(
            gl: &mut GlobalContext,
            arenas: &'a AstArenas,
            diagnostics: &mut Diagnostics,
            expr: AstId<Expr>,
            builder: &mut BasicBlockBuilder,
            l: VariableKey,
            l_ty: VType,
            r: VariableKey,
            r_ty: VType,
        ) -> Result<(VariableKey, VType), ()> {
            let (l, _, r, _) = coerce_bin_arithmetic(
                gl,
                arenas,
                diagnostics,
                expr,
                builder,
                l,
                l_ty,
                r,
                r_ty,
            )?;
            Ok((builder.$builder_f(gl, l, r), VType::SCALAR_NET))
        }
        )+
    };
}

impl_bin_arithmetic! {
    bin_multiply => multiply,
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
}

fn bin_divide<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    builder: &mut BasicBlockBuilder,
    l: VariableKey,
    l_ty: VType,
    r: VariableKey,
    r_ty: VType,
) -> Result<(VariableKey, VType), ()> {
    if l_ty != VType::Integer || r_ty != VType::Integer {
        diagnostics.not_yet_implemented(arenas.get_span(expr), "divide with non-integer arguments");
        return Err(());
    }

    Ok((builder.i64_divide(gl, l, r), VType::Integer))
}

fn bin_modulus<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    builder: &mut BasicBlockBuilder,
    l: VariableKey,
    l_ty: VType,
    r: VariableKey,
    r_ty: VType,
) -> Result<(VariableKey, VType), ()> {
    if l_ty != VType::Integer || r_ty != VType::Integer {
        diagnostics
            .not_yet_implemented(arenas.get_span(expr), "modulus with non-integer arguments");
        return Err(());
    }

    Ok((builder.i64_modulus(gl, l, r), VType::Integer))
}

pub fn sign_extend_or_truncate(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    src: VariableKey,
    from: VType,
    to: VType,
) -> VariableKey {
    if from == to {
        return src;
    }

    let to_type = to.to_ir_info();
    match (from, to) {
        (VType::Integer, VType::Net(_)) => builder.cast(gl, src, to_type),
        (VType::Net(_), VType::Integer) => builder.cast(gl, src, Type::Decimal),
        (VType::Net(n), VType::Net(m)) => {
            if n > m {
                builder.slice(gl, src, m)
            } else {
                builder.cast(gl, src, to_type)
            }
        }

        (VType::Integer, VType::Integer) => unreachable!(),
    }
}
