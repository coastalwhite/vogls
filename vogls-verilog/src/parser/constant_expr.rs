use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression, ConstantPrimary};
use crate::ast::{DecimalRef, StringRef};
use crate::span::Span;
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, ParseError, Parser};
use super::{Diagnostics, ParseErrorKind, utils::*};

impl<'a> Consumable<'a> for ConstantMinTypMaxExpression {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
        // constant_mintypmax_expression ::=
        //   constant_expression
        // | constant_expression : constant_expression : constant_expression

        let (min, min_span) =
            parse_with_span::<ConstantExpr>(p, arenas, diagnostics.as_deref_mut())?;
        if p.tkw.next_if_equals(T::Colon) {
            let typ = parse::<ConstantExpr>(p, arenas, diagnostics.as_deref_mut())?;
            p.tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
            let (max, max_span) =
                parse_with_span::<ConstantExpr>(p, arenas, diagnostics.as_deref_mut())?;
            let span = min_span | max_span;
            Ok((Self::MinTypMax { min, typ, max }, span))
        } else {
            Ok((Self::Single(min), min_span))
        }
    }
}

impl<'a> Consumable<'a> for ConstantExpr {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
        // constant_expression ::=
        //   constant_primary
        // | unary_operator { attribute_instance } constant_primary
        // | constant_expression binary_operator { attribute_instance } constant_expression
        // | constant_expression ? { attribute_instance } constant_expression : constant_expression

        // @Incomplete
        let (primary, span) = ConstantPrimary::consume(p, arenas, diagnostics.as_deref_mut())?;

        Ok((Self::Primary(primary), span))
    }
}

impl<'a> Consumable<'a> for ConstantPrimary {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        use Token as T;

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

        let peeked = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
        match peeked.kind {
            T::Decimal => {
                let (decimal, span) = DecimalRef::consume(p, arenas, diagnostics.as_deref_mut())?;
                Ok((Self::Number(decimal), span))
            }
            T::String => {
                let (string, span) = StringRef::consume(p, arenas, diagnostics.as_deref_mut())?;
                Ok((Self::String(string), span))
            }
            _ => {
                diagnostics.map(|d| d.incomplete(*peeked.span, "constant_primary"));
                Err(ParseErrorKind::Incomplete)
            }
        }
    }
}
