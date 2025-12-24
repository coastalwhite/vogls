use crate::ast::Identifier;
use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::expr::Expr;
use crate::ast::module::{
    AlwaysConstruct, CaseGenerateConstruct, CaseGenerateItem, CaseGeneratePattern, ContinousAssign,
    Dimension, GateInstantiation, GenerateBlock, GenerateRegion, GenvarAssignment,
    GenvarDeclaration, IfGenerateConstruct, InitialConstruct, InoutDeclaration, InputDeclaration,
    IntegerDeclaration, ListOfPortConnections, LocalParameterDeclaration, LoopGenerateConstruct,
    Module, ModuleInstance, ModuleInstantiation, ModuleItem, ModuleOrGenerateItem,
    ModuleOrGenerateItemDeclaration, ModulePorts, NInputGateInstance, NInputGateInstantiation,
    NInputGateType, NameOfGateInstance, NamedParameterAssignment, NamedPortConnection,
    NetAssignment, NetDeclAssignment, NetDeclaration, NetDeclarationNets, NetIdent, NetType,
    NonPortModuleItem, OutputDeclaration, OutputNet, ParamAssignment, ParameterDeclaration,
    ParameterDeclarationTyping, ParameterValueAssignment, Port, PortDeclaration, PortExpression,
    PortReference, Range, RegDeclaration, TaskDeclaration, VariableType,
};
use crate::ast::statement::{NetLValue, Statement, StatementOrNull};
use crate::parser::TokenRange;
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

        let attribute_instances =
            parse_zero_or_more_while_next(tkw, sc, arenas, diagnostics.as_deref_mut(), |t| {
                t == T::LeftParenStar
            })?;
        tkw.next_expect(T::KeywordModule, diagnostics.as_deref_mut())?;
        let end_at = tkw.try_find_corresponding(
            T::KeywordEndModule,
            tkw.offset - 1,
            diagnostics.as_deref_mut(),
        )?;

        let module_identifier =
            item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let mut module_parameter_port_list = None;
        if tkw.next_if_equals(T::Hash) {
            tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
            let Some(end) = tkw.find_next_same_depth(T::RightParen) else {
                if let Some(diagnostics) = diagnostics.as_deref_mut() {
                    diagnostics.no_corresponding(tkw.offset - 1, T::RightParen);
                }
                return Err(());
            };
            module_parameter_port_list = Some(parse_one_or_more_delimited::<ParameterDeclaration>(
                &mut tkw.end_at(end),
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?);
            tkw.offset = end + 1;
        }
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
            module_parameter_port_list,
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
                let generate_region =
                    GenerateRegion::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::GenerateRegion(generate_region))
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
        let port_identifiers = parse_one_or_more_while::<Identifier>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            },
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
        let port_identifiers = parse_one_or_more_while::<Identifier>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            },
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
        let net = match *tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?.kind {
            T::KeywordSupply0
            | T::KeywordSupply1
            | T::KeywordTri
            | T::KeywordTriand
            | T::KeywordTrior
            | T::KeywordTri0
            | T::KeywordTri1
            | T::KeywordUwire
            | T::KeywordWire
            | T::KeywordWand
            | T::KeywordWor
            | T::KeywordReg
            | T::KeywordInteger
            | T::KeywordTime => Some(item_parse::<OutputNet>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?),
            _ => None,
        };
        let signed = tkw.next_if_equals(T::KeywordSigned);
        let mut range = None;
        if tkw.get(tkw.offset).is_some_and(|t| *t.kind == T::LeftBrace) {
            range = Some(parse::<Range>(tkw, sc, arenas, diagnostics.as_deref_mut())?);
        }
        let identifiers = parse_one_or_more_while::<Identifier>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            },
        )?;

        Ok(Self {
            net,
            signed,
            range,
            identifiers,
        })
    }
}

impl<'a> Consumable<'a> for OutputNet {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        Ok(match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
            T::KeywordReg => Self::Register,
            T::KeywordInteger => Self::Integer,
            T::KeywordTime => Self::Time,
            _ => {
                tkw.offset -= 1;
                Self::NetType(NetType::consume(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                )?)
            }
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

impl<'a> Consumable<'a> for GenerateRegion {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // generate_region ::= generate { module_or_generate_item } endgenerate

        tkw.next_expect(T::KeywordGenerate, diagnostics.as_deref_mut())?;
        let Some(end_generate) = tkw.find_next_same_depth(T::KeywordEndGenerate) else {
            diagnostics.map(|d| d.no_corresponding(tkw.offset - 1, T::KeywordEndGenerate));
            return Err(());
        };
        let module_or_generate_item = parse_zero_or_more::<ModuleOrGenerateItem>(
            &mut tkw.end_at(end_generate),
            sc,
            arenas,
            diagnostics.as_deref_mut(),
        )?;
        tkw.offset = end_generate + 1;

        Ok(Self {
            module_or_generate_item,
        })
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
            | T::KeywordInteger
            | T::KeywordGenvar
            | T::KeywordTask => {
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
            T::KeywordFor => {
                let loop_generate_construct =
                    parse::<LoopGenerateConstruct>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::LoopGenerateConstruct(loop_generate_construct))
            }
            T::KeywordIf => {
                let if_generate_construct =
                    parse::<IfGenerateConstruct>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::IfGenerateConstruct(if_generate_construct))
            }
            T::KeywordCase => {
                let case_generate_construct =
                    parse::<CaseGenerateConstruct>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::CaseGenerateConstruct(case_generate_construct))
            }
            T::KeywordLocalParam => {
                let local_parameter_declaration = parse::<LocalParameterDeclaration>(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::LocalParameterDeclaration(local_parameter_declaration))
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
        let mut parameter_value_assignment = None;
        if tkw.is_next_equal_to(T::Hash) {
            parameter_value_assignment = Some(parse::<ParameterValueAssignment>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?);
        }
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
            parameter_value_assignment,
            module_instances,
        })
    }
}

impl<'a> Consumable<'a> for ParameterValueAssignment {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // parameter_value_assignment ::= # ( list_of_parameter_assignments )
        // list_of_parameter_assignments ::=
        //   ordered_parameter_assignment { , ordered_parameter_assignment }
        // | named_parameter_assignment { , named_parameter_assignment }

        tkw.next_expect(T::Hash, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let Some(end) = tkw.find_next_same_depth(T::RightParen) else {
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.no_corresponding(tkw.offset - 1, T::RightParen);
            }
            return Err(());
        };
        let result = if tkw.is_next_equal_to(T::Dot) {
            Self::Named(parse_one_or_more_delimited::<NamedParameterAssignment>(
                &mut tkw.end_at(end),
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?)
        } else {
            Self::Ordered(parse_one_or_more_delimited::<ConstantExpr>(
                &mut tkw.end_at(end),
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?)
        };
        tkw.offset = end + 1;
        Ok(result)
    }
}

impl<'a> Consumable<'a> for NamedParameterAssignment {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // named_parameter_assignment ::= . parameter_identifier ( [ mintypmax_expression ] )

        tkw.next_expect(T::Dot, diagnostics.as_deref_mut())?;
        let identifier = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let mut expression = None;
        if !tkw.next_if_equals(T::RightParen) {
            expression = Some(parse::<ConstantMinTypMaxExpression>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?);
            tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
        }

        Ok(Self {
            identifier,
            expression,
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
            T::KeywordGenvar => {
                let genvar_declaration =
                    parse::<GenvarDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Genvar(genvar_declaration))
            }
            T::KeywordTask => {
                let task_declaration =
                    parse::<TaskDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Task(task_declaration))
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
        let mut range = None;
        if tkw.is_next_equal_to(T::LeftBrace) {
            range = Some(parse::<Range>(tkw, sc, arenas, diagnostics.as_deref_mut())?);
        }

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
            NetDeclarationNets::Idents(parse_one_or_more_delimited::<NetIdent>(
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

impl<'a> Consumable<'a> for NetIdent {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // net_identifier { dimension }

        let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let dimension = parse_zero_or_more_while_next::<Dimension>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |t| t == T::LeftBrace,
        )?;

        Ok(Self { ident, dimension })
    }
}

impl<'a> Consumable<'a> for Dimension {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
        // dimension ::= [ dimension_constant_expression : dimension_constant_expression ]

        tkw.next_expect(T::LeftBrace, diagnostics.as_deref_mut())?;
        let Some(end_brace) = tkw.find_next_same_depth(T::RightBrace) else {
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.no_corresponding(tkw.offset - 1, T::RightBrace);
            }
            return Err(());
        };
        let Some(colon) = tkw.end_at(end_brace).find_next_same_depth(T::Colon) else {
            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                diagnostics.not_found(
                    TokenRange {
                        start: tkw.offset,
                        end: end_brace,
                    },
                    T::Colon,
                );
            }
            return Err(());
        };

        let lhs = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut());
        if lhs.is_err() {
            tkw.offset = colon;
        }

        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;

        let rhs = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut());
        if rhs.is_err() {
            tkw.offset = end_brace;
        }

        tkw.next_expect(T::RightBrace, diagnostics.as_deref_mut())?;

        // Reporting errors from both left- and right-hand side.
        let (Ok(lhs), Ok(rhs)) = (lhs, rhs) else {
            return Err(());
        };
        Ok(Dimension { lhs, rhs })
    }
}

impl<'a> Consumable<'a> for GenvarDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // genvar_declaration ::= genvar list_of_genvar_identifiers ;

        tkw.next_expect(T::KeywordGenvar, diagnostics.as_deref_mut())?;
        let identifiers = parse_one_or_more_while::<Identifier>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            },
        )?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self { identifiers })
    }
}

impl<'a> Consumable<'a> for TaskDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
        // task_declaration ::= task [ automatic ] task_identifier ;
        //   { task_item_declaration }
        //   statement_or_null
        //   endtask
        // | task [ automatic ] task_identifier ( [ task_port_list ] ) ;
        //   { block_item_declaration }
        //   statement_or_null
        //   endtask

        tkw.next_expect(T::KeywordTask, diagnostics.as_deref_mut())?;
        let automatic = tkw.next_if_equals(T::KeywordAutomatic);
        let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
        let statement_or_null =
            parse::<StatementOrNull>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::KeywordEndTask, diagnostics.as_deref_mut())?;

        Ok(Self {
            ident,
            automatic,
            statement_or_null,
        })
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

        tkw.next_expect(T::KeywordReg, diagnostics.as_deref_mut())?;
        let signed = tkw.next_if_equals(T::KeywordSigned);
        let mut range = None;
        if tkw.is_next_equal_to(T::LeftBrace) {
            range = Some(parse::<Range>(tkw, sc, arenas, diagnostics.as_deref_mut())?);
        }
        let variable_types = parse_one_or_more_delimited::<VariableType>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            signed,
            range,
            variable_types,
        })
    }
}

impl<'a> Consumable<'a> for VariableType {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // variable_type ::=
        //   variable_identifier { dimension } |
        //   variable_identifier = constant_expression
        // @Incomplete

        let identifier = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let dimensions =
            parse_zero_or_more_while_next(tkw, sc, arenas, diagnostics.as_deref_mut(), |t| {
                t == T::LeftBrace
            })?;
        Ok(Self {
            identifier,
            dimensions,
        })
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

impl<'a> Consumable<'a> for LocalParameterDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // local_parameter_declaration ::=
        //   localparam [ signed ] [ range ] list_of_param_assignments
        // | localparam parameter_type list_of_param_assignments

        tkw.next_expect(T::KeywordLocalParam, diagnostics.as_deref_mut())?;
        let typing =
            parse::<ParameterDeclarationTyping>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let assignments = parse_one_or_more_delimited_and_after::<ParamAssignment>(
            tkw,
            sc,
            arenas,
            T::Comma,
            T::Ident,
            diagnostics.as_deref_mut(),
        )?;

        Ok(Self {
            typing,
            assignments,
        })
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
        let typing =
            parse::<ParameterDeclarationTyping>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let assignments = parse_one_or_more_delimited_and_after::<ParamAssignment>(
            tkw,
            sc,
            arenas,
            T::Comma,
            T::Ident,
            diagnostics.as_deref_mut(),
        )?;

        Ok(Self {
            typing,
            assignments,
        })
    }
}

impl<'a> Consumable<'a> for ParameterDeclarationTyping {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;
        Ok(match tkw.get(tkw.offset).map(|t| *t.kind) {
            Some(T::KeywordInteger) => {
                tkw.offset += 1;
                Self::Integer
            }
            Some(T::KeywordReal) => {
                tkw.offset += 1;
                Self::Real
            }
            Some(T::KeywordRealtime) => {
                tkw.offset += 1;
                Self::Realtime
            }
            Some(T::KeywordTime) => {
                tkw.offset += 1;
                Self::Time
            }
            _ => {
                let signed = tkw.next_if_equals(T::KeywordSigned);
                let mut range = None;
                if tkw.is_next_equal_to(T::LeftBrace) {
                    range = Some(parse::<Range>(tkw, sc, arenas, diagnostics.as_deref_mut())?);
                }
                Self::None(signed, range)
            }
        })
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

impl<'a> Consumable<'a> for LoopGenerateConstruct {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // loop_generate_construct ::= for ( genvar_initialization ; genvar_expression ; genvar_iteration ) generate_block

        tkw.next_expect(T::KeywordFor, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;

        let initialization =
            parse::<GenvarAssignment>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
        let condition = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
        let iteration = parse::<GenvarAssignment>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        let block = parse::<GenerateBlock>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            initialization,
            condition,
            iteration,
            block,
        })
    }
}

impl<'a> Consumable<'a> for IfGenerateConstruct {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // if_generate_construct ::= if ( constant_expression ) generate_block_or_null
        //   [ else generate_block_or_null ]

        tkw.next_expect(T::KeywordIf, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;

        let condition = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        let truthy = parse::<Option<GenerateBlock>>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        let mut falsy = None;
        if tkw.next_if_equals(T::KeywordElse) {
            falsy = Some(parse::<Option<GenerateBlock>>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?);
        }

        Ok(Self {
            condition,
            truthy,
            falsy,
        })
    }
}

impl<'a> Consumable<'a> for CaseGenerateConstruct {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // case_generate_construct ::= case ( constant_expression ) case_generate_item { case_generate_item } endcase

        let case_offset = tkw.offset;
        tkw.next_expect(T::KeywordCase, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;

        let value = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        let Some(end) = tkw.find_next_same_depth(T::KeywordEndCase) else {
            diagnostics.map(|d| d.no_corresponding(case_offset, T::KeywordEndCase));
            return Err(());
        };

        let items = parse_one_or_more::<CaseGenerateItem>(
            &mut tkw.end_at(end),
            sc,
            arenas,
            diagnostics.as_deref_mut(),
        )?;
        tkw.offset = end + 1;

        Ok(Self { value, items })
    }
}

impl<'a> Consumable<'a> for CaseGenerateItem {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // case_generate_item ::= constant_expression { , constant_expression } : generate_block_or_null | default [ : ] generate_block_or_null

        let pattern = if tkw.next_if_equals(T::KeywordDefault) {
            tkw.next_if_equals(T::Colon);
            CaseGeneratePattern::Default
        } else {
            let Some(end) = tkw.find_next_same_depth(T::Colon) else {
                diagnostics.map(|d| d.no_corresponding(tkw.offset, T::Colon));
                return Err(());
            };

            let values = parse_one_or_more_delimited::<ConstantExpr>(
                &mut tkw.end_at(end),
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            tkw.offset = end + 1;
            CaseGeneratePattern::Exprs(values)
        };

        let block = parse::<Option<GenerateBlock>>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        Ok(Self { pattern, block })
    }
}

impl<'a> Consumable<'a> for GenvarAssignment {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // genvar_initialization ::= genvar_identifier = constant_expression
        // genvar_iteration      ::= genvar_identifier = genvar_expression

        let ident = item_parse(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let expr = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self { ident, expr })
    }
}

impl<'a> Consumable<'a> for GenerateBlock {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // generate_block ::=
        //   module_or_generate_item
        // | begin [ : generate_block_identifier ] { module_or_generate_item } end

        if tkw.next_if_equals(T::KeywordBegin) {
            let Some(end) = tkw.find_next_same_depth(T::KeywordEnd) else {
                diagnostics.map(|d| d.no_corresponding(tkw.offset - 1, T::KeywordEnd));
                return Err(());
            };

            let mut identifier = None;
            if tkw.next_if_equals(T::Colon) {
                identifier = Some(item_parse::<Identifier>(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                )?);
            }
            let module_or_generate_item = parse_zero_or_more::<ModuleOrGenerateItem>(
                &mut tkw.end_at(end),
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?;
            tkw.offset = end + 1;
            Ok(Self::BeginEnd(identifier, module_or_generate_item))
        } else {
            Ok(Self::ModuleOrGenerateItem(parse::<ModuleOrGenerateItem>(
                tkw,
                sc,
                arenas,
                diagnostics,
            )?))
        }
    }
}

impl<'a> Consumable<'a> for Option<GenerateBlock> {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // generate_block_or_null ::= generate_block |;

        if tkw.next_if_equals(T::Semicolon) {
            Ok(None)
        } else {
            GenerateBlock::consume(tkw, sc, arenas, diagnostics).map(Some)
        }
    }
}
