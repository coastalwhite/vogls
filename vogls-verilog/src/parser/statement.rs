use crate::ast::expr::Expr;
use crate::ast::statement::{
    BlockingAssignment, DelayControl, DelayOrEventControl, DelayValue, EventControl,
    EventExpression, NetLValue, NonBlockingAssignment, ProceduralTimingControl, SeqBlock,
    Statement, SystemTaskEnable, SystemTaskIdentifier, VariableLValue,
};
use crate::ast::{AstIdRange, DecimalRef, Identifier, TextRef};
use crate::parser::ItemParsable;
use crate::parser::token_walker::TokenRange;
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, Parser};
use super::{Diagnostics, ParseErrorKind, utils::*};

impl<'a> Consumable<'a> for Statement {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
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
                let seq_block = parse::<SeqBlock>(p, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::SeqBlock(seq_block))
            }
            T::Hash | T::AtSign => {
                let procedural_timing_control =
                    parse::<ProceduralTimingControl>(p, arenas, diagnostics.as_deref_mut())?;

                let statement = try_parse::<Statement>(p, arenas);
                Ok(Self::ProceduralTimingControlStatement(
                    procedural_timing_control,
                    statement,
                ))
            }
            _ => {
                if let Some(blocking_assignment) = try_parse::<BlockingAssignment>(p, arenas) {
                    p.tkw
                        .next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                    Ok(Self::BlockingAssignment(blocking_assignment))
                } else if let Some(non_blocking_assignment) =
                    try_parse::<NonBlockingAssignment>(p, arenas)
                {
                    p.tkw
                        .next_expect(T::Semicolon, diagnostics.as_deref_mut())?;
                    Ok(Self::NonBlockingAssignment(non_blocking_assignment))
                } else if let Some(system_task_enable) = try_parse::<SystemTaskEnable>(p, arenas) {
                    Ok(Self::SystemTaskEnable(system_task_enable))
                } else {
                    diagnostics.map(|d| d.incomplete(p.tkw.offset, "statement"));
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
    ) -> Result<Self, ParseErrorKind> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // net_lvalue ::=
        //   hierarchical_net_identifier [ { [ constant_expression ] } [ constant_range_expression ] ]
        // | { net_lvalue { , net_lvalue } }

        // @Incomplete

        let ident = Identifier::item_parse(p, arenas, diagnostics.as_deref_mut())?;

        Ok(Self { ident })
    }
}

impl<'a> Consumable<'a> for VariableLValue {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
        // variable_lvalue ::=
        //   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
        //   | { variable_lvalue { , variable_lvalue } }

        // @Incomplete

        let ident = Identifier::item_parse(p, arenas, diagnostics.as_deref_mut())?;

        Ok(Self { ident })
    }
}

impl<'a> Consumable<'a> for BlockingAssignment {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // blocking_assignment ::= variable_lvalue = [ delay_or_event_control ] expression

        let variable_lvalue = parse::<VariableLValue>(p, arenas, diagnostics.as_deref_mut())?;
        p.tkw.next_expect(T::Equals, diagnostics.as_deref_mut())?;
        let delay_or_event_control = try_parse::<DelayOrEventControl>(p, arenas);
        let expression = parse::<Expr>(p, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            variable_lvalue,
            delay_or_event_control,
            expression,
        })
    }
}

impl<'a> Consumable<'a> for NonBlockingAssignment {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
        // nonblocking_assignment ::= variable_lvalue <= [ delay_or_event_control ] expression

        let variable_lvalue = parse::<VariableLValue>(p, arenas, diagnostics.as_deref_mut())?;
        p.tkw
            .next_expect(T::LessThanEquals, diagnostics.as_deref_mut())?;
        let delay_or_event_control = try_parse::<DelayOrEventControl>(p, arenas);
        let expression = parse::<Expr>(p, arenas, diagnostics.as_deref_mut())?;

        Ok(Self {
            variable_lvalue,
            delay_or_event_control,
            expression,
        })
    }
}

impl<'a> Consumable<'a> for SeqBlock {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // seq_block ::= begin [ : block_identifier { block_item_declaration } ] { statement } end

        // @Incomplete: [ : block_identifier { block_item_declaration } ]
        p.tkw
            .next_expect(T::KeywordBegin, diagnostics.as_deref_mut())?;
        let statements = parse_until_reaching::<Statement>(
            p,
            arenas,
            T::KeywordEnd,
            diagnostics.as_deref_mut(),
        )?;

        Ok(Self { statements })
    }
}

impl<'a> Consumable<'a> for DelayOrEventControl {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_or_event_control ::=
        //   delay_control
        //   | event_control
        //   | repeat ( expression ) event_control

        let peeked = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
        match *peeked.kind {
            T::Hash => {
                let delay_control = parse::<DelayControl>(p, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::DelayControl(delay_control))
            }
            T::AtSign => {
                let event_control = parse::<EventControl>(p, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::EventControl(event_control))
            }
            T::KeywordRepeat => {
                diagnostics.map(|d| d.incomplete(p.tkw.offset, "delay_or_event_control repeat"));
                Err(ParseErrorKind::Incomplete)
            }
            t => {
                diagnostics.map(|d| d.unexpected_token(p.tkw.offset, t));
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
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // delay_control ::=
        //   # delay_value
        // | # ( mintypmax_expression )

        p.tkw.next_expect(T::Hash, diagnostics.as_deref_mut())?;
        // @Incomplete: | # ( mintypmax_expression )
        let delay_value = parse::<DelayValue>(p, arenas, diagnostics.as_deref_mut())?;

        Ok(Self::DelayValue(delay_value))
    }
}

impl<'a> Consumable<'a> for DelayValue {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
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
                let decimal = DecimalRef::consume(p, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::UnsignedNumber(decimal))
            }
            T::Ident => {
                let ident = Identifier::consume(p, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::Identifier(ident))
            }
            _ => {
                diagnostics.map(|d| d.incomplete(p.tkw.offset, "delay_value"));
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
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_control ::=
        //   @ hierarchical_event_identifier
        // | @ ( event_expression )
        // | @*
        // | @ (*)

        p.tkw.next_expect(T::AtSign, diagnostics.as_deref_mut())?;
        // @Incomplete: @ hierarchical_event_identifier
        // @Incomplete: @*
        // @Incomplete: @ (*)
        p.tkw
            .next_expect(T::LeftParen, diagnostics.as_deref_mut())?;
        let event_expression = parse::<EventExpression>(p, arenas, diagnostics.as_deref_mut())?;
        p.tkw
            .next_expect(T::RightParen, diagnostics.as_deref_mut())?;

        Ok(Self::EventExpression(event_expression))
    }
}

impl<'a> Consumable<'a> for EventExpression {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
        // event_expression ::=
        //   expression
        // | posedge expression
        // | negedge expression
        // | event_expression or event_expression

        let start = p.tkw.offset;
        let mut event_expression = None;

        // @Incomplete: | event_expression or event_expression
        loop {
            let start_current = p.tkw.offset;
            let peeked = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
            let current_event_expression = match peeked.kind {
                T::KeywordPosedge => {
                    p.tkw.next();
                    let expr = parse::<Expr>(p, arenas, diagnostics.as_deref_mut())?;
                    Self::Posedge(expr)
                }
                T::KeywordNegedge => {
                    p.tkw.next();
                    let expr = parse::<Expr>(p, arenas, diagnostics.as_deref_mut())?;
                    Self::Negedge(expr)
                }
                _ => {
                    let expr = parse::<Expr>(p, arenas, diagnostics.as_deref_mut())?;
                    Self::Expression(expr)
                }
            };

            event_expression = match event_expression {
                None => Some(current_event_expression),
                Some(expr) => {
                    let token_range = TokenRange {
                        start,
                        end: p.tkw.offset,
                    };
                    let expr = arenas.add(expr, token_range);

                    let token_range = TokenRange {
                        start: start_current,
                        end: p.tkw.offset,
                    };
                    let current_event_expression =
                        arenas.add(current_event_expression, token_range);

                    Some(Self::OrList(expr, current_event_expression))
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
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // procedural_timing_control ::=
        //   delay_control
        // | event_control

        let peeked = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
        match *peeked.kind {
            T::Hash => {
                let delay_control = parse::<DelayControl>(p, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::DelayControl(delay_control))
            }
            T::AtSign => {
                let event_control = parse::<EventControl>(p, arenas, diagnostics.as_deref_mut())?;
                Ok(Self::EventControl(event_control))
            }
            t => {
                diagnostics.map(|d| d.unexpected_token(p.tkw.offset, t));
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
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
        // system_task_enable ::= system_task_identifier [ ( [ expression ] { , [ expression ] } ) ] ;

        let system_task_identifier =
            SystemTaskIdentifier::item_parse(p, arenas, diagnostics.as_deref_mut())?;
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
        p.tkw
            .next_expect(T::Semicolon, diagnostics.as_deref_mut())?;

        Ok(Self {
            system_task_identifier,
            expressions,
        })
    }
}

impl<'a> Consumable<'a> for SystemTaskIdentifier {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;
        let t = p
            .tkw
            .next_expect(T::DollarIdent, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.start() + 1..span.end()];
        Self::from_item(content, arenas, diagnostics.as_deref_mut())
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
