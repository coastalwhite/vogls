use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_utils::{TableMap, VgHashMap, new_table_key};
use vogls_world::{World, WorldError};

use crate::number::{
    Base, skip_binary, skip_decimal, skip_hexadecimal, skip_octal, skip_scientific_exp, skip_sign,
    take_base,
};

use crate::span::Span;

pub type FileIdx = u32;
#[derive(Default, Clone)]
pub struct Tokenized {
    pub tokens: Vec<Token>,
    pub spans: Vec<Span>,
    pub file_idxs: Vec<FileIdx>,
    pub contents: Vec<Arc<str>>,
    pub paths: Vec<Option<Arc<Path>>>,
    pub file_line_offsets: Vec<Vec<usize>>,
}

new_table_key! { pub struct MacroKey; }

#[derive(Default, Clone)]
pub struct Macros {
    idents: IdentTable,
    macros: TableMap<MacroKey, IdentId, Macro>,
}

impl Macros {
    pub fn define(&mut self, name: &str, item: Macro) -> Option<Macro> {
        let name = self.idents.get_or_insert(name);
        self.define_with_ident_id(name, item)
    }

    fn define_with_ident_id(&mut self, name: IdentId, item: Macro) -> Option<Macro> {
        let (_, old_macro) = self.macros.insert(name, item);
        old_macro
    }

    pub fn undefine(&mut self, name: &str) -> Option<Macro> {
        let name = self.idents.get_or_insert(name);
        self.macros.unlink(name).map(|(_, item)| item)
    }

    fn find(&self, name: &str) -> Option<(MacroKey, &Macro)> {
        let name = self.idents.get(name)?;
        self.find_by_ident_id(name)
    }

    fn find_by_ident_id(&self, name: IdentId) -> Option<(MacroKey, &Macro)> {
        self.macros.get(name)
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.idents
            .get(name)
            .is_some_and(|k| self.macros.contains_key(k))
    }
}

#[derive(Default, Clone)]
pub struct Macro {
    tokens: Vec<Token>,
    spans: Vec<Span>,
    file: Vec<FileIdx>,
    args: Option<(Vec<(usize, usize)>, usize)>,
}

#[derive(Debug)]
pub struct TokenizeError {
    line: u64,
    file: Option<Arc<Path>>,
    reason: TokenizeErrorReason,
}

impl fmt::Display for TokenizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { line, file, reason } = self;
        match file {
            None => f.write_str("<unknown>")?,
            Some(file) => file.display().fmt(f)?,
        }
        write!(f, ":{line}: {reason}")
    }
}
impl fmt::Display for TokenizeErrorReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FailedToOpenFile(path, err) => {
                write!(f, "failed to open {}. Reason: {err}", path.display())
            }
            Self::PreprocessorMismatch => f.write_str("else/endif/elsif without matching if"),
            Self::IncludeInMacro => {
                f.write_str("Include preprocessor directive in a macro definition")
            }
            Self::NestedDefine => f.write_str("Nested preprocessor definition statement"),
            Self::DefineWithoutName => f.write_str("Definition without name"),
            Self::ExpectedConditionalIdent => {
                f.write_str("Expected macro ident for ifdef/ifndef/elsif")
            }
            Self::UnclosedFunctionMacro(macro_name) => {
                write!(f, "Function macro '{macro_name}' is unclosed")
            }
            Self::MalformedMacroArguments(macro_name) => {
                write!(
                    f,
                    "Function macro '{macro_name}' has malformed argument definitions"
                )
            }
            Self::MacroArgumentMismatch(macro_name) => {
                write!(
                    f,
                    "Function macro '{macro_name}' has different number of arguments"
                )
            }
            Self::FunctionMacroWithoutArgs(macro_name) => {
                write!(
                    f,
                    "Function macro '{macro_name}' is called without arguments"
                )
            }
            Self::ZeroArgMacroFunction(macro_name) => {
                write!(
                    f,
                    "Function macro '{macro_name}' cannot have zero arguments"
                )
            }
            Self::NestedFunctionMacro(macro_name, nested) => {
                write!(
                    f,
                    "Function macro '{macro_name}' calls another function macro '{nested}', which is not currently allowed"
                )
            }
        }
    }
}

impl std::error::Error for TokenizeError {}

#[derive(Debug)]
pub enum TokenizeErrorReason {
    FailedToOpenFile(PathBuf, WorldError),
    IncludeInMacro,
    NestedDefine,
    DefineWithoutName,
    ExpectedConditionalIdent,
    PreprocessorMismatch,
    MalformedMacroArguments(String),
    UnclosedFunctionMacro(String),
    MacroArgumentMismatch(String),
    FunctionMacroWithoutArgs(String),
    ZeroArgMacroFunction(String),
    NestedFunctionMacro(String, String),
}

impl Tokenized {
    pub fn tokenize_with_macros(
        content: Arc<str>,
        path: Option<Arc<Path>>,
        world: &mut dyn World,
        macros: &mut Macros,
    ) -> Result<Self, Box<TokenizeError>> {
        let mut ts = Self {
            tokens: Vec::new(),
            spans: Vec::new(),
            file_idxs: Vec::new(),
            paths: Vec::new(),
            contents: Vec::new(),
            file_line_offsets: Vec::new(),
        };
        ts.append_tokenize_with_macros(content, path, world, macros)?;
        Ok(ts)
    }

    pub fn append_tokenize_with_macros(
        &mut self,
        content: Arc<str>,
        path: Option<Arc<Path>>,
        world: &mut dyn World,
        macros: &mut Macros,
    ) -> Result<(), Box<TokenizeError>> {
        let Self {
            tokens,
            spans,
            file_idxs,
            contents,
            paths,
            file_line_offsets,
        } = self;

        use Token as T;

        // @Performance: bit-field
        #[derive(Clone, Copy)]
        struct IfState {
            has_been_taken_before: bool,
            has_else: bool,
        }

        struct MacroArgs {
            name: IdentId,
            depth: usize,
        }

        enum LexState {
            Base,
            Macro(MacroItem),
            MacroArgs(MacroArgs),
        }

        // Stack and depth for the preprocessor `ifdef`, `ifdef`, `elsif`, `else` and `endif.
        //
        // If `if_untaken_depth >= if_stack.len()`, it means the current tokens are being included
        // in the output stream. If `if_untaken_depth < if_stack.len()`, the tokens are tokenized
        // but not saved.
        let mut if_stack = Vec::<IfState>::new();
        let mut if_untaken_depth = 0;

        // Scratch buffers for function macros.
        let mut scratch_funcmacro_tokens = Vec::new();
        let mut scratch_funcmacro_spans = Vec::new();
        let mut scratch_funcmacro_file_idxs = Vec::new();
        let mut scratch_funcmacro_splits = Vec::new();

        struct MacroItem {
            name: IdentId,
            arguments: VgHashMap<IdentId, usize>,
            argument_positions: Vec<(usize, usize)>,
            has_args: bool,
        }

        struct LexItem {
            file_idx: u32,

            start: usize,
            i: usize,
            end_offset: usize,

            state: LexState,
        }

        let mut lex_stack = Vec::new();
        lex_stack.push(LexItem {
            file_idx: paths.len() as u32,
            start: 0,
            i: 0,
            end_offset: content.len(),
            state: LexState::Base,
        });
        contents.push(content);
        paths.push(path);
        file_line_offsets.push(vec![0]);

        'lex_stack: while let Some(LexItem {
            file_idx,
            start,
            mut i,
            end_offset,
            mut state,
        }) = lex_stack.pop()
        {
            let content = contents[file_idx as usize].clone();
            let bytes = content[..end_offset].as_bytes();
            while let Some(&b) = bytes.get(i) {
                let (token, length) = match b {
                    b'\n' => {
                        i += 1;
                        file_line_offsets[file_idx as usize].push(i);
                        continue;
                    }
                    b' ' | b'\r' | b'\t' => {
                        i += 1;
                        continue;
                    }

                    b'(' => match bytes.get(i + 1) {
                        Some(b'*') if matches!(bytes.get(i + 2), Some(b')')) => {
                            (T::LeftParenStarRightParen, 3)
                        }
                        Some(b'*') => (T::LeftParenStar, 2),
                        _ => {
                            if let LexState::MacroArgs(a) = &mut state {
                                a.depth += 1;
                            }
                            (T::LeftParen, 1)
                        }
                    },
                    b')' => {
                        if let LexState::MacroArgs(a) = &mut state {
                            a.depth -= 1;
                            if a.depth == 0 {
                                break;
                            }
                        }
                        (T::RightParen, 1)
                    }
                    b'[' => (T::LeftBrace, 1),
                    b']' => (T::RightBrace, 1),
                    b'{' => (T::LeftBracket, 1),
                    b'}' => (T::RightBracket, 1),
                    b':' => (T::Colon, 1),
                    b';' => (T::Semicolon, 1),
                    b',' => (T::Comma, 1),
                    b'.' => (T::Dot, 1),
                    b'?' => (T::QuestionMark, 1),
                    b'@' => (T::AtSign, 1),
                    b'#' => (T::Hash, 1),
                    b'%' => (T::Procent, 1),

                    b'+' => match bytes.get(i + 1) {
                        Some(b':') => (T::PlusColon, 2),
                        _ => (T::Plus, 1),
                    },
                    b'-' => match bytes.get(i + 1) {
                        Some(b':') => (T::MinusColon, 2),
                        _ => (T::Minus, 1),
                    },
                    b'/' => match bytes.get(i + 1) {
                        // Line comments
                        Some(b'/') => {
                            i = bytes[i + 2..]
                                .iter()
                                .position(|c: &u8| *c == b'\n')
                                .map_or(bytes.len(), |j| i + 2 + j);
                            continue;
                        }

                        // Block comments
                        Some(b'*') => {
                            i += 2;
                            let mut prev_was_star = false;
                            while let Some(b) = bytes.get(i) {
                                let done = prev_was_star && *b == b'/';
                                prev_was_star = *b == b'*';

                                i += 1;
                                if done {
                                    break;
                                }

                                if *b == b'\n' {
                                    file_line_offsets[file_idx as usize].push(i);
                                }
                            }
                            continue;
                        }
                        _ => (T::Slash, 1),
                    },
                    b'\\' => {
                        let end = bytes[i + 1..]
                            .iter()
                            .position(|b| b.is_ascii_whitespace())
                            .unwrap_or(bytes.len() - i - 1);
                        (T::Ident, end + 1)
                    }
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
                        Some(b'>') => (T::StarGreaterThan, 2),
                        Some(b')') => (T::StarRightParen, 2),
                        _ => (T::Star, 1),
                    },
                    b'=' => match (bytes.get(i + 1), bytes.get(i + 2)) {
                        (Some(b'='), Some(b'=')) => (T::TripleEquals, 3),
                        (Some(b'='), _) => (T::DoubleEquals, 2),
                        (Some(b'>'), _) => (T::EqualsGreaterThan, 2),
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
                    b'"' => match str_length(&content[i..]) {
                        None => (T::Unknown, 1),
                        Some(l) => (T::String, l),
                    },
                    b'0'..=b'9' => {
                        let mut offset = i + 1;
                        skip_decimal(bytes, &mut offset);
                        fn is_valid_prefix(s: &str) -> bool {
                            let pattern = &['D', 'd', 'B', 'b', 'O', 'o', 'H', 'h'];
                            s.starts_with('\'')
                                && if s[1..].starts_with(['S', 's']) {
                                    s[2..].starts_with(pattern)
                                } else {
                                    s[1..].starts_with(pattern)
                                }
                        }

                        if bytes[i] == b'0' || !is_valid_prefix(&content[offset..]) {
                            match bytes.get(offset) {
                                Some(b'e' | b'E') if skip_scientific_exp(bytes, &mut offset) => {
                                    (T::Real, offset - i)
                                }
                                Some(b'.')
                                    if bytes
                                        .get(offset + 1)
                                        .is_some_and(|b| b.is_ascii_digit() || *b == b'_') =>
                                {
                                    offset += 1;
                                    skip_decimal(bytes, &mut offset);
                                    skip_scientific_exp(bytes, &mut offset);
                                    (T::Real, offset - i)
                                }
                                _ => (T::Decimal, offset - i),
                            }
                        } else {
                            offset += 1;
                            skip_sign(bytes, &mut offset);
                            let f = match take_base(bytes, &mut offset).unwrap() {
                                Base::Decimal => skip_decimal,
                                Base::Binary => skip_binary,
                                Base::Octal => skip_octal,
                                Base::Hexadecimal => skip_hexadecimal,
                            };
                            skip_whitespace(bytes, &mut offset);
                            f(bytes, &mut offset);
                            (T::Number, offset - i)
                        }
                    }
                    b'\'' => {
                        let mut offset = i + 1;
                        skip_sign(bytes, &mut offset);
                        match take_base(bytes, &mut offset) {
                            None => (T::Unknown, 1),
                            Some(base) => {
                                skip_whitespace(bytes, &mut offset);
                                let f = match base {
                                    Base::Decimal => skip_decimal,
                                    Base::Binary => skip_binary,
                                    Base::Octal => skip_octal,
                                    Base::Hexadecimal => skip_hexadecimal,
                                };
                                f(bytes, &mut offset);
                                (T::Number, offset - i)
                            }
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
                            "unsigned" => T::KeywordUnsigned,
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
                            _ => {
                                if let LexState::Macro(preprocessor_macro) = &mut state
                                    && let Some(arg_ident_id) = macros.idents.get(word)
                                    && let Some(arg_idx) =
                                        preprocessor_macro.arguments.get_mut(&arg_ident_id)
                                {
                                    preprocessor_macro
                                        .argument_positions
                                        .push((*arg_idx, tokens.len() - start));
                                    i += word.len();
                                    continue;
                                }

                                T::Ident
                            }
                        };
                        (token, length)
                    }
                    b'$' if matches!(bytes.get(i + 1), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_')) => {
                        (T::DollarIdent, 1 + ident_length(&content[i + 1..]))
                    }

                    // Compiler directives
                    b'`' if matches!(bytes.get(i + 1), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_')) => {
                        let directive_length = ident_length(&content[i + 1..]);
                        let directive = &content[i + 1..][..directive_length];

                        match directive {
                            // Preprocessor macro definition
                            "define" => {
                                if !matches!(state, LexState::Base) {
                                    return Err(Box::new(TokenizeError {
                                        line: self.file_line_offsets[file_idx as usize].len()
                                            as u64,
                                        file: self.paths[file_idx as usize].clone(),
                                        reason: TokenizeErrorReason::NestedDefine,
                                    }));
                                }

                                let mut j = i + 1 + directive_length;
                                skip_sameline_whitespace(&content, &mut j);
                                if bytes.get(j).is_none_or(|&b| !is_fst_ident_byte(b)) {
                                    return Err(Box::new(TokenizeError {
                                        line: self.file_line_offsets[file_idx as usize].len()
                                            as u64,
                                        file: self.paths[file_idx as usize].clone(),
                                        reason: TokenizeErrorReason::DefineWithoutName,
                                    }));
                                }

                                let name_length = ident_length(&content[j..]);
                                let name = &content[j..][..name_length];
                                // @TODO: Disallow overwriting compiler intrinsics.
                                j += name_length;

                                let mut arguments = VgHashMap::default();
                                let mut has_args = false;
                                if content[j..].starts_with("(") {
                                    has_args = true;
                                    j += 1;

                                    skip_sameline_whitespace(&content, &mut j);
                                    loop {
                                        let Some(&b) = bytes.get(j) else {
                                            return Err(Box::new(TokenizeError {
                                                line: self.file_line_offsets[file_idx as usize]
                                                    .len()
                                                    as u64,
                                                file: self.paths[file_idx as usize].clone(),
                                                reason:
                                                    TokenizeErrorReason::MalformedMacroArguments(
                                                        name.to_string(),
                                                    ),
                                            }));
                                        };
                                        if b == b')' {
                                            break;
                                        }

                                        if !arguments.is_empty() {
                                            if b != b',' {
                                                return Err(Box::new(TokenizeError {
                                                    line: self.file_line_offsets[file_idx as usize]
                                                        .len()
                                                        as u64,
                                                    file: self.paths[file_idx as usize].clone(),
                                                    reason:
                                                        TokenizeErrorReason::MalformedMacroArguments(
                                                            name.to_string(),
                                                        ),
                                                }));
                                            }
                                            j += 1;
                                            skip_sameline_whitespace(&content, &mut j);
                                        }
                                        if bytes.get(j).is_none_or(|&b| !is_fst_ident_byte(b)) {
                                            return Err(Box::new(TokenizeError {
                                                line: self.file_line_offsets[file_idx as usize]
                                                    .len()
                                                    as u64,
                                                file: self.paths[file_idx as usize].clone(),
                                                reason:
                                                    TokenizeErrorReason::MalformedMacroArguments(
                                                        name.to_string(),
                                                    ),
                                            }));
                                        }

                                        let argument_length = ident_length(&content[j..]);
                                        let argument = &content[j..][..argument_length];
                                        let argument = macros.idents.get_or_insert(&argument);
                                        let argument_idx = arguments.len();
                                        arguments.insert(argument, argument_idx);
                                        j += argument_length;
                                        skip_sameline_whitespace(&content, &mut j);
                                    }

                                    j += 1;

                                    if arguments.is_empty() {
                                        return Err(Box::new(TokenizeError {
                                            line: self.file_line_offsets[file_idx as usize].len()
                                                as u64,
                                            file: self.paths[file_idx as usize].clone(),
                                            reason: TokenizeErrorReason::ZeroArgMacroFunction(
                                                name.to_string(),
                                            ),
                                        }));
                                    }
                                }

                                let mut is_escaped = false;
                                let end = bytes[j..]
                                    .iter()
                                    .position(|&b| {
                                        let is_unescaped_nl = b == b'\n' && !is_escaped;
                                        is_escaped = b == b'\\';
                                        is_unescaped_nl
                                    })
                                    .map_or(bytes.len(), |e| e + j);

                                if if_untaken_depth < if_stack.len() {
                                    i = end;
                                    continue;
                                }

                                lex_stack.push(LexItem {
                                    file_idx,
                                    start,
                                    i: end,
                                    end_offset,
                                    state: LexState::Base,
                                });
                                lex_stack.push(LexItem {
                                    file_idx,
                                    start: tokens.len(),
                                    i: j,
                                    end_offset: end,
                                    state: LexState::Macro(MacroItem {
                                        name: macros.idents.get_or_insert(name),
                                        arguments,
                                        argument_positions: Vec::new(),
                                        has_args,
                                    }),
                                });
                                continue 'lex_stack;
                            }
                            "undef" => {
                                let mut j = i + 1 + directive_length;
                                skip_sameline_whitespace(&content, &mut j);
                                if bytes.get(j).is_none_or(|&b| !is_fst_ident_byte(b)) {
                                    return Err(Box::new(TokenizeError {
                                        line: self.file_line_offsets[file_idx as usize].len()
                                            as u64,
                                        file: self.paths[file_idx as usize].clone(),
                                        reason: TokenizeErrorReason::DefineWithoutName,
                                    }));
                                }

                                let name_length = ident_length(&content[j..]);
                                let name = &content[j..][..name_length];
                                i = j + name_length;

                                // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 352
                                // An attempt to undefine a text macro that was not previously
                                // defined using a `define compiler directive can result in a
                                // warning.
                                if if_untaken_depth >= if_stack.len() {
                                    _ = macros.undefine(name);
                                }
                                continue;
                            }

                            // Preprocessor control-flow
                            "ifdef" | "ifndef" => {
                                let mut j = i + 1 + directive_length;
                                skip_sameline_whitespace(&content, &mut j);
                                if bytes.get(j).is_none_or(|&b| !is_fst_ident_byte(b)) {
                                    return Err(Box::new(TokenizeError {
                                        line: self.file_line_offsets[file_idx as usize].len()
                                            as u64,
                                        file: self.paths[file_idx as usize].clone(),
                                        reason: TokenizeErrorReason::ExpectedConditionalIdent,
                                    }));
                                }

                                let name_length = ident_length(&content[j..]);
                                let name = &content[j..][..name_length];
                                i = j + name_length;

                                let mut if_item = IfState {
                                    has_been_taken_before: false,
                                    has_else: false,
                                };
                                if if_untaken_depth >= if_stack.len() {
                                    let is_taken =
                                        macros.is_defined(name) ^ (directive == "ifndef");
                                    if_item.has_been_taken_before = is_taken;
                                    if is_taken {
                                        if_untaken_depth += 1;
                                    } else {
                                        if_untaken_depth = if_stack.len();
                                    }
                                }
                                if_stack.push(if_item);
                                continue;
                            }
                            "elsif" => {
                                let mut j = i + 1 + directive_length;
                                skip_sameline_whitespace(&content, &mut j);
                                if bytes.get(j).is_none_or(|&b| !is_fst_ident_byte(b)) {
                                    return Err(Box::new(TokenizeError {
                                        line: self.file_line_offsets[file_idx as usize].len()
                                            as u64,
                                        file: self.paths[file_idx as usize].clone(),
                                        reason: TokenizeErrorReason::ExpectedConditionalIdent,
                                    }));
                                }

                                let name_length = ident_length(&content[j..]);
                                let name = &content[j..][..name_length];
                                i = j + name_length;

                                if if_untaken_depth >= if_stack.len() {
                                    let Some(mut if_item) = if_stack.pop() else {
                                        return Err(Box::new(TokenizeError {
                                            line: self.file_line_offsets[file_idx as usize].len()
                                                as u64,
                                            file: self.paths[file_idx as usize].clone(),
                                            reason: TokenizeErrorReason::PreprocessorMismatch,
                                        }));
                                    };
                                    let is_taken = !if_item.has_been_taken_before
                                        && macros.is_defined(name) ^ (directive == "ifndef");
                                    if_item.has_been_taken_before = is_taken;
                                    if is_taken {
                                        if_untaken_depth += 1;
                                    } else {
                                        if_untaken_depth = if_stack.len();
                                    }
                                    if_stack.push(if_item);
                                }
                                continue;
                            }
                            "else" => {
                                i += 1 + directive_length;

                                // Take i.f.f. if_untaken_depth == if_stack - 1 && [no-branches-taken-before]
                                // Untake i.f.f. if_untaken_depth > if_stack

                                let Some(mut if_item) =
                                    if_stack.pop().filter(|item| !item.has_else)
                                else {
                                    return Err(Box::new(TokenizeError {
                                        line: self.file_line_offsets[file_idx as usize].len()
                                            as u64,
                                        file: self.paths[file_idx as usize].clone(),
                                        reason: TokenizeErrorReason::PreprocessorMismatch,
                                    }));
                                };

                                if if_untaken_depth > if_stack.len() {
                                    if_untaken_depth = if_stack.len();
                                } else if if_untaken_depth == if_stack.len()
                                    && !if_item.has_been_taken_before
                                {
                                    if_untaken_depth = if_stack.len() + 1;
                                }

                                if_item.has_been_taken_before = true;
                                if_item.has_else = true;

                                if_stack.push(if_item);
                                continue;
                            }
                            "endif" => {
                                i += 1 + directive_length;
                                let Some(_) = if_stack.pop() else {
                                    return Err(Box::new(TokenizeError {
                                        line: self.file_line_offsets[file_idx as usize].len()
                                            as u64,
                                        file: self.paths[file_idx as usize].clone(),
                                        reason: TokenizeErrorReason::PreprocessorMismatch,
                                    }));
                                };
                                continue;
                            }

                            "include" => {
                                if !matches!(state, LexState::Base) {
                                    return Err(Box::new(TokenizeError {
                                        line: self.file_line_offsets[file_idx as usize].len()
                                            as u64,
                                        file: self.paths[file_idx as usize].clone(),
                                        reason: TokenizeErrorReason::IncludeInMacro,
                                    }));
                                }

                                let mut j = i + 1 + directive_length;
                                skip_sameline_whitespace(&content, &mut j);
                                if content[j..].starts_with('"')
                                    && let Some(l) = str_length(&content[j..])
                                {
                                    i = j + l;

                                    if if_untaken_depth < if_stack.len() {
                                        continue;
                                    }

                                    // @TODO: escaping
                                    let s = &content[j + 1..][..l - 2];
                                    let path = match &paths[file_idx as usize] {
                                        None => todo!(),
                                        Some(path) => path.as_ref(),
                                    };
                                    // @TODO: better error handling
                                    let path = path
                                        .parent()
                                        .ok_or_else(|| {
                                            std::io::Error::new(
                                                std::io::ErrorKind::NotFound,
                                                "parent directory not found".to_string(),
                                            )
                                        })
                                        .map(|p| p.join(s));
                                    let path = match path {
                                        Err(err) => {
                                            return Err(Box::new(TokenizeError {
                                                line: self.file_line_offsets[file_idx as usize]
                                                    .len()
                                                    as u64,
                                                file: self.paths[file_idx as usize].clone(),
                                                reason: TokenizeErrorReason::FailedToOpenFile(
                                                    PathBuf::default(),
                                                    err.into(),
                                                ),
                                            }));
                                        }
                                        Ok(path) => path,
                                    };
                                    let content: Arc<str> = match world.read_to_string(&path) {
                                        Ok(content) => content.into(),
                                        Err(err) => {
                                            return Err(Box::new(TokenizeError {
                                                line: self.file_line_offsets[file_idx as usize]
                                                    .len()
                                                    as u64,
                                                file: self.paths[file_idx as usize].clone(),
                                                reason: TokenizeErrorReason::FailedToOpenFile(
                                                    path, err,
                                                ),
                                            }));
                                        }
                                    };

                                    lex_stack.push(LexItem {
                                        file_idx,
                                        start,
                                        i,
                                        end_offset,
                                        state: LexState::Base,
                                    });
                                    lex_stack.push(LexItem {
                                        file_idx: contents.len() as u32,
                                        start: tokens.len(),
                                        i: 0,
                                        end_offset: content.len(),
                                        state: LexState::Base,
                                    });
                                    contents.push(content);
                                    paths.push(Some(path.into()));
                                    file_line_offsets.push(vec![0]);
                                    continue 'lex_stack;
                                }
                            }

                            // Preprocessor directives that need to be passed to the parser.
                            // @TODO: Use markers in parser.
                            "celldefine" | "endcelldefine" => {},
                            "default_nettype" => {}
                            "resetall" => {}
                            "line" => todo!(),
                            "timescale" => {}
                            "unconnected_drive" | "nounconnected_drive" => todo!(),
                            "pragma" => todo!(),
                            "begin_keywords" | "end_keywords" => todo!(),

                            // Macros
                            _ => {
                                i += 1 + directive_length;
                                if if_untaken_depth >= if_stack.len()
                                    && let Some((_, m)) = macros.find(directive)
                                {
                                    if m.args.is_some() {
                                        // Function macros.
                                        //
                                        // These work by pushing a LexItem that lexes until the
                                        // function arguments are complete. Afterwards, they are
                                        // drained and converted placed into the macro expansion.

                                        skip_bytes_sameline_whitespace(bytes, &mut i);
                                        if bytes.get(i) != Some(&b'(') {
                                            return Err(Box::new(TokenizeError {
                                                line: self.file_line_offsets[file_idx as usize]
                                                    .len()
                                                    as u64,
                                                file: self.paths[file_idx as usize].clone(),
                                                reason:
                                                    TokenizeErrorReason::FunctionMacroWithoutArgs(
                                                        directive.to_string(),
                                                    ),
                                            }));
                                        }
                                        i += 1;

                                        lex_stack.push(LexItem {
                                            file_idx,
                                            start,
                                            i: usize::MAX, // Intentionally push a problematic state.
                                            end_offset,
                                            state,
                                        });

                                        // @TODO: Support nested function macros
                                        for lex_item in lex_stack.iter().rev() {
                                            match &lex_item.state {
                                                LexState::Base => break,
                                                LexState::Macro(m) => {
                                                    return Err(Box::new(TokenizeError {
                                                        line: self.file_line_offsets
                                                            [file_idx as usize]
                                                            .len()
                                                            as u64,
                                                        file: self.paths[file_idx as usize].clone(),
                                                        reason:
                                                            TokenizeErrorReason::NestedFunctionMacro(
                                                                macros.idents[m.name].to_string(),
                                                                directive.to_string(),
                                                            ),
                                                    }));
                                                }
                                                LexState::MacroArgs(_) => continue,
                                            }
                                        }

                                        lex_stack.push(LexItem {
                                            file_idx,
                                            start: tokens.len(),
                                            i,
                                            end_offset,
                                            state: LexState::MacroArgs(MacroArgs {
                                                name: macros.idents.get_or_insert(directive),
                                                depth: 1,
                                            }),
                                        });

                                        continue 'lex_stack;
                                    }

                                    tokens.extend_from_slice(&m.tokens);
                                    spans.extend_from_slice(&m.spans);
                                    file_idxs.extend_from_slice(&m.file);
                                }
                                continue;
                            }
                        }

                        (T::Directive, 1 + directive_length)
                    }
                    _ => (T::Unknown, 1),
                };

                if if_untaken_depth >= if_stack.len() {
                    tokens.push(token);
                    spans.push(Span::new(i, i + length));
                    file_idxs.push(file_idx);
                }
                i += length;
            }

            match state {
                LexState::Base => {}
                LexState::Macro(preprocessor_macro) => {
                    // Register a macro.

                    macros.define_with_ident_id(
                        preprocessor_macro.name,
                        Macro {
                            tokens: tokens.drain(start..).collect(),
                            spans: spans.drain(start..).collect(),
                            file: file_idxs.drain(start..).collect(),
                            args: preprocessor_macro.has_args.then_some((
                                preprocessor_macro.argument_positions,
                                preprocessor_macro.arguments.len(),
                            )),
                        },
                    );
                }
                LexState::MacroArgs(a) => {
                    if a.depth > 0 {
                        return Err(Box::new(TokenizeError {
                            line: self.file_line_offsets[file_idx as usize].len() as u64,
                            file: self.paths[file_idx as usize].clone(),
                            reason: TokenizeErrorReason::UnclosedFunctionMacro(
                                macros.idents[a.name].to_string(),
                            ),
                        }));
                    }

                    // Lexing afterwards should continue from this point.
                    let parent = lex_stack.last_mut().unwrap();
                    parent.i = i + 1;

                    let m = &macros.find_by_ident_id(a.name).unwrap().1;
                    let (argument_positions, num_args) = &m.args.as_ref().unwrap();

                    scratch_funcmacro_tokens.clear();
                    scratch_funcmacro_spans.clear();
                    scratch_funcmacro_file_idxs.clear();
                    scratch_funcmacro_splits.clear();

                    scratch_funcmacro_tokens.extend(tokens.drain(start..));
                    scratch_funcmacro_spans.extend(spans.drain(start..));
                    scratch_funcmacro_file_idxs.extend(file_idxs.drain(start..));

                    // Find all the separate arguments in the argument list.
                    let mut paren_depth = 0;
                    let mut brace_depth = 0;
                    let mut bracket_depth = 0;
                    for (i, &t) in scratch_funcmacro_tokens.iter().enumerate() {
                        use Token as T;
                        paren_depth += usize::from(t == T::LeftParen);
                        paren_depth -= usize::from(t == T::RightParen);
                        brace_depth += usize::from(t == T::LeftBrace);
                        brace_depth -= usize::from(t == T::RightBrace);
                        bracket_depth += usize::from(t == T::LeftBracket);
                        bracket_depth -= usize::from(t == T::RightBracket);
                        if t == T::Comma
                            && paren_depth == 0
                            && brace_depth == 0
                            && bracket_depth == 0
                        {
                            scratch_funcmacro_splits.push(i);
                        }
                    }

                    let num_found_args = scratch_funcmacro_splits.len() + 1;
                    if num_found_args != *num_args {
                        return Err(Box::new(TokenizeError {
                            line: self.file_line_offsets[file_idx as usize].len() as u64,
                            file: self.paths[file_idx as usize].clone(),
                            reason: TokenizeErrorReason::MacroArgumentMismatch(
                                macros.idents[a.name].to_string(),
                            ),
                        }));
                    }

                    // Expand the macro and replace all the argument references with the given
                    // arguments.
                    let mut prev = 0;
                    for &(arg, offset) in argument_positions.iter() {
                        tokens.extend_from_slice(&m.tokens[prev..offset]);
                        spans.extend_from_slice(&m.spans[prev..offset]);
                        file_idxs.extend_from_slice(&m.file[prev..offset]);
                        prev = offset;

                        let arg_start = if arg == 0 {
                            0
                        } else {
                            scratch_funcmacro_splits[arg - 1] + 1
                        };
                        let arg_end = if arg >= scratch_funcmacro_splits.len() {
                            scratch_funcmacro_tokens.len()
                        } else {
                            scratch_funcmacro_splits[arg]
                        };

                        tokens.extend_from_slice(&scratch_funcmacro_tokens[arg_start..arg_end]);
                        spans.extend_from_slice(&scratch_funcmacro_spans[arg_start..arg_end]);
                        file_idxs
                            .extend_from_slice(&scratch_funcmacro_file_idxs[arg_start..arg_end]);
                    }
                    tokens.extend_from_slice(&m.tokens[prev..]);
                    spans.extend_from_slice(&m.spans[prev..]);
                    file_idxs.extend_from_slice(&m.file[prev..]);
                }
            }
        }

        Ok(())
    }
}

fn is_fst_ident_byte(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'_')
}

fn ident_length(s: &str) -> usize {
    debug_assert!(s.starts_with(|c: char| is_fst_ident_byte((c as u32) as u8)));

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

fn skip_sameline_whitespace(s: &str, i: &mut usize) {
    let b = s.as_bytes();
    skip_bytes_sameline_whitespace(b, i);
}
fn skip_bytes_sameline_whitespace(b: &[u8], i: &mut usize) {
    while b.get(*i).is_some_and(|b| matches!(b, b' ' | b'\r' | b'\t')) {
        *i += 1;
    }
}
fn skip_whitespace(s: &[u8], i: &mut usize) {
    *i += s[*i..].len() - s[*i..].trim_ascii_start().len();
}

fn str_length(s: &str) -> Option<usize> {
    debug_assert!(s.starts_with('"'));
    let bytes = s.as_bytes();
    let mut i = 1;
    while let Some(b) = bytes.get(i) {
        match b {
            b'"' => return Some(i + 1),
            // @TODO: Better escaping.
            b'\\' => i += 2,
            _ => i += 1,
        }
    }

    None
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
        pub enum Token {
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
            let consumed = Tokenized::tokenize_with_macros($example.into(), None, &mut vogls_world::std::StdWorld::new(), &mut Macros::default()).unwrap();
            assert_eq!(consumed.tokens.len(), 1);
            assert_eq!(consumed.tokens[0], Token::$ident, "Example \"{}\" of token {} had the invalid kind", $example, stringify!($ident));
            )+
        }
    };
}

define_tokens! {
    Ident = "abc",
    DollarIdent = "$abc",
    Directive = "`default_nettype",
    String = "\"this is a string\"",
    Number = "50'd50",
    Decimal = "50",
    Real = "1.5",

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
    PlusColon = "+:",
    MinusColon = "-:",

    Equals = "=",
    AtSign = "@",
    Hash = "#",

    LeftParenStar = "(*",
    StarRightParen = "*)",
    LeftParenStarRightParen = "(*)",

    StarGreaterThan = "*>",
    EqualsGreaterThan = "=>",

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
    KeywordUnsigned = "unsigned",
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
