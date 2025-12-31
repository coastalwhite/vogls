use std::io;

use vogls_bits::VectorSize;
use vogls_bits::load::load_partial_u64;

use crate::Bits;

#[derive(Debug, Clone)]
pub struct DynFormatString {
    content: Box<str>,
    arguments: Box<[(usize, DynFormatArgument)]>,
}

#[derive(Debug, Clone, Default)]
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

            let num_chars = arg_fmt.base.num_chars(&arg_bits);
            let num_max_chars = arg_fmt.base.num_max_chars(arg_bits.size());
            assert!(num_chars <= num_max_chars);

            match arg_fmt.padding {
                Padding::ZeroPaddedToSize => {
                    for _ in num_chars..num_max_chars {
                        f.write_all(&[b'0'])?;
                    }
                }
                Padding::ZeroPaddedTo(size) => {
                    for _ in num_chars.min(size)..size {
                        f.write_all(&[b'0'])?;
                    }
                }
                Padding::NoPadding => {}
            }

            match &arg_bits {
                Bits::Small(v, _) => match arg_fmt.base {
                    Base::Binary => write!(f, "{v:b}"),
                    Base::Octal => write!(f, "{v:o}"),
                    Base::Hexadecimal => write!(f, "{v:x}"),
                    Base::Decimal => write!(f, "{v:x}"),
                }?,
                Bits::Big(_, v) => match arg_fmt.base {
                    Base::Binary => {
                        for b in v.iter().rev() {
                            if *b == 0 {
                                continue;
                            }
                            write!(f, "{b:b}")?
                        }
                    }
                    Base::Octal => {
                        let left_over = arg_bits.size().get() % (6 * 8);
                        let num_full = arg_bits.size().get() / (6 * 8);

                        if left_over != 0 {
                            let v = load_partial_u64(
                                &arg_bits.as_slice()
                                    [arg_bits.as_slice().len() - left_over.div_ceil(8) as usize..],
                                VectorSize::new(left_over).unwrap(),
                            );
                            if v != 0 {
                                write!(f, "{v:o}")?;
                            }
                        }
                        for w in arg_bits.as_slice()[..num_full as usize * 3]
                            .windows(6)
                            .rev()
                        {
                            let v = load_partial_u64(w, VectorSize::new(6 * 8).unwrap());
                            if v != 0 {
                                write!(f, "{v:o}")?;
                            }
                        }
                    }
                    Base::Hexadecimal => {
                        for b in v.iter().rev() {
                            if *b == 0 {
                                continue;
                            }
                            write!(f, "{b:x}")?
                        }
                    }
                    Base::Decimal => todo!(),
                },
            }
        }

        f.write_all(self.content[at..].as_bytes())
    }
}
