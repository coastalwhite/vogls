use crate::ast::Identifier;
use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::expr::Expr;
use crate::ast::module::{
    AlwaysConstruct, ContinousAssign, GateInstantiation, InitialConstruct, InoutDeclaration,
    InputDeclaration, IntegerDeclaration, ListOfPortConnections, Module, ModuleInstance,
    ModuleInstantiation, ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration,
    ModulePorts, NInputGateInstance, NInputGateInstantiation, NInputGateType, NameOfGateInstance,
    NamedPortConnection, NetAssignment, NetDeclAssignment, NetDeclaration, NetDeclarationNets,
    NetType, NonPortModuleItem, OutputDeclaration, ParamAssignment, ParameterDeclaration, Port,
    PortDeclaration, PortExpression, PortReference, Range, RegDeclaration,
};
use crate::ast::statement::{NetLValue, Statement};
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, ParserScratches, TokenWalker};
use super::{Diagnostics, utils::*};

impl<'a> Consumable<'a> for Module {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 487
        // module_declaration ::=
        // { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ]
        // list_of_ports ; { module_item }
        // endmodule
        // | { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ]
        // [ list_of_port_declarations ] ; { non_port_module_item }
        // endmodule

        let attribute_instances = parse_zero_or_more_while_next(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            T::LeftParenStar,
        )?;
        tkw.next_expect(T::KeywordModule, diagnostics.as_deref_mut())?;
        let end_at = tkw.try_find_corresponding(
            T::KeywordEndModule,
            tkw.offset - 1,
            diagnostics.as_deref_mut(),
        )?;

        let module_identifier =
            item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        // @Incomplete: [ module_parameter_port_list ]
        let ports = if tkw.next_if_equals(T::LeftParen) {
            let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
            match peeked.kind {
                T::RightParen => {
                    tkw.next();
                    ModulePorts::PortDeclarations(Default::default())
                }
                T::KeywordInput | T::KeywordOutput | T::KeywordInout => {
                    let port_declarations = parse_zero_or_more_delimited::<PortDeclaration>(
                        tkw,
                        sc,
                        arenas,
                        T::Comma,
                        diagnostics.as_deref_mut(),
                    )?;
                    tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

                    ModulePorts::PortDeclarations(port_declarations)
                }
                _ => {
                    let ports = parse_one_or_more_delimited::<Port>(
                        tkw,
                        sc,
                        arenas,
                        T::Comma,
                        diagnostics.as_deref_mut(),
                    )?;
                    tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

                    ModulePorts::Ports(ports)
                }
            }
        } else {
            ModulePorts::PortDeclarations(Default::default())
        };
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
        let mut items_tkw = tkw.end_at(end_at);
        tkw.offset = end_at + 1;
        let module_items = parse_zero_or_more::<ModuleItem>(
            &mut items_tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
        )?;

        Ok(Module {
            attribute_instances,
            module_identifier,
            ports,
            module_items,
        })
    }
}

impl<'a> Consumable<'a> for ModuleItem {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // module_item ::=
        //   port_declaration ;
        // | non_port_module_item
        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::KeywordInput | T::KeywordOutput | T::KeywordInout => {
                let port_declaration =
                    parse::<PortDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::PortDeclaration(port_declaration))
            }
            _ => {
                let non_port_module_item =
                    parse::<NonPortModuleItem>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::NonPortModuleItem(non_port_module_item))
            }
        }
    }
}

impl<'a> Consumable<'a> for NonPortModuleItem {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // non_port_module_item ::=
        // module_or_generate_item
        // | generate_region
        // | specify_block
        // | { attribute_instance } parameter_declaration ;
        // | { attribute_instance } specparam_declaration

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::KeywordParameter => {
                let parameter_declaration =
                    parse::<ParameterDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::ParameterDeclaration(parameter_declaration))
            }
            T::KeywordGenerate => {
                diagnostics
                    .map(|d| d.incomplete(tkw.offset, "non_port_module_item::generate_region"));
                Err(())
            }
            T::KeywordSpecify => {
                diagnostics
                    .map(|d| d.incomplete(tkw.offset, "non_port_module_item::specify_block"));
                Err(())
            }
            T::KeywordSpecParam => {
                diagnostics.map(|d| {
                    d.incomplete(tkw.offset, "non_port_module_item::specparam_declaration")
                });
                Err(())
            }
            _ => {
                let module_or_generate_item =
                    parse::<ModuleOrGenerateItem>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::ModuleOrGenerateItem(module_or_generate_item))
            }
        }
    }
}

impl<'a> Consumable<'a> for Port {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port ::=
        //   [ port_expression ]
        // | . port_identifier ( [ port_expression ] )

        // @Incomplete: . port_identifier ( [ port_expression ] )

        let port_expression = parse::<PortExpression>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        Ok(Self::PortExpression(port_expression))
    }
}

impl<'a> Consumable<'a> for PortExpression {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_expression ::=
        //   port_reference
        // | { port_reference { , port_reference } }

        // @Incomplete: { port_reference { , port_reference } }

        let port_reference = parse::<PortReference>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        Ok(Self {
            references: port_reference,
        })
    }
}

impl<'a> Consumable<'a> for PortReference {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_reference ::=
        //   port_identifier [ [ constant_range_expression ] ]

        // @Incomplete: [ [ constant_range_expression ] ]

        let identifier = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        Ok(Self { identifier })
    }
}

impl<'a> Consumable<'a> for PortDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_declaration ::=
        //   {attribute_instance} inout_declaration
        // | {attribute_instance} input_declaration
        // | {attribute_instance} output_declaration

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match *peeked.kind {
            T::KeywordInout => {
                let inout_declaration =
                    parse::<InoutDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Inout(inout_declaration))
            }
            T::KeywordInput => {
                let input_declaration =
                    parse::<InputDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Input(input_declaration))
            }
            T::KeywordOutput => {
                let output_declaration =
                    parse::<OutputDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Output(output_declaration))
            }
            _ => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, *peeked.kind));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for InoutDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // inout_declaration ::= inout [ net_type ] [ signed ] [ range ] list_of_port_identifiers

        tkw.next_expect(T::KeywordInout, diagnostics.as_deref_mut())?;
        let mut net_type = None;
        if let Some(val) = try_item_parse::<NetType>(tkw, sc, arenas) {
            net_type = Some(val);
        }
        let signed = tkw.next_if_equals(T::KeywordSigned);
        // @Incomplete: [ range ]
        let port_identifiers = parse_one_or_more_delimited_until_fail::<Identifier>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;

        Ok(Self {
            net_type,
            signed,
            range: None,
            port_identifiers,
        })
    }
}

impl<'a> Consumable<'a> for InputDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // input_declaration ::= input [ net_type ] [ signed ] [ range ] list_of_port_identifiers

        tkw.next_expect(T::KeywordInput, diagnostics.as_deref_mut())?;
        let mut net_type = None;
        if let Some(val) = try_item_parse::<NetType>(tkw, sc, arenas) {
            net_type = Some(val);
        }
        let signed = tkw.next_if_equals(T::KeywordSigned);
        let mut range = None;
        if tkw.get(tkw.offset).is_some_and(|t| *t.kind == T::LeftBrace) {
            range = Some(parse::<Range>(tkw, sc, arenas, diagnostics.as_deref_mut())?);
        }
        let port_identifiers = parse_one_or_more_delimited_until_fail::<Identifier>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;

        Ok(Self {
            net_type,
            signed,
            range,
            port_identifiers,
        })
    }
}

impl<'a> Consumable<'a> for OutputDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // output_declaration ::=
        //   output [ net_type ] [ signed ] [ range ] list_of_port_identifiers
        // | output reg [ signed ] [ range ] list_of_variable_port_identifiers
        // | output output_variable_type list_of_variable_port_identifiers

        tkw.next_expect(T::KeywordOutput, diagnostics.as_deref_mut())?;
        let mut net_type = None;
        if let Some(val) = try_item_parse::<NetType>(tkw, sc, arenas) {
            net_type = Some(val);
        }
        let signed = tkw.next_if_equals(T::KeywordSigned);
        // @Incomplete: reg | output_variable_type
        let mut range = None;
        if tkw.get(tkw.offset).is_some_and(|t| *t.kind == T::LeftBrace) {
            range = Some(parse::<Range>(tkw, sc, arenas, diagnostics.as_deref_mut())?);
        }
        let identifiers = parse_one_or_more_delimited_until_fail::<Identifier>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;

        Ok(Self {
            net: net_type,
            signed,
            range,
            identifiers,
        })
    }
}

impl<'a> Consumable<'a> for NetType {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        _arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // net_type ::=
        //   supply0 | supply1
        // | tri
        // | triand | trior | tri0 | tri1
        // | uwire | wire | wand | wor

        let token = tkw.try_next(diagnostics.as_deref_mut())?;
        let result = match *token.kind {
            T::KeywordSupply0 => Ok(Self::Supply0),
            T::KeywordSupply1 => Ok(Self::Supply1),
            T::KeywordTri => Ok(Self::Tri),
            T::KeywordTriand => Ok(Self::TriAnd),
            T::KeywordTrior => Ok(Self::TriOr),
            T::KeywordTri0 => Ok(Self::Tri0),
            T::KeywordUwire => Ok(Self::Uwire),
            T::KeywordWire => Ok(Self::Wire),
            T::KeywordWand => Ok(Self::WAnd),
            T::KeywordWor => Ok(Self::WOr),
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                Err(())
            }
        }?;
        Ok(result)
    }
}

impl<'a> Consumable<'a> for ModuleOrGenerateItem {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

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

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::KeywordInitial => {
                let initial_construct =
                    parse::<InitialConstruct>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::InitialConstruct(initial_construct))
            }
            T::KeywordAlways => {
                let always_construct =
                    parse::<AlwaysConstruct>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::AlwaysConstruct(always_construct))
            }
            T::KeywordAssign => {
                let continous_assign =
                    parse::<ContinousAssign>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::ContinuousAssign(continous_assign))
            }
            T::Ident => {
                let module_instance =
                    parse::<ModuleInstantiation>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::ModuleInstantiation(module_instance))
            }
            T::KeywordAnd
            | T::KeywordNand
            | T::KeywordOr
            | T::KeywordNor
            | T::KeywordXor
            | T::KeywordXnor => {
                let gate_instance =
                    parse::<GateInstantiation>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::GateInstantiation(gate_instance))
            }
            T::KeywordSupply0
            | T::KeywordSupply1
            | T::KeywordTri
            | T::KeywordTriand
            | T::KeywordTrior
            | T::KeywordTri0
            | T::KeywordUwire
            | T::KeywordWire
            | T::KeywordWand
            | T::KeywordWor
            | T::KeywordReg
            | T::KeywordInteger => {
                let module_or_generate_item_declaration = parse::<ModuleOrGenerateItemDeclaration>(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::ModuleOrGenerateItemDeclaration(
                    module_or_generate_item_declaration,
                ))
            }
            _ => {
                diagnostics.map(|d| d.incomplete(tkw.offset, "module_or_generate_item"));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for ContinousAssign {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // continuous_assign ::= assign [ drive_strength ] [ delay3 ] list_of_net_assignments ;
        // list_of_net_assignments ::= net_assignment { , net_assignment }

        tkw.next_expect(T::KeywordAssign, diagnostics.as_deref_mut())?;
        let list_of_net_assignments = parse_one_or_more_delimited::<NetAssignment>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            list_of_net_assignments,
        })
    }
}

impl<'a> Consumable<'a> for NetAssignment {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // continuous_assign ::= assign [ drive_strength ] [ delay3 ] list_of_net_assignments ;
        // list_of_net_assignments ::= net_assignment { , net_assignment }

        let net_lvalue = parse::<NetLValue>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let expression = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            net_lvalue,
            expression,
        })
    }
}

impl<'a> Consumable<'a> for ModuleInstantiation {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // module_instantiation ::=
        //   module_identifier [ parameter_value_assignment ]
        //   module_instance { , module_instance } ;

        let module_identifier =
            item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let module_instances = parse_one_or_more_delimited::<ModuleInstance>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(ModuleInstantiation {
            module_identifier,
            module_instances,
        })
    }
}

impl<'a> Consumable<'a> for ModuleInstance {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // module_instance ::= name_of_module_instance ( [ list_of_port_connections ] )

        let name_of_module_instance =
            item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let list_of_port_connections =
            parse::<ListOfPortConnections>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        Ok(ModuleInstance {
            name_of_module_instance,
            list_of_port_connections,
        })
    }
}

impl<'a> Consumable<'a> for ListOfPortConnections {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // list_of_port_connections ::=
        //   ordered_port_connection { , ordered_port_connection }
        // | named_port_connection { , named_port_connection }

        if tkw.get(tkw.offset).is_some_and(|t| *t.kind == T::Dot) {
            let named = parse_zero_or_more_delimited::<NamedPortConnection>(
                tkw,
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            Ok(Self::Named(named))
        } else {
            let ordered = parse_zero_or_more_delimited::<Expr>(
                tkw,
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            Ok(Self::Ordered(ordered))
        }
    }
}

impl<'a> Consumable<'a> for NamedPortConnection {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // named_port_connection ::= { attribute_instance } . port_identifier ( [ expression ] )

        tkw.next_expect(T::Dot, diagnostics.as_deref_mut())?;
        let port_identifier =
            item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let expression = if !tkw
            .get(tkw.offset)
            .is_some_and(|t| *t.kind == T::RightParen)
        {
            Some(parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?)
        } else {
            None
        };
        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        Ok(NamedPortConnection {
            port_identifier,
            expression,
        })
    }
}

impl<'a> Consumable<'a> for InitialConstruct {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // initial_construct ::= initial statement

        tkw.next_expect(T::KeywordInitial, diagnostics.as_deref_mut())?;
        let statement = parse::<Statement>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self(statement))
    }
}

impl<'a> Consumable<'a> for AlwaysConstruct {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // always_construct ::= always statement

        tkw.next_expect(T::KeywordAlways, diagnostics.as_deref_mut())?;
        let statement = parse::<Statement>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self(statement))
    }
}

impl<'a> Consumable<'a> for GateInstantiation {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

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

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::KeywordAnd
            | T::KeywordNand
            | T::KeywordOr
            | T::KeywordNor
            | T::KeywordXor
            | T::KeywordXnor => {
                let n_input_gate_instantiation =
                    parse::<NInputGateInstantiation>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::NInput(n_input_gate_instantiation))
            }
            _ => {
                diagnostics.map(|d| d.incomplete(tkw.offset, "gate_instantiation"));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for NInputGateInstantiation {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // n_input_gatetype [drive_strength] [delay2] n_input_gate_instance { , n_input_gate_instance } ;

        let gatetype = item_parse::<NInputGateType>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        // @Incomplete: drive_strength
        // @Incomplete: delay2
        let instances = parse_one_or_more_delimited::<NInputGateInstance>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            gatetype,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for NInputGateType {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        _arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // n_input_gatetype ::= and | nand | or | nor | xor | xnor

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        let value = match *t.kind {
            T::KeywordAnd => Self::And,
            T::KeywordNand => Self::Nand,
            T::KeywordOr => Self::Or,
            T::KeywordNor => Self::Nor,
            T::KeywordXor => Self::Xor,
            T::KeywordXnor => Self::Xnor,
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                return Err(());
            }
        };

        Ok(value)
    }
}

impl<'a> Consumable<'a> for NInputGateInstance {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // n_input_gate_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal { , input_terminal } )

        let name = try_parse::<NameOfGateInstance>(tkw, sc, arenas);
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let output_terminal = parse::<NetLValue>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let input_terminals = parse_one_or_more_delimited::<Expr>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        Ok(Self {
            name,
            output_terminal,
            input_terminals,
        })
    }
}

impl<'a> Consumable<'a> for NameOfGateInstance {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // name_of_gate_instance ::= gate_instance_identifier [ range ]

        // @Incomplete
        let identifier = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self { identifier })
    }
}

impl<'a> Consumable<'a> for ModuleOrGenerateItemDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

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

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::KeywordSupply0
            | T::KeywordSupply1
            | T::KeywordTri
            | T::KeywordTriand
            | T::KeywordTrior
            | T::KeywordTri0
            | T::KeywordUwire
            | T::KeywordWire
            | T::KeywordWand
            | T::KeywordWor => {
                let net_declaration =
                    parse::<NetDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Net(net_declaration))
            }
            T::KeywordReg => {
                let reg_declaration =
                    parse::<RegDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Reg(reg_declaration))
            }
            T::KeywordInteger => {
                let integer_declaration =
                    parse::<IntegerDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Integer(integer_declaration))
            }
            _ => {
                diagnostics
                    .map(|d| d.incomplete(tkw.offset, "module_or_generate_item_declaration"));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for NetDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

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
        let net_type = item_parse::<NetType>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let signed = tkw.next_if_equals(T::KeywordSigned);
        let range = try_parse::<Range>(tkw, sc, arenas);

        tkw.next_expect(T::Ident, diagnostics.as_deref_mut())?;
        let is_assignments = tkw.is_next_equal_to(T::Equals);
        tkw.offset -= 1;

        let nets = if is_assignments {
            //   net_type [ drive_strength ] [ signed ] [ delay3 ] list_of_net_decl_assignments ;
            // | net_type [ drive_strength ] [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_decl_assignments ;
            NetDeclarationNets::Assignments(parse_one_or_more_delimited::<NetDeclAssignment>(
                tkw,
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?)
        } else {
            //   net_type [ signed ] [ delay3 ] list_of_net_identifiers ;
            // | net_type [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_identifiers ;
            NetDeclarationNets::Idents(parse_one_or_more_delimited::<Identifier>(
                tkw,
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?)
        };

        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            net_type,
            signed,
            range,
            nets,
        })
    }
}

impl<'a> Consumable<'a> for NetDeclAssignment {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // net_decl_assignment ::= net_identifier = expression

        let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self { ident, expr })
    }
}

impl<'a> Consumable<'a> for RegDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // reg_declaration ::= reg [ signed ] [ range ] list_of_variable_identifiers ;

        // @Incomplete
        tkw.next_expect(T::KeywordReg, diagnostics.as_deref_mut())?;
        let identifiers = parse_one_or_more_delimited::<Identifier>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self { identifiers })
    }
}

impl<'a> Consumable<'a> for IntegerDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // integer_declaration ::= integer list_of_variable_identifiers ;

        tkw.next_expect(T::KeywordInteger, diagnostics.as_deref_mut())?;
        let identifiers = parse_one_or_more_delimited::<Identifier>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self { identifiers })
    }
}

impl<'a> Consumable<'a> for ParameterDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // parameter_declaration ::=
        //   parameter [ signed ] [ range ] list_of_param_assignments
        // | parameter parameter_type list_of_param_assignments

        tkw.next_expect(T::KeywordParameter, diagnostics.as_deref_mut())?;
        // @Incomplete
        let assignments = parse_one_or_more_delimited::<ParamAssignment>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;

        Ok(Self { assignments })
    }
}

impl<'a> Consumable<'a> for ParamAssignment {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // param_assignment ::= parameter_identifier = constant_mintypmax_expression

        let param = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let constant =
            parse::<ConstantMinTypMaxExpression>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        Ok(Self { param, constant })
    }
}

impl<'a> Consumable<'a> for Range {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
        // range ::= [ msb_constant_expression : lsb_constant_expression ]

        tkw.next_expect(T::LeftBrace, diagnostics.as_deref_mut())?;
        let msb = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
        let lsb = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightBrace, diagnostics.as_deref_mut())?;

        Ok(Self { msb, lsb })
    }
}
