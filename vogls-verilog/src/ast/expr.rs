use std::fmt;

use crate::parser::AstArenas;

use super::constant_expr::ConstantExpr;
use super::statement::SystemTaskIdentifier;
use super::{AstId, AstIdRange, AstItem, DecimalRef, Identifier, SizedNumberRef, StringRef};

#[derive(Clone, Copy)]
pub enum BitSlice {
    MsbLsb(AstId<ConstantExpr>, AstId<ConstantExpr>),
    PlusWidth(AstId<Expr>, AstId<ConstantExpr>),
    MinusWidth(AstId<Expr>, AstId<ConstantExpr>),
}

#[derive(Clone, Copy)]
pub struct Replication {
    pub constant_expr: AstId<ConstantExpr>,
    pub exprs: AstIdRange<Expr>,
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
    Power,
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
    ArithmeticLeftShift,
    ArithmeticRightShift,
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
    Unary(UnaryOperator, AstId<Expr>),
    Binary(BinaryOperator, AstId<Expr>, AstId<Expr>),
    Concatenation(AstIdRange<Expr>),
    Replication(Replication),
    Ternary(AstId<Expr>, AstId<Expr>, AstId<Expr>),
    Ident(AstItem<Identifier>, AstIdRange<Expr>, Option<BitSlice>),
    FunctionCall(AstItem<Identifier>, AstIdRange<Expr>),
    SystemFunctionCall(AstItem<SystemTaskIdentifier>, Option<AstIdRange<Expr>>),
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
            B::Power => "**",
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
            B::ArithmeticLeftShift => "<<<",
            B::ArithmeticRightShift => ">>>",
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
        let indent = depth * 2;
        write!(f, "{:indent$}", " ")?;
        match self {
            Expr::Unary(op, child) => {
                writeln!(f, "unary [{}]", op.into_str())?;
                arenas.get(*child).tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Binary(op, lhs, rhs) => {
                writeln!(f, "binary [{}]", op.into_str())?;
                arenas.get(*lhs).tree_fmt_impl(arenas, f, depth + 1)?;
                arenas.get(*rhs).tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Concatenation(..) => f.write_str("concatenation")?,
            Expr::Replication(Replication {
                constant_expr,
                exprs,
            }) => {
                writeln!(f, "replication")?;
                arenas
                    .get(constant_expr.into_expr())
                    .tree_fmt_impl(arenas, f, depth + 1)?;
                for e in exprs.iter() {
                    arenas.get(e).tree_fmt_impl(arenas, f, depth + 1)?;
                }
            }
            Expr::Ternary(condition, truthy, falsy) => {
                writeln!(f, "ternary")?;
                arenas.get(*condition).tree_fmt_impl(arenas, f, depth + 1)?;
                arenas.get(*truthy).tree_fmt_impl(arenas, f, depth + 1)?;
                arenas.get(*falsy).tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Ident(ident, expr, range_expr) => {
                writeln!(f, "ident: {}", &arenas.ident_table[ident.item.0])?;
                for e in expr.iter() {
                    arenas.get(e).tree_fmt_impl(arenas, f, depth + 1)?;
                }
                if let Some(range_expr) = range_expr {
                    match range_expr {
                        BitSlice::MsbLsb(msb, lsb) => {
                            arenas
                                .get(msb.into_expr())
                                .tree_fmt_impl(arenas, f, depth + 1)?;
                            arenas
                                .get(lsb.into_expr())
                                .tree_fmt_impl(arenas, f, depth + 1)?;
                        }
                        BitSlice::PlusWidth(base, width) => {
                            arenas.get(*base).tree_fmt_impl(arenas, f, depth + 1)?;
                            arenas
                                .get(width.into_expr())
                                .tree_fmt_impl(arenas, f, depth + 1)?;
                        }
                        BitSlice::MinusWidth(base, width) => {
                            arenas.get(*base).tree_fmt_impl(arenas, f, depth + 1)?;
                            arenas
                                .get(width.into_expr())
                                .tree_fmt_impl(arenas, f, depth + 1)?;
                        }
                    }
                }
            }
            Expr::FunctionCall(ident, exprs) => {
                writeln!(f, "fn call: {}", &arenas.ident_table[ident.item.0])?;
                for e in exprs.iter() {
                    arenas.get(e).tree_fmt_impl(arenas, f, depth + 1)?;
                }
            }
            Expr::SystemFunctionCall(ident, exprs) => {
                writeln!(f, "system fn call: {}", &arenas.ident_table[ident.item.0])?;
                if let Some(exprs) = exprs {
                    for e in exprs.iter() {
                        arenas.get(e).tree_fmt_impl(arenas, f, depth + 1)?;
                    }
                }
            }
            Expr::Decimal(decimal) => {
                writeln!(f, "decimal: {}", arenas.decimals[decimal.at])?;
            }
            Expr::Sized(..) => f.write_str("sized")?,
            Expr::String(..) => f.write_str("string")?,
        }

        Ok(())
    }
}
