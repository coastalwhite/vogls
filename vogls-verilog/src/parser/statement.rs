use crate::ast::expr::Expr;
use crate::ast::statement::{BlockingAssignment, NonBlockingAssignment, SeqBlock, Statement, VariableLValue};
use crate::ast::IdentRef;
use crate::lexer::TokenKind;
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
            _ => Err(ParseError::Incomplete("statement")),
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
        // @Incomplete: [ delay_or_event_control ]
        let (expression, expression_span) = Expr::parse_with_span(p, arenas)?;

        let span = variable_lvalue_span | expression_span;

        Ok((Self { variable_lvalue, expression }, span))
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
        // @Incomplete: [ delay_or_event_control ]
        let (expression, expression_span) = Expr::parse_with_span(p, arenas)?;

        let span = variable_lvalue_span | expression_span;

        Ok((Self { variable_lvalue, expression }, span))
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
