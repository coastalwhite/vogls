use std::fmt;

use vogls_ir::{SCALAR_VSIZE, VectorSize};

use crate::parser::AstArenas;

use super::constant_expr::ConstantExpr;
use super::specify::ModulePathExpr;
use super::statement::SystemTaskIdentifier;
use super::{AstId, AstIdRange, AstItem, DecimalRef, HIdent, SizedNumberRef, StringRef};

#[derive(Clone, Copy)]
pub enum BitSlice<'a> {
    MsbLsb(AstId<'a, ConstantExpr<'a>>, AstId<'a, ConstantExpr<'a>>),
    PlusWidth(AstId<'a, Expr<'a>>, AstId<'a, ConstantExpr<'a>>),
    MinusWidth(AstId<'a, Expr<'a>>, AstId<'a, ConstantExpr<'a>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BitSliceKind {
    MsbLsb,
    PlusWidth,
    MinusWidth,
}

#[derive(Clone, Copy)]
pub struct Replication<'a> {
    pub constant_expr: AstId<'a, ConstantExpr<'a>>,
    pub exprs: AstIdRange<'a, Expr<'a>>,
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

impl UnaryOperator {
    pub fn is_self_determined(self) -> bool {
        use UnaryOperator as O;
        match self {
            O::LogicalNegation
            | O::ReductionAnd
            | O::ReductionOr
            | O::ReductionNand
            | O::ReductionNor
            | O::ReductionXor
            | O::ReductionXnor => true,
            O::BitwiseNegation | O::SignPlus | O::SignMinus => false,
        }
    }
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

impl BinaryOperator {
    pub fn is_self_determined(self) -> (bool, bool) {
        use BinaryOperator as O;
        match self {
            O::Power
            | O::Multiply
            | O::Divide
            | O::Modulus
            | O::BinaryPlus
            | O::BinaryMinus
            | O::GreaterThan
            | O::GreaterThanEqual
            | O::LessThan
            | O::LessThanEqual
            | O::LogicalEquality
            | O::LogicalInequality
            | O::CaseEquality
            | O::CaseInequality
            | O::BitwiseAnd
            | O::BitwiseXor
            | O::BitwiseXnor
            | O::BitwiseOr => (false, false),
            O::ArithmeticLeftShift | O::ArithmeticRightShift | O::ShiftLeft | O::ShiftRight => {
                (false, true)
            }
            O::LogicalAnd | O::LogicalOr => (true, true),
        }
    }

    pub fn output_width(self, lhs: VectorSize, rhs: VectorSize) -> VectorSize {
        use BinaryOperator as O;
        match self {
            O::GreaterThan
            | O::GreaterThanEqual
            | O::LessThan
            | O::LessThanEqual
            | O::LogicalEquality
            | O::LogicalInequality
            | O::CaseEquality
            | O::CaseInequality
            | O::LogicalAnd
            | O::LogicalOr => SCALAR_VSIZE,

            O::ShiftLeft | O::ShiftRight | O::ArithmeticLeftShift | O::ArithmeticRightShift => lhs,

            O::Power
            | O::Multiply
            | O::Divide
            | O::Modulus
            | O::BinaryPlus
            | O::BinaryMinus
            | O::BitwiseAnd
            | O::BitwiseXor
            | O::BitwiseXnor
            | O::BitwiseOr => lhs.max(rhs),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Expr<'a> {
    Unary(UnaryOperator, AstId<'a, Expr<'a>>),
    Binary(BinaryOperator, AstId<'a, Expr<'a>>, AstId<'a, Expr<'a>>),
    Concatenation(AstIdRange<'a, Expr<'a>>),
    Replication(Replication<'a>),
    Ternary(
        AstId<'a, Expr<'a>>,
        AstId<'a, Expr<'a>>,
        AstId<'a, Expr<'a>>,
    ),
    Ident(HIdent<'a>, AstIdRange<'a, Expr<'a>>, Option<BitSlice<'a>>),
    FunctionCall(HIdent<'a>, AstIdRange<'a, Expr<'a>>),
    SystemFunctionCall(
        AstItem<SystemTaskIdentifier>,
        Option<AstIdRange<'a, Expr<'a>>>,
    ),
    Decimal(DecimalRef),
    Sized(AstItem<SizedNumberRef>),
    String(StringRef),
}

impl<'a> Expr<'a> {
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

    pub fn output_size(self, width: VectorSize) -> VectorSize {
        match self {
            UnaryOperator::LogicalNegation
            | UnaryOperator::ReductionAnd
            | UnaryOperator::ReductionOr
            | UnaryOperator::ReductionNand
            | UnaryOperator::ReductionNor
            | UnaryOperator::ReductionXor
            | UnaryOperator::ReductionXnor => SCALAR_VSIZE,
            UnaryOperator::BitwiseNegation | UnaryOperator::SignPlus | UnaryOperator::SignMinus => {
                width
            }
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

impl<'a> BitSlice<'a> {
    pub fn exprs(&self) -> [AstId<'a, Expr<'a>>; 2] {
        match self {
            BitSlice::MsbLsb(msb, lsb) => [msb.into_expr(), lsb.into_expr()],
            BitSlice::PlusWidth(offset, width) | BitSlice::MinusWidth(offset, width) => {
                [*offset, width.into_expr()]
            }
        }
    }
}

impl<'a> AstId<'a, Expr<'a>> {
    pub fn into_constant(self) -> AstId<'a, ConstantExpr<'a>> {
        AstId {
            node: unsafe { std::mem::transmute(self.node) },
            loc: self.loc,
        }
    }
    pub fn into_module_path_expr(self) -> AstId<'a, ModulePathExpr<'a>> {
        AstId {
            node: unsafe { std::mem::transmute(self.node) },
            loc: self.loc,
        }
    }
}
impl<'a> AstId<'a, ConstantExpr<'a>> {
    pub fn into_expr(self) -> AstId<'a, Expr<'a>> {
        AstId {
            node: unsafe { std::mem::transmute(self.node) },
            loc: self.loc,
        }
    }
}
impl<'a> AstId<'a, ModulePathExpr<'a>> {
    pub fn into_expr(self) -> AstId<'a, Expr<'a>> {
        AstId {
            node: unsafe { std::mem::transmute(self.node) },
            loc: self.loc,
        }
    }
}

impl<'a> BitSlice<'a> {
    pub fn kind(&self) -> BitSliceKind {
        match self {
            Self::MsbLsb(..) => BitSliceKind::MsbLsb,
            Self::PlusWidth(..) => BitSliceKind::PlusWidth,
            Self::MinusWidth(..) => BitSliceKind::MinusWidth,
        }
    }
}

impl<'a> Expr<'a> {
    pub fn tree_display<'b>(&'b self, arenas: &'b AstArenas) -> impl fmt::Display + 'b {
        struct X<'a>(&'a Expr<'a>, &'a AstArenas);
        impl<'a> fmt::Display for X<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.tree_fmt(self.1, f)
            }
        }
        X(self, arenas)
    }
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
                child.tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Binary(op, lhs, rhs) => {
                writeln!(f, "binary [{}]", op.into_str())?;
                lhs.tree_fmt_impl(arenas, f, depth + 1)?;
                rhs.tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Concatenation(exprs) => {
                writeln!(f, "concatenation")?;
                for e in exprs.iter() {
                    e.tree_fmt_impl(arenas, f, depth + 1)?;
                }
            }
            Expr::Replication(Replication {
                constant_expr,
                exprs,
            }) => {
                writeln!(f, "replication")?;
                constant_expr
                    .into_expr()
                    .tree_fmt_impl(arenas, f, depth + 1)?;
                for e in exprs.iter() {
                    e.tree_fmt_impl(arenas, f, depth + 1)?;
                }
            }
            Expr::Ternary(condition, truthy, falsy) => {
                writeln!(f, "ternary")?;
                condition.tree_fmt_impl(arenas, f, depth + 1)?;
                truthy.tree_fmt_impl(arenas, f, depth + 1)?;
                falsy.tree_fmt_impl(arenas, f, depth + 1)?;
            }
            Expr::Ident(ident, expr, range_expr) => {
                writeln!(f, "ident: {}", &arenas.ident_table[ident.ident.item.0])?;
                for e in expr.iter() {
                    e.tree_fmt_impl(arenas, f, depth + 1)?;
                }
                if let Some(range_expr) = range_expr {
                    match range_expr {
                        BitSlice::MsbLsb(msb, lsb) => {
                            msb.into_expr().tree_fmt_impl(arenas, f, depth + 1)?;
                            lsb.into_expr().tree_fmt_impl(arenas, f, depth + 1)?;
                        }
                        BitSlice::PlusWidth(base, width) => {
                            base.tree_fmt_impl(arenas, f, depth + 1)?;
                            width.into_expr().tree_fmt_impl(arenas, f, depth + 1)?;
                        }
                        BitSlice::MinusWidth(base, width) => {
                            base.tree_fmt_impl(arenas, f, depth + 1)?;
                            width.into_expr().tree_fmt_impl(arenas, f, depth + 1)?;
                        }
                    }
                }
            }
            Expr::FunctionCall(ident, exprs) => {
                writeln!(f, "fn call: {}", &arenas.ident_table[ident.ident.item.0])?;
                for e in exprs.iter() {
                    e.tree_fmt_impl(arenas, f, depth + 1)?;
                }
            }
            Expr::SystemFunctionCall(ident, exprs) => {
                writeln!(f, "system fn call: {}", &arenas.ident_table[ident.item.0])?;
                if let Some(exprs) = exprs {
                    for e in exprs.iter() {
                        e.tree_fmt_impl(arenas, f, depth + 1)?;
                    }
                }
            }
            Expr::Decimal(decimal) => {
                writeln!(f, "decimal: {}", arenas.decimals[decimal.at])?;
            }
            Expr::Sized(..) => writeln!(f, "sized")?,
            Expr::String(..) => writeln!(f, "string")?,
        }

        Ok(())
    }
}
