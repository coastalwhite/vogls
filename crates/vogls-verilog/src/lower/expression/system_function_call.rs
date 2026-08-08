use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::dyn_format_string::{DynFormatArgument, DynFormatString};
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, Bits, INTEGER_VSIZE, IntrinsicOp, LogicMode, RandomKind, SCALAR_VSIZE,
    Signal, SignalFlags, SignalKey, TIME_VSIZE, VSIZE_32, VariableKey, VectorSize,
};

use crate::ast::expr::Expr;
use crate::ast::statement::SystemTaskIdentifier;
use crate::ast::{AstId, AstIdRange, AstItem};
use crate::lower::expression::{
    coerce_to_max_size_ty, lower_expr, sign_or_zero_extend, truncate_or_extend,
};
use crate::lower::vvalue::VValue;
use crate::lower::{Diagnostics, LowerContext, MutLowerContext, VType, try_resolve_net};
use crate::parser::AstArenas;

use super::get_expr_type;

pub fn lower_system_function_call(
    arenas: &AstArenas,
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
                mctx.diagnostics
                    .not_yet_implemented(arenas.get_span(expr), "intrinsic not expected amount");
                return Err(());
            }
        };
    }

    // @Performance: Use a perfect hashmap here.
    match &arenas.ident_table[ident.item.0] {
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
            Ok((builder.time(mctx.gl()), VType::UnsignedNet(TIME_VSIZE)))
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
            let clog2 = builder.revminus_constant(
                mctx.gl(),
                lz,
                Bits::new_u32(e_ty.bit_length().get()),
            );
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

        // VoGLS specific system function calls
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
                    arenas.get_span(expr),
                    "select condition has to be scalar",
                );
                return Err(());
            }

            let ty = coerce_to_max_size_ty(truthy_ty, falsy_ty);
            let truthy =
                sign_or_zero_extend(mctx.gl(), builder, truthy, truthy_ty, ty.bit_length());
            let falsy =
                sign_or_zero_extend(mctx.gl(), builder, falsy, falsy_ty, ty.bit_length());
            let e = builder.select(mctx.gl(), cond, truthy, falsy);
            Ok((e, ty))
        }

        _ => {
            mctx.diagnostics
                .not_yet_implemented(arenas.get_span(expr), "unknown system function call");
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
        "clog2" => Ok(VType::UnsignedNet(VSIZE_32)),

        // VoGLS specific system function calls
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

            let Expr::Sized(sized) = &*src else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "slice first argument should be sized",
                );
                return Err(());
            };
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

            let sized = &ctx.arenas.sized_numbers[sized.item.at];
            let src = sized.value.clone();
            let offset = ctx.arenas.decimals[offset.at].extract_exact_u32().unwrap();
            let width = ctx.arenas.decimals[width.at].extract_exact_u32().unwrap();

            let Some(width) = VectorSize::new(width) else {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_item_span(ident),
                    "width should be non-zero",
                );
                return Err(());
            };

            if width > src.size() {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_item_span(ident), "width <= src.size()");
                return Err(());
            }

            // We intentionally don't use slice_constant here as that might get optimized in the
            // future.
            let src = builder.constant(mctx.gl(), src);
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

        // VoGLS specific system function calls
        "vogls_dbg" => {
            ensure_num_args_equal!(1);
            diagnostics
                .not_yet_implemented(arenas.get_span(expr), "vogls_dbg is not yet implemented");
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
