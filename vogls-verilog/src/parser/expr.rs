use crate::ast::expr::{BinaryOperator, BitSlice, Expr, UnaryOperator};
use crate::ast::{AstId, AstIdRange, AstItem, DecimalRef, Identifier, SizedNumberRef, StringRef};
use crate::parser::ParseErrorReason;
use crate::parser::token_walker::TokenRange;
use crate::parser::utils::item_parse;
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, Diagnostics, ParserScratches, TokenWalker};

pub(crate) type BindingPower = u8;

pub(crate) enum StackItem {
    Paren,
    Bracket(Vec<Expr>, Vec<TokenRange>),
    Brace(AstItem<Identifier>, Vec<Expr>, Vec<TokenRange>),
    BraceS2(
        AstItem<Identifier>,
        AstIdRange<Expr>,
        AstId<Expr>,
        BraceVariant,
    ),
    Unary(UnaryOperator),
    Binary(BinaryOperator, AstId<Expr>),
    TernaryS1(AstId<Expr>),
    TernaryS2(AstId<Expr>, AstId<Expr>),
}

pub(crate) enum BraceVariant {
    MsbLsb,
    BasePlus,
    BaseMinus,
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
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        sc.exprs_sp.clear();

        let mut min_bp: BindingPower = 0;
        let mut current: (Expr, TokenRange);

        let result = 'outer: loop {
            macro_rules! deepen {
                ($item:expr, $bp:expr, $span:expr) => {{
                    sc.exprs_sp.push(($item, min_bp, $span));
                    min_bp = $bp;
                    continue 'outer;
                }};
            }

            let token = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
            let span = TokenRange {
                start: tkw.offset,
                end: tkw.offset + 1,
            };
            current = {
                match token.kind {
                    T::Ident => {
                        let ident =
                            item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

                        if tkw.next_if_equals(T::LeftBrace) {
                            deepen!(StackItem::Brace(ident, Vec::new(), Vec::new()), 0, span)
                        } else {
                            (Expr::Ident(ident, AstIdRange::default(), None), span)
                        }
                    }
                    T::Decimal => (
                        Expr::Decimal(
                            item_parse::<DecimalRef>(tkw, sc, arenas, diagnostics.as_deref_mut())?
                                .item,
                        ),
                        span,
                    ),
                    T::Number => (
                        Expr::Sized(item_parse::<SizedNumberRef>(
                            tkw,
                            sc,
                            arenas,
                            diagnostics.as_deref_mut(),
                        )?),
                        span,
                    ),
                    T::String => (
                        Expr::String(
                            item_parse::<StringRef>(tkw, sc, arenas, diagnostics.as_deref_mut())?
                                .item,
                        ),
                        span,
                    ),
                    T::LeftBracket => {
                        tkw.offset += 1;
                        deepen!(StackItem::Bracket(Vec::new(), Vec::new()), 0, span)
                    }
                    T::LeftParen => {
                        tkw.offset += 1;
                        deepen!(StackItem::Paren, 0, span)
                    }
                    t => {
                        let t = *t;
                        tkw.next();
                        let (r_bp, op) = token_to_prefix_op(t).ok_or_else(|| {
                            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                                diagnostics
                                    .errors
                                    .push((span, ParseErrorReason::UnexpectedToken(t)));
                            }
                            ()
                        })?;
                        deepen!(StackItem::Unary(op), r_bp, span);
                    }
                }
            };

            loop {
                loop {
                    let Some(peeked) = tkw.get(tkw.offset) else {
                        break;
                    };

                    // Ternary operator ( ... ? ... : ... )
                    if *peeked.kind == T::QuestionMark {
                        let (l_bp, r_bp) = (1, 2);

                        if l_bp < min_bp {
                            break;
                        }

                        tkw.next();
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

                    tkw.next();
                    let span = current.1;
                    let lhs = arenas.add_tuple(current);
                    deepen!(StackItem::Binary(op, lhs), r_bp, span);
                }

                let Some((item, bp, loc)) = sc.exprs_sp.pop() else {
                    break 'outer current;
                };

                let location = TokenRange {
                    start: loc.start,
                    end: current.1.end,
                };

                match item {
                    StackItem::Paren => {
                        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                    }
                    StackItem::Bracket(mut exprs, mut ranges) => {
                        exprs.push(current.0);
                        ranges.push(current.1);
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightBracket => {
                                current = (
                                    Expr::Concatenation(arenas.add_range(exprs, ranges)),
                                    location,
                                );
                            }
                            T::Comma => {
                                deepen!(StackItem::Bracket(exprs, ranges), 0, loc);
                            }
                            t => {
                                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                                return Err(());
                            }
                        }
                    }
                    StackItem::Brace(ident, mut current_braced, mut current_trs) => {
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightBrace => {
                                current_braced.push(current.0);
                                current_trs.push(current.1);
                                if tkw.next_if_equals(T::LeftBrace) {
                                    deepen!(
                                        StackItem::Brace(ident, current_braced, current_trs),
                                        0,
                                        loc
                                    );
                                } else {
                                    let braced = arenas.add_range(current_braced, current_trs);
                                    current = (Expr::Ident(ident, braced, None), location)
                                }
                            }
                            T::Colon => {
                                let exprs = arenas.add_range(current_braced, current_trs);
                                let braced = arenas.add_tuple(current);
                                deepen!(
                                    StackItem::BraceS2(ident, exprs, braced, BraceVariant::MsbLsb),
                                    0,
                                    loc
                                );
                            }
                            T::PlusColon => {
                                let exprs = arenas.add_range(current_braced, current_trs);
                                let braced = arenas.add_tuple(current);
                                deepen!(
                                    StackItem::BraceS2(
                                        ident,
                                        exprs,
                                        braced,
                                        BraceVariant::BasePlus
                                    ),
                                    0,
                                    loc
                                );
                            }
                            T::MinusColon => {
                                let exprs = arenas.add_range(current_braced, current_trs);
                                let braced = arenas.add_tuple(current);
                                deepen!(
                                    StackItem::BraceS2(
                                        ident,
                                        exprs,
                                        braced,
                                        BraceVariant::BaseMinus
                                    ),
                                    0,
                                    loc
                                );
                            }
                            t => {
                                diagnostics.map(|d| d.unexpected_token(tkw.offset - 1, t));
                                return Err(());
                            }
                        }
                    }
                    StackItem::BraceS2(subject, exprs, lhs, variant) => {
                        let rhs = arenas.add_tuple(current);
                        let bit_slice = match variant {
                            BraceVariant::MsbLsb => {
                                BitSlice::MsbLsb(lhs.into_constant(), rhs.into_constant())
                            }
                            BraceVariant::BasePlus => BitSlice::PlusWidth(lhs, rhs.into_constant()),
                            BraceVariant::BaseMinus => {
                                BitSlice::MinusWidth(lhs, rhs.into_constant())
                            }
                        };
                        tkw.next_expect(T::RightBrace, diagnostics.as_deref_mut())?;
                        current = (Expr::Ident(subject, exprs, Some(bit_slice)), location);
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
                        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
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
