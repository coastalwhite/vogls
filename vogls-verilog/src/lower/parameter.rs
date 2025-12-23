use vogls_ir::GlobalContext;

use crate::ast::AstId;
use crate::ast::module::ParameterDeclarationTyping;
use crate::parser::AstArenas;

use super::scope::Scope;
use super::{Diagnostics, VType, VTypeKey, VTypeTable, range_to_width};

pub fn parameter_typing_to_type<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope,
    diagnostics: &mut Diagnostics,
    typing: AstId<ParameterDeclarationTyping>,
) -> Result<VTypeKey, ()> {
    Ok(match arenas.get(typing) {
        ParameterDeclarationTyping::None(_, range) => match range {
            Some(range) => {
                let width = range_to_width(gl, arenas, types, scope, diagnostics, *range)?;
                types.insert(VType::VectorNet(width))
            }
            None => types.scalar_net(),
        },
        ParameterDeclarationTyping::Integer => types.integer(),
        ParameterDeclarationTyping::Real
        | ParameterDeclarationTyping::Realtime
        | ParameterDeclarationTyping::Time => {
            diagnostics
                .not_yet_implemented(arenas.get_span(typing), "real / realtime / time parameter");
            return Err(());
        }
    })
}
