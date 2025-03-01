use crate::ast::expr::Expr;
use crate::ast::statement::{
    BlockingAssignment, DelayControl, DelayOrEventControl, DelayValue, EventControl,
    EventExpression, NonBlockingAssignment, ProceduralTimingControl, SeqBlock, Statement,
    VariableLValue,
};
use crate::ast::{DecimalRef, IdentRef};
use crate::lexer::{FromLexerError, TokenKind};
use crate::parser::ItemParsable;
use crate::span::Span;

use super::{AstArenas, Consumable, Parsable, ParseError, Parser};

impl<'a> Consumable<'a> for Statement {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

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

        let peeked = p.lexer().next_expect_peek()?;
        match peeked.kind() {
            TK::KeywordBegin => {
                peeked.release();
                let (seq_block, span) = SeqBlock::parse_with_span(p, arenas)?;
                Ok((Self::SeqBlock(seq_block), span))
            }
            TK::Hash | TK::AtSign => {
                peeked.release();
                let (procedural_timing_control, span) =
                    ProceduralTimingControl::parse_with_span(p, arenas)?;
                Ok((
                    Self::ProceduralTimingControlStatement(procedural_timing_control),
                    span,
                ))
            }
            _ => {
                peeked.release();
                if let Ok((blocking_assignment, blocking_assignment_span)) =
                    BlockingAssignment::parse_with_span(p, arenas)
                {
                    let semicolon_span = p.lexer().expect(TK::Semicolon)?.span();
                    Ok((
                        Self::BlockingAssignment(blocking_assignment),
                        blocking_assignment_span | semicolon_span,
                    ))
                } else if let Ok((non_blocking_assignment, non_blocking_assignment_span)) =
                    NonBlockingAssignment::parse_with_span(p, arenas)
                {
                    let semicolon_span = p.lexer().expect(TK::Semicolon)?.span();
                    Ok((
                        Self::NonBlockingAssignment(non_blocking_assignment),
                        non_blocking_assignment_span | semicolon_span,
                    ))
                } else {
                    Err(ParseError::Incomplete("statement"))
                }
            }
        }
    }
}
impl<'a> Parsable<'a> for Statement {}

impl<'a> Consumable<'a> for VariableLValue {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // variable_lvalue ::=
        //   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
        //   | { variable_lvalue { , variable_lvalue } }

        // @Incomplete

        let (ident, span) = IdentRef::parse_with_span(p, arenas)?;

        Ok((Self { ident }, span))
    }
}
impl<'a> Parsable<'a> for VariableLValue {}

impl<'a> Consumable<'a> for BlockingAssignment {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // blocking_assignment ::= variable_lvalue = [ delay_or_event_control ] expression

        let (variable_lvalue, variable_lvalue_span) = VariableLValue::parse_with_span(p, arenas)?;
        p.lexer().expect(TK::Equals)?;
        let delay_or_event_control = DelayOrEventControl::try_parse(p, arenas);
        let (expression, expression_span) = Expr::parse_with_span(p, arenas)?;

        let span = variable_lvalue_span | expression_span;

        Ok((
            Self {
                variable_lvalue,
                delay_or_event_control,
                expression,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for BlockingAssignment {}

impl<'a> Consumable<'a> for NonBlockingAssignment {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // nonblocking_assignment ::= variable_lvalue <= [ delay_or_event_control ] expression

        let (variable_lvalue, variable_lvalue_span) = VariableLValue::parse_with_span(p, arenas)?;
        p.lexer().expect(TK::LessThanEquals)?;
        let delay_or_event_control = DelayOrEventControl::try_parse(p, arenas);
        let (expression, expression_span) = Expr::parse_with_span(p, arenas)?;

        let span = variable_lvalue_span | expression_span;

        Ok((
            Self {
                variable_lvalue,
                delay_or_event_control,
                expression,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for NonBlockingAssignment {}

impl<'a> Consumable<'a> for SeqBlock {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // seq_block ::= begin [ : block_identifier { block_item_declaration } ] { statement } end

        // @Incomplete: [ : block_identifier { block_item_declaration } ]
        let begin_kw_span = p.lexer().expect(TK::KeywordBegin)?.span();
        let (statements, end_kw) = Statement::parse_until_reaching(p, arenas, TK::KeywordEnd)?;

        let span = begin_kw_span | end_kw.span();

        Ok((Self { statements }, span))
    }
}
impl<'a> Parsable<'a> for SeqBlock {}

impl<'a> Consumable<'a> for DelayOrEventControl {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_or_event_control ::=
        //   delay_control
        //   | event_control
        //   | repeat ( expression ) event_control

        let peeked = p.lexer().next_expect_peek()?;
        match peeked.kind() {
            TK::Hash => {
                peeked.release();
                let (delay_control, span) = DelayControl::parse_with_span(p, arenas)?;
                Ok((Self::DelayControl(delay_control), span))
            }
            TK::AtSign => {
                peeked.release();
                let (event_control, span) = EventControl::parse_with_span(p, arenas)?;
                Ok((Self::EventControl(event_control), span))
            }
            TK::KeywordRepeat => {
                peeked.release();
                Err(ParseError::Incomplete("delay_or_event_control repeat"))
            }
            _ => Err(ParseError::unexpected_token(peeked.commit())),
        }
    }
}
impl<'a> Parsable<'a> for DelayOrEventControl {}

impl<'a> Consumable<'a> for DelayControl {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_control ::=
        //   # delay_value
        // | # ( mintypmax_expression )

        let hash_span = p.lexer().expect(TK::Hash)?.span();
        // @Incomplete: | # ( mintypmax_expression )
        let (delay_value, delay_value_span) = DelayValue::parse_with_span(p, arenas)?;

        let span = hash_span | delay_value_span;

        Ok((Self::DelayValue(delay_value), span))
    }
}
impl<'a> Parsable<'a> for DelayControl {}

impl<'a> Consumable<'a> for DelayValue {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // delay_value ::=
        //   unsigned_number
        // | real_number
        // | identifier

        // @Incomplete: | real_number
        // @Incomplete: | identifier

        let (decimal, decimal_span) = DecimalRef::parse_with_span(p, arenas)?;

        Ok((Self(decimal), decimal_span))
    }
}
impl<'a> Parsable<'a> for DelayValue {}

impl<'a> Consumable<'a> for EventControl {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_control ::=
        //   @ hierarchical_event_identifier
        // | @ ( event_expression )
        // | @*
        // | @ (*)

        let at_sign_span = p.lexer().expect(TK::AtSign)?.span();
        // @Incomplete:   @ hierarchical_event_identifier
        // @Incomplete: | @*
        // @Incomplete: | @ (*)
        p.lexer().expect(TK::LeftParen)?;
        let event_expression = EventExpression::parse(p, arenas)?;
        let right_paren_span = p.lexer().expect(TK::RightParen)?.span();

        let span = at_sign_span | right_paren_span;

        Ok((Self::EventExpression(event_expression), span))
    }
}
impl<'a> Parsable<'a> for EventControl {}

impl<'a> Consumable<'a> for EventExpression {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_expression ::=
        //   expression
        // | posedge expression
        // | negedge expression
        // | event_expression or event_expression

        let mut event_expression = None;

        // @Incomplete: | event_expression or event_expression
        loop {
            let peeked = p.lexer().next_expect_peek()?;
            let (current_event_expression, current_span) = match peeked.kind() {
                TK::KeywordPosedge => {
                    let posedge_kw_span = peeked.commit().span();
                    let (expr, expr_span) = Expr::parse_with_span(p, arenas)?;
                    let span = posedge_kw_span | expr_span;
                    (Self::Posedge(expr), span)
                }
                TK::KeywordNegedge => {
                    let negedge_kw_span = peeked.commit().span();
                    let (expr, expr_span) = Expr::parse_with_span(p, arenas)?;
                    let span = negedge_kw_span | expr_span;
                    (Self::Negedge(expr), span)
                }
                _ => {
                    peeked.release();
                    let (expr, expr_span) = Expr::parse_with_span(p, arenas)?;
                    (Self::Expression(expr), expr_span)
                }
            };

            event_expression = match event_expression {
                None => Some((current_event_expression, current_span)),
                Some((expr, span)) => {
                    let expr = arenas.add(expr, span);
                    let current_event_expression =
                        arenas.add(current_event_expression, current_span);
                    Some((
                        Self::OrList(expr, current_event_expression),
                        span | current_span,
                    ))
                }
            };

            let Some(peeked) = p.lexer().peek() else {
                break;
            };

            if peeked.kind() != TK::KeywordOr {
                peeked.release();
                break;
            }
        }

        Ok(event_expression.unwrap())
    }
}
impl<'a> Parsable<'a> for EventExpression {}

impl<'a> Consumable<'a> for ProceduralTimingControl {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // procedural_timing_control ::=
        //   delay_control
        // | event_control

        let peeked = p.lexer().next_expect_peek()?;
        match peeked.kind() {
            TK::Hash => {
                peeked.release();
                let (delay_control, span) = DelayControl::parse_with_span(p, arenas)?;
                Ok((Self::DelayControl(delay_control), span))
            }
            TK::AtSign => {
                peeked.release();
                let (event_control, span) = EventControl::parse_with_span(p, arenas)?;
                Ok((Self::EventControl(event_control), span))
            }
            _ => Err(ParseError::unexpected_token(peeked.commit())),
        }
    }
}
impl<'a> Parsable<'a> for ProceduralTimingControl {}
