use super::AstId;
use super::expr::Expr;

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
// constant_expression ::=
//   constant_primary
// | unary_operator { attribute_instance } constant_primary
// | constant_expression binary_operator { attribute_instance } constant_expression
// | constant_expression ? { attribute_instance } constant_expression : constant_expression
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ConstantExpr<'a>(pub Expr<'a>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
// constant_range_expression ::=
//   constant_expression
// | msb_constant_expression : lsb_constant_expression
#[derive(Clone, Copy)]
pub enum ConstantRangeExpression<'a> {
    Single(AstId<'a, ConstantExpr<'a>>),
    MsbLsb {
        msb: AstId<'a, ConstantExpr<'a>>,
        lsb: AstId<'a, ConstantExpr<'a>>,
    },
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
// constant_mintypmax_expression ::=
//   constant_expression
// | constant_expression : constant_expression : constant_expression
#[derive(Clone, Copy)]
pub enum ConstantMinTypMaxExpression<'a> {
    Single(AstId<'a, ConstantExpr<'a>>),
    MinTypMax {
        min: AstId<'a, ConstantExpr<'a>>,
        typ: AstId<'a, ConstantExpr<'a>>,
        max: AstId<'a, ConstantExpr<'a>>,
    },
}
