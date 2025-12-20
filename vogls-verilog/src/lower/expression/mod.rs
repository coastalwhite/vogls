use vogls_ir::{BasicBlockBuilder, Bits, GlobalContext, Value, VariableKey, VectorSize};

use crate::ast::AstId;
use crate::ast::expr::{BinaryOperator, BitPartSelect, BitSlice, Expr, UnaryOperator};
use crate::lower::constant_expr::eval_constant_expr;
use crate::lower::scope::SymbolVariant;
use crate::lower::{VType, VTypeKey, msb_lsb_to_width};
use crate::number::Decimal;
use crate::parser::AstArenas;

use super::scope::Scope;
use super::{Diagnostics, VTypeTable};

pub fn lower_expr<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    expr: AstId<Expr>,
) -> Result<VariableKey, ()> {
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
    let mut result_stack: Vec<Option<(VariableKey, VTypeKey)>> = Vec::new();

    dispatch_stack.push(StackItem::new(expr));

    while let Some(mut item) = dispatch_stack.pop() {
        match arenas.get(item.expr) {
            Expr::BitPartSelect(select) => {
                let BitPartSelect { subject, braced } = select;
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend([*braced, *subject].into_iter().map(StackItem::new));
                    continue;
                }

                let braced = result_stack.pop().unwrap();
                let subject = result_stack.pop().unwrap();

                let (Some((subject, _)), Some((braced, _))) = (subject, braced) else {
                    result_stack.push(None);
                    continue;
                };

                result_stack.push(Some((
                    builder.select_bit(gl, subject, braced),
                    types.scalar_net(),
                )));
            }
            Expr::BitSlice(subject, slice) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.push(StackItem::new(*subject));
                    match slice {
                        BitSlice::MsbLsb(..) => {}
                        BitSlice::PlusWidth(base, _) | BitSlice::MinusWidth(base, _) => {
                            dispatch_stack.push(StackItem::new(*base))
                        }
                    }
                    continue;
                }

                let Some((subject, _)) = result_stack.pop().unwrap() else {
                    result_stack.push(None);
                    continue;
                };

                let (lsb, width) = match slice {
                    BitSlice::MsbLsb(msb, lsb) => {
                        let (_msb, lsb, width) =
                            msb_lsb_to_width(gl, arenas, types, scope, diagnostics, *msb, *lsb)?;
                        let lsb_v = builder.constant(gl, Value::Decimal(lsb as i64));
                        (lsb_v, width)
                    }
                    BitSlice::PlusWidth(_, width) => {
                        let Some((lsb, _)) = result_stack.pop().unwrap() else {
                            result_stack.push(None);
                            continue;
                        };
                        let width =
                            eval_constant_expr(gl, arenas, types, scope, diagnostics, *width)?;
                        let width = width.as_integer().unwrap() as VectorSize;
                        (lsb, width)
                    }
                    BitSlice::MinusWidth(_, width) => {
                        let Some((lsb, _)) = result_stack.pop().unwrap() else {
                            result_stack.push(None);
                            continue;
                        };

                        let width =
                            eval_constant_expr(gl, arenas, types, scope, diagnostics, *width)?;
                        let width = width.as_integer().unwrap();
                        let width_v = builder.constant(gl, Value::Decimal(width - 1));
                        let lsb = builder.minus(gl, lsb, width_v);
                        (lsb, width as VectorSize)
                    }
                };

                let ty = types.insert(VType::VectorNet(width));
                let shifted = builder.lsr(gl, subject, lsb);
                let result = builder.slice(gl, shifted, width as VectorSize);
                result_stack.push(Some((result, ty)));
            }
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
                    O::LogicalNegation => (builder.logical_neg(gl, child), types.scalar_net()),
                    O::BitwiseNegation => (builder.binary_neg(gl, child), ty),
                    O::ReductionAnd => todo!(),
                    O::ReductionOr => todo!(),
                    O::ReductionNand => todo!(),
                    O::ReductionNor => todo!(),
                    O::ReductionXor => (builder.reduce_xor(gl, child), types.scalar_net()),
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
                let result = (op)(
                    gl,
                    arenas,
                    types,
                    diagnostics,
                    expr,
                    builder,
                    l,
                    l_ty,
                    r,
                    r_ty,
                );

                error |= result.is_err();
                result_stack.push(result.ok());
            }
            Expr::Concatenation(exprs) => {
                if exprs.is_empty() {
                    let var = builder.constant(gl, Value::Bits(Bits::Small(0, 0)));
                    let ty = types.insert(VType::VectorNet(0));
                    result_stack.push(Some((var, ty)));
                    continue;
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
                let Some(mut width) = types[ty].net_width() else {
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
                    let Some(next_width) = types[next_ty].net_width() else {
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
                let ty = types.insert(VType::VectorNet(width));
                result_stack.push(Some((output, ty)));
            }
            Expr::Replication(_) => todo!(),
            Expr::Ternary(_, _, _) => todo!(),
            Expr::Ident(ast_ident) => {
                let ident = arenas.get_ident(ast_ident.item.0);
                let Some(symbol_key) = scope.get(&ident) else {
                    diagnostics.var_not_found(arenas, *ast_ident);
                    result_stack.push(None);
                    error = true;
                    continue;
                };
                let symbol = &scope.symbols[symbol_key];
                let var = match symbol.variant {
                    SymbolVariant::Constant(value) => {
                        builder.constant(gl, Value::Decimal(value.unwrap()))
                    }
                    SymbolVariant::Genvar(value) => {
                        builder.constant(gl, Value::Decimal(value.unwrap()))
                    }
                    SymbolVariant::Signal(key) => builder.probe(gl, key),
                    SymbolVariant::Variable(None) => todo!(),
                    SymbolVariant::Variable(Some(key)) => key,
                };
                result_stack.push(Some((var, symbol.ty)));
            }
            Expr::Decimal(decimal) => {
                let decimal = &arenas.decimals[decimal.at];
                let decimal = match decimal {
                    Decimal::Small(v) => *v as i64,
                    _ => todo!(),
                };

                result_stack.push(Some((
                    builder.constant(gl, Value::Decimal(decimal)),
                    types.integer(),
                )));
            }
            Expr::Sized(sized) => {
                let sized = &arenas.sized_numbers[sized.item.at];
                let Some(size) = sized.size else { todo!() };
                let crate::number::Bits::Small(v) = sized.value else {
                    todo!()
                };
                let width = size.as_u32();
                let var = builder.constant(gl, Value::Bits(Bits::Small(v, width)));

                result_stack.push(Some((var, types.insert(VType::VectorNet(width)))));
            }
            Expr::String(_) => todo!(),
        }
    }

    if error {
        return Err(());
    }

    assert_eq!(result_stack.len(), 1);
    let Some((value, _)) = result_stack.pop().unwrap() else {
        panic!();
    };
    Ok(value)
}

// i op j, where op is: + - * / % & | ^ ^~ ~^
pub fn coerce_bin_arithmetic<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    builder: &mut BasicBlockBuilder,
    l: VariableKey,
    l_ty: VTypeKey,
    r: VariableKey,
    r_ty: VTypeKey,
) -> Result<(VariableKey, VTypeKey, VariableKey, VTypeKey), ()> {
    // max(L(i),L(j))

    macro_rules! array_err {
        () => {{
            diagnostics
                .not_yet_implemented(arenas.get_span(expr), "arithmetic operator with array type");
            return Err(());
        }};
    }

    if l_ty == r_ty {
        if types[l_ty].is_array() {
            array_err!();
        }

        return Ok((l, l_ty, r, r_ty));
    }

    if types[l_ty].is_array() || types[r_ty].is_array() {
        array_err!();
    }

    let l_width = types[l_ty].net_width();
    let r_width = types[r_ty].net_width();

    let width = match (l_width, r_width) {
        (Some(l), Some(r)) => l.max(r),
        (Some(s), _) | (_, Some(s)) => s,
        (None, None) => unreachable!(),
    };

    let ty = gl.types.insert(vogls_ir::Type::Bits(width));
    let l = builder.cast(gl, l, ty);
    let r = builder.cast(gl, r, ty);

    let ty = types.insert(VType::VectorNet(width));
    Ok((l, ty, r, ty))
}

macro_rules! impl_bin_arithmetic {
    ($($f:ident => $builder_f:ident),+ $(,)?) => {
        $(
        fn $f<'a>(
            gl: &mut GlobalContext,
            arenas: &'a AstArenas,
            types: &mut VTypeTable,
            diagnostics: &mut Diagnostics,
            expr: AstId<Expr>,
            builder: &mut BasicBlockBuilder,
            l: VariableKey,
            l_ty: VTypeKey,
            r: VariableKey,
            r_ty: VTypeKey,
        ) -> Result<(VariableKey, VTypeKey), ()> {
            let (l, l_ty, r, _) = coerce_bin_arithmetic(
                gl,
                arenas,
                types,
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
            types: &mut VTypeTable,
            diagnostics: &mut Diagnostics,
            expr: AstId<Expr>,
            builder: &mut BasicBlockBuilder,
            l: VariableKey,
            l_ty: VTypeKey,
            r: VariableKey,
            r_ty: VTypeKey,
        ) -> Result<(VariableKey, VTypeKey), ()> {
            let (l, _, r, _) = coerce_bin_arithmetic(
                gl,
                arenas,
                types,
                diagnostics,
                expr,
                builder,
                l,
                l_ty,
                r,
                r_ty,
            )?;
            Ok((builder.$builder_f(gl, l, r), types.scalar_net()))
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
    types: &mut VTypeTable,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    builder: &mut BasicBlockBuilder,
    l: VariableKey,
    l_ty: VTypeKey,
    r: VariableKey,
    r_ty: VTypeKey,
) -> Result<(VariableKey, VTypeKey), ()> {
    if l_ty != types.integer() || r_ty != types.integer() {
        diagnostics.not_yet_implemented(arenas.get_span(expr), "divide with non-integer arguments");
        return Err(());
    }

    Ok((builder.i64_divide(gl, l, r), types.integer()))
}

fn bin_modulus<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    builder: &mut BasicBlockBuilder,
    l: VariableKey,
    l_ty: VTypeKey,
    r: VariableKey,
    r_ty: VTypeKey,
) -> Result<(VariableKey, VTypeKey), ()> {
    if l_ty != types.integer() || r_ty != types.integer() {
        diagnostics.not_yet_implemented(arenas.get_span(expr), "modulus with non-integer arguments");
        return Err(());
    }

    Ok((builder.i64_modulus(gl, l, r), types.integer()))
}
