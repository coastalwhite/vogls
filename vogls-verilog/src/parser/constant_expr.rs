use crate::arena::Arena;
use crate::ast::constant_expr::{
    ConstantExpr, ConstantMinTypMaxExpression, ConstantRangeExpression,
};
use crate::ast::expr::Expr;
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, ParserScratches, TokenWalker};
use super::{Diagnostics, utils::*};

impl<'a> Consumable<'a> for ConstantMinTypMaxExpression<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
        // constant_mintypmax_expression ::=
        //   constant_expression
        // | constant_expression : constant_expression : constant_expression

        let min = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        if tkw.next_if_equals(T::Colon) {
            let typ = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
            tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
            let max = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
            Ok(Self::MinTypMax { min, typ, max })
        } else {
            Ok(Self::Single(min))
        }
    }
}

impl<'a> Consumable<'a> for ConstantRangeExpression<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
        // constant_range_expression ::=
        //   constant_expression
        // | msb_constant_expression : lsb_constant_expression

        let msb = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        if tkw.next_if_equals(T::Colon) {
            let lsb = parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
            Ok(Self::MsbLsb { msb, lsb })
        } else {
            Ok(Self::Single(msb))
        }
    }
}

impl<'a> Consumable<'a> for ConstantExpr<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        let expr = Expr::consume(tkw, sc, arenas, ast, diagnostics)?;
        Ok(Self(expr))
    }
}
