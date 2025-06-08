use super::{AstId, DecimalRef, StringRef};

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 504
// constant_expression ::=
//   constant_primary
// | unary_operator { attribute_instance } constant_primary
// | constant_expression binary_operator { attribute_instance } constant_expression
// | constant_expression ? { attribute_instance } constant_expression : constant_expression
#[derive(Clone, Copy)]
pub enum ConstantExpr {
    Primary(ConstantPrimary),
    // @Incomplete
}

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
#[derive(Clone, Copy)]
pub enum ConstantPrimary {
    Number(DecimalRef),
    // @Incomplete
    String(StringRef),
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
