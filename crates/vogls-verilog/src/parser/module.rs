use crate::arena::Arena;
use crate::ast::constant_expr::{
    ConstantExpr, ConstantMinTypMaxExpression, ConstantRangeExpression,
};
use crate::ast::expr::Expr;
use crate::ast::module::{
    AlwaysConstruct, BlockItemDeclaration, CaseGenerateConstruct, CaseGenerateItem,
    CaseGeneratePattern, CmosSwitchInstance, CmosSwitchInstantiation, CmosSwitchType,
    ContinousAssign, Dimension, EnableGateInstance, EnableGateInstantiation, EnableGateType,
    FunctionDeclaration, FunctionRangeOrType, GateInstantiation, GenerateBlock, GenerateRegion,
    GenvarAssignment, GenvarDeclaration, IfGenerateConstruct, InitialConstruct, InoutDeclaration,
    InputDeclaration, IntegerDeclaration, ListOfPortConnections, LocalParameterDeclaration,
    LoopGenerateConstruct, Module, ModuleInstance, ModuleInstantiation, ModuleItem,
    ModuleOrGenerateItem, ModuleOrGenerateItemContent, ModuleOrGenerateItemDeclaration,
    ModulePorts, MosSwitchInstance, MosSwitchInstantiation, MosSwitchType, NInputGateInstance,
    NInputGateInstantiation, NInputGateType, NOutputGateInstance, NOutputGateInstantiation,
    NOutputGateType, NameOfGateInstance, NamedParameterAssignment, NamedPortConnection,
    NetAssignment, NetDeclAssignment, NetDeclaration, NetDeclarationNets, NetIdent, NetType,
    NonPortModuleItem, OutputDeclaration, OutputNet, ParamAssignment, ParameterDeclaration,
    ParameterDeclarationTyping, ParameterValueAssignment, PassEnSwitchInstance,
    PassEnSwitchInstantiation, PassEnSwitchType, PassSwitchInstance, PassSwitchInstantiation,
    PassSwitchType, Port, PortDeclaration, PortExpression, PortReference, PullGateInstance,
    PullGateInstantiation, Range, RealDeclaration, RealtimeDeclaration, RegDeclaration,
    TaskDeclaration, TaskPortItem, TaskPortItemContent, TfInoutDeclaration, TfInputDeclaration,
    TfOutputDeclaration, TfType, TimeDeclaration, TimeScale, VariableType, VariableTypeVariant,
};
use crate::ast::specify::{
    EdgeIdentifier, ModulePathExpr, PathDeclaration, PathDeclarationVariant, PathDelayValue,
    PolarityOperator, SpecifyBlock, SpecifyBlockItem, StateDependentCondition, SystemTimingCheck,
    TerminalDescriptor,
};
use crate::ast::statement::{
    Delay2, Delay3, NetLValue, Statement, StatementOrNull, SystemTaskIdentifier,
};
use crate::ast::udp::UdpInstantiation;
use crate::ast::{AstIdRange, AttributeInstance, DriveStrength, Identifier};
use crate::parser::{TokenRange, is_drive_strength_kw};
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, ParserScratches, TokenWalker};
use super::{Diagnostics, utils::*};

impl<'a> Consumable<'a> for Module<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
            parse_zero_or_more_while_next(tkw, sc, arenas, ast, diagnostics.as_deref_mut(), |t| {
                t == T::LeftParenStar
            })?;
        tkw.next_expect(T::KeywordModule, diagnostics.as_deref_mut())?;
        let end_at = tkw.try_find_corresponding(
            T::KeywordEndModule,
            tkw.offset - 1,
            diagnostics.as_deref_mut(),
        )?;

        let module_identifier =
            item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
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
                ast,
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
                        ast,
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
                        ast,
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
        let module_items =
            parse_zero_or_more::<ModuleItem>(&mut items_tkw, sc, arenas, ast, diagnostics)?;
        Ok(Module {
            attribute_instances,
            module_identifier,
            module_parameter_port_list,
            ports,
            module_items,
            default_nettype: None,
            time_scale: TimeScale::default(),
        })
    }
}

impl<'a> Consumable<'a> for ModuleItem<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // module_item ::=
        //   port_declaration ;
        // | non_port_module_item

        parse_zero_or_more_while_next::<AttributeInstance>(
            tkw,
            sc,
            arenas,
            ast,
            diagnostics.as_deref_mut(),
            |t| t == T::LeftParenStar,
        )?;
        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::KeywordInput | T::KeywordOutput | T::KeywordInout => {
                let port_declaration =
                    parse::<PortDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::Semicolon, diagnostics)?;
                Ok(Self::PortDeclaration(port_declaration))
            }
            _ => {
                let non_port_module_item =
                    parse::<NonPortModuleItem>(tkw, sc, arenas, ast, diagnostics)?;
                Ok(Self::NonPortModuleItem(non_port_module_item))
            }
        }
    }
}

impl<'a> Consumable<'a> for NonPortModuleItem<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                let parameter_declaration = parse::<ParameterDeclaration>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::ParameterDeclaration(parameter_declaration))
            }
            T::KeywordGenerate => {
                let generate_region =
                    GenerateRegion::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::GenerateRegion(generate_region))
            }
            T::KeywordSpecify => {
                let specify_block =
                    SpecifyBlock::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::SpecifyBlock(specify_block))
            }
            T::KeywordSpecParam => {
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset, "non_port_module_item::specparam_declaration");
                }
                Err(())
            }
            _ => {
                let module_or_generate_item =
                    parse::<ModuleOrGenerateItem>(tkw, sc, arenas, ast, diagnostics)?;
                Ok(Self::ModuleOrGenerateItem(module_or_generate_item))
            }
        }
    }
}

impl<'a> Consumable<'a> for Port<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port ::=
        //   [ port_expression ]
        // | . port_identifier ( [ port_expression ] )

        // @Incomplete: . port_identifier ( [ port_expression ] )

        let port_expression = parse::<PortExpression>(tkw, sc, arenas, ast, diagnostics)?;
        Ok(Self::PortExpression(port_expression))
    }
}

impl<'a> Consumable<'a> for PortExpression<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_expression ::=
        //   port_reference
        // | { port_reference { , port_reference } }

        // @Incomplete: { port_reference { , port_reference } }

        let port_reference = parse::<PortReference>(tkw, sc, arenas, ast, diagnostics)?;
        Ok(Self {
            references: port_reference,
        })
    }
}

impl<'a> Consumable<'a> for PortReference<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
        // port_reference ::=
        //   port_identifier [ [ constant_range_expression ] ]

        // @Incomplete: [ [ constant_range_expression ] ]

        let identifier = item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics)?;
        Ok(Self {
            identifier,
            range: None,
        })
    }
}

impl<'a> Consumable<'a> for PortDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                    parse::<InoutDeclaration>(tkw, sc, arenas, ast, diagnostics)?;
                Ok(Self::Inout(inout_declaration))
            }
            T::KeywordInput => {
                let input_declaration =
                    parse::<InputDeclaration>(tkw, sc, arenas, ast, diagnostics)?;
                Ok(Self::Input(input_declaration))
            }
            T::KeywordOutput => {
                let output_declaration =
                    parse::<OutputDeclaration>(tkw, sc, arenas, ast, diagnostics)?;
                Ok(Self::Output(output_declaration))
            }
            _ => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, *peeked.kind);
                }
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for InoutDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // inout_declaration ::= inout [ net_type ] [ signed ] [ range ] list_of_port_identifiers

        tkw.next_expect(T::KeywordInout, diagnostics.as_deref_mut())?;
        let mut net_type = None;
        if let Some(val) = try_item_parse::<NetType>(tkw, sc, arenas, ast) {
            net_type = Some(val);
        }
        let signed = tkw.next_if_equals(T::KeywordSigned);
        // @Incomplete: [ range ]
        let port_identifiers =
            parse_one_or_more_while::<Identifier>(tkw, sc, arenas, ast, diagnostics, |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            })?;

        Ok(Self {
            net_type,
            signed,
            range: None,
            port_identifiers,
        })
    }
}

impl<'a> Consumable<'a> for InputDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // input_declaration ::= input [ net_type ] [ signed ] [ range ] list_of_port_identifiers

        tkw.next_expect(T::KeywordInput, diagnostics.as_deref_mut())?;
        let mut net_type = None;
        if let Some(val) = try_item_parse::<NetType>(tkw, sc, arenas, ast) {
            net_type = Some(val);
        }
        let signed = tkw.next_if_equals(T::KeywordSigned);
        let mut range = None;
        if tkw.get(tkw.offset).is_some_and(|t| *t.kind == T::LeftBrace) {
            range = Some(parse::<Range>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let port_identifiers =
            parse_one_or_more_while::<Identifier>(tkw, sc, arenas, ast, diagnostics, |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            })?;

        Ok(Self {
            net_type,
            signed,
            range,
            port_identifiers,
        })
    }
}

impl<'a> Consumable<'a> for OutputDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                ast,
                diagnostics.as_deref_mut(),
            )?),
            _ => None,
        };
        let signed = tkw.next_if_equals(T::KeywordSigned);
        let mut range = None;
        if tkw.get(tkw.offset).is_some_and(|t| *t.kind == T::LeftBrace) {
            range = Some(parse::<Range>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let identifiers =
            parse_one_or_more_while::<Identifier>(tkw, sc, arenas, ast, diagnostics, |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            })?;

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
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        Ok(match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
            T::KeywordReg => Self::Register,
            T::KeywordInteger => Self::Integer,
            T::KeywordTime => Self::Time,
            _ => {
                tkw.offset -= 1;
                Self::NetType(NetType::consume(tkw, sc, arenas, ast, diagnostics)?)
            }
        })
    }
}

impl<'a> Consumable<'a> for NetType {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
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
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                Err(())
            }
        }?;
        Ok(result)
    }
}

impl<'a> Consumable<'a> for GenerateRegion<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // generate_region ::= generate { module_or_generate_item } endgenerate

        tkw.next_expect(T::KeywordGenerate, diagnostics.as_deref_mut())?;
        let Some(end_generate) = tkw.find_next_same_depth(T::KeywordEndGenerate) else {
            if let Some(d) = diagnostics {
                d.no_corresponding(tkw.offset - 1, T::KeywordEndGenerate);
            }
            return Err(());
        };
        let module_or_generate_item = parse_zero_or_more::<ModuleOrGenerateItem>(
            &mut tkw.end_at(end_generate),
            sc,
            arenas,
            ast,
            diagnostics,
        )?;
        tkw.offset = end_generate + 1;

        Ok(Self {
            module_or_generate_item,
        })
    }
}

impl<'a> Consumable<'a> for SpecifyBlock<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500
        // specify_block ::= specify { specify_item } endspecify

        tkw.next_expect(T::KeywordSpecify, diagnostics.as_deref_mut())?;
        let items = parse_zero_or_more_while_next::<SpecifyBlockItem>(
            tkw,
            sc,
            arenas,
            ast,
            diagnostics.as_deref_mut(),
            |t| t != T::KeywordEndSpecify,
        )?;
        tkw.next_expect(T::KeywordEndSpecify, diagnostics)?;

        Ok(SpecifyBlock { items })
    }
}

impl<'a> Consumable<'a> for SpecifyBlockItem<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500
        // specify_item ::=
        //   specparam_declaration
        // | pulsestyle_declaration
        // | showcancelled_declaration
        // | path_declaration
        // | system_timing_check

        match *tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?.kind {
            T::KeywordSpecParam => {
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset, "specify_block_item::spec_param");
                }
                Err(())
            }
            T::KeywordPulseStyleOnEvent | T::KeywordPulseStyleOnDetect => {
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset, "specify_block_item::pulsestyle_declaration")
                }
                Err(())
            }
            T::KeywordShowCancelled | T::KeywordNoShowCancelled => {
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset, "specify_block_item::showcancelled_declaration")
                }
                Err(())
            }
            T::LeftParen | T::KeywordIf | T::KeywordIfnone => {
                let path_declaration =
                    PathDeclaration::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::PathDeclaration(path_declaration))
            }
            T::DollarIdent => {
                let system_timing_check =
                    SystemTimingCheck::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::SystemTimingCheck(system_timing_check))
            }
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for PathDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // path_declaration ::=
        //   simple_path_declaration ;
        // | edge_sensitive_path_declaration ;
        // | state_dependent_path_declaration ;

        let mut state_dependent_condition = None;
        let mut edge_identifier = None;
        let mut polarity_operator = None;
        let mut data_source_expression = None;

        let allow_edge_sensitive = !tkw.is_next_equal_to(T::KeywordIfnone);

        if tkw.is_next(|t| matches!(t, T::KeywordIf | T::KeywordIfnone)) {
            state_dependent_condition = Some(parse::<StateDependentCondition>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }

        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;

        if allow_edge_sensitive
            && tkw.is_next(|t| matches!(t, T::KeywordPosedge | T::KeywordNegedge))
        {
            edge_identifier = Some(item_parse::<EdgeIdentifier>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let input_terminal_descriptors = parse_one_or_more_while::<TerminalDescriptor>(
            tkw,
            sc,
            arenas,
            ast,
            diagnostics.as_deref_mut(),
            |tkw| tkw.next_if_equals(T::Comma),
        )?;

        if edge_identifier.is_none() && tkw.is_next(|t| matches!(t, T::Minus | T::Plus)) {
            polarity_operator = Some(item_parse::<PolarityOperator>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let simple_path_declaration_variant =
            match *tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?.kind {
                T::EqualsGreaterThan => {
                    tkw.offset += 1;
                    PathDeclarationVariant::Full
                }
                T::StarGreaterThan => {
                    tkw.offset += 1;
                    PathDeclarationVariant::Parallel
                }
                t => {
                    if let Some(d) = diagnostics {
                        d.unexpected_token(tkw.offset, t);
                    }
                    return Err(());
                }
            };

        let mut is_edge_sensitive_path = edge_identifier.is_some();
        if is_edge_sensitive_path {
            tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        } else if allow_edge_sensitive && polarity_operator.is_none() {
            is_edge_sensitive_path = tkw.next_if_equals(T::LeftParen);
        }

        let output_terminal_descriptors = parse_one_or_more_while::<TerminalDescriptor>(
            tkw,
            sc,
            arenas,
            ast,
            diagnostics.as_deref_mut(),
            |tkw| tkw.next_if_equals(T::Comma),
        )?;

        if is_edge_sensitive_path {
            if tkw.is_next(|t| matches!(t, T::Minus | T::Plus)) {
                polarity_operator = Some(item_parse::<PolarityOperator>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?);
            }

            tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
            data_source_expression = Some(parse::<Expr>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
            tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
        }

        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let path_delay_value =
            parse::<PathDelayValue>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(PathDeclaration {
            state_dependent_condition,
            edge_identifier,
            input_terminal_descriptors,
            polarity_operator,
            variant: simple_path_declaration_variant,
            data_source_expression,
            output_terminal_descriptors,
            path_delay_value,
        })
    }
}

impl<'a> Consumable<'a> for StateDependentCondition<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
            T::KeywordIf => {
                tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
                let module_path_expr =
                    parse::<ModulePathExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                Ok(Self::If(module_path_expr))
            }
            T::KeywordIfnone => Ok(Self::Ifnone),
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset - 1, t);
                }
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for ModulePathExpr<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        let expr = Expr::consume(tkw, sc, arenas, ast, diagnostics)?;
        Ok(Self(expr))
    }
}

impl<'a> Consumable<'a> for EdgeIdentifier {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 501
        // edge_identifier ::= posedge | negedge

        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
            T::KeywordPosedge => Ok(Self::Posedge),
            T::KeywordNegedge => Ok(Self::Negedge),
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset - 1, t);
                }
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for TerminalDescriptor<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500
        // specify_input_terminal_descriptor ::= input_identifier [ [ constant_range_expression ] ]
        // specify_output_terminal_descriptor ::= output_identifier [ [ constant_range_expression ] ]

        let ident = item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let mut constant_range_expr = None;
        if tkw.next_if_equals(T::LeftBrace) {
            constant_range_expr = Some(parse::<ConstantRangeExpression>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
            tkw.next_expect(T::RightBrace, diagnostics)?;
        }
        Ok(Self {
            ident,
            constant_range_expr,
        })
    }
}

impl<'a> Consumable<'a> for PolarityOperator {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500
        // specify_input_terminal_descriptor ::= input_identifier [ [ constant_range_expression ] ]
        // specify_output_terminal_descriptor ::= output_identifier [ [ constant_range_expression ] ]

        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
            T::Minus => Ok(Self::Minus),
            T::Plus => Ok(Self::Plus),
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset - 1, t);
                }
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for PathDelayValue<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500 - 501
        // path_delay_value ::=
        //   list_of_path_delay_expressions
        // | ( list_of_path_delay_expressions )

        let starts_with_left_paren = tkw.next_if_equals(T::LeftParen);
        let list_of_delay_expressions = parse_one_or_more_while::<ConstantMinTypMaxExpression>(
            tkw,
            sc,
            arenas,
            ast,
            diagnostics.as_deref_mut(),
            |tkw| tkw.next_if_equals(T::Comma),
        )?;

        if !matches!(list_of_delay_expressions.len(), 1 | 2 | 3 | 6 | 12) {
            if let Some(d) = diagnostics {
                d.errors.push((
                    arenas.get_range_span(list_of_delay_expressions),
                    crate::parser::ParseErrorReason::Incomplete(
                        "invalid amount of delay expressions",
                    ),
                ));
            }
            return Err(());
        }

        if starts_with_left_paren {
            tkw.next_expect(T::RightParen, diagnostics)?;
        }

        Ok(Self {
            list_of_delay_expressions,
        })
    }
}

impl<'a> Consumable<'a> for SystemTimingCheck {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        let system_timing_check_ident =
            SystemTaskIdentifier::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        let item = match &arenas.ident_table[system_timing_check_ident.0] {
            "setup" => Self::Setup,
            "hold" => Self::Hold,
            "setuphold" => Self::SetupHold,
            "recovery" => Self::Recovery,
            "removal" => Self::Removal,
            "recrem" => Self::Recrem,
            "skew" => Self::Skew,
            "timeskew" => Self::TimeSkew,
            "fullskew" => Self::FullSkew,
            "period" => Self::Period,
            "width" => Self::Width,
            "nochange" => Self::NoChange,
            _ => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset - 1, T::DollarIdent);
                }
                return Err(());
            }
        };

        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let Some(offset) = tkw.find_next_same_depth(T::RightParen) else {
            if let Some(d) = diagnostics {
                d.no_corresponding(tkw.offset - 1, T::RightParen);
            }
            return Err(());
        };
        tkw.offset = offset + 1;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(item)
    }
}

impl<'a> Consumable<'a> for ModuleOrGenerateItem<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        let attribute_instances =
            parse_zero_or_more_while_next(tkw, sc, arenas, ast, diagnostics.as_deref_mut(), |t| {
                t == T::LeftParenStar
            })?;
        let content = ModuleOrGenerateItemContent::consume(tkw, sc, arenas, ast, diagnostics)?;

        Ok(Self {
            attribute_instances,
            content,
        })
    }
}
impl<'a> Consumable<'a> for ModuleOrGenerateItemContent<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                    parse::<InitialConstruct>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::InitialConstruct(initial_construct))
            }
            T::KeywordAlways => {
                let always_construct =
                    parse::<AlwaysConstruct>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::AlwaysConstruct(always_construct))
            }
            T::KeywordAssign => {
                let continous_assign =
                    parse::<ContinousAssign>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::ContinuousAssign(continous_assign))
            }
            T::Ident => {
                let start = tkw.offset;
                let ident = Identifier::consume(tkw, sc, arenas, ast, None);
                tkw.offset = start;

                if ident.is_ok_and(|ident| sc.udps.contains(&ident.0)) {
                    let udp_instance = parse::<UdpInstantiation>(
                        tkw,
                        sc,
                        arenas,
                        ast,
                        diagnostics.as_deref_mut(),
                    )?;
                    Ok(Self::UdpInstantiation(udp_instance))
                } else {
                    let module_instance = parse::<ModuleInstantiation>(
                        tkw,
                        sc,
                        arenas,
                        ast,
                        diagnostics.as_deref_mut(),
                    )?;
                    Ok(Self::ModuleInstantiation(module_instance))
                }
            }
            T::KeywordAnd
            | T::KeywordNand
            | T::KeywordOr
            | T::KeywordNor
            | T::KeywordXor
            | T::KeywordXnor
            | T::KeywordBuf
            | T::KeywordNot
            | T::KeywordBufif0
            | T::KeywordBufif1
            | T::KeywordNotif0
            | T::KeywordNotif1
            | T::KeywordNmos
            | T::KeywordPmos
            | T::KeywordRnmos
            | T::KeywordRpmos
            | T::KeywordCmos
            | T::KeywordRcmos
            | T::KeywordTranif0
            | T::KeywordTranif1
            | T::KeywordRtranif0
            | T::KeywordRtranif1
            | T::KeywordTran
            | T::KeywordRtran => {
                let gate_instance =
                    parse::<GateInstantiation>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
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
            | T::KeywordReal
            | T::KeywordRealtime
            | T::KeywordTime
            | T::KeywordGenvar
            | T::KeywordTask
            | T::KeywordFunction => {
                let module_or_generate_item_declaration = parse::<ModuleOrGenerateItemDeclaration>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::ModuleOrGenerateItemDeclaration(
                    module_or_generate_item_declaration,
                ))
            }
            T::KeywordFor => {
                let loop_generate_construct = parse::<LoopGenerateConstruct>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::LoopGenerateConstruct(loop_generate_construct))
            }
            T::KeywordIf => {
                let if_generate_construct =
                    parse::<IfGenerateConstruct>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::IfGenerateConstruct(if_generate_construct))
            }
            T::KeywordCase => {
                let case_generate_construct = parse::<CaseGenerateConstruct>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::CaseGenerateConstruct(case_generate_construct))
            }
            T::KeywordLocalParam => {
                let local_parameter_declaration = parse::<LocalParameterDeclaration>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::LocalParameterDeclaration(local_parameter_declaration))
            }
            _ => {
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset, "module_or_generate_item");
                }
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for ContinousAssign<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            list_of_net_assignments,
        })
    }
}

impl<'a> Consumable<'a> for NetAssignment<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // continuous_assign ::= assign [ drive_strength ] [ delay3 ] list_of_net_assignments ;
        // list_of_net_assignments ::= net_assignment { , net_assignment }

        let net_lvalue = parse::<NetLValue>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let expression = parse::<Expr>(tkw, sc, arenas, ast, diagnostics)?;

        Ok(Self {
            net_lvalue,
            expression,
        })
    }
}

impl<'a> Consumable<'a> for ModuleInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // module_instantiation ::=
        //   module_identifier [ parameter_value_assignment ]
        //   module_instance { , module_instance } ;

        let module_identifier =
            item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let mut parameter_value_assignment = None;
        if tkw.is_next_equal_to(T::Hash) {
            parameter_value_assignment = Some(parse::<ParameterValueAssignment>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let module_instances = parse_one_or_more_delimited::<ModuleInstance>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(ModuleInstantiation {
            module_identifier,
            parameter_value_assignment,
            module_instances,
        })
    }
}

impl<'a> Consumable<'a> for ParameterValueAssignment<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                ast,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?)
        } else {
            Self::Ordered(parse_one_or_more_delimited::<ConstantExpr>(
                &mut tkw.end_at(end),
                sc,
                arenas,
                ast,
                T::Comma,
                diagnostics,
            )?)
        };
        tkw.offset = end + 1;
        Ok(result)
    }
}

impl<'a> Consumable<'a> for NamedParameterAssignment<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // named_parameter_assignment ::= . parameter_identifier ( [ mintypmax_expression ] )

        tkw.next_expect(T::Dot, diagnostics.as_deref_mut())?;
        let identifier =
            item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let mut expression = None;
        if !tkw.next_if_equals(T::RightParen) {
            expression = Some(parse::<ConstantMinTypMaxExpression>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
            tkw.next_expect(T::RightParen, diagnostics)?;
        }

        Ok(Self {
            identifier,
            expression,
        })
    }
}

impl<'a> Consumable<'a> for ModuleInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // module_instance ::= name_of_module_instance ( [ list_of_port_connections ] )

        let name_of_module_instance =
            item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        let mut range = None;
        if tkw.is_next_equal_to(T::LeftBrace) {
            range = Some(parse::<Range>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }

        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let list_of_port_connections =
            parse::<ListOfPortConnections>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(ModuleInstance {
            name_of_module_instance,
            range,
            list_of_port_connections,
        })
    }
}

impl<'a> Consumable<'a> for ListOfPortConnections<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                ast,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            Ok(Self::Named(named))
        } else {
            let ordered =
                parse_zero_or_more_delimited::<Expr>(tkw, sc, arenas, ast, T::Comma, diagnostics)?;
            Ok(Self::Ordered(ordered))
        }
    }
}

impl<'a> Consumable<'a> for NamedPortConnection<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // named_port_connection ::= { attribute_instance } . port_identifier ( [ expression ] )

        tkw.next_expect(T::Dot, diagnostics.as_deref_mut())?;
        let port_identifier =
            item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let expression = if !tkw
            .get(tkw.offset)
            .is_some_and(|t| *t.kind == T::RightParen)
        {
            Some(parse::<Expr>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?)
        } else {
            None
        };
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(NamedPortConnection {
            port_identifier,
            expression,
        })
    }
}

impl<'a> Consumable<'a> for InitialConstruct<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // initial_construct ::= initial statement

        tkw.next_expect(T::KeywordInitial, diagnostics.as_deref_mut())?;
        let statement = parse::<Statement>(tkw, sc, arenas, ast, diagnostics)?;

        Ok(Self(statement))
    }
}

impl<'a> Consumable<'a> for AlwaysConstruct<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // always_construct ::= always statement

        tkw.next_expect(T::KeywordAlways, diagnostics.as_deref_mut())?;
        let statement = parse::<Statement>(tkw, sc, arenas, ast, diagnostics)?;

        Ok(Self(statement))
    }
}

impl<'a> Consumable<'a> for GateInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                let n_input_gate_instantiation = parse::<NInputGateInstantiation>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::NInput(n_input_gate_instantiation))
            }
            T::KeywordBuf | T::KeywordNot => {
                let n_output_gate_instantiation = parse::<NOutputGateInstantiation>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::NOutput(n_output_gate_instantiation))
            }
            T::KeywordBufif0 | T::KeywordBufif1 | T::KeywordNotif0 | T::KeywordNotif1 => {
                let enable_gate_instantiation = parse::<EnableGateInstantiation>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::Enable(enable_gate_instantiation))
            }
            T::KeywordNmos | T::KeywordPmos | T::KeywordRnmos | T::KeywordRpmos => {
                let mos_switch_instantiation = parse::<MosSwitchInstantiation>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::Mos(mos_switch_instantiation))
            }
            T::KeywordCmos | T::KeywordRcmos => {
                let cmos_switch_instantiation = parse::<CmosSwitchInstantiation>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::Cmos(cmos_switch_instantiation))
            }
            T::KeywordTranif0 | T::KeywordTranif1 | T::KeywordRtranif0 | T::KeywordRtranif1 => {
                let pass_en_switch_instantiation = parse::<PassEnSwitchInstantiation>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::PassEn(pass_en_switch_instantiation))
            }
            T::KeywordTran | T::KeywordRtran => {
                let pass_switch_instantiation = parse::<PassSwitchInstantiation>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::PassSwitch(pass_switch_instantiation))
            }
            T::KeywordPullup => {
                tkw.offset += 1;
                let pullup_instantiation = parse::<PullGateInstantiation>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::Pullup(pullup_instantiation))
            }
            T::KeywordPulldown => {
                tkw.offset += 1;
                let pulldown_instantiation = parse::<PullGateInstantiation>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::Pulldown(pulldown_instantiation))
            }
            _ => {
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset, "gate_instantiation");
                }
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for EnableGateInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // enable_gatetype [drive_strength] [delay3] enable_gate_instance { , enable_gate_instance } ;

        let gatetype =
            item_parse::<EnableGateType>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let mut drive_strength = None;
        if tkw.is_next_equal_to(T::LeftParen)
            && tkw
                .get(tkw.offset + 1)
                .is_some_and(|t| is_drive_strength_kw(*t.kind))
        {
            drive_strength = Some(item_parse::<DriveStrength>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }

        let mut delay = None;
        if tkw.is_next_equal_to(T::Hash) {
            delay = Some(parse::<Delay3>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let instances = parse_one_or_more_delimited::<EnableGateInstance>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            gatetype,
            drive_strength,
            delay,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for EnableGateType {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // enable_gatetype ::= enable_gatetype ::= bufif0 | bufif1 | notif0 | notif1

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        let value = match *t.kind {
            T::KeywordBufif0 => Self::BufIf0,
            T::KeywordBufif1 => Self::BufIf1,
            T::KeywordNotif0 => Self::NotIf0,
            T::KeywordNotif1 => Self::NotIf1,
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                return Err(());
            }
        };

        Ok(value)
    }
}

impl<'a> Consumable<'a> for EnableGateInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // enable_gate_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal , enable_terminal )

        let name = try_parse::<NameOfGateInstance>(tkw, sc, arenas, ast);
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let output_terminal = parse::<NetLValue>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let input_terminal = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let enable_terminal = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(Self {
            name,
            output_terminal,
            input_terminal,
            enable_terminal,
        })
    }
}

impl<'a> Consumable<'a> for CmosSwitchInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // cmos_switchtype [delay3] cmos_switch_instance { , cmos_switch_instance } ;

        let gatetype =
            item_parse::<CmosSwitchType>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        let mut delay = None;
        if tkw.is_next_equal_to(T::Hash) {
            delay = Some(parse::<Delay3>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let instances = parse_one_or_more_delimited::<CmosSwitchInstance>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            gatetype,
            delay,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for PassSwitchInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // pass_switchtype pass_switch_instance { , pass_switch_instance } ;

        let gatetype =
            item_parse::<PassSwitchType>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        let instances = parse_one_or_more_delimited::<PassSwitchInstance>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            gatetype,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for PassEnSwitchInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // pass_en_switchtype [delay2] pass_enable_switch_instance { , pass_enable_switch_instance } ;

        let gatetype =
            item_parse::<PassEnSwitchType>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let mut delay = None;
        if tkw.is_next_equal_to(T::Hash) {
            delay = Some(parse::<Delay2>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let instances = parse_one_or_more_delimited::<PassEnSwitchInstance>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            gatetype,
            delay,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for PullGateInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        //   pullup [pullup_strength] pull_gate_instance { , pull_gate_instance } ;
        // | pulldown [pulldown_strength] pull_gate_instance { , pull_gate_instance } ;

        let pullup_strength = None;
        // @Incomplete: pullup / pulldown strength
        let instances = parse_one_or_more_delimited::<PullGateInstance>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            pullup_strength,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for PullGateInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // pull_gate_instance ::= [ name_of_gate_instance ] ( output_terminal )

        let name = try_parse::<NameOfGateInstance>(tkw, sc, arenas, ast);
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let output_terminal = parse::<NetLValue>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(Self {
            name,
            output_terminal,
        })
    }
}

impl<'a> Consumable<'a> for MosSwitchInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // mos_switchtype [delay3] mos_switch_instance { , mos_switch_instance } ;

        let gatetype =
            item_parse::<MosSwitchType>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        let mut delay = None;
        if tkw.is_next_equal_to(T::Hash) {
            delay = Some(parse::<Delay3>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let instances = parse_one_or_more_delimited::<EnableGateInstance>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            gatetype,
            delay,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for MosSwitchType {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // enable_gatetype ::= enable_gatetype ::= bufif0 | bufif1 | notif0 | notif1

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        let value = match *t.kind {
            T::KeywordNmos => Self::NMos,
            T::KeywordPmos => Self::PMos,
            T::KeywordRnmos => Self::RNMos,
            T::KeywordRpmos => Self::RPMos,
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                return Err(());
            }
        };

        Ok(value)
    }
}

impl<'a> Consumable<'a> for CmosSwitchType {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // cmos_switchtype ::= cmos | rcmos

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        let value = match *t.kind {
            T::KeywordCmos => Self::Cmos,
            T::KeywordRcmos => Self::Rcmos,
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                return Err(());
            }
        };

        Ok(value)
    }
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// pass_switchtype ::= tran | rtran
impl<'a> Consumable<'a> for PassSwitchType {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // pass_switchtype ::= tran | rtran

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        let value = match *t.kind {
            T::KeywordTran => Self::Tran,
            T::KeywordRtran => Self::RTran,
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                return Err(());
            }
        };

        Ok(value)
    }
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// pass_en_switchtype ::= tranif0 | tranif1 | rtranif1 | rtranif0
impl<'a> Consumable<'a> for PassEnSwitchType {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // pass_en_switchtype ::= tranif0 | tranif1 | rtranif1 | rtranif0

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        let value = match *t.kind {
            T::KeywordTranif0 => Self::Tranif0,
            T::KeywordTranif1 => Self::Tranif1,
            T::KeywordRtranif0 => Self::Rtranif0,
            T::KeywordRtranif1 => Self::Rtranif1,
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                return Err(());
            }
        };

        Ok(value)
    }
}

impl<'a> Consumable<'a> for PassSwitchInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // pass_switch_instance ::= [ name_of_gate_instance ] ( inout_terminal , inout_terminal )

        let name = try_parse::<NameOfGateInstance>(tkw, sc, arenas, ast);
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let fst = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let snd = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(Self { name, fst, snd })
    }
}

impl<'a> Consumable<'a> for PassEnSwitchInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // pass_enable_switch_instance ::= [ name_of_gate_instance ] ( inout_terminal , inout_terminal , enable_terminal )

        let name = try_parse::<NameOfGateInstance>(tkw, sc, arenas, ast);
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let fst = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let snd = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let enable_terminal = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(Self {
            name,
            fst,
            snd,
            enable_terminal,
        })
    }
}

impl<'a> Consumable<'a> for CmosSwitchInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // cmos_switch_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal , ncontrol_terminal , pcontrol_terminal )

        let name = try_parse::<NameOfGateInstance>(tkw, sc, arenas, ast);
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let output_terminal = parse::<NetLValue>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let input_terminal = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let ncontrol_terminal = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let pcontrol_terminal = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(Self {
            name,
            output_terminal,
            input_terminal,
            ncontrol_terminal,
            pcontrol_terminal,
        })
    }
}

impl<'a> Consumable<'a> for MosSwitchInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // mos_switch_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal , enable_terminal )

        let name = try_parse::<NameOfGateInstance>(tkw, sc, arenas, ast);
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let output_terminal = parse::<NetLValue>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let input_terminal = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let enable_terminal = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(Self {
            name,
            output_terminal,
            input_terminal,
            enable_terminal,
        })
    }
}

impl<'a> Consumable<'a> for NInputGateInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // n_input_gatetype [drive_strength] [delay2] n_input_gate_instance { , n_input_gate_instance } ;

        let gatetype =
            item_parse::<NInputGateType>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        // @Incomplete: drive_strength
        // @Incomplete: delay2
        let instances = parse_one_or_more_delimited::<NInputGateInstance>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            gatetype,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for NInputGateType {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
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
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                return Err(());
            }
        };

        Ok(value)
    }
}

impl<'a> Consumable<'a> for NInputGateInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // n_input_gate_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal { , input_terminal } )

        let name = try_parse::<NameOfGateInstance>(tkw, sc, arenas, ast);
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let output_terminal = parse::<NetLValue>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let input_terminals = parse_one_or_more_delimited::<Expr>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(Self {
            name,
            output_terminal,
            input_terminals,
        })
    }
}

impl<'a> Consumable<'a> for NOutputGateInstantiation<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // n_output_gatetype [drive_strength] [delay2] n_output_gate_instance { , n_output_gate_instance }

        let gatetype =
            item_parse::<NOutputGateType>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        // @Incomplete: drive_strength
        // @Incomplete: delay2
        let instances = parse_one_or_more_delimited::<NOutputGateInstance>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            gatetype,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for NOutputGateType {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // n_output_gatetype ::= buf | not

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        let value = match *t.kind {
            T::KeywordBuf => Self::Buf,
            T::KeywordNot => Self::Not,
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                return Err(());
            }
        };

        Ok(value)
    }
}

impl<'a> Consumable<'a> for NOutputGateInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // n_output_gate_instance ::= [ name_of_gate_instance ] ( output_terminal { , output_terminal } , input_terminal )

        let name = try_parse::<NameOfGateInstance>(tkw, sc, arenas, ast);
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let output_terminal = parse::<NetLValue>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let output_terminals = AstIdRange::single(output_terminal);
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let input_terminal = parse::<Expr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        if tkw.is_next_equal_to(T::Comma) {
            if let Some(d) = diagnostics {
                d.incomplete(tkw.offset, "Multi output gate like this");
            }
            return Err(());
        }
        tkw.next_expect(T::RightParen, diagnostics)?;

        Ok(Self {
            name,
            output_terminals,
            input_terminal,
        })
    }
}

impl<'a> Consumable<'a> for NameOfGateInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
        // name_of_gate_instance ::= gate_instance_identifier [ range ]

        let identifier =
            item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let mut range = None;
        if tkw.is_next_equal_to(T::LeftBrace) {
            range = Some(parse::<Range>(tkw, sc, arenas, ast, diagnostics)?);
        }

        Ok(Self { identifier, range })
    }
}

impl<'a> Consumable<'a> for ModuleOrGenerateItemDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                    parse::<NetDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::Net(net_declaration))
            }
            T::KeywordReg => {
                let reg_declaration =
                    parse::<RegDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::Reg(reg_declaration))
            }
            T::KeywordInteger => {
                let integer_declaration =
                    parse::<IntegerDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::Integer(integer_declaration))
            }
            T::KeywordTime => {
                let time_declaration =
                    parse::<TimeDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::Time(time_declaration))
            }
            T::KeywordReal => {
                let real_declaration =
                    parse::<RealDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::Real(real_declaration))
            }
            T::KeywordRealtime => {
                let realtime_declaration =
                    parse::<RealtimeDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::Realtime(realtime_declaration))
            }
            T::KeywordGenvar => {
                let genvar_declaration =
                    parse::<GenvarDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::Genvar(genvar_declaration))
            }
            T::KeywordTask => {
                let task_declaration =
                    parse::<TaskDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::Task(task_declaration))
            }
            T::KeywordFunction => {
                let function_declaration =
                    parse::<FunctionDeclaration>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                Ok(Self::Function(function_declaration))
            }
            _ => {
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset, "module_or_generate_item_declaration");
                }
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for NetDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
        let net_type = item_parse::<NetType>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let signed = tkw.next_if_equals(T::KeywordSigned);
        let mut range = None;
        if tkw.is_next_equal_to(T::LeftBrace) {
            range = Some(parse::<Range>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
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
                ast,
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
                ast,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?)
        };

        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            net_type,
            signed,
            range,
            nets,
        })
    }
}

impl<'a> Consumable<'a> for NetDeclAssignment<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // net_decl_assignment ::= net_identifier = expression

        let ident = item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let expr = parse::<Expr>(tkw, sc, arenas, ast, diagnostics)?;

        Ok(Self { ident, expr })
    }
}

impl<'a> Consumable<'a> for NetIdent<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // net_identifier { dimension }

        let ident = item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let dimension =
            parse_zero_or_more_while_next::<Dimension>(tkw, sc, arenas, ast, diagnostics, |t| {
                t == T::LeftBrace
            })?;

        Ok(Self { ident, dimension })
    }
}

impl<'a> Consumable<'a> for Dimension<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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

        let lhs = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut());
        if lhs.is_err() {
            tkw.offset = colon;
        }

        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;

        let rhs = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut());
        if rhs.is_err() {
            tkw.offset = end_brace;
        }

        tkw.next_expect(T::RightBrace, diagnostics)?;

        // Reporting errors from both left- and right-hand side.
        let (Ok(lhs), Ok(rhs)) = (lhs, rhs) else {
            return Err(());
        };
        Ok(Dimension { lhs, rhs })
    }
}

impl<'a> Consumable<'a> for GenvarDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
            ast,
            diagnostics.as_deref_mut(),
            |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            },
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self { identifiers })
    }
}

impl<'a> Consumable<'a> for TaskDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
        let ident = item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let (task_ports, block_item_decls) = if tkw.next_if_equals(T::LeftParen) {
            let mut fst = true;
            let task_ports = parse_zero_or_more_while::<TaskPortItem>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
                |tkw| {
                    if fst
                        && matches!(
                            tkw.get(tkw.offset).map(|t| t.kind),
                            Some(
                                T::LeftParenStar
                                    | T::KeywordInput
                                    | T::KeywordOutput
                                    | T::KeywordInout
                            )
                        )
                    {
                        fst = false;
                        true
                    } else if !fst
                        && tkw.is_next_equal_to(T::Comma)
                        && matches!(
                            tkw.get(tkw.offset + 1).map(|t| t.kind),
                            Some(
                                T::LeftParenStar
                                    | T::KeywordInput
                                    | T::KeywordOutput
                                    | T::KeywordInout
                            )
                        )
                    {
                        tkw.offset += 1;
                        true
                    } else {
                        false
                    }
                },
            )?;

            tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
            tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

            let block_item_decls = parse_zero_or_more_while_next::<BlockItemDeclaration>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
                |t| {
                    matches!(
                        t,
                        T::KeywordReg
                            | T::KeywordInteger
                            | T::KeywordTime
                            | T::KeywordReal
                            | T::KeywordRealtime
                            | T::KeywordEvent
                            | T::KeywordLocalParam
                            | T::KeywordParameter
                    )
                },
            )?;

            (task_ports, block_item_decls)
        } else {
            tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

            let mut ports = Vec::new();
            let mut ports_trs = Vec::new();
            let mut block_item_decls = Vec::new();
            let mut block_item_decls_trs = Vec::new();

            loop {
                let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
                match *peeked.kind {
                    T::KeywordReg
                    | T::KeywordInteger
                    | T::KeywordTime
                    | T::KeywordReal
                    | T::KeywordRealtime
                    | T::KeywordEvent
                    | T::KeywordLocalParam
                    | T::KeywordParameter => {
                        let start = tkw.offset;
                        block_item_decls.push(BlockItemDeclaration::consume(
                            tkw,
                            sc,
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?);
                        let token_range = TokenRange {
                            start,
                            end: tkw.offset,
                        };
                        block_item_decls_trs.push(token_range);
                    }
                    T::KeywordInput | T::KeywordOutput | T::KeywordInout => {
                        let start = tkw.offset;
                        ports.push(TaskPortItem::consume(
                            tkw,
                            sc,
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?);
                        let token_range = TokenRange {
                            start,
                            end: tkw.offset,
                        };
                        ports_trs.push(token_range);
                        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                    }
                    _ => break,
                }
            }

            let task_ports = push_range(arenas, ast, ports, ports_trs);
            let block_item_decls = push_range(arenas, ast, block_item_decls, block_item_decls_trs);
            (task_ports, block_item_decls)
        };
        let statement_or_null =
            parse::<StatementOrNull>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::KeywordEndTask, diagnostics)?;

        Ok(Self {
            ident,
            automatic,
            task_ports,
            block_item_decls,
            statement_or_null,
        })
    }
}

impl<'a> Consumable<'a> for RegDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // reg_declaration ::= reg [ signed ] [ range ] list_of_variable_identifiers ;

        tkw.next_expect(T::KeywordReg, diagnostics.as_deref_mut())?;
        let signed = tkw.next_if_equals(T::KeywordSigned);
        let mut range = None;
        if tkw.is_next_equal_to(T::LeftBrace) {
            range = Some(parse::<Range>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }
        let variable_types = parse_one_or_more_delimited::<VariableType>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self {
            signed,
            range,
            variable_types,
        })
    }
}

impl<'a> Consumable<'a> for VariableType<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // variable_type ::=
        //   variable_identifier { dimension } |
        //   variable_identifier = constant_expression
        // @Incomplete

        let identifier =
            item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let variant = if tkw.next_if_equals(T::Equals) {
            let expr = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
            VariableTypeVariant::ConstantExpr(expr)
        } else {
            let dimensions =
                parse_zero_or_more_while_next(tkw, sc, arenas, ast, diagnostics, |t| {
                    t == T::LeftBrace
                })?;
            VariableTypeVariant::Dimensions(dimensions)
        };
        Ok(Self {
            identifier,
            variant,
        })
    }
}

impl<'a> Consumable<'a> for IntegerDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // integer_declaration ::= integer list_of_variable_identifiers ;

        tkw.next_expect(T::KeywordInteger, diagnostics.as_deref_mut())?;
        let variable_types = parse_one_or_more_delimited::<VariableType>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self { variable_types })
    }
}

impl<'a> Consumable<'a> for TimeDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // time_declaration ::= time list_of_variable_identifiers ;

        tkw.next_expect(T::KeywordTime, diagnostics.as_deref_mut())?;
        let variable_types = parse_one_or_more_delimited::<VariableType>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self { variable_types })
    }
}

impl<'a> Consumable<'a> for RealDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // real_declaration ::= real list_of_real_identifiers ;

        tkw.next_expect(T::KeywordReal, diagnostics.as_deref_mut())?;
        let variable_types = parse_one_or_more_delimited::<VariableType>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self { variable_types })
    }
}

impl<'a> Consumable<'a> for RealtimeDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
        // realtime_declaration ::= realtime list_of_real_identifiers ;

        tkw.next_expect(T::KeywordRealtime, diagnostics.as_deref_mut())?;
        let variable_types = parse_one_or_more_delimited::<VariableType>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics)?;

        Ok(Self { variable_types })
    }
}

impl<'a> Consumable<'a> for LocalParameterDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // local_parameter_declaration ::=
        //   localparam [ signed ] [ range ] list_of_param_assignments
        // | localparam parameter_type list_of_param_assignments

        tkw.next_expect(T::KeywordLocalParam, diagnostics.as_deref_mut())?;
        let typing =
            parse::<ParameterDeclarationTyping>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let assignments = parse_one_or_more_delimited_and_after::<ParamAssignment>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            T::Ident,
            diagnostics,
        )?;

        Ok(Self {
            typing,
            assignments,
        })
    }
}

impl<'a> Consumable<'a> for ParameterDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
        // parameter_declaration ::=
        //   parameter [ signed ] [ range ] list_of_param_assignments
        // | parameter parameter_type list_of_param_assignments

        tkw.next_expect(T::KeywordParameter, diagnostics.as_deref_mut())?;
        let typing =
            parse::<ParameterDeclarationTyping>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let assignments = parse_one_or_more_delimited_and_after::<ParamAssignment>(
            tkw,
            sc,
            arenas,
            ast,
            T::Comma,
            T::Ident,
            diagnostics,
        )?;

        Ok(Self {
            typing,
            assignments,
        })
    }
}

impl<'a> Consumable<'a> for ParameterDeclarationTyping<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        diagnostics: Option<&mut Diagnostics>,
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
                    range = Some(parse::<Range>(tkw, sc, arenas, ast, diagnostics)?);
                }
                Self::None(signed, range)
            }
        })
    }
}

impl<'a> Consumable<'a> for ParamAssignment<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // param_assignment ::= parameter_identifier = constant_mintypmax_expression

        let param = item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let constant = parse::<ConstantMinTypMaxExpression>(tkw, sc, arenas, ast, diagnostics)?;
        Ok(Self { param, constant })
    }
}

impl<'a> Consumable<'a> for Range<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
        // range ::= [ msb_constant_expression : lsb_constant_expression ]

        tkw.next_expect(T::LeftBrace, diagnostics.as_deref_mut())?;
        let msb = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
        let lsb = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightBrace, diagnostics)?;

        Ok(Self { msb, lsb })
    }
}

impl<'a> Consumable<'a> for LoopGenerateConstruct<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // loop_generate_construct ::= for ( genvar_initialization ; genvar_expression ; genvar_iteration ) generate_block

        tkw.next_expect(T::KeywordFor, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;

        let initialization =
            parse::<GenvarAssignment>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
        let condition = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
        let iteration =
            parse::<GenvarAssignment>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        let block = parse::<GenerateBlock>(tkw, sc, arenas, ast, diagnostics)?;

        Ok(Self {
            initialization,
            condition,
            iteration,
            block,
        })
    }
}

impl<'a> Consumable<'a> for IfGenerateConstruct<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
        // if_generate_construct ::= if ( constant_expression ) generate_block_or_null
        //   [ else generate_block_or_null ]

        tkw.next_expect(T::KeywordIf, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;

        let condition = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        let truthy =
            parse::<Option<GenerateBlock>>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        let mut falsy = None;
        if tkw.next_if_equals(T::KeywordElse) {
            falsy = Some(parse::<Option<GenerateBlock>>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics,
            )?);
        }

        Ok(Self {
            condition,
            truthy,
            falsy,
        })
    }
}

impl<'a> Consumable<'a> for CaseGenerateConstruct<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // case_generate_construct ::= case ( constant_expression ) case_generate_item { case_generate_item } endcase

        let case_offset = tkw.offset;
        tkw.next_expect(T::KeywordCase, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;

        let value = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        let Some(end) = tkw.find_next_same_depth(T::KeywordEndCase) else {
            if let Some(d) = diagnostics {
                d.no_corresponding(case_offset, T::KeywordEndCase);
            }
            return Err(());
        };

        let items = parse_one_or_more::<CaseGenerateItem>(
            &mut tkw.end_at(end),
            sc,
            arenas,
            ast,
            diagnostics,
        )?;
        tkw.offset = end + 1;

        Ok(Self { value, items })
    }
}

impl<'a> Consumable<'a> for CaseGenerateItem<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                if let Some(d) = diagnostics {
                    d.no_corresponding(tkw.offset, T::Colon);
                }
                return Err(());
            };

            let values = parse_one_or_more_delimited::<ConstantExpr>(
                &mut tkw.end_at(end),
                sc,
                arenas,
                ast,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            tkw.offset = end + 1;
            CaseGeneratePattern::Exprs(values)
        };

        let block = parse::<Option<GenerateBlock>>(tkw, sc, arenas, ast, diagnostics)?;
        Ok(Self { pattern, block })
    }
}

impl<'a> Consumable<'a> for GenvarAssignment<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // genvar_initialization ::= genvar_identifier = constant_expression
        // genvar_iteration      ::= genvar_identifier = genvar_expression

        let ident = item_parse(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let expr = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics)?;

        Ok(Self { ident, expr })
    }
}

impl<'a> Consumable<'a> for GenerateBlock<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // generate_block ::=
        //   module_or_generate_item
        // | begin [ : generate_block_identifier ] { module_or_generate_item } end

        if tkw.next_if_equals(T::KeywordBegin) {
            let Some(end) = tkw.find_next_same_depth(T::KeywordEnd) else {
                if let Some(d) = diagnostics {
                    d.no_corresponding(tkw.offset - 1, T::KeywordEnd);
                }
                return Err(());
            };

            let mut identifier = None;
            if tkw.next_if_equals(T::Colon) {
                identifier = Some(item_parse::<Identifier>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?);
            }
            let module_or_generate_item = parse_zero_or_more::<ModuleOrGenerateItem>(
                &mut tkw.end_at(end),
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?;
            tkw.offset = end + 1;
            Ok(Self::BeginEnd(identifier, module_or_generate_item))
        } else {
            Ok(Self::ModuleOrGenerateItem(parse::<ModuleOrGenerateItem>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics,
            )?))
        }
    }
}

impl<'a> Consumable<'a> for Option<GenerateBlock<'a>> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // generate_block_or_null ::= generate_block |;

        if tkw.next_if_equals(T::Semicolon) {
            Ok(None)
        } else {
            GenerateBlock::consume(tkw, sc, arenas, ast, diagnostics).map(Some)
        }
    }
}

impl<'a> Consumable<'a> for BlockItemDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // block_item_declaration ::=
        //   { attribute_instance } reg [ signed ] [ range ] list_of_block_variable_identifiers ;
        // | { attribute_instance } integer list_of_block_variable_identifiers ;
        // | { attribute_instance } time list_of_block_variable_identifiers ;
        // | { attribute_instance } real list_of_block_real_identifiers ;
        // | { attribute_instance } realtime list_of_block_real_identifiers ;
        // | { attribute_instance } event_declaration
        // | { attribute_instance } local_parameter_declaration ;
        // | { attribute_instance } parameter_declaration ;

        match *tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?.kind {
            T::KeywordReg => {
                tkw.offset += 1;
                let signed = tkw.next_if_equals(T::KeywordSigned);
                let mut range = None;
                if tkw.is_next_equal_to(T::LeftBrace) {
                    range = Some(parse::<Range>(
                        tkw,
                        sc,
                        arenas,
                        ast,
                        diagnostics.as_deref_mut(),
                    )?);
                }
                let identifiers = parse_one_or_more_delimited::<VariableType>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    T::Comma,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::Reg {
                    signed,
                    range,
                    identifiers,
                })
            }
            T::KeywordInteger => {
                tkw.offset += 1;
                let identifiers = parse_one_or_more_delimited::<VariableType>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    T::Comma,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::Integer(identifiers))
            }
            T::KeywordReal => {
                tkw.offset += 1;
                let identifiers = parse_one_or_more_delimited::<VariableType>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    T::Comma,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::Real(identifiers))
            }
            T::KeywordLocalParam => {
                let local_parameter_declaration = parse::<LocalParameterDeclaration>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::LocalParameterDeclaration(local_parameter_declaration))
            }
            T::KeywordParameter => {
                let parameter_declaration = parse::<ParameterDeclaration>(
                    tkw,
                    sc,
                    arenas,
                    ast,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::ParameterDeclaration(parameter_declaration))
            }
            T::KeywordTime | T::KeywordRealtime | T::KeywordEvent => {
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset, "block_item_declaration");
                }
                Err(())
            }
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset - 1, t);
                }
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for TfType<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // tf_input_declaration ::=
        //   input [ reg ] [ signed ] [ range ] list_of_port_identifiers
        // | input task_port_type list_of_port_identifiers
        // task_port_type ::= integer | real | realtime | time

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match *peeked.kind {
            T::KeywordReg | T::KeywordSigned | T::LeftBrace => {
                let reg = tkw.next_if_equals(T::KeywordReg);
                let signed = tkw.next_if_equals(T::KeywordSigned);
                let mut range = None;
                if tkw.is_next_equal_to(T::LeftBrace) {
                    range = Some(parse::<Range>(tkw, sc, arenas, ast, diagnostics)?);
                }
                Ok(Self::Net { reg, signed, range })
            }
            T::KeywordInteger => {
                tkw.offset += 1;
                Ok(Self::Integer)
            }
            T::KeywordReal => {
                tkw.offset += 1;
                Ok(Self::Real)
            }
            T::KeywordRealtime => {
                tkw.offset += 1;
                Ok(Self::Realtime)
            }
            T::KeywordTime => {
                tkw.offset += 1;
                Ok(Self::Time)
            }
            _ => Ok(Self::Net {
                reg: false,
                signed: false,
                range: None,
            }),
        }
    }
}

impl<'a> Consumable<'a> for TfInputDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // tf_input_declaration ::=
        //   input [ reg ] [ signed ] [ range ] list_of_port_identifiers
        // | input task_port_type list_of_port_identifiers

        tkw.next_expect(T::KeywordInput, diagnostics.as_deref_mut())?;
        let tf_type = TfType::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let port_identifiers =
            parse_one_or_more_while::<Identifier>(tkw, sc, arenas, ast, diagnostics, |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            })?;

        Ok(Self {
            tf_type,
            port_identifiers,
        })
    }
}

impl<'a> Consumable<'a> for TfOutputDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // tf_output_declaration ::=
        //   output [ reg ] [ signed ] [ range ] list_of_port_identifiers
        // | output task_port_type list_of_port_identifiers

        tkw.next_expect(T::KeywordOutput, diagnostics.as_deref_mut())?;
        let tf_type = TfType::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let port_identifiers =
            parse_one_or_more_while::<Identifier>(tkw, sc, arenas, ast, diagnostics, |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            })?;

        Ok(Self {
            tf_type,
            port_identifiers,
        })
    }
}

impl<'a> Consumable<'a> for TfInoutDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
        // tf_output_declaration ::=
        //   inout [ reg ] [ signed ] [ range ] list_of_port_identifiers
        // | inout task_port_type list_of_port_identifiers

        tkw.next_expect(T::KeywordInout, diagnostics.as_deref_mut())?;
        let tf_type = TfType::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let port_identifiers =
            parse_one_or_more_while::<Identifier>(tkw, sc, arenas, ast, diagnostics, |tkw| {
                tkw.get(tkw.offset + 1).is_some_and(|t| *t.kind == T::Ident)
                    && tkw.next_if_equals(T::Comma)
            })?;

        Ok(Self {
            tf_type,
            port_identifiers,
        })
    }
}

impl<'a> Consumable<'a> for TaskPortItem<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
        // task_port_item ::=
        //   { attribute_instance } tf_input_declaration
        // | { attribute_instance } tf_output_declaration
        // | { attribute_instance } tf_inout_declaration

        let attribute_instances =
            parse_zero_or_more_while_next(tkw, sc, arenas, ast, diagnostics.as_deref_mut(), |t| {
                t == T::LeftParenStar
            })?;
        let t = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        use TaskPortItemContent as C;
        let content = match *t.kind {
            T::KeywordInput => C::Input(TfInputDeclaration::consume(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?),
            T::KeywordOutput => C::Output(TfOutputDeclaration::consume(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?),
            T::KeywordInout => C::Inout(TfInoutDeclaration::consume(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?),
            t => {
                if let Some(d) = diagnostics {
                    d.unexpected_token(tkw.offset, t);
                }
                return Err(());
            }
        };
        Ok(Self {
            attribute_instances,
            content,
        })
    }
}

impl<'a> Consumable<'a> for FunctionRangeOrType<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
        // function_range_or_type ::= [ signed ] [ range ] | integer | real | realtime | time

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match *peeked.kind {
            T::KeywordSigned => {
                tkw.offset += 1;
                let mut range = None;
                if tkw.is_next_equal_to(T::LeftBrace) {
                    range = Some(parse::<Range>(
                        tkw,
                        sc,
                        arenas,
                        ast,
                        diagnostics.as_deref_mut(),
                    )?);
                }
                Ok(Self::Signed(range))
            }
            T::KeywordInteger => {
                tkw.offset += 1;
                Ok(Self::Integer)
            }
            T::KeywordReal => {
                tkw.offset += 1;
                Ok(Self::Real)
            }
            T::KeywordRealtime => {
                tkw.offset += 1;
                Ok(Self::Realtime)
            }
            T::KeywordTime => {
                tkw.offset += 1;
                Ok(Self::Time)
            }
            _ => {
                let mut range = None;
                if tkw.is_next_equal_to(T::LeftBrace) {
                    range = Some(parse::<Range>(tkw, sc, arenas, ast, diagnostics)?);
                }
                Ok(Self::Unsigned(range))
            }
        }
    }
}

impl<'a> Consumable<'a> for FunctionDeclaration<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
        // function_declaration ::=
        //   function [ automatic ] [ function_range_or_type ] function_identifier ;
        //     function_item_declaration { function_item_declaration }
        //     function_statement
        //   endfunction
        // | function [ automatic ] [ function_range_or_type ] function_identifier ( function_port_list ) ;
        //     { block_item_declaration }
        //     function_statement
        //   endfunction

        tkw.next_expect(T::KeywordFunction, diagnostics.as_deref_mut())?;
        let automatic = tkw.next_if_equals(T::KeywordAutomatic);
        let range_or_type =
            parse::<FunctionRangeOrType>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let ident = item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

        let (tf_input_decls, block_item_decls) = if tkw.next_if_equals(T::LeftParen) {
            let tf_input_decls = parse_one_or_more_delimited(
                tkw,
                sc,
                arenas,
                ast,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
            tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
            let block_item_decls = parse_zero_or_more_while_next::<BlockItemDeclaration>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
                |t| {
                    matches!(
                        t,
                        T::KeywordReg
                            | T::KeywordInteger
                            | T::KeywordTime
                            | T::KeywordReal
                            | T::KeywordRealtime
                            | T::KeywordEvent
                            | T::KeywordLocalParam
                            | T::KeywordParameter
                    )
                },
            )?;

            (tf_input_decls, block_item_decls)
        } else {
            tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

            let mut tf_input_decls = Vec::new();
            let mut tf_input_decls_trs = Vec::new();
            let mut block_item_decls = Vec::new();
            let mut block_item_decls_trs = Vec::new();

            loop {
                let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
                match *peeked.kind {
                    T::KeywordReg
                    | T::KeywordInteger
                    | T::KeywordTime
                    | T::KeywordReal
                    | T::KeywordRealtime
                    | T::KeywordEvent
                    | T::KeywordLocalParam
                    | T::KeywordParameter => {
                        let start = tkw.offset;
                        block_item_decls.push(BlockItemDeclaration::consume(
                            tkw,
                            sc,
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?);
                        let token_range = TokenRange {
                            start,
                            end: tkw.offset,
                        };
                        block_item_decls_trs.push(token_range);
                    }
                    T::KeywordInput => {
                        let start = tkw.offset;
                        tf_input_decls.push(TfInputDeclaration::consume(
                            tkw,
                            sc,
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?);
                        let token_range = TokenRange {
                            start,
                            end: tkw.offset,
                        };
                        tf_input_decls_trs.push(token_range);

                        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                    }
                    _ => break,
                }
            }

            let tf_input_decls = push_range(arenas, ast, tf_input_decls, tf_input_decls_trs);
            let block_item_decls = push_range(arenas, ast, block_item_decls, block_item_decls_trs);
            (tf_input_decls, block_item_decls)
        };
        let statement = parse::<Statement>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::KeywordEndFunction, diagnostics)?;

        Ok(Self {
            automatic,
            range_or_type,
            ident,
            tf_input_decls,
            block_item_decls,
            statement,
        })
    }
}
