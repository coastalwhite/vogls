use vogls_ir::{INTEGER_VSIZE, SCALAR_VSIZE};

use crate::ast::AstId;
use crate::ast::module::ParameterDeclarationTyping;
use crate::parser::AstArenas;

use super::Scope;
use super::{Diagnostics, VType, evaluate_range};

pub fn parameter_typing_to_type<'a>(
    arenas: &'a AstArenas,
    scope: &mut Scope,
    diagnostics: &mut Diagnostics,
    typing: AstId<ParameterDeclarationTyping>,
) -> Result<(i64, i64, VType), ()> {
    Ok(match arenas.get(typing) {
        ParameterDeclarationTyping::None(signed, range) => {
            let (msb, lsb, width) = match range {
                None => (0, 0, SCALAR_VSIZE),
                Some(range) => evaluate_range(arenas, scope.eval(), diagnostics, *range)?,
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
