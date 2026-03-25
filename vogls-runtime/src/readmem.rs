use vogls_bits::set_subslice::tv_l_set;
use vogls_bits::{Bits, BitsDataRef, Mode, VectorSize};
use vogls_codegen::{HeapOffset, HeapRef};

fn trim_start_sep(s: &str) -> &str {
    s.trim_start()
}

fn split_sep(s: &str) -> (&str, &str) {
    s.split_once(|c: char| char::is_ascii_whitespace(&c))
        .unwrap_or((s, ""))
}

pub fn read_mem(
    path: &str,
    heap: &mut [u64],
    heap_ref: HeapRef,
    mode: Mode,
    // @TODO: Support
    offset: u32,
    _limit: u32,
    stride: VectorSize,
    binary: bool,
) -> std::io::Result<()> {
    // @Performance. Don't read into a string.
    let s = std::fs::read_to_string(path)?;
    let mut s = &s[..];
    let mut elem = HeapRef {
        offset: HeapOffset {
            bit_offset: heap_ref.offset.bit_offset + offset as usize,
        },
        size: stride,
    };
    while !s.is_empty() {
        s = trim_start_sep(s);
        let bs;
        (bs, s) = split_sep(s);

        let bits = if binary {
            Bits::parse_binary(bs, stride)
        } else {
            Bits::parse_hexadecimal(bs, stride)
        }
        .unwrap();

        // @Performance+Correctness. Use subslice set.
        match (bits.as_data_ref(), mode) {
            (BitsDataRef::InlineTv(v), Mode::TwoValue) => {
                _ = tv_l_set(
                    heap,
                    std::slice::from_ref(&v),
                    VectorSize::new(heap.len() as u32 * 64).unwrap(),
                    elem.offset.bit_offset as u32,
                    bits.size(),
                )
            }
            (BitsDataRef::InlineTv(_), Mode::FourValue) => todo!(),
            (BitsDataRef::SeparateTv(_), Mode::TwoValue) => todo!(),
            (BitsDataRef::SeparateTv(_), Mode::FourValue) => todo!(),
            (BitsDataRef::InlineFv(_, _), Mode::TwoValue) => todo!(),
            (BitsDataRef::InlineFv(_, _), Mode::FourValue) => todo!(),
            (BitsDataRef::SeparateFv(_), Mode::TwoValue) => todo!(),
            (BitsDataRef::SeparateFv(_), Mode::FourValue) => todo!(),
        }
        elem.offset.bit_offset += stride.get() as usize;
    }

    Ok(())
}
