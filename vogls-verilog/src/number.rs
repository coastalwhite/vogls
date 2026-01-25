use std::num::NonZeroU32;

use vogls_ir::{Bits, Mode, VectorSize};

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
        && matches!(b, b'0'..=b'9' | b'x' | b'z' | b'X' | b'Z' | b'_')
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

pub fn take_size<'a>(s: &'a str) -> Result<(&'a str, VectorSize), ()> {
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

pub fn parse_decimal_bits(s: &str, size: Option<VectorSize>) -> Result<Bits, ()> {
    let Some(size) = size else {
        todo!("Decimal with inferred size");
    };

    if size.get() <= 64 {
        let mut value = 0u64;
        for b in s.bytes() {
            if b == b'_' {
                continue;
            }

            let v = match b {
                b'x' | b'X' => 0,
                b'z' | b'Z' => 0,
                _ => b - b'0',
            };

            value = value.checked_mul(10).ok_or(())?;
            value = value.checked_add(u64::from(v)).ok_or(())?;
        }
        Ok(Bits::from_u64(size, value))
    } else {
        todo!("Big Decimal Numbers");
    }
}

pub fn take_bits<const NBITS_PER_VALUE: usize>(
    s: &str,
    size: Option<VectorSize>,
    to_value: impl Fn(u8) -> u8,
) -> Result<Bits, ()> {
    let size = match size {
        None => {
            let mut count = 0u32;
            for b in s.bytes() {
                count += u32::from(b != b'_');
            }
            VectorSize::new(count * (NBITS_PER_VALUE as u32)).ok_or(())?
        }
        Some(s) => s,
    };
    let contains_special = s
        .as_bytes()
        .iter()
        .find(|b| matches!(b, b'x' | b'X' | b'z' | b'Z'))
        .is_some();

    if !contains_special && size.get() <= 64 {
        let mut value = 0u64;
        for b in s.bytes() {
            if b == b'_' {
                continue;
            }
            let v = to_value(b);
            value <<= NBITS_PER_VALUE;
            value |= u64::from(v);
        }
        Ok(Bits::from_u64(size, value))
    } else if contains_special && size.get() <= 32 {
        let mut res_spc = 0u32;
        let mut res_val = 0u32;
        for b in s.bytes() {
            if b == b'_' {
                continue;
            }
            let (spc, val) = match b {
                b'x' | b'X' => (0u8, 0u8),
                b'z' | b'Z' => (0u8, (1u8 << NBITS_PER_VALUE) - 1),
                _ => (1u8, to_value(b)),
            };
            res_spc <<= NBITS_PER_VALUE;
            res_val <<= NBITS_PER_VALUE;
            res_spc |= u32::from(spc);
            res_val |= u32::from(val);
        }
        Ok(Bits::from_four_value_u64(size, res_spc, res_val))
    } else if !contains_special {
        let nwords = size.get().div_ceil(64) as usize;
        let mut value = vec![0u64; nwords];
        let mut i = 0;
        while i < s.len() {
            let b = s.as_bytes()[s.len() - i - 1];
            if b == b'_' {
                continue;
            }

            let v = to_value(b);
            value[i / (64 / NBITS_PER_VALUE)] |=
                (v as u64) << (i % (64 / NBITS_PER_VALUE)) * NBITS_PER_VALUE;
            i += 1;
        }
        Ok(Bits::from_boxed_slice(Mode::TwoValue, size, value.into()))
    } else {
        let nwords = size.get().div_ceil(64) as usize;
        let mut value = vec![0u64; 2 * nwords];
        let mut i = 0;
        while i < s.len() {
            let b = s.as_bytes()[s.len() - i - 1];
            if b == b'_' {
                continue;
            }

            let (spc, val) = match b {
                b'x' | b'X' => (0, 0),
                b'z' | b'Z' => (0, (1u8 << NBITS_PER_VALUE) - 1),
                _ => (1, to_value(b)),
            };

            value[i / (64 / NBITS_PER_VALUE)] |=
                (spc as u64) << (i % (64 / NBITS_PER_VALUE)) * NBITS_PER_VALUE;
            value[nwords + i / (64 / NBITS_PER_VALUE)] |=
                (val as u64) << (i % (64 / NBITS_PER_VALUE)) * NBITS_PER_VALUE;
            i += 1;
        }
        Ok(Bits::from_boxed_slice(Mode::FourValue, size, value.into()))
    }
}

pub fn take_binary_bits(s: &str, size: Option<VectorSize>) -> Result<Bits, ()> {
    take_bits::<1>(s, size, |b| b - b'0')
}

pub fn take_octal_bits(s: &str, size: Option<VectorSize>) -> Result<Bits, ()> {
    let size = match size {
        None => {
            let mut count = 0u32;
            for b in s.bytes() {
                count += u32::from(b != b'_');
            }
            let count = count.checked_mul(3).ok_or(())?;
            VectorSize::new(count).ok_or(())?
        }
        Some(s) => s,
    };
    let contains_special = s
        .as_bytes()
        .iter()
        .find(|b| matches!(b, b'x' | b'X' | b'z' | b'Z'))
        .is_some();
    if contains_special {
        todo!()
    }

    if size.get() <= 64 {
        let mut value = 0u64;
        for b in s.bytes() {
            if b == b'_' {
                continue;
            }

            let v = match b {
                b'x' | b'X' => 0,
                b'z' | b'Z' => 0,
                _ => b - b'0',
            };

            value <<= 3;
            value |= u64::from(v);
        }
        Ok(Bits::from_u64(size, value))
    } else {
        todo!()
    }
}

pub fn take_hexadecimal_bits(s: &str, size: Option<VectorSize>) -> Result<Bits, ()> {
    take_bits::<4>(s, size, |b| match b {
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => b - b'0',
    })
}
