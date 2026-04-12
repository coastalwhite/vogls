#[derive(Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

pub struct Tokenized {
    pub tokens: Vec<Token>,
    pub spans: Vec<Span>,
}

pub fn tokenize(s: &str) -> Tokenized {
    let mut tokens = Vec::new();
    let mut spans = Vec::new();

    let mut i = 0;
    let bs = s.as_bytes();

    use Token as T;
    while let Some(b) = bs.get(i) {
        let (token, length) = match b {
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
                continue;
            }

            b'(' => (T::LeftParen, 1),
            b')' => (T::RightParen, 1),

            b'/' => (T::Slash, 1),
            b'.' => (T::Dot, 1),
            b'+' => (T::Plus, 1),
            b'-' => (T::Minus, 1),
            b'*' => (T::Star, 1),
            b'%' => (T::Modulo, 1),
            b'<' => match bs.get(i + 1) {
                Some(b'<') => (T::DoubleLeftAngle, 2),
                Some(b'=') => (T::LeftAngleEquals, 2),
                _ => (T::LeftAngle, 1),
            },
            b'>' => match bs.get(i + 1) {
                Some(b'>') => (T::DoubleRightAngle, 2),
                Some(b'=') => (T::RightAngleEquals, 2),
                _ => (T::RightAngle, 1),
            },
            b'!' => match (bs.get(i + 1), bs.get(i + 2)) {
                (Some(b'='), Some(b'=')) => (T::BangDoubleEquals, 2),
                (Some(b'='), _) => (T::BangEquals, 2),
                _ => (T::Bang, 1),
            },
            b'~' => match bs.get(i + 1) {
                Some(b'&') => (T::TildeAmpersand, 2),
                Some(b'|') => (T::TildeBar, 2),
                Some(b'^') => (T::TildeCaret, 2),
                _ => (T::Tilde, 1),
            },
            b'&' => match bs.get(i + 1) {
                Some(b'&') => (T::DoubleAmpersand, 2),
                _ => (T::Ampersand, 1),
            },
            b'|' => match bs.get(i + 1) {
                Some(b'|') => (T::DoubleBar, 2),
                _ => (T::Bar, 1),
            },
            b'^' => match bs.get(i + 1) {
                Some(b'~') => (T::CaretTilde, 2),
                _ => (T::Caret, 1),
            },
            b'=' => match (bs.get(i + 1), bs.get(i + 2)) {
                (Some(b'='), Some(b'=')) => (T::TripleEquals, 2),
                (Some(b'='), _) => (T::DoubleEquals, 2),
                _ => (T::Caret, 1),
            },

            b'"' => {
                let mut j = i + 1;
                let mut is_escaped = false;
                while let Some(&b) = bs.get(j)
                    && (is_escaped || b != b'"')
                {
                    is_escaped = !is_escaped & (b == b'\\');
                    j += 1;
                }
                if j == bs.len() {
                    // @TODO: Better error.
                    panic!("unclosed string");
                }
                let str_length = j - i + 1;
                (T::QString, str_length)
            }

            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' => {
                let mut j = i + 1;
                while let Some(b) = bs.get(j)
                    && matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z')
                {
                    // @TODO: Escaped characters
                    j += 1;
                }
                let ident_length = j - i;
                (T::Ident, ident_length)
            }

            _ => (T::Unknown, 1),
        };

        let span = Span {
            start: i,
            end: i + length,
        };
        spans.push(span);
        tokens.push(token);

        i += length;
    }

    Tokenized { tokens, spans }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    Unknown,

    LeftParen,
    RightParen,
    LeftAngle,
    RightAngle,

    Slash,
    Dot,

    Plus,
    Minus,
    Bang,
    Tilde,
    Ampersand,
    TildeAmpersand,
    Bar,
    TildeBar,
    Caret,
    CaretTilde,
    TildeCaret,
    Star,
    Modulo,
    DoubleEquals,
    BangEquals,
    TripleEquals,
    BangDoubleEquals,
    DoubleAmpersand,
    DoubleBar,
    LeftAngleEquals,
    RightAngleEquals,
    DoubleLeftAngle,
    DoubleRightAngle,

    QString,

    Ident,
}
