use crate::ast::module::{
    AlwaysConstruct, InitialConstruct, InoutDeclaration, InputDeclaration, ListOfPortConnections,
    Module, ModuleInstance, ModuleInstantiation, ModuleOrGenerateItem, NetType, NonPortModuleItem,
    OutputDeclaration, OutputNet, PortDeclaration,
};
use crate::ast::statement::Statement;
use crate::ast::{AstItem, Identifier};
use crate::lexer::{FromLexerError, Token, TokenKind};
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
        let module_identifier = Identifier::parse(p, arenas)?;
        // @Incomplete: [ module_parameter_port_list ]
        // @Incomplete:  | list_of_ports
        let port_declarations = if p.lexer.next_if_equals(TK::LeftParen).is_some() {
            let port_declarations =
                PortDeclaration::parse_zero_or_more_delimited(p, arenas, TK::Comma)?;
            p.lexer.expect(TK::RightParen)?;

            port_declarations
        } else {
            Default::default()
        };
        p.lexer().expect(TK::Semicolon)?;
        // @Incomplete: | module_item
        let (module_items, endmodule_kw_token) =
            NonPortModuleItem::parse_until_reaching(p, arenas, TK::KeywordEndModule)?;

        let span = module_kw_span | endmodule_kw_token.span();

        Ok((
            Module {
                module_identifier,
                port_declarations,
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

        let (module_or_generate_item, span) = ModuleOrGenerateItem::parse_with_span(p, arenas)?;
        Ok((Self::ModuleOrGenerateItem(module_or_generate_item), span))

        // @Incomplete: | generate_region
        // @Incomplete: | specify_block
        // @Incomplete: | { attribute_instance } parameter_declaration ;
        // @Incomplete: | { attribute_instance } specparam_declaration
    }
}
impl<'a> Parsable<'a> for NonPortModuleItem {}

impl<'a> Consumable<'a> for PortDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_declaration ::=
        //   {attribute_instance} inout_declaration
        // | {attribute_instance} input_declaration
        // | {attribute_instance} output_declaration

        let peeked = p.lexer().next_expect_peek()?;
        match peeked.kind() {
            TK::KeywordInout => {
                peeked.release();
                let (inout_declaration, span) = InoutDeclaration::parse_with_span(p, arenas)?;
                Ok((Self::Inout(inout_declaration), span))
            }
            TK::KeywordInput => {
                peeked.release();
                let (input_declaration, span) = InputDeclaration::parse_with_span(p, arenas)?;
                Ok((Self::Input(input_declaration), span))
            }
            TK::KeywordOutput => {
                peeked.release();
                let (output_declaration, span) = OutputDeclaration::parse_with_span(p, arenas)?;
                Ok((Self::Output(output_declaration), span))
            }
            _ => Err(ParseError::unexpected_token(peeked.commit())),
        }
    }
}
impl<'a> Parsable<'a> for PortDeclaration {}

impl<'a> Consumable<'a> for InoutDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // inout_declaration ::= inout [ net_type ] [ signed ] [ range ] list_of_port_identifiers

        let inout_kw_span = p.lexer.expect(TK::KeywordInout)?.span();
        let mut net_type = None;
        let mut end_span = inout_kw_span;
        if let Some((val, span)) = NetType::try_parse_with_span(p, arenas) {
            net_type = Some(val);
            end_span = span;
        }
        let signed_token = p.lexer.next_if_equals(TK::KeywordSigned);
        let signed = signed_token.is_some();
        if let Some(signed_token) = signed_token {
            end_span = signed_token.span();
        }
        // @Incomplete: [ range ]
        let port_identifiers = Identifier::parse_zero_or_more_delimited(p, arenas, TK::Comma)?;
        if let Some(last) = port_identifiers.last() {
            end_span = *arenas.spans.get(last.loc).unwrap();
        }

        let span = inout_kw_span | end_span;
        Ok((
            Self {
                net_type,
                signed,
                range: None,
                port_identifiers,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for InoutDeclaration {}

impl<'a> Consumable<'a> for InputDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // input_declaration ::= input [ net_type ] [ signed ] [ range ] list_of_port_identifiers

        let input_kw_span = p.lexer.expect(TK::KeywordInput)?.span();
        let mut net_type = None;
        let mut end_span = input_kw_span;
        if let Some((val, span)) = NetType::try_parse_with_span(p, arenas) {
            net_type = Some(val);
            end_span = span;
        }
        let signed_token = p.lexer.next_if_equals(TK::KeywordSigned);
        let signed = signed_token.is_some();
        if let Some(signed_token) = signed_token {
            end_span = signed_token.span();
        }
        // @Incomplete: [ range ]
        let port_identifiers = Identifier::parse_zero_or_more_delimited(p, arenas, TK::Comma)?;
        if let Some(last) = port_identifiers.last() {
            end_span = *arenas.spans.get(last.loc).unwrap();
        }

        let span = input_kw_span | end_span;
        Ok((
            Self {
                net_type,
                signed,
                range: None,
                port_identifiers,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for InputDeclaration {}

impl<'a> Consumable<'a> for OutputDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // output_declaration ::=
        //   output [ net_type ] [ signed ] [ range ] list_of_port_identifiers
        // | output reg [ signed ] [ range ] list_of_variable_port_identifiers
        // | output output_variable_type list_of_variable_port_identifiers

        let output_kw_span = p.lexer.expect(TK::KeywordOutput)?.span();
        let mut net_type = None;
        let mut end_span = output_kw_span;
        if let Some((val, span)) = NetType::try_parse_with_span(p, arenas) {
            net_type = Some(val);
            end_span = span;
        }
        let signed_token = p.lexer.next_if_equals(TK::KeywordSigned);
        let signed = signed_token.is_some();
        if let Some(signed_token) = signed_token {
            end_span = signed_token.span();
        }
        // @Incomplete: reg | output_variable_type
        // @Incomplete: [ range ]
        let identifiers = Identifier::parse_zero_or_more_delimited(p, arenas, TK::Comma)?;
        if let Some(last) = identifiers.last() {
            end_span = *arenas.spans.get(last.loc).unwrap();
        }

        let span = output_kw_span | end_span;
        Ok((
            Self {
                net: net_type,
                signed,
                range: None,
                identifiers,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for OutputDeclaration {}

impl<'a> Consumable<'a> for NetType {
    fn consume(p: &mut Parser<'a>, _arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // net_type ::=
        //   supply0 | supply1
        // | tri
        // | triand | trior | tri0 | tri1
        // | uwire | wire | wand | wor

        p.lexer.expect_map(|content, span| match content.kind() {
            TK::KeywordSupply0 => Ok(Self::Supply0),
            TK::KeywordSupply1 => Ok(Self::Supply1),
            TK::KeywordTri => Ok(Self::Tri),
            TK::KeywordTriand => Ok(Self::TriAnd),
            TK::KeywordTrior => Ok(Self::TriOr),
            TK::KeywordTri0 => Ok(Self::Tri0),
            TK::KeywordUwire => Ok(Self::Uwire),
            TK::KeywordWire => Ok(Self::Wire),
            TK::KeywordWand => Ok(Self::WAnd),
            TK::KeywordWor => Ok(Self::WOr),
            _ => Err(ParseError::unexpected_token(Token::new(content, span))),
        })
    }
}
impl<'a> ItemParsable<'a> for NetType {
    type Item = NetType;
    fn from_item(item: Self::Item, _arenas: &mut AstArenas) -> Result<Self, ParseError> {
        Ok(item)
    }
}

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
            TK::Ident => {
                peeked.release();
                let (module_instance, span) = ModuleInstantiation::parse_with_span(p, arenas)?;
                Ok((Self::ModuleInstantiation(module_instance), span))
            }
            _ => Err(ParseError::incomplete(
                Some(peeked.commit().span()),
                "module_or_generate_item",
            )),
        }
    }
}
impl<'a> Parsable<'a> for ModuleOrGenerateItem {}

impl<'a> Consumable<'a> for ModuleInstantiation {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // module_instantiation ::=
        //   module_identifier [ parameter_value_assignment ]
        //   module_instance { , module_instance } ;

        let (module_identifier, module_identifier_span) = Identifier::parse_with_span(p, arenas)?;
        let module_instances = ModuleInstance::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
        let semicolon_span = p.lexer().expect(TK::Semicolon)?.span();

        let span = module_identifier_span | semicolon_span;

        Ok((
            ModuleInstantiation {
                module_identifier,
                module_instances,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for ModuleInstantiation {}

impl<'a> Consumable<'a> for ModuleInstance {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // module_instance ::= name_of_module_instance ( [ list_of_port_connections ] )

        let (name_of_module_instance, name_of_module_instance_span) =
            Identifier::parse_with_span(p, arenas)?;
        p.lexer().expect(TK::LeftParen)?;
        let list_of_port_connections = ListOfPortConnections::parse(p, arenas)?;
        let right_paren_span = p.lexer().expect(TK::RightParen)?.span();

        let span = name_of_module_instance_span | right_paren_span;

        Ok((
            ModuleInstance {
                name_of_module_instance,
                list_of_port_connections,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for ModuleInstance {}

impl<'a> Consumable<'a> for ListOfPortConnections {
    fn consume(p: &mut Parser<'a>, _arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // @TODO

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // list_of_port_connections ::=
        //   ordered_port_connection { , ordered_port_connection }
        // | named_port_connection { , named_port_connection }

        let span = p.lexer().span_at_cursor();
        Ok((Self::Named, span))
    }
}
impl<'a> Parsable<'a> for ListOfPortConnections {}

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
