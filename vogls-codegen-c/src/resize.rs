use std::io;

use vogls_ir::LogicMode;

use crate::mask;

use super::{CVar, INDENT};

pub fn cgc_copy(f: &mut impl io::Write, dst: CVar, src: CVar) -> io::Result<()> {
    assert_eq!(dst.ty.mode, src.ty.mode);
    assert_eq!(dst.ty.size, src.ty.size);

    let (d, s) = (dst.ident, src.ident);
    match dst.ty.array_size() {
        Some(arr_size) => writeln!(
            f,
            "{INDENT}for (int i = 0; i < {arr_size}; ++i) {d}[i] = {s}[i];"
        ),
        None => writeln!(f, "{INDENT}{d} = {s};"),
    }
}

pub fn cgc_truncate(f: &mut impl io::Write, dst: CVar, src: CVar) -> io::Result<()> {
    assert_eq!(dst.ty.mode, src.ty.mode);
    assert!(dst.ty.size <= src.ty.size);

    if dst.ty.size == src.ty.size {
        return cgc_copy(f, dst, src);
    }

    let msbs_mask = if dst.ty.size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << dst.ty.size.get()) - 1
    };

    let (d, s) = (dst.ident, src.ident);
    match (dst.ty.mode, dst.ty.array_size(), src.ty.array_size()) {
        (LogicMode::TwoValue, None, None) => writeln!(
            f,
            "{INDENT}{d} = ({})({s} & 0x{msbs_mask:x});",
            dst.ty.element_type()
        )?,
        (LogicMode::TwoValue, None, Some(_)) => writeln!(
            f,
            "{INDENT}{d} = ({})({s}[0] & 0x{msbs_mask:x});",
            dst.ty.element_type()
        )?,
        (LogicMode::TwoValue, Some(_), None) => unreachable!(),
        (LogicMode::TwoValue, Some(dst_arr_size), Some(_)) => {
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {}; ++i) {d}[i] = {s}[i];",
                dst_arr_size - 1
            )?;
            writeln!(
                f,
                "{INDENT}{d}[{i}] = {s}[{i}] & {msbs_mask:x};",
                i = dst_arr_size - 1
            )?;
        }
        (LogicMode::FourValue, _, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_zero_extend(f: &mut impl io::Write, dst: CVar, src: CVar) -> io::Result<()> {
    assert_eq!(dst.ty.mode, src.ty.mode);
    assert!(dst.ty.size >= src.ty.size);

    if dst.ty.size == src.ty.size {
        return cgc_copy(f, dst, src);
    }

    let (d, s) = (dst.ident, src.ident);
    match (dst.ty.mode, dst.ty.array_size(), src.ty.array_size()) {
        (LogicMode::TwoValue, None, None) => {
            writeln!(f, "{INDENT}{d} = ({}){s};", dst.ty.element_type())?
        }
        (LogicMode::TwoValue, Some(dst_arr_size), None) => {
            writeln!(f, "{INDENT}{d}[0] = (uint64_t)({s});",)?;
            writeln!(
                f,
                "{INDENT}for (int i = 1; i < {dst_arr_size}; ++i) {d}[i] = 0;",
            )?;
        }
        (LogicMode::TwoValue, Some(dst_arr_size), Some(src_arr_size)) => {
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {src_arr_size}; ++i) {d}[i] = {s}[i];",
            )?;
            writeln!(
                f,
                "{INDENT}for (int i = {src_arr_size}; i < {dst_arr_size}; ++i) {d}[i] = 0;",
            )?;
        }

        (_, None, Some(_)) => unreachable!(),
        (LogicMode::FourValue, _, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_sign_extend(f: &mut impl io::Write, dst: CVar, src: CVar) -> io::Result<()> {
    assert_eq!(dst.ty.mode, src.ty.mode);
    assert!(dst.ty.size >= src.ty.size);

    if dst.ty.size == src.ty.size {
        return cgc_copy(f, dst, src);
    }

    let (d, s) = (dst.ident, src.ident);
    match (dst.ty.mode, dst.ty.array_size(), src.ty.array_size()) {
        (LogicMode::TwoValue, None, None) => writeln!(
            f,
            "{INDENT}{d} = (({unsigned_elem_ty})(((({signed_elem_ty}){s}) << {shift}) >> {shift})) & 0x{mask:x};",
            unsigned_elem_ty = dst.ty.element_type(),
            signed_elem_ty = src.ty.element_type().signed_ty_str(),
            shift = src.ty.element_type().size().get() - src.ty.size.get(),
            mask = mask(dst.ty.size.get()),
        )?,
        (LogicMode::TwoValue, Some(dst_arr_size), None) => {
            todo!()
        }
        (LogicMode::TwoValue, Some(dst_arr_size), Some(src_arr_size)) => {
            todo!()
        }

        (_, None, Some(_)) => unreachable!(),
        (LogicMode::FourValue, _, _) => todo!(),
    }

    Ok(())
}
