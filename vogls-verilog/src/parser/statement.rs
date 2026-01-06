use vogls_ir::token_range::TokenRange;

use crate::ast::constant_expr::{ConstantExpr, ConstantRangeExpression};
use crate::ast::expr::Expr;
use crate::ast::module::BlockItemDeclaration;
use crate::ast::statement::{
    Block, BlockingAssignment, CaseItem, CaseItemPattern, CaseStatement, CaseStatementVariant,
    ConditionalStatement, DelayControl, DelayOrEventControl, DelayValue, EventControl,
    EventExpression, EventExpressionPrimary, IfBranch, LoopStatement, LoopStatementVariant,
    MinTypMaxExpression, NetLValue, NetLValueFlat, NonBlockingAssignment, ProceduralTimingControl,
    ProceduralTimingControlStatement, SeqBlock, Statement, StatementContent, StatementOrNull,
    SystemTaskEnable, SystemTaskIdentifier, TaskEnable, VariableAssignment, VariableLValue,
    VariableLValueFlat, WaitStatement,
};
use crate::ast::{
    AstIdRange, AstItem, AttributeInstance, DecimalRef, Identifier, RangeExpression, TextRef,
};
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, ParserScratches, TokenWalker};
use super::{Diagnostics, utils::*};

impl<'a> Consumable<'a> for Statement {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // statement ::=
        //   { attribute_instance } blocking_assignment ;
        //   | { attribute_instance } case_statement
        //   | { attribute_instance } conditional_statement
        //   | { attribute_instance } disable_statement
        //   | { attribute_instance } event_trigger
        //   | { attribute_instance } loop_statement
        //   | { attribute_instance } nonblocking_assignment ;
        //   | { attribute_instance } par_block
        //   | { attribute_instance } procedural_continuous_assignments ;
        //   | { attribute_instance } procedural_timing_control_statement
        //   | { attribute_instance } seq_block
        //   | { attribute_instance } system_task_enable
        //   | { attribute_instance } task_enable
        //   | { attribute_instance } wait_statement

        let attr_instances = parse_zero_or_more_while_next::<AttributeInstance>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |t| t == T::LeftParenStar,
        )?;
        let content = StatementContent::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            attr_instances,
            content,
        })
    }
}

impl<'a> Consumable<'a> for StatementContent {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // statement ::=
        //   { attribute_instance } blocking_assignment ;
        //   | { attribute_instance } case_statement
        //   | { attribute_instance } conditional_statement
        //   | { attribute_instance } disable_statement
        //   | { attribute_instance } event_trigger
        //   | { attribute_instance } loop_statement
        //   | { attribute_instance } nonblocking_assignment ;
        //   | { attribute_instance } par_block
        //   | { attribute_instance } procedural_continuous_assignments ;
        //   | { attribute_instance } procedural_timing_control_statement
        //   | { attribute_instance } seq_block
        //   | { attribute_instance } system_task_enable
        //   | { attribute_instance } task_enable
        //   | { attribute_instance } wait_statement

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::KeywordBegin => {
                let seq_block = parse::<SeqBlock>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::SeqBlock(seq_block))
            }
            T::Hash | T::AtSign => {
                let procedural_timing_control_statement = parse::<ProceduralTimingControlStatement>(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                )?;
                Ok(Self::ProceduralTimingControlStatement(
                    procedural_timing_control_statement,
                ))
            }
            T::KeywordForever | T::KeywordRepeat | T::KeywordWhile | T::KeywordFor => {
                Ok(Self::LoopStatement(parse::<LoopStatement>(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                )?))
            }
            T::KeywordCase | T::KeywordCaseX | T::KeywordCaseZ => {
                Ok(Self::CaseStatement(parse::<CaseStatement>(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                )?))
            }
            T::KeywordIf => Ok(Self::ConditionalStatement(parse::<ConditionalStatement>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?)),
            T::DollarIdent => Ok(Self::SystemTaskEnable(parse::<SystemTaskEnable>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?)),
            T::KeywordWait => Ok(Self::WaitStatement(parse::<WaitStatement>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?)),
            T::Ident => {
                match *tkw
                    .try_get(tkw.offset + 1, diagnostics.as_deref_mut())?
                    .kind
                {
                    T::Semicolon => {
                        // @Incomplete: This also supports arguments
                        let task_enable =
                            parse::<TaskEnable>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                        tkw.offset += 1;
                        Ok(Self::TaskEnable(task_enable))
                    }
                    T::Equals => {
                        let ba = parse::<BlockingAssignment>(
                            tkw,
                            sc,
                            arenas,
                            diagnostics.as_deref_mut(),
                        )?;
                        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                        return Ok(Self::BlockingAssignment(ba));
                    }
                    T::LessThanEquals => {
                        let nba = parse::<NonBlockingAssignment>(
                            tkw,
                            sc,
                            arenas,
                            diagnostics.as_deref_mut(),
                        )?;
                        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                        return Ok(Self::NonBlockingAssignment(nba));
                    }
                    T::LeftBrace => {
                        let start = tkw.offset;
                        let end = loop {
                            tkw.offset += 2;
                            let Some(corresponding_brace) = tkw.find_next_same_depth(T::RightBrace)
                            else {
                                diagnostics
                                    .map(|d| d.no_corresponding(tkw.offset - 1, T::RightBrace));
                                return Err(());
                            };

                            tkw.offset = corresponding_brace;
                            if tkw
                                .get(corresponding_brace + 1)
                                .is_none_or(|t| *t.kind != T::LeftBrace)
                            {
                                break corresponding_brace + 1;
                            }
                        };
                        tkw.offset = start;

                        match *tkw.try_get(end, diagnostics.as_deref_mut())?.kind {
                            T::LessThanEquals => {
                                let nba = parse::<NonBlockingAssignment>(
                                    tkw,
                                    sc,
                                    arenas,
                                    diagnostics.as_deref_mut(),
                                )?;
                                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                                Ok(Self::NonBlockingAssignment(nba))
                            }
                            T::Equals => {
                                let ba = parse::<BlockingAssignment>(
                                    tkw,
                                    sc,
                                    arenas,
                                    diagnostics.as_deref_mut(),
                                )?;
                                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                                Ok(Self::BlockingAssignment(ba))
                            }
                            _ => {
                                diagnostics.map(|d| d.incomplete(tkw.offset, "statement"));
                                Err(())
                            }
                        }
                    }
                    _ => {
                        diagnostics.map(|d| d.incomplete(tkw.offset, "statement"));
                        Err(())
                    }
                }
            }
            T::LeftBracket => {
                tkw.offset += 1;
                let Some(end) = tkw.find_next_same_depth(T::RightBracket) else {
                    diagnostics.map(|d| d.no_corresponding(tkw.offset - 1, T::RightBracket));
                    return Err(());
                };
                tkw.offset -= 1;

                match *tkw.try_get(end + 1, diagnostics.as_deref_mut())?.kind {
                    T::LessThanEquals => {
                        let nba = parse::<NonBlockingAssignment>(
                            tkw,
                            sc,
                            arenas,
                            diagnostics.as_deref_mut(),
                        )?;
                        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                        Ok(Self::NonBlockingAssignment(nba))
                    }
                    T::Equals => {
                        let ba = parse::<BlockingAssignment>(
                            tkw,
                            sc,
                            arenas,
                            diagnostics.as_deref_mut(),
                        )?;
                        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                        Ok(Self::BlockingAssignment(ba))
                    }
                    _ => {
                        diagnostics.map(|d| d.incomplete(tkw.offset, "statement"));
                        Err(())
                    }
                }
            }
            T::KeywordEnd => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, T::KeywordEnd));
                Err(())
            }
            _ => {
                diagnostics.map(|d| d.incomplete(tkw.offset, "statement"));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for NetLValue {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // net_lvalue ::=
        //   hierarchical_net_identifier [ { [ constant_expression ] } [ constant_range_expression ] ]
        // | { net_lvalue { , net_lvalue } }

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match *peeked.kind {
            T::Ident => Ok(Self(AstIdRange::single(parse::<NetLValueFlat>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?))),
            T::LeftBracket => {
                tkw.offset += 1;
                // @Incomplete: This actually allows recursive variable_lvalue.
                let lvalues = parse_one_or_more_delimited::<NetLValueFlat>(
                    tkw,
                    sc,
                    arenas,
                    T::Comma,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::RightBracket, diagnostics.as_deref_mut())?;
                Ok(Self(lvalues))
            }
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for NetLValueFlat {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // net_lvalue ::=
        //   hierarchical_net_identifier [ { [ constant_expression ] } [ constant_range_expression ] ]
        // | { net_lvalue { , net_lvalue } }
        let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let mut constant_exprs = AstIdRange::default();
        let mut constant_range_expression = None;
        if tkw.get(tkw.offset).is_some_and(|t| *t.kind == T::LeftBrace) {
            let mut items = Vec::new();
            let mut spans = Vec::new();

            let mut end = tkw.try_find_corresponding_balanced(tkw.offset);
            tkw.offset += 1;
            while tkw.get(end + 1).is_some_and(|t| *t.kind == T::LeftBrace) {
                let mut item_tkw = tkw.end_at(end);
                let start = item_tkw.offset;
                let item =
                    ConstantExpr::consume(&mut item_tkw, sc, arenas, diagnostics.as_deref_mut())?;
                let token_range = TokenRange { start, end };
                items.push(item);
                spans.push(token_range);

                tkw.offset = end + 2;
                end = tkw.try_find_corresponding_balanced(end + 1);
            }

            let mut item_tkw = tkw.end_at(end);
            tkw.offset = end + 1;
            constant_exprs = arenas.add_range(items, spans);
            constant_range_expression = Some(parse::<ConstantRangeExpression>(
                &mut item_tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?);
        }
        Ok(Self {
            ident,
            constant_exprs,
            constant_range_expression,
        })
    }
}

impl<'a> Consumable<'a> for VariableLValue {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // variable_lvalue ::=
        //   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
        //   | { variable_lvalue { , variable_lvalue } }

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match *peeked.kind {
            T::Ident => Ok(Self(AstIdRange::single(parse::<VariableLValueFlat>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?))),
            T::LeftBracket => {
                tkw.offset += 1;
                // @Incomplete: This actually allows recursive variable_lvalue.
                let lvalues = parse_one_or_more_delimited::<VariableLValueFlat>(
                    tkw,
                    sc,
                    arenas,
                    T::Comma,
                    diagnostics.as_deref_mut(),
                )?;
                tkw.next_expect(T::RightBracket, diagnostics.as_deref_mut())?;
                Ok(Self(lvalues))
            }
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for VariableLValueFlat {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // variable_lvalue ::=
        //   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
        //   | { variable_lvalue { , variable_lvalue } }

        let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let mut exprs = AstIdRange::default();
        let mut range_expression = None;
        if tkw.get(tkw.offset).is_some_and(|t| *t.kind == T::LeftBrace) {
            let mut items = Vec::new();
            let mut spans = Vec::new();

            let mut end = tkw.try_find_corresponding_balanced(tkw.offset);
            tkw.offset += 1;
            while tkw.get(end + 1).is_some_and(|t| *t.kind == T::LeftBrace) {
                let mut item_tkw = tkw.end_at(end);
                let start = item_tkw.offset;
                let item = Expr::consume(&mut item_tkw, sc, arenas, diagnostics.as_deref_mut())?;
                let token_range = TokenRange { start, end };
                items.push(item);
                spans.push(token_range);

                tkw.offset = end + 2;
                end = tkw.try_find_corresponding_balanced(end + 1);
            }

            let mut item_tkw = tkw.end_at(end);
            tkw.offset = end + 1;
            exprs = arenas.add_range(items, spans);
            range_expression = Some(parse::<RangeExpression>(
                &mut item_tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?);
        }
        Ok(Self {
            ident,
            exprs,
            range_expression,
        })
    }
}

impl<'a> Consumable<'a> for RangeExpression {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 505
        // range_expression ::=
        //   expression
        // | msb_constant_expression : lsb_constant_expression
        // | base_expression +: width_constant_expression
        // | base_expression -: width_constant_expression
        // base_expression ::= expression
        // width_constant_expression ::= constant_expression
        // msb_constant_expression ::= constant_expression
        // lsb_constant_expression ::= constant_expression

        let Some(separator) =
            tkw.find_next_one_of_same_depth(&[T::Colon, T::PlusColon, T::MinusColon])
        else {
            return Ok(Self::Expr(parse::<Expr>(tkw, sc, arenas, diagnostics)?));
        };
        let separator_kind = *tkw.get(separator).unwrap().kind;

        match separator_kind {
            T::Colon => {
                let lhs = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut());
                if lhs.is_err() {
                    tkw.offset = separator;
                }
                tkw.next_expect(separator_kind, diagnostics.as_deref_mut())?;
                let rhs = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::MsbLsb(lhs?, rhs))
            }
            T::PlusColon => {
                let lhs = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut());
                if lhs.is_err() {
                    tkw.offset = separator;
                }
                tkw.next_expect(separator_kind, diagnostics.as_deref_mut())?;
                let rhs = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::BasePlus(lhs?, rhs))
            }
            T::MinusColon => {
                let lhs = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut());
                if lhs.is_err() {
                    tkw.offset = separator;
                }
                tkw.next_expect(separator_kind, diagnostics.as_deref_mut())?;
                let rhs = parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::BaseMinus(lhs?, rhs))
            }
            _ => unreachable!(),
        }
    }
}

impl<'a> Consumable<'a> for BlockingAssignment {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // blocking_assignment ::= variable_lvalue = [ delay_or_event_control ] expression

        let variable_lvalue = parse::<VariableLValue>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let delay_or_event_control = try_parse::<DelayOrEventControl>(tkw, sc, arenas);
        let expression = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            variable_lvalue,
            delay_or_event_control,
            expression,
        })
    }
}

impl<'a> Consumable<'a> for NonBlockingAssignment {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // nonblocking_assignment ::= variable_lvalue <= [ delay_or_event_control ] expression

        let variable_lvalue = parse::<VariableLValue>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LessThanEquals, diagnostics.as_deref_mut())?;
        let delay_or_event_control = try_parse::<DelayOrEventControl>(tkw, sc, arenas);
        let expression = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            variable_lvalue,
            delay_or_event_control,
            expression,
        })
    }
}

impl<'a> Consumable<'a> for SeqBlock {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // seq_block ::= begin [ : block_identifier { block_item_declaration } ] { statement } end

        tkw.next_expect(T::KeywordBegin, diagnostics.as_deref_mut())?;
        let mut block = None;
        if tkw.next_if_equals(T::Colon) {
            block = Some(parse::<Block>(tkw, sc, arenas, diagnostics.as_deref_mut())?);
        }
        let statements = parse_zero_or_more_while_next::<Statement>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |t| t != T::KeywordEnd,
        )?;
        tkw.next_expect(T::KeywordEnd, diagnostics.as_deref_mut())?;

        Ok(Self { block, statements })
    }
}

impl<'a> Consumable<'a> for Block {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // block_identifier { block_item_declaration }

        let block_identifier =
            item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let block_item_decls = parse_zero_or_more_while_next::<BlockItemDeclaration>(
            tkw,
            sc,
            arenas,
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

        Ok(Self {
            block_identifier,
            block_item_decls,
        })
    }
}

impl<'a> Consumable<'a> for DelayOrEventControl {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_or_event_control ::=
        //   delay_control
        //   | event_control
        //   | repeat ( expression ) event_control

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match *peeked.kind {
            T::Hash => {
                let delay_control =
                    parse::<DelayControl>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::DelayControl(delay_control))
            }
            T::AtSign => {
                let event_control =
                    parse::<EventControl>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::EventControl(event_control))
            }
            T::KeywordRepeat => {
                diagnostics.map(|d| d.incomplete(tkw.offset, "delay_or_event_control repeat"));
                Err(())
            }
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for DelayControl {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_control ::=
        //   # delay_value
        // | # ( mintypmax_expression )

        tkw.next_expect(T::Hash, diagnostics.as_deref_mut())?;

        if tkw.next_if_equals(T::LeftParen) {
            let mintypmax =
                parse::<MinTypMaxExpression>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
            tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
            Ok(Self::MinTypMax(mintypmax))
        } else {
            let delay_value = parse::<DelayValue>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
            Ok(Self::DelayValue(delay_value))
        }
    }
}

impl<'a> Consumable<'a> for DelayValue {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // delay_value ::=
        //   unsigned_number
        // | real_number
        // | identifier

        // @Incomplete: | real_number
        // @Incomplete: | identifier

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::Decimal => {
                let decimal = DecimalRef::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::UnsignedNumber(decimal))
            }
            T::Ident => {
                let ident = Identifier::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Identifier(ident))
            }
            _ => {
                diagnostics.map(|d| d.incomplete(tkw.offset, "delay_value"));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for MinTypMaxExpression {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 505
        // mintypmax_expression ::=
        //   expression
        // | expression : expression : expression

        let min_typical = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        if tkw.next_if_equals(T::Colon) {
            let typical = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
            tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
            let max = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

            Ok(Self {
                typical,
                min_max: Some((min_typical, max)),
            })
        } else {
            Ok(Self {
                typical: min_typical,
                min_max: None,
            })
        }
    }
}

impl<'a> Consumable<'a> for EventControl {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_control ::=
        //   @ hierarchical_event_identifier
        // | @ ( event_expression )
        // | @*
        // | @ (*)

        tkw.next_expect(T::AtSign, diagnostics.as_deref_mut())?;
        if tkw.next_if_equals(T::Star) || tkw.next_if_equals(T::LeftParenStarRightParen) {
            return Ok(Self::Star);
        }
        if tkw.next_if_equals(T::Ident) {
            let event_expression =
                parse::<EventExpressionPrimary>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
            return Ok(Self::EventExpression(EventExpression(AstIdRange::single(
                event_expression,
            ))));
        }
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let event_expression =
            EventExpression::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        Ok(Self::EventExpression(event_expression))
    }
}

impl<'a> Consumable<'a> for EventExpressionPrimary {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<EventExpressionPrimary, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_expression ::=
        //   expression
        // | posedge expression
        // | negedge expression
        // | event_expression or event_expression
        // | event_expression, event_expression

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        Ok(match peeked.kind {
            T::KeywordPosedge => {
                tkw.offset += 1;
                let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Self::Posedge(expr)
            }
            T::KeywordNegedge => {
                tkw.offset += 1;
                let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Self::Negedge(expr)
            }
            _ => {
                let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Self::Expression(expr)
            }
        })
    }
}

impl<'a> Consumable<'a> for EventExpression {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_expression ::=
        //   expression
        // | posedge expression
        // | negedge expression
        // | event_expression or event_expression
        // | event_expression, event_expression

        parse_one_or_more_delimited_one_of::<EventExpressionPrimary>(
            tkw,
            sc,
            arenas,
            &[T::KeywordOr, T::Comma],
            diagnostics,
        )
        .map(Self)
    }
}

impl<'a> Consumable<'a> for ProceduralTimingControl {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // procedural_timing_control ::=
        //   delay_control
        // | event_control

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match *peeked.kind {
            T::Hash => {
                let delay_control =
                    parse::<DelayControl>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::DelayControl(delay_control))
            }
            T::AtSign => {
                let event_control =
                    parse::<EventControl>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::EventControl(event_control))
            }
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for SystemTaskEnable {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // system_task_enable ::= system_task_identifier [ ( [ expression ] { , [ expression ] } ) ] ;

        let system_task_identifier =
            item_parse::<SystemTaskIdentifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let mut expressions = AstIdRange::default();
        if tkw.next_if_equals(T::LeftParen) {
            expressions = parse_zero_or_more_delimited::<Expr>(
                tkw,
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
        }
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            system_task_identifier,
            expressions,
        })
    }
}

impl<'a> Consumable<'a> for SystemTaskIdentifier {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;
        let t = tkw.next_expect(T::DollarIdent, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &tkw.content(file)[span.start() + 1..span.end()];
        let start = arenas.text.len();
        let end = start + content.len();
        arenas.text.push_str(content);
        Ok(Self(TextRef { start, end }))
    }
}

impl<'a> Consumable<'a> for VariableAssignment {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // variable_assignment ::= variable_lvalue = expression

        let lvalue = parse::<VariableLValue>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self { lvalue, expr })
    }
}

impl<'a> Consumable<'a> for LoopStatement {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // variable_assignment ::= variable_lvalue = expression

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        let variant = match *peeked.kind {
            T::KeywordForever => {
                tkw.offset += 1;
                LoopStatementVariant::Forever
            }
            T::KeywordRepeat => {
                tkw.offset += 1;
                tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
                let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                LoopStatementVariant::Repeat(expr)
            }
            T::KeywordWhile => {
                tkw.offset += 1;
                tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
                let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                LoopStatementVariant::While(expr)
            }
            T::KeywordFor => {
                tkw.offset += 1;
                tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
                let initialization =
                    parse::<VariableAssignment>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                let condition = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                let step =
                    parse::<VariableAssignment>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                LoopStatementVariant::For(initialization, condition, step)
            }

            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                return Err(());
            }
        };

        let statement = parse::<Statement>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        Ok(Self { variant, statement })
    }
}

impl<'a> Consumable<'a> for CaseStatement {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // case_statement ::=
        //   case ( expression )  case_item { case_item } endcase
        // | casez ( expression ) case_item { case_item } endcase
        // | casex ( expression ) case_item { case_item } endcase

        let t = tkw.try_next(diagnostics.as_deref_mut())?;
        let variant = match *t.kind {
            T::KeywordCase => CaseStatementVariant::Case,
            T::KeywordCaseX => CaseStatementVariant::CaseX,
            T::KeywordCaseZ => CaseStatementVariant::CaseZ,
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset - 1, t));
                return Err(());
            }
        };

        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        let items = parse_one_or_more_while_next::<CaseItem>(
            tkw,
            sc,
            arenas,
            diagnostics.as_deref_mut(),
            |t| t != T::KeywordEndCase,
        )?;
        tkw.next_expect(T::KeywordEndCase, diagnostics.as_deref_mut())?;

        Ok(Self {
            variant,
            expr,
            items,
        })
    }
}

impl<'a> Consumable<'a> for CaseItem {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // case_item ::=
        //   expression { , expression } : statement_or_null
        // | default [ : ] statement_or_null

        let start = tkw.offset;
        let (token_range, pattern) = if tkw.next_if_equals(T::KeywordDefault) {
            let token_range = TokenRange {
                start,
                end: tkw.offset,
            };
            tkw.next_if_equals(T::Colon);
            (token_range, CaseItemPattern::Default)
        } else {
            let Some(colon_offset) = tkw.find_next_same_depth(T::Colon) else {
                diagnostics.map(|d| d.no_corresponding(tkw.offset, T::Colon));
                return Err(());
            };
            let mut expressions_tkw = tkw.end_at(colon_offset);
            let expressions = parse_one_or_more_delimited::<Expr>(
                &mut expressions_tkw,
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            let token_range = TokenRange {
                start,
                end: expressions_tkw.offset,
            };
            tkw.offset = colon_offset + 1;
            (token_range, CaseItemPattern::Expressions(expressions))
        };
        let loc = arenas.spans.len();
        arenas.spans.push(token_range);
        let pattern = AstItem { item: pattern, loc };
        let statement_or_null =
            parse::<StatementOrNull>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            pattern,
            statement_or_null,
        })
    }
}

impl<'a> Consumable<'a> for StatementOrNull {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // statement_or_null ::= statement | { attribute_instance } ;

        let start = tkw.offset;
        while tkw.is_next_equal_to(T::LeftParenStar) {
            AttributeInstance::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        }
        // @Performance: Remove consumed data from the arenas afterwards.

        if tkw.is_next_equal_to(T::Semicolon) {
            let end = tkw.offset;
            tkw.offset = start;
            let attr_instances = parse_zero_or_more::<AttributeInstance>(
                &mut tkw.end_at(end),
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?;
            tkw.offset = end + 1;
            return Ok(Self::Attribute(attr_instances));
        }

        tkw.offset = start;
        let statement = parse::<Statement>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        Ok(Self::Statement(statement))
    }
}

impl<'a> Consumable<'a> for ConditionalStatement {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // conditional_statement ::
        //   if ( expression ) statement_or_null
        //   [ else statement_or_null ]
        // | if_else_if_statement
        // if_else_if_statement ::=
        //   if ( expression ) statement_or_null
        //   { else if ( expression ) statement_or_null }
        //   [ else statement_or_null ]

        let if_branch = IfBranch::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        let mut items = Vec::new();
        let mut spans = Vec::new();
        while tkw.next_if_equals(T::KeywordElse) {
            if tkw.is_next_equal_to(T::KeywordIf) {
                let start = tkw.offset;
                let item = IfBranch::consume(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                let token_range = TokenRange {
                    start,
                    end: tkw.offset,
                };
                items.push(item);
                spans.push(token_range);
            } else {
                let else_branch = Some(parse::<StatementOrNull>(
                    tkw,
                    sc,
                    arenas,
                    diagnostics.as_deref_mut(),
                )?);
                let else_ifs = arenas.add_range(items, spans);
                return Ok(Self {
                    if_branch,
                    else_ifs,
                    else_branch,
                });
            }
        }

        let else_ifs = arenas.add_range(items, spans);
        Ok(Self {
            if_branch,
            else_ifs,
            else_branch: None,
        })
    }
}

impl<'a> Consumable<'a> for IfBranch {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        tkw.next_expect(T::KeywordIf, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let condition = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
        let statement = parse::<StatementOrNull>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            condition,
            statement,
        })
    }
}

impl<'a> Consumable<'a> for ProceduralTimingControlStatement {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // procedural_timing_control_statement ::= procedural_timing_control statement_or_null

        let procedural_timing_control =
            parse::<ProceduralTimingControl>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let statement_or_null =
            parse::<StatementOrNull>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            procedural_timing_control,
            statement_or_null,
        })
    }
}

impl<'a> Consumable<'a> for TaskEnable {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // @Incomplete
        let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        Ok(Self { ident })
    }
}

impl<'a> Consumable<'a> for WaitStatement {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // wait_statement ::= wait ( expression ) statement_or_null

        tkw.next_expect(T::KeywordWait, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let Some(end) = tkw.find_next_same_depth(T::RightParen) else {
            diagnostics.map(|d| d.no_corresponding(tkw.offset - 1, T::RightParen));
            return Err(());
        };

        let expression = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut());
        let expression_end = tkw.offset;
        tkw.offset = end + 1;
        let statement_or_null =
            parse::<StatementOrNull>(tkw, sc, arenas, diagnostics.as_deref_mut());

        if end != expression_end {
            diagnostics.map(|d| {
                d.leftover_tokens(TokenRange {
                    start: expression_end,
                    end,
                })
            });
            return Err(());
        }

        let (expression, statement_or_null) = (expression?, statement_or_null?);

        Ok(Self {
            expression,
            statement_or_null,
        })
    }
}
