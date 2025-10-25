use crate::number::{Decimal, SizedNumber};
use crate::span::Span;

use super::Token;

impl<'a> Token<'a> {
    pub fn span(&self) -> Span {
        self.span
    }

    pub fn content(&self) -> &TokenContent<'a> {
        &self.content
    }

    pub fn kind(&self) -> TokenKind {
        self.content.kind()
    }

    pub fn is_kind(&self, kind: TokenKind) -> bool {
        self.content.kind() == kind
    }

    pub fn take(self) -> (TokenContent<'a>, Span) {
        (self.content, self.span)
    }
}

macro_rules! define_tokens {
    (@i $_:ty) => { _ };
    (
        <$lt:lifetime> {
            $(
            $(#[$attr:meta])*
            $ident:ident$(($content:ty))? = $example:literal,
            )+
        }
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum TokenContent<$lt> {
            $(
            $(#[$attr])*
            $ident$(($content))?,
            )+
            Unknown,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum TokenKind {
            $(
            $(#[$attr])*
            $ident,
            )+
            Unknown,
        }

        impl<$lt> TokenContent<$lt> {
            #[inline(always)]
            pub fn kind(&self) -> TokenKind {
                match self {
                    $(Self::$ident$((define_tokens!(@i $content)))? => TokenKind::$ident,)+
                    Self::Unknown => TokenKind::Unknown,
                }
            }
        }

        #[cfg(test)]
        #[test]
        fn token_examples() {
            $(
            let consumed = TokenContent::consume($example);
            assert!(consumed.is_some(), "Example \"{}\" of token {} failed to generate any result", $example, stringify!($ident));
            let consumed = consumed.unwrap();
            assert_eq!(consumed.1.kind(), TokenKind::$ident, "Example \"{}\" of token {} had the invalid kind", $example, stringify!($ident));
            assert_eq!(consumed.0, "", "Example \"{}\" of token {} had leftover text", $example, stringify!($ident));
            )+
        }

        #[cfg(test)]
        #[test]
        fn tokenizer_examples() {
            use super::tokenizer::TokenBuffer;
            $(
            let consumed = TokenBuffer::tokenize($example);
            assert_eq!(consumed.tokens().len(), 1);
            assert_eq!(consumed.tokens()[0], TokenKind::$ident, "Example \"{}\" of token {} had the invalid kind", $example, stringify!($ident));
            )+
        }
    };
}

define_tokens! {
    <'a> {
        Ident(&'a str) = "abc",
        DollarIdent(&'a str) = "$abc",
        String(&'a str) = "\"this is a string\"",
        Number(SizedNumber) = "50'd50",
        Decimal(Decimal) = "50",

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
}
