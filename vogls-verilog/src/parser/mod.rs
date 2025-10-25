use std::path::PathBuf;

use crate::arena::Arena;
use crate::ast::module::Module;
use crate::ast::{
    AstId, AstIdRange, AstItem, DecimalRef, Identifier, SizedNumberRef, StringRef, TextRef,
};
use crate::lexer::{FromLexer2Error, Lexer2, Takeable, TokenKind};
use crate::number::{Decimal, SizedNumber};
use crate::span::Span;

mod constant_expr;
mod expr;
mod module;
mod statement;
// mod net;

pub struct Parser<'a> {
    lexer: Lexer2<'a>,
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
        &self.idents[ident_ref.start..ident_ref.end]
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

    pub fn report(&self, path: &str, content: &str, out: &mut String) -> std::fmt::Result {
        use std::fmt::Write;
        writeln!(out, "Failed to read file. Reason: {:?}", self.reason)?;
        if let Some(location) = self.location {
            let lines = lines_with_offset(&content);
            let start_line =
                match lines.binary_search_by_key(&location.start(), |(offset, _)| *offset) {
                    Ok(v) => v,
                    Err(v) => v - 1,
                };
            let end_line = match lines.binary_search_by_key(&location.end(), |(offset, _)| *offset)
            {
                Ok(v) => v,
                Err(v) => v - 1,
            };

            const CTX_LINES: usize = 2;
            let ctx_start_line = start_line.saturating_sub(CTX_LINES);
            let ctx_end_line = end_line.saturating_add(1 + CTX_LINES).min(lines.len());

            writeln!(out, "[{path}:{}]:", ctx_start_line + 1)?;
            for line in ctx_start_line..start_line {
                let (_, line) = lines[line];
                writeln!(out, "| {line}")?;
            }

            if start_line == end_line {
                let (offset, line) = lines[start_line];
                writeln!(out, "> {line}")?;
                writeln!(
                    out,
                    "  {:start_pad$}{:len$}",
                    "",
                    "^",
                    start_pad = location.start() - offset,
                    len = location.len()
                )?;
            } else {
                let (offset, line) = lines[start_line];
                writeln!(out, "> {line}")?;
                writeln!(
                    out,
                    "  {:start_pad$}{:len$}",
                    "",
                    "^",
                    start_pad = location.start() - offset,
                    len = line.len() - location.start() - offset,
                )?;

                for line in start_line + 1..end_line {
                    let (_, line) = lines[line];
                    writeln!(out, "> {line}")?;
                    writeln!(out, "  {:len$}", "^", len = line.len(),)?;
                }

                let (offset, line) = lines[end_line];
                writeln!(out, "> {line}")?;
                writeln!(out, "  {:len$}", "^", len = location.end() - offset,)?;
            }

            for line in end_line.saturating_add(1).min(ctx_end_line)..ctx_end_line {
                let (_, line) = lines[line];
                writeln!(out, "| {line}")?;
            }
        }
        Ok(())
    }
}

fn lines_with_offset(mut s: &str) -> Vec<(usize, &str)> {
    let original_length = s.len();
    let mut vs = Vec::new();
    while let Some(p) = s.find(['\n', '\r']) {
        if s.as_bytes()[p] == b'\r' {
            todo!();
        }

        let offset = original_length - s.len();
        vs.push((offset, &s[..p]));
        s = &s[p + 1..];
    }

    if !s.is_empty() {
        let offset = original_length - s.len();
        vs.push((offset, s));
    }

    vs
}

#[derive(Debug, Clone)]
pub enum ParseErrorReason {
    MissingToken,
    UnexpectedToken(TokenKind),
    Incomplete(&'static str),
}

impl FromLexer2Error for ParseError {
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
            reason: ParseErrorReason::UnexpectedToken(TokenKind::Dot),
        }
    }
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer2<'a>) -> Self {
        Self {
            lexer,
            exprs_sp: Vec::with_capacity(16),
        }
    }

    pub fn parse_file(&mut self) -> Result<Ast, ParseError> {
        let mut arenas = AstArenas::default();
        let modules = Module::parse_one_or_more(self, &mut arenas)?;

        Ok(Ast {
            modules,
            arenas,
            path: PathBuf::default(),
        })
    }
}

pub trait Consumable<'a>: Sized + Copy + 'static {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError>;
    fn try_consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Option<(Self, Span)> {
        let save = p.lexer.offset;

        match Self::consume(p, arenas) {
            Ok(v) => Some(v),
            Err(_) => {
                p.lexer.offset = save;
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
    ) -> Result<AstIdRange<Self>, ParseError> {
        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();

        loop {
            let token = p.lexer.try_next()?;
            if *token.kind == end {
                break;
            }
            p.lexer.offset -= 1;

            let (item, span) = Self::consume(p, arenas)?;
            items.push(item);
            spans.push(span);
        }

        Ok(arenas.add_range(items, spans))
    }

    fn parse_one_or_more_delimited(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        delimiter: TokenKind,
    ) -> Result<AstIdRange<Self>, ParseError> {
        let (item, span) = Self::consume(p, arenas)?;

        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();
        items.push(item);
        spans.push(span);

        loop {
            if !p.lexer.next_if_equals(delimiter) {
                break;
            }

            let (item, span) = Self::consume(p, arenas)?;
            items.push(item);
            spans.push(span);
        }

        Ok(arenas.add_range(items, spans))
    }

    fn parse_zero_or_more_delimited(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        delimiter: TokenKind,
    ) -> Result<AstIdRange<Self>, ParseError> {
        let Some((item, span)) = Self::try_consume(p, arenas) else {
            return Ok(AstIdRange::default());
        };

        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();
        items.push(item);
        spans.push(span);

        loop {
            if !p.lexer.next_if_equals(delimiter) {
                break;
            }

            let (item, span) = Self::consume(p, arenas)?;
            items.push(item);
            spans.push(span);
        }

        Ok(arenas.add_range(items, spans))
    }

    fn parse_one_or_more(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
    ) -> Result<AstIdRange<Self>, ParseError> {
        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();

        loop {
            let (item, span) = Self::consume(p, arenas)?;
            items.push(item);
            spans.push(span);

            if p.lexer.is_empty() {
                break;
            }
        }

        Ok(arenas.add_range(items, spans))
    }
}

impl<'a> Consumable<'a> for Identifier {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        let span = *p.lexer.next_expect(TokenKind::Ident)?;
        let content = &p.lexer.content()[span.as_range()];
        Ok((Self::from_item(content, arenas)?, span))
    }
}
impl<'a> ItemParsable<'a> for Identifier {
    type Item = &'a str;
    fn from_item(item: Self::Item, arenas: &mut AstArenas) -> Result<Self, ParseError> {
        let start = arenas.idents.len();
        let end = start + item.len();
        arenas.idents.push_str(item);
        Ok(Self(TextRef { start, end }))
    }
}

impl<'a> Consumable<'a> for DecimalRef {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        let span = *p.lexer.next_expect(TokenKind::Decimal)?;
        let content = &p.lexer.content()[span.as_range()];
        let (_, decimal) = Decimal::take(content);
        Ok((Self::from_item(decimal, arenas)?, span))
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
        let span = *p.lexer.next_expect(TokenKind::Number)?;
        let content = &p.lexer.content()[span.as_range()];
        let (_, number) = SizedNumber::take(content);
        Ok((Self::from_item(number, arenas)?, span))
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

impl<'a> Consumable<'a> for StringRef {
    fn consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Result<(Self, Span), ParseError> {
        let span = *p.lexer.next_expect(TokenKind::String)?;
        let content = &p.lexer.content()[span.as_range()];
        let content = &content[1..content.len() - 1];

        if content.contains("\\") {
            todo!()
        }

        Ok((Self::from_item(content, arenas)?, span))
    }
}
impl<'a> ItemParsable<'a> for StringRef {
    type Item = &'a str;
    fn from_item(item: Self::Item, arenas: &mut AstArenas) -> Result<Self, ParseError> {
        let start = arenas.idents.len();
        let end = start + item.len();
        arenas.idents.push_str(item);
        Ok(Self(TextRef { start, end }))
    }
}

pub trait ItemParsable<'a>: Consumable<'a> {
    type Item;
    fn from_item(item: Self::Item, arenas: &mut AstArenas) -> Result<Self, ParseError>;
    fn ast_from_item(
        item: Self::Item,
        span: Span,
        arenas: &mut AstArenas,
    ) -> Result<AstItem<Self>, ParseError> {
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

    fn parse_until_reaching(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        end: TokenKind,
    ) -> Result<AstIdRange<Self>, ParseError> {
        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();

        loop {
            let peek = p.lexer.try_get(p.lexer.offset)?;
            if *peek.kind == end {
                break;
            }
            p.lexer.offset -= 1;

            let (item, span) = Self::consume(p, arenas)?;
            items.push(item);
            spans.push(span);
        }

        Ok(arenas.add_range(items, spans))
    }

    fn parse_one_or_more_delimited_until_fail(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        delimiter: TokenKind,
    ) -> Result<AstIdRange<Self>, ParseError> {
        let (item, span) = Self::consume(p, arenas)?;

        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();
        items.push(item);
        spans.push(span);

        loop {
            let save = p.lexer.offset;
            if !p.lexer.next_if_equals(delimiter) {
                break;
            }

            let Some((item, span)) = Self::try_consume(p, arenas) else {
                p.lexer.offset = save;
                break;
            };
            items.push(item);
            spans.push(span);
        }

        Ok(arenas.add_range(items, spans))
    }

    fn parse_one_or_more_delimited(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        delimiter: TokenKind,
    ) -> Result<AstIdRange<Self>, ParseError> {
        let (item, span) = Self::consume(p, arenas)?;

        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();
        items.push(item);
        spans.push(span);

        loop {
            if !p.lexer.next_if_equals(delimiter) {
                break;
            }

            let (item, span) = Self::consume(p, arenas)?;
            items.push(item);
            spans.push(span);
        }

        Ok(arenas.add_range(items, spans))
    }

    fn parse_zero_or_more_delimited(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        delimiter: TokenKind,
    ) -> Result<AstIdRange<Self>, ParseError> {
        let Some((item, span)) = Self::try_consume(p, arenas) else {
            return Ok(AstIdRange::default());
        };

        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();
        items.push(item);
        spans.push(span);

        loop {
            if !p.lexer.next_if_equals(delimiter) {
                break;
            }

            let (item, span) = Self::consume(p, arenas)?;
            items.push(item);
            spans.push(span);
        }

        Ok(arenas.add_range(items, spans))
    }
}
