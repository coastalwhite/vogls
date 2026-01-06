use vogls_ir::{GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE};

use crate::ast::constant_expr::ConstantMinTypMaxExpression;
use crate::ast::AstId;
use crate::ast::module::{LocalParameterDeclaration, ParamAssignment, ParameterDeclarationTyping};
use crate::lower::expression::eval_constant_expr;
use crate::lower::scope::{Symbol, SymbolVariant};
use crate::parser::AstArenas;

use super::scope::Scope;
use super::{Diagnostics, VType, evaluate_range};

pub fn parameter_typing_to_type<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope,
    diagnostics: &mut Diagnostics,
    typing: AstId<ParameterDeclarationTyping>,
) -> Result<(i64, i64, VType), ()> {
    Ok(match arenas.get(typing) {
        ParameterDeclarationTyping::None(signed, range) => {
            let (msb, lsb, width) = match range {
                None => (0, 0, SCALAR_VSIZE),
                Some(range) => evaluate_range(gl, arenas, scope, diagnostics, *range)?,
            };
            (msb, lsb, VType::net(width, *signed))
        }
        ParameterDeclarationTyping::Integer => (31, 0, VType::SignedNet(INTEGER_VSIZE)),
        ParameterDeclarationTyping::Real
        | ParameterDeclarationTyping::Realtime
        | ParameterDeclarationTyping::Time => {
            diagnostics
                .not_yet_implemented(arenas.get_span(typing), "real / realtime / time parameter");
            return Err(());
        }
    })
}

pub fn push_localparameter_into_scope<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    id: AstId<LocalParameterDeclaration>,
) -> Result<(), ()> {
    let LocalParameterDeclaration {
        typing,
        assignments,
    } = arenas.get(id);

    // @FIXME: Coerce value to ty.
    let _ty = parameter_typing_to_type(gl, arenas, scope, diagnostics, *typing)?;
    for assignment in assignments.iter() {
        let ParamAssignment { param, constant } = arenas.get(assignment);
        let key = arenas.get_ident(param.item.0);
        let value = arenas.get(*constant);
        match value {
            ConstantMinTypMaxExpression::Single(id) => {
                let value = eval_constant_expr(gl, arenas, &scope, diagnostics, *id)?;
                let symbol_key = scope.symbols.insert(Symbol {
                    name: key.to_string(),
                    definition_site: arenas.get_item_span(*param),
                    variant: SymbolVariant::Constant(value.clone()),
                });
                scope.push(key, symbol_key);
            }
            ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
        }
    }
    Ok(())
}
