use crate::ast::expr::{BinaryOperator, BitPartSelect, Expr, UnaryOperator};
use crate::ast::{AstId, DecimalRef, Identifier, SizedNumberRef, StringRef};
use crate::parser::token_walker::TokenRange;
use crate::parser::{ItemParsable, ParseErrorReason};
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, Diagnostics, ParseErrorKind, Parser};

pub(crate) type BindingPower = u8;

pub(crate) enum StackItem {
    Paren,
    Brace(AstId<Expr>),
    Unary(UnaryOperator),
    Binary(BinaryOperator, AstId<Expr>),
    TernaryS1(AstId<Expr>),
    TernaryS2(AstId<Expr>, AstId<Expr>),
}

fn token_to_prefix_op(t: Token) -> Option<(u8, UnaryOperator)> {
    use Token as T;
    use UnaryOperator as U;
    match t {
        T::Bang => Some((26, U::LogicalNegation)),
        T::Tilde => Some((26, U::BitwiseNegation)),
        T::Ampersand => Some((26, U::ReductionAnd)),
        T::Bar => Some((26, U::ReductionOr)),
        T::TildeAmpersand => Some((26, U::ReductionNand)),
        T::TildeBar => Some((26, U::ReductionNor)),
        T::Caret => Some((26, U::ReductionXor)),
        T::TildeCaret | T::CaretTilde => Some((26, U::ReductionXnor)),
        T::Plus => Some((25, U::SignPlus)),
        T::Minus => Some((25, U::SignMinus)),
        _ => None,
    }
}

fn token_to_binary_op(t: Token) -> Option<(u8, u8, BinaryOperator)> {
    use BinaryOperator as B;
    use Token as T;
    match t {
        T::Star => Some((23, 24, B::Multiply)),
        T::Slash => Some((23, 24, B::Divide)),
        T::Procent => Some((23, 24, B::Modulus)),
        T::Plus => Some((21, 22, B::BinaryPlus)),
        T::Minus => Some((21, 22, B::BinaryMinus)),
        T::DoubleLessThan => Some((19, 20, B::ShiftLeft)),
        T::DoubleGreaterThan => Some((19, 20, B::ShiftRight)),
        T::GreaterThan => Some((17, 18, B::GreaterThan)),
        T::GreaterThanEquals => Some((17, 18, B::GreaterThanEqual)),
        T::LessThan => Some((17, 18, B::LessThan)),
        T::LessThanEquals => Some((17, 18, B::LessThanEqual)),
        T::DoubleEquals => Some((15, 16, B::LogicalEquality)),
        T::BangEquals => Some((15, 16, B::LogicalInequality)),
        T::TripleEquals => Some((13, 14, B::CaseEquality)),
        T::BangDoubleEquals => Some((13, 14, B::CaseInequality)),
        T::Ampersand => Some((11, 12, B::BitwiseAnd)),
        T::Caret => Some((9, 10, B::BitwiseXor)),
        T::TildeCaret | T::CaretTilde => Some((9, 10, B::BitwiseXnor)),
        T::Bar => Some((7, 8, B::BitwiseOr)),
        T::DoubleAmpersand => Some((5, 6, B::LogicalAnd)),
        T::DoubleBar => Some((3, 4, B::LogicalOr)),
        _ => None,
    }
}
impl<'a> Consumable<'a> for Expr {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        use Token as T;

        p.exprs_sp.clear();

        let mut min_bp: BindingPower = 0;
        let mut current: (Expr, TokenRange);

        let result = 'outer: loop {
            macro_rules! deepen {
                ($item:expr, $bp:expr, $span:expr) => {{
                    p.exprs_sp.push(($item, min_bp, $span));
                    min_bp = $bp;
                    continue 'outer;
                }};
            }

            let token = p.tkw.try_get(p.tkw.offset, diagnostics.as_deref_mut())?;
            let span = TokenRange {
                start: p.tkw.offset,
                end: p.tkw.offset + 1,
            };
            current = {
                match token.kind {
                    T::Ident => (
                        Expr::Ident(Identifier::item_parse(
                            p,
                            arenas,
                            diagnostics.as_deref_mut(),
                        )?),
                        span,
                    ),
                    T::Decimal => (
                        Expr::Decimal(
                            DecimalRef::item_parse(p, arenas, diagnostics.as_deref_mut())?.item,
                        ),
                        span,
                    ),
                    T::Number => (
                        Expr::Sized(SizedNumberRef::item_parse(
                            p,
                            arenas,
                            diagnostics.as_deref_mut(),
                        )?),
                        span,
                    ),
                    T::String => (
                        Expr::String(
                            StringRef::item_parse(p, arenas, diagnostics.as_deref_mut())?.item,
                        ),
                        span,
                    ),
                    T::LeftParen => {
                        p.tkw.offset += 1;
                        deepen!(StackItem::Paren, 0, span)
                    }
                    t => {
                        let t = *t;
                        p.tkw.next();
                        let (r_bp, op) = token_to_prefix_op(t).ok_or_else(|| {
                            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                                diagnostics
                                    .errors
                                    .push((span, ParseErrorReason::UnexpectedToken(t)));
                            }
                            ParseErrorKind::UnexpectedToken
                        })?;
                        deepen!(StackItem::Unary(op), r_bp, span);
                    }
                }
            };

            loop {
                loop {
                    let Some(peeked) = p.tkw.get(p.tkw.offset) else {
                        break;
                    };

                    // Bit/Part Select ( ... [ ... ] )
                    if *peeked.kind == T::LeftBrace {
                        p.tkw.next();
                        let span = current.1;
                        let subject = arenas.add_tuple(current);
                        deepen!(StackItem::Brace(subject), 0, span);
                    }

                    // Ternary operator ( ... ? ... : ... )
                    if *peeked.kind == T::QuestionMark {
                        let (l_bp, r_bp) = (1, 2);

                        if l_bp < min_bp {
                            break;
                        }

                        p.tkw.next();
                        let span = current.1;
                        let condition = arenas.add_tuple(current);
                        deepen!(StackItem::TernaryS1(condition), r_bp, span);
                    }

                    let Some((l_bp, r_bp, op)) = token_to_binary_op(*peeked.kind) else {
                        break;
                    };

                    if l_bp < min_bp {
                        break;
                    }

                    p.tkw.next();
                    let span = current.1;
                    let lhs = arenas.add_tuple(current);
                    deepen!(StackItem::Binary(op, lhs), r_bp, span);
                }

                let Some((item, bp, loc)) = p.exprs_sp.pop() else {
                    break 'outer current;
                };

                let location = TokenRange {
                    start: loc.start,
                    end: current.1.end,
                };

                match item {
                    StackItem::Paren => {
                        p.tkw
                            .next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                    }
                    StackItem::Brace(subject) => {
                        p.tkw
                            .next_expect(T::RightBrace, diagnostics.as_deref_mut())?;
                        let braced = arenas.add_tuple(current);
                        let bitpartselect = BitPartSelect { subject, braced };
                        current = (Expr::BitPartSelect(bitpartselect), location)
                    }
                    StackItem::Unary(op) => {
                        let subexpr = arenas.add_tuple(current);
                        current = (Expr::Unary(op, subexpr), location)
                    }
                    StackItem::Binary(op, lhs) => {
                        let rhs = arenas.add_tuple(current);
                        current = (Expr::Binary(op, lhs, rhs), location)
                    }
                    StackItem::TernaryS1(condition) => {
                        p.tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
                        let truthy = arenas.add_tuple(current);
                        deepen!(StackItem::TernaryS2(condition, truthy), bp, loc);
                    }
                    StackItem::TernaryS2(condition, truthy) => {
                        let falsy = arenas.add_tuple(current);
                        current = (Expr::Ternary(condition, truthy, falsy), location)
                    }
                }

                min_bp = bp;
            }
        };

        Ok(result.0)
    }
}
