use crate::ast::constant_expr::ConstantMinTypMaxExpression;
use crate::ast::expr::Expr;
use crate::ast::module::{
    AlwaysConstruct, GateInstantiation, InitialConstruct, InoutDeclaration, InputDeclaration,
    ListOfPortConnections, Module, ModuleInstance, ModuleInstantiation, ModuleItem,
    ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration, ModulePorts, NInputGateInstance,
    NInputGateInstantiation, NInputGateType, NameOfGateInstance, NetDeclaration, NetType,
    NonPortModuleItem, OutputDeclaration, OutputNet, ParamAssignment, ParameterDeclaration, Port,
    PortDeclaration, PortExpression, PortReference, RegDeclaration,
};
use crate::ast::statement::{NetLValue, Statement};
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
        let ports = if p.lexer.next_if_equals(TK::LeftParen).is_some() {
            let peeked = p.lexer().next_expect_peek()?;
            match peeked.kind() {
                TK::RightParen => {
                    peeked.commit();
                    ModulePorts::PortDeclarations(Default::default())
                }
                TK::KeywordInput | TK::KeywordOutput | TK::KeywordInout => {
                    peeked.release();
                    let port_declarations =
                        PortDeclaration::parse_zero_or_more_delimited(p, arenas, TK::Comma)?;
                    p.lexer.expect(TK::RightParen)?;

                    ModulePorts::PortDeclarations(port_declarations)
                }
                _ => {
                    peeked.release();
                    let ports = Port::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
                    p.lexer.expect(TK::RightParen)?;

                    ModulePorts::Ports(ports)
                }
            }
        } else {
            ModulePorts::PortDeclarations(Default::default())
        };
        p.lexer().expect(TK::Semicolon)?;
        let (module_items, endmodule_kw_token) =
            ModuleItem::parse_until_reaching(p, arenas, TK::KeywordEndModule)?;

        let span = module_kw_span | endmodule_kw_token.span();

        Ok((
            Module {
                module_identifier,
                ports,
                module_items,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for Module {}

impl<'a> Consumable<'a> for ModuleItem {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // module_item ::=
        //   port_declaration ;
        // | non_port_module_item
        let peeked = p.lexer().next_expect_peek()?;
        match peeked.kind() {
            TK::KeywordInput | TK::KeywordOutput | TK::KeywordInout => {
                peeked.release();
                let (port_declaration, span) = PortDeclaration::parse_with_span(p, arenas)?;
                p.lexer.expect(TK::Semicolon)?;
                Ok((Self::PortDeclaration(port_declaration), span))
            }
            _ => {
                peeked.release();
                let (non_port_module_item, span) = NonPortModuleItem::parse_with_span(p, arenas)?;
                Ok((Self::NonPortModuleItem(non_port_module_item), span))
            }
        }
    }
}
impl<'a> Parsable<'a> for ModuleItem {}

impl<'a> Consumable<'a> for NonPortModuleItem {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // non_port_module_item ::=
        // module_or_generate_item
        // | generate_region
        // | specify_block
        // | { attribute_instance } parameter_declaration ;
        // | { attribute_instance } specparam_declaration

        let peeked = p.lexer.next_expect_peek()?;
        match peeked.kind() {
            // @Incomplete: | generate_region
            // @Incomplete: | specify_block
            // @Incomplete: | { attribute_instance } specparam_declaration
            TK::KeywordParameter => {
                peeked.release();
                let (parameter_declaration, span) =
                    ParameterDeclaration::parse_with_span(p, arenas)?;
                p.lexer.expect(TK::Semicolon)?;
                Ok((Self::ParameterDeclaration(parameter_declaration), span))
            }
            _ => {
                peeked.release();
                let (module_or_generate_item, span) =
                    ModuleOrGenerateItem::parse_with_span(p, arenas)?;
                Ok((Self::ModuleOrGenerateItem(module_or_generate_item), span))
            }
        }
    }
}
impl<'a> Parsable<'a> for NonPortModuleItem {}

impl<'a> Consumable<'a> for Port {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port ::=
        //   [ port_expression ]
        // | . port_identifier ( [ port_expression ] )

        // @Incomplete: . port_identifier ( [ port_expression ] )

        let (port_expression, span) = PortExpression::parse_with_span(p, arenas)?;
        Ok((Self::PortExpression(port_expression), span))
    }
}
impl<'a> Parsable<'a> for Port {}

impl<'a> Consumable<'a> for PortExpression {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_expression ::=
        //   port_reference
        // | { port_reference { , port_reference } }

        // @Incomplete: { port_reference { , port_reference } }

        let (port_reference, span) = PortReference::parse_with_span(p, arenas)?;
        Ok((
            Self {
                references: port_reference,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for PortExpression {}

impl<'a> Consumable<'a> for PortReference {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_reference ::=
        //   port_identifier [ [ constant_range_expression ] ]

        // @Incomplete: [ [ constant_range_expression ] ]

        let (identifier, span) = Identifier::parse_with_span(p, arenas)?;
        Ok((Self { identifier }, span))
    }
}
impl<'a> Parsable<'a> for PortReference {}

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
        let port_identifiers = Identifier::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
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
        let port_identifiers = Identifier::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
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
        let identifiers = Identifier::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
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
            TK::KeywordAnd
            | TK::KeywordNand
            | TK::KeywordOr
            | TK::KeywordNor
            | TK::KeywordXor
            | TK::KeywordXnor => {
                peeked.release();
                let (gate_instance, span) = GateInstantiation::parse_with_span(p, arenas)?;
                Ok((Self::GateInstantiation(gate_instance), span))
            }
            TK::KeywordSupply0
            | TK::KeywordSupply1
            | TK::KeywordTri
            | TK::KeywordTriand
            | TK::KeywordTrior
            | TK::KeywordTri0
            | TK::KeywordUwire
            | TK::KeywordWire
            | TK::KeywordWand
            | TK::KeywordWor
            | TK::KeywordReg => {
                peeked.release();
                let (module_or_generate_item_declaration, span) =
                    ModuleOrGenerateItemDeclaration::parse_with_span(p, arenas)?;
                Ok((
                    Self::ModuleOrGenerateItemDeclaration(module_or_generate_item_declaration),
                    span,
                ))
            }
            _ => {
                let token = peeked.commit();
                dbg!(token.content());
                Err(ParseError::incomplete(
                    Some(token.span()),
                    "module_or_generate_item",
                ))
            }
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
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // list_of_port_connections ::=
        //   ordered_port_connection { , ordered_port_connection }
        // | named_port_connection { , named_port_connection }
        
        let ordered = Expr::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
        let span = arenas.spans[ordered.loc];

        Ok((Self::Ordered(ordered), span))
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

impl<'a> Consumable<'a> for GateInstantiation {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // gate_instantiation ::=
        //   cmos_switchtype [delay3] cmos_switch_instance { , cmos_switch_instance } ;
        // | enable_gatetype [drive_strength] [delay3] enable_gate_instance { , enable_gate_instance } ;
        // | mos_switchtype [delay3] mos_switch_instance { , mos_switch_instance } ;
        // | n_input_gatetype [drive_strength] [delay2] n_input_gate_instance { , n_input_gate_instance } ;
        // | n_output_gatetype [drive_strength] [delay2] n_output_gate_instance { , n_output_gate_instance } ;
        // | pass_en_switchtype [delay2] pass_enable_switch_instance { , pass_enable_switch_instance } ;
        // | pass_switchtype pass_switch_instance { , pass_switch_instance } ;
        // | pulldown [pulldown_strength] pull_gate_instance { , pull_gate_instance } ;
        // | pullup [pullup_strength] pull_gate_instance { , pull_gate_instance } ;

        let peeked = p.lexer().next_expect_peek()?;
        match peeked.kind() {
            TK::KeywordAnd
            | TK::KeywordNand
            | TK::KeywordOr
            | TK::KeywordNor
            | TK::KeywordXor
            | TK::KeywordXnor => {
                peeked.release();
                let (n_input_gate_instantiation, span) =
                    NInputGateInstantiation::parse_with_span(p, arenas)?;
                Ok((Self::NInput(n_input_gate_instantiation), span))
            }
            _ => {
                let token = peeked.commit();
                dbg!(token.content());
                Err(ParseError::incomplete(
                    Some(token.span()),
                    "gate_instantiation",
                ))
            }
        }
    }
}
impl<'a> Parsable<'a> for GateInstantiation {}

impl<'a> Consumable<'a> for NInputGateInstantiation {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // n_input_gatetype [drive_strength] [delay2] n_input_gate_instance { , n_input_gate_instance } ;

        let (gatetype, gatetype_span) = NInputGateType::parse_with_span(p, arenas)?;
        // @Incomplete: drive_strength
        // @Incomplete: delay2
        let instances = NInputGateInstance::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
        let semicolon_span = p.lexer.expect(TK::Semicolon)?.span();

        let span = gatetype_span | semicolon_span;

        Ok((
            Self {
                gatetype,
                instances,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for NInputGateInstantiation {}

impl<'a> Consumable<'a> for NInputGateType {
    fn consume(p: &mut Parser<'a>, _arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // n_input_gatetype ::= and | nand | or | nor | xor | xnor

        let t = p.lexer.next_expect()?;
        let value = match t.kind() {
            TK::KeywordAnd => Self::And,
            TK::KeywordNand => Self::Nand,
            TK::KeywordOr => Self::Or,
            TK::KeywordNor => Self::Nor,
            TK::KeywordXor => Self::Xor,
            TK::KeywordXnor => Self::Xnor,
            _ => return Err(ParseError::unexpected_token(t)),
        };

        Ok((value, t.span()))
    }
}
impl<'a> ItemParsable<'a> for NInputGateType {
    type Item = NInputGateType;

    fn from_item(item: Self::Item, _arenas: &mut AstArenas) -> Result<Self, ParseError> {
        Ok(item)
    }
}

impl<'a> Consumable<'a> for NInputGateInstance {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // n_input_gate_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal { , input_terminal } )

        let (name, name_span) = NameOfGateInstance::parse_with_span(p, arenas)?;
        p.lexer.expect(TK::LeftParen)?;
        let output_terminal = NetLValue::parse(p, arenas)?;
        p.lexer.expect(TK::Comma)?;
        let input_terminals = Expr::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
        let right_paren_span = p.lexer.expect(TK::RightParen)?.span();

        let span = name_span | right_paren_span;

        Ok((
            Self {
                name,
                output_terminal,
                input_terminals,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for NInputGateInstance {}

impl<'a> Consumable<'a> for NameOfGateInstance {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // name_of_gate_instance ::= gate_instance_identifier [ range ]

        // @Incomplete
        let (identifier, span) = Identifier::parse_with_span(p, arenas)?;

        Ok((Self { identifier }, span))
    }
}
impl<'a> Parsable<'a> for NameOfGateInstance {}

impl<'a> Consumable<'a> for ModuleOrGenerateItemDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // module_or_generate_item_declaration ::=
        //   net_declaration
        // | reg_declaration
        // | integer_declaration
        // | real_declaration
        // | time_declaration
        // | realtime_declaration
        // | event_declaration
        // | genvar_declaration
        // | task_declaration
        // | function_declaration

        let peeked = p.lexer().next_expect_peek()?;
        match peeked.kind() {
            TK::KeywordSupply0
            | TK::KeywordSupply1
            | TK::KeywordTri
            | TK::KeywordTriand
            | TK::KeywordTrior
            | TK::KeywordTri0
            | TK::KeywordUwire
            | TK::KeywordWire
            | TK::KeywordWand
            | TK::KeywordWor => {
                peeked.release();
                let (net_declaration, span) = NetDeclaration::parse_with_span(p, arenas)?;
                Ok((Self::Net(net_declaration), span))
            }
            TK::KeywordReg => {
                peeked.release();
                let (reg_declaration, span) = RegDeclaration::parse_with_span(p, arenas)?;
                Ok((Self::Reg(reg_declaration), span))
            }
            _ => {
                let token = peeked.commit();
                dbg!(token.content());
                Err(ParseError::incomplete(
                    Some(token.span()),
                    "module_or_generate_item_declaration",
                ))
            }
        }
    }
}
impl<'a> Parsable<'a> for ModuleOrGenerateItemDeclaration {}

impl<'a> Consumable<'a> for NetDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // net_declaration ::=
        //   net_type [ signed ] [ delay3 ] list_of_net_identifiers ;
        // | net_type [ drive_strength ] [ signed ] [ delay3 ] list_of_net_decl_assignments ;
        // | net_type [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_identifiers ;
        // | net_type [ drive_strength ] [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_decl_assignments ;
        // | trireg [ charge_strength ] [ signed ] [ delay3 ] list_of_net_identifiers ;
        // | trireg [ drive_strength ] [ signed ] [ delay3 ] list_of_net_decl_assignments ;
        // | trireg [ charge_strength ] [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_identifiers ;
        // | trireg [ drive_strength ] [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_decl_assignments ;

        // @Incomplete
        let (net_type, net_type_span) = NetType::parse_with_span(p, arenas)?;
        let identifiers = Identifier::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
        let semicolon_span = p.lexer.expect(TK::Semicolon)?.span();

        let span = net_type_span | semicolon_span;

        Ok((
            Self {
                net_type,
                identifiers,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for NetDeclaration {}

impl<'a> Consumable<'a> for RegDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // reg_declaration ::= reg [ signed ] [ range ] list_of_variable_identifiers ;

        // @Incomplete
        let reg_kw_span = p.lexer.expect(TK::KeywordReg)?.span();
        let identifiers = Identifier::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
        let semicolon_span = p.lexer.expect(TK::Semicolon)?.span();

        let span = reg_kw_span | semicolon_span;

        Ok((Self { identifiers }, span))
    }
}
impl<'a> Parsable<'a> for RegDeclaration {}

impl<'a> Consumable<'a> for ParameterDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // parameter_declaration ::=
        //   parameter [ signed ] [ range ] list_of_param_assignments
        // | parameter parameter_type list_of_param_assignments

        let parameter_kw_span = p.lexer.expect(TK::KeywordParameter)?.span();
        // @Incomplete
        let assignments = ParamAssignment::parse_one_or_more_delimited(p, arenas, TK::Comma)?;
        let last_span = arenas.get_span(assignments.last().unwrap());

        let span = parameter_kw_span | last_span;

        Ok((Self { assignments }, span))
    }
}
impl<'a> Parsable<'a> for ParameterDeclaration {}

impl<'a> Consumable<'a> for ParamAssignment {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // param_assignment ::= parameter_identifier = constant_mintypmax_expression

        let (param, param_span) = Identifier::parse_with_span(p, arenas)?;
        p.lexer.expect(TK::Equals)?;
        let (constant, constant_span) = ConstantMinTypMaxExpression::parse_with_span(p, arenas)?;

        let span = param_span | constant_span;

        Ok((Self { param, constant }, span))
    }
}
impl<'a> Parsable<'a> for ParamAssignment {}
