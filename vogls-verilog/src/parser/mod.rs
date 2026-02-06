use vogls_frontend::ident_table::IdentTable;

use crate::arena::Arena;
use crate::ast::constant_expr::ConstantExpr;
use crate::ast::module::Module;
use crate::ast::{
    AstId, AstIdRange, AstItem, AttrSpec, AttributeInstance, DecimalRef, HIdent, HIdentComponent,
    Identifier, SizedNumber, SizedNumberRef, StringRef, TextRef,
};
use crate::number::{
    Base, Sign, parse_decimal_bits, skip_sign, take_base, take_binary_bits, take_hexadecimal_bits,
    take_octal_bits, take_size,
};
use crate::tokenizer::Token;
pub use diagnostics::{Diagnostics, report, report_error};
pub use token_walker::TokenWalker;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{Bits, INTEGER_VSIZE};

use self::utils::{item_parse, parse, parse_one_or_more_delimited};

mod constant_expr;
mod diagnostics;
mod expr;
mod module;
mod statement;
mod token_walker;
mod utils;

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
    pub decimals: Vec<Bits>,
    pub sized_numbers: Vec<SizedNumber>,
    pub ident_table: IdentTable,
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
    LeftoverTokens,
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

#[derive(Clone, Copy)]
pub enum DefaultNettype {
    Wire,
    Tri,
    Tri0,
    Tri1,
    Wand,
    Triand,
    Wor,
    Trior,
    Trireg,
    Uwire,
}

#[derive(Default)]
pub struct ParseContext {
    _timescale: Option<(TimeSize, TimeUnit, TimeSize, TimeUnit)>,
    default_nettype: Option<DefaultNettype>,
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
            T::KeywordModule | T::LeftParenStar => {
                let start = tkw.offset;
                let mut module =
                    Module::consume(tkw, scratches, &mut arenas, diagnostics.as_deref_mut())?;
                module.default_nettype = ctx.default_nettype;
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
                    "default_nettype" => {
                        tkw.offset += 1;
                        let t = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
                        let nettype = match *t.kind {
                            T::KeywordWire => Some(DefaultNettype::Wire),
                            T::KeywordTri => Some(DefaultNettype::Tri),
                            T::KeywordTri0 => Some(DefaultNettype::Tri0),
                            T::KeywordTri1 => Some(DefaultNettype::Tri1),
                            T::KeywordWand => Some(DefaultNettype::Wand),
                            T::KeywordTriand => Some(DefaultNettype::Triand),
                            T::KeywordWor => Some(DefaultNettype::Wor),
                            T::KeywordTrior => Some(DefaultNettype::Trior),
                            T::KeywordTrireg => Some(DefaultNettype::Trireg),
                            T::KeywordUwire => Some(DefaultNettype::Uwire),
                            T::Ident if &tkw.content(*t.file)[t.span.as_range()] == "none" => None,
                            t => {
                                if let Some(diagnostics) = diagnostics {
                                    diagnostics.unexpected_token(tkw.offset - 1, t);
                                }
                                return Err(());
                            }
                        };
                        tkw.offset += 1;
                        ctx.default_nettype = nettype;
                    }
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
        let content = &content[usize::from(content.starts_with('\\'))..];
        let ident_id = arenas.ident_table.get_or_insert(content);
        Ok(Self(ident_id))
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
        match parse_decimal_bits(content, Some(INTEGER_VSIZE)) {
            Ok(decimal) => {
                let at = arenas.decimals.len();
                arenas.decimals.push(decimal);
                Ok(Self { at })
            }
            Err(_) => {
                diagnostics.map(|d| d.incomplete(tkw.offset - 1, "decimal overflow"));
                Err(())
            }
        }
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
        let (content, size) = if content.starts_with('\'') {
            (content, None)
        } else {
            let Ok((content, size)) = take_size(&content) else {
                diagnostics.map(|d| d.incomplete(tkw.offset - 1, "decimal overflow"));
                return Err(());
            };
            (content, Some(size))
        };
        debug_assert!(content.starts_with('\''));
        let mut i = 1;
        let has_sign = skip_sign(content.as_bytes(), &mut i);
        let base = take_base(content.as_bytes(), &mut i).unwrap();
        let content = &content[i..];
        let content = content.trim_ascii_start();

        let f = match base {
            Base::Decimal => parse_decimal_bits,
            Base::Binary => take_binary_bits,
            Base::Octal => take_octal_bits,
            Base::Hexadecimal => take_hexadecimal_bits,
        };

        let Ok(bits) = f(content, size) else {
            diagnostics.map(|d| d.incomplete(tkw.offset - 1, "decimal overflow"));
            return Err(());
        };

        let sign = if has_sign {
            Sign::Signed
        } else {
            Sign::Unsigned
        };
        let number = SizedNumber {
            inferred_size: size.is_none(),
            sign,
            base,
            value: bits,
        };
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

        let start = arenas.text.len();
        let mut i = 0;
        while let Some(bs_pos) = content[i..].find('\\') {
            arenas.text.push_str(&content[i..][..bs_pos]);
            match content.as_bytes()[i + bs_pos + 1] {
                b'\\' => arenas.text.push('\\'),
                b'n' => arenas.text.push('\n'),
                b'r' => arenas.text.push('\r'),
                b't' => arenas.text.push('\t'),
                _ => {
                    i += bs_pos + 1;
                    continue;
                }
            }
            i += bs_pos + 2;
        }
        arenas.text.push_str(&content[i..]);
        let end = arenas.text.len();

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

impl<'a> Consumable<'a> for HIdent {
    fn consume(
        tkw: &mut TokenWalker<'a>,
        sc: &mut ParserScratches,
        arenas: &mut AstArenas,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 508
        // hierarchical_identifier ::= { identifier [ [ constant_expression ] ] . } identifier

        // @Optimize: Scratchpad this somehow, it is a bit difficult because we can be recursive
        // here.
        let mut items = Vec::new();
        let mut spans = Vec::new();

        loop {
            let ident = item_parse::<Identifier>(tkw, sc, arenas, diagnostics.as_deref_mut())?;

            let t = tkw.get(tkw.offset).map(|t| *t.kind);
            if t == Some(T::Dot) {
                items.push(HIdentComponent {
                    ident,
                    constant_expr: None,
                });
                spans.push(arenas.get_item_span(ident));
                tkw.offset += 1;
                continue;
            } else if t == Some(T::LeftBrace) {
                tkw.offset += 1;
                let Some(at) = tkw.find_next_same_depth(T::RightBrace) else {
                    diagnostics.map(|d| d.no_corresponding(tkw.offset - 1, T::RightBrace));
                    return Err(());
                };

                if tkw.get(at + 1).map(|t| *t.kind) == Some(T::Dot) {
                    let constant_expr =
                        parse::<ConstantExpr>(tkw, sc, arenas, diagnostics.as_deref_mut())?;
                    tkw.next_expect(T::RightBrace, diagnostics.as_deref_mut())?;
                    tkw.offset += 1;
                    items.push(HIdentComponent {
                        ident,
                        constant_expr: Some(constant_expr),
                    });
                    spans.push(arenas.get_item_span(ident));
                    continue;
                }
                tkw.offset -= 1;
            }

            return Ok(HIdent {
                components: arenas.add_range(items, spans),
                ident,
            });
        }
    }
}
