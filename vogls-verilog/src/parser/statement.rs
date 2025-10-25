use crate::ast::expr::Expr;
use crate::ast::statement::{
    BlockingAssignment, DelayControl, DelayOrEventControl, DelayValue, EventControl,
    EventExpression, NetLValue, NonBlockingAssignment, ProceduralTimingControl, SeqBlock,
    Statement, SystemTaskEnable, SystemTaskIdentifier, VariableLValue,
};
use crate::ast::{AstIdRange, DecimalRef, Identifier, TextRef};
use crate::lexer::{FromLexer2Error, TokenKind};
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

        let peeked = p.lexer.try_get(p.lexer.offset)?;
        match peeked.kind {
            TK::KeywordBegin => {
                let (seq_block, span) = SeqBlock::parse_with_span(p, arenas)?;
                Ok((Self::SeqBlock(seq_block), span))
            }
            TK::Hash | TK::AtSign => {
                let (procedural_timing_control, procedural_timing_control_span) =
                    ProceduralTimingControl::parse_with_span(p, arenas)?;
                let mut span = procedural_timing_control_span;

                let statement =
                    Statement::try_parse_with_span(p, arenas).map(|(stmt, stmt_span)| {
                        span |= stmt_span;
                        stmt
                    });
                Ok((
                    Self::ProceduralTimingControlStatement(procedural_timing_control, statement),
                    span,
                ))
            }
            _ => {
                if let Some((blocking_assignment, blocking_assignment_span)) =
                    BlockingAssignment::try_parse_with_span(p, arenas)
                {
                    let semicolon_span = *p.lexer.next_expect(TK::Semicolon)?;
                    Ok((
                        Self::BlockingAssignment(blocking_assignment),
                        blocking_assignment_span | semicolon_span,
                    ))
                } else if let Some((non_blocking_assignment, non_blocking_assignment_span)) =
                    NonBlockingAssignment::try_parse_with_span(p, arenas)
                {
                    let semicolon_span = *p.lexer.next_expect(TK::Semicolon)?;
                    Ok((
                        Self::NonBlockingAssignment(non_blocking_assignment),
                        non_blocking_assignment_span | semicolon_span,
                    ))
                } else if let Some((system_task_enable, system_task_enable_span)) =
                    SystemTaskEnable::try_parse_with_span(p, arenas)
                {
                    Ok((
                        Self::SystemTaskEnable(system_task_enable),
                        system_task_enable_span,
                    ))
                } else {
                    Err(ParseError::incomplete(
                        Some(p.lexer.span_at_cursor()),
                        "statement",
                    ))
                }
            }
        }
    }
}
impl<'a> Parsable<'a> for Statement {}

impl<'a> Consumable<'a> for NetLValue {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // net_lvalue ::=
        //   hierarchical_net_identifier [ { [ constant_expression ] } [ constant_range_expression ] ]
        // | { net_lvalue { , net_lvalue } }

        // @Incomplete

        let (ident, span) = Identifier::parse_with_span(p, arenas)?;

        Ok((Self { ident }, span))
    }
}
impl<'a> Parsable<'a> for NetLValue {}

impl<'a> Consumable<'a> for VariableLValue {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // variable_lvalue ::=
        //   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
        //   | { variable_lvalue { , variable_lvalue } }

        // @Incomplete

        let (ident, span) = Identifier::parse_with_span(p, arenas)?;

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
        p.lexer.next_expect(TK::Equals)?;
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
        p.lexer.next_expect(TK::LessThanEquals)?;
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
        let begin_kw_span = *p.lexer.next_expect(TK::KeywordBegin)?;
        let statements = Statement::parse_until_reaching(p, arenas, TK::KeywordEnd)?;

        let span = begin_kw_span | *p.lexer.get(p.lexer.offset - 1).unwrap().span;

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

        let peeked = p.lexer.try_get(p.lexer.offset)?;
        match peeked.kind {
            TK::Hash => {
                let (delay_control, span) = DelayControl::parse_with_span(p, arenas)?;
                Ok((Self::DelayControl(delay_control), span))
            }
            TK::AtSign => {
                let (event_control, span) = EventControl::parse_with_span(p, arenas)?;
                Ok((Self::EventControl(event_control), span))
            }
            TK::KeywordRepeat => Err(ParseError::incomplete(
                Some(p.lexer.span_at_cursor()),
                "delay_or_event_control repeat",
            )),
            _ => Err(ParseError::unexpected_token()),
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

        let hash_span = *p.lexer.next_expect(TK::Hash)?;
        // @Incomplete: | # ( mintypmax_expression )
        let (delay_value, delay_value_span) = DelayValue::parse_with_span(p, arenas)?;

        let span = hash_span | delay_value_span;

        Ok((Self::DelayValue(delay_value), span))
    }
}
impl<'a> Parsable<'a> for DelayControl {}

impl<'a> Consumable<'a> for DelayValue {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // delay_value ::=
        //   unsigned_number
        // | real_number
        // | identifier

        // @Incomplete: | real_number
        // @Incomplete: | identifier

        let peeked = p.lexer.try_get(p.lexer.offset)?;
        match peeked.kind {
            TK::Decimal => {
                let (decimal, span) = DecimalRef::consume(p, arenas)?;
                Ok((Self::UnsignedNumber(decimal), span))
            }
            TK::Ident => {
                let (ident, span) = Identifier::consume(p, arenas)?;
                Ok((Self::Identifier(ident), span))
            }
            _ => Err(ParseError::incomplete(
                Some(p.lexer.span_at_cursor()),
                "delay_value",
            )),
        }
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

        let at_sign_span = *p.lexer.next_expect(TK::AtSign)?;
        // @Incomplete: @ hierarchical_event_identifier
        // @Incomplete: @*
        // @Incomplete: @ (*)
        p.lexer.next_expect(TK::LeftParen)?;
        let event_expression = EventExpression::parse(p, arenas)?;
        let right_paren_span = *p.lexer.next_expect(TK::RightParen)?;

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
            let peeked = p.lexer.try_get(p.lexer.offset)?;
            let (current_event_expression, current_span) = match peeked.kind {
                TK::KeywordPosedge => {
                    let posedge_kw_span = *peeked.span;
                    p.lexer.next();
                    let (expr, expr_span) = Expr::parse_with_span(p, arenas)?;
                    let span = posedge_kw_span | expr_span;
                    (Self::Posedge(expr), span)
                }
                TK::KeywordNegedge => {
                    let negedge_kw_span = *peeked.span;
                    p.lexer.next();
                    let (expr, expr_span) = Expr::parse_with_span(p, arenas)?;
                    let span = negedge_kw_span | expr_span;
                    (Self::Negedge(expr), span)
                }
                _ => {
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

            let Some(peeked) = p.lexer.get(p.lexer.offset) else {
                break;
            };

            if *peeked.kind != TK::KeywordOr {
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

        let peeked = p.lexer.try_get(p.lexer.offset)?;
        match peeked.kind {
            TK::Hash => {
                let (delay_control, span) = DelayControl::parse_with_span(p, arenas)?;
                Ok((Self::DelayControl(delay_control), span))
            }
            TK::AtSign => {
                let (event_control, span) = EventControl::parse_with_span(p, arenas)?;
                Ok((Self::EventControl(event_control), span))
            }
            _ => Err(ParseError::unexpected_token()),
        }
    }
}
impl<'a> Parsable<'a> for ProceduralTimingControl {}

impl<'a> Consumable<'a> for SystemTaskEnable {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // system_task_enable ::= system_task_identifier [ ( [ expression ] { , [ expression ] } ) ] ;

        let (system_task_identifier, system_task_identifier_span) =
            SystemTaskIdentifier::parse_with_span(p, arenas)?;
        let mut expressions = AstIdRange::default();
        if p.lexer.next_if_equals(TK::LeftParen) {
            expressions = Expr::parse_zero_or_more_delimited(p, arenas, TK::Comma)?;
            p.lexer.next_expect(TK::RightParen)?;
        }
        let semicolon_span = *p.lexer.next_expect(TK::Semicolon)?;

        let span = system_task_identifier_span | semicolon_span;

        Ok((
            Self {
                system_task_identifier,
                expressions,
            },
            span,
        ))
    }
}
impl<'a> Parsable<'a> for SystemTaskEnable {}

impl<'a> Consumable<'a> for SystemTaskIdentifier {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;
        let span = *p.lexer.next_expect(TK::DollarIdent)?;
        let content = &p.lexer.content()[span.start() + 1..span.end()];
        Ok((Self::from_item(content, arenas)?, span))
    }
}
impl<'a> ItemParsable<'a> for SystemTaskIdentifier {
    type Item = &'a str;
    fn from_item(item: Self::Item, arenas: &mut AstArenas) -> Result<Self, ParseError> {
        let start = arenas.idents.len();
        let end = start + item.len();
        arenas.idents.push_str(item);
        Ok(Self(TextRef { start, end }))
    }
}
