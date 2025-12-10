use crate::ast::constant_expr::{ConstantExpr, ConstantRangeExpression};
use crate::ast::expr::Expr;
use crate::ast::statement::{
    BlockingAssignment, CaseItem, CaseItemPattern, CaseStatement, CaseStatementVariant,
    ConditionalStatement, DelayControl, DelayOrEventControl, DelayValue, EventControl,
    EventExpression, IfBranch, LoopStatement, LoopStatementVariant, NetLValue,
    NonBlockingAssignment, ProceduralTimingControl, SeqBlock, Statement, StatementOrNull,
    SystemTaskEnable, SystemTaskIdentifier, VariableAssignment, VariableLValue,
};
use crate::ast::{AstIdRange, AstItem, DecimalRef, Identifier, TextRef};
use crate::parser::token_walker::TokenRange;
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

        // @Incomplete: { attribute_instance }
        // @Incomplete

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::KeywordBegin => {
                let seq_block = parse::<SeqBlock>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::SeqBlock(seq_block))
            }
            T::Hash | T::AtSign => {
                let procedural_timing_control =
                    parse::<ProceduralTimingControl>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

                let statement = try_parse::<Statement>(tkw, sc, arenas);
                Ok(Self::ProceduralTimingControlStatement(
                    procedural_timing_control,
                    statement,
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
            _ => {
                if let Some(blocking_assignment) = try_parse::<BlockingAssignment>(tkw, sc, arenas)
                {
                    tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                    Ok(Self::BlockingAssignment(blocking_assignment))
                } else if let Some(non_blocking_assignment) =
                    try_parse::<NonBlockingAssignment>(tkw, sc, arenas)
                {
                    tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                    Ok(Self::NonBlockingAssignment(non_blocking_assignment))
                } else if let Some(system_task_enable) =
                    try_parse::<SystemTaskEnable>(tkw, sc, arenas)
                {
                    Ok(Self::SystemTaskEnable(system_task_enable))
                } else {
                    diagnostics.map(|d| d.incomplete(tkw.offset, "statement"));
                    Err(())
                }
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
            T::Ident => {
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
                        let item = ConstantExpr::consume(
                            &mut item_tkw,
                            sc,
                            arenas,
                            diagnostics.as_deref_mut(),
                        )?;
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
            T::LeftBracket => {
                diagnostics.map(|d| d.incomplete(tkw.offset, "netlvalue::left_brace"));
                Err(())
            }
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                Err(())
            }
        }
    }
}

impl<'a> Consumable<'a> for VariableLValue {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // variable_lvalue ::=
        //   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
        //   | { variable_lvalue { , variable_lvalue } }

        // @Incomplete

        let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self { ident })
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

        // @Incomplete: [ : block_identifier { block_item_declaration } ]
        tkw.next_expect(T::KeywordBegin, diagnostics.as_deref_mut())?;
        let statements = parse_until_reaching::<Statement>(
            tkw,
            sc,
            arenas,
            T::KeywordEnd,
            diagnostics.as_deref_mut(),
        )?;

        Ok(Self { statements })
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
        // @Incomplete: | # ( mintypmax_expression )
        let delay_value = parse::<DelayValue>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

        Ok(Self::DelayValue(delay_value))
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
        // @Incomplete: @ hierarchical_event_identifier
        // @Incomplete: @*
        // @Incomplete: @ (*)
        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let event_expression =
            parse::<EventExpression>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        Ok(Self::EventExpression(event_expression))
    }
}

impl<'a> Consumable<'a> for EventExpression {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_expression ::=
        //   expression
        // | posedge expression
        // | negedge expression
        // | event_expression or event_expression

        let start = tkw.offset;
        let mut event_expression = None;

        // @Incomplete: | event_expression or event_expression
        loop {
            let start_current = tkw.offset;
            let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
            let current_event_expression = match peeked.kind {
                T::KeywordPosedge => {
                    tkw.next();
                    let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                    Self::Posedge(expr)
                }
                T::KeywordNegedge => {
                    tkw.next();
                    let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                    Self::Negedge(expr)
                }
                _ => {
                    let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                    Self::Expression(expr)
                }
            };

            event_expression = match event_expression {
                None => Some(current_event_expression),
                Some(expr) => {
                    let token_range = TokenRange {
                        start,
                        end: tkw.offset,
                    };
                    let expr = arenas.add(expr, token_range);

                    let token_range = TokenRange {
                        start: start_current,
                        end: tkw.offset,
                    };
                    let current_event_expression =
                        arenas.add(current_event_expression, token_range);

                    Some(Self::OrList(expr, current_event_expression))
                }
            };

            let Some(peeked) = tkw.get(tkw.offset) else {
                break;
            };

            if *peeked.kind != T::KeywordOr {
                break;
            }
        }

        Ok(event_expression.unwrap())
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

        let peeked = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
        let variant = match *peeked.kind {
            T::KeywordCase => CaseStatementVariant::Case,
            T::KeywordCaseX => CaseStatementVariant::CaseX,
            T::KeywordCaseZ => CaseStatementVariant::CaseZ,
            t => {
                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                return Err(());
            }
        };

        tkw.next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let expr = parse::<Expr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        let items =
            parse_one_or_more_until_fail::<CaseItem>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
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
            let expressions = parse_one_or_more_delimited::<Expr>(
                tkw,
                sc,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            let token_range = TokenRange {
                start,
                end: tkw.offset,
            };
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

        let result = match try_parse::<Statement>(tkw, sc, arenas) {
            None => {
                let attr_instances =
                    parse_one_or_more_until_fail(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                Self::Attribute(attr_instances)
            }
            Some(statement) => Self::Statement(statement),
        };
        tkw.next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(result)
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
