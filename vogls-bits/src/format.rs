use std::fmt::{self, Alignment};

use crate::arithmetic::FvLogicValue;
use crate::{Bits, VectorSize};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BitsFormatBase {
    Binary,
    Octal,
    LowerHex,
    UpperHex,
    Decimal,
}

#[derive(Debug, Clone)]
pub enum BitsFormatWidth {
    Minimum(usize),
    Shrink,
    Expand,
}

#[derive(Clone)]
pub struct BitsFormatOptions {
    pub prefix: bool,
    pub base: BitsFormatBase,
    pub separator: Option<char>,
    pub align: Option<Alignment>,
    pub fill: char,
    pub width: BitsFormatWidth,
}

impl Default for BitsFormatOptions {
    fn default() -> Self {
        Self {
            prefix: false,
            base: BitsFormatBase::LowerHex,
            separator: Some('_'),
            align: None,
            fill: ' ',
            width: BitsFormatWidth::Shrink,
        }
    }
}

pub struct BitsDisplay<'a> {
    pub bits: &'a Bits,
    pub options: &'a BitsFormatOptions,
}

impl BitsFormatOptions {
    pub fn incorperate_formatter_options(&mut self, f: &fmt::Formatter<'_>) {
        self.prefix |= f.alternate();
        if let Some(align) = f.align() {
            self.align = Some(align);
            self.fill = f.fill();
        }
        if let Some(width) = f.width() {
            self.width = BitsFormatWidth::Minimum(width);
        }
    }
}

impl BitsFormatBase {
    pub fn num_digits(&self, bits: &Bits) -> usize {
        match self {
            Self::Binary => (bits.size().get() - bits.leading_zeroes()).max(1) as usize,
            Self::Octal => (bits.size().get() - bits.leading_zeroes())
                .div_ceil(3)
                .max(1) as usize,
            Self::LowerHex | Self::UpperHex => (bits.size().get() - bits.leading_zeroes())
                .div_ceil(4)
                .max(1) as usize,
            Self::Decimal => {
                if bits.contains_special() {
                    1
                } else {
                    let data_ref = bits.as_data_ref();
                    let (val, _) = data_ref.to_u64_slices();

                    if val.len() > 2 {
                        todo!()
                    }

                    let v = if val.len() == 2 {
                        ((val[1] as u128) << 64) | (val[0] as u128)
                    } else {
                        val[0] as u128
                    };

                    if v == 0 {
                        return 1;
                    }

                    let flog10 = v.ilog10();
                    let clog10 = if 10u128.pow(flog10) == v {
                        flog10
                    } else {
                        flog10 + 1
                    };
                    clog10 as usize
                }
            }
        }
    }

    pub fn max_num_digits(&self, size: VectorSize) -> usize {
        match self {
            Self::Binary => size.get() as usize,
            Self::Octal => size.get().div_ceil(3) as usize,
            Self::LowerHex | Self::UpperHex => size.get().div_ceil(4) as usize,
            Self::Decimal => {
                if size.get() > 128 {
                    todo!()
                }

                let v = 1u128.unbounded_shl(size.get()).wrapping_sub(1);
                let flog10 = v.ilog10();
                let clog10 = if 10u128.pow(flog10) == v {
                    flog10
                } else {
                    flog10 + 1
                };
                clog10 as usize
            }
        }
    }

    fn separator_digits(&self) -> usize {
        match self {
            BitsFormatBase::Binary => 4,
            BitsFormatBase::Octal => 3,
            BitsFormatBase::LowerHex | BitsFormatBase::UpperHex => 4,
            BitsFormatBase::Decimal => 3,
        }
    }
}

impl<'a> fmt::Display for BitsDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut options = self.options.clone();
        options.incorperate_formatter_options(f);
        fmt_bits(&self.bits, f, &options)
    }
}
impl<'a> fmt::Binary for BitsDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut options = self.options.clone();
        options.incorperate_formatter_options(f);
        options.base = BitsFormatBase::Binary;
        fmt_bits(&self.bits, f, &options)
    }
}
impl<'a> fmt::Octal for BitsDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut options = self.options.clone();
        options.incorperate_formatter_options(f);
        options.base = BitsFormatBase::Octal;
        fmt_bits(&self.bits, f, &options)
    }
}
impl<'a> fmt::LowerHex for BitsDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut options = self.options.clone();
        options.incorperate_formatter_options(f);
        options.base = BitsFormatBase::LowerHex;
        fmt_bits(&self.bits, f, &options)
    }
}
impl<'a> fmt::UpperHex for BitsDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut options = self.options.clone();
        options.incorperate_formatter_options(f);
        options.base = BitsFormatBase::UpperHex;
        fmt_bits(&self.bits, f, &options)
    }
}

fn fmt_bits(bits: &Bits, f: &mut impl fmt::Write, options: &BitsFormatOptions) -> fmt::Result {
    use BitsFormatBase as B;

    let num_digits = options.base.num_digits(bits);
    let num_fill = match options.width {
        BitsFormatWidth::Shrink => 0,
        BitsFormatWidth::Expand => options.base.max_num_digits(bits.size()) - num_digits,
        BitsFormatWidth::Minimum(width) => width.saturating_sub(num_digits),
    };

    let (num_left_fill, num_right_fill) = match options.align.unwrap_or(Alignment::Right) {
        Alignment::Left => (0, num_fill),
        Alignment::Center => (num_fill.div_ceil(2), num_fill / 2),
        Alignment::Right => (num_fill, 0),
    };

    let size = bits.size();
    if options.prefix {
        write!(f, "{size}'")?;
        let c = match options.base {
            B::Binary => 'b',
            B::Octal => 'o',
            B::LowerHex | B::UpperHex => 'h',
            B::Decimal => 'd',
        };
        f.write_char(c)?;
    }

    for i in 0..num_left_fill {
        f.write_char(options.fill)?;
        if let Some(separator) = options.separator
            && matches!(options.width, BitsFormatWidth::Expand)
            && i % options.base.separator_digits() == options.base.separator_digits() - 1
            && i != 0
        {
            f.write_char(separator)?;
        }
    }

    match options.base {
        B::Binary => fmt_bits_binary(bits, f, options.separator),
        B::Octal => fmt_bits_octal(bits, f, options.separator),
        B::LowerHex => fmt_bits_hex(bits, f, options.separator, false),
        B::UpperHex => fmt_bits_hex(bits, f, options.separator, true),
        B::Decimal => fmt_bits_decimal(bits, f, options.separator),
    }?;

    for _ in 0..num_right_fill {
        f.write_char(options.fill)?;
    }

    Ok(())
}

fn fmt_bits_binary(bits: &Bits, f: &mut impl fmt::Write, separator: Option<char>) -> fmt::Result {
    fn print(
        f: &mut impl fmt::Write,
        val: u64,
        spc: Option<u64>,
        size: VectorSize,
        separator: Option<char>,
        fst: &mut bool,
    ) -> fmt::Result {
        assert!(size.get() <= 64);

        // Cut off leading zeros.
        let mut actual_size = size.get();
        if *fst {
            let mut leading_zeroes = (val << (64 - size.get())).leading_zeros();
            if let Some(spc) = spc {
                leading_zeroes = leading_zeroes.min((spc << (64 - size.get())).leading_ones());
            }
            leading_zeroes = leading_zeroes.min(size.get());
            actual_size = actual_size.min(64 - leading_zeroes);

            if actual_size == 0 {
                return Ok(());
            }
        }

        if !*fst && let Some(separator) = separator {
            f.write_char(separator)?;
            *fst = false;
        }

        match spc {
            None => {
                for i in 0..actual_size {
                    let shift = actual_size - i - 1;
                    let v = (val >> shift) & 1;
                    f.write_char((b'0' + v as u8).into())?;

                    if let Some(separator) = separator
                        && shift % 4 == 0
                        && shift != 0
                    {
                        f.write_char(separator)?;
                    }
                }
                Ok(())
            }
            Some(spc) => {
                for i in 0..actual_size {
                    let shift = actual_size - i - 1;
                    let s = (spc >> shift) & 1;
                    let v = (val >> shift) & 1;
                    let fv = FvLogicValue::from_repr(((s as u8) << 1) | (v as u8));

                    f.write_char(match fv {
                        FvLogicValue::X => 'x',
                        FvLogicValue::Z => 'z',
                        FvLogicValue::L0 => '0',
                        FvLogicValue::L1 => '1',
                    })?;

                    if let Some(separator) = separator
                        && shift % 4 == 0
                        && shift != 0
                    {
                        f.write_char(separator)?;
                    }
                }
                Ok(())
            }
        }
    }

    let data_ref = bits.as_data_ref();
    let (val, spc) = data_ref.to_u64_slices();
    let msw_val = *val.last().unwrap();
    let msw_size = bits.size().get() % 64;
    let rem_size = bits.size().get() - msw_size;
    let mut fst = true;
    match spc {
        None => {
            if let Some(msw_size) = VectorSize::new(msw_size) {
                print(f, msw_val, None, msw_size, separator, &mut fst)?;
            }
            for i in (0..rem_size.div_ceil(64) as usize).rev() {
                print(
                    f,
                    val[i],
                    None,
                    VectorSize::new(64).unwrap(),
                    separator,
                    &mut fst,
                )?;
            }
        }
        Some(spc) => {
            if let Some(msw_size) = VectorSize::new(msw_size) {
                let msw_spc = *spc.last().unwrap();
                print(f, msw_val, Some(msw_spc), msw_size, separator, &mut fst)?;
            }
            for i in (0..rem_size.div_ceil(64) as usize).rev() {
                print(
                    f,
                    val[i],
                    Some(spc[i]),
                    VectorSize::new(64).unwrap(),
                    separator,
                    &mut fst,
                )?;
            }
        }
    }

    Ok(())
}
fn fmt_bits_octal(bits: &Bits, f: &mut impl fmt::Write, _separator: Option<char>) -> fmt::Result {
    if bits.contains_special() {
        todo!();
    }

    let data_ref = bits.as_data_ref();
    let (val, _) = data_ref.to_u64_slices();

    if val.len() > 1 {
        todo!()
    }

    write!(f, "{:o}", val[0])
}
fn fmt_bits_hex(
    bits: &Bits,
    f: &mut impl fmt::Write,
    separator: Option<char>,
    is_upper: bool,
) -> fmt::Result {
    fn nimble_to_digit(v: u8, s: Option<u8>, nbits: u32, is_upper: bool) -> char {
        debug_assert!(nbits <= 4);
        let mask = (1u8 << nbits) - 1;
        if let Some(s) = s
            && s & mask != mask
        {
            if s == 0 && v == 0 {
                'x'
            } else if s == 0 && v == mask {
                'z'
            } else if !s & !v != 0 {
                'X'
            } else {
                'Z'
            }
        } else {
            if v >= 10 {
                if is_upper {
                    b'A' + (v - 10)
                } else {
                    b'a' + (v - 10)
                }
            } else {
                b'0' + v
            }
            .into()
        }
    }

    fn print(
        f: &mut impl fmt::Write,
        val: u64,
        spc: Option<u64>,
        size: VectorSize,
        separator: Option<char>,
        fst: &mut bool,
        is_upper: bool,
    ) -> fmt::Result {
        assert!(size.get() <= 64);

        // Cut off leading zeros.
        let mut actual_size = size.get();
        if *fst {
            let mut leading_zeroes = (val << (64 - size.get())).leading_zeros();
            if let Some(spc) = spc {
                leading_zeroes = leading_zeroes.min((spc << (64 - size.get())).leading_ones());
            }
            leading_zeroes = leading_zeroes.min(size.get());
            actual_size = size.get() - leading_zeroes;

            if actual_size == 0 {
                return Ok(());
            }
        }

        if !*fst && let Some(separator) = separator {
            f.write_char(separator)?;
        }
        *fst = false;
        let mut rem_size = actual_size;

        match spc {
            None => {
                while rem_size > 0 {
                    let shift = if rem_size % 4 == 0 {
                        rem_size - 4
                    } else {
                        rem_size - rem_size % 4
                    };
                    let v = (val >> shift) & 0xF;

                    let c = nimble_to_digit(v as u8, None, rem_size - shift, is_upper);
                    f.write_char(c)?;

                    if let Some(separator) = separator
                        && shift % 16 == 0
                        && shift != 0
                    {
                        f.write_char(separator)?;
                    }

                    rem_size = shift;
                }
                Ok(())
            }
            Some(spc) => {
                while rem_size > 0 {
                    let shift = if rem_size % 4 == 0 {
                        rem_size - 4
                    } else {
                        rem_size - rem_size % 4
                    };
                    let s = (spc >> shift) & 0xF;
                    let v = (val >> shift) & 0xF;

                    let c = nimble_to_digit(v as u8, Some(s as u8), rem_size - shift, is_upper);
                    f.write_char(c)?;

                    if let Some(separator) = separator
                        && shift % 16 == 0
                        && shift != 0
                    {
                        f.write_char(separator)?;
                    }

                    rem_size = shift;
                }
                Ok(())
            }
        }
    }

    let data_ref = bits.as_data_ref();
    let (val, spc) = data_ref.to_u64_slices();
    let msw_val = *val.last().unwrap();
    let msw_size = bits.size().get() % 64;
    let rem_size = bits.size().get() - msw_size;
    let mut fst = true;
    match spc {
        None => {
            if let Some(msw_size) = VectorSize::new(msw_size) {
                print(f, msw_val, None, msw_size, separator, &mut fst, is_upper)?;
            }
            for i in (0..rem_size.div_ceil(64) as usize).rev() {
                print(
                    f,
                    val[i],
                    None,
                    VectorSize::new(64).unwrap(),
                    separator,
                    &mut fst,
                    is_upper,
                )?;
            }
        }
        Some(spc) => {
            if let Some(msw_size) = VectorSize::new(msw_size) {
                let msw_spc = *spc.last().unwrap();
                print(
                    f,
                    msw_val,
                    Some(msw_spc),
                    msw_size,
                    separator,
                    &mut fst,
                    is_upper,
                )?;
            }
            for i in (0..rem_size.div_ceil(64) as usize).rev() {
                print(
                    f,
                    val[i],
                    Some(spc[i]),
                    VectorSize::new(64).unwrap(),
                    separator,
                    &mut fst,
                    is_upper,
                )?;
            }
        }
    }

    if fst {
        f.write_char('0')?;
    }

    Ok(())
}
fn fmt_bits_decimal(bits: &Bits, f: &mut impl fmt::Write, _separator: Option<char>) -> fmt::Result {
    if bits.contains_special() {
        // @FIXME: This should print 'X' and 'Z' if not all digits are special.
        if bits.contains_unknown() {
            f.write_char('x')?;
        } else {
            f.write_char('z')?;
        }
        return Ok(());
    }

    let data_ref = bits.as_data_ref();
    let (val, _) = data_ref.to_u64_slices();

    if val.len() > 1 {
        todo!()
    }

    write!(f, "{}", val[0])
}
