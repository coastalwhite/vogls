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
        (1u64 << (dst.ty.size.get() % 64)) - 1
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
        (LogicMode::TwoValue, Some(dst_arr_size), Some(_)) => {
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {}; ++i) {d}[i] = {s}[i];",
                dst_arr_size - 1
            )?;
            writeln!(
                f,
                "{INDENT}{d}[{i}] = {s}[{i}] & 0x{msbs_mask:x};",
                i = dst_arr_size - 1
            )?;
        }
        (LogicMode::FourValue, None, None) => {
            let spc_mask = msbs_mask;
            let val_mask = msbs_mask << dst.ty.size.get();
            writeln!(
                f,
                "{INDENT}{d} = ({dst_ty_elem})({s} & 0x{spc_mask:x}) | ({dst_ty_elem})(({s} >> {shift}) & 0x{val_mask:x});",
                dst_ty_elem = dst.ty.element_type(),
                shift = src.ty.size.get() - dst.ty.size.get(),
            )?
        }
        (LogicMode::FourValue, None, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(
                f,
                "{INDENT}{d} = ({dst_ty_elem})({s}[0] & 0x{msbs_mask:x}) | ({dst_ty_elem})(({s}[{num_words}] & 0x{msbs_mask:x}) << {shift});",
                dst_ty_elem = dst.ty.element_type(),
                shift = dst.ty.size.get(),
            )?
        }
        (LogicMode::FourValue, Some(dst_arr_size), Some(src_arr_size)) => {
            let dst_num_words = dst_arr_size / 2;
            let src_num_words = src_arr_size / 2;

            let num_copy_words = if dst.ty.size.get() % 64 == 0 {
                dst_num_words
            } else {
                dst_num_words - 1
            };
            if num_copy_words > 0 {
                writeln!(
                    f,
                    "{INDENT}memmove({d}, {s}, {num_copy_words}*sizeof(uint64_t));"
                )?;
                writeln!(
                    f,
                    "{INDENT}memmove({d}+{dst_num_words}, {s}+{src_num_words}, {num_copy_words}*sizeof(uint64_t));"
                )?;
            }
            if dst.ty.size.get() % 64 != 0 {
                let spc_i = dst_num_words - 1;
                let dst_val_i = dst_arr_size - 1;
                let src_val_i = src_num_words + spc_i;
                writeln!(f, "{INDENT}{d}[{spc_i}] = {s}[{spc_i}] & 0x{msbs_mask:x};")?;
                writeln!(
                    f,
                    "{INDENT}{d}[{dst_val_i}] = {s}[{src_val_i}] & 0x{msbs_mask:x};"
                )?;
            }
        }

        (_, Some(_), None) => unreachable!(),
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

        (LogicMode::FourValue, None, None) => {
            let dst_elem_ty = dst.ty.element_type();
            let src_val_mask = mask(src.ty.size.get());
            let src_spc_mask = src_val_mask << src.ty.size.get();
            let ext_mask = mask(dst.ty.size.get() - src.ty.size.get())
                << (dst.ty.size.get() + src.ty.size.get());
            writeln!(
                f,
                "{INDENT}{d} = (((({dst_elem_ty}){s}) & 0x{src_spc_mask:x}) << {diff_size}) | ((({dst_elem_ty}){s}) & 0x{src_val_mask:x}) | 0x{ext_mask:x};",
                diff_size = dst.ty.size.get() - src.ty.size.get(),
            )?
        }
        (LogicMode::FourValue, Some(dst_arr_size), None) => {
            let src_val_mask = mask(src.ty.size.get());
            let src_spc_mask = src_val_mask << src.ty.size.get();
            let ext_mask = mask(dst.ty.size.get().min(64) - src.ty.size.get()) << src.ty.size.get();
            let num_words = dst_arr_size / 2;
            writeln!(
                f,
                "{INDENT}{d}[0] = ((((uint64_t){s}) & 0x{src_spc_mask:x}) >> {ssize}) | 0x{ext_mask:x};",
                ssize = src.ty.size,
            )?;
            writeln!(
                f,
                "{INDENT}{d}[{num_words}] = ((uint64_t){s}) & 0x{src_val_mask:x};"
            )?;
            if num_words > 1 {
                writeln!(
                    f,
                    "{INDENT}for (int i = 1; i < {num_words}; ++i) {{ {d}[i] = ~0; {d}[{num_words}+i] = 0; }}"
                )?;
                if dst.ty.size.get() % 64 != 0 {
                    let last_i = dst_arr_size - 1;
                    let mask = mask(dst.ty.size.get() % 64);
                    writeln!(f, "{INDENT}d[{last_i}] = 0x{mask:x};")?;
                }
            }
        }
        (LogicMode::FourValue, Some(dst_arr_size), Some(src_arr_size)) => {
            let src_num_words = src_arr_size / 2;
            let dst_num_words = dst_arr_size / 2;
            writeln!(
                f,
                "{INDENT}memmove({d}, {s}, {src_num_words}*sizeof(uint64_t));"
            )?;
            writeln!(
                f,
                "{INDENT}memmove({d}+{dst_num_words}, {s}+{src_num_words}, {src_num_words}*sizeof(uint64_t));"
            )?;
            if src.ty.size.get() % 64 != 0 {
                let ext_mask = mask((dst.ty.size.get() - src.ty.size.get()).min(63))
                    << (src.ty.size.get() % 64);
                writeln!(f, "{INDENT}{d}[{src_num_words} - 1] |= 0x{ext_mask:x};")?;
            }
            if src_arr_size > dst_arr_size {
                let diff_num_words = dst_arr_size - src_arr_size;
                let sum_num_words = dst_arr_size + src_arr_size;
                writeln!(
                    f,
                    "{INDENT}memset({d}+{src_num_words}, 0xFF, {diff_num_words}*sizeof(uint64_t));"
                )?;
                if dst.ty.size.get() % 64 != 0 {
                    let last_i = dst_arr_size - 1;
                    let mask = mask(dst.ty.size.get() % 64);
                    writeln!(f, "{INDENT}{d}[{last_i} = 0x{mask:x};")?;
                }
                writeln!(
                    f,
                    "{INDENT}memset({d}+{sum_num_words}, 0, {diff_num_words}*sizeof(uint64_t));"
                )?;
            }
        }

        (_, None, Some(_)) => unreachable!(),
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
            writeln!(
                f,
                "{INDENT}{d}[0] = ({unsigned_elem_ty})(((({signed_elem_ty}){s}) << {shift}) >> {shift});",
                unsigned_elem_ty = dst.ty.element_type(),
                signed_elem_ty = src.ty.element_type().signed_ty_str(),
                shift = src.ty.element_type().size().get() - src.ty.size.get(),
            )?;
            let dst_arr_size_m_1 = dst_arr_size - 1;
            write!(
                f,
                "{INDENT}{{ uint64_t sign_mask = (!(((uint64_t){s}) >> {shift})) - 1; ",
                shift = src.ty.size.get() - 1,
            )?;
            if dst_arr_size > 2 {
                write!(
                    f,
                    "for (int i = 1; i < {dst_arr_size_m_1}; ++i) {{ {d}[i] = sign_mask; }} "
                )?;
            }
            writeln!(
                f,
                "{d}[{dst_arr_size_m_1}] = sign_mask & 0x{mask:x}; }}",
                mask = if dst.ty.size.get() % 64 == 0 {
                    u64::MAX
                } else {
                    mask(dst.ty.size.get() % 64)
                }
            )?;
        }
        (LogicMode::TwoValue, Some(dst_arr_size), Some(src_arr_size)) => {
            todo!()
        }

        (_, None, Some(_)) => unreachable!(),
        (LogicMode::FourValue, _, _) => todo!(),
    }

    Ok(())
}
