use crate::arena::Arena;
use crate::ast::constant_expr::ConstantExpr;
use crate::ast::module::Module;
use crate::ast::{
    AstId, AstIdRange, AstItem, AttrSpec, AttributeInstance, DecimalRef, Identifier,
    SizedNumberRef, StringRef, TextRef,
};
use crate::number::{Decimal, SizedNumber};
use crate::tokenizer::{Takeable, Token};
pub use diagnostics::{Diagnostics, report, report_error};
pub use token_walker::{TokenRange, TokenWalker};

use self::utils::{item_parse, parse, parse_one_or_more_delimited};

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

    pub fn get_range_span<T: Copy>(&self, id: AstIdRange<T>) -> TokenRange {
        TokenRange {
            start: self.spans[id.loc].start,
            end: self.spans[id.loc + id.len()].end,
        }
    }

    pub fn get_span<T: Copy>(&self, id: AstId<T>) -> TokenRange {
        self.spans[id.loc]
    }

    pub fn get<T: Copy + 'static>(&self, id: AstId<T>) -> &T {
        self.nodes.get(id.node)
    }

    pub fn get_item_span<T: Copy>(&self, id: AstItem<T>) -> TokenRange {
        self.spans[id.loc]
    }

    pub fn get_ident(&self, ident_ref: TextRef) -> &str {
        &self.text[ident_ref.start..ident_ref.end]
    }

    pub fn to_item<T: Copy + 'static>(&self, id: AstId<T>) -> AstItem<T> {
        AstItem {
            item: *self.get(id),
            loc: id.loc,
        }
    }
}

pub struct Ast {
    pub modules: AstIdRange<Module>,
    pub arenas: AstArenas,
}

#[derive(Debug, Clone, Copy)]
pub enum ParseErrorKind {
    MissingToken,
    UnexpectedToken,
    Incomplete,
    NoCorresponding,
}

#[derive(Debug, Clone)]
pub enum ParseErrorReason {
    MissingToken,
    UnexpectedToken(Token),
    Incomplete(&'static str),
    NoCorresponding(Token),
    NotFound(Token),
}

pub enum TimeUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
    Picoseconds,
    Femtoseconds,
}
pub enum TimeSize {
    N1,
    N10,
    N100,
}

#[derive(Default)]
pub struct ParseContext {
    timescale: Option<(TimeSize, TimeUnit, TimeSize, TimeUnit)>,
}

pub fn parse_file(
    tkw: &mut TokenWalker<'_>,
    scratches: &mut ParserScratches,
    mut diagnostics: Option<&mut Diagnostics>,
    ctx: &mut ParseContext,
) -> Result<Ast, ()> {
    use Token as T;

    let mut arenas = AstArenas::default();

    let mut modules = Vec::new();
    let mut trs = Vec::new();

    while let Some(t) = tkw.get(tkw.offset) {
        match *t.kind {
            T::KeywordModule => {
                let start = tkw.offset;
                let module =
                    Module::consume(tkw, scratches, &mut arenas, diagnostics.as_deref_mut())?;
                let token_range = TokenRange {
                    start,
                    end: tkw.offset,
                };
                modules.push(module);
                trs.push(token_range);
            }
            T::Directive => {
                let (span, file) = (*t.span, *t.file);
                let content = &tkw.content(file)[span.as_range()];
                let directive = &content[1..content.len()];

                match directive {
                    "timescale" => tkw.offset += 6,
                    _ => {
                        if let Some(diagnostics) = diagnostics.as_deref_mut() {
                            diagnostics.incomplete(tkw.offset, "directive not-yet supported");
                        }
                        return Err(());
                    }
                }
            }
            t => {
                if let Some(diagnostics) = diagnostics {
                    diagnostics.unexpected_token(tkw.offset, t);
                }
                return Err(());
            }
        }
    }

    let modules = arenas.add_range(modules, trs);

    Ok(Ast { modules, arenas })
}

pub trait Consumable<'a>: Sized + Copy + 'static {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()>;
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
    ) -> Result<Self, ()> {
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
    ) -> Result<Self, ()> {
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
    ) -> Result<Self, ()> {
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
    ) -> Result<Self, ()> {
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

impl<'a> Consumable<'a> for AttributeInstance {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 507
        // attribute_instance ::= (* attr_spec { , attr_spec } *)

        tkw.next_expect(T::LeftParenStar, diagnostics.as_deref_mut())?;
        let attr_specs = parse_one_or_more_delimited::<AttrSpec>(
            tkw,
            sc,
            arenas,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::StarRightParen, diagnostics.as_deref_mut())?;

        Ok(Self(attr_specs))
    }
}

impl<'a> Consumable<'a> for AttrSpec {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 507
        // attr_spec ::= attr_name [ = constant_expression ]
        // attr_name ::= identifier

        let attr_name = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
        let mut constant_expression = None;
        if tkw.next_if_equals(T::Equals) {
            constant_expression = Some(parse::<ConstantExpr>(
                tkw,
                sc,
                arenas,
                diagnostics.as_deref_mut(),
            )?);
        }

        Ok(Self {
            attr_name,
            constant_expression,
        })
    }
}
