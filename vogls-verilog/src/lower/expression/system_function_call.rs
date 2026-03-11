use vogls_ir::dyn_format_string::{DynFormatArgument, DynFormatString};
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, INTEGER_VSIZE, IntrinsicOp, TIME_VSIZE, VariableKey,
};

use crate::ast::expr::Expr;
use crate::ast::statement::SystemTaskIdentifier;
use crate::ast::{AstId, AstIdRange, AstItem};
use crate::lower::expression::{coerce_to_max_size_ty, sign_or_zero_extend};
use crate::lower::vvalue::VValue;
use crate::lower::{Diagnostics, Scope, VType, try_resolve_net};
use crate::parser::AstArenas;

pub fn lower_system_function_call<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
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
            Ok((builder.time(gl), VType::UnsignedNet(TIME_VSIZE)))
        }
        "random" => {
            ensure_num_args_equal!(0);
            Ok((builder.random(gl), VType::UnsignedNet(TIME_VSIZE)))
        }
        "clog2" => {
            diagnostics.not_yet_implemented(arenas.get_span(expr), "clog2 is not yet implemented");
            Err(())
        }

        // VoGLS specific system function calls
        "vogls_dbg" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            let format_str =
                DynFormatString::new("\n".into(), [(0, DynFormatArgument::default())].into());
            builder.intrinsic(gl, IntrinsicOp::Display(Box::new(format_str)), [e].into());
            Ok((e, e_ty))
        }
        "vogls_copyx" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let ty = coerce_to_max_size_ty(l_ty, r_ty);
            let l = sign_or_zero_extend(gl, builder, l, l_ty, ty.force_net_width());
            let r = sign_or_zero_extend(gl, builder, r, r_ty, ty.force_net_width());
            let e = builder.copy_x(gl, l, r);
            Ok((e, ty))
        }
        "vogls_copyz" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let ty = coerce_to_max_size_ty(l_ty, r_ty);
            let l = sign_or_zero_extend(gl, builder, l, l_ty, ty.force_net_width());
            let r = sign_or_zero_extend(gl, builder, r, r_ty, ty.force_net_width());
            let e = builder.copy_z(gl, l, r);
            Ok((e, ty))
        }
        "vogls_min" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let ty = coerce_to_max_size_ty(l_ty, r_ty);
            let l = sign_or_zero_extend(gl, builder, l, l_ty, ty.force_net_width());
            let r = sign_or_zero_extend(gl, builder, r, r_ty, ty.force_net_width());
            let e = builder.min(gl, l, r);
            Ok((e, ty))
        }
        "vogls_max" => {
            ensure_num_args_equal!(2);
            let (l, l_ty) = arguments[1].ok_or(())?;
            let (r, r_ty) = arguments[0].ok_or(())?;

            let ty = coerce_to_max_size_ty(l_ty, r_ty);
            let l = sign_or_zero_extend(gl, builder, l, l_ty, ty.force_net_width());
            let r = sign_or_zero_extend(gl, builder, r, r_ty, ty.force_net_width());
            let e = builder.max(gl, l, r);
            Ok((e, ty))
        }

        _ => {
            diagnostics.not_yet_implemented(arenas.get_span(expr), "unknown system function call");
            Err(())
        }
    }
}

pub fn lower_unevaluated_system_function_call<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    scope: &Scope,
    ident: AstItem<SystemTaskIdentifier>,
    arguments: Option<AstIdRange<Expr>>,
) -> Result<Option<(VariableKey, VType)>, ()> {
    match &arenas.ident_table[ident.item.0] {
        "vogls_lupdt" => {
            let Some(arguments) = arguments else {
                diagnostics.not_yet_implemented(
                    arenas.get_item_span(ident),
                    "last update time requires one argument",
                );
                return Err(());
            };

            let Some(expr) = arguments.first().filter(|_| arguments.len() == 1) else {
                diagnostics.not_yet_implemented(
                    arenas.get_item_span(ident),
                    "last update time requires one argument",
                );
                return Err(());
            };

            let Expr::Ident(arg_ident, array_exprs, bitslice) = arenas.get(expr) else {
                diagnostics.not_yet_implemented(
                    arenas.get_item_span(ident),
                    "last update time expects an identifier",
                );
                return Err(());
            };

            if !array_exprs.is_empty() || bitslice.is_some() {
                diagnostics.not_yet_implemented(
                    arenas.get_item_span(ident),
                    "last update time expects an identifier",
                );
                return Err(());
            }

            let net_symbol =
                try_resolve_net(scope.key, scope.table, arenas, *arg_ident, diagnostics)?;
            Ok(Some((
                builder.lupdt(gl, net_symbol.signal),
                VType::UnsignedNet(TIME_VSIZE),
            )))
        }
        _ => Ok(None),
    }
}

pub fn eval_constant<'a>(
    arenas: &'a AstArenas,
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
            let clog2 = v.clog2();
            Ok(VValue::SignedNet(
                Bits::new_u32(clog2.into()).truncate_or_zero_extend(INTEGER_VSIZE),
            ))
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
