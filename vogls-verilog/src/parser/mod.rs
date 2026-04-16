use std::str::FromStr;

use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_utils::VgHashSet;

use crate::arena::Arena;
use crate::ast::constant_expr::ConstantExpr;
use crate::ast::module::{Description, Module, TimeScale};
use crate::ast::udp::UdpDeclaration;
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

use self::token_walker::TokenLoc;
use self::utils::{item_parse, parse, parse_one_or_more_delimited};

mod constant_expr;
mod diagnostics;
mod expr;
mod module;
mod statement;
mod token_walker;
mod udp;
mod utils;

#[derive(Default)]
pub struct ParserScratches<'a> {
    /// A `scratchpad` to parse expressions
    exprs_sp: Vec<(expr::StackItem<'a>, expr::BindingPower, TokenRange)>,
    udps: VgHashSet<IdentId>,
}

#[derive(Default)]
pub struct AstArenas {
    pub spans: Vec<TokenRange>,
    pub text: String,
    pub decimals: Vec<Bits>,
    pub sized_numbers: Vec<SizedNumber>,
    pub ident_table: IdentTable,
}
impl AstArenas {
    pub fn add_tr(&mut self, tr: TokenRange) -> usize {
        let loc = self.spans.len();
        self.spans.push(tr);
        loc
    }

    pub fn add_tr_range(&mut self, trs: impl IntoIterator<Item = TokenRange>) -> usize {
        let loc = self.spans.len();
        self.spans.extend(trs);
        loc
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

    pub fn get_item_span<T: Copy>(&self, id: AstItem<T>) -> TokenRange {
        self.spans[id.loc]
    }

    pub fn get_ident(&self, ident_ref: TextRef) -> &str {
        &self.text[ident_ref.start..ident_ref.end]
    }

    pub fn to_item<T: Copy + 'static>(&self, id: AstId<T>) -> AstItem<T> {
        AstItem {
            item: *id,
            loc: id.loc,
        }
    }
}

pub struct Ast<'a> {
    pub descriptions: AstIdRange<'a, Description<'a>>,
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

#[derive(Clone, Copy)]
pub enum TimeUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
    Picoseconds,
    Femtoseconds,
}
#[derive(Clone, Copy)]
pub enum TimeSize {
    N1,
    N10,
    N100,
}
impl TimeSize {
    fn into_u64(self) -> u64 {
        match self {
            TimeSize::N1 => 1,
            TimeSize::N10 => 10,
            TimeSize::N100 => 100,
        }
    }
}
impl TimeUnit {
    fn convert_from_fs(self, fs: u64) -> u64 {
        match self {
            TimeUnit::Seconds => fs * 10u64.pow(15),
            TimeUnit::Milliseconds => fs * 10u64.pow(12),
            TimeUnit::Microseconds => fs * 10u64.pow(9),
            TimeUnit::Nanoseconds => fs * 10u64.pow(6),
            TimeUnit::Picoseconds => fs * 10u64.pow(3),
            TimeUnit::Femtoseconds => fs,
        }
    }
}

impl FromStr for TimeSize {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(Self::N1),
            "10" => Ok(Self::N10),
            "100" => Ok(Self::N100),
            _ => Err(()),
        }
    }
}

impl FromStr for TimeUnit {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "s" => Ok(Self::Seconds),
            "ms" => Ok(Self::Milliseconds),
            "us" => Ok(Self::Microseconds),
            "ns" => Ok(Self::Nanoseconds),
            "ps" => Ok(Self::Picoseconds),
            "fs" => Ok(Self::Femtoseconds),
            _ => Err(()),
        }
    }
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

pub struct ParseContext {
    timescale: (TimeSize, TimeUnit, TimeSize, TimeUnit),
    default_nettype: Option<DefaultNettype>,
}

impl ParseContext {
    pub fn new() -> Self {
        Self {
            timescale: (
                TimeSize::N1,
                TimeUnit::Seconds,
                TimeSize::N1,
                TimeUnit::Seconds,
            ),
            default_nettype: None,
        }
    }
}

pub fn parse_file<'a>(
    tkw: &mut TokenWalker<'_>,
    scratches: &mut ParserScratches<'a>,
    mut diagnostics: Option<&mut Diagnostics>,
    arenas: &mut AstArenas,
    ast: &'a Arena,
    ctx: &mut ParseContext,
) -> Result<Ast<'a>, ()> {
    use Token as T;

    // @NOTE
    // UDPs instantiations use the exact same syntax as module instantiations. In many cases, the
    // only differentiating item is the name. We, therefore, make an overview of all the UDPs
    // defined and use that to differentiate between them.
    scratches.udps.clear();
    for (i, t) in tkw.tokens.iter().enumerate() {
        if *t == T::KeywordPrimitive {
            tkw.offset = i + 1;
            if let Ok(ident) = Identifier::consume(tkw, scratches, arenas, &ast, None) {
                scratches.udps.insert(ident.0);
            }
        }
    }
    tkw.offset = 0;

    let mut descriptions = Vec::new();
    let mut trs = Vec::new();

    loop {
        let attr_instances = utils::parse_zero_or_more_while_next::<AttributeInstance>(
            tkw,
            scratches,
            arenas,
            &ast,
            diagnostics.as_deref_mut(),
            |t| t == T::LeftParenStar,
        )?;

        let Some(t) = tkw.get(tkw.offset) else {
            break;
        };

        match *t.kind {
            T::KeywordModule => {
                let start = tkw.offset;
                let mut module =
                    Module::consume(tkw, scratches, arenas, &ast, diagnostics.as_deref_mut())?;
                let tr = TokenRange {
                    start,
                    end: tkw.offset,
                };

                module.attribute_instances = attr_instances;
                module.default_nettype = ctx.default_nettype;
                let (tu, tu_unit, tp, tp_unit) = ctx.timescale;
                module.time_scale = TimeScale {
                    time_unit: tu_unit.convert_from_fs(tu.into_u64()),
                    time_precision: tp_unit.convert_from_fs(tp.into_u64()),
                };
                descriptions.push(Description::Module(utils::push(arenas, ast, module, tr)));
                trs.push(tr);
            }
            T::KeywordPrimitive => {
                let start = tkw.offset;
                let mut primitive = UdpDeclaration::consume(
                    tkw,
                    scratches,
                    arenas,
                    &ast,
                    diagnostics.as_deref_mut(),
                )?;
                let tr = TokenRange {
                    start,
                    end: tkw.offset,
                };
                primitive.attribute_instances = attr_instances;
                descriptions.push(Description::Udp(utils::push(arenas, ast, primitive, tr)));
                trs.push(tr);
            }
            T::KeywordConfig => {
                diagnostics.map(|d| d.incomplete(tkw.offset, "description::config"));
                return Err(());
            }
            T::Directive => {
                if !attr_instances.is_empty() {
                    // @Note: This is not entirely correct although, this is not really a limitation
                    // you should generally run into.
                    if let Some(diagnostics) = diagnostics {
                        diagnostics.unexpected_token(tkw.offset, T::Directive);
                        return Err(());
                    }
                }

                let (span, file) = (*t.span, *t.file);
                let content = &tkw.content(file)[span.as_range()];
                let directive = &content[1..content.len()];

                match directive {
                    "timescale" => {
                        tkw.offset += 1;
                        let TokenLoc { kind, span, file } =
                            tkw.next_expect(T::Decimal, diagnostics.as_deref_mut())?;
                        let (&kind, &span, &file) = (kind, span, file);
                        let Ok(unit_size) = TimeSize::from_str(&tkw.content(file)[span.as_range()])
                        else {
                            if let Some(diagnostics) = diagnostics {
                                diagnostics.unexpected_token(tkw.offset - 1, kind);
                            }
                            return Err(());
                        };
                        let TokenLoc { kind, span, file } =
                            tkw.next_expect(T::Ident, diagnostics.as_deref_mut())?;
                        let (&kind, &span, &file) = (kind, span, file);
                        let Ok(unit_unit) = TimeUnit::from_str(&tkw.content(file)[span.as_range()])
                        else {
                            if let Some(diagnostics) = diagnostics {
                                diagnostics.unexpected_token(tkw.offset - 1, kind);
                            }
                            return Err(());
                        };
                        tkw.next_expect(T::Slash, diagnostics.as_deref_mut())?;
                        let TokenLoc { kind, span, file } =
                            tkw.next_expect(T::Decimal, diagnostics.as_deref_mut())?;
                        let (&kind, &span, &file) = (kind, span, file);
                        let Ok(prec_size) = TimeSize::from_str(&tkw.content(file)[span.as_range()])
                        else {
                            if let Some(diagnostics) = diagnostics {
                                diagnostics.unexpected_token(tkw.offset - 1, kind);
                            }
                            return Err(());
                        };
                        let TokenLoc { kind, span, file } =
                            tkw.next_expect(T::Ident, diagnostics.as_deref_mut())?;
                        let (&kind, &span, &file) = (kind, span, file);
                        let Ok(prec_unit) = TimeUnit::from_str(&tkw.content(file)[span.as_range()])
                        else {
                            if let Some(diagnostics) = diagnostics {
                                diagnostics.unexpected_token(tkw.offset - 1, kind);
                            }
                            return Err(());
                        };

                        ctx.timescale = (unit_size, unit_unit, prec_size, prec_unit);
                    }
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

    let descriptions = utils::push_range(arenas, ast, descriptions, trs);
    Ok(Ast { descriptions })
}

pub trait Consumable<'a>: Sized + Copy {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()>;
    fn try_consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
    ) -> Option<Self> {
        let save = tkw.offset;
        match Self::consume(tkw, sc, arenas, ast, None) {
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
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        _ast: &'a Arena,
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
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        _ast: &'a Arena,
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
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        _ast: &'a Arena,
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
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        _ast: &'a Arena,
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

impl<'a> Consumable<'a> for AttributeInstance<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
            ast,
            T::Comma,
            diagnostics.as_deref_mut(),
        )?;
        tkw.next_expect(T::StarRightParen, diagnostics.as_deref_mut())?;

        Ok(Self(attr_specs))
    }
}

impl<'a> Consumable<'a> for AttrSpec<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 507
        // attr_spec ::= attr_name [ = constant_expression ]
        // attr_name ::= identifier

        let attr_name = item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
        let mut constant_expression = None;
        if tkw.next_if_equals(T::Equals) {
            constant_expression = Some(parse::<ConstantExpr>(
                tkw,
                sc,
                arenas,
                ast,
                diagnostics.as_deref_mut(),
            )?);
        }

        Ok(Self {
            attr_name,
            constant_expression,
        })
    }
}

impl<'a> Consumable<'a> for HIdent<'a> {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        sc: &mut ParserScratches<'a>,
        arenas: &mut AstArenas,
        ast: &'a Arena,
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
            let ident = item_parse::<Identifier>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;

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
                        parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics.as_deref_mut())?;
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

            let loc = arenas.add_tr_range(spans);
            let node = ast.extend(items);
            let components = AstIdRange { node, loc };

            return Ok(HIdent { components, ident });
        }
    }
}
