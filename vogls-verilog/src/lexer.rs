use std::path::Path;
use std::rc::Rc;

use crate::number::{
    Base, BinaryBits, Bits, Decimal, DecimalBits, HexadecimalBits, OctalBits, Sign, Size,
};

use crate::ident::Ident;
use crate::span::Span;

pub struct TokenWalker<'a> {
    tokens: &'a [TokenKind],
    spans: &'a [Span],
    content: &'a str,
    pub offset: usize,
    path: Option<Rc<Path>>,
}

#[derive(Debug)]
pub struct Token<'a> {
    pub kind: &'a TokenKind,
    pub span: &'a Span,
}

impl<'a> TokenWalker<'a> {
    pub fn new(content: &'a str, path: Option<Rc<Path>>, buffer: &'a TokenBuffer) -> Self {
        Self {
            tokens: buffer.tokens(),
            spans: buffer.spans(),
            content,
            offset: 0,
            path,
        }
    }

    pub fn content(&self) -> &str {
        self.content
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.offset >= self.tokens.len()
    }

    pub fn cursor_offset(&self) -> usize {
        if self.offset == 0 {
            0
        } else {
            self.spans[self.offset - 1].end()
        }
    }

    pub fn span_at_cursor(&self) -> Span {
        let cursor = self.cursor_offset();
        Span::new(cursor, cursor)
    }

    pub fn get(&self, i: usize) -> Option<Token> {
        if i >= self.tokens.len() {
            return None;
        }

        Some(Token {
            kind: &self.tokens[i],
            span: &self.spans[i],
        })
    }

    pub fn try_get<E: FromLexerError>(&self, i: usize) -> Result<Token, E> {
        self.get(i)
            .ok_or_else(|| E::missing_token(self.cursor_offset()))
    }

    pub fn next_if_equals(&mut self, kind: TokenKind) -> bool {
        let Some(next) = self.next() else {
            return false;
        };
        let next = *next.kind;
        self.offset -= usize::from(next != kind);
        next == kind
    }

    pub fn next(&mut self) -> Option<Token> {
        if self.is_empty() {
            return None;
        }

        self.offset += 1;
        self.get(self.offset - 1)
    }

    pub fn try_next<E: FromLexerError>(&mut self) -> Result<Token, E> {
        if self.is_empty() {
            return Err(E::missing_token(self.cursor_offset()));
        }

        self.offset += 1;
        Ok(self.get(self.offset - 1).unwrap())
    }

    pub fn next_back(&mut self) -> Option<Token> {
        if self.offset == 0 {
            return None;
        }

        self.offset -= 1;
        self.get(self.offset)
    }

    pub fn next_expect<E: FromLexerError>(&mut self, kind: TokenKind) -> Result<&Span, E> {
        let next = self.try_next()?;
        if *next.kind != kind {
            return Err(E::unexpected_token());
        }
        Ok(next.span)
    }
}

pub struct TokenBuffer {
    tokens: Vec<TokenKind>,
    spans: Vec<Span>,
}

impl TokenBuffer {
    pub fn tokens(&self) -> &[TokenKind] {
        &self.tokens
    }

    pub fn spans(&self) -> &[Span] {
        &self.spans
    }

    pub fn tokenize(content: &str) -> Self {
        use TokenKind as T;

        let mut tokens = Vec::new();
        let mut offsets = Vec::new();

        let bytes = content.as_bytes();
        let mut i = 0;

        while let Some(&b) = bytes.get(i) {
            let (token, length) = match b {
                b' ' | b'\r' | b'\t' | b'\n' => {
                    i += 1;
                    continue;
                }

                b'(' => (T::LeftParen, 1),
                b')' => (T::RightParen, 1),
                b'[' => (T::LeftBrace, 1),
                b']' => (T::RightBrace, 1),
                b'{' => (T::LeftBracket, 1),
                b'}' => (T::RightBracket, 1),
                b':' => (T::Colon, 1),
                b';' => (T::Semicolon, 1),
                b',' => (T::Comma, 1),
                b'.' => (T::Dot, 1),
                b'+' => (T::Plus, 1),
                b'-' => (T::Minus, 1),
                b'?' => (T::QuestionMark, 1),
                b'@' => (T::AtSign, 1),
                b'#' => (T::Hash, 1),
                b'/' => (T::Slash, 1),
                b'%' => (T::Procent, 1),

                b'!' => match (bytes.get(i + 1), bytes.get(i + 2)) {
                    (Some(b'='), Some(b'=')) => (T::BangDoubleEquals, 3),
                    (Some(b'='), _) => (T::BangEquals, 2),
                    (_, _) => (T::Bang, 1),
                },
                b'~' => match bytes.get(i + 1) {
                    Some(b'&') => (T::TildeAmpersand, 2),
                    Some(b'|') => (T::TildeBar, 2),
                    Some(b'^') => (T::TildeCaret, 2),
                    _ => (T::Tilde, 1),
                },
                b'^' => match bytes.get(i + 1) {
                    Some(b'~') => (T::CaretTilde, 2),
                    _ => (T::Caret, 1),
                },
                b'&' => match bytes.get(i + 1) {
                    Some(b'&') => (T::DoubleAmpersand, 2),
                    _ => (T::Ampersand, 1),
                },
                b'|' => match bytes.get(i + 1) {
                    Some(b'|') => (T::DoubleBar, 2),
                    _ => (T::Bar, 1),
                },
                b'*' => match bytes.get(i + 1) {
                    Some(b'*') => (T::DoubleStar, 2),
                    _ => (T::Star, 1),
                },
                b'=' => match (bytes.get(i + 1), bytes.get(i + 2)) {
                    (Some(b'='), Some(b'=')) => (T::TripleEquals, 3),
                    (Some(b'='), _) => (T::DoubleEquals, 2),
                    (_, _) => (T::Equals, 1),
                },
                b'>' => match (bytes.get(i + 1), bytes.get(i + 2)) {
                    (Some(b'>'), Some(b'>')) => (T::TripleGreaterThan, 3),
                    (Some(b'>'), _) => (T::DoubleGreaterThan, 2),
                    (Some(b'='), _) => (T::GreaterThanEquals, 2),
                    (_, _) => (T::GreaterThan, 1),
                },
                b'<' => match (bytes.get(i + 1), bytes.get(i + 2)) {
                    (Some(b'<'), Some(b'<')) => (T::TripleLessThan, 3),
                    (Some(b'<'), _) => (T::DoubleLessThan, 2),
                    (Some(b'='), _) => (T::LessThanEquals, 2),
                    (_, _) => (T::LessThan, 1),
                },
                b'"' => {
                    let mut token = T::String;
                    let mut j = i + 1;
                    let end_offset = loop {
                        let Some(b) = bytes.get(j) else {
                            token = T::Unknown;
                            break j;
                        };

                        match b {
                            b'"' => break j + 1,
                            // @TODO: Better escaping.
                            b'\\' => j += 2,
                            _ => j += 1,
                        }
                    };
                    (token, end_offset - i)
                }
                b'0'..=b'9' => {
                    let s = &content[i..];
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

                    // @TODO: Do without materializing
                    if s.starts_with('0')
                        || !is_valid_prefix(s.trim_start_matches(|c: char| matches!(c, '0'..='9')))
                    {
                        let (s, _) = Decimal::take(s);
                        let length = initial_length - s.len();
                        (T::Decimal, length)
                    } else {
                        let (s, _) = Size::take(s);
                        debug_assert!(s.starts_with('\''));
                        let s = &s[1..];
                        let (s, _) = Sign::take(s);
                        let (s, base) = Base::take(s);

                        fn into_bits((s, bs): (&str, impl Into<Bits>)) -> (&str, Bits) {
                            (s, bs.into())
                        }

                        let (s, _) = match base {
                            Base::Decimal => into_bits(DecimalBits::take(s)),
                            Base::Binary => into_bits(BinaryBits::take(s)),
                            Base::Octal => into_bits(OctalBits::take(s)),
                            Base::Hexadecimal => into_bits(HexadecimalBits::take(s)),
                        };

                        let length = initial_length - s.len();
                        (T::Number, length)
                    }
                }
                b'\'' => {
                    let mut offset = i;
                    if matches!(bytes.get(offset + 1), Some(b's' | b'S')) {
                        offset += 1;
                    }

                    // @TODO: Do without materializing
                    match bytes.get(offset + 1) {
                        Some(b'D' | b'd') => (
                            T::Number,
                            content.len() - i - DecimalBits::take(&content[offset..]).0.len(),
                        ),
                        Some(b'B' | b'b') => (
                            T::Number,
                            content.len() - i - BinaryBits::take(&content[offset..]).0.len(),
                        ),
                        Some(b'O' | b'o') => (
                            T::Number,
                            content.len() - i - OctalBits::take(&content[offset..]).0.len(),
                        ),
                        Some(b'X' | b'x') => (
                            T::Number,
                            content.len() - i - HexadecimalBits::take(&content[offset..]).0.len(),
                        ),
                        _ => (T::Unknown, 1),
                    }
                }
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                    let length = ident_length(&content[i..]);
                    let word = &content[i..i + length];
                    let token = match word {
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
                        _ => T::Ident,
                    };
                    (token, length)
                }
                b'$' if matches!(bytes.get(i + 1), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_')) => {
                    (T::DollarIdent, 1 + ident_length(&content[i + 1..]))
                }
                _ => (T::Unknown, 1),
            };

            tokens.push(token);
            offsets.push(Span::new(i, i + length));
            i += length;
        }

        Self {
            tokens,
            spans: offsets,
        }
    }
}

fn ident_length(s: &str) -> usize {
    debug_assert!(s.starts_with(|c: char| matches!(c, 'a'..='z' | 'A'..='Z' | '_')));

    fn leftover_pattern(x: char) -> bool {
        x.is_ascii_alphanumeric() || matches!(x, '_' | '$')
    }

    let start_length = s.len();

    let mut chars = s.chars();
    chars.next().unwrap();

    let leftover = chars.as_str();
    let leftover = leftover.trim_start_matches(leftover_pattern);

    start_length - leftover.len()
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

pub trait FromLexerError {
    fn missing_token(at: usize) -> Self;
    fn unexpected_token() -> Self;
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

macro_rules! define_tokens {
    (
        $(
        $(#[$attr:meta])*
        $ident:ident = $example:literal,
        )+
    ) => {

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u8)]
        pub enum TokenKind {
            $(
            $(#[$attr])*
            $ident,
            )+
            Unknown,
        }

        #[cfg(test)]
        #[test]
        fn tokenizer_examples() {
            $(
            let consumed = TokenBuffer::tokenize($example);
            assert_eq!(consumed.tokens().len(), 1);
            assert_eq!(consumed.tokens()[0], TokenKind::$ident, "Example \"{}\" of token {} had the invalid kind", $example, stringify!($ident));
            )+
        }
    };
}

define_tokens! {
    Ident = "abc",
    DollarIdent = "$abc",
    String = "\"this is a string\"",
    Number = "50'd50",
    Decimal = "50",

    /// `(`
    LeftParen = "(",
    /// `)`
    RightParen = ")",
    /// `[`
    LeftBrace = "[",
    /// `]`
    RightBrace = "]",
    /// `{`
    LeftBracket = "{",
    /// `}`
    RightBracket = "}",

    Semicolon = ";",
    Colon = ":",
    Comma = ",",
    Dot = ".",

    // Operators
    Plus = "+",
    Minus = "-",
    Bang = "!",
    Tilde = "~",
    Ampersand = "&",
    TildeAmpersand = "~&",
    Bar = "|",
    TildeBar = "~|",
    Caret = "^",
    TildeCaret = "~^",
    CaretTilde = "^~",
    Star = "*",
    Slash = "/",
    Procent = "%",
    DoubleEquals = "==",
    BangEquals = "!=",
    TripleEquals = "===",
    BangDoubleEquals = "!==",
    DoubleAmpersand = "&&",
    DoubleBar = "||",
    DoubleStar = "**",
    LessThan = "<",
    LessThanEquals = "<=",
    GreaterThan = ">",
    GreaterThanEquals = ">=",
    DoubleGreaterThan = ">>",
    DoubleLessThan = "<<",
    TripleGreaterThan = ">>>",
    TripleLessThan = "<<<",
    QuestionMark = "?",

    Equals = "=",
    AtSign = "@",
    Hash = "#",

    // Keywords
    KeywordAlways = "always",
    KeywordAnd = "and",
    KeywordAssign = "assign",
    KeywordAutomatic = "automatic",
    KeywordBegin = "begin",
    KeywordBuf = "buf",
    KeywordBufif0 = "bufif0",
    KeywordBufif1 = "bufif1",
    KeywordCase = "case",
    KeywordCaseX = "casex",
    KeywordCaseZ = "casez",
    KeywordCell = "cell",
    KeywordCmos = "cmos",
    KeywordConfig = "config",
    KeywordDeassign = "deassign",
    KeywordDefault = "default",
    KeywordDefParam = "defparam",
    KeywordDesign = "design",
    KeywordDisable = "disable",
    KeywordEdge = "edge",
    KeywordElse = "else",
    KeywordEnd = "end",
    KeywordEndCase = "endcase",
    KeywordEndConfig = "endconfig",
    KeywordEndFunction = "endfunction",
    KeywordEndGenerate = "endgenerate",
    KeywordEndModule = "endmodule",
    KeywordEndPrimitive = "endprimitive",
    KeywordEndSpecify = "endspecify",
    KeywordEndTable = "endtable",
    KeywordEndTask = "endtask",
    KeywordEvent = "event",
    KeywordFor = "for",
    KeywordForce = "force",
    KeywordForever = "forever",
    KeywordFork = "fork",
    KeywordFunction = "function",
    KeywordGenerate = "generate",
    KeywordGenvar = "genvar",
    KeywordHighz0 = "highz0",
    KeywordHighz1 = "highz1",
    KeywordIf = "if",
    KeywordIfnone = "ifnone",
    KeywordIncdir = "incdir",
    KeywordInclude = "include",
    KeywordInitial = "initial",
    KeywordInout = "inout",
    KeywordInput = "input",
    KeywordInstance = "instance",
    KeywordInteger = "integer",
    KeywordJoin = "join",
    KeywordLarge = "large",
    KeywordLiblist = "liblist",
    KeywordLibrary = "library",
    KeywordLocalParam = "localparam",
    KeywordMacroModule = "macromodule",
    KeywordMedium = "medium",
    KeywordModule = "module",
    KeywordNand = "nand",
    KeywordNegedge = "negedge",
    KeywordNmos = "nmos",
    KeywordNor = "nor",
    KeywordNoShowCancelled = "noshowcancelled",
    KeywordNot = "not",
    KeywordNotif0 = "notif0",
    KeywordNotif1 = "notif1",
    KeywordOr = "or",
    KeywordOutput = "output",
    KeywordParameter = "parameter",
    KeywordPmos = "pmos",
    KeywordPosedge = "posedge",
    KeywordPrimitive = "primitive",
    KeywordPull0 = "pull0",
    KeywordPull1 = "pull1",
    KeywordPulldown = "pulldown",
    KeywordPullup = "pullup",
    KeywordPulseStyleOnEvent = "pulsestyle_onevent",
    KeywordPulseStyleOnDetect = "pulsestyle_ondetect",
    KeywordRcmos = "rcmos",
    KeywordReal = "real",
    KeywordRealtime = "realtime",
    KeywordReg = "reg",
    KeywordRelease = "release",
    KeywordRepeat = "repeat",
    KeywordRnmos = "rnmos",
    KeywordRpmos = "rpmos",
    KeywordRtran = "rtran",
    KeywordRtranif0 = "rtranif0",
    KeywordRtranif1 = "rtranif1",
    KeywordScalared = "scalared",
    KeywordShowCancelled = "showcancelled",
    KeywordSigned = "signed",
    KeywordSmall = "small",
    KeywordSpecify = "specify",
    KeywordSpecParam = "specparam",
    KeywordStrong0 = "strong0",
    KeywordStrong1 = "strong1",
    KeywordSupply0 = "supply0",
    KeywordSupply1 = "supply1",
    KeywordTable = "table",
    KeywordTask = "task",
    KeywordTime = "time",
    KeywordTran = "tran",
    KeywordTranif0 = "tranif0",
    KeywordTranif1 = "tranif1",
    KeywordTri = "tri",
    KeywordTri0 = "tri0",
    KeywordTri1 = "tri1",
    KeywordTriand = "triand",
    KeywordTrior = "trior",
    KeywordTrireg = "trireg",
    KeywordUnsigned1 = "unsigned1",
    KeywordUse = "use",
    KeywordUwire = "uwire",
    KeywordVectored = "vectored",
    KeywordWait = "wait",
    KeywordWand = "wand",
    KeywordWeak0 = "weak0",
    KeywordWeak1 = "weak1",
    KeywordWhile = "while",
    KeywordWire = "wire",
    KeywordWor = "wor",
    KeywordXnor = "xnor",
    KeywordXor = "xor",
}
