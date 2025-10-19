use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression, ConstantPrimary};
use crate::ast::{DecimalRef, StringRef};
use crate::lexer::TokenKind;
use crate::parser::ItemParsable;
use crate::span::Span;

use super::{AstArenas, Consumable, Parsable, ParseError, Parser};

impl<'a> Consumable<'a> for ConstantMinTypMaxExpression {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
        // constant_mintypmax_expression ::=
        //   constant_expression
        // | constant_expression : constant_expression : constant_expression

        let (min, min_span) = ConstantExpr::parse_with_span(p, arenas)?;
        if p.lexer.next_if_equals(TK::Colon).is_some() {
            let typ = ConstantExpr::parse(p, arenas)?;
            p.lexer.expect(TK::Colon)?;
            let (max, max_span) = ConstantExpr::parse_with_span(p, arenas)?;
            let span = min_span | max_span;
            Ok((Self::MinTypMax { min, typ, max }, span))
        } else {
            Ok((Self::Single(min), min_span))
        }
    }
}
impl<'a> Parsable<'a> for ConstantMinTypMaxExpression {}

impl<'a> Consumable<'a> for ConstantExpr {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
        // constant_expression ::=
        //   constant_primary
        // | unary_operator { attribute_instance } constant_primary
        // | constant_expression binary_operator { attribute_instance } constant_expression
        // | constant_expression ? { attribute_instance } constant_expression : constant_expression

        // @Incomplete
        let (primary, span) = ConstantPrimary::consume(p, arenas)?;

        Ok((Self::Primary(primary), span))
    }
}
impl<'a> Parsable<'a> for ConstantExpr {}

impl<'a> Consumable<'a> for ConstantPrimary {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use TokenKind as TK;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 505
        // constant_primary ::=
        //   number
        // | parameter_identifier [ [ constant_range_expression ] ]
        // | specparam_identifier [ [ constant_range_expression ] ]
        // | constant_concatenation
        // | constant_multiple_concatenation
        // | constant_function_call
        // | constant_system_function_call
        // | ( constant_mintypmax_expression )
        // | string

        let peeked = p.lexer.next_expect_peek()?;
        match peeked.kind() {
            TK::Decimal => {
                peeked.release();
                let (decimal, span) = DecimalRef::consume(p, arenas)?;
                Ok((Self::Number(decimal), span))
            }
            TK::String => {
                peeked.release();
                let (string, span) = StringRef::consume(p, arenas)?;
                Ok((Self::String(string), span))
            }
            _ => {
                let token = peeked.commit();
                Err(ParseError::incomplete(
                    Some(token.span()),
                    "constant_primary",
                ))
            }
        }
    }
}
impl<'a> Parsable<'a> for ConstantPrimary {}
