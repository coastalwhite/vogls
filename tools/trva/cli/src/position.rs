use std::error::Error;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct SectionPosition(pub u32);

impl fmt::Display for SectionPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#010x}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Radix {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

impl Radix {
    pub const fn value(self) -> u32 {
        match self {
            Self::Binary => 2,
            Self::Octal => 8,
            Self::Decimal => 10,
            Self::Hexadecimal => 16,
        }
    }
 
    pub const fn name(self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::Octal => "octal",
            Self::Decimal => "decimal",
            Self::Hexadecimal => "hexadecimal",
        }
    }
 
    pub const fn digit_set(self) -> &'static str {
        match self {
            Self::Binary => "`0` or `1`",
            Self::Octal => "`0`-`7`",
            Self::Decimal => "`0`-`9`",
            Self::Hexadecimal => "`0`-`9`, `a`-`f` or `A`-`F`",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseSectionPositionError {
    /// The argument was empty or only whitespace.
    Empty,
    /// A radix prefix such as `0x` was given, but no digits followed it.
    NoDigits { prefix: &'static str },
    /// A character that is not a digit in the selected radix.
    InvalidDigit {
        ch: char,
        offset: usize,
        radix: Radix,
    },
    /// A hex digit appeared in an unprefixed literal - almost always a
    /// forgotten `0x`, so it gets its own message.
    MissingHexPrefix { ch: char, offset: usize },
    /// A `_` separator that is not between two digits.
    StrayUnderscore { offset: usize },
    /// The value is well-formed but larger than `u32::MAX`.
    Overflow,
}
 
impl fmt::Display for ParseSectionPositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ParseSectionPositionError::Empty => {
                write!(f, "expected an address such as `0x2000_0000`, found nothing")
            }
            ParseSectionPositionError::NoDigits { prefix } => {
                write!(f, "no digits after the `{prefix}` prefix")
            }
            ParseSectionPositionError::InvalidDigit { ch, offset, radix } => write!(
                f,
                "invalid {} digit `{}` at offset {} (expected {})",
                radix.name(),
                ch.escape_debug(),
                offset,
                radix.digit_set(),
            ),
            ParseSectionPositionError::MissingHexPrefix { ch, offset } => write!(
                f,
                "invalid decimal digit `{}` at offset {}; \
                 hexadecimal addresses need a `0x` prefix",
                ch.escape_debug(),
                offset,
            ),
            ParseSectionPositionError::StrayUnderscore { offset } => write!(
                f,
                "`_` at offset {offset} must sit between two digits",
            ),
            ParseSectionPositionError::Overflow => write!(
                f,
                "address does not fit in 32 bits (the highest address is 0xffff_ffff)",
            ),
        }
    }
}
 
impl Error for ParseSectionPositionError {}
 
impl FromStr for SectionPosition {
    type Err = ParseSectionPositionError;
 
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lead = s.len() - s.trim_start().len();
        let body = s.trim();
 
        if body.is_empty() {
            return Err(ParseSectionPositionError::Empty);
        }
 
        let (radix, prefix, digits) = if let Some(rest) = strip_either(body, "0x", "0X") {
            (Radix::Hexadecimal, "0x", rest)
        } else if let Some(rest) = strip_either(body, "0o", "0O") {
            (Radix::Octal, "0o", rest)
        } else if let Some(rest) = strip_either(body, "0b", "0B") {
            (Radix::Binary, "0b", rest)
        } else {
            (Radix::Decimal, "", body)
        };
 
        let base = lead + prefix.len();
        let mut value: u32 = 0;
        let mut seen_digit = false;
        let mut trailing_underscore: Option<usize> = None;
 
        for (i, ch) in digits.char_indices() {
            let offset = base + i;
 
            if ch == '_' {
                if !seen_digit {
                    return Err(ParseSectionPositionError::StrayUnderscore { offset });
                }
                trailing_underscore = Some(offset);
                continue;
            }
            trailing_underscore = None;
 
            let digit = match ch.to_digit(radix.value()) {
                Some(digit) => digit,
                None if radix == Radix::Decimal && ch.is_ascii_hexdigit() => {
                    return Err(ParseSectionPositionError::MissingHexPrefix { ch, offset });
                }
                None => {
                    return Err(ParseSectionPositionError::InvalidDigit { ch, offset, radix });
                }
            };
            seen_digit = true;
 
            value = value
                .checked_mul(radix.value())
                .and_then(|acc| acc.checked_add(digit))
                .ok_or(ParseSectionPositionError::Overflow)?;
        }
 
        if let Some(offset) = trailing_underscore {
            return Err(ParseSectionPositionError::StrayUnderscore { offset });
        }
        if !seen_digit {
            return Err(ParseSectionPositionError::NoDigits { prefix });
        }
 
        Ok(SectionPosition(value))
    }
}
 
fn strip_either<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    s.strip_prefix(a).or_else(|| s.strip_prefix(b))
}
