use std::path::PathBuf;

use crate::arena::Arena;
use crate::ast::module::Module;
use crate::ast::{AstId, AstIdRange, DecimalRef, Identifier, SizedNumberRef, StringRef, TextRef};
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

#[derive(Default)]
pub struct ParserScratches {
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

pub fn parse_file(
    tkw: &mut TokenWalker<'_>,
    scratches: &mut ParserScratches,
    diagnostics: Option<&mut Diagnostics>,
) -> Result<Ast, ()> {
    let mut arenas = AstArenas::default();
    match utils::parse_one_or_more::<Module>(tkw, scratches, &mut arenas, diagnostics) {
        Ok(modules) => Ok(Ast {
            modules,
            arenas,
            path: PathBuf::default(),
        }),
        Err(_) => Err(()),
    }
}

pub trait Consumable<'a>: Sized + Copy + 'static {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind>;
    fn try_consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
    ) -> Option<Self> {
        let save = tkw.offset;
        match Self::consume(tkw, sc, arenas, None) {
            Ok(v) => Some(v),
            Err(_) => {
                tkw.offset = save;
                None
            }
        }
    }
}

impl<'a> Consumable<'a> for Identifier {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        let t = tkw.next_expect(Token::Ident, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &tkw.content(file)[span.as_range()];
        let start = arenas.text.len();
        let end = start + content.len();
        arenas.text.push_str(content);
        Ok(Self(TextRef { start, end }))
    }
}

impl<'a> Consumable<'a> for DecimalRef {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        let t = tkw.next_expect(Token::Decimal, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &tkw.content(file)[span.as_range()];
        let (_, decimal) = Decimal::take(content);
        let at = arenas.decimals.len();
        arenas.decimals.push(decimal);
        Ok(Self { at })
    }
}

impl<'a> Consumable<'a> for SizedNumberRef {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        let t = tkw.next_expect(Token::Number, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &tkw.content(file)[span.as_range()];
        let (_, number) = SizedNumber::take(content);
        let at = arenas.sized_numbers.len();
        arenas.sized_numbers.push(number);
        Ok(Self { at })
    }
}

impl<'a> Consumable<'a> for StringRef {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        _sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ParseErrorKind> {
        let t = tkw.next_expect(Token::String, diagnostics.as_deref_mut())?;
        let (span, file) = (*t.span, *t.file);
        let content = &tkw.content(file)[span.as_range()];
        let content = &content[1..content.len() - 1];

        if content.contains("\\") {
            todo!()
        }

        let start = arenas.text.len();
        let end = start + content.len();
        arenas.text.push_str(content);
        Ok(Self(TextRef { start, end }))
    }
}
