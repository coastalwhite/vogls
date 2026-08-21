use std::fmt::{self, Alignment, Write};
use std::io;

use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
use vogls_utils::NonMaxU32;

use crate::time::{TimeFormat, TimeResolution};
use crate::{Bits, VSIZE_64};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DynFormatString {
    content: Box<str>,
    arguments: Box<[(usize, DynFormatArgument)]>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Padding {
    ZeroPaddedToSize,
    ZeroPaddedTo(u32),
    #[default]
    NoPadding,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Base {
    #[default]
    Adaptive,
    Binary,
    Octal,
    Hexadecimal,
    Decimal,
    Ascii,
    Exponential,
    Float,
    FloatAdaptive,
    /// `%t` applied to an integer time value (e.g. `$time`).
    Time,
    /// `%t` applied to a real time value (e.g. `$realtime`).
    TimeReal,
}
impl Base {
    pub fn is_fp_representation(self) -> bool {
        matches!(self, Self::Exponential | Self::Float | Self::FloatAdaptive)
    }
    pub fn is_time(self) -> bool {
        matches!(self, Self::Time | Self::TimeReal)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DynFormatArgument {
    pub padding: Padding,
    pub base: Base,
    /// Precision for floating-point formatting. Ignored otherwise.
    pub precision: Option<NonMaxU32>,
    pub signed: bool,
    pub prefix: bool,
    /// Time unit the argument value is expressed in. Only meaningful for the
    /// `Time`/`TimeReal` bases: `%t` arguments (`$time`/`$realtime`) are scaled
    /// to the module's `timescale` unit at lowering, so `%t` formatting must
    /// interpret them relative to that unit rather than the global resolution.
    pub time_unit: TimeResolution,
}

impl Default for DynFormatArgument {
    fn default() -> Self {
        Self {
            padding: Default::default(),
            base: Default::default(),
            precision: None,
            signed: false,
            prefix: true,
            // Only meaningful for the `Time`/`TimeReal` bases; the value is a
            // placeholder for every other base.
            time_unit: TimeResolution::S1,
        }
    }
}

impl DynFormatString {
    pub fn new(content: Box<str>, arguments: Box<[(usize, DynFormatArgument)]>) -> Self {
        assert!(
            arguments.iter().all(|(a, _)| *a <= content.len())
                && arguments.windows(2).all(|a| a[0].0 <= a[1].0)
        );

        Self { content, arguments }
    }

    pub fn from_string(s: String) -> Self {
        Self::new(s.into(), [].into())
    }

    pub fn write_to(
        &self,
        f: &mut impl io::Write,
        arguments: impl ExactSizeIterator<Item = Bits>,
        time_format: &TimeFormat,
        time_resolution: TimeResolution,
    ) -> io::Result<()> {
        assert_eq!(self.arguments.len(), arguments.len());
        let mut at = 0;
        for ((arg_at, arg_fmt), arg_bits) in self.arguments.iter().zip(arguments) {
            f.write_all(self.content[at..*arg_at].as_bytes())?;
            at = *arg_at;
            format_bits(
                f,
                &arg_bits,
                arg_fmt.padding,
                arg_fmt.base,
                arg_fmt.precision,
                arg_fmt.signed,
                arg_fmt.prefix,
                arg_fmt.time_unit,
                time_format,
                time_resolution,
            )?;
        }

        f.write_all(self.content[at..].as_bytes())
    }

    pub fn content(&self) -> &str {
        &self.content
    }
    pub fn arguments(&self) -> &[(usize, DynFormatArgument)] {
        &self.arguments
    }
}

enum RealFormatKind {
    /// `%f` / `%F`
    Decimal,

    /// `%e` / `%E`
    Exponential,

    /// `%g` / `%G`
    Adaptive,
}

/// Formatting for the real format specifiers.
struct FormatReal {
    kind: RealFormatKind,
    value: f64,
    precision: Option<NonMaxU32>,
    minimum_field_width: Option<u32>,
}

impl fmt::Display for FormatReal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            kind,
            value,
            precision,
            minimum_field_width,
        } = self;
        let (v, p, w) = (*value, *precision, *minimum_field_width);
        let p = p.map_or(6, |p| p.get() as usize);
        let w = w.map(|w| w as usize);

        if !v.is_finite() {
            let lit = if v.is_nan() {
                "nan"
            } else if v.is_sign_positive() {
                "inf"
            } else {
                "-inf"
            };
            return match w {
                None => f.write_str(lit),
                Some(w) => write!(f, "{lit:>w$}"),
            };
        }

        match kind {
            RealFormatKind::Decimal => match w {
                None => write!(f, "{v:.p$}"),
                Some(w) => write!(f, "{v:>w$.p$}"),
            },
            RealFormatKind::Exponential => {
                let raw = format!("{v:.p$e}");
                let (mant, exp) = raw
                    .split_once('e')
                    .expect("Format should always output and `e`");
                let x: i32 = exp.parse().unwrap();
                let v = format!(
                    "{mant}e{}{:02}",
                    if x < 0 { '-' } else { '+' },
                    x.unsigned_abs()
                );

                match w {
                    None => f.write_str(&v),
                    Some(w) => write!(f, "{v:>w$}"),
                }
            }
            RealFormatKind::Adaptive => {
                let p = p.max(1);
                let e = format!("{:.*e}", p - 1, v);
                let (mant, exp) = e
                    .split_once('e')
                    .expect("`e` format should always contain `e`");
                let x: i32 = exp.parse().unwrap();

                fn strip_decimals(s: &mut String) {
                    if s.contains('.') {
                        while s.ends_with('0') {
                            s.pop();
                        }
                        if s.ends_with('.') {
                            s.pop();
                        }
                    }
                }

                let s = if x >= -4 && x < p as i32 {
                    let mut s = format!("{:.*}", (p as i32 - 1 - x) as usize, v);
                    strip_decimals(&mut s);
                    s
                } else {
                    let mut m = mant.to_string();
                    strip_decimals(&mut m);
                    format!(
                        "{m}e{}{:02}",
                        if x.is_negative() { '-' } else { '+' },
                        x.unsigned_abs()
                    )
                };

                match w {
                    Some(w) => write!(f, "{s:>w$}"),
                    None => f.write_str(&s),
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn format_bits(
    f: &mut impl io::Write,
    bits: &Bits,
    padding: Padding,
    base: Base,
    precision: Option<NonMaxU32>,
    signed: bool,
    prefix: bool,
    time_unit: TimeResolution,
    time_format: &TimeFormat,
    time_resolution: TimeResolution,
) -> io::Result<()> {
    let base = match base {
        Base::Adaptive => {
            if bits.contains_special() && bits.count_ones() + bits.count_ones() != 0 {
                BitsFormatBase::Binary
            } else {
                BitsFormatBase::LowerHex
            }
        }
        Base::Binary => BitsFormatBase::Binary,
        Base::Octal => BitsFormatBase::Octal,
        Base::Hexadecimal => BitsFormatBase::LowerHex,
        Base::Decimal => BitsFormatBase::Decimal,
        Base::Float => {
            return write!(
                f,
                "{}",
                FormatReal {
                    kind: RealFormatKind::Decimal,
                    value: bits.extract_exact_f64().unwrap(),
                    precision,
                    minimum_field_width: match padding {
                        Padding::NoPadding | Padding::ZeroPaddedToSize => None,
                        Padding::ZeroPaddedTo(w) => Some(w),
                    }
                }
            );
        }
        Base::Exponential => {
            return write!(
                f,
                "{}",
                FormatReal {
                    kind: RealFormatKind::Exponential,
                    value: bits.extract_exact_f64().unwrap(),
                    precision,
                    minimum_field_width: match padding {
                        Padding::NoPadding | Padding::ZeroPaddedToSize => None,
                        Padding::ZeroPaddedTo(w) => Some(w),
                    }
                }
            );
        }
        Base::FloatAdaptive => {
            return write!(
                f,
                "{}",
                FormatReal {
                    kind: RealFormatKind::Adaptive,
                    value: bits.extract_exact_f64().unwrap(),
                    precision,
                    minimum_field_width: match padding {
                        Padding::NoPadding | Padding::ZeroPaddedToSize => None,
                        Padding::ZeroPaddedTo(w) => Some(w),
                    }
                }
            );
        }
        Base::Ascii => {
            #[inline(always)]
            fn to_hex(b: u8) -> u8 {
                static TABLE: [u8; 16] = [
                    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A', b'B', b'C',
                    b'D', b'E', b'F',
                ];
                TABLE[b as usize]
            }
            for b in bits.be_bytes_iter() {
                match b {
                    None => f.write_all(b" ")?,
                    Some(b) => {
                        if b == 0 {
                            break;
                        }

                        if b.is_ascii() {
                            write!(f, "{}", char::from(b))?;
                        } else {
                            f.write_all(&[b'\\', b'x', to_hex(b >> 4), to_hex(b & 0xF)])?;
                        }
                    }
                }
            }
            return Ok(());
        }
        Base::Time | Base::TimeReal => {
            const POW10_U128: [u128; 39] = {
                let mut v = [0u128; _];
                let mut i = 0u32;
                while i < 39 {
                    v[i as usize] = 10u128.pow(i);
                    i += 1;
                }
                v
            };

            // %t provides the value in ticks of the module's time unit
            let unit_to_reso =
                (time_unit.pow10_over_fs() as i32 - time_resolution.pow10_over_fs() as i32).max(0);
            let ticks_per_unit = POW10_U128[unit_to_reso as usize];

            let ticks: u128 = match base {
                Base::TimeReal => {
                    let v = bits
                        .extract_exact_f64()
                        .expect("Should get a genuine f64 for this base");

                    let scaled = (v * ticks_per_unit as f64).round();
                    if !scaled.is_finite() || scaled <= 0.0 {
                        0
                    } else if scaled >= u128::MAX as f64 {
                        u128::MAX
                    } else {
                        scaled as u128
                    }
                }
                Base::Time => {
                    // @Incorrect. This truncates values that are too big. I am not
                    // sure what to do with those.
                    let value = bits.clone().truncate_or_zero_extend(VSIZE_64);
                    let Some(units) = value.extract_exact_u64() else {
                        if value.contains_unknown() {
                            write!(f, "x")?;
                        } else {
                            write!(f, "z")?;
                        }
                        return Ok(());
                    };
                    units as u128 * ticks_per_unit
                }
                _ => unreachable!(),
            };

            let d = time_format.time_unit.pow10_over_fs() as i32
                - time_resolution.pow10_over_fs() as i32;
            let req_p = time_format.precision_number as i32;
            let eff_p = req_p.min(d.max(0)); // digits we actually compute
            let zeros = (req_p - eff_p) as usize; // digits that are always '0'
            let e = eff_p - d; // e <= 15, so u128 is always enough

            let scaled: u128 = if e >= 0 {
                ticks * POW10_U128[e as usize]
            } else {
                let div = POW10_U128[(-e) as usize];
                (ticks + div / 2) / div // round half up; time is non-negative
            };

            let unit = POW10_U128[eff_p as usize];
            let (int_part, frac_part) = (scaled / unit, scaled % unit);

            let mut int_part_num_digits = 1;
            let mut v = int_part;
            while v >= 10 {
                v /= 10;
                int_part_num_digits += 1;
            }
            let len = int_part_num_digits
                + if req_p > 0 { 1 + req_p as usize } else { 0 }
                + time_format.suffix_string.len();

            let width_override = match padding {
                Padding::ZeroPaddedToSize => None,
                Padding::ZeroPaddedTo(n) => Some(n),
                Padding::NoPadding => Some(0),
            };
            let width = width_override.unwrap_or(time_format.minimum_field_width) as usize;

            for _ in len..width {
                f.write_all(b" ")?;
            }

            write!(f, "{int_part}")?;
            if req_p > 0 {
                f.write_all(b".")?;
                // Emit exactly `req_p` fractional digits: `eff_p` computed ones
                // followed by `zeros` always-zero ones. Guard `eff_p == 0`, since
                // formatting an integer still prints a leading "0" at width 0.
                if eff_p > 0 {
                    write!(f, "{frac_part:0>eff$}", eff = eff_p as usize)?;
                }
                for _ in 0..zeros {
                    f.write_all(b"0")?;
                }
            }
            write!(f, "{}", &time_format.suffix_string)?;

            return Ok(());
        }
    };
    let mut options = BitsFormatOptions {
        base,
        prefix,
        separator: None,
        signed,
        ..Default::default()
    };

    if options.base != BitsFormatBase::Decimal {
        options.fill = '0';
    }
    match padding {
        Padding::ZeroPaddedToSize => {
            options.width = BitsFormatWidth::Expand;
            options.align = Some(Alignment::Right);
        }
        Padding::ZeroPaddedTo(size) => {
            options.width = BitsFormatWidth::Minimum(size as usize);
            options.align = None;
        }
        Padding::NoPadding => {}
    }

    write!(f, "{}", bits.display(&options))
}

impl DynFormatString {
    pub fn display_format<'a>(&'a self) -> impl fmt::Display + 'a {
        struct Escape<'a, W>(&'a mut W);
        impl<'a, W: fmt::Write> fmt::Write for Escape<'a, W> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                let mut last_write = 0;
                for (i, c) in s.char_indices() {
                    match c {
                        '\n' => {
                            self.0.write_str(&s[last_write..i])?;
                            self.0.write_str("\\n")?;
                            last_write = i + 1;
                        }
                        '\t' => {
                            self.0.write_str(&s[last_write..i])?;
                            self.0.write_str("\\t")?;
                            last_write = i + 1;
                        }
                        '\r' => {
                            self.0.write_str(&s[last_write..i])?;
                            self.0.write_str("\\r")?;
                            last_write = i + 1;
                        }
                        '"' => {
                            self.0.write_str(&s[last_write..i])?;
                            self.0.write_str("\\\"")?;
                            last_write = i + 1;
                        }
                        '{' => {
                            self.0.write_str(&s[last_write..i])?;
                            self.0.write_str("{{")?;
                            last_write = i + 1;
                        }
                        '\\' => {
                            self.0.write_str(&s[last_write..i])?;
                            self.0.write_str("\\\\")?;
                            last_write = i + 1;
                        }
                        _ => {}
                    }
                }
                self.0.write_str(&s[last_write..])
            }
        }

        struct D<'a>(&'a DynFormatString);
        impl<'a> fmt::Display for D<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_char('"')?;

                let mut last = 0;
                for (at, _options) in self.0.arguments() {
                    let at = *at;
                    Escape(f).write_str(&self.0.content[last..at])?;
                    last = at;

                    f.write_str("{}")?;
                }

                Escape(f).write_str(&self.0.content[last..])?;
                f.write_char('"')?;
                Ok(())
            }
        }
        D(self)
    }
}
