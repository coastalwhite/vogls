use crate::{Bits, Mode, VectorSize};

pub fn num_digits(s: &str) -> Result<VectorSize, ()> {
    let mut count = 0u32;
    for b in s.bytes() {
        count += u32::from(b != b'_');
    }
    VectorSize::new(count).ok_or(())
}

fn take_bits<const NBITS_PER_VALUE: usize>(
    s: &str,
    size: VectorSize,
    to_value: impl Fn(u8) -> u8,
    is_value: impl Fn(u8) -> bool,
) -> Result<Bits, ()> {
    let mut has_four_value_logic = false;
    let mut contains_value = false;
    let mut has_invalid_digit = false;
    for b in s.as_bytes() {
        has_four_value_logic |= matches!(b, b'x' | b'X' | b'z' | b'Z');
        contains_value |= is_value(*b);
        has_invalid_digit |= !(is_value(*b) || *b == b'_');
    }

    let has_no_digits = !has_four_value_logic && !contains_value;
    if has_invalid_digit || has_no_digits {
        return Err(());
    }

    // Case 1: Inlineable two-value logic.
    if !has_four_value_logic && size.get() <= 64 {
        let mut value = 0u64;
        let mut i = 0;
        for b in s.bytes().rev() {
            if b == b'_' {
                continue;
            }
            let v = to_value(b);
            value |= u64::from(v) << i;
            i += NBITS_PER_VALUE as u32;
            if i >= size.get() {
                break;
            }
        }
        return Ok(Bits::from_u64(size, value));
    }

    // Case 2: Non-inlineable two-value logic.
    if !has_four_value_logic {
        let nwords = size.get().div_ceil(64) as usize;
        let mut value = vec![0u64; nwords];
        let mut i = 0;
        for b in s.bytes().rev() {
            if b == b'_' {
                continue;
            }
            let v = to_value(b);
            value[(i as usize) / 64] |= (v as u64) << ((i as usize) % 64);
            i += NBITS_PER_VALUE as u32;
            if i >= size.get() {
                break;
            }
        }
        return Ok(Bits::from_boxed_slice(Mode::TwoValue, size, value.into()));
    }

    // Case 3: Inlineable four-value logic.
    if size.get() <= 32 {
        let mut res_spc = 0u32;
        let mut res_val = 0u32;
        let mut i = 0;
        let mut j = 0;
        let mut last = 0u8;
        while i < size.get() {
            let b = if let Some(idx) = (s.len() - 1).checked_sub(j) {
                j += 1;
                s.as_bytes()[idx]
            } else {
                last
            };
            if b == b'_' {
                continue;
            }
            last = b;

            let (spc, val) = match b {
                b'x' | b'X' => (0u8, 0u8),
                b'z' | b'Z' => (0u8, (1u8 << NBITS_PER_VALUE) - 1),
                _ => ((1u8 << NBITS_PER_VALUE) - 1, to_value(b)),
            };
            res_spc |= u32::from(spc) << i;
            res_val |= u32::from(val) << i;
            i += NBITS_PER_VALUE as u32;
        }
        return Ok(Bits::from_four_value_u64(size, res_spc, res_val));
    }

    // Case 4: Non-inlineable four-value logic.
    let nwords = size.get().div_ceil(64) as usize;
    let mut value = vec![0u64; 2 * nwords];
    let mut i = 0;
    let mut j = 0;
    let mut last = 0u8;
    while i < size.get() {
        let b = if let Some(idx) = (s.len() - 1).checked_sub(j) {
            j += 1;
            s.as_bytes()[idx]
        } else {
            last
        };
        if b == b'_' {
            continue;
        }
        last = b;

        let (spc, val) = match b {
            b'x' | b'X' => (0, 0),
            b'z' | b'Z' => (0, (1u8 << NBITS_PER_VALUE) - 1),
            _ => ((1u8 << NBITS_PER_VALUE) - 1, to_value(b)),
        };

        value[(i as usize) / 64] |= (spc as u64) << (i % 64);
        value[nwords + (i as usize) / 64] |= (val as u64) << (i % 64);
        i += NBITS_PER_VALUE as u32;
    }
    Ok(Bits::from_boxed_slice(Mode::FourValue, size, value.into()))
}

pub fn parse_bits_binary(s: &str, size: VectorSize) -> Result<Bits, ()> {
    take_bits::<1>(
        s,
        size,
        |b| b.wrapping_sub(b'0'),
        |b| matches!(b, b'0' | b'1' | b'x' | b'X' | b'z' | b'Z'),
    )
}
pub fn parse_bits_hexadecimal(s: &str, size: VectorSize) -> Result<Bits, ()> {
    take_bits::<4>(
        s,
        size,
        |b| match b {
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => b - b'0',
        },
        |b| matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b'x' | b'X' | b'z' | b'Z'),
    )
}
pub fn parse_bits_octal(s: &str, size: VectorSize) -> Result<Bits, ()> {
    let mut has_four_value_logic = false;
    let mut contains_value = false;
    let mut has_invalid_digit = false;
    for b in s.as_bytes() {
        has_four_value_logic |= matches!(b, b'x' | b'X' | b'z' | b'Z');
        contains_value |= matches!(b, b'0'..b'8');
        has_invalid_digit |= !matches!(b, b'0'..b'8' | b'_' | b'x' | b'X' | b'z' | b'Z');
    }

    let has_at_least_one_digit = !has_four_value_logic && !contains_value;
    if has_invalid_digit || has_at_least_one_digit {
        return Err(());
    }

    // Case 1: Inlineable two-value logic.
    if !has_four_value_logic && size.get() <= 64 {
        let mut value = 0u64;
        let mut i = 0;
        for b in s.bytes().rev() {
            if b == b'_' {
                continue;
            }
            let v = b - b'0';
            value |= u64::from(v) << i;
            i += 3;
            if i >= size.get() {
                break;
            }
        }
        return Ok(Bits::from_u64(size, value));
    };

    // Case 2: Non-inlineable two-value logic.
    if !has_four_value_logic {
        let nwords = size.get().div_ceil(64) as usize;
        let mut value = vec![0u64; nwords];
        let mut i = 0;
        for j in 0..s.len() {
            let b = s.as_bytes()[s.len() - j - 1];
            if b == b'_' {
                continue;
            }

            let v = b - b'0';
            value[i / 64] |= (v as u64) << (i % 64);
            if i % 64 >= 63 && 64 - i % 64 < size.get() as usize - i {
                value[(i as usize / 64) + 1] |= (v as u64) >> (64 - (i % 64));
            }
            i += 3;
        }
        return Ok(Bits::from_boxed_slice(Mode::TwoValue, size, value.into()));
    }

    // Case 3: Inlineable four-value logic.
    if size.get() <= 32 {
        let mut res_spc = 0u32;
        let mut res_val = 0u32;
        let mut i = 0;
        for b in s.bytes().rev() {
            if b == b'_' {
                continue;
            }

            let (spc, val) = match b {
                b'x' | b'X' => (0u8, 0u8),
                b'z' | b'Z' => (0u8, 0b111u8),
                _ => (0b111u8, b - b'0'),
            };

            res_val |= u32::from(val) << i;
            res_spc |= u32::from(spc) << i;
            i += 3;
        }
        if i < size.get() {
            let fst = s.as_bytes().iter().find(|b| **b != b'_').unwrap();
            let (spc, val) = match fst {
                b'x' | b'X' => (0, 0),
                b'z' | b'Z' => (0, 0b111),
                _ => (0b111u8, fst - b'0'),
            };
            while i < size.get() {
                res_val |= u32::from(val) << i;
                res_spc |= u32::from(spc) << i;
                i += 3;
            }
        }
        return Ok(Bits::from_four_value_u64(size, res_spc, res_val));
    }

    // Case 4: Non-inlineable four-value logic.
    let nwords = size.get().div_ceil(64) as usize;
    let mut value = vec![0u64; 2 * nwords];
    let mut i = 0;
    let mut j = 0;
    let mut last = 0u8;
    while i < size.get() {
        let b = if let Some(idx) = (s.len() - 1).checked_sub(j) {
            j += 1;
            s.as_bytes()[idx]
        } else {
            last
        };
        if b == b'_' {
            continue;
        }
        last = b;

        let (spc, val) = match b {
            b'x' | b'X' => (0u8, 0u8),
            b'z' | b'Z' => (0u8, 0b111u8),
            _ => (0b111u8, b - b'0'),
        };
        value[(i as usize) / 64] |= (spc as u64) << (i % 64);
        value[nwords + (i as usize) / 64] |= (val as u64) << (i % 64);
        if i % 64 >= 63 && 64 - i % 64 < size.get() - i {
            value[(i as usize / 64) + 1] |= (spc as u64) >> (64 - i % 64);
            value[nwords + (i as usize / 64) + 1] |= (val as u64) >> (64 - i % 64);
        }
        i += 3;
    }
    Ok(Bits::from_boxed_slice(Mode::FourValue, size, value.into()))
}

#[cfg(test)]
mod tests {
    use crate::BitsDataRef;

    use super::*;

    #[test]
    fn parse_binary() {
        macro_rules! assert_tv {
            ($str:expr, $size:expr, !) => {
                let size = VectorSize::new($size).unwrap();
                assert!(parse_bits_binary($str, size).is_err());
            };
            ($str:expr, $size:expr, $output:expr) => {
                let size = VectorSize::new($size).unwrap();
                let output = parse_bits_binary($str, size).unwrap();
                assert_eq!(output.size(), size);
                assert_eq!(output.as_data_ref(), $output);
            };
        }

        assert_tv!("x", 1, BitsDataRef::InlineFv(0b0, 0b0));
        assert_tv!("z", 1, BitsDataRef::InlineFv(0b0, 0b1));
        assert_tv!("0", 1, BitsDataRef::InlineTv(0o0));
        assert_tv!("1", 1, BitsDataRef::InlineTv(0o1));
        assert_tv!("2", 1, !);

        assert_tv!("x", 4, BitsDataRef::InlineFv(0o00, 0o00));
        assert_tv!("z", 4, BitsDataRef::InlineFv(0o00, 0o17));
        assert_tv!("0", 4, BitsDataRef::InlineTv(0o00));
        assert_tv!("1", 4, BitsDataRef::InlineTv(0o01));
        assert_tv!("2", 4, !);

        assert_tv!("1010", 4, BitsDataRef::InlineTv(0b1010));
        assert_tv!("__1011", 4, BitsDataRef::InlineTv(0b1011));
        assert_tv!("1x1z", 4, BitsDataRef::InlineFv(0b1010, 0b1011));
        assert_tv!(
            "1001_1110_0000_1111_0010_0011_0001_0101_1011_1010_1001_1110_0000_1011_1010_1111_0001_0010",
            72,
            BitsDataRef::SeparateTv(&[
                0b0000_1111_0010_0011_0001_0101_1011_1010_1001_1110_0000_1011_1010_1111_0001_0010,
                0b1001_1110
            ])
        );
        assert_tv!(
            "1xx1_1110_0000_1xxx_0010_0011_0z01_0101_1011_10x0_1001_1110_0000_1011_1010_1111_0001_0zz0",
            72,
            BitsDataRef::SeparateFv(&[
                0b1111_1000_1111_1111_1011_1111_1111_1101_1111_1111_1111_1111_1111_1111_1111_1001,
                0b1001_1111,
                0b0000_1000_0010_0011_0101_0101_1011_1000_1001_1110_0000_1011_1010_1111_0001_0110,
                0b1001_1110,
            ])
        );
    }

    #[test]
    fn parse_hexadecimal() {
        macro_rules! assert_tv {
            ($str:expr, $size:expr, !) => {
                let size = VectorSize::new($size).unwrap();
                assert!(parse_bits_hexadecimal($str, size).is_err());
            };
            ($str:expr, $size:expr, $output:expr) => {
                let size = VectorSize::new($size).unwrap();
                let output = parse_bits_hexadecimal($str, size).unwrap();
                assert_eq!(output.size(), size);
                assert_eq!(output.as_data_ref(), $output);
            };
        }

        assert_tv!("x", 1, BitsDataRef::InlineFv(0b0, 0b0));
        assert_tv!("z", 1, BitsDataRef::InlineFv(0b0, 0b1));
        assert_tv!("0", 1, BitsDataRef::InlineTv(0x0));
        assert_tv!("1", 1, BitsDataRef::InlineTv(0x1));
        assert_tv!("2", 1, BitsDataRef::InlineTv(0x0));
        assert_tv!("7", 1, BitsDataRef::InlineTv(0x1));
        assert_tv!("B", 1, BitsDataRef::InlineTv(0x1));
        assert_tv!("G", 1, !);

        assert_tv!("x", 4, BitsDataRef::InlineFv(0x0, 0x0));
        assert_tv!("z", 4, BitsDataRef::InlineFv(0x0, 0xF));
        assert_tv!("0", 4, BitsDataRef::InlineTv(0x0));
        assert_tv!("1", 4, BitsDataRef::InlineTv(0x1));
        assert_tv!("2", 4, BitsDataRef::InlineTv(0x2));
        assert_tv!("7", 4, BitsDataRef::InlineTv(0x7));
        assert_tv!("B", 4, BitsDataRef::InlineTv(0xB));
        assert_tv!("G", 4, !);

        assert_tv!("A070", 16, BitsDataRef::InlineTv(0xA070));
        assert_tv!("__B491", 16, BitsDataRef::InlineTv(0xB491));
        assert_tv!("Ax5z", 16, BitsDataRef::InlineFv(0xF0F0, 0xA05F));
        assert_tv!(
            "9e_0f23_15ba_9e0b_af12",
            72,
            BitsDataRef::SeparateTv(&[0x0f23_15ba_9e0b_af12, 0x9e])
        );
        assert_tv!(
            "9e_xf23_1zza_9x0b_af1x",
            72,
            BitsDataRef::SeparateFv(&[0x0FFF_F00F_F0FF_FFF0, 0xFF, 0x0f23_1FFa_900b_af10, 0x9e])
        );
    }

    #[test]
    fn parse_octal() {
        macro_rules! assert_tv {
            ($str:expr, $size:expr, !) => {
                let size = VectorSize::new($size).unwrap();
                assert!(parse_bits_octal($str, size).is_err());
            };
            ($str:expr, $size:expr, $output:expr) => {
                let size = VectorSize::new($size).unwrap();
                let output = parse_bits_octal($str, size).unwrap();
                assert_eq!(output.size(), size);
                assert_eq!(output.as_data_ref(), $output);
            };
        }

        assert_tv!("x", 1, BitsDataRef::InlineFv(0b0, 0b0));
        assert_tv!("z", 1, BitsDataRef::InlineFv(0b0, 0b1));
        assert_tv!("0", 1, BitsDataRef::InlineTv(0o0));
        assert_tv!("1", 1, BitsDataRef::InlineTv(0o1));
        assert_tv!("7", 1, BitsDataRef::InlineTv(0o1));
        assert_tv!("8", 12, !);

        assert_tv!("x", 4, BitsDataRef::InlineFv(0o00, 0o00));
        assert_tv!("z", 4, BitsDataRef::InlineFv(0o00, 0o17));
        assert_tv!("0", 4, BitsDataRef::InlineTv(0o00));
        assert_tv!("1", 4, BitsDataRef::InlineTv(0o01));
        assert_tv!("7", 4, BitsDataRef::InlineTv(0o07));
        assert_tv!("08", 4, !);

        assert_tv!("1234", 12, BitsDataRef::InlineTv(0o1234));
        assert_tv!("__1234", 12, BitsDataRef::InlineTv(0o1234));
        assert_tv!("1x3z", 12, BitsDataRef::InlineFv(0b111_000_111_000, 0o1037));
        assert_tv!(
            "2131_2340_1234_5152_3611_2314",
            72,
            BitsDataRef::SeparateTv(&[0o11_2340_1234_5152_3611_2314, 0o213 >> 1])
        );
        assert_tv!(
            "2131_2x40_1234_5z52_3611_2x14",
            72,
            BitsDataRef::SeparateFv(&[
                0o17_7077_7777_7077_7777_7077,
                0o777 >> 1,
                0o11_2040_1234_5752_3611_2014,
                0o213 >> 1
            ])
        );
    }
}
