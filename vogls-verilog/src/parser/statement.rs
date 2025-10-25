use crate::ast::expr::Expr;
use crate::ast::statement::{
    BlockingAssignment, DelayControl, DelayOrEventControl, DelayValue, EventControl,
    EventExpression, NetLValue, NonBlockingAssignment, ProceduralTimingControl, SeqBlock,
    Statement, SystemTaskEnable, SystemTaskIdentifier, VariableLValue,
};
use crate::ast::{AstIdRange, DecimalRef, Identifier, TextRef};
use crate::parser::ItemParsable;
use crate::span::Span;
use crate::tokenizer::{FromLexerError, Token};

use super::utils::*;
use super::{AstArenas, Consumable, ParseError, Parser};

impl<'a> Consumable<'a> for Statement {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
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

        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match peeked.kind {
            T::KeywordBegin => {
                let (seq_block, span) = parse_with_span::<SeqBlock>(p, arenas)?;
                Ok((Self::SeqBlock(seq_block), span))
            }
            T::Hash | T::AtSign => {
                let (procedural_timing_control, procedural_timing_control_span) =
                    parse_with_span::<ProceduralTimingControl>(p, arenas)?;
                let mut span = procedural_timing_control_span;

                let statement =
                    try_parse_with_span::<Statement>(p, arenas).map(|(stmt, stmt_span)| {
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
                    try_parse_with_span::<BlockingAssignment>(p, arenas)
                {
                    let semicolon_span = *p.tkw.next_expect(T::Semicolon)?;
                    Ok((
                        Self::BlockingAssignment(blocking_assignment),
                        blocking_assignment_span | semicolon_span,
                    ))
                } else if let Some((non_blocking_assignment, non_blocking_assignment_span)) =
                    try_parse_with_span::<NonBlockingAssignment>(p, arenas)
                {
                    let semicolon_span = *p.tkw.next_expect(T::Semicolon)?;
                    Ok((
                        Self::NonBlockingAssignment(non_blocking_assignment),
                        non_blocking_assignment_span | semicolon_span,
                    ))
                } else if let Some((system_task_enable, system_task_enable_span)) =
                    try_parse_with_span::<SystemTaskEnable>(p, arenas)
                {
                    Ok((
                        Self::SystemTaskEnable(system_task_enable),
                        system_task_enable_span,
                    ))
                } else {
                    Err(ParseError::incomplete(
                        Some(p.tkw.span_at_cursor()),
                        "statement",
                    ))
                }
            }
        }
    }
}

impl<'a> Consumable<'a> for NetLValue {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // net_lvalue ::=
        //   hierarchical_net_identifier [ { [ constant_expression ] } [ constant_range_expression ] ]
        // | { net_lvalue { , net_lvalue } }

        // @Incomplete

        let (ident, span) = Identifier::item_parse_with_span(p, arenas)?;

        Ok((Self { ident }, span))
    }
}

impl<'a> Consumable<'a> for VariableLValue {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // variable_lvalue ::=
        //   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
        //   | { variable_lvalue { , variable_lvalue } }

        // @Incomplete

        let (ident, span) = Identifier::item_parse_with_span(p, arenas)?;

        Ok((Self { ident }, span))
    }
}

impl<'a> Consumable<'a> for BlockingAssignment {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // blocking_assignment ::= variable_lvalue = [ delay_or_event_control ] expression

        let (variable_lvalue, variable_lvalue_span) = parse_with_span::<VariableLValue>(p, arenas)?;
        p.tkw.next_expect(T::Equals)?;
        let delay_or_event_control = try_parse::<DelayOrEventControl>(p, arenas);
        let (expression, expression_span) = parse_with_span::<Expr>(p, arenas)?;

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

impl<'a> Consumable<'a> for NonBlockingAssignment {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // nonblocking_assignment ::= variable_lvalue <= [ delay_or_event_control ] expression

        let (variable_lvalue, variable_lvalue_span) = parse_with_span::<VariableLValue>(p, arenas)?;
        p.tkw.next_expect(T::LessThanEquals)?;
        let delay_or_event_control = try_parse::<DelayOrEventControl>(p, arenas);
        let (expression, expression_span) = parse_with_span::<Expr>(p, arenas)?;

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

impl<'a> Consumable<'a> for SeqBlock {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // seq_block ::= begin [ : block_identifier { block_item_declaration } ] { statement } end

        // @Incomplete: [ : block_identifier { block_item_declaration } ]
        let begin_kw_span = *p.tkw.next_expect(T::KeywordBegin)?;
        let statements = parse_until_reaching::<Statement>(p, arenas, T::KeywordEnd)?;

        let span = begin_kw_span | *p.tkw.get(p.tkw.offset - 1).unwrap().span;

        Ok((Self { statements }, span))
    }
}

impl<'a> Consumable<'a> for DelayOrEventControl {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_or_event_control ::=
        //   delay_control
        //   | event_control
        //   | repeat ( expression ) event_control

        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match peeked.kind {
            T::Hash => {
                let (delay_control, span) = parse_with_span::<DelayControl>(p, arenas)?;
                Ok((Self::DelayControl(delay_control), span))
            }
            T::AtSign => {
                let (event_control, span) = parse_with_span::<EventControl>(p, arenas)?;
                Ok((Self::EventControl(event_control), span))
            }
            T::KeywordRepeat => Err(ParseError::incomplete(
                Some(p.tkw.span_at_cursor()),
                "delay_or_event_control repeat",
            )),
            _ => Err(ParseError::unexpected_token()),
        }
    }
}

impl<'a> Consumable<'a> for DelayControl {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_control ::=
        //   # delay_value
        // | # ( mintypmax_expression )

        let hash_span = *p.tkw.next_expect(T::Hash)?;
        // @Incomplete: | # ( mintypmax_expression )
        let (delay_value, delay_value_span) = parse_with_span::<DelayValue>(p, arenas)?;

        let span = hash_span | delay_value_span;

        Ok((Self::DelayValue(delay_value), span))
    }
}

impl<'a> Consumable<'a> for DelayValue {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // delay_value ::=
        //   unsigned_number
        // | real_number
        // | identifier

        // @Incomplete: | real_number
        // @Incomplete: | identifier

        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match peeked.kind {
            T::Decimal => {
                let (decimal, span) = DecimalRef::consume(p, arenas)?;
                Ok((Self::UnsignedNumber(decimal), span))
            }
            T::Ident => {
                let (ident, span) = Identifier::consume(p, arenas)?;
                Ok((Self::Identifier(ident), span))
            }
            _ => Err(ParseError::incomplete(
                Some(p.tkw.span_at_cursor()),
                "delay_value",
            )),
        }
    }
}

impl<'a> Consumable<'a> for EventControl {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_control ::=
        //   @ hierarchical_event_identifier
        // | @ ( event_expression )
        // | @*
        // | @ (*)

        let at_sign_span = *p.tkw.next_expect(T::AtSign)?;
        // @Incomplete: @ hierarchical_event_identifier
        // @Incomplete: @*
        // @Incomplete: @ (*)
        p.tkw.next_expect(T::LeftParen)?;
        let event_expression = parse::<EventExpression>(p, arenas)?;
        let right_paren_span = *p.tkw.next_expect(T::RightParen)?;

        let span = at_sign_span | right_paren_span;

        Ok((Self::EventExpression(event_expression), span))
    }
}

impl<'a> Consumable<'a> for EventExpression {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_expression ::=
        //   expression
        // | posedge expression
        // | negedge expression
        // | event_expression or event_expression

        let mut event_expression = None;

        // @Incomplete: | event_expression or event_expression
        loop {
            let peeked = p.tkw.try_get(p.tkw.offset)?;
            let (current_event_expression, current_span) = match peeked.kind {
                T::KeywordPosedge => {
                    let posedge_kw_span = *peeked.span;
                    p.tkw.next();
                    let (expr, expr_span) = parse_with_span::<Expr>(p, arenas)?;
                    let span = posedge_kw_span | expr_span;
                    (Self::Posedge(expr), span)
                }
                T::KeywordNegedge => {
                    let negedge_kw_span = *peeked.span;
                    p.tkw.next();
                    let (expr, expr_span) = parse_with_span::<Expr>(p, arenas)?;
                    let span = negedge_kw_span | expr_span;
                    (Self::Negedge(expr), span)
                }
                _ => {
                    let (expr, expr_span) = parse_with_span::<Expr>(p, arenas)?;
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

            let Some(peeked) = p.tkw.get(p.tkw.offset) else {
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
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // procedural_timing_control ::=
        //   delay_control
        // | event_control

        let peeked = p.tkw.try_get(p.tkw.offset)?;
        match peeked.kind {
            T::Hash => {
                let (delay_control, span) = parse_with_span::<DelayControl>(p, arenas)?;
                Ok((Self::DelayControl(delay_control), span))
            }
            T::AtSign => {
                let (event_control, span) = parse_with_span::<EventControl>(p, arenas)?;
                Ok((Self::EventControl(event_control), span))
            }
            _ => Err(ParseError::unexpected_token()),
        }
    }
}

impl<'a> Consumable<'a> for SystemTaskEnable {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // system_task_enable ::= system_task_identifier [ ( [ expression ] { , [ expression ] } ) ] ;

        let (system_task_identifier, system_task_identifier_span) =
            SystemTaskIdentifier::item_parse_with_span(p, arenas)?;
        let mut expressions = AstIdRange::default();
        if p.tkw.next_if_equals(T::LeftParen) {
            expressions = parse_zero_or_more_delimited::<Expr>(p, arenas, T::Comma)?;
            p.tkw.next_expect(T::RightParen)?;
        }
        let semicolon_span = *p.tkw.next_expect(T::Semicolon)?;

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

impl<'a> Consumable<'a> for SystemTaskIdentifier {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use Token as T;
        let span = *p.tkw.next_expect(T::DollarIdent)?;
        let content = &p.tkw.content()[span.start() + 1..span.end()];
        Ok((Self::from_item(content, arenas)?, span))
    }
}
impl<'a> ItemParsable<'a> for SystemTaskIdentifier {
    type Item = &'a str;
    fn from_item(item: Self::Item, arenas: &mut AstArenas) -> Result<Self, ParseError> {
        let start = arenas.text.len();
        let end = start + item.len();
        arenas.text.push_str(item);
        Ok(Self(TextRef { start, end }))
    }
}
