use std::fmt::Alignment;
use std::io;

use vogls_bits::VectorSize;
use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
use vogls_bits::load::load_partial_u64;

use crate::Bits;

#[derive(Debug, Clone)]
pub struct DynFormatString {
    content: Box<str>,
    arguments: Box<[(usize, DynFormatArgument)]>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Padding {
    ZeroPaddedToSize,
    ZeroPaddedTo(u32),
    #[default]
    NoPadding,
}
#[derive(Debug, Clone, Copy, Default)]
pub enum Base {
    Binary,
    Octal,
    #[default]
    Hexadecimal,
    Decimal,
}

impl Base {
    fn num_chars(self, v: &Bits) -> u32 {
        match self {
            Base::Binary => v.size().get() - v.leading_zeroes(),
            Base::Octal => (v.size().get() - v.leading_zeroes()).div_ceil(3),
            Base::Hexadecimal => (v.size().get() - v.leading_zeroes()).div_ceil(4),
            Base::Decimal => v.clog10(),
        }
        .max(1)
    }

    fn num_max_chars(self, v: VectorSize) -> u32 {
        match self {
            Base::Binary => v.get(),
            Base::Octal => v.get().div_ceil(3),
            Base::Hexadecimal => v.get().div_ceil(4),
            Base::Decimal => ((2.0f64.log10() * f64::from(v.get())).ceil()) as u32,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DynFormatArgument {
    pub padding: Padding,
    pub base: Base,
}

impl DynFormatString {
    pub fn new(content: Box<str>, arguments: Box<[(usize, DynFormatArgument)]>) -> Self {
        assert!(
            arguments.iter().all(|(a, _)| *a <= content.len())
                && arguments.windows(2).all(|a| a[0].0 <= a[1].0)
        );

        Self { content, arguments }
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
            format_bits(f, &arg_bits, arg_fmt.padding, arg_fmt.base)?;
        }

        f.write_all(self.content[at..].as_bytes())
    }
}

pub fn format_bits(
    f: &mut impl io::Write,
    bits: &Bits,
    padding: Padding,
    base: Base,
) -> io::Result<()> {
    let mut options = BitsFormatOptions::default();

    options.base = match base {
        Base::Binary => BitsFormatBase::Binary,
        Base::Octal => BitsFormatBase::Octal,
        Base::Hexadecimal => BitsFormatBase::LowerHex,
        Base::Decimal => BitsFormatBase::Decimal,
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
