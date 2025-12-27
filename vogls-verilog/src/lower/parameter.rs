use vogls_ir::{GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE};

use crate::ast::AstId;
use crate::ast::module::ParameterDeclarationTyping;
use crate::parser::AstArenas;

use super::scope::Scope;
use super::{Diagnostics, VType, range_to_width};

pub fn parameter_typing_to_type<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope,
    diagnostics: &mut Diagnostics,
    typing: AstId<ParameterDeclarationTyping>,
) -> Result<VType, ()> {
    Ok(match arenas.get(typing) {
        ParameterDeclarationTyping::None(signed, range) => {
            let width = match range {
                Some(range) => range_to_width(gl, arenas, scope, diagnostics, *range)?,
                None => SCALAR_VSIZE,
            };
            VType::net(width, *signed)
        }
        ParameterDeclarationTyping::Integer => VType::SignedNet(INTEGER_VSIZE),
        ParameterDeclarationTyping::Real
        | ParameterDeclarationTyping::Realtime
        | ParameterDeclarationTyping::Time => {
            diagnostics
                .not_yet_implemented(arenas.get_span(typing), "real / realtime / time parameter");
            return Err(());
        }
    })
}
