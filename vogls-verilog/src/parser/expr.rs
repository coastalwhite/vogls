use crate::ast::expr::{BinaryOperator, BitPartSelect, Expr, UnaryOperator};
use crate::ast::{AstId, DecimalRef, Identifier, SizedNumberRef, StringRef, TextRef};
use crate::lexer::{FromLexerError, Token, TokenContent, TokenKind};
use crate::parser::ItemParsable;
use crate::span::Span;

use super::{AstArenas, Consumable, Parsable, ParseError, Parser};

pub(crate) type BindingPower = u8;

pub(crate) enum StackItem {
    Paren,
    Brace(AstId<Expr>),
    Unary(UnaryOperator),
    Binary(BinaryOperator, AstId<Expr>),
    TernaryS1(AstId<Expr>),
    TernaryS2(AstId<Expr>, AstId<Expr>),
}

fn token_to_prefix_op(t: TokenKind) -> Option<(u8, UnaryOperator)> {
    use TokenKind as T;
    use UnaryOperator as U;
    match t {
        T::Exclamation => Some((26, U::LogicalNegation)),
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

fn token_to_binary_op(t: TokenKind) -> Option<(u8, u8, BinaryOperator)> {
    use BinaryOperator as B;
    use TokenKind as T;
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
        T::ExclamationEquals => Some((15, 16, B::LogicalInequality)),
        T::TripleEquals => Some((13, 14, B::CaseEquality)),
        T::ExclamationDoubleEquals => Some((13, 14, B::CaseInequality)),
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
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        use ParseError as E;
        use TokenContent as T;

        p.exprs_sp.clear();

        let mut min_bp: BindingPower = 0;
        let mut current: (Expr, Span);

        let result = 'outer: loop {
            macro_rules! deepen {
                ($item:expr, $bp:expr, $span:expr) => {{
                    p.exprs_sp.push(($item, min_bp, $span));
                    min_bp = $bp;
                    continue 'outer;
                }};
            }

            let (t, span) = p.lexer().next().ok_or(E::MissingToken)?.take();
            current = {
                match t {
                    T::Ident(i) => (
                        Expr::Ident(Identifier::ast_from_item(i, span, arenas)?),
                        span,
                    ),
                    T::Decimal(d) => (
                        Expr::Decimal(DecimalRef::ast_from_item(d, span, arenas)?),
                        span,
                    ),
                    T::Number(n) => (
                        Expr::Sized(SizedNumberRef::ast_from_item(n, span, arenas)?),
                        span,
                    ),
                    T::String(s) => (
                        Expr::String(StringRef::from_item(s, arenas)?),
                        span,
                    ),
                    T::LeftParen => deepen!(StackItem::Paren, 0, span),
                    t => {
                        let (r_bp, op) = token_to_prefix_op(t.kind())
                            .ok_or(E::unexpected_token(Token::new(t, span)))?;
                        deepen!(StackItem::Unary(op), r_bp, span);
                    }
                }
            };

            loop {
                loop {
                    let Some(peeked) = p.lexer.peek() else {
                        break;
                    };

                    // Bit/Part Select ( ... [ ... ] )
                    if matches!(peeked.kind(), TokenKind::LeftBrace) {
                        peeked.commit();
                        let span = current.1;
                        let subject = arenas.add_tuple(current);
                        deepen!(StackItem::Brace(subject), 0, span);
                    }

                    // Ternary operator ( ... ? ... : ... )
                    if matches!(peeked.kind(), TokenKind::QuestionMark) {
                        let (l_bp, r_bp) = (1, 2);

                        if l_bp < min_bp {
                            peeked.release();
                            break;
                        }

                        peeked.commit();
                        let span = current.1;
                        let condition = arenas.add_tuple(current);
                        deepen!(StackItem::TernaryS1(condition), r_bp, span);
                    }

                    let Some((l_bp, r_bp, op)) = token_to_binary_op(peeked.kind()) else {
                        peeked.release();
                        break;
                    };

                    if l_bp < min_bp {
                        peeked.release();
                        break;
                    }

                    peeked.commit();
                    let span = current.1;
                    let lhs = arenas.add_tuple(current);
                    deepen!(StackItem::Binary(op, lhs), r_bp, span);
                }

                let Some((item, bp, loc)) = p.exprs_sp.pop() else {
                    break 'outer current;
                };

                let location = loc | current.1;

                match item {
                    StackItem::Paren => {
                        p.lexer.expect(TokenKind::RightParen)?;
                    }
                    StackItem::Brace(subject) => {
                        p.lexer.expect(TokenKind::RightBrace)?;
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
                        p.lexer.expect(TokenKind::Colon)?;
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

        Ok(result)
    }
}
impl<'a> Parsable<'a> for Expr {}
