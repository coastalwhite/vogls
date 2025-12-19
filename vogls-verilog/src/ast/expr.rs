use std::fmt;

use crate::parser::AstArenas;

use super::constant_expr::ConstantExpr;
use super::{AstId, AstIdRange, AstItem, DecimalRef, Identifier, SizedNumberRef, StringRef};

#[derive(Clone, Copy)]
pub struct BitPartSelect {
    pub(crate) subject: AstId<Expr>,
    pub(crate) braced: AstId<Expr>,
}

#[derive(Clone, Copy)]
pub enum BitSlice {
    MsbLsb(AstId<ConstantExpr>, AstId<ConstantExpr>),
    PlusWidth(AstId<Expr>, AstId<ConstantExpr>),
    MinusWidth(AstId<Expr>, AstId<ConstantExpr>),
}

#[derive(Clone, Copy)]
#[expect(unused)]
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

#[derive(Clone, Copy)]
pub enum Expr {
    BitPartSelect(BitPartSelect),
    BitSlice(AstId<Expr>, BitSlice),
    Unary(UnaryOperator, AstId<Expr>),
    Binary(BinaryOperator, AstId<Expr>, AstId<Expr>),
    Concatenation(AstIdRange<Expr>),
    Replication(Replication),
    Ternary(AstId<Expr>, AstId<Expr>, AstId<Expr>),
    Ident(AstItem<Identifier>),
    Decimal(DecimalRef),
    Sized(AstItem<SizedNumberRef>),
    String(StringRef),
}

impl Expr {
    pub fn into_str_literal(&self) -> Option<StringRef> {
        match self {
            Expr::String(s) => Some(*s),
            _ => None,
        }
    }

    pub fn into_decimal_literal(&self) -> Option<DecimalRef> {
        match self {
            Expr::Decimal(d) => Some(*d),
            _ => None,
        }
    }
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

impl AstId<Expr> {
    pub fn into_constant(self) -> AstId<ConstantExpr> {
        AstId {
            node: unsafe { self.node.transmute() },
            loc: self.loc,
        }
    }
}
impl AstId<ConstantExpr> {
    pub fn into_expr(self) -> AstId<Expr> {
        AstId {
            node: unsafe { self.node.transmute() },
            loc: self.loc,
        }
    }
}

impl Expr {
    pub fn tree_fmt(&self, arenas: &AstArenas, mut f: impl fmt::Write) -> fmt::Result {
        self.tree_fmt_impl(arenas, &mut f, 0)
    }

    fn tree_fmt_impl(
        &self,
        arenas: &AstArenas,
        f: &mut impl fmt::Write,
        depth: usize,
    ) -> fmt::Result {
        write!(f, "{:>0$}", depth * 2)?;
        match self {
            Expr::BitPartSelect(bps) => {
                writeln!(f, "bps")?;
                arenas
                    .get(bps.subject)
                    .tree_fmt_impl(arenas, f, depth + 1)?;
                arenas.get(bps.braced).tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::BitSlice(subject, _) => {
                writeln!(f, "bit_slice")?;
                arenas.get(*subject).tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Unary(op, child) => {
                writeln!(f, "unary [{}]", op.into_str())?;
                arenas.get(*child).tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Binary(op, lhs, rhs) => {
                writeln!(f, "binary [{}]", op.into_str())?;
                arenas.get(*lhs).tree_fmt_impl(arenas, f, depth + 1)?;
                arenas.get(*rhs).tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Concatenation(..) => todo!(),
            Expr::Replication(..) => todo!(),
            Expr::Ternary(condition, truthy, falsy) => {
                writeln!(f, "ternary")?;
                arenas.get(*condition).tree_fmt_impl(arenas, f, depth + 1)?;
                arenas.get(*truthy).tree_fmt_impl(arenas, f, depth + 1)?;
                arenas.get(*falsy).tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Ident(ident) => {
                writeln!(f, "ident: {}", arenas.get_ident(ident.item.0))?;
            }
            Expr::Decimal(decimal) => {
                writeln!(f, "decimal: {}", arenas.decimals[decimal.at])?;
            }
            Expr::Sized(..) => todo!(),
            Expr::String(..) => todo!(),
        }

        Ok(())
    }
}
