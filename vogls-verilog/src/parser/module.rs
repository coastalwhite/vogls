use crate::ast::constant_expr::ConstantMinTypMaxExpression;
use crate::ast::expr::Expr;
use crate::ast::module::{
    AlwaysConstruct, ContinousAssign, GateInstantiation, InitialConstruct, InoutDeclaration,
    InputDeclaration, ListOfPortConnections, Module, ModuleInstance, ModuleInstantiation,
    ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration, ModulePorts,
    NInputGateInstance, NInputGateInstantiation, NInputGateType, NameOfGateInstance,
    NamedPortConnection, NetAssignment, NetDeclaration, NetType, NonPortModuleItem,
    OutputDeclaration, ParamAssignment, ParameterDeclaration, Port, PortDeclaration,
    PortExpression, PortReference, RegDeclaration,
};
use crate::ast::statement::{NetLValue, Statement};
use crate::ast::Identifier;
use crate::lexer::{FromLexerError, TokenKind};
use crate::parser::ItemParsable;
use crate::span::Span;

use super::utils::*;
use super::{AstArenas, Consumable, ParseError, Parser};

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
        let module_kw_span = *p.tkw.next_expect(TK::KeywordModule)?;
        let module_identifier = Identifier::item_parse(p, arenas)?;
        // @Incomplete: [ module_parameter_port_list ]
        let ports = if p.tkw.next_if_equals(TK::LeftParen) {
            let peeked = p.tkw.try_get(p.tkw.offset)?;
            match peeked.kind {
                TK::RightParen => {
                    p.tkw.next();
                    ModulePorts::PortDeclarations(Default::default())
                }
                TK::KeywordInput | TK::KeywordOutput | TK::KeywordInout => {
                    let port_declarations =
                        parse_zero_or_more_delimited::<PortDeclaration>(p, arenas, TK::Comma)?;
                    p.tkw.next_expect(TK::RightParen)?;

                    ModulePorts::PortDeclarations(port_declarations)
                }
                _ => {
                    let ports = parse_one_or_more_delimited::<Port>(p, arenas, TK::Comma)?;
                    p.tkw.next_expect(TK::RightParen)?;

                    ModulePorts::Ports(ports)
                }
            }
        } else {
            ModulePorts::PortDeclarations(Default::default())
        };
        p.tkw.next_expect(TK::Semicolon)?;
        let module_items = parse_until_reaching::<ModuleItem>(p, arenas, TK::KeywordEndModule)?;

        let span = module_kw_span | (*p.tkw.get(p.tkw.offset - 1).unwrap().span);

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

impl<'a> Consumable<'a> for ModuleItem {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // module_item ::=
        //   port_declaration ;
        // | non_port_module_item
        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match peeked.kind {
            TK::KeywordInput | TK::KeywordOutput | TK::KeywordInout => {
                let (port_declaration, span) = parse_with_span::<PortDeclaration>(p, arenas)?;
                p.tkw.next_expect(TK::Semicolon)?;
                Ok((Self::PortDeclaration(port_declaration), span))
            }
            _ => {
                let (non_port_module_item, span) = parse_with_span::<NonPortModuleItem>(p, arenas)?;
                Ok((Self::NonPortModuleItem(non_port_module_item), span))
            }
        }
    }
}

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

        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match peeked.kind {
            // @Incomplete: | generate_region
            // @Incomplete: | specify_block
            // @Incomplete: | { attribute_instance } specparam_declaration
            TK::KeywordParameter => {
                let (parameter_declaration, span) =
                    parse_with_span::<ParameterDeclaration>(p, arenas)?;
                p.tkw.next_expect(TK::Semicolon)?;
                Ok((Self::ParameterDeclaration(parameter_declaration), span))
            }
            _ => {
                let (module_or_generate_item, span) =
                    parse_with_span::<ModuleOrGenerateItem>(p, arenas)?;
                Ok((Self::ModuleOrGenerateItem(module_or_generate_item), span))
            }
        }
    }
}

impl<'a> Consumable<'a> for Port {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port ::=
        //   [ port_expression ]
        // | . port_identifier ( [ port_expression ] )

        // @Incomplete: . port_identifier ( [ port_expression ] )

        let (port_expression, span) = parse_with_span::<PortExpression>(p, arenas)?;
        Ok((Self::PortExpression(port_expression), span))
    }
}

impl<'a> Consumable<'a> for PortExpression {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_expression ::=
        //   port_reference
        // | { port_reference { , port_reference } }

        // @Incomplete: { port_reference { , port_reference } }

        let (port_reference, span) = parse_with_span::<PortReference>(p, arenas)?;
        Ok((
            Self {
                references: port_reference,
            },
            span,
        ))
    }
}

impl<'a> Consumable<'a> for PortReference {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_reference ::=
        //   port_identifier [ [ constant_range_expression ] ]

        // @Incomplete: [ [ constant_range_expression ] ]

        let (identifier, span) = Identifier::item_parse_with_span(p, arenas)?;
        Ok((Self { identifier }, span))
    }
}

impl<'a> Consumable<'a> for PortDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_declaration ::=
        //   {attribute_instance} inout_declaration
        // | {attribute_instance} input_declaration
        // | {attribute_instance} output_declaration

        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match *peeked.kind {
            TK::KeywordInout => {
                let (inout_declaration, span) = parse_with_span::<InoutDeclaration>(p, arenas)?;
                Ok((Self::Inout(inout_declaration), span))
            }
            TK::KeywordInput => {
                let (input_declaration, span) = parse_with_span::<InputDeclaration>(p, arenas)?;
                Ok((Self::Input(input_declaration), span))
            }
            TK::KeywordOutput => {
                let (output_declaration, span) = parse_with_span::<OutputDeclaration>(p, arenas)?;
                Ok((Self::Output(output_declaration), span))
            }
            _ => Err(ParseError::unexpected_token()),
        }
    }
}

impl<'a> Consumable<'a> for InoutDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // inout_declaration ::= inout [ net_type ] [ signed ] [ range ] list_of_port_identifiers

        let inout_kw_span = *p.tkw.next_expect(TK::KeywordInout)?;
        let mut net_type = None;
        if let Some(val) = NetType::try_item_parse(p, arenas) {
            net_type = Some(val);
        }
        let signed = p.tkw.next_if_equals(TK::KeywordSigned);
        // @Incomplete: [ range ]
        let port_identifiers =
            parse_one_or_more_delimited_until_fail::<Identifier>(p, arenas, TK::Comma)?;
        let last = port_identifiers.last().unwrap();
        let end_span = *arenas.spans.get(last.loc).unwrap();

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

impl<'a> Consumable<'a> for InputDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // input_declaration ::= input [ net_type ] [ signed ] [ range ] list_of_port_identifiers

        let input_kw_span = *p.tkw.next_expect(TK::KeywordInput)?;
        let mut net_type = None;
        if let Some(val) = NetType::try_item_parse(p, arenas) {
            net_type = Some(val);
        }
        let signed = p.tkw.next_if_equals(TK::KeywordSigned);
        // @Incomplete: [ range ]
        let port_identifiers =
            parse_one_or_more_delimited_until_fail::<Identifier>(p, arenas, TK::Comma)?;
        let last = port_identifiers.last().unwrap();
        let end_span = *arenas.spans.get(last.loc).unwrap();

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

impl<'a> Consumable<'a> for OutputDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // output_declaration ::=
        //   output [ net_type ] [ signed ] [ range ] list_of_port_identifiers
        // | output reg [ signed ] [ range ] list_of_variable_port_identifiers
        // | output output_variable_type list_of_variable_port_identifiers

        let output_kw_span = *p.tkw.next_expect(TK::KeywordOutput)?;
        let mut net_type = None;
        if let Some(val) = NetType::try_item_parse(p, arenas) {
            net_type = Some(val);
        }
        let signed = p.tkw.next_if_equals(TK::KeywordSigned);
        // @Incomplete: reg | output_variable_type
        // @Incomplete: [ range ]
        let identifiers =
            parse_one_or_more_delimited_until_fail::<Identifier>(p, arenas, TK::Comma)?;
        let last = identifiers.last().unwrap();
        let end_span = *arenas.spans.get(last.loc).unwrap();

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

impl<'a> Consumable<'a> for NetType {
    fn consume(p: &mut Parser<'a>, _arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // net_type ::=
        //   supply0 | supply1
        // | tri
        // | triand | trior | tri0 | tri1
        // | uwire | wire | wand | wor

        let token = p.tkw.try_next()?;
        let result = match token.kind {
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
            _ => Err(ParseError::unexpected_token()),
        }?;
        Ok((result, *token.span))
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

        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match peeked.kind {
            TK::KeywordInitial => {
                let (initial_construct, span) = parse_with_span::<InitialConstruct>(p, arenas)?;
                Ok((Self::InitialConstruct(initial_construct), span))
            }
            TK::KeywordAlways => {
                let (always_construct, span) = parse_with_span::<AlwaysConstruct>(p, arenas)?;
                Ok((Self::AlwaysConstruct(always_construct), span))
            }
            TK::KeywordAssign => {
                let (continous_assign, span) = parse_with_span::<ContinousAssign>(p, arenas)?;
                Ok((Self::ContinuousAssign(continous_assign), span))
            }
            TK::Ident => {
                let (module_instance, span) = parse_with_span::<ModuleInstantiation>(p, arenas)?;
                Ok((Self::ModuleInstantiation(module_instance), span))
            }
            TK::KeywordAnd
            | TK::KeywordNand
            | TK::KeywordOr
            | TK::KeywordNor
            | TK::KeywordXor
            | TK::KeywordXnor => {
                let (gate_instance, span) = parse_with_span::<GateInstantiation>(p, arenas)?;
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
                let (module_or_generate_item_declaration, span) =
                    parse_with_span::<ModuleOrGenerateItemDeclaration>(p, arenas)?;
                Ok((
                    Self::ModuleOrGenerateItemDeclaration(module_or_generate_item_declaration),
                    span,
                ))
            }
            _ => {
                let token = p.tkw.try_next()?;
                Err(ParseError::incomplete(
                    Some(*token.span),
                    "module_or_generate_item",
                ))
            }
        }
    }
}

impl<'a> Consumable<'a> for ContinousAssign {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // continuous_assign ::= assign [ drive_strength ] [ delay3 ] list_of_net_assignments ;
        // list_of_net_assignments ::= net_assignment { , net_assignment }

        let assign_span = *p.tkw.next_expect(TK::KeywordAssign)?;

        let list_of_net_assignments =
            parse_one_or_more_delimited::<NetAssignment>(p, arenas, TK::Comma)?;
        let semicolon_span = *p.tkw.next_expect(TK::Semicolon)?;

        let span = assign_span | semicolon_span;
        Ok((
            Self {
                list_of_net_assignments,
            },
            span,
        ))
    }
}

impl<'a> Consumable<'a> for NetAssignment {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // continuous_assign ::= assign [ drive_strength ] [ delay3 ] list_of_net_assignments ;
        // list_of_net_assignments ::= net_assignment { , net_assignment }

        let (net_lvalue, net_lvalue_span) = parse_with_span::<NetLValue>(p, arenas)?;
        p.tkw.next_expect(TK::Equals)?;
        let (expression, expression_span) = parse_with_span::<Expr>(p, arenas)?;

        let span = net_lvalue_span | expression_span;

        Ok((
            Self {
                net_lvalue,
                expression,
            },
            span,
        ))
    }
}

impl<'a> Consumable<'a> for ModuleInstantiation {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // module_instantiation ::=
        //   module_identifier [ parameter_value_assignment ]
        //   module_instance { , module_instance } ;

        let (module_identifier, module_identifier_span) = Identifier::item_parse_with_span(p, arenas)?;
        let module_instances = parse_one_or_more_delimited::<ModuleInstance>(p, arenas, TK::Comma)?;
        let semicolon_span = *p.tkw.next_expect(TK::Semicolon)?;

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

impl<'a> Consumable<'a> for ModuleInstance {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // module_instance ::= name_of_module_instance ( [ list_of_port_connections ] )

        let (name_of_module_instance, name_of_module_instance_span) =
            Identifier::item_parse_with_span(p, arenas)?;
        p.tkw.next_expect(TK::LeftParen)?;
        let list_of_port_connections = parse::<ListOfPortConnections>(p, arenas)?;
        let right_paren_span = *p.tkw.next_expect(TK::RightParen)?;

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

impl<'a> Consumable<'a> for ListOfPortConnections {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // list_of_port_connections ::=
        //   ordered_port_connection { , ordered_port_connection }
        // | named_port_connection { , named_port_connection }

        if p.tkw
            .get(p.tkw.offset)
            .is_some_and(|t| *t.kind == TK::Dot)
        {
            let named = parse_zero_or_more_delimited::<NamedPortConnection>(p, arenas, TK::Comma)?;
            let span = arenas.spans[named.loc];
            Ok((Self::Named(named), span))
        } else {
            let ordered = parse_zero_or_more_delimited::<Expr>(p, arenas, TK::Comma)?;
            let span = arenas.spans[ordered.loc];
            Ok((Self::Ordered(ordered), span))
        }
    }
}

impl<'a> Consumable<'a> for NamedPortConnection {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // named_port_connection ::= { attribute_instance } . port_identifier ( [ expression ] )

        let dot_span = *p.tkw.next_expect(TK::Dot)?;
        let port_identifier = Identifier::item_parse(p, arenas)?;
        p.tkw.next_expect(TK::LeftParen)?;
        let expression = if !p
            .tkw
            .get(p.tkw.offset)
            .is_some_and(|t| *t.kind == TK::RightParen)
        {
            Some(parse::<Expr>(p, arenas)?)
        } else {
            None
        };
        let right_paren_span = *p.tkw.next_expect(TK::RightParen)?;
        let span = dot_span | right_paren_span;

        Ok((
            NamedPortConnection {
                port_identifier,
                expression,
            },
            span,
        ))
    }
}

impl<'a> Consumable<'a> for InitialConstruct {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // initial_construct ::= initial statement

        let initial_kw_span = *p.tkw.next_expect(TK::KeywordInitial)?;
        let (statement, span) = parse_with_span::<Statement>(p, arenas)?;

        let span = initial_kw_span | span;

        Ok((Self(statement), span))
    }
}

impl<'a> Consumable<'a> for AlwaysConstruct {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // always_construct ::= always statement

        let always_kw_span = *p.tkw.next_expect(TK::KeywordAlways)?;
        let (statement, span) = parse_with_span::<Statement>(p, arenas)?;

        let span = always_kw_span | span;

        Ok((Self(statement), span))
    }
}

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

        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match peeked.kind {
            TK::KeywordAnd
            | TK::KeywordNand
            | TK::KeywordOr
            | TK::KeywordNor
            | TK::KeywordXor
            | TK::KeywordXnor => {
                let (n_input_gate_instantiation, span) =
                    parse_with_span::<NInputGateInstantiation>(p, arenas)?;
                Ok((Self::NInput(n_input_gate_instantiation), span))
            }
            _ => Err(ParseError::incomplete(
                Some(*peeked.span),
                "gate_instantiation",
            )),
        }
    }
}

impl<'a> Consumable<'a> for NInputGateInstantiation {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // n_input_gatetype [drive_strength] [delay2] n_input_gate_instance { , n_input_gate_instance } ;

        let (gatetype, gatetype_span) = NInputGateType::item_parse_with_span(p, arenas)?;
        // @Incomplete: drive_strength
        // @Incomplete: delay2
        let instances = parse_one_or_more_delimited::<NInputGateInstance>(p, arenas, TK::Comma)?;
        let semicolon_span = *p.tkw.next_expect(TK::Semicolon)?;

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

impl<'a> Consumable<'a> for NInputGateType {
    fn consume(p: &mut Parser<'a>, _arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // n_input_gatetype ::= and | nand | or | nor | xor | xnor

        let t = p.tkw.try_next()?;
        let value = match t.kind {
            TK::KeywordAnd => Self::And,
            TK::KeywordNand => Self::Nand,
            TK::KeywordOr => Self::Or,
            TK::KeywordNor => Self::Nor,
            TK::KeywordXor => Self::Xor,
            TK::KeywordXnor => Self::Xnor,
            _ => return Err(ParseError::unexpected_token()),
        };

        Ok((value, *t.span))
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

        let name = try_parse_with_span::<NameOfGateInstance>(p, arenas);
        let mut start_span = *p.tkw.next_expect(TK::LeftParen)?;
        let name = name.map(|(name, name_span)| {
            start_span = name_span;
            name
        });
        let output_terminal = parse::<NetLValue>(p, arenas)?;
        p.tkw.next_expect(TK::Comma)?;
        let input_terminals = parse_one_or_more_delimited::<Expr>(p, arenas, TK::Comma)?;
        let right_paren_span = *p.tkw.next_expect(TK::RightParen)?;

        let span = start_span | right_paren_span;

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

impl<'a> Consumable<'a> for NameOfGateInstance {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // name_of_gate_instance ::= gate_instance_identifier [ range ]

        // @Incomplete
        let (identifier, span) = Identifier::item_parse_with_span(p, arenas)?;

        Ok((Self { identifier }, span))
    }
}

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

        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match peeked.kind {
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
                let (net_declaration, span) = parse_with_span::<NetDeclaration>(p, arenas)?;
                Ok((Self::Net(net_declaration), span))
            }
            TK::KeywordReg => {
                let (reg_declaration, span) = parse_with_span::<RegDeclaration>(p, arenas)?;
                Ok((Self::Reg(reg_declaration), span))
            }
            _ => {
                let token = p.tkw.try_next()?;
                Err(ParseError::incomplete(
                    Some(*token.span),
                    "module_or_generate_item_declaration",
                ))
            }
        }
    }
}

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
        let (net_type, net_type_span) = NetType::item_parse_with_span(p, arenas)?;
        let identifiers = parse_one_or_more_delimited::<Identifier>(p, arenas, TK::Comma)?;
        let semicolon_span = *p.tkw.next_expect(TK::Semicolon)?;

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

impl<'a> Consumable<'a> for RegDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // reg_declaration ::= reg [ signed ] [ range ] list_of_variable_identifiers ;

        // @Incomplete
        let reg_kw_span = *p.tkw.next_expect(TK::KeywordReg)?;
        let identifiers = parse_one_or_more_delimited::<Identifier>(p, arenas, TK::Comma)?;
        let semicolon_span = *p.tkw.next_expect(TK::Semicolon)?;

        let span = reg_kw_span | semicolon_span;

        Ok((Self { identifiers }, span))
    }
}

impl<'a> Consumable<'a> for ParameterDeclaration {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // parameter_declaration ::=
        //   parameter [ signed ] [ range ] list_of_param_assignments
        // | parameter parameter_type list_of_param_assignments

        let parameter_kw_span = *p.tkw.next_expect(TK::KeywordParameter)?;
        // @Incomplete
        let assignments = parse_one_or_more_delimited::<ParamAssignment>(p, arenas, TK::Comma)?;
        let last_span = arenas.get_span(assignments.last().unwrap());

        let span = parameter_kw_span | last_span;

        Ok((Self { assignments }, span))
    }
}

impl<'a> Consumable<'a> for ParamAssignment {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // param_assignment ::= parameter_identifier = constant_mintypmax_expression

        let (param, param_span) = Identifier::item_parse_with_span(p, arenas)?;
        p.tkw.next_expect(TK::Equals)?;
        let (constant, constant_span) = parse_with_span::<ConstantMinTypMaxExpression>(p, arenas)?;

        let span = param_span | constant_span;

        Ok((Self { param, constant }, span))
    }
}
