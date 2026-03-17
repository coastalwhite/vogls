use std::io;

use vogls_ir::{LogicMode, SCALAR_VSIZE};

use crate::{CVar, INDENT, mask};

pub fn cgc_negate(f: &mut impl io::Write, dst: CVar, src: CVar) -> io::Result<()> {
    assert_eq!(dst.ty.mode, src.ty.mode);
    assert_eq!(dst.ty.size, src.ty.size);

    let size = dst.ty.size;
    let msbs_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << (dst.ty.size.get() % 64)) - 1
    };

    let (d, s) = (dst.ident, src.ident);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = {s} ^ 0x{msbs_mask:x};")?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {}; ++i) {d}[i] = ~{s}[i];",
                arr_size - 1
            )?;
            writeln!(
                f,
                "{INDENT}{d}[{i}] = {s}[{i}] ^ 0x{msbs_mask:x};",
                i = arr_size - 1
            )?;
        }
        // (spc, val) -> (spc, spc & !val)
        (LogicMode::FourValue, None) => {
            writeln!(
                f,
                "{INDENT}{d} = ({s} & 0x{msbs_mask:x}) | ((({s} << {size}) & ~{s}) & 0x{mask:x});",
                mask = mask(src.ty.size.get() * 2)
            )?;
        }
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(
                f,
                "{INDENT}memmove({d}, {s}, {num_words}*sizeof(uint64_t));"
            )?;
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {num_words}; ++i) {d}[{num_words}+i] = {s}[i] & ~{s}[{num_words}+i];"
            )?;
        }
    }

    Ok(())
}

pub fn cgc_reduce_or(f: &mut impl io::Write, dst: CVar, src: CVar) -> io::Result<()> {
    assert_eq!(dst.ty.mode, src.ty.mode);
    assert_eq!(dst.ty.size, SCALAR_VSIZE);

    let (d, s) = (dst.ident, src.ident);
    match (dst.ty.mode, src.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = (uint8_t)({s} != 0);")?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(
                f,
                "{INDENT}{d} = (uint8_t)({s}[{i}] != 0);",
                i = arr_size - 1
            )?;
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {end}; ++i) {d} |= (uint8_t)({s}[{end} - i - 1] != 0);",
                end = arr_size - 1
            )?;
        }

        // z0 = (spc & value) != 0
        // z1 = (spc == mask) | z0
        (LogicMode::FourValue, None) => {
            let size = src.ty.size;
            let spc_mask = mask(src.ty.size.get());
            writeln!(f, "{INDENT}{{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}uint8_t z0 = (({s} << {size}) & {s}) != 0;"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}uint8_t z1 = (({s} & 0x{spc_mask:x}) == 0x{spc_mask:x}) | z0;"
            )?;
            writeln!(f, "{INDENT}{INDENT}{d} = (z0 << 1) | z1;")?;
            writeln!(f, "{INDENT}}}")?;
        }
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            let num_words_m_1 = num_words - 1;
            writeln!(f, "{INDENT}{{")?;
            writeln!(f, "{INDENT}{INDENT}uint8_t z0 = 0, z1 = 1;")?;
            let num_words_loop = if src.ty.size.get() % 64 == 0 {
                num_words
            } else {
                num_words - 1
            };
            if num_words_loop > 0 {
                writeln!(
                    f,
                    "{INDENT}{INDENT}for (int i = 0; i < {num_words_loop}; ++i) {{ z0 |= ({s}[i] & {s}[{num_words}+i]) != 0; z1 &= ~{s}[i] == 0; }}"
                )?;
            }
            if src.ty.size.get() % 64 != 0 {
                let last_i = arr_size - 1;
                let mask = mask(src.ty.size.get() % 64);
                writeln!(
                    f,
                    "{INDENT}{INDENT}z0 |= ({s}[{num_words_m_1}] & {s}[{last_i}]) != 0;"
                )?;
                writeln!(
                    f,
                    "{INDENT}{INDENT}z1 &= {s}[{num_words_m_1}] == 0x{mask:x};"
                )?;
            }
            writeln!(f, "{INDENT}{INDENT}z1 |= z0;")?;
            writeln!(f, "{INDENT}{INDENT}{d} = (z0 << 1) | z1;")?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_reduce_and(f: &mut impl io::Write, dst: CVar, src: CVar) -> io::Result<()> {
    assert_eq!(dst.ty.mode, src.ty.mode);
    assert_eq!(dst.ty.size, SCALAR_VSIZE);

    let (d, s) = (dst.ident, src.ident);
    match (dst.ty.mode, src.ty.array_size()) {
        (LogicMode::TwoValue, None) => {
            let msbs_mask = mask(src.ty.size.get());
            writeln!(f, "{INDENT}{d} = (uint8_t)({s} == 0x{msbs_mask:x});")?
        }
        (LogicMode::TwoValue, Some(arr_size)) => {
            let msbs_mask = if src.ty.size.get() % 64 == 0 {
                u64::MAX
            } else {
                mask(src.ty.size.get() % 64)
            };
            writeln!(
                f,
                "{INDENT}{d} = (uint8_t)({s}[{i}] == 0x{msbs_mask:x});",
                i = arr_size - 1
            )?;
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {end}; ++i) {d} &= (uint8_t)(({s}[{end_idx}-i]) == ~0);",
                end = arr_size - 1,
                end_idx = arr_size - 2
            )?;
        }

        // z1 = &spc | (spc & !value != 0);
        // z0 = &spc & &value;
        (LogicMode::FourValue, None) => {
            let size = src.ty.size;
            let spc_mask = mask(src.ty.size.get());
            let val_mask = spc_mask << size.get();
            writeln!(f, "{INDENT}{{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}uint8_t redandspc = ({s} & 0x{spc_mask:x}) == 0x{spc_mask:x};"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}uint8_t z0 = redandspc & (({s} & 0x{val_mask:x}) == 0x{val_mask:x});"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}uint8_t z1 = redandspc | ((({s} << {size}) & ({s} ^ 0x{val_mask:x})) != 0);"
            )?;
            writeln!(f, "{INDENT}{INDENT}{d} = (z0 << 1) | z1;")?;
            writeln!(f, "{INDENT}}}")?;
        }
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(f, "{INDENT}{{")?;
            writeln!(f, "{INDENT}{INDENT}uint8_t redandspc = 1, z0 = 1, z1 = 0;")?;

            let num_words_loop = if src.ty.size.get() % 64 == 0 {
                num_words
            } else {
                num_words - 1
            };
            if num_words_loop > 0 {
                writeln!(
                    f,
                    "{INDENT}{INDENT}for (int i = 0; i < {num_words_loop}; ++i) {{ redandspc &= ~{s}[i] == 0; z0 &= ~{s}[{num_words}+i] == 0; z1 |= ({s}[i] & ~{s}[{num_words}+i]) != 0; }}"
                )?;
            }
            if src.ty.size.get() % 64 != 0 {
                let mask = mask(src.ty.size.get() % 64);
                let last_i = arr_size - 1;
                writeln!(
                    f,
                    "{INDENT}{INDENT}redandspc &= {s}[{num_words_loop}] == 0x{mask:x};"
                )?;
                writeln!(f, "{INDENT}{INDENT}z0 &= {s}[{last_i}] == 0x{mask:x};;")?;
                writeln!(
                    f,
                    "{INDENT}{INDENT}z1 |= ({s}[{num_words_loop}] & ~{s}[{last_i}]) != 0;"
                )?;
            }
            writeln!(f, "{INDENT}{INDENT}z1 |= redandspc;")?;
            writeln!(f, "{INDENT}{INDENT}z0 &= redandspc;")?;
            writeln!(f, "{INDENT}{INDENT}{d} = (z0 << 1) | z1;")?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_reduce_xor(f: &mut impl io::Write, dst: CVar, src: CVar) -> io::Result<()> {
    assert_eq!(dst.ty.mode, src.ty.mode);
    assert_eq!(dst.ty.size, SCALAR_VSIZE);

    let (d, s) = (dst.ident, src.ident);
    match (dst.ty.mode, src.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(
            f,
            "{INDENT}{d} = (uint8_t)(popcount{}({s}) % 2 == 1);",
            src.ty.element_type().size()
        )?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(
                f,
                "{INDENT}{{ uint32_t cnt = 0; for (int i = 0; i < {arr_size}; ++i) {{ cnt += popcount64({s}[i]); }} {d} = (uint8_t)(cnt % 2 == 1); }}",
            )?;
        }

        // z0 = z1 & (popcount(value) % 2 == 0)
        // z1 = spc == mask
        (LogicMode::FourValue, None) => {
            let size = src.ty.size;
            let spc_mask = mask(src.ty.size.get());
            let val_mask = mask(src.ty.size.get()) << size.get();
            writeln!(f, "{INDENT}{{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}uint8_t z1 = ({s} & 0x{spc_mask:x}) == 0x{spc_mask:x};"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}uint8_t z0 = z1 & (popcount{}({s} & 0x{val_mask:x}) % 2 == 1);",
                src.ty.element_type().size()
            )?;
            writeln!(f, "{INDENT}{INDENT}{d} = (z0 << 1) | z1;")?;
            writeln!(f, "{INDENT}}}")?;
        }
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            let num_words_m_1 = num_words - 1;
            writeln!(f, "{INDENT}{{")?;
            writeln!(f, "{INDENT}{INDENT}uint8_t z0, z1 = 1;")?;
            writeln!(f, "{INDENT}{INDENT}uint32_t cnt = 0;")?;
            let num_words_loop = if src.ty.size.get() % 64 == 0 {
                num_words
            } else {
                num_words - 1
            };
            if num_words_loop > 0 {
                writeln!(
                    f,
                    "{INDENT}{INDENT}for (int i = 0; i < {num_words_loop}; ++i) {{ cnt += popcount64({s}[{num_words}+i]); z1 &= ~{s}[i] == 0; }}"
                )?;
            }
            if src.ty.size.get() % 64 != 0 {
                let last_i = arr_size - 1;
                let mask = mask(src.ty.size.get() % 64);
                writeln!(f, "{INDENT}{INDENT}cnt += popcount64({s}[{last_i}]);")?;
                writeln!(
                    f,
                    "{INDENT}{INDENT}z1 &= {s}[{num_words_m_1}] == 0x{mask:x};"
                )?;
            }
            writeln!(f, "{INDENT}{INDENT}z0 = z1 & (cnt % 2 == 1);")?;
            writeln!(f, "{INDENT}{INDENT}{d} = (z0 << 1) | z1;")?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}
