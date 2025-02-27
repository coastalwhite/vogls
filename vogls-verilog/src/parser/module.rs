use crate::ast::module::{AlwaysConstruct, InitialConstruct, Module, ModuleOrGenerateItem, NonPortModuleItem};
use crate::ast::statement::Statement;
use crate::ast::IdentRef;
use crate::lexer::TokenKind;
use crate::parser::ItemParsable;
use crate::span::Span;

use super::{AstArenas, Consumable, Parsable, ParseError, Parser};

impl<'a> Consumable<'a> for Module {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 487
        // module_declaration ::=
        // { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ]
        // list_of_ports ; { module_item }
        // endmodule
        // | { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ]
        // [ list_of_port_declarations ] ; { non_port_module_item }
        // endmodule

        // @Incomplete: { attribute_instance }
        let module_kw_span = p.lexer().expect(TK::KeywordModule)?.span();
        let module_identifier = IdentRef::parse(p, arenas)?;
        // @Incomplete: [ module_parameter_port_list ]
        // @Incomplete: list_of_ports | [ list_of_port_declarations ]
        p.lexer().expect(TK::Semicolon)?;
        // @Incomplete: | module_item
        let (module_items, endmodule_kw_token) =
            NonPortModuleItem::parse_until_reaching(p, arenas, TK::KeywordEndModule)?;

        let span = module_kw_span | endmodule_kw_token.span();

        Ok((
            Module {
                module_identifier,
                module_items,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for Module {}

impl<'a> Consumable<'a> for NonPortModuleItem {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // non_port_module_item ::=
        // module_or_generate_item
        // | generate_region
        // | specify_block
        // | { attribute_instance } parameter_declaration ;
        // | { attribute_instance } specparam_declaration
        
        if let Some((module_or_generate_item, span)) = ModuleOrGenerateItem::try_parse_with_span(p, arenas) {
            return Ok((Self::ModuleOrGenerateItem(module_or_generate_item), span));
        }

        // @Incomplete
        Err(ParseError::Incomplete("non_port_module_item"))
    }
}
impl<'a> Parsable<'a> for NonPortModuleItem {}

impl<'a> Consumable<'a> for ModuleOrGenerateItem {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // module_or_generate_item ::=
        // { attribute_instance } module_or_generate_item_declaration
        // | { attribute_instance } local_parameter_declaration ;
        // | { attribute_instance } parameter_override
        // | { attribute_instance } continuous_assign
        // | { attribute_instance } gate_instantiation
        // | { attribute_instance } udp_instantiation
        // | { attribute_instance } module_instantiation
        // | { attribute_instance } initial_construct
        // | { attribute_instance } always_construct
        // | { attribute_instance } loop_generate_construct
        // | { attribute_instance } conditional_generate_construct
        
        // @Incomplete: { attribute_instance }
        // @Incomplete

        let peeked = p.lexer().next_expect_peek()?;
        match peeked.kind() {
            TK::KeywordInitial => {
                peeked.release();
                let (initial_construct, span) = InitialConstruct::parse_with_span(p, arenas)?;
                Ok((Self::InitialConstruct(initial_construct), span))
            }
            TK::KeywordAlways => {
                peeked.release();
                let (always_construct, span) = AlwaysConstruct::parse_with_span(p, arenas)?;
                Ok((Self::AlwaysConstruct(always_construct), span))
            }
            _ => {
                Err(ParseError::Incomplete("module_or_generate_item"))
            }
        }
    }
}
impl<'a> Parsable<'a> for ModuleOrGenerateItem {}

impl<'a> Consumable<'a> for InitialConstruct {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // initial_construct ::= initial statement
        
        let initial_kw_span = p.lexer().expect(TK::KeywordInitial)?.span();
        let (statement, span) = Statement::parse_with_span(p, arenas)?;

        let span = initial_kw_span | span;

        Ok((Self(statement), span))
    }
}
impl<'a> Parsable<'a> for InitialConstruct {}

impl<'a> Consumable<'a> for AlwaysConstruct {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // always_construct ::= always statement
        
        let always_kw_span = p.lexer().expect(TK::KeywordAlways)?.span();
        let (statement, span) = Statement::parse_with_span(p, arenas)?;

        let span = always_kw_span | span;

        Ok((Self(statement), span))
    }
}
impl<'a> Parsable<'a> for AlwaysConstruct {}
