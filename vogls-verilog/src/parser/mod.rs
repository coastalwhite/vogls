use std::path::PathBuf;

use crate::arena::Arena;
use crate::ast::module::Module;
use crate::ast::{
    AstId, AstIdRange, AstItem, DecimalRef, Identifier, SizedNumberRef, StringRef, TextRef,
};
use crate::number::{Decimal, SizedNumber};
use crate::span::Span;
use crate::tokenizer::{FromLexerError, Takeable, Token};
pub use token_walker::TokenWalker;
pub use diagnostics::{Diagnostics, report_error};

mod constant_expr;
mod expr;
mod module;
mod statement;
mod token_walker;
mod utils;
mod diagnostics;
// mod net;

pub struct Parser<'a> {
    tkw: TokenWalker<'a>,
    /// A `scratchpad` to parse expressions
    exprs_sp: Vec<(expr::StackItem, expr::BindingPower, Span)>,
}

#[derive(Default)]
pub struct AstArenas {
    pub nodes: Arena,
    pub spans: Vec<Span>,

    pub text: String,
    pub decimals: Vec<Decimal>,
    pub sized_numbers: Vec<SizedNumber>,
}
impl AstArenas {
    fn add<T: Copy + 'static>(&mut self, item: T, span: Span) -> AstId<T> {
        let loc = self.spans.len();
        self.spans.push(span);
        AstId {
            node: self.nodes.add(item),
            loc,
        }
    }

    fn add_tuple<T: Copy + 'static>(&mut self, (item, span): (T, Span)) -> AstId<T> {
        self.add(item, span)
    }

    fn add_range<T: Copy + 'static>(
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

    pub fn get_span<T: Copy>(&self, id: AstId<T>) -> Span {
        self.spans[id.loc]
    }

    pub fn get<T: Copy + 'static>(&self, id: AstId<T>) -> &T {
        self.nodes.get(id.node)
    }

    pub fn get_ident(&self, ident_ref: TextRef) -> &str {
        &self.text[ident_ref.start..ident_ref.end]
    }
}

pub struct Ast {
    pub modules: AstIdRange<Module>,
    pub arenas: AstArenas,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub location: Option<Span>,
    pub reason: ParseErrorReason,
}

impl ParseError {
    fn incomplete(location: Option<Span>, ident: &'static str) -> ParseError {
        Self {
            location,
            reason: ParseErrorReason::Incomplete(ident),
        }
    }

}

#[derive(Debug, Clone, Copy)]
pub enum ParseErrorKind {
    MissingToken,
    UnexpectedToken,
    Incomplete,
}

#[derive(Debug, Clone)]
pub enum ParseErrorReason {
    MissingToken,
    UnexpectedToken(Token),
    Incomplete(&'static str),
}

impl FromLexerError for ParseError {
    fn missing_token(at: usize) -> Self {
        Self {
            location: Some(Span::new(at, at)),
            reason: ParseErrorReason::MissingToken,
        }
    }
    fn unexpected_token() -> Self {
        // println!("{}", std::backtrace::Backtrace::force_capture());
        Self {
            location: None,
            reason: ParseErrorReason::UnexpectedToken(Token::Dot),
        }
    }
}

impl<'a> Parser<'a> {
    pub fn new(lexer: TokenWalker<'a>) -> Self {
        Self {
            tkw: lexer,
            exprs_sp: Vec::with_capacity(16),
        }
    }

    pub fn parse_file(&mut self, diagnostics: Option<&mut Diagnostics>) -> Result<Ast, ()> {
        let mut arenas = AstArenas::default();
        match utils::parse_one_or_more::<Module>(self, &mut arenas, diagnostics) {
            Ok(modules) => Ok(Ast {
                modules,
                arenas,
                path: PathBuf::default(),
            }),
            Err(_) => Err(()),
        }
    }
}

pub trait Consumable<'a>: Sized + Copy + 'static {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind>;
    fn try_consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Option<(Self, Span)> {
        let save = p.tkw.offset;

        match Self::consume(p, arenas, None) {
            Ok(v) => Some(v),
            Err(_) => {
                p.tkw.offset = save;
                None
            }
        }
    }
}

impl<'a> Consumable<'a> for Identifier {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        let t = p
            .tkw
            .next_expect(Token::Ident, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.as_range()];
        Ok((Self::from_item(content, arenas, diagnostics)?, span))
    }
}
impl<'a> ItemParsable<'a> for Identifier {
    type Item = &'a str;
    fn from_item(
        item: Self::Item,
        arenas: &mut AstArenas,
        _diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        let start = arenas.text.len();
        let end = start + item.len();
        arenas.text.push_str(item);
        Ok(Self(TextRef { start, end }))
    }
}

impl<'a> Consumable<'a> for DecimalRef {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        let t = p
            .tkw
            .next_expect(Token::Decimal, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.as_range()];
        let (_, decimal) = Decimal::take(content);
        Ok((Self::from_item(decimal, arenas, diagnostics)?, span))
    }
}
impl<'a> ItemParsable<'a> for DecimalRef {
    type Item = Decimal;
    fn from_item(
        item: Self::Item,
        arenas: &mut AstArenas,
        _diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        let at = arenas.decimals.len();
        arenas.decimals.push(item);
        Ok(Self { at })
    }
}

impl<'a> Consumable<'a> for SizedNumberRef {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        let t = p
            .tkw
            .next_expect(Token::Number, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.as_range()];
        let (_, number) = SizedNumber::take(content);
        Ok((Self::from_item(number, arenas, diagnostics)?, span))
    }
}
impl<'a> ItemParsable<'a> for SizedNumberRef {
    type Item = SizedNumber;
    fn from_item(
        item: Self::Item,
        arenas: &mut AstArenas,
        _diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        let at = arenas.sized_numbers.len();
        arenas.sized_numbers.push(item);
        Ok(Self { at })
    }
}

impl<'a> Consumable<'a> for StringRef {
    fn consume(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(Self, Span), ParseErrorKind> {
        let t = p
            .tkw
            .next_expect(Token::String, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.as_range()];
        let content = &content[1..content.len() - 1];

        if content.contains("\\") {
            todo!()
        }

        Ok((Self::from_item(content, arenas, diagnostics)?, span))
    }
}
impl<'a> ItemParsable<'a> for StringRef {
    type Item = &'a str;
    fn from_item(
        item: Self::Item,
        arenas: &mut AstArenas,
        _diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        let start = arenas.text.len();
        let end = start + item.len();
        arenas.text.push_str(item);
        Ok(Self(TextRef { start, end }))
    }
}

pub trait ItemParsable<'a>: Consumable<'a> {
    type Item;
    fn from_item(
        item: Self::Item,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind>;
    fn ast_from_item(
        item: Self::Item,
        span: Span,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<AstItem<Self>, ParseErrorKind> {
        let item = Self::from_item(item, arenas, diagnostics)?;
        let loc = arenas.spans.len();
        arenas.spans.push(span);
        Ok(AstItem { item, loc })
    }

    fn item_parse(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<AstItem<Self>, ParseErrorKind> {
        Ok(Self::item_parse_with_span(p, arenas, diagnostics)?.0)
    }

    fn item_parse_with_span(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<(AstItem<Self>, Span), ParseErrorKind> {
        let (item, span) = Self::consume(p, arenas, diagnostics)?;
        let loc = arenas.spans.len();
        arenas.spans.push(span);
        Ok((AstItem { item, loc }, span))
    }

    fn try_item_parse(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Option<AstItem<Self>> {
        Some(Self::try_item_parse_with_span(p, arenas)?.0)
    }

    fn try_item_parse_with_span(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
    ) -> Option<(AstItem<Self>, Span)> {
        let (item, span) = Self::try_consume(p, arenas)?;
        let loc = arenas.spans.len();
        arenas.spans.push(span);
        Some((AstItem { item, loc }, span))
    }
}
