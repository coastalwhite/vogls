use crate::ident::Ident;
use crate::number::{Decimal, SizedNumber};

use super::AstId;

#[derive(Clone)]
pub struct BitPartSelect {
    pub(crate) subject: AstId<Expr>,
    pub(crate) braced: AstId<Expr>,
}

#[derive(Clone)]
pub struct Replication {
    pub(crate) subject: AstId<Expr>,
    pub(crate) repeats: AstId<Expr>,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOperator {
    LogicalNegation,
    BitwiseNegation,
    ReductionAnd,
    ReductionOr,
    ReductionNand,
    ReductionNor,
    ReductionXor,
    ReductionXnor,
    SignPlus,
    SignMinus,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOperator {
    Multiply,
    Divide,
    Modulus,
    BinaryPlus,
    BinaryMinus,
    ShiftLeft,
    ShiftRight,
    GreaterThan,
    GreaterThanEqual,
    LessThan,
    LessThanEqual,
    LogicalEquality,
    LogicalInequality,
    CaseEquality,
    CaseInequality,
    BitwiseAnd,
    BitwiseXor,
    BitwiseXnor,
    BitwiseOr,
    LogicalAnd,
    LogicalOr,
}

#[derive(Clone)]
pub enum Expr {
    BitPartSelect(BitPartSelect),
    Unary(UnaryOperator, AstId<Expr>),
    Binary(BinaryOperator, AstId<Expr>, AstId<Expr>),
    Concatenation(Vec<AstId<Expr>>),
    Replication(Replication),
    Ternary(AstId<Expr>, AstId<Expr>, AstId<Expr>),
    Ident(Ident),
    Decimal(Decimal),
    Sized(SizedNumber),
}

impl UnaryOperator {
    fn into_str(self) -> &'static str {
        use UnaryOperator as U;
        match self {
            U::LogicalNegation => "!",
            U::BitwiseNegation => "~",
            U::ReductionAnd => "&",
            U::ReductionOr => "|",
            U::ReductionNand => "~&",
            U::ReductionNor => "~|",
            U::ReductionXor => "^",
            U::ReductionXnor => "~^",
            U::SignPlus => "+",
            U::SignMinus => "-",
        }
    }
}

impl BinaryOperator {
    fn into_str(self) -> &'static str {
        use BinaryOperator as B;
        match self {
            B::Multiply => "*",
            B::Divide => "/",
            B::Modulus => "%",
            B::BinaryPlus => "+",
            B::BinaryMinus => "-",
            B::ShiftLeft => "<<",
            B::ShiftRight => ">>",
            B::GreaterThan => ">",
            B::GreaterThanEqual => ">=",
            B::LessThan => "<",
            B::LessThanEqual => "<=",
            B::LogicalEquality => "==",
            B::LogicalInequality => "!=",
            B::CaseEquality => "===",
            B::CaseInequality => "!==",
            B::BitwiseAnd => "&",
            B::BitwiseXor => "^",
            B::BitwiseXnor => "~^",
            B::BitwiseOr => "|",
            B::LogicalAnd => "&&",
            B::LogicalOr => "||",
        }
    }
}
