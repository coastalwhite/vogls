use std::path::PathBuf;

use crate::arena::Arena;
use crate::ast::{AstId, Module};
use crate::ident::Ident;
use crate::lexer::{FromLexerError, Lexer, Token, TokenContent, TokenKind};
use crate::span::Span;

mod expr;
// mod module;
// mod net;
// mod stmt;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    /// A `scratchpad` to parse expressions
    exprs_sp: Vec<(expr::StackItem, expr::BindingPower, Span)>,
}

#[derive(Default)]
pub struct AstArenas {
    pub nodes: Arena,
    pub spans: Vec<Span>,
}
impl AstArenas {
    fn add<T>(&mut self, item: T, span: Span) -> AstId<T> {
        let loc = self.spans.len();
        self.spans.push(span);
        AstId {
            node: self.nodes.add(item),
            loc,
        }
    }

    fn add_tuple<T>(&mut self, (item, span): (T, Span)) -> AstId<T> {
        self.add(item, span)
    }
}

pub struct Ast {
    pub root: AstId<Module>,
    pub nodes: Arena,
    pub spans: Vec<Span>,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum ParseError {
    MissingToken,
    UnexpectedToken(TokenKind),
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

        Ok((module, arenas))
    }

    fn lexer(&mut self) -> &mut Lexer<'a> {
        &mut self.lexer
    }
}

pub trait Consumable<'a>: Sized {
    fn consume(p: &mut Parser<'a>, ast: &mut AstArenas) -> Result<(Self, Span), ParseError>;

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

pub trait Parsable<'a>: Consumable<'a> + Sized {
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
        let (item, span) = Self::try_consume(p, arenas)?;
        Some(arenas.add(item, span))
    }
    fn parse_until_reaching(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        end: TokenKind,
    ) -> Result<(Vec<AstId<Self>>, Token<'a>), ParseError> {
        let mut items = Vec::new();

        let end_token = loop {
            let Some(peek) = p.lexer.peek() else {
                // @TODO: Better Error
                return Err(ParseError::MissingToken);
            };

            if peek.kind() == end {
                break peek.commit();
            }

            peek.release();
            items.push(Self::parse(p, arenas)?);
        };

        Ok((items, end_token))
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

impl<'a> Consumable<'a> for Ident {
    fn consume(p: &mut Parser<'a>, _arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        let token = p.lexer().next().ok_or(ParseError::MissingToken)?;
        let (content, span) = token.take();

        let kind = content.kind();

        if let TokenContent::Ident(ident) = content {
            return Ok((Ident::new(ident), span));
        }

        Err(ParseError::UnexpectedToken(kind))
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
