use vogls_ir::token_range::TokenRange;

use crate::ast::constant_expr::ConstantExpr;
use crate::ast::module::Range;
use crate::ast::statement::NetLValue;
use crate::ast::udp::{
    UdpBody, UdpCombinationalEntry, UdpDeclaration, UdpDeclarationPortList, UdpEdgeIndicator,
    UdpEdgeSymbol, UdpInitVal, UdpInitialStatement, UdpInputDeclaration, UdpInstance,
    UdpInstantiation, UdpLevelSymbol, UdpNextState, UdpOutputDeclaration, UdpOutputSymbol,
    UdpPortDeclaration, UdpPorts, UdpRegDeclaration, UdpSequentialEntry,
};
use crate::ast::{AstIdRange, AstItem, Identifier};
use crate::parser::utils::{
    item_parse, parse, parse_one_or_more_while, parse_one_or_more_while_next,
    parse_zero_or_more_while, parse_zero_or_more_while_next,
};
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, Diagnostics, ParserScratches, TokenWalker};

impl<'a> Consumable<'a> for UdpDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // udp_declaration ::=
        //   { attribute_instance } primitive udp_identifier ( udp_port_list ) ;
        //     udp_port_declaration { udp_port_declaration }
        //     udp_body
        //   endprimitive
        // | { attribute_instance } primitive udp_identifier ( udp_declaration_port_list ) ;
        //     udp_body
        //   endprimitive

        let attribute_instances =
            parse_zero_or_more_while_next(tkw, sc, arenas, diagnostics.as_deref_mut(), |t| {
                t == T::LeftParenStar
            })?;
        tkw.next_expect(T::KeywordPrimitive, diagnostics.as_deref_mut())?;
        let identifier = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;

        let ports = if tkw.is_next(|t| matches!(t, T::KeywordInput | T::KeywordOutput)) {
            let output_decl =
                parse::<UdpOutputDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
            tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
            let input_decls = parse_one_or_more_while::<UdpInputDeclaration>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
                |t| t.next_if_equals(T::Comma),
            )?;
            tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
            tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

            UdpPorts::DeclarationPortList(UdpDeclarationPortList {
                output_decl,
                input_decls,
            })
        } else {
            let mut first = true;
            let idents = parse_one_or_more_while::<Identifier>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
                |tkw| std::mem::replace(&mut first, false) || tkw.next_if_equals(T::Comma),
            )?;
            tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
            tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

            let decls = parse_one_or_more_while_next::<UdpPortDeclaration>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
                |t| matches!(t, T::KeywordInput | T::KeywordOutput | T::KeywordReg),
            )?;

            UdpPorts::PortList(idents, decls)
        };

        let initial = if tkw.next_if_equals(T::KeywordInitial) {
            Some(parse::<UdpInitialStatement>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?)
        } else {
            None
        };

        tkw.next_expect(T::KeywordTable, diagnostics.as_deref_mut())?;
        let body = if !tkw.is_next_equal_to(T::KeywordEndTable) {
            let mut is_sequential = initial.is_some();
            if !is_sequential {
                let mut num_colons = 0;
                let mut i = tkw.offset;
                while let Some(t) = tkw.get(i)
                    && !matches!(t.kind, T::Semicolon)
                {
                    num_colons += u32::from(matches!(t.kind, T::Colon));
                    i += 1;
                }
                is_sequential = num_colons == 2;
            }
            if is_sequential {
                let entries = parse_one_or_more_while_next::<UdpSequentialEntry>(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                    |t| !matches!(t, T::KeywordEndTable),
                )?;
                UdpBody::Sequential(initial, entries)
            } else {
                let entries = parse_one_or_more_while_next::<UdpCombinationalEntry>(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                    |t| !matches!(t, T::KeywordEndTable),
                )?;
                UdpBody::Combinational(entries)
            }
        } else {
            UdpBody::Combinational(AstIdRange::default())
        };
        tkw.next_expect(T::KeywordEndTable, diagnostics.as_deref_mut())?;

        tkw.next_expect(T::KeywordEndPrimitive, diagnostics.as_deref_mut())?;

        Ok(Self {
            attribute_instances,
            identifier,
            ports,
            body,
        })
    }
}

impl<'a> Consumable<'a> for UdpPortDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // udp_port_declaration ::=
        //   udp_output_declaration ;
        // | udp_input_declaration ;
        // | udp_reg_declaration ;

        match tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?.kind {
            T::KeywordInput => {
                let decl =
                    parse::<UdpInputDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::Input(decl))
            }
            T::KeywordOutput => {
                let decl =
                    parse::<UdpOutputDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::Output(decl))
            }
            T::KeywordReg => {
                let decl = parse::<UdpRegDeclaration>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                Ok(Self::Reg(decl))
            }
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, *t));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for UdpInputDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // udp_input_declaration ::= { attribute_instance } input list_of_port_identifiers

        let attribute_instances =
            parse_zero_or_more_while_next(tkw, sc, arenas, diagnostics.as_deref_mut(), |t| {
                t == T::LeftParenStar
            })?;
        tkw.next_expect(T::KeywordInput, diagnostics.as_deref_mut())?;

        let port_idents =
            parse_one_or_more_while::<Identifier>(tkw, sc, arenas, diagnostics, |tkw| {
                tkw.next_if_equals(T::Comma)
            })?;

        Ok(Self {
            attribute_instances,
            port_idents,
        })
    }
}
impl<'a> Consumable<'a> for UdpOutputDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // udp_output_declaration ::=
        //   { attribute_instance } output port_identifier
        // | { attribute_instance } output reg port_identifier [ = constant_expression ]

        let attribute_instances =
            parse_zero_or_more_while_next(tkw, sc, arenas, diagnostics.as_deref_mut(), |t| {
                t == T::LeftParenStar
            })?;
        tkw.next_expect(T::KeywordOutput, diagnostics.as_deref_mut())?;
        let is_reg = tkw.next_if_equals(T::KeywordReg);
        let port_identifier =
            item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        let mut constant_expr = None;
        if is_reg && tkw.next_if_equals(T::Equals) {
            constant_expr = Some(parse::<ConstantExpr>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?);
        }

        Ok(Self {
            attribute_instances,
            is_reg,
            port_identifier,
            constant_expr,
        })
    }
}
impl<'a> Consumable<'a> for UdpRegDeclaration {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
        // udp_reg_declaration ::= { attribute_instance } reg variable_identifier

        let attribute_instances =
            parse_zero_or_more_while_next(tkw, sc, arenas, diagnostics.as_deref_mut(), |t| {
                t == T::LeftParenStar
            })?;
        tkw.next_expect(T::KeywordReg, diagnostics.as_deref_mut())?;
        let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            attribute_instances,
            ident,
        })
    }
}

impl<'a> Consumable<'a> for UdpCombinationalEntry {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // combinational_entry ::= level_input_list : output_symbol ;

        let level_input_list = parse_one_or_more_while_next::<UdpLevelSymbol>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |t| !matches!(t, T::Colon),
        )?;
        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
        let output_symbol =
            item_parse::<UdpOutputSymbol>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            level_input_list,
            output_symbol,
        })
    }
}
impl<'a> Consumable<'a> for UdpSequentialEntry {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // sequential_entry ::= seq_input_list : current_state : next_state ;
        // seq_input_list ::= level_input_list | edge_input_list
        // edge_input_list ::= { level_symbol } edge_indicator { level_symbol }

        let level_list = parse_zero_or_more_while::<UdpLevelSymbol>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |tkw| {
                tkw.token_content(tkw.offset).is_some_and(|t| {
                    t.len() == 1 && byte_to_level_symbol(t.as_bytes()[0]).is_some()
                })
            },
        )?;
        let mut edge_list = None;
        if tkw.is_next_equal_to(T::LeftParen)
            || tkw
                .token_content(tkw.offset)
                .is_some_and(|t| t.len() == 1 && byte_to_edge_symbol(t.as_bytes()[0]).is_some())
        {
            let edge_indicator =
                parse::<UdpEdgeIndicator>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
            let after_level_list = parse_zero_or_more_while_next::<UdpLevelSymbol>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
                |t| !matches!(t, T::Colon),
            )?;
            edge_list = Some((edge_indicator, after_level_list));
        }

        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
        let current_state =
            item_parse::<UdpLevelSymbol>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
        let next_state = item_parse::<UdpNextState>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            level_list,
            edge_list,
            current_state,
            next_state,
        })
    }
}

impl<'a> Consumable<'a> for UdpInitialStatement {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // udp_initial_statement ::= initial output_port_identifier = init_val ;

        let output_port_ident =
            item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let init_val = item_parse::<UdpInitVal>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            output_port_ident,
            init_val,
        })
    }
}

fn byte_to_level_symbol(b: u8) -> Option<UdpLevelSymbol> {
    use UdpLevelSymbol as S;
    match b {
        b'0' => Some(S::L0),
        b'1' => Some(S::L1),
        b'x' | b'X' => Some(S::X),
        b'b' | b'B' => Some(S::B),
        b'?' => Some(S::QuestionMark),
        _ => None,
    }
}

fn byte_to_output_symbol(b: u8) -> Option<UdpOutputSymbol> {
    use UdpOutputSymbol as S;
    match b {
        b'0' => Some(S::L0),
        b'1' => Some(S::L1),
        b'x' | b'X' => Some(S::X),
        _ => None,
    }
}

fn byte_to_edge_symbol(b: u8) -> Option<UdpEdgeSymbol> {
    use UdpEdgeSymbol as S;
    match b {
        b'r' | b'R' => Some(S::R),
        b'f' | b'F' => Some(S::F),
        b'p' | b'P' => Some(S::P),
        b'n' | b'N' => Some(S::N),
        b'*' => Some(S::Star),
        _ => None,
    }
}

impl<'a> Consumable<'a> for UdpLevelSymbol {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        _arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // level_symbol ::= 0 | 1 | x | X | ? | b | B

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        match *t.kind {
            tkind @ (T::Decimal | T::Ident) => {
                let (span, file) = (*t.span, *t.file);
                let content = &tkw.content(file)[span.as_range()];
                if content.len() == 1
                    && let Some(symbol) = byte_to_level_symbol(content.as_bytes()[0])
                {
                    Ok(symbol)
                } else {
                    diagnostics.map(|d| d.unexpected_token(tkw.offset, tkind));
                    Err(())
                }
            }
            T::QuestionMark => Ok(Self::QuestionMark),
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for UdpOutputSymbol {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        _arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // output_symbol ::= 0 | 1 | x | X

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        match *t.kind {
            tkind @ (T::Decimal | T::Ident) => {
                let (span, file) = (*t.span, *t.file);
                let content = &tkw.content(file)[span.as_range()];
                if content.len() == 1
                    && let Some(symbol) = byte_to_output_symbol(content.as_bytes()[0])
                {
                    Ok(symbol)
                } else {
                    diagnostics.map(|d| d.unexpected_token(tkw.offset, tkind));
                    Err(())
                }
            }
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for UdpEdgeSymbol {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        _arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // edge_symbol ::= r | R | f | F | p | P | n | N | *

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        match *t.kind {
            tkind @ (T::Decimal | T::Ident) => {
                let (span, file) = (*t.span, *t.file);
                let content = &tkw.content(file)[span.as_range()];
                if content.len() == 1
                    && let Some(symbol) = byte_to_edge_symbol(content.as_bytes()[0])
                {
                    Ok(symbol)
                } else {
                    diagnostics.map(|d| d.unexpected_token(tkw.offset, tkind));
                    Err(())
                }
            }
            T::Star => Ok(Self::Star),
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for UdpNextState {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // next_state ::= output_symbol | -

        if tkw.next_if_equals(T::Minus) {
            Ok(Self::Dash)
        } else {
            UdpOutputSymbol::consume(tkw, sc, arenas, diagnostics).map(Self::Output)
        }
    }
}

impl<'a> Consumable<'a> for UdpEdgeIndicator {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // edge_indicator ::= ( level_symbol level_symbol ) | edge_symbol

        if tkw.next_if_equals(T::LeftParen) {
            let t = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
            if t.span.len() == 2 {
                let (kind, span, file) = (*t.kind, *t.span, *t.file);
                let content = &tkw.content(file)[span.as_range()];

                let before = content.as_bytes()[0];
                let after = content.as_bytes()[1];

                let (Some(before), Some(after)) =
                    (byte_to_level_symbol(before), byte_to_level_symbol(after))
                else {
                    diagnostics.map(|d| d.unexpected_token(tkw.offset, kind));
                    return Err(());
                };
                let tr = TokenRange::at(tkw.offset);
                tkw.offset += 1;

                let loc = arenas.spans.len();
                arenas.spans.push(tr);

                let before = AstItem { item: before, loc };
                let after = AstItem { item: after, loc };
                tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                Ok(Self::Levels(before, after))
            } else {
                let before =
                    item_parse::<UdpLevelSymbol>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                let after =
                    item_parse::<UdpLevelSymbol>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                Ok(Self::Levels(before, after))
            }
        } else {
            item_parse::<UdpEdgeSymbol>(tkw, sc, arenas, diagnostics.as_deref_mut()).map(Self::Edge)
        }
    }
}

impl<'a> Consumable<'a> for UdpInitVal {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        _arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // init_val ::= 1'b0 | 1'b1 | 1'bx | 1'bX | 1'B0 | 1'B1 | 1'Bx | 1'BX | 1 | 0

        let result = match tkw.token_content(tkw.offset) {
            Some("1'b0" | "1'B0" | "0") => UdpInitVal::L0,
            Some("1'b1" | "1'B1" | "1") => UdpInitVal::L1,
            Some("1'bx" | "1'bX" | "1'Bx" | "1'BX") => UdpInitVal::X,
            None => {
                diagnostics.map(|d| d.missing_token(tkw.offset));
                return Err(());
            }
            Some(_) => {
                let t = tkw.get(tkw.offset).unwrap().kind;
                diagnostics.map(|d| d.unexpected_token(tkw.offset, *t));
                return Err(());
            }
        };
        tkw.offset += 1;
        Ok(result)
    }
}

impl<'a> Consumable<'a> for UdpInstantiation {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // udp_instantiation ::= udp_identifier [ drive_strength ] [ delay2 ] udp_instance { , udp_instance } ;

        let identifier = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        // @Incomplete
        let instances = parse_one_or_more_while::<UdpInstance>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |tkw| tkw.next_if_equals(T::Comma),
        )?;
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            identifier,
            instances,
        })
    }
}

impl<'a> Consumable<'a> for UdpInstance {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // udp_instance ::= [ name_of_udp_instance ] ( output_terminal , input_terminal { , input_terminal } )
        // name_of_udp_instance ::= udp_instance_identifier [ range ]

        let mut name = None;
        if tkw.is_next_equal_to(T::Ident) {
            let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
            let range = if tkw.is_next_equal_to(T::LeftBrace) {
                Some(parse::<Range>(tkw, sc, arenas, diagnostics.as_deref_mut())?)
            } else {
                None
            };
            name = Some((ident, range));
        }

        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let output_terminal = parse::<NetLValue>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Comma, diagnostics.as_deref_mut())?;
        let input_terminals =
            parse_one_or_more_while(tkw, sc, arenas, diagnostics.as_deref_mut(), |tkw| {
                tkw.next_if_equals(T::Comma)
            })?;
        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        Ok(Self {
            name,
            output_terminal,
            input_terminals,
        })
    }
}
