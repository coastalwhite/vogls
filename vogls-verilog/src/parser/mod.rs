use std::path::PathBuf;

use crate::arena::Arena;
use crate::ast::module::Module;
use crate::ast::{
    AstId, AstIdRange, AstItem, DecimalRef, Identifier, SizedNumberRef, StringRef, TextRef,
};
use crate::number::{Decimal, SizedNumber};
use crate::tokenizer::{Takeable, Token};
pub use diagnostics::{Diagnostics, report_error};
pub use token_walker::TokenWalker;

use self::token_walker::TokenRange;

mod constant_expr;
mod diagnostics;
mod expr;
mod module;
mod statement;
mod token_walker;
mod utils;
// mod net;

pub struct Parser<'a> {
    tkw: TokenWalker<'a>,
    /// A `scratchpad` to parse expressions
    exprs_sp: Vec<(expr::StackItem, expr::BindingPower, TokenRange)>,
}

#[derive(Default)]
pub struct AstArenas {
    pub nodes: Arena,
    pub spans: Vec<TokenRange>,

    pub text: String,
    pub decimals: Vec<Decimal>,
    pub sized_numbers: Vec<SizedNumber>,
}
impl AstArenas {
    fn add<T: Copy + 'static>(&mut self, item: T, range: TokenRange) -> AstId<T> {
        let loc = self.spans.len();
        self.spans.push(range);
        AstId {
            node: self.nodes.add(item),
            loc,
        }
    }

    fn add_tuple<T: Copy + 'static>(&mut self, (item, span): (T, TokenRange)) -> AstId<T> {
        self.add(item, span)
    }

    fn add_range<T: Copy + 'static>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        spans: impl IntoIterator<Item = TokenRange>,
    ) -> AstIdRange<T> {
        let loc = self.spans.len();
        self.spans.extend(spans);
        AstIdRange {
            node: self.nodes.extend(items),
            loc,
        }
    }

    pub fn get_span<T: Copy>(&self, id: AstId<T>) -> TokenRange {
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
    ) -> Result<Self, ParseErrorKind>;
    fn try_consume(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Option<Self> {
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
    ) -> Result<Self, ParseErrorKind> {
        let t = p
            .tkw
            .next_expect(Token::Ident, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.as_range()];
        Ok(Self::from_item(content, arenas, diagnostics)?)
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
    ) -> Result<Self, ParseErrorKind> {
        let t = p
            .tkw
            .next_expect(Token::Decimal, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.as_range()];
        let (_, decimal) = Decimal::take(content);
        Ok(Self::from_item(decimal, arenas, diagnostics)?)
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
    ) -> Result<Self, ParseErrorKind> {
        let t = p
            .tkw
            .next_expect(Token::Number, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.as_range()];
        let (_, number) = SizedNumber::take(content);
        Ok(Self::from_item(number, arenas, diagnostics)?)
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
    ) -> Result<Self, ParseErrorKind> {
        let t = p
            .tkw
            .next_expect(Token::String, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &p.tkw.content(file)[span.as_range()];
        let content = &content[1..content.len() - 1];

        if content.contains("\\") {
            todo!()
        }

        Ok(Self::from_item(content, arenas, diagnostics)?)
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
        token_range: TokenRange,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<AstItem<Self>, ParseErrorKind> {
        let item = Self::from_item(item, arenas, diagnostics)?;
        let loc = arenas.spans.len();
        arenas.spans.push(token_range);
        Ok(AstItem { item, loc })
    }

    fn item_parse(
        p: &mut Parser<'a>,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<AstItem<Self>, ParseErrorKind> {
        utils::item_parse(p, arenas, diagnostics)
    }

    fn try_item_parse(p: &mut Parser<'a>, arenas: &mut AstArenas) -> Option<AstItem<Self>> {
        utils::try_item_parse(p, arenas)
    }
}
