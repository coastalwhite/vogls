use std::marker::PhantomData;

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, SignalSlice, VectorSize};

use crate::ast::AstId;
use crate::ast::expr::{BinaryOperator, Expr, UnaryOperator};
use crate::elaborate::VSymbol;
use crate::lower::addressing::{ConstantAddressingContext, lower_addressing};
use crate::lower::expression::get_expr_type;
use crate::lower::cond_expr::{CondExpr, CondExprBuilder};
use crate::lower::{Diagnostics, LowerContext, VType, eval_constant_expr, try_resolve_hident};
use crate::number::Sign;

use super::StackItem;

enum StepErr {
    Continue,
    Unsupported,
}

/// Converts the state-dependent condition `expr` of a module path to a [`CondExpr`] in the path
/// conditions of the driver resolution, if it only combines scalar nets and constants with
/// logical, bit-wise, equality and conditional operators.
pub fn lower_expr_to_cond_expr<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    builder: &mut CondExprBuilder<'_>,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    context_width: Option<VectorSize>,
) -> Option<CondExpr> {
    let mut dispatch_stack: Vec<StackItem<'a>> = Vec::new();
    let mut result_stack: Vec<(CondExpr, VType)> = Vec::new();

    dispatch_stack.push(StackItem::new(expr, context_width));

    while let Some(item) = dispatch_stack.pop() {
        let (cond_expr, vtype) = match lower_subexpr_to_cond_expr(
            ctx,
            gl,
            builder,
            scope,
            item.clone(),
            &mut dispatch_stack,
            &mut result_stack,
        ) {
            Err(StepErr::Continue) => continue,
            Err(StepErr::Unsupported) => {
                let value = eval_constant_expr(
                    gl,
                    ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut Diagnostics::default(),
                    expr.into_constant(),
                    context_width,
                )
                .ok()?;
                let ty = value.ty();
                (builder.constant(&value.into_bits()), ty)
            }
            Ok(v) => v,
        };
        result_stack.push((cond_expr, vtype));
    }

    assert_eq!(result_stack.len(), 1);
    let (value, _ty) = result_stack.pop().unwrap();
    Some(value)
}

fn lower_subexpr_to_cond_expr<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    builder: &mut CondExprBuilder<'_>,
    scope: SymbolId,
    mut item: StackItem<'a>,
    dispatch_stack: &mut Vec<StackItem<'a>>,
    result_stack: &mut Vec<(CondExpr, VType)>,
) -> Result<(CondExpr, VType), StepErr> {
    Ok(match *item.expr {
        Expr::Unary(op, child) => {
            use UnaryOperator as O;

            if !item.dispatched {
                item.dispatched = true;

                let child_context_width = item.context_width.filter(|_| !op.is_self_determined());
                dispatch_stack.push(item);
                dispatch_stack.push(StackItem::new(child, child_context_width));
                return Err(StepErr::Continue);
            }

            let (child, ty) = result_stack.pop().unwrap();

            if let Some(context_width) = item.context_width
                && !ty.is_real()
                && !op.is_self_determined()
                && context_width > ty.bit_length()
            {
                return Err(StepErr::Unsupported);
            }

            let (variable, ty) = match op {
                O::LogicalNegation if ty.is_real() => {
                    return Err(StepErr::Unsupported);
                }
                O::LogicalNegation => (builder.logical_not(child), VType::SCALAR_NET),
                O::BitwiseNegation
                | O::ReductionAnd
                | O::ReductionOr
                | O::ReductionNand
                | O::ReductionNor
                | O::ReductionXor
                | O::ReductionXnor
                    if ty.is_real() =>
                {
                    return Err(StepErr::Unsupported);
                }
                O::BitwiseNegation => (builder.bitwise_not(child), ty),
                O::ReductionAnd => (builder.reduce_and(child), VType::SCALAR_NET),
                O::ReductionOr => (builder.reduce_or(child), VType::SCALAR_NET),
                O::ReductionNand => (builder.reduce_nand(child), VType::SCALAR_NET),
                O::ReductionNor => (builder.reduce_nor(child), VType::SCALAR_NET),
                O::ReductionXor => (builder.reduce_xor(child), VType::SCALAR_NET),
                O::ReductionXnor => (builder.reduce_xnor(child), VType::SCALAR_NET),

                O::SignPlus => (child, ty),
                O::SignMinus if ty.is_real() => return Err(StepErr::Unsupported),
                O::SignMinus => return Err(StepErr::Unsupported),
            };
            (variable, ty)
        }
        Expr::Binary(op, l, r) => {
            use BinaryOperator as O;
            if !item.dispatched {
                item.dispatched = true;

                let l_ty = get_expr_type(
                    gl,
                    ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut Diagnostics::default(),
                    l,
                )
                .map_err(|_| StepErr::Unsupported)?;
                let r_ty = get_expr_type(
                    gl,
                    ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut Diagnostics::default(),
                    r,
                )
                .map_err(|_| StepErr::Unsupported)?;

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
                return Err(StepErr::Continue);
            }

            let (r, r_ty) = result_stack.pop().unwrap();
            let (l, l_ty) = result_stack.pop().unwrap();

            let (l_is_self_det, r_is_self_det) = op.is_self_determined();
            if let Some(context_width) = item.context_width {
                if !l_is_self_det && !l_ty.is_real() && context_width > l_ty.bit_length() {
                    return Err(StepErr::Unsupported);
                }
                if !r_is_self_det && !r_ty.is_real() && context_width > r_ty.bit_length() {
                    return Err(StepErr::Unsupported);
                }
            }

            macro_rules! logical_op {
                ($f:ident) => {{
                    if l_ty != r_ty || l_ty.bit_length() != SCALAR_VSIZE || l_ty.is_real() {
                        return Err(StepErr::Unsupported);
                    }
                    (builder.$f(l, r), VType::SCALAR_NET)
                }};
            }
            macro_rules! bitwise_op {
                ($f:ident) => {{
                    if l_ty != r_ty || l_ty.is_real() {
                        return Err(StepErr::Unsupported);
                    }
                    (builder.$f(l, r), l_ty)
                }};
            }

            match op {
                O::Power
                | O::Multiply
                | O::BinaryPlus
                | O::BinaryMinus
                | O::Divide
                | O::Modulus
                | O::ShiftLeft
                | O::ShiftRight
                | O::GreaterThan
                | O::GreaterThanEqual
                | O::LessThan
                | O::LessThanEqual
                | O::ArithmeticLeftShift
                | O::ArithmeticRightShift => return Err(StepErr::Unsupported),
                O::LogicalEquality => logical_op!(eq),
                O::LogicalInequality => logical_op!(neq),
                O::CaseEquality => logical_op!(case_eq),
                O::CaseInequality => logical_op!(case_neq),
                O::BitwiseAnd => bitwise_op!(and),
                O::BitwiseXor => bitwise_op!(xor),
                O::BitwiseXnor => bitwise_op!(xnor),
                O::BitwiseOr => bitwise_op!(or),
                O::LogicalAnd | O::LogicalOr if l_ty.is_real() || r_ty.is_real() => {
                    return Err(StepErr::Unsupported);
                }
                O::LogicalAnd => (builder.logical_and(l, r), VType::SCALAR_NET),
                O::LogicalOr => (builder.logical_or(l, r), VType::SCALAR_NET),
            }
        }
        Expr::Concatenation(_) => return Err(StepErr::Unsupported),
        Expr::Replication(_) => return Err(StepErr::Unsupported),
        Expr::Ternary(condition, truthy, falsy) => {
            if !item.dispatched {
                item.dispatched = true;

                let l_ty = get_expr_type(
                    gl,
                    ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut Diagnostics::default(),
                    truthy,
                )
                .map_err(|_| StepErr::Unsupported)?;
                let r_ty = get_expr_type(
                    gl,
                    ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut Diagnostics::default(),
                    falsy,
                )
                .map_err(|_| StepErr::Unsupported)?;

                let mut child_context_width = VectorSize::max(l_ty.bit_length(), r_ty.bit_length());
                if let Some(context_width) = item.context_width {
                    child_context_width = child_context_width.max(context_width);
                }

                dispatch_stack.push(item);
                dispatch_stack.push(StackItem::new_no_ctx(condition));
                dispatch_stack.push(StackItem::new(truthy, Some(child_context_width)));
                dispatch_stack.push(StackItem::new(falsy, Some(child_context_width)));
                return Err(StepErr::Continue);
            }

            // @TODO: This is not 100% semantically correct. It should mix truthy and falsy
            // when the condition is `x` or `z`.

            let condition = result_stack.pop().unwrap();
            let truthy = result_stack.pop().unwrap();
            let falsy = result_stack.pop().unwrap();

            let ((c, c_ty), (t, t_ty), (f, f_ty)) = (condition, truthy, falsy);

            if t_ty.is_real() || f_ty.is_real() {
                return Err(StepErr::Unsupported);
            } else {
                let size = t_ty.bit_length().max(f_ty.bit_length());
                if size > t_ty.bit_length() {
                    return Err(StepErr::Unsupported);
                }
                if size > f_ty.bit_length() {
                    return Err(StepErr::Unsupported);
                }

                if let Some(context_width) = item.context_width {
                    if context_width > t_ty.bit_length() {
                        return Err(StepErr::Unsupported);
                    }
                    if context_width > f_ty.bit_length() {
                        return Err(StepErr::Unsupported);
                    }
                }
            }

            let c = if c_ty.bit_length() != SCALAR_VSIZE {
                builder.to_logical(c)
            } else {
                c
            };
            let result = builder.mux(c, t, f);
            (result, t_ty)
        }
        Expr::Ident(ast_ident, exprs, range_expression) => {
            let Ok(symbol_key) = try_resolve_hident(
                scope,
                &ctx.table,
                ctx.arenas,
                ast_ident,
                &mut Diagnostics::default(),
            ) else {
                return Err(StepErr::Unsupported);
            };

            let symbol = &ctx.table[symbol_key].content;

            let (ty, dims, transform, signal) = match &symbol {
                VSymbol::Signal(s) => (s.ty, &s.dims[..], s.transform, s.terminals.probe_signal()),
                _ => return Err(StepErr::Unsupported),
            };

            let mut actx = ConstantAddressingContext {
                gl,
                arenas: ctx.arenas,
                table: &ctx.table,
                scope,
                diagnostics: &mut Diagnostics::default(),
                loc: 0,
                _pd: PhantomData,
            };
            let Ok(address) = lower_addressing(
                &mut actx,
                ty.bit_length(),
                &dims,
                transform,
                exprs.iter().map(|e| e.into_constant()),
                range_expression.map(Into::into),
            ) else {
                return Err(StepErr::Unsupported);
            };
            if address.array.is_some() {
                return Err(StepErr::Unsupported);
            }

            let width = ty.bit_length();
            let (slice, ty) = match address.elem_offset {
                None => (SignalSlice::with_end(width), ty),
                Some(offset) => {
                    let slice = u32::try_from(offset)
                        .ok()
                        .and_then(|lsb| SignalSlice::from_width(lsb, address.output_width))
                        .filter(|slice| slice.msb() < width.get())
                        .ok_or(StepErr::Unsupported)?;
                    (slice, VType::net(address.output_width, !address.is_unsigned))
                }
            };
            (builder.terminal(signal, slice), ty)
        }
        Expr::Decimal(decimal) => {
            let decimal = &ctx.arenas.decimals[decimal.at];
            (builder.constant(decimal), VType::SignedNet(INTEGER_VSIZE))
        }
        Expr::Sized(sized) => {
            let sized = &ctx.arenas.sized_numbers[sized.item.at];
            let signed = matches!(sized.sign, Sign::Signed);
            let size = sized.value.size();
            let var = builder.constant(&sized.value);
            (var, VType::net(size, signed))
        }

        Expr::FunctionCall(..)
        | Expr::SystemFunctionCall(..)
        | Expr::Real(..)
        | Expr::String(..) => return Err(StepErr::Unsupported),
    })
}
