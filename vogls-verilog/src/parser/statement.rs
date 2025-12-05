use crate::ast::expr::Expr;
use crate::ast::statement::{
    BlockingAssignment, DelayControl, DelayOrEventControl, DelayValue, EventControl,
    EventExpression, NetLValue, NonBlockingAssignment, ProceduralTimingControl, SeqBlock,
    Statement, SystemTaskEnable, SystemTaskIdentifier, VariableLValue,
};
use crate::ast::{AstIdRange, DecimalRef, Identifier, TextRef};
use crate::parser::ItemParsable;
use crate::span::Span;
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, Parser};
use super::{Diagnostics, ParseErrorKind, utils::*};

impl<'a> Consumable<'a> for Statement {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
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

        let peeked = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::KeywordBegin => {
                let (seq_block, span) =
                    parse_with_span::<SeqBlock>(p, arenas, diagnostics.as_deref_mut())?;
                Ok((Self::SeqBlock(seq_block), span))
            }
            T::Hash | T::AtSign => {
                let (procedural_timing_control, procedural_timing_control_span) =
                    parse_with_span::<ProceduralTimingControl>(
                        p,
                        arenas,
                        diagnostics.as_deref_mut(),
                    )?;
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
                    let semicolon_span = *p
                        .tkw
                        .next_expect(T::Semicolon, diagnostics.as_deref_mut())?
                        .span;
                    Ok((
                        Self::BlockingAssignment(blocking_assignment),
                        blocking_assignment_span | semicolon_span,
                    ))
                } else if let Some((non_blocking_assignment, non_blocking_assignment_span)) =
                    try_parse_with_span::<NonBlockingAssignment>(p, arenas)
                {
                    let semicolon_span = *p
                        .tkw
                        .next_expect(T::Semicolon, diagnostics.as_deref_mut())?
                        .span;
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
                    diagnostics.map(|d| d.incomplete(p.tkw.span_at_cursor(), "statement"));
                    Err(ParseErrorKind::Incomplete)
                }
            }
        }
    }
}

impl<'a> Consumable<'a> for NetLValue {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // net_lvalue ::=
        //   hierarchical_net_identifier [ { [ constant_expression ] } [ constant_range_expression ] ]
        // | { net_lvalue { , net_lvalue } }

        // @Incomplete

        let (ident, span) =
            Identifier::item_parse_with_span(p, arenas, diagnostics.as_deref_mut())?;

        Ok((Self { ident }, span))
    }
}

impl<'a> Consumable<'a> for VariableLValue {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // variable_lvalue ::=
        //   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
        //   | { variable_lvalue { , variable_lvalue } }

        // @Incomplete

        let (ident, span) =
            Identifier::item_parse_with_span(p, arenas, diagnostics.as_deref_mut())?;

        Ok((Self { ident }, span))
    }
}

impl<'a> Consumable<'a> for BlockingAssignment {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // blocking_assignment ::= variable_lvalue = [ delay_or_event_control ] expression

        let (variable_lvalue, variable_lvalue_span) =
            parse_with_span::<VariableLValue>(p, arenas, diagnostics.as_deref_mut())?;
        p.tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let delay_or_event_control = try_parse::<DelayOrEventControl>(p, arenas);
        let (expression, expression_span) =
            parse_with_span::<Expr>(p, arenas, diagnostics.as_deref_mut())?;

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
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // nonblocking_assignment ::= variable_lvalue <= [ delay_or_event_control ] expression

        let (variable_lvalue, variable_lvalue_span) =
            parse_with_span::<VariableLValue>(p, arenas, diagnostics.as_deref_mut())?;
        p.tkw
            .next_expect(T::LessThanEquals, diagnostics.as_deref_mut())?;
        let delay_or_event_control = try_parse::<DelayOrEventControl>(p, arenas);
        let (expression, expression_span) =
            parse_with_span::<Expr>(p, arenas, diagnostics.as_deref_mut())?;

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
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // seq_block ::= begin [ : block_identifier { block_item_declaration } ] { statement } end

        // @Incomplete: [ : block_identifier { block_item_declaration } ]
        let begin_kw_span = *p
            .tkw
            .next_expect(T::KeywordBegin, diagnostics.as_deref_mut())?
            .span;
        let statements = parse_until_reaching::<Statement>(
            p,
            arenas,
            T::KeywordEnd,
            diagnostics.as_deref_mut(),
        )?;

        let span = begin_kw_span | *p.tkw.get(p.tkw.offset - 1).unwrap().span;

        Ok((Self { statements }, span))
    }
}

impl<'a> Consumable<'a> for DelayOrEventControl {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_or_event_control ::=
        //   delay_control
        //   | event_control
        //   | repeat ( expression ) event_control

        let peeked = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::Hash => {
                let (delay_control, span) =
                    parse_with_span::<DelayControl>(p, arenas, diagnostics.as_deref_mut())?;
                Ok((Self::DelayControl(delay_control), span))
            }
            T::AtSign => {
                let (event_control, span) =
                    parse_with_span::<EventControl>(p, arenas, diagnostics.as_deref_mut())?;
                Ok((Self::EventControl(event_control), span))
            }
            T::KeywordRepeat => {
                diagnostics.map(|d| d.incomplete(*peeked.span, "delay_or_event_control repeat"));
                Err(ParseErrorKind::Incomplete)
            }
            _ => {
                diagnostics.map(|d| d.unexpected_token(*peeked.span, *peeked.kind));
                Err(ParseErrorKind::UnexpectedToken)
            }
        }
    }
}

impl<'a> Consumable<'a> for DelayControl {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_control ::=
        //   # delay_value
        // | # ( mintypmax_expression )

        let hash_span = *p.tkw.next_expect(T::Hash, diagnostics.as_deref_mut())?.span;
        // @Incomplete: | # ( mintypmax_expression )
        let (delay_value, delay_value_span) =
            parse_with_span::<DelayValue>(p, arenas, diagnostics.as_deref_mut())?;

        let span = hash_span | delay_value_span;

        Ok((Self::DelayValue(delay_value), span))
    }
}

impl<'a> Consumable<'a> for DelayValue {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
        // delay_value ::=
        //   unsigned_number
        // | real_number
        // | identifier

        // @Incomplete: | real_number
        // @Incomplete: | identifier

        let peeked = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::Decimal => {
                let (decimal, span) = DecimalRef::consume(p, arenas, diagnostics.as_deref_mut())?;
                Ok((Self::UnsignedNumber(decimal), span))
            }
            T::Ident => {
                let (ident, span) = Identifier::consume(p, arenas, diagnostics.as_deref_mut())?;
                Ok((Self::Identifier(ident), span))
            }
            _ => {
                diagnostics.map(|d| d.incomplete(*peeked.span, "delay_value"));
                Err(ParseErrorKind::Incomplete)
            }
        }
    }
}

impl<'a> Consumable<'a> for EventControl {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_control ::=
        //   @ hierarchical_event_identifier
        // | @ ( event_expression )
        // | @*
        // | @ (*)

        let at_sign_span = *p
            .tkw
            .next_expect(T::AtSign, diagnostics.as_deref_mut())?
            .span;
        // @Incomplete: @ hierarchical_event_identifier
        // @Incomplete: @*
        // @Incomplete: @ (*)
        p.tkw
            .next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let event_expression = parse::<EventExpression>(p, arenas, diagnostics.as_deref_mut())?;
        let right_paren_span = *p
            .tkw
            .next_expect(T::RightParen, diagnostics.as_deref_mut())?
            .span;

        let span = at_sign_span | right_paren_span;

        Ok((Self::EventExpression(event_expression), span))
    }
}

impl<'a> Consumable<'a> for EventExpression {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
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
            let peeked = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
            let (current_event_expression, current_span) = match peeked.kind {
                T::KeywordPosedge => {
                    let posedge_kw_span = *peeked.span;
                    p.tkw.next();
                    let (expr, expr_span) =
                        parse_with_span::<Expr>(p, arenas, diagnostics.as_deref_mut())?;
                    let span = posedge_kw_span | expr_span;
                    (Self::Posedge(expr), span)
                }
                T::KeywordNegedge => {
                    let negedge_kw_span = *peeked.span;
                    p.tkw.next();
                    let (expr, expr_span) =
                        parse_with_span::<Expr>(p, arenas, diagnostics.as_deref_mut())?;
                    let span = negedge_kw_span | expr_span;
                    (Self::Negedge(expr), span)
                }
                _ => {
                    let (expr, expr_span) =
                        parse_with_span::<Expr>(p, arenas, diagnostics.as_deref_mut())?;
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
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // procedural_timing_control ::=
        //   delay_control
        // | event_control

        let peeked = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::Hash => {
                let (delay_control, span) =
                    parse_with_span::<DelayControl>(p, arenas, diagnostics.as_deref_mut())?;
                Ok((Self::DelayControl(delay_control), span))
            }
            T::AtSign => {
                let (event_control, span) =
                    parse_with_span::<EventControl>(p, arenas, diagnostics.as_deref_mut())?;
                Ok((Self::EventControl(event_control), span))
            }
            _ => {
                diagnostics.map(|d| d.unexpected_token(*peeked.span, *peeked.kind));
                Err(ParseErrorKind::UnexpectedToken)
            }
        }
    }
}

impl<'a> Consumable<'a> for SystemTaskEnable {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // system_task_enable ::= system_task_identifier [ ( [ expression ] { , [ expression ] } ) ] ;

        let (system_task_identifier, system_task_identifier_span) =
            SystemTaskIdentifier::item_parse_with_span(p, arenas, diagnostics.as_deref_mut())?;
        let mut expressions = AstIdRange::default();
        if p.tkw.next_if_equals(T::LeftParen) {
            expressions = parse_zero_or_more_delimited::<Expr>(
                p,
                arenas,
                T::Comma,
                diagnostics.as_deref_mut(),
            )?;
            p.tkw
                .next_expect(T::RightParen, diagnostics.as_deref_mut())?;
        }
        let semicolon_span = *p
            .tkw
            .next_expect(T::Semicolon, diagnostics.as_deref_mut())?
            .span;

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
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;
        let t = p
            .tkw
            .next_expect(T::DollarIdent, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.start() + 1..span.end()];
        Ok((
            Self::from_item(content, arenas, diagnostics.as_deref_mut())?,
            span,
        ))
    }
}
impl<'a> ItemParsable<'a> for SystemTaskIdentifier {
    type Item = &'a str;
    fn from_item(
        item: Self::Item,
        arenas: &mut AstArenas,
        _diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        let start = arenas.text.len();
        let end = start + item.len();
        arenas.text.push_str(item);
        Ok(Self(TextRef { start, end }))
    }
}
