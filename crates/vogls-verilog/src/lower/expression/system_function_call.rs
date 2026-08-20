use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::dyn_format_string::{DynFormatArgument, DynFormatString};
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, Bits, INTEGER_VSIZE, IntrinsicOp, LogicMode, RandomKind, SCALAR_VSIZE,
    Signal, SignalFlags, SignalKey, TIME_VSIZE, VSIZE_32, VSIZE_64, VariableKey, VectorSize,
};

use crate::ast::expr::Expr;
use crate::ast::statement::SystemTaskIdentifier;
use crate::ast::{AstId, AstIdRange, AstItem};
use crate::lower::expression::{
    coerce_to_max_size_ty, lower_expr, sign_or_zero_extend, to_real, truncate_or_extend,
};
use crate::lower::vvalue::{Real, VValue};
use crate::lower::{Diagnostics, LowerContext, MutLowerContext, VType, try_resolve_net};
use crate::parser::AstArenas;

use super::get_expr_type;

pub fn lower_system_function_call<'a, 'b>(
    ctx: &LowerContext<'a, 'b>,
    mctx: &mut MutLowerContext,
    builder: &mut BasicBlockBuilder,
    expr: AstId<Expr>,
    ident: AstItem<SystemTaskIdentifier>,
    // arguments are in reverse order
    arguments: &[Option<(VariableKey, VType)>],
) -> Result<(VariableKey, VType), ()> {
    macro_rules! ensure_num_args_equal {
        ($expected:expr) => {
            let num_args = arguments.len();
            if num_args != $expected {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(expr),
                    "intrinsic not expected amount",
                );
                return Err(());
            }
        };
    }
    // Math functions accept any numeric argument; non-real operands are coerced
    // to real via `to_real` before the operation is applied.
    macro_rules! real_unary_fn {
        ($builder_method:ident) => {{
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let e = to_real(mctx.gl(), builder, e, e_ty);
            let e = builder.$builder_method(mctx.gl(), e);
            Ok((e, VType::Real))
        }};
    }
    macro_rules! real_binary_fn {
        ($builder_method:ident) => {{
            ensure_num_args_equal!(2);
            // Arguments are in reverse order.
            let (lhs, lhs_ty) = arguments[1].ok_or(())?;
            let (rhs, rhs_ty) = arguments[0].ok_or(())?;
            let lhs = to_real(mctx.gl(), builder, lhs, lhs_ty);
            let rhs = to_real(mctx.gl(), builder, rhs, rhs_ty);
            let e = builder.$builder_method(mctx.gl(), lhs, rhs);
            Ok((e, VType::Real))
        }};
    }

    // @Performance: Use a perfect hashmap here.
    match &ctx.arenas.ident_table[ident.item.0] {
        "signed" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let e_ty = e_ty.to_signed();
            Ok((e, e_ty))
        }
        "unsigned" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let e_ty = e_ty.to_unsigned();
            Ok((e, e_ty))
        }
        "time" => {
            ensure_num_args_equal!(0);
            let ticks = builder.time(mctx.gl());
            let ticks_per_unit = ctx
                .time_scale
                .unit
                .truncate_or_multiply_to(1, ctx.time_resolution);
            let ticks = if ticks_per_unit > 1 {
                // Round half away from zero
                let biased =
                    builder.plus_constant(mctx.gl(), ticks, Bits::new_u64(ticks_per_unit / 2));
                builder.divide_constant(mctx.gl(), biased, Bits::new_u64(ticks_per_unit))
            } else {
                ticks
            };
            Ok((ticks, VType::UnsignedNet(TIME_VSIZE)))
        }
        "realtime" => {
            ensure_num_args_equal!(0);
            let ticks = builder.time(mctx.gl());
            let ticks = builder.real_from_unsigned_decimal(mctx.gl(), ticks);
            let ticks_per_unit = ctx
                .time_scale
                .unit
                .truncate_or_multiply_f64_to(1.0, ctx.time_resolution);
            let ticks_per_unit = builder.constant_u64(mctx.gl(), ticks_per_unit.to_bits());
            let ticks = builder.real_div(mctx.gl(), ticks, ticks_per_unit);
            Ok((ticks, VType::Real))
        }
        "clog2" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let size = e_ty.bit_length();

            // @Performance. Maybe this should get a special instruction
            let e_m_1 = builder.minus_constant(
                mctx.gl(),
                e,
                Bits::new_u32(1).truncate_or_zero_extend(size),
            );
            let lz = builder.count_leading_zeros(mctx.gl(), e_m_1);
            let clog2 =
                builder.revminus_constant(mctx.gl(), lz, Bits::new_u32(e_ty.bit_length().get()));
            let is_zero = builder.case_equals_constant(mctx.gl(), e, Bits::new_zeroed(size));
            let contains_spc = builder.reduce_xor(mctx.gl(), e);
            let contains_spc = builder.case_equals_constant(
                mctx.gl(),
                contains_spc,
                Bits::new_unknown(SCALAR_VSIZE),
            );
            let zero = builder.constant_u32(mctx.gl(), 0);
            let clog2 = builder.select(mctx.gl(), is_zero, zero, clog2);
            let x = builder.constant(mctx.gl(), Bits::new_unknown(VSIZE_32));
            let clog2 = builder.select(mctx.gl(), contains_spc, x, clog2);

            Ok((clog2, VType::UnsignedNet(VSIZE_32)))
        }

        // Math Functions
        "ln" => real_unary_fn!(real_ln),
        "log10" => real_unary_fn!(real_log10),
        "exp" => real_unary_fn!(real_exp),
        "sqrt" => real_unary_fn!(real_sqrt),
        "floor" => real_unary_fn!(real_floor),
        "ceil" => real_unary_fn!(real_ceil),
        "sin" => real_unary_fn!(real_sin),
        "cos" => real_unary_fn!(real_cos),
        "tan" => real_unary_fn!(real_tan),
        "asin" => real_unary_fn!(real_asin),
        "acos" => real_unary_fn!(real_acos),
        "atan" => real_unary_fn!(real_atan),
        "sinh" => real_unary_fn!(real_sinh),
        "cosh" => real_unary_fn!(real_cosh),
        "tanh" => real_unary_fn!(real_tanh),
        "asinh" => real_unary_fn!(real_asinh),
        "acosh" => real_unary_fn!(real_acosh),
        "atanh" => real_unary_fn!(real_atanh),
        "atan2" => real_binary_fn!(real_atan2),
        "hypot" => real_binary_fn!(real_hypot),

        "bitstoreal" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            if e_ty.is_real() {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(expr), "cannot call on real");
                return Err(());
            }
            let e = truncate_or_extend(mctx.gl(), builder, e, e_ty, VSIZE_64);
            Ok((e, VType::Real))
        }
        "realtobits" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let e = super::coerce_to(mctx.gl(), builder, e, e_ty, VType::Real);
            Ok((e, VType::UnsignedNet(VSIZE_64)))
        }
        "itor" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let e = match e_ty {
                VType::SignedNet(_) => builder.real_from_signed_decimal(mctx.gl(), e),
                VType::UnsignedNet(_) => builder.real_from_unsigned_decimal(mctx.gl(), e),
                VType::Real => e,
            };
            Ok((e, VType::Real))
        }
        "rtoi" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let e = super::coerce_to(mctx.gl(), builder, e, e_ty, VType::Real);
            let e = builder.real_truncate(mctx.gl(), e);
            let e = builder.real_to_i64(mctx.gl(), e);
            let e = builder.truncate(mctx.gl(), e, INTEGER_VSIZE);
            Ok((e, VType::SignedNet(INTEGER_VSIZE)))
        }

        // VoGLS specific system function calls
        "vogls_rawticks" => {
            ensure_num_args_equal!(0);
            Ok((builder.time(mctx.gl()), VType::UnsignedNet(TIME_VSIZE)))
        }
        "vogls_dbg" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let format_str =
                DynFormatString::new("\n".into(), [(0, DynFormatArgument::default())].into());
            builder.intrinsic(
                mctx.gl(),
                IntrinsicOp::Display(Box::new(format_str)),
                [e].into(),
            );
            Ok((e, e_ty))
        }
        "vogls_blackbox" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let e = builder.blackbox(mctx.gl(), e);
            Ok((e, e_ty))
        }
        "vogls_copyx" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let ty = coerce_to_max_size_ty(l_ty, r_ty);
            let l = sign_or_zero_extend(mctx.gl(), builder, l, l_ty, ty.bit_length());
            let r = sign_or_zero_extend(mctx.gl(), builder, r, r_ty, ty.bit_length());
            let e = builder.copy_x(mctx.gl(), l, r);
            Ok((e, ty))
        }
        "vogls_copyz" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let ty = coerce_to_max_size_ty(l_ty, r_ty);
            let l = sign_or_zero_extend(mctx.gl(), builder, l, l_ty, ty.bit_length());
            let r = sign_or_zero_extend(mctx.gl(), builder, r, r_ty, ty.bit_length());
            let e = builder.copy_z(mctx.gl(), l, r);
            Ok((e, ty))
        }
        "vogls_min" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let ty = coerce_to_max_size_ty(l_ty, r_ty);
            let l = sign_or_zero_extend(mctx.gl(), builder, l, l_ty, ty.bit_length());
            let r = sign_or_zero_extend(mctx.gl(), builder, r, r_ty, ty.bit_length());
            let e = builder.min(mctx.gl(), l, r);
            Ok((e, ty))
        }
        "vogls_max" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let ty = coerce_to_max_size_ty(l_ty, r_ty);
            let l = sign_or_zero_extend(mctx.gl(), builder, l, l_ty, ty.bit_length());
            let r = sign_or_zero_extend(mctx.gl(), builder, r, r_ty, ty.bit_length());
            let e = builder.max(mctx.gl(), l, r);
            Ok((e, ty))
        }
        "vogls_posedge" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let l = truncate_or_extend(mctx.gl(), builder, l, l_ty, SCALAR_VSIZE);
            let r = truncate_or_extend(mctx.gl(), builder, r, r_ty, SCALAR_VSIZE);
            let e = builder.posedge(mctx.gl(), l, r);
            Ok((e, VType::net(SCALAR_VSIZE, false)))
        }
        "vogls_negedge" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let l = truncate_or_extend(mctx.gl(), builder, l, l_ty, SCALAR_VSIZE);
            let r = truncate_or_extend(mctx.gl(), builder, r, r_ty, SCALAR_VSIZE);
            let e = builder.negedge(mctx.gl(), l, r);
            Ok((e, VType::net(SCALAR_VSIZE, false)))
        }
        "vogls_select" => {
            ensure_num_args_equal!(3);
            let (cond, cond_ty) = arguments[2].ok_or(())?;
            let (truthy, truthy_ty) = arguments[1].ok_or(())?;
            let (falsy, falsy_ty) = arguments[0].ok_or(())?;

            if cond_ty.bit_length() != SCALAR_VSIZE {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(expr),
                    "select condition has to be scalar",
                );
                return Err(());
            }

            let ty = coerce_to_max_size_ty(truthy_ty, falsy_ty);
            let truthy =
                sign_or_zero_extend(mctx.gl(), builder, truthy, truthy_ty, ty.bit_length());
            let falsy = sign_or_zero_extend(mctx.gl(), builder, falsy, falsy_ty, ty.bit_length());
            let e = builder.select(mctx.gl(), cond, truthy, falsy);
            Ok((e, ty))
        }

        _ => {
            mctx.diagnostics
                .not_yet_implemented(ctx.arenas.get_span(expr), "unknown system function call");
            Err(())
        }
    }
}

pub fn get_system_function_call_output_ty(
    arenas: &AstArenas,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    ident: AstItem<SystemTaskIdentifier>,
    // arguments are in reverse order
    arguments: &[Option<VType>],
) -> Result<VType, ()> {
    macro_rules! ensure_num_args_equal {
        ($expected:expr) => {
            let num_args = arguments.len();
            if num_args != $expected {
                diagnostics
                    .not_yet_implemented(arenas.get_span(expr), "intrinsic not expected amount");
                return Err(());
            }
        };
    }
    // @Performance: Use a perfect hashmap here.
    match &arenas.ident_table[ident.item.0] {
        "signed" => {
            ensure_num_args_equal!(1);
            let e_ty = arguments[0].ok_or(())?;
            Ok(e_ty.to_signed())
        }
        "unsigned" => {
            ensure_num_args_equal!(1);
            let e_ty = arguments[0].ok_or(())?;
            Ok(e_ty.to_unsigned())
        }
        "time" => {
            ensure_num_args_equal!(0);
            Ok(VType::UnsignedNet(TIME_VSIZE))
        }
        "realtime" => {
            ensure_num_args_equal!(0);
            Ok(VType::Real)
        }
        "clog2" => {
            ensure_num_args_equal!(1);
            Ok(VType::UnsignedNet(VSIZE_32))
        }

        // Math functions
        "ln" | "log10" | "exp" | "sqrt" | "floor" | "ceil" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "asinh" | "acosh" | "atanh" => {
            ensure_num_args_equal!(1);
            Ok(VType::Real)
        }
        "atan2" | "hypot" => {
            ensure_num_args_equal!(2);
            Ok(VType::Real)
        }
        "bitstoreal" => {
            ensure_num_args_equal!(1);
            Ok(VType::Real)
        }
        "realtobits" => {
            ensure_num_args_equal!(1);
            Ok(VType::UnsignedNet(VSIZE_64))
        }
        "itor" => {
            ensure_num_args_equal!(1);
            Ok(VType::Real)
        }
        "rtoi" => {
            ensure_num_args_equal!(1);
            Ok(VType::SignedNet(INTEGER_VSIZE))
        }

        // VoGLS specific system function calls
        "vogls_rawticks" => {
            ensure_num_args_equal!(0);
            Ok(VType::UnsignedNet(TIME_VSIZE))
        }
        "vogls_dbg" => {
            ensure_num_args_equal!(1);
            let e_ty = arguments[0].ok_or(())?;
            Ok(e_ty)
        }
        "vogls_blackbox" => {
            ensure_num_args_equal!(1);
            let e_ty = arguments[0].ok_or(())?;
            Ok(e_ty)
        }
        "vogls_posedge" | "vogls_negedge" => {
            ensure_num_args_equal!(2);
            Ok(VType::UnsignedNet(SCALAR_VSIZE))
        }
        "vogls_copyx" | "vogls_copyz" | "vogls_min" | "vogls_max" => {
            ensure_num_args_equal!(2);
            let l_ty = arguments[1].ok_or(())?;
            let r_ty = arguments[0].ok_or(())?;
            Ok(coerce_to_max_size_ty(l_ty, r_ty))
        }
        "vogls_select" => {
            ensure_num_args_equal!(3);
            let cond_ty = arguments[2].ok_or(())?;
            let l_ty = arguments[1].ok_or(())?;
            let r_ty = arguments[0].ok_or(())?;
            if cond_ty.bit_length() != SCALAR_VSIZE {
                diagnostics.not_yet_implemented(
                    arenas.get_span(expr),
                    "select condition has to be scalar",
                );
                return Err(());
            }
            Ok(coerce_to_max_size_ty(l_ty, r_ty))
        }

        _ => {
            diagnostics.not_yet_implemented(arenas.get_span(expr), "unknown system function call");
            Err(())
        }
    }
}

pub fn lower_unevaluated_system_function_call<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    ident: AstItem<SystemTaskIdentifier>,
    arguments: Option<AstIdRange<'a, Expr<'a>>>,
) -> Result<Option<(VariableKey, VType)>, ()> {
    match &ctx.arenas.ident_table[ident.item.0] {
        "bits" => {
            let Some(expr) = arguments.and_then(|a| a.first().filter(|_| a.len() == 1)) else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "bits requires one argument",
                );
                return Err(());
            };

            let ty = get_expr_type(
                &mctx.gl,
                ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                expr,
            )?;
            let variable = builder.constant_u32(mctx.gl(), ty.bit_length().get());
            Ok(Some((variable, VType::net(INTEGER_VSIZE, false))))
        }

        "random" => {
            if arguments.is_some_and(|a| a.len() > 1) {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "random expects at most one identifier",
                );
                return Err(());
            }

            let (prb_signal, drv_signal, signal_ty) =
                get_prob_dist_fn_seed(ctx, mctx, scope, ident, arguments)?;

            let seed = builder.probe(mctx.gl(), prb_signal);
            let seed = truncate_or_extend(mctx.gl(), builder, seed, signal_ty, VSIZE_32);
            let min = builder.constant_u32(mctx.gl(), i32::MIN.cast_unsigned());
            let max = builder.constant_u32(mctx.gl(), i32::MAX.cast_unsigned());
            let packed = builder.random(mctx.gl(), RandomKind::Uniform, seed, &[min, max]);
            let new_seed = builder.slice_constant(mctx.gl(), packed, 32, VSIZE_32);
            let result = builder.truncate(mctx.gl(), packed, VSIZE_32);
            let new_seed_trunc = truncate_or_extend(
                mctx.gl(),
                builder,
                new_seed,
                VType::UnsignedNet(VSIZE_32),
                signal_ty.bit_length(),
            );
            builder.drive(mctx.gl(), drv_signal, new_seed_trunc);
            Ok(Some((result, VType::SignedNet(VSIZE_32))))
        }
        system_fn @ ("dist_uniform" | "dist_normal" | "dist_exponential" | "dist_poisson"
        | "dist_chi_square" | "dist_t" | "dist_erlang") => {
            let kind = match system_fn {
                "dist_uniform" => RandomKind::Uniform,
                "dist_normal" => RandomKind::Normal,
                "dist_exponential" => RandomKind::Exponential,
                "dist_poisson" => RandomKind::Poisson,

                "dist_chi_square" => RandomKind::ChiSquare,
                "dist_t" => RandomKind::T,
                "dist_erlang" => RandomKind::Erlang,
                _ => unreachable!(),
            };

            let num_args = match kind {
                RandomKind::Uniform | RandomKind::Normal | RandomKind::Erlang => 2,
                RandomKind::Exponential
                | RandomKind::Poisson
                | RandomKind::ChiSquare
                | RandomKind::T => 1,
            };

            let Some(arguments) = arguments.filter(|v| v.len() == num_args + 1) else {
                mctx.diagnostics.invalid_num_arguments(
                    ctx.arenas.get_item_span(ident),
                    format!("{system_fn} requires 3 arguments"),
                );
                return Err(());
            };

            let (prb_signal, drv_signal, signal_ty) =
                get_prob_dist_fn_seed_signals(ctx, mctx, scope, ident, arguments.get(0))?;

            let seed = builder.probe(mctx.gl(), prb_signal);
            let seed = truncate_or_extend(mctx.gl(), builder, seed, signal_ty, VSIZE_32);
            let mut args = Vec::with_capacity(num_args);
            for arg in arguments.iter().skip(1) {
                args.push(lower_expr(ctx, mctx, scope, builder, arg, Some(VSIZE_32))?.0);
            }
            let packed = builder.random(mctx.gl(), kind, seed, &args);
            let new_seed = builder.slice_constant(mctx.gl(), packed, 32, VSIZE_32);
            let result = builder.truncate(mctx.gl(), packed, VSIZE_32);
            let new_seed_trunc = truncate_or_extend(
                mctx.gl(),
                builder,
                new_seed,
                VType::UnsignedNet(VSIZE_32),
                signal_ty.bit_length(),
            );
            builder.drive(mctx.gl(), drv_signal, new_seed_trunc);
            Ok(Some((result, VType::SignedNet(VSIZE_32))))
        }

        "vogls_lupdt" => {
            let Some(arguments) = arguments else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "last update time requires one argument",
                );
                return Err(());
            };

            let Some(expr) = arguments.first().filter(|_| arguments.len() == 1) else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "last update time requires one argument",
                );
                return Err(());
            };

            let Expr::Ident(arg_ident, array_exprs, bitslice) = &*expr else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "last update time expects an identifier",
                );
                return Err(());
            };

            if !array_exprs.is_empty() || bitslice.is_some() {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "last update time expects an identifier",
                );
                return Err(());
            }

            let net_symbol = try_resolve_net(
                scope,
                &ctx.table,
                ctx.arenas,
                *arg_ident,
                &mut mctx.diagnostics,
            )?;
            let signal = net_symbol.net.probe_signal();
            Ok(Some((
                builder.lupdt(mctx.gl(), signal),
                VType::UnsignedNet(TIME_VSIZE),
            )))
        }
        "vogls_slice" => {
            let Some(arguments) = arguments else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "slice requires three arguments",
                );
                return Err(());
            };

            if arguments.len() != 3 {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "slice requires three arguments",
                );
                return Err(());
            }

            let (src, offset, width) = (arguments.get(0), arguments.get(1), arguments.get(2));

            let (src, src_ty) = lower_expr(ctx, mctx, scope, builder, src, None)?;
            let Expr::Decimal(offset) = &*offset else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "slice snd argument should be decimal",
                );
                return Err(());
            };
            let Expr::Decimal(width) = &*width else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "slice snd argument should be decimal",
                );
                return Err(());
            };

            let offset = ctx.arenas.decimals[offset.at].extract_exact_u32().unwrap();
            let width = ctx.arenas.decimals[width.at].extract_exact_u32().unwrap();

            let Some(width) = VectorSize::new(width) else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "width should be non-zero",
                );
                return Err(());
            };

            if width > src_ty.bit_length() {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_item_span(ident), "width <= src.size()");
                return Err(());
            }

            // We intentionally don't use slice_constant here as that might get optimized in the
            // future.
            let offset = builder.constant_u32(mctx.gl(), offset);
            Ok(Some((
                builder.slice(mctx.gl(), src, offset, width),
                VType::UnsignedNet(width),
            )))
        }
        _ => Ok(None),
    }
}

pub fn lower_unevaluated_system_function_call_ty<'a>(
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    ident: AstItem<SystemTaskIdentifier>,
    // arguments are in reverse order
    arguments: Option<AstIdRange<'a, Expr<'a>>>,
) -> Result<Option<VType>, ()> {
    macro_rules! ensure_num_args_equal {
        ($expected:expr) => {
            let num_args = arguments.map_or(0, |a| a.len());
            if num_args != $expected {
                diagnostics
                    .not_yet_implemented(arenas.get_span(expr), "intrinsic not expected amount");
                return Err(());
            }
        };
    }

    // @Performance: Use a perfect hashmap here.
    match &arenas.ident_table[ident.item.0] {
        "bits" => {
            ensure_num_args_equal!(1);
            Ok(Some(VType::UnsignedNet(INTEGER_VSIZE)))
        }
        "random" | "dist_uniform" | "dist_normal" | "dist_exponential" | "dist_poisson"
        | "dist_chi_square" | "dist_t" | "dist_erlang" => Ok(Some(VType::SignedNet(VSIZE_32))),
        "vogls_lupdt" => {
            ensure_num_args_equal!(1);
            Ok(Some(VType::UnsignedNet(TIME_VSIZE)))
        }
        "vogls_slice" => {
            ensure_num_args_equal!(3);
            let arguments = arguments.unwrap();
            let width = arguments.get(2);

            let Expr::Decimal(width) = &*width else {
                diagnostics.not_yet_implemented(
                    arenas.get_item_span(ident),
                    "slice snd argument should be decimal",
                );
                return Err(());
            };

            let width = arenas.decimals[width.at].extract_exact_u32().unwrap();

            let Some(width) = VectorSize::new(width) else {
                diagnostics
                    .not_yet_implemented(arenas.get_item_span(ident), "width should be non-zero");
                return Err(());
            };
            Ok(Some(VType::UnsignedNet(width)))
        }

        _ => Ok(None),
    }
}

pub fn eval_constant(
    arenas: &AstArenas,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    ident: AstItem<SystemTaskIdentifier>,
    arguments: &[Option<VValue>],
) -> Result<VValue, ()> {
    macro_rules! ensure_num_args_equal {
        ($expected:expr) => {
            let num_args = arguments.len();
            if num_args != $expected {
                diagnostics
                    .not_yet_implemented(arenas.get_span(expr), "intrinsic not expected amount");
                return Err(());
            }
        };
    }
    // Math functions accept any numeric argument; non-real operands are coerced
    // to real before the operation is applied.
    macro_rules! real_unary_const {
        ($method:ident) => {{
            ensure_num_args_equal!(1);
            let v = arguments[0].clone().ok_or(())?;
            let r = v.into_real().as_f64().$method();
            Ok(VValue::Real(Real::from_f64(r)))
        }};
    }
    macro_rules! real_binary_const {
        ($method:ident) => {{
            ensure_num_args_equal!(2);
            // Arguments are in reverse order.
            let lhs = arguments[1].clone().ok_or(())?.into_real().as_f64();
            let rhs = arguments[0].clone().ok_or(())?.into_real().as_f64();
            Ok(VValue::Real(Real::from_f64(lhs.$method(rhs))))
        }};
    }

    // @Performance: Use a perfect hashmap here.
    match &arenas.ident_table[ident.item.0] {
        "signed" => {
            ensure_num_args_equal!(1);
            diagnostics.not_yet_implemented(arenas.get_span(expr), "signed is not yet implemented");
            Err(())
        }
        "unsigned" => {
            ensure_num_args_equal!(1);
            diagnostics
                .not_yet_implemented(arenas.get_span(expr), "unsigned is not yet implemented");
            Err(())
        }
        "time" => {
            ensure_num_args_equal!(0);
            diagnostics.not_yet_implemented(arenas.get_span(expr), "time is not yet implemented");
            Err(())
        }
        "realtime" => {
            ensure_num_args_equal!(0);
            diagnostics
                .not_yet_implemented(arenas.get_span(expr), "realtime is not yet implemented");
            Err(())
        }
        "random" => {
            ensure_num_args_equal!(0);
            diagnostics.not_yet_implemented(
                arenas.get_span(expr),
                "random is not allow in constant expressions",
            );
            Err(())
        }
        "clog2" => {
            ensure_num_args_equal!(1);
            let v = arguments[0].as_ref().ok_or(())?;
            let clog2 = match v.clog2() {
                None => Bits::new_unknown(VSIZE_32),
                Some(value) => Bits::new_u32(value),
            };
            Ok(VValue::SignedNet(clog2))
        }

        // Math Functions
        "ln" => real_unary_const!(ln),
        "log10" => real_unary_const!(log10),
        "exp" => real_unary_const!(exp),
        "sqrt" => real_unary_const!(sqrt),
        "floor" => real_unary_const!(floor),
        "ceil" => real_unary_const!(ceil),
        "sin" => real_unary_const!(sin),
        "cos" => real_unary_const!(cos),
        "tan" => real_unary_const!(tan),
        "asin" => real_unary_const!(asin),
        "acos" => real_unary_const!(acos),
        "atan" => real_unary_const!(atan),
        "sinh" => real_unary_const!(sinh),
        "cosh" => real_unary_const!(cosh),
        "tanh" => real_unary_const!(tanh),
        "asinh" => real_unary_const!(asinh),
        "acosh" => real_unary_const!(acosh),
        "atanh" => real_unary_const!(atanh),
        "atan2" => real_binary_const!(atan2),
        "hypot" => real_binary_const!(hypot),

        "bitstoreal" => {
            ensure_num_args_equal!(1);
            let e = arguments[0].as_ref().ok_or(())?.clone();
            let e = e.coerce(&VType::UnsignedNet(VSIZE_64));
            let e = e.into_bits().extract_exact_u64().unwrap();
            Ok(VValue::Real(Real::from_f64(f64::from_bits(e))))
        }
        "realtobits" => {
            ensure_num_args_equal!(1);
            let e = arguments[0].as_ref().ok_or(())?.clone();
            let e = e.into_real();
            Ok(VValue::UnsignedNet(Bits::new_u64(e.0.to_bits())))
        }
        "itor" => {
            ensure_num_args_equal!(1);
            let e = arguments[0].as_ref().ok_or(())?.clone();
            Ok(VValue::Real(e.into_real()))
        }
        "rtoi" => {
            ensure_num_args_equal!(1);
            let e = arguments[0].as_ref().ok_or(())?.clone();
            let e = e.into_real();
            Ok(VValue::SignedNet(
                Bits::new_u64(e.0.trunc() as i64 as u64).truncate(INTEGER_VSIZE),
            ))
        }

        // VoGLS specific system function calls
        "vogls_dbg" => {
            ensure_num_args_equal!(1);
            diagnostics
                .not_yet_implemented(arenas.get_span(expr), "vogls_dbg is not yet implemented");
            Err(())
        }
        "vogls_rawticks" => {
            ensure_num_args_equal!(0);
            diagnostics.not_yet_implemented(arenas.get_span(expr), "vogls_rawticks is not yet implemented");
            Err(())
        }

        _ => {
            diagnostics.not_yet_implemented(arenas.get_span(expr), "unknown system function call");
            Err(())
        }
    }
}

fn get_prob_dist_fn_seed<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    ident: AstItem<SystemTaskIdentifier>,
    arguments: Option<AstIdRange<'a, Expr<'a>>>,
) -> Result<(SignalKey, SignalKey, VType), ()> {
    match arguments {
        Some(exprs) if exprs.len() == 1 => {
            get_prob_dist_fn_seed_signals(ctx, mctx, scope, ident, exprs.get(0))
        }
        _ => {
            let signal = mctx.gl.global_seed.get_or_insert_with(|| {
                mctx.gl.signals.insert(Signal {
                    name: "__VOGLS_RANDOM_SEED".to_string(),
                    size: VSIZE_32,
                    initialize: Some(Bits::new_zeroed(VSIZE_32)),
                    mode: LogicMode::TwoValue,
                    flags: SignalFlags::EMPTY,
                    origin: TokenRange::default(),
                })
            });
            Ok((*signal, *signal, VType::SignedNet(VSIZE_32)))
        }
    }
}

fn get_prob_dist_fn_seed_signals<'a, 'b>(
    ctx: &LowerContext<'a, 'b>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    ident: AstItem<SystemTaskIdentifier>,
    expr: AstId<'a, Expr<'a>>,
) -> Result<(SignalKey, SignalKey, VType), ()> {
    let Expr::Ident(arg_ident, array_exprs, bitslice) = &*expr else {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_item_span(ident),
            "random expects an identifier",
        );
        return Err(());
    };

    if !array_exprs.is_empty() || bitslice.is_some() {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_item_span(ident),
            "random expects an identifier",
        );
        return Err(());
    }

    let net_symbol = try_resolve_net(
        scope,
        &ctx.table,
        ctx.arenas,
        *arg_ident,
        &mut mctx.diagnostics,
    )?;
    Ok((
        net_symbol.net.probe_signal(),
        net_symbol.net.blocking_drive_signal(),
        net_symbol.ty,
    ))
}
