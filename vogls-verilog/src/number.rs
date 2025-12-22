use std::fmt::{self, Write};
use std::num::NonZeroU32;
use std::sync::Arc;

use crate::tokenizer::Takeable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Size(NonZeroU32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Unsigned,
    Signed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    Decimal,
    Binary,
    Octal,
    Hexadecimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizedNumber {
    pub size: Option<Size>,
    pub sign: Sign,
    pub base: Base,
    pub value: Bits,
}

pub struct DecimalBits(Bits);
pub struct BinaryBits(Bits);
pub struct OctalBits(Bits);
pub struct HexadecimalBits(Bits);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decimal {
    Small(u64),
    Large(Arc<LargeBits>),
}

impl<'a> Takeable<'a> for SizedNumber {
    fn take(s: &'a str) -> (&'a str, Self) {
        let (s, size) = if s.starts_with("'") {
            (s, None)
        } else {
            let (s, size) = Size::take(s);
            debug_assert!(s.starts_with('\''));
            (s, Some(size))
        };
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

        let value = SizedNumber {
            size,
            sign,
            base,
            value,
        };
        (s, value)
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Small(v) => v.fmt(f),
            Self::Large(_) => todo!(),
        }
    }
}

impl fmt::Display for SizedNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(size) = self.size {
            size.0.fmt(f)?;
        }
        f.write_char('\'')?;
        if matches!(self.sign, Sign::Signed) {
            f.write_char('s')?;
        }
        match self.base {
            Base::Decimal => write!(f, "d{}", self.value),
            Base::Binary => write!(f, "b{:b}", self.value),
            Base::Octal => write!(f, "o{:o}", self.value),
            Base::Hexadecimal => write!(f, "x{:x}", self.value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bits {
    Small(u64),
    Large(Arc<LargeBits>),
}

impl fmt::Display for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bits::Small(v) => v.fmt(f),
            Bits::Large(_) => todo!(),
        }
    }
}

impl fmt::Binary for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bits::Small(v) => v.fmt(f),
            Bits::Large(_) => todo!(),
        }
    }
}

impl fmt::Octal for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bits::Small(v) => v.fmt(f),
            Bits::Large(_) => todo!(),
        }
    }
}

impl fmt::LowerHex for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bits::Small(v) => v.fmt(f),
            Bits::Large(_) => todo!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LargeBits(Vec<u128>);

impl<'a> Takeable<'a> for Size {
    fn take(s: &'a str) -> (&'a str, Self) {
        let mut chars = s.char_indices();

        let (_, fst) = chars.next().unwrap();
        let fst = fst.to_digit(10).unwrap();

        let mut size = NonZeroU32::new(fst).unwrap();

        for (i, c) in chars.filter(|(_, c)| *c != '_') {
            let Some(c) = c.to_digit(10) else {
                return (&s[i..], Size(size));
            };

            size = size.checked_mul(NonZeroU32::new(10).unwrap()).unwrap();
            size = size.checked_add(c).unwrap();
        }

        ("", Size(size))
    }
}

impl<'a> Takeable<'a> for Sign {
    fn take(s: &'a str) -> (&'a str, Self) {
        match s.bytes().next() {
            Some(b'S' | b's') => (&s[1..], Self::Signed),
            _ => (s, Self::Unsigned),
        }
    }
}

impl<'a> Takeable<'a> for Base {
    fn take(s: &'a str) -> (&'a str, Self) {
        match s.bytes().next() {
            Some(b'D' | b'd') => (&s[1..], Self::Decimal),
            Some(b'B' | b'b') => (&s[1..], Self::Binary),
            Some(b'O' | b'o') => (&s[1..], Self::Octal),
            Some(b'H' | b'h') => (&s[1..], Self::Hexadecimal),
            Some(c) => unreachable!("found: {}", char::from(c)),
            None => unreachable!(),
        }
    }
}

impl Base {
    pub fn is_valid(c: char) -> bool {
        matches!(c, 'D' | 'd' | 'B' | 'b' | 'O' | 'o' | 'X' | 'x')
    }
}

impl<'a> Takeable<'a> for Decimal {
    fn take(s: &'a str) -> (&'a str, Self) {
        debug_assert!(s.starts_with(|c: char| matches!(c, '0'..='9' | '_')));

        let mut value = 0u64;
        let mut bytes = s.bytes();

        let mut offset = 0;
        loop {
            let Some(b) = bytes.next().filter(|b| matches!(b, b'0'..=b'9' | b'_')) else {
                return (&s[offset..], Decimal::Small(value));
            };

            offset += 1;

            if b == b'_' {
                continue;
            }

            let digit = b - b'0';

            let Some(new_value) = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(u64::from(digit)))
            else {
                break;
            };

            value = new_value;
        }

        todo!("Big Decimal Numbers");
    }
}

impl Size {
    pub fn as_u32(self) -> u32 {
        self.0.into()
    }
}

impl From<DecimalBits> for Bits {
    fn from(v: DecimalBits) -> Bits {
        v.0
    }
}
impl From<BinaryBits> for Bits {
    fn from(v: BinaryBits) -> Bits {
        v.0
    }
}
impl From<OctalBits> for Bits {
    fn from(v: OctalBits) -> Bits {
        v.0
    }
}
impl From<HexadecimalBits> for Bits {
    fn from(v: HexadecimalBits) -> Bits {
        v.0
    }
}

impl<'a> Takeable<'a> for DecimalBits {
    fn take(s: &'a str) -> (&'a str, Self) {
        debug_assert!(s.starts_with(|c: char| matches!(c, '0'..='9' | '_')));

        let mut value = 0u64;
        let mut bytes = s.bytes();

        let mut offset = 0;
        loop {
            let Some(b) = bytes.next().filter(|b| matches!(b, b'0'..=b'9' | b'_')) else {
                return (&s[offset..], DecimalBits(Bits::Small(value)));
            };

            offset += 1;

            if b == b'_' {
                continue;
            }

            let digit = b - b'0';

            let Some(new_value) = value
                .checked_mul(10)
                .and_then(|v| v.checked_add(u64::from(digit)))
            else {
                break;
            };

            value = new_value;
        }

        todo!("Big Decimal Numbers");
    }
}

impl<'a> Takeable<'a> for BinaryBits {
    fn take(s: &'a str) -> (&'a str, Self) {
        debug_assert!(s.starts_with(&['0', '1', '_']));

        let mut value = 0u64;
        let mut bytes = s.bytes();

        let mut offset = 0;
        let mut num_bits = 0;
        loop {
            let Some(b) = bytes.next().filter(|b| matches!(b, b'0'..=b'1' | b'_')) else {
                return (&s[offset..], BinaryBits(Bits::Small(value)));
            };

            offset += 1;

            if b == b'_' {
                continue;
            }

            if num_bits + 1 > 64 {
                break;
            }

            let digit = b - b'0';

            value <<= 1;
            value |= u64::from(digit);

            num_bits += 1;
        }

        todo!("Big Binary Numbers");
    }
}

impl<'a> Takeable<'a> for OctalBits {
    fn take(s: &'a str) -> (&'a str, Self) {
        debug_assert!(s.starts_with(|b: char| matches!(b, '0'..='7')));

        let mut value = 0u64;
        let mut bytes = s.bytes();

        let mut offset = 0;
        let mut num_bits = 0;
        loop {
            let Some(b) = bytes.next().filter(|b| matches!(b, b'0'..=b'7' | b'_')) else {
                return (&s[offset..], OctalBits(Bits::Small(value)));
            };

            offset += 1;

            if b == b'_' {
                continue;
            }

            if num_bits + 3 > 64 {
                break;
            }

            let digit = b - b'0';

            value <<= 3;
            value |= u64::from(digit);

            num_bits += 3;
        }

        todo!("Big Octal Numbers");
    }
}

impl<'a> Takeable<'a> for HexadecimalBits {
    fn take(s: &'a str) -> (&'a str, Self) {
        fn is_hexadecimal_digit(b: u8) -> bool {
            matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
        }

        debug_assert!(s.starts_with(|c: char| matches!(c, '0'..='9' | 'a'..='f' | 'A'..='F')));

        let mut value = 0u64;
        let mut bytes = s.bytes();

        let mut offset = 0;
        let mut num_bits = 0;
        loop {
            let Some(b) = bytes
                .next()
                .filter(|b| is_hexadecimal_digit(*b) | (*b == b'_'))
            else {
                return (&s[offset..], HexadecimalBits(Bits::Small(value)));
            };

            offset += 1;

            if b == b'_' {
                continue;
            }

            if num_bits + 4 > 64 {
                break;
            }

            let digit = match b {
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                b'0'..=b'9' => b - b'0',
                _ => unreachable!(),
            };

            value <<= 4;
            value |= u64::from(digit);

            num_bits += 4;
        }

        todo!("Big Hex Numbers");
    }
}
