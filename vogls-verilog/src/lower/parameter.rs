use vogls_ir::GlobalContext;

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
        ParameterDeclarationTyping::None(_, range) => match range {
            Some(range) => {
                let width = range_to_width(gl, arenas, scope, diagnostics, *range)?;
                VType::Net(width)
            }
            None => VType::SCALAR_NET,
        },
        ParameterDeclarationTyping::Integer => VType::Integer,
        ParameterDeclarationTyping::Real
        | ParameterDeclarationTyping::Realtime
        | ParameterDeclarationTyping::Time => {
            diagnostics
                .not_yet_implemented(arenas.get_span(typing), "real / realtime / time parameter");
            return Err(());
        }
    })
}
