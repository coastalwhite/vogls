use std::fmt::Alignment;
use std::io;

use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};

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
) -> io::Result<()> {
    let mut options = BitsFormatOptions::default();

    options.base = match base {
        Base::Binary => BitsFormatBase::Binary,
        Base::Octal => BitsFormatBase::Octal,
        Base::Hexadecimal => BitsFormatBase::LowerHex,
        Base::Decimal => BitsFormatBase::Decimal,
    };

    options.separator = None;
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
