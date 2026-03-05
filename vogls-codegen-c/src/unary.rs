use std::io;

use vogls_ir::{LogicMode, SCALAR_VSIZE};

use crate::{CVar, INDENT};

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
                "{INDENT}for (int i = 0; i < {}; ++i) {d}[i] = {s}[i];",
                arr_size - 1
            )?;
            writeln!(
                f,
                "{INDENT}{d}[{i}] = {s}[{i}] ^ 0x{msbs_mask:x};",
                i = arr_size - 1
            )?;
        }
        (LogicMode::FourValue, _) => todo!(),
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
        (LogicMode::FourValue, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_reduce_and(f: &mut impl io::Write, dst: CVar, src: CVar) -> io::Result<()> {
    assert_eq!(dst.ty.mode, src.ty.mode);
    assert_eq!(dst.ty.size, SCALAR_VSIZE);

    let size = src.ty.size;
    let msbs_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << dst.ty.size.get()) - 1
    };

    let (d, s) = (dst.ident, src.ident);
    match (dst.ty.mode, src.ty.array_size()) {
        (LogicMode::TwoValue, None) => {
            writeln!(f, "{INDENT}{d} = (uint8_t)({s} == 0x{msbs_mask:x});")?
        }
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(
                f,
                "{INDENT}{d} = (uint8_t)({s}[{i}] == 0x{msbs_mask:x});",
                i = arr_size - 1
            )?;
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {end}; ++i) {d} &= (uint8_t)((~{s}[{end} - i - 1]) == 0);",
                end = arr_size - 1
            )?;
        }
        (LogicMode::FourValue, _) => todo!(),
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
        (LogicMode::FourValue, _) => todo!(),
    }

    Ok(())
}
