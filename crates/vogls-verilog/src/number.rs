use std::num::NonZeroU32;

use vogls_ir::bits::parse::BitsParseError;
use vogls_ir::{Bits, VectorSize};

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

pub fn skip_decimal(s: &[u8], i: &mut usize) {
    while let Some(b) = s.get(*i)
        && matches!(b, b'0'..=b'9' | b'_')
    {
        *i += 1;
    }
}
pub fn skip_binary(s: &[u8], i: &mut usize) {
    while let Some(b) = s.get(*i)
        && matches!(b, b'0'..=b'1' | b'x' | b'z' | b'X' | b'Z' | b'_')
    {
        *i += 1;
    }
}
pub fn skip_octal(s: &[u8], i: &mut usize) {
    while let Some(b) = s.get(*i)
        && matches!(b, b'0'..=b'7' | b'x' | b'z' | b'X' | b'Z' | b'_')
    {
        *i += 1;
    }
}
pub fn skip_hexadecimal(s: &[u8], i: &mut usize) {
    while let Some(b) = s.get(*i)
        && matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b'x' | b'z' | b'X' | b'Z' | b'_')
    {
        *i += 1;
    }
}

pub fn skip_sign(s: &[u8], i: &mut usize) -> bool {
    let has_sign = matches!(s.get(*i), Some(b's' | b'S'));
    *i += usize::from(has_sign);
    has_sign
}

#[rustfmt::skip]
pub fn take_base(s: &[u8], i: &mut usize) -> Option<Base> {
    match s.get(*i) {
        Some(b'D' | b'd') => { *i += 1; Some(Base::Decimal)     },
        Some(b'B' | b'b') => { *i += 1; Some(Base::Binary)      },
        Some(b'O' | b'o') => { *i += 1; Some(Base::Octal)       },
        Some(b'H' | b'h') => { *i += 1; Some(Base::Hexadecimal) },
        _ => None,
    }
}

pub fn take_size(s: &str) -> Result<(&str, VectorSize), ()> {
    let mut chars = s.char_indices();

    let (_, fst) = chars.next().unwrap();
    let fst = fst.to_digit(10).unwrap();

    let mut size = fst;

    for (i, c) in chars.filter(|(_, c)| *c != '_') {
        let Some(c) = c.to_digit(10) else {
            return Ok((&s[i..], VectorSize::new(size).ok_or(())?));
        };

        size = size.checked_mul(10).ok_or(())?;
        size = size.checked_add(c).unwrap();
    }

    let size = VectorSize::new(size).ok_or(())?;
    Ok(("", size))
}

impl Base {
    pub fn is_valid(c: char) -> bool {
        matches!(c, 'D' | 'd' | 'B' | 'b' | 'O' | 'o' | 'X' | 'x')
    }
}

pub fn parse_decimal_bits(s: &str, size: Option<VectorSize>) -> Result<Bits, BitsParseError> {
    let size = match size {
        Some(size) => size,
        None => {
            let num_digits = s.bytes().filter(|b| b.is_ascii_digit()).count();
            let num_digits = u32::try_from(num_digits).map_err(|_| BitsParseError)?;
            if num_digits == 0 {
                return Err(BitsParseError);
            }
            match s.bytes().find(|b| matches!(b, b'1'..=b'9')) {
                None => NonZeroU32::MIN,
                Some(fst_digit) => {
                    let fst_digit = fst_digit - b'0';
                    let num_bits =
                        (f64::from(num_digits) * 10f64.log2() + f64::from(fst_digit).log2()).ceil();
                    if !num_bits.is_finite() || num_bits < 1.0 || num_bits > u32::MAX as f64 {
                        return Err(BitsParseError);
                    }
                    VectorSize::new(num_bits as u32).unwrap()
                }
            }
        }
    };

    if size.get() <= 64 {
        let mut value = 0u64;
        for b in s.bytes() {
            if b == b'_' {
                continue;
            }

            let v = b - b'0';
            value = value.checked_mul(10).ok_or(BitsParseError)?;
            value = value.checked_add(u64::from(v)).ok_or(BitsParseError)?;
        }
        Ok(Bits::from_u64(size, value))
    } else {
        todo!()
    }
}

pub fn take_binary_bits(s: &str, size: Option<VectorSize>) -> Result<Bits, BitsParseError> {
    let size = match size {
        None => vogls_ir::bits::parse::num_digits(s)
            .ok_or(BitsParseError)?
            .checked_mul(VectorSize::new(1u32).unwrap())
            .ok_or(BitsParseError)?,
        Some(s) => s,
    };
    vogls_ir::bits::parse::parse_bits_binary(s, size)
}

pub fn take_octal_bits(s: &str, size: Option<VectorSize>) -> Result<Bits, BitsParseError> {
    let size = match size {
        None => vogls_ir::bits::parse::num_digits(s)
            .ok_or(BitsParseError)?
            .checked_mul(VectorSize::new(3u32).unwrap())
            .ok_or(BitsParseError)?,
        Some(s) => s,
    };
    vogls_ir::bits::parse::parse_bits_octal(s, size)
}

pub fn take_hexadecimal_bits(s: &str, size: Option<VectorSize>) -> Result<Bits, BitsParseError> {
    let size = match size {
        None => vogls_ir::bits::parse::num_digits(s)
            .ok_or(BitsParseError)?
            .checked_mul(VectorSize::new(4u32).unwrap())
            .ok_or(BitsParseError)?,
        Some(s) => s,
    };
    vogls_ir::bits::parse::parse_bits_hexadecimal(s, size)
}
