use std::fmt::{self, Alignment, Write};
use std::io;

use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};

use crate::Bits;

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
}
impl Base {
    pub fn is_fp_representation(self) -> bool {
        matches!(self, Self::Exponential | Self::Float | Self::FloatAdaptive)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DynFormatArgument {
    pub padding: Padding,
    pub base: Base,
    pub signed: bool,
    pub prefix: bool,
}

impl Default for DynFormatArgument {
    fn default() -> Self {
        Self {
            padding: Default::default(),
            base: Default::default(),
            signed: false,
            prefix: true,
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
                arg_fmt.signed,
                arg_fmt.prefix,
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

pub fn format_bits(
    f: &mut impl io::Write,
    bits: &Bits,
    padding: Padding,
    base: Base,
    signed: bool,
    prefix: bool,
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
        Base::Float => return write!(f, "{}", bits.extract_exact_f64().unwrap()),
        Base::Exponential => return write!(f, "{:e}", bits.extract_exact_f64().unwrap()),
        Base::FloatAdaptive => {
            let v = bits.extract_exact_f64().unwrap();
            let float = format!("{v}");
            let exp = format!("{v:e}");
            let min = if float.len() < exp.len() { float } else { exp };
            return write!(f, "{min}");
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
