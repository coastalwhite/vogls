use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;

pub use token::{TokenContent, TokenKind};

use crate::ident::Ident;
use crate::number::{
    Base, BinaryBits, Bits, Decimal, DecimalBits, HexadecimalBits, OctalBits, Sign, Size,
    SizedNumber,
};
use crate::span::Span;

mod token;

pub struct Lexer<'a> {
    inner: LexerInner<'a>,
    peeked: Option<Token<'a>>,
}

#[must_use]
pub struct LexerSave<'a> {
    #[cfg(debug_assertions)]
    content: &'a str,
    #[cfg(debug_assertions)]
    path: Option<Rc<Path>>,

    offset: usize,

    _pd: PhantomData<&'a ()>,
}

impl<'a> LexerSave<'a> {
    pub fn ignore(self) {
        drop(self);
    }
}

#[derive(Clone)]
struct LexerInner<'a> {
    content: &'a str,
    offset: usize,
    path: Option<Rc<Path>>,
}

#[must_use]
pub struct Peeked<'a, 'b> {
    lexer: &'b mut Lexer<'a>,
    token: Token<'a>,
}

pub struct Token<'a> {
    content: TokenContent<'a>,
    span: Span,
}

impl<'a> Token<'a> {
    pub fn new(content: TokenContent<'a>, span: Span) -> Self {
        Self { content, span }
    }
}

#[derive(Debug)]
pub enum ConsumeError {
    MissingCharacter,
    UnexpectedCharacter(char),
}

pub trait Consumable<'a>: Sized {
    fn consume(s: &'a str) -> Result<Consumed<'a, Self>, ConsumeError>;
}

pub trait Takeable<'a>: Sized {
    fn take(s: &'a str) -> (&'a str, Self);
}

pub struct Consumed<'a, T>(pub &'a str, pub T);

impl<'a> LexerInner<'a> {
    fn new(content: &'a str, path: Option<Rc<Path>>) -> Self {
        Self {
            content,
            path,
            offset: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        let new_length = self.content[self.offset..].trim_start().len();
        let trimmed_length = self.content.len() - self.offset - new_length;
        self.offset += trimmed_length;
    }

    fn skip_whitespace_and_comments(&mut self) {
        let mut offset;
        loop {
            self.skip_whitespace();

            offset = self.offset;
            let content = &self.content[self.offset..];
            if content.starts_with("//") {
                let Some(newline_pos) = self.content[self.offset + 2..].find('\n') else {
                    self.offset = self.content.len();
                    break;
                };

                self.offset += 2 + newline_pos + 1;
            } else if content.starts_with("/*") {
                let Some(endblock_pos) = self.content[2..].find("*/") else {
                    self.offset = self.content.len();
                    break;
                };

                self.offset += 2 + endblock_pos + 2;
            }

            // Nothing changed. Break the loop.
            if offset == self.offset {
                break;
            }
        }
    }

    fn save(&self) -> LexerSave<'a> {
        LexerSave {
            #[cfg(debug_assertions)]
            content: self.content,
            #[cfg(debug_assertions)]
            path: self.path.clone(),

            offset: self.offset,

            _pd: PhantomData::default(),
        }
    }

    fn restore(&mut self, save: LexerSave<'a>) {
        debug_assert_eq!(self.content, save.content);
        debug_assert_eq!(self.path, save.path);

        self.offset = save.offset;
    }
}

pub trait FromLexerError<'a> {
    fn missing_token(at: usize) -> Self;
    fn unexpected_token(token: Token<'a>) -> Self;
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a str, path: Option<Rc<Path>>) -> Self {
        Self {
            inner: LexerInner::new(content, path),
            peeked: None,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.inner.path.as_deref()
    }

    pub fn inspect_content(&self) -> &str {
        &self.inner.content[self.inner.offset
            ..self
                .inner
                .offset
                .saturating_add(10)
                .min(self.inner.content.len())]
    }

    pub fn peek<'b>(&'b mut self) -> Option<Peeked<'a, 'b>> {
        let token = self.next()?;
        Some(Peeked { lexer: self, token })
    }
    pub fn next_expect_peek<'b, E: FromLexerError<'a>>(&'b mut self) -> Result<Peeked<'a, 'b>, E> {
        let token = self.next_expect()?;
        Ok(Peeked { lexer: self, token })
    }

    pub fn save<'b>(&self) -> LexerSave<'a> {
        let mut save = self.inner.save();
        if let Some(peeked) = &self.peeked {
            save.offset = peeked.span().start();
        }
        save
    }

    pub fn restore<'b>(&mut self, save: LexerSave<'a>) {
        self.inner.restore(save);
        self.peeked = None;
    }

    pub fn span_at_cursor(&self) -> Span {
        Span::new(self.inner.offset, self.inner.offset)
    }

    pub fn next_expect<E: FromLexerError<'a>>(&mut self) -> Result<Token<'a>, E> {
        self.next()
            .ok_or_else(|| E::missing_token(self.inner.offset))
    }

    pub fn expect<E: FromLexerError<'a>>(&mut self, kind: TokenKind) -> Result<Token<'a>, E> {
        let token = self.next_expect()?;

        if token.kind() != kind {
            return Err(E::unexpected_token(token));
        }

        Ok(token)
    }

    pub fn expect_map<T, E: FromLexerError<'a>>(
        &mut self,
        mut f: impl FnMut(TokenContent<'a>, Span) -> Result<T, E>,
    ) -> Result<(T, Span), E> {
        let Token { content, span } = self
            .next()
            .ok_or_else(|| E::missing_token(self.inner.offset))?;

        f(content, span).map(|r| (r, span))
    }

    pub fn next_if_equals(&mut self, kind: TokenKind) -> Option<Token<'a>> {
        let peeked = self.peek()?;

        if peeked.kind() == kind {
            Some(peeked.commit())
        } else {
            peeked.release();
            None
        }
    }
}

impl<'a, 'b> Peeked<'a, 'b> {
    pub fn release(self) -> TokenKind {
        let kind = self.kind();
        debug_assert!(self.lexer.peeked.is_none());
        self.lexer.peeked = Some(self.token);
        kind
    }

    pub fn commit(self) -> Token<'a> {
        debug_assert!(self.lexer.peeked.is_none());
        self.token
    }

    pub fn kind(&self) -> TokenKind {
        self.token.kind()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(peeked) = self.peeked.take() {
            return Some(peeked);
        }

        self.inner.next()
    }
}

impl<'a> Iterator for LexerInner<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.content.len() {
            return None;
        }

        self.skip_whitespace_and_comments();

        let start = self.offset;
        let Consumed(leftover, content) = TokenContent::consume(&self.content[self.offset..])?;
        self.offset += self.content.len() - self.offset - leftover.len();
        let end = self.offset;

        let span = Span::new(start, end);
        let token = Token { content, span };

        Some(token)
    }
}

impl<'a> TokenContent<'a> {
    fn consume(s: &'a str) -> Option<Consumed<'a, Self>> {
        use TokenContent as T;

        let mut chars = s.chars();
        let fst = chars.next()?;
        let snd = chars.next();
        let trd = chars.next();

        let (content, length) = match (fst, snd, trd) {
            ('(', _, _) => (T::LeftParen, 1),
            (')', _, _) => (T::RightParen, 1),

            ('[', _, _) => (T::LeftBrace, 1),
            (']', _, _) => (T::RightBrace, 1),

            ('{', _, _) => (T::LeftBracket, 1),
            ('}', _, _) => (T::RightBracket, 1),

            (';', _, _) => (T::Semicolon, 1),
            (':', _, _) => (T::Colon, 1),
            (',', _, _) => (T::Comma, 1),

            ('+', _, _) => (T::Plus, 1),
            ('-', _, _) => (T::Minus, 1),
            ('!', Some('='), Some('=')) => (T::ExclamationDoubleEquals, 3),
            ('!', Some('='), _) => (T::ExclamationEquals, 2),
            ('!', _, _) => (T::Exclamation, 1),
            ('~', Some('&'), _) => (T::TildeAmpersand, 2),
            ('~', Some('|'), _) => (T::TildeBar, 2),
            ('~', Some('^'), _) => (T::TildeCaret, 2),
            ('~', _, _) => (T::Tilde, 1),
            ('&', Some('&'), _) => (T::DoubleAmpersand, 2),
            ('&', _, _) => (T::Ampersand, 1),
            ('|', Some('|'), _) => (T::DoubleBar, 2),
            ('|', _, _) => (T::Bar, 1),
            ('^', Some('~'), _) => (T::CaretTilde, 2),
            ('^', _, _) => (T::Caret, 1),
            ('*', Some('*'), _) => (T::DoubleStar, 2),
            ('*', _, _) => (T::Star, 1),
            ('/', _, _) => (T::Slash, 1),
            ('%', _, _) => (T::Procent, 1),
            ('=', Some('='), Some('=')) => (T::TripleEquals, 3),
            ('=', Some('='), _) => (T::DoubleEquals, 2),
            ('<', Some('<'), Some('<')) => (T::TripleLessThan, 3),
            ('<', Some('<'), _) => (T::DoubleLessThan, 2),
            ('<', Some('='), _) => (T::LessThanEquals, 2),
            ('<', _, _) => (T::LessThan, 1),
            ('>', Some('>'), Some('>')) => (T::TripleGreaterThan, 3),
            ('>', Some('>'), _) => (T::DoubleGreaterThan, 2),
            ('>', Some('='), _) => (T::GreaterThanEquals, 2),
            ('>', _, _) => (T::GreaterThan, 1),
            ('?', _, _) => (T::QuestionMark, 1),

            ('@', _, _) => (T::AtSign, 1),
            ('=', _, _) => (T::Equals, 1),
            ('#', _, _) => (T::Hash, 1),

            ('"', _, _) => {
                match s[1..].find('"') {
                    // @Optimize: Use single pass here.
                    Some(end_position)
                        if s[1..1 + end_position]
                            .bytes()
                            .all(|b| b < 128 && b != b'\n') =>
                    {
                        (T::String(&s[1..1 + end_position]), end_position + 2)
                    }
                    _ => (T::Unknown, 1),
                }
            }

            ('0'..='9', _, _) => {
                let initial_length = s.len();

                fn is_valid_prefix(s: &str) -> bool {
                    let pattern = &['D', 'd', 'B', 'b', 'O', 'o', 'X', 'x'];
                    s.starts_with('\'')
                        && if s[1..].starts_with(&['S', 's']) {
                            s[2..].starts_with(pattern)
                        } else {
                            s[1..].starts_with(pattern)
                        }
                }

                if s.starts_with('0')
                    || !is_valid_prefix(s.trim_start_matches(|c: char| matches!(c, '0'..='9')))
                {
                    let (s, decimal) = Decimal::take(s);
                    let length = initial_length - s.len();
                    (T::Decimal(decimal), length)
                } else {
                    let (s, size) = Size::take(s);
                    debug_assert!(s.starts_with('\''));
                    let s = &s[1..];
                    let (s, sign) = Sign::take(s);
                    let (s, base) = Base::take(s);

                    fn into_bits((s, bs): (&str, impl Into<Bits>)) -> (&str, Bits) {
                        (s, bs.into())
                    }

                    let (s, value) = match base {
                        Base::Decimal => into_bits(DecimalBits::take(s)),
                        Base::Binary => into_bits(BinaryBits::take(s)),
                        Base::Octal => into_bits(OctalBits::take(s)),
                        Base::Hexadecimal => into_bits(HexadecimalBits::take(s)),
                    };

                    let length = initial_length - s.len();
                    (
                        T::Number(SizedNumber {
                            size: Some(size),
                            sign,
                            base,
                            value,
                        }),
                        length,
                    )
                }
            }
            ('\'', Some('S' | 's'), b) | ('\'', b, _)
                if matches!(b, Some('D' | 'd' | 'B' | 'b' | 'O' | 'o' | 'X' | 'x')) =>
            {
                let initial_length = s.len();

                let s = &s[1..];

                // @TODO: add better specialization
                let (s, is_signed) = Sign::take(s);
                let (s, base) = Base::take(s);

                fn into_bits((s, bs): (&str, impl Into<Bits>)) -> (&str, Bits) {
                    (s, bs.into())
                }

                let (s, value) = match base {
                    Base::Decimal => into_bits(DecimalBits::take(s)),
                    Base::Binary => into_bits(BinaryBits::take(s)),
                    Base::Octal => into_bits(OctalBits::take(s)),
                    Base::Hexadecimal => into_bits(HexadecimalBits::take(s)),
                };
                let length = initial_length - s.len();

                (
                    T::Number(SizedNumber {
                        size: None,
                        sign: is_signed,
                        base,
                        value,
                    }),
                    length,
                )
            }
            ('$', Some('a'..='z' | 'A'..='Z' | '_'), _) => {
                let (leftover, content) = Ident::take(&s[1..]);
                let content = &s[1..1 + content.as_str().len()];
                let length = s.len() - leftover.len();
                let content = T::DollarIdent(content);
                (content, length)
            }
            ('a'..='z' | 'A'..='Z' | '_', _, _) => {
                let (leftover, content) = Ident::take(s);
                let content = &s[..content.as_str().len()];
                let length = s.len() - leftover.len();

                // @TODO: This is quite inefficient
                let content = match content {
                    "always" => T::KeywordAlways,
                    "and" => T::KeywordAnd,
                    "assign" => T::KeywordAssign,
                    "automatic" => T::KeywordAutomatic,
                    "begin" => T::KeywordBegin,
                    "buf" => T::KeywordBuf,
                    "bufif0" => T::KeywordBufif0,
                    "bufif1" => T::KeywordBufif1,
                    "case" => T::KeywordCase,
                    "casex" => T::KeywordCaseX,
                    "casez" => T::KeywordCaseZ,
                    "cell" => T::KeywordCell,
                    "cmos" => T::KeywordCmos,
                    "config" => T::KeywordConfig,
                    "deassign" => T::KeywordDeassign,
                    "default" => T::KeywordDefault,
                    "defparam" => T::KeywordDefParam,
                    "design" => T::KeywordDesign,
                    "disable" => T::KeywordDisable,
                    "edge" => T::KeywordEdge,
                    "else" => T::KeywordElse,
                    "end" => T::KeywordEnd,
                    "endcase" => T::KeywordEndCase,
                    "endconfig" => T::KeywordEndConfig,
                    "endfunction" => T::KeywordEndFunction,
                    "endgenerate" => T::KeywordEndGenerate,
                    "endmodule" => T::KeywordEndModule,
                    "endprimitive" => T::KeywordEndPrimitive,
                    "endspecify" => T::KeywordEndSpecify,
                    "endtable" => T::KeywordEndTable,
                    "endtask" => T::KeywordEndTask,
                    "event" => T::KeywordEvent,
                    "for" => T::KeywordFor,
                    "force" => T::KeywordForce,
                    "forever" => T::KeywordForever,
                    "fork" => T::KeywordFork,
                    "function" => T::KeywordFunction,
                    "generate" => T::KeywordGenerate,
                    "genvar" => T::KeywordGenvar,
                    "highz0" => T::KeywordHighz0,
                    "highz1" => T::KeywordHighz1,
                    "if" => T::KeywordIf,
                    "ifnone" => T::KeywordIfnone,
                    "incdir" => T::KeywordIncdir,
                    "include" => T::KeywordInclude,
                    "initial" => T::KeywordInitial,
                    "inout" => T::KeywordInout,
                    "input" => T::KeywordInput,
                    "instance" => T::KeywordInstance,
                    "integer" => T::KeywordInteger,
                    "join" => T::KeywordJoin,
                    "large" => T::KeywordLarge,
                    "liblist" => T::KeywordLiblist,
                    "library" => T::KeywordLibrary,
                    "localparam" => T::KeywordLocalParam,
                    "macromodule" => T::KeywordMacroModule,
                    "medium" => T::KeywordMedium,
                    "module" => T::KeywordModule,
                    "nand" => T::KeywordNand,
                    "negedge" => T::KeywordNegedge,
                    "nmos" => T::KeywordNmos,
                    "nor" => T::KeywordNor,
                    "noshowcancelled" => T::KeywordNoShowCancelled,
                    "not" => T::KeywordNot,
                    "notif0" => T::KeywordNotif0,
                    "notif1" => T::KeywordNotif1,
                    "or" => T::KeywordOr,
                    "output" => T::KeywordOutput,
                    "parameter" => T::KeywordParameter,
                    "pmos" => T::KeywordPmos,
                    "posedge" => T::KeywordPosedge,
                    "primitive" => T::KeywordPrimitive,
                    "pull0" => T::KeywordPull0,
                    "pull1" => T::KeywordPull1,
                    "pulldown" => T::KeywordPulldown,
                    "pullup" => T::KeywordPullup,
                    "pulsestyle_onevent" => T::KeywordPulseStyleOnEvent,
                    "pulsestyle_ondetect" => T::KeywordPulseStyleOnDetect,
                    "rcmos" => T::KeywordRcmos,
                    "real" => T::KeywordReal,
                    "realtime" => T::KeywordRealtime,
                    "reg" => T::KeywordReg,
                    "release" => T::KeywordRelease,
                    "repeat" => T::KeywordRepeat,
                    "rnmos" => T::KeywordRnmos,
                    "rpmos" => T::KeywordRpmos,
                    "rtran" => T::KeywordRtran,
                    "rtranif0" => T::KeywordRtranif0,
                    "rtranif1" => T::KeywordRtranif1,
                    "scalared" => T::KeywordScalared,
                    "showcancelled" => T::KeywordShowCancelled,
                    "signed" => T::KeywordSigned,
                    "small" => T::KeywordSmall,
                    "specify" => T::KeywordSpecify,
                    "specparam" => T::KeywordSpecParam,
                    "strong0" => T::KeywordStrong0,
                    "strong1" => T::KeywordStrong1,
                    "supply0" => T::KeywordSupply0,
                    "supply1" => T::KeywordSupply1,
                    "table" => T::KeywordTable,
                    "task" => T::KeywordTask,
                    "time" => T::KeywordTime,
                    "tran" => T::KeywordTran,
                    "tranif0" => T::KeywordTranif0,
                    "tranif1" => T::KeywordTranif1,
                    "tri" => T::KeywordTri,
                    "tri0" => T::KeywordTri0,
                    "tri1" => T::KeywordTri1,
                    "triand" => T::KeywordTriand,
                    "trior" => T::KeywordTrior,
                    "trireg" => T::KeywordTrireg,
                    "unsigned1" => T::KeywordUnsigned1,
                    "use" => T::KeywordUse,
                    "uwire" => T::KeywordUwire,
                    "vectored" => T::KeywordVectored,
                    "wait" => T::KeywordWait,
                    "wand" => T::KeywordWand,
                    "weak0" => T::KeywordWeak0,
                    "weak1" => T::KeywordWeak1,
                    "while" => T::KeywordWhile,
                    "wire" => T::KeywordWire,
                    "wor" => T::KeywordWor,
                    "xnor" => T::KeywordXnor,
                    "xor" => T::KeywordXor,
                    _ => T::Ident(content),
                };

                (content, length)
            }

            _ => (T::Unknown, 1),
        };

        let leftover = &s[length..];
        Some(Consumed(leftover, content))
    }
}

impl<'a, T> Consumed<'a, T> {
    pub fn map<R, F: Fn(T) -> R>(self, f: F) -> Consumed<'a, R> {
        Consumed(self.0, f(self.1))
    }
}

impl<'a> Takeable<'a> for Ident {
    fn take(s: &'a str) -> (&'a str, Self) {
        debug_assert!(s.starts_with(|c: char| matches!(c, 'a'..='z' | 'A'..='Z' | '_')));

        fn leftover_pattern(x: char) -> bool {
            x.is_ascii_alphanumeric() || matches!(x, '_' | '$')
        }

        let start_length = s.len();

        let mut chars = s.chars();
        chars.next().unwrap();

        let leftover = chars.as_str();
        let leftover = leftover.trim_start_matches(leftover_pattern);

        let consumed_length = start_length - leftover.len();
        let consumed = &s[..consumed_length];

        (leftover, Ident::new(consumed))
    }
}
