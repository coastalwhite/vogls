use std::path::PathBuf;

use crate::arena::Arena;
use crate::ast::module::Module;
use crate::ast::{AstId, AstIdRange, AstItem, DecimalRef, IdentRef, SizedNumberRef};
use crate::lexer::{FromLexerError, Lexer, Token, TokenContent, TokenKind};
use crate::number::{Decimal, SizedNumber};
use crate::span::Span;

mod expr;
mod module;
mod statement;
// mod net;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    /// A `scratchpad` to parse expressions
    exprs_sp: Vec<(expr::StackItem, expr::BindingPower, Span)>,
}

#[derive(Default)]
pub struct AstArenas {
    pub nodes: Arena,
    pub spans: Vec<Span>,

    pub idents: String,
    pub decimals: Vec<Decimal>,
    pub sized_numbers: Vec<SizedNumber>,
}
impl AstArenas {
    fn add<T: Copy>(&mut self, item: T, span: Span) -> AstId<T> {
        let loc = self.spans.len();
        self.spans.push(span);
        AstId {
            node: self.nodes.add(item),
            loc,
        }
    }

    fn add_tuple<T: Copy>(&mut self, (item, span): (T, Span)) -> AstId<T> {
        self.add(item, span)
    }

    fn add_range<T: Copy>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        spans: impl IntoIterator<Item = Span>,
    ) -> AstIdRange<T> {
        let loc = self.spans.len();
        self.spans.extend(spans);
        AstIdRange {
            node: self.nodes.extend(items),
            loc,
        }
    }
}

pub struct Ast {
    pub root: AstId<Module>,
    pub arenas: AstArenas,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum ParseError {
    MissingToken,
    UnexpectedToken(TokenKind),
    Incomplete(&'static str),
}

impl<'a> FromLexerError<'a> for ParseError {
    fn missing_token(_: usize) -> Self {
        Self::MissingToken
    }
    fn unexpected_token(token: Token<'a>) -> Self {
        Self::UnexpectedToken(token.kind())
    }
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer,
            exprs_sp: Vec::with_capacity(16),
        }
    }

    pub fn parse_file(&mut self) -> Result<Ast, ParseError> {
        let mut arenas = AstArenas::default();
        let module = Module::parse(self, &mut arenas)?;

        Ok(Ast {
            root: module,
            arenas,
            path: PathBuf::default(),
        })
    }

    fn lexer(&mut self) -> &mut Lexer<'a> {
        &mut self.lexer
    }
}

pub trait Consumable<'a>: Sized + Copy {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError>;

    fn try_consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Option<(Self, Span)> {
        let save = p.lexer().save();

        match Self::consume(p, arenas) {
            Ok(v) => {
                save.ignore();
                Some(v)
            }
            Err(_) => {
                p.lexer().restore(save);
                None
            }
        }
    }
}

pub trait Parsable<'a>: Consumable<'a> {
    fn parse(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<AstId<Self>, ParseError> {
        Ok(Self::parse_with_span(p, arenas)?.0)
    }

    fn parse_with_span(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
    ) -> Result<(AstId<Self>, Span), ParseError> {
        let (item, span) = Self::consume(p, arenas)?;
        Ok((arenas.add(item, span), span))
    }

    fn try_parse(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Option<AstId<Self>> {
        Some(Self::try_parse_with_span(p, arenas)?.0)
    }

    fn try_parse_with_span(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
    ) -> Option<(AstId<Self>, Span)> {
        let (item, span) = Self::try_consume(p, arenas)?;
        Some((arenas.add(item, span), span))
    }

    fn parse_until_reaching(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        end: TokenKind,
    ) -> Result<(AstIdRange<Self>, Token<'a>), ParseError> {
        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();

        let end_token = loop {
            let Some(peek) = p.lexer.peek() else {
                // @TODO: Better Error
                return Err(ParseError::MissingToken);
            };

            if peek.kind() == end {
                break peek.commit();
            }

            peek.release();
            let (item, span) = Self::consume(p, arenas)?;
            items.push(item);
            spans.push(span);
        };

        Ok((arenas.add_range(items, spans), end_token))
    }

    fn parse_one_or_more_delimited(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        delimiter: TokenKind,
    ) -> Result<(Vec<AstId<Self>>, Span), ParseError> {
        let (item, mut span) = Self::consume(p, arenas)?;
        let item = arenas.add(item, span);

        let mut items = Vec::new();
        items.push(item);

        loop {
            if p.lexer.next_if_equals(delimiter).is_none() {
                break;
            }

            let (item, end_loc) = Self::consume(p, arenas)?;
            span |= end_loc;
            let item = arenas.add(item, end_loc);

            items.push(item);
        }

        Ok((items, span))
    }

    fn parse_zero_or_more_delimited(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        delimiter: TokenKind,
    ) -> Result<Vec<AstId<Self>>, ParseError> {
        let Some(item) = Self::try_parse(p, arenas) else {
            return Ok(Vec::new());
        };

        let mut items = Vec::new();
        items.push(item);

        loop {
            if p.lexer.next_if_equals(delimiter).is_none() {
                break;
            }

            items.push(Self::parse(p, arenas)?);
        }

        Ok(items)
    }
}

// #[derive(Debug, Clone)]
// pub enum EventExpression<'a> {
//     Plain(AstId<'a, Expr<'a>>),
//     Posedge(AstId<'a, Expr<'a>>),
//     Negedge(AstId<'a, Expr<'a>>),
//     OrList(Vec<AstId<'a, Expr<'a>>>),
// }

impl<'a> Consumable<'a> for IdentRef {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        let token = p.lexer().next_expect()?;
        let (content, span) = token.take();

        let kind = content.kind();

        if let TokenContent::Ident(ident) = content {
            return Ok((Self::from_item(ident, arenas)?, span));
        }

        Err(ParseError::UnexpectedToken(kind))
    }
}
impl<'a> ItemParsable<'a> for IdentRef {
    type Item = &'a str;
    fn from_item(item: Self::Item, arenas: &mut AstArenas) -> Result<Self, ParseError> {
        let start = arenas.idents.len();
        let end = start + item.len();
        arenas.idents.push_str(item);
        Ok(IdentRef { start, end })
    }
}

impl<'a> Consumable<'a> for DecimalRef {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        let token = p.lexer().next_expect()?;
        let (content, span) = token.take();

        let kind = content.kind();

        if let TokenContent::Decimal(d) = content {
            return Ok((Self::from_item(d, arenas)?, span));
        }

        Err(ParseError::UnexpectedToken(kind))
    }
}
impl<'a> ItemParsable<'a> for DecimalRef {
    type Item = Decimal;
    fn from_item(item: Self::Item, arenas: &mut AstArenas) -> Result<Self, ParseError> {
        let at = arenas.decimals.len();
        arenas.decimals.push(item);
        Ok(Self { at })
    }
}

impl<'a> Consumable<'a> for SizedNumberRef {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        let token = p.lexer().next_expect()?;
        let (content, span) = token.take();

        let kind = content.kind();

        if let TokenContent::Number(n) = content {
            return Ok((Self::from_item(n, arenas)?, span));
        }

        Err(ParseError::UnexpectedToken(kind))
    }
}
impl<'a> ItemParsable<'a> for SizedNumberRef {
    type Item = SizedNumber;
    fn from_item(item: Self::Item, arenas: &mut AstArenas) -> Result<Self, ParseError> {
        let at = arenas.sized_numbers.len();
        arenas.sized_numbers.push(item);
        Ok(Self { at })
    }
}

pub trait ItemParsable<'a>: Consumable<'a> {
    type Item;
    fn from_item(item: Self::Item, arenas: &mut AstArenas) -> Result<Self, ParseError>;
    fn ast_from_item(item: Self::Item, span: Span, arenas: &mut AstArenas) -> Result<AstItem<Self>, ParseError> {
        let item = Self::from_item(item, arenas)?;
        let loc = arenas.spans.len();
        arenas.spans.push(span);
        Ok(AstItem { item, loc })
    }

    fn parse(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<AstItem<Self>, ParseError> {
        Ok(Self::parse_with_span(p, arenas)?.0)
    }

    fn parse_with_span(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
    ) -> Result<(AstItem<Self>, Span), ParseError> {
        let (item, span) = Self::consume(p, arenas)?;
        let loc = arenas.spans.len();
        arenas.spans.push(span);
        Ok((AstItem { item, loc }, span))
    }

    fn try_parse(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Option<AstItem<Self>> {
        Some(Self::try_parse_with_span(p, arenas)?.0)
    }

    fn try_parse_with_span(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
    ) -> Option<(AstItem<Self>, Span)> {
        let (item, span) = Self::try_consume(p, arenas)?;
        let loc = arenas.spans.len();
        arenas.spans.push(span);
        Some((AstItem { item, loc }, span))
    }
}
//
// #[cfg(test)]
// pub mod tests {
//     use crate::ast::AstDisplayable;
//
//     use super::*;
//
//     pub fn parse<'a, T: AstDisplayable<'a> + Parsable<'a>>(s: &'a str) -> Result<T, ParseError> {
//         let lexer = Lexer::new(s, None);
//         let mut parser = Parser::new(lexer);
//         let mut arenas = AstArenas::new();
//
//         let id = T::parse(&mut parser, &mut arenas)?;
//
//         println!("{}", id.display(&arenas));
//
//         let lexer = Lexer::new(s, None);
//         let mut parser = Parser::new(lexer);
//         let mut arenas = AstArenas::new();
//
//         let (item, _) = T::consume(&mut parser, &mut arenas)?;
//
//         Ok(item)
//     }
//
//     #[test]
//     fn expr() {
//         // let expr = parse("5 + ( 7)[5] / (3[1] - 3 > 3 << !+2)");
//         let expr = parse("a + (2 ? 1 : 4) * 3 - 4");
//         assert!(matches!(dbg!(&expr), Ok(Expr::Binary(..))));
//         assert!(false);
//     }
// }
