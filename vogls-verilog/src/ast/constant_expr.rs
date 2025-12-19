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
pub struct ConstantExpr(pub Expr);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
// constant_range_expression ::=
//   constant_expression
// | msb_constant_expression : lsb_constant_expression
#[derive(Clone, Copy)]
pub enum ConstantRangeExpression {
    Single(AstId<ConstantExpr>),
    MsbLsb {
        msb: AstId<ConstantExpr>,
        lsb: AstId<ConstantExpr>,
    },
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
// constant_mintypmax_expression ::=
//   constant_expression
// | constant_expression : constant_expression : constant_expression
#[derive(Clone, Copy)]
pub enum ConstantMinTypMaxExpression {
    Single(AstId<ConstantExpr>),
    MinTypMax {
        min: AstId<ConstantExpr>,
        typ: AstId<ConstantExpr>,
        max: AstId<ConstantExpr>,
    },
}
