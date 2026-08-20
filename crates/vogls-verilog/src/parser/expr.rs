use vogls_ir::token_range::TokenRange;
use vogls_utils::VgHashSet;

use crate::arena::Arena;
use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{BinaryOperator, BitSlice, Expr, Replication, UnaryOperator};
use crate::ast::statement::SystemTaskIdentifier;
use crate::ast::{AstId, AstIdRange, AstItem, DecimalRef, HIdent, SizedNumberRef, StringRef};
use crate::parser::ParseErrorReason;
use crate::parser::utils::{item_parse, push, push_range};
use crate::tokenizer::Token;

use super::{AstArenas, Consumable, Diagnostics, ParserScratches, TokenWalker};

pub(crate) type BindingPower = u8;

pub(crate) enum StackItem<'a> {
    Paren,
    Bracket,
    Concatenation(Vec<Expr<'a>>, Vec<TokenRange>),
    Replication(AstId<'a, ConstantExpr<'a>>, Vec<Expr<'a>>, Vec<TokenRange>),
    Brace(HIdent<'a>, Vec<Expr<'a>>, Vec<TokenRange>),
    BraceS2(
        HIdent<'a>,
        AstIdRange<'a, Expr<'a>>,
        AstId<'a, Expr<'a>>,
        BraceVariant,
    ),
    SystemFnCall(
        AstItem<SystemTaskIdentifier>,
        Vec<Expr<'a>>,
        Vec<TokenRange>,
    ),
    FnCall(HIdent<'a>, Vec<Expr<'a>>, Vec<TokenRange>),
    Unary(UnaryOperator),
    Binary(BinaryOperator, AstId<'a, Expr<'a>>),
    TernaryS1(AstId<'a, Expr<'a>>),
    TernaryS2(AstId<'a, Expr<'a>>, AstId<'a, Expr<'a>>),
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
        T::Bang => Some((27, U::LogicalNegation)),
        T::Tilde => Some((27, U::BitwiseNegation)),
        T::Ampersand => Some((27, U::ReductionAnd)),
        T::Bar => Some((27, U::ReductionOr)),
        T::TildeAmpersand => Some((27, U::ReductionNand)),
        T::TildeBar => Some((27, U::ReductionNor)),
        T::Caret => Some((27, U::ReductionXor)),
        T::TildeCaret | T::CaretTilde => Some((27, U::ReductionXnor)),
        T::Plus => Some((27, U::SignPlus)),
        T::Minus => Some((27, U::SignMinus)),
        _ => None,
    }
}

fn token_to_binary_op(t: Token) -> Option<(u8, u8, BinaryOperator)> {
    use BinaryOperator as B;
    use Token as T;
    match t {
        T::DoubleStar => Some((25, 26, B::Power)),
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
        T::TripleLessThan => Some((17, 18, B::ArithmeticLeftShift)),
        T::TripleGreaterThan => Some((17, 18, B::ArithmeticRightShift)),
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

impl<'a> Consumable<'a> for Expr<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
                        let ident = HIdent::consume(
                            tkw,
                            // We cannot reuse the exprs_sp
                            &mut ParserScratches {
                                udps: VgHashSet::default(),
                                exprs_sp: Vec::new(),
                            },
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?;

                        if tkw.next_if_equals(T::LeftBrace) {
                            deepen!(StackItem::Brace(ident, Vec::new(), Vec::new()), 0, span)
                        } else if tkw.next_if_equals(T::LeftParen) {
                            deepen!(StackItem::FnCall(ident, Vec::new(), Vec::new()), 0, span)
                        } else {
                            (Expr::Ident(ident, AstIdRange::default(), None), span)
                        }
                    }
                    T::DollarIdent => {
                        let ident = item_parse::<SystemTaskIdentifier>(
                            tkw,
                            sc,
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?;

                        if tkw.next_if_equals(T::LeftParen) {
                            if tkw.next_if_equals(T::RightParen) {
                                (
                                    Expr::SystemFunctionCall(ident, Some(AstIdRange::default())),
                                    span,
                                )
                            } else {
                                deepen!(
                                    StackItem::SystemFnCall(ident, Vec::new(), Vec::new()),
                                    0,
                                    span
                                )
                            }
                        } else {
                            (Expr::SystemFunctionCall(ident, None), span)
                        }
                    }
                    T::Real => {
                        let content =
                            f64::consume(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
                        (Expr::Real(content), span)
                    }
                    T::Decimal => (
                        Expr::Decimal(
                            item_parse::<DecimalRef>(
                                tkw,
                                sc,
                                arenas,
                                ast,
                                diagnostics.as_deref_mut(),
                            )?
                            .item,
                        ),
                        span,
                    ),
                    T::Number => (
                        Expr::Sized(item_parse::<SizedNumberRef>(
                            tkw,
                            sc,
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?),
                        span,
                    ),
                    T::String => (
                        Expr::String(
                            item_parse::<StringRef>(
                                tkw,
                                sc,
                                arenas,
                                ast,
                                diagnostics.as_deref_mut(),
                            )?
                            .item,
                        ),
                        span,
                    ),
                    T::LeftBracket => {
                        tkw.offset += 1;
                        deepen!(StackItem::Bracket, 0, span)
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
                        })?;
                        deepen!(StackItem::Unary(op), r_bp, span);
                    }
                }
            };

            loop {
                'inner: {
                    let Some(peeked) = tkw.get(tkw.offset) else {
                        break 'inner;
                    };

                    // Ternary operator ( ... ? ... : ... )
                    if *peeked.kind == T::QuestionMark {
                        let (l_bp, r_bp) = (2, 1);

                        if l_bp < min_bp {
                            break 'inner;
                        }

                        tkw.offset += 1;
                        let span = current.1;
                        let condition = push(arenas, ast, current.0, current.1);
                        deepen!(StackItem::TernaryS1(condition), r_bp, span);
                    }

                    let Some((l_bp, r_bp, op)) = token_to_binary_op(*peeked.kind) else {
                        break 'inner;
                    };

                    if l_bp < min_bp {
                        break 'inner;
                    }

                    tkw.offset += 1;
                    let span = current.1;
                    let lhs = push(arenas, ast, current.0, current.1);
                    deepen!(StackItem::Binary(op, lhs), r_bp, span);
                }

                let Some((item, bp, loc)) = sc.exprs_sp.pop() else {
                    break 'outer current;
                };

                let location = TokenRange {
                    start: loc.start,
                    end: current.1.end,
                };

                min_bp = bp;
                match item {
                    StackItem::Paren => {
                        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                    }
                    StackItem::Bracket => match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                        T::RightBracket => {
                            let expr = push(arenas, ast, current.0, current.1);
                            let expr = AstIdRange::single(expr);
                            current = (Expr::Concatenation(expr), location);
                        }
                        T::Comma => {
                            deepen!(
                                StackItem::Concatenation(vec![current.0], vec![current.1]),
                                0,
                                loc
                            );
                        }
                        T::LeftBracket => {
                            let expr = push(arenas, ast, current.0, current.1);
                            let expr = expr.into_constant();
                            deepen!(StackItem::Replication(expr, Vec::new(), Vec::new()), 0, loc);
                        }
                        t => {
                            if let Some(d) = diagnostics {
                                d.unexpected_token(tkw.offset, t);
                            }
                            return Err(());
                        }
                    },
                    StackItem::Concatenation(mut exprs, mut trs) => {
                        exprs.push(current.0);
                        trs.push(current.1);
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightBracket => {
                                let exprs = push_range(arenas, ast, exprs, trs);
                                current = (Expr::Concatenation(exprs), location);
                            }
                            T::Comma => {
                                deepen!(StackItem::Concatenation(exprs, trs), 0, loc);
                            }
                            t => {
                                if let Some(d) = diagnostics {
                                    d.unexpected_token(tkw.offset, t);
                                }
                                return Err(());
                            }
                        }
                    }
                    StackItem::Replication(constant_expr, mut exprs, mut trs) => {
                        exprs.push(current.0);
                        trs.push(current.1);
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightBracket => {
                                tkw.next_expect(T::RightBracket, diagnostics.as_deref_mut())?;
                                let exprs = push_range(arenas, ast, exprs, trs);
                                current = (
                                    Expr::Replication(Replication {
                                        constant_expr,
                                        exprs,
                                    }),
                                    location,
                                );
                            }
                            T::Comma => {
                                deepen!(StackItem::Replication(constant_expr, exprs, trs), 0, loc);
                            }
                            t => {
                                if let Some(d) = diagnostics {
                                    d.unexpected_token(tkw.offset, t);
                                }
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
                                    let braced =
                                        push_range(arenas, ast, current_braced, current_trs);
                                    current = (Expr::Ident(ident, braced, None), location)
                                }
                            }
                            T::Colon => {
                                let exprs = push_range(arenas, ast, current_braced, current_trs);
                                let braced = push(arenas, ast, current.0, current.1);
                                deepen!(
                                    StackItem::BraceS2(ident, exprs, braced, BraceVariant::MsbLsb),
                                    0,
                                    loc
                                );
                            }
                            T::PlusColon => {
                                let exprs = push_range(arenas, ast, current_braced, current_trs);
                                let braced = push(arenas, ast, current.0, current.1);
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
                                let exprs = push_range(arenas, ast, current_braced, current_trs);
                                let braced = push(arenas, ast, current.0, current.1);
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
                                if let Some(d) = diagnostics {
                                    d.unexpected_token(tkw.offset - 1, t);
                                }
                                return Err(());
                            }
                        }
                    }
                    StackItem::BraceS2(subject, exprs, lhs, variant) => {
                        let rhs = push(arenas, ast, current.0, current.1);
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
                    StackItem::SystemFnCall(ident, mut params, mut trs) => {
                        params.push(current.0);
                        trs.push(current.1);
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightParen => {
                                let params = push_range(arenas, ast, params, trs);
                                current = (Expr::SystemFunctionCall(ident, Some(params)), location);
                            }
                            T::Comma => {
                                deepen!(StackItem::SystemFnCall(ident, params, trs), 0, location)
                            }
                            t => {
                                if let Some(d) = diagnostics {
                                    d.unexpected_token(tkw.offset - 1, t);
                                }
                                return Err(());
                            }
                        }
                    }
                    StackItem::FnCall(ident, mut params, mut trs) => {
                        params.push(current.0);
                        trs.push(current.1);
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightParen => {
                                let params = push_range(arenas, ast, params, trs);
                                current = (Expr::FunctionCall(ident, params), location);
                            }
                            T::Comma => {
                                deepen!(StackItem::FnCall(ident, params, trs), 0, location)
                            }
                            t => {
                                if let Some(d) = diagnostics {
                                    d.unexpected_token(tkw.offset - 1, t);
                                }
                                return Err(());
                            }
                        }
                    }
                    StackItem::Unary(op) => {
                        let subexpr = push(arenas, ast, current.0, current.1);
                        current = (Expr::Unary(op, subexpr), location)
                    }
                    StackItem::Binary(op, lhs) => {
                        let rhs = push(arenas, ast, current.0, current.1);
                        current = (Expr::Binary(op, lhs, rhs), location)
                    }
                    StackItem::TernaryS1(condition) => {
                        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
                        let truthy = push(arenas, ast, current.0, current.1);
                        deepen!(StackItem::TernaryS2(condition, truthy), bp, loc);
                    }
                    StackItem::TernaryS2(condition, truthy) => {
                        let falsy = push(arenas, ast, current.0, current.1);
                        current = (Expr::Ternary(condition, truthy, falsy), location)
                    }
                }
            }
        };
        Ok(result.0)
    }
}
