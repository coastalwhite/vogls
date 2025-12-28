use vogls_ir::{
    BasicBlockBuilder, GlobalContext, IntrinsicArg, IntrinsicOp, TIME_VSIZE, VariableKey,
};

use crate::ast::expr::Expr;
use crate::ast::statement::SystemTaskIdentifier;
use crate::ast::{AstId, AstItem};
use crate::lower::{Diagnostics, VType};
use crate::parser::AstArenas;

pub fn lower_system_function_call<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    expr: AstId<Expr>,
    ident: AstItem<SystemTaskIdentifier>,
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
    match arenas.get_ident(ident.item.0) {
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
        "clog2" => {
            diagnostics.not_yet_implemented(arenas.get_span(expr), "clog2 is not yet implemented");
            Err(())
        }

        // VoGLS specific system function calls
        "vogls_dbg" => {
            ensure_num_args_equal!(1);
            let (e, e_ty) = arguments[0].ok_or(())?;
            builder.intrinsic(gl, IntrinsicOp::Display, vec![IntrinsicArg::Variable(e)]);
            Ok((e, e_ty))
        }

        _ => {
            diagnostics.not_yet_implemented(arenas.get_span(expr), "unknown system function call");
            Err(())
        }
    }
}
