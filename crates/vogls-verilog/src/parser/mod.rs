use std::fmt;
use std::str::FromStr;

use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_ir::time::{TimeResolution, TimeSize, TimeUnit};
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

#[derive(Default, Clone)]
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

#[derive(Clone, Copy)]
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

impl fmt::Display for ParseErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingToken => f.write_str("missing token"),
            Self::UnexpectedToken(token) => write!(f, "unexpected token: {token:?}"),
            Self::Incomplete(reason) => write!(f, "not yet implemented: {reason}"),
            Self::NoCorresponding(token) => write!(f, "no corresponding: {token:?}"),
            Self::NotFound(token) => write!(f, "not found: {token:?}"),
            Self::LeftoverTokens => f.write_str("leftover tokens"),
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
    timescale: TimeScale,
    default_nettype: Option<DefaultNettype>,
    pub min_time_precision: TimeResolution,
}

impl Default for ParseContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ParseContext {
    pub const fn new() -> Self {
        let timescale = TimeScale::new();
        let minimum_time_precision = timescale.precision;
        Self {
            timescale,
            default_nettype: Some(DefaultNettype::Wire),
            min_time_precision: minimum_time_precision,
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
            if let Ok(ident) = Identifier::consume(tkw, scratches, arenas, ast, None) {
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
            ast,
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
                    Module::consume(tkw, scratches, arenas, ast, diagnostics.as_deref_mut())?;
                let tr = TokenRange {
                    start,
                    end: tkw.offset,
                };

                module.attribute_instances = attr_instances;
                module.default_nettype = ctx.default_nettype;
                module.time_scale = ctx.timescale;
                descriptions.push(Description::Module(utils::push(arenas, ast, module, tr)));
                trs.push(tr);
            }
            T::KeywordPrimitive => {
                let start = tkw.offset;
                let mut primitive = UdpDeclaration::consume(
                    tkw,
                    scratches,
                    arenas,
                    ast,
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
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset, "description::config");
                }
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
                        let unit = TimeResolution {
                            unit: unit_unit,
                            size: unit_size,
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
                        let precision = TimeResolution {
                            unit: prec_unit,
                            size: prec_size,
                        };

                        ctx.timescale = TimeScale { unit, precision };
                        ctx.min_time_precision = ctx.min_time_precision.min(precision);
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
                    "resetall" => {
                        tkw.offset += 1;
                        *ctx = ParseContext::new();
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
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        let t = tkw.next_expect(Token::Ident, diagnostics)?;
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
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset - 1, "decimal overflow");
                }
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
            let Ok((content, size)) = take_size(content) else {
                if let Some(d) = diagnostics {
                    d.incomplete(tkw.offset - 1, "decimal overflow");
                }
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
            if let Some(d) = diagnostics {
                d.incomplete(tkw.offset - 1, "decimal overflow");
            }
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
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        let t = tkw.next_expect(Token::String, diagnostics)?;
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
        tkw.next_expect(T::StarRightParen, diagnostics)?;

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
            constant_expression = Some(parse::<ConstantExpr>(tkw, sc, arenas, ast, diagnostics)?);
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
                    if let Some(d) = diagnostics {
                        d.no_corresponding(tkw.offset - 1, T::RightBrace);
                    }
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

impl<'a> Consumable<'a> for f64 {
    fn consume(
        tkw: &mut TokenWalker<'_>,
        _sc: &mut ParserScratches<'a>,
        _arenas: &mut AstArenas,
        _ast: &'a Arena,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<Self, ()> {
        use Token as T;

        let token = tkw.next_expect(T::Real, diagnostics.as_deref_mut())?;
        let file = *token.file;
        let span = *token.span;
        let content = &tkw.content(file)[span.as_range()];
        // @NOTE
        // Real's can contain underscores (`_`) which the default Rust parser
        // cannot parse. Therefore, we remove them here if they are present.
        let content = if content.contains('_') {
            std::borrow::Cow::Owned(content.replace('_', ""))
        } else {
            std::borrow::Cow::Borrowed(content)
        };

        // @Correctness
        // We use Rust's default floating-point parser here. I am not confident
        // that this gives the expected results all the time, but it should produce
        // acceptable results 99% of the time.
        let Ok(content) = content.parse() else {
            if let Some(diagnostics) = diagnostics {
                diagnostics.incomplete(tkw.offset, "wrong real literal");
            }
            return Err(());
        };

        Ok(content)
    }
}
