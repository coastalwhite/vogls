use std::io;

use vogls_bits::util::saturating_rem;
use vogls_ir::LogicMode;

use crate::{CExpr, mask};

use super::INDENT;

pub fn cgc_copy(f: &mut impl io::Write, dst: CExpr, src: CExpr<'_>) -> io::Result<()> {
    assert_eq!(dst.ty().mode, src.ty().mode);
    assert_eq!(dst.ty().size, src.ty().size);

    let (d, s) = (dst, src);
    match dst.ty().array_size() {
        Some(arr_size) => writeln!(f, "{INDENT}memcpy({d}, {s}, {arr_size}*sizeof(uint64_t));"),
        None => writeln!(f, "{INDENT}{d} = {s};"),
    }
}

pub fn cgc_truncate(f: &mut impl io::Write, dst: CExpr, src: CExpr<'_>) -> io::Result<()> {
    assert_eq!(dst.ty().mode, src.ty().mode);
    assert!(dst.ty().size <= src.ty().size);

    if dst.ty().size == src.ty().size {
        return cgc_copy(f, dst, src);
    }

    let msbs_mask = if dst.ty().size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << (dst.ty().size.get() % 64)) - 1
    };

    let (d, s) = (dst, src);
    match (dst.ty().mode, dst.ty().array_size(), src.ty().array_size()) {
        (LogicMode::TwoValue, None, None) => writeln!(
            f,
            "{INDENT}{d} = ({})({s} & 0x{msbs_mask:x});",
            dst.ty().element_type()
        )?,
        (LogicMode::TwoValue, None, Some(_)) => writeln!(
            f,
            "{INDENT}{d} = ({})({s}[0] & 0x{msbs_mask:x});",
            dst.ty().element_type()
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
            let val_mask = msbs_mask << dst.ty().size.get();
            writeln!(
                f,
                "{INDENT}{d} = ({dst_ty_elem})({s} & 0x{spc_mask:x}) | ({dst_ty_elem})(({s} >> {shift}) & 0x{val_mask:x});",
                dst_ty_elem = dst.ty().element_type(),
                shift = src.ty().size.get() - dst.ty().size.get(),
            )?
        }
        (LogicMode::FourValue, None, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(
                f,
                "{INDENT}{d} = ({dst_ty_elem})({s}[0] & 0x{msbs_mask:x}) | ({dst_ty_elem})(({s}[{num_words}] & 0x{msbs_mask:x}) << {shift});",
                dst_ty_elem = dst.ty().element_type(),
                shift = dst.ty().size.get(),
            )?
        }
        (LogicMode::FourValue, Some(dst_arr_size), Some(src_arr_size)) => {
            let dst_num_words = dst_arr_size / 2;
            let src_num_words = src_arr_size / 2;

            let num_copy_words = if dst.ty().size.get() % 64 == 0 {
                dst_num_words
            } else {
                dst_num_words - 1
            };
            if num_copy_words > 0 {
                writeln!(
                    f,
                    "{INDENT}memcpy({d}, {s}, {num_copy_words}*sizeof(uint64_t));"
                )?;
                writeln!(
                    f,
                    "{INDENT}memcpy({d}+{dst_num_words}, {s}+{src_num_words}, {num_copy_words}*sizeof(uint64_t));"
                )?;
            }
            if dst.ty().size.get() % 64 != 0 {
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

pub fn cgc_zero_extend(f: &mut impl io::Write, dst: CExpr<'_>, src: CExpr<'_>) -> io::Result<()> {
    assert_eq!(dst.ty().mode, src.ty().mode);
    assert!(dst.ty().size >= src.ty().size);

    if dst.ty().size == src.ty().size {
        return cgc_copy(f, dst, src.into());
    }

    let (d, s) = (dst, src);
    match (dst.ty().mode, dst.ty().array_size(), src.ty().array_size()) {
        (LogicMode::TwoValue, None, None) => {
            writeln!(f, "{INDENT}{d} = ({}){s};", dst.ty().element_type())?
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
            let dst_elem_ty = dst.ty().element_type();
            let src_spc_mask = mask(src.ty().size.get());
            let src_val_mask = src_spc_mask << src.ty().size.get();
            let ext_mask = mask(dst.ty().size.get() - src.ty().size.get()) << src.ty().size.get();
            writeln!(
                f,
                "{INDENT}{d} = (((({dst_elem_ty}){s}) & 0x{src_val_mask:x}) << {diff_size}) | ((({dst_elem_ty}){s}) & 0x{src_spc_mask:x}) | 0x{ext_mask:x};",
                diff_size = dst.ty().size.get() - src.ty().size.get(),
            )?
        }
        (LogicMode::FourValue, Some(dst_arr_size), None) => {
            let src_spc_mask = mask(src.ty().size.get());
            let src_val_mask = src_spc_mask << src.ty().size.get();
            let ext_mask =
                mask((dst.ty().size.get() - src.ty().size.get()).min(63)) << src.ty().size.get();
            let num_words = dst_arr_size / 2;
            writeln!(
                f,
                "{INDENT}{d}[0] = (((uint64_t){s}) & 0x{src_spc_mask:x}) | 0x{ext_mask:x};"
            )?;
            writeln!(
                f,
                "{INDENT}{d}[{num_words}] = (((uint64_t){s}) & 0x{src_val_mask:x}) >> {ssize};",
                ssize = src.ty().size,
            )?;
            if num_words > 1 {
                writeln!(
                    f,
                    "{INDENT}for (int i = 1; i < {num_words}; ++i) {{ {d}[i] = ~0; {d}[{num_words}+i] = 0; }}"
                )?;
                if dst.ty().size.get() % 64 != 0 {
                    let last_spc_i = num_words - 1;
                    let mask = mask(dst.ty().size.get() % 64);
                    writeln!(f, "{INDENT}{d}[{last_spc_i}] = 0x{mask:x};")?;
                }
            }
        }
        (LogicMode::FourValue, Some(dst_arr_size), Some(src_arr_size)) => {
            let src_num_words = src_arr_size / 2;
            let dst_num_words = dst_arr_size / 2;
            writeln!(
                f,
                "{INDENT}memcpy({d}, {s}, {src_num_words}*sizeof(uint64_t));"
            )?;
            writeln!(
                f,
                "{INDENT}memcpy({d}+{dst_num_words}, {s}+{src_num_words}, {src_num_words}*sizeof(uint64_t));"
            )?;
            if src.ty().size.get() % 64 != 0 {
                let ext_mask = mask((dst.ty().size.get() - src.ty().size.get()).min(63))
                    << (src.ty().size.get() % 64);
                writeln!(f, "{INDENT}{d}[{src_num_words} - 1] |= 0x{ext_mask:x};")?;
            }
            if dst_arr_size > src_arr_size {
                let diff_num_words = dst_num_words - src_num_words;
                let sum_num_words = dst_num_words + src_num_words;
                writeln!(
                    f,
                    "{INDENT}memset({d}+{src_num_words}, 0xFF, {diff_num_words}*sizeof(uint64_t));"
                )?;
                if dst.ty().size.get() % 64 != 0 {
                    let last_spc_i = dst_num_words - 1;
                    let mask = mask(dst.ty().size.get() % 64);
                    writeln!(f, "{INDENT}{d}[{last_spc_i}] = 0x{mask:x};")?;
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

pub fn cgc_sign_extend(f: &mut impl io::Write, dst: CExpr, src: CExpr) -> io::Result<()> {
    assert_eq!(dst.ty().mode, src.ty().mode);
    assert!(dst.ty().size >= src.ty().size);

    if dst.ty().size == src.ty().size {
        return cgc_copy(f, dst, src.into());
    }

    let (d, s) = (dst, src);
    match (dst.ty().mode, dst.ty().array_size(), src.ty().array_size()) {
        (LogicMode::TwoValue, None, None) => writeln!(
            f,
            "{INDENT}{d} = (({unsigned_elem_ty})((({signed_elem_ty})(({unsigned_elem_ty}){s} << {shift})) >> {shift})) & 0x{mask:x};",
            unsigned_elem_ty = dst.ty().element_type(),
            signed_elem_ty = dst.ty().element_type().signed_ty_str(),
            shift = dst.ty().element_type().size().get() - src.ty().size.get(),
            mask = mask(dst.ty().size.get()),
        )?,
        (LogicMode::TwoValue, Some(dst_arr_size), None) => {
            writeln!(
                f,
                "{INDENT}{d}[0] = (({unsigned_elem_ty})((({signed_elem_ty})(({unsigned_elem_ty}){s} << {shift})) >> {shift}));",
                unsigned_elem_ty = dst.ty().element_type(),
                signed_elem_ty = dst.ty().element_type().signed_ty_str(),
                shift = dst.ty().element_type().size().get() - src.ty().size.get(),
            )?;
            let dst_arr_size_m_1 = dst_arr_size - 1;
            writeln!(f, "{INDENT}{{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}uint8_t sign_mask = !(({s} >> {shift}) & 1) - 1;",
                shift = src.ty().size.get() - 1,
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}memset({d}+1, sign_mask, {dst_arr_size_m_1}*sizeof(uint64_t));"
            )?;
            if dst.ty().size.get() % 64 != 0 {
                let mask = mask(dst.ty().size.get() % 64);
                writeln!(
                    f,
                    "{INDENT}{INDENT}{d}[{dst_arr_size_m_1}] = sign_mask & 0x{mask:x};"
                )?;
            }
            writeln!(f, "{INDENT}}}")?;
        }
        (LogicMode::TwoValue, Some(dst_arr_size), Some(src_arr_size)) => {
            let num_items_main_loop = if src.ty().size.get() % 64 == 0 {
                src_arr_size
            } else {
                src_arr_size - 1
            };
            if num_items_main_loop > 0 {
                writeln!(
                    f,
                    "{INDENT}memcpy({d}, {s}, {num_items_main_loop}*sizeof(uint64_t));"
                )?;
            }
            let last_src_i = src_arr_size - 1;
            if src.ty().size.get() % 64 != 0 {
                writeln!(
                    f,
                    "{INDENT}{d}[{last_src_i}] = (({unsigned_elem_ty})((({signed_elem_ty})(({unsigned_elem_ty}){s}[{last_src_i}] << {shift})) >> {shift}));",
                    unsigned_elem_ty = dst.ty().element_type(),
                    signed_elem_ty = dst.ty().element_type().signed_ty_str(),
                    shift = 64 - (src.ty().size.get() % 64),
                )?;
            }

            let shift = saturating_rem(src.ty().size.get(), 64) - 1;
            if dst_arr_size > src_arr_size {
                let diff_arr_size = dst_arr_size - src_arr_size;
                writeln!(
                    f,
                    "{INDENT}memset({d}+{src_arr_size}, !(({s}[{last_src_i}] >> {shift}) & 1) - 1, {diff_arr_size}*sizeof(uint64_t));"
                )?;
            }
            if dst.ty().size.get() % 64 != 0 {
                let last_dst_i = dst_arr_size - 1;
                let mask = mask(dst.ty().size.get() % 64);
                writeln!(f, "{INDENT}{d}[{last_dst_i}] &= 0x{mask:x};")?;
            }
        }
        (LogicMode::FourValue, None, None) => {
            writeln!(
                f,
                "{INDENT}{d} = (({unsigned_elem_ty})((({signed_elem_ty})(({unsigned_elem_ty}){s} << {shift})) >> {shift})) & 0x{mask:x};",
                unsigned_elem_ty = dst.ty().element_type(),
                signed_elem_ty = dst.ty().element_type().signed_ty_str(),
                shift = dst.ty().element_type().size().get() - src.ty().size.get(),
                mask = mask(dst.ty().size.get()),
            )?;
            writeln!(
                f,
                "{INDENT}{d} |= ((({unsigned_elem_ty})((({signed_elem_ty})(({unsigned_elem_ty}){s} << {lshift})) >> {rshift})) & 0x{mask:x}) << {dst_size};",
                unsigned_elem_ty = dst.ty().element_type(),
                signed_elem_ty = dst.ty().element_type().signed_ty_str(),
                lshift = dst.ty().element_type().size().get() - 2 * src.ty().size.get(),
                rshift = dst.ty().element_type().size().get() - src.ty().size.get(),
                mask = mask(dst.ty().size.get()),
                dst_size = dst.ty().size,
            )?;
        }
        (LogicMode::FourValue, Some(dst_arr_size), None) => {
            let num_words = dst_arr_size / 2;
            let num_words_m_1 = num_words - 1;
            writeln!(
                f,
                "{INDENT}{d}[0] = ({unsigned_elem_ty})((({signed_elem_ty})(({unsigned_elem_ty}){s} << {shift})) >> {shift});",
                unsigned_elem_ty = dst.ty().element_type(),
                signed_elem_ty = dst.ty().element_type().signed_ty_str(),
                shift = dst.ty().element_type().size().get() - src.ty().size.get(),
            )?;
            writeln!(
                f,
                "{INDENT}{d}[{num_words}] = ({unsigned_elem_ty})((({signed_elem_ty})(({unsigned_elem_ty}){s} << {lshift})) >> {rshift});",
                unsigned_elem_ty = dst.ty().element_type(),
                signed_elem_ty = dst.ty().element_type().signed_ty_str(),
                lshift = dst.ty().element_type().size().get() - 2 * src.ty().size.get(),
                rshift = dst.ty().element_type().size().get() - src.ty().size.get(),
            )?;

            if num_words > 1 {
                let num_words_p_1 = num_words + 1;
                writeln!(f, "{INDENT}{{")?;
                writeln!(
                    f,
                    "{INDENT}{INDENT}uint8_t spc_sign_mask = !(({s} >> {shift}) & 1) - 1;",
                    shift = src.ty().size.get() - 1,
                )?;
                writeln!(
                    f,
                    "{INDENT}{INDENT}uint8_t val_sign_mask = !(({s} >> {shift}) & 1) - 1;",
                    shift = 2 * src.ty().size.get() - 1,
                )?;
                writeln!(
                    f,
                    "{INDENT}{INDENT}memset({d}+1, spc_sign_mask, {num_words_m_1}*sizeof(uint64_t));"
                )?;
                writeln!(
                    f,
                    "{INDENT}{INDENT}memset({d}+{num_words_p_1}, val_sign_mask, {num_words_m_1}*sizeof(uint64_t));"
                )?;
                writeln!(f, "{INDENT}}}")?;
            }

            if dst.ty().size.get() % 64 != 0 {
                let last_i = dst_arr_size - 1;
                let mask = mask(dst.ty().size.get() % 64);
                writeln!(f, "{INDENT}{d}[{num_words_m_1}] &= 0x{mask:x};")?;
                writeln!(f, "{INDENT}{d}[{last_i}] &= 0x{mask:x};")?;
            }
        }
        (LogicMode::FourValue, Some(dst_arr_size), Some(src_arr_size)) => {
            let dst_num_words = dst_arr_size / 2;
            let src_num_words = src_arr_size / 2;

            let num_items_main_loop = if src.ty().size.get() % 64 == 0 {
                src_num_words
            } else {
                src_num_words - 1
            };
            if num_items_main_loop > 0 {
                writeln!(
                    f,
                    "{INDENT}memcpy({d}, {s}, {num_items_main_loop}*sizeof(uint64_t));"
                )?;
                writeln!(
                    f,
                    "{INDENT}memcpy({d}+{dst_num_words}, {s}+{src_num_words}, {num_items_main_loop}*sizeof(uint64_t));"
                )?;
            }
            let src_num_words_m_1 = src_num_words - 1;
            let src_arr_size_m_1 = src_arr_size - 1;
            if src.ty().size.get() % 64 != 0 {
                let sum_num_words_m_1 = dst_num_words + src_num_words_m_1;
                writeln!(
                    f,
                    "{INDENT}{d}[{src_num_words_m_1}] = (({unsigned_elem_ty})((({signed_elem_ty})(({unsigned_elem_ty}){s}[{src_num_words_m_1}] << {shift})) >> {shift}));",
                    unsigned_elem_ty = dst.ty().element_type(),
                    signed_elem_ty = dst.ty().element_type().signed_ty_str(),
                    shift = 64 - (src.ty().size.get() % 64),
                )?;
                writeln!(
                    f,
                    "{INDENT}{d}[{sum_num_words_m_1}] = (({unsigned_elem_ty})((({signed_elem_ty})(({unsigned_elem_ty}){s}[{src_arr_size_m_1}] << {shift})) >> {shift}));",
                    unsigned_elem_ty = dst.ty().element_type(),
                    signed_elem_ty = dst.ty().element_type().signed_ty_str(),
                    shift = 64 - (src.ty().size.get() % 64),
                )?;
            }

            let shift = saturating_rem(src.ty().size.get(), 64) - 1;
            if dst_arr_size > src_arr_size {
                let sum_num_words = dst_num_words + src_num_words;
                let diff_num_words = dst_num_words - src_num_words;
                writeln!(
                    f,
                    "{INDENT}memset({d}+{src_num_words}, !(({s}[{src_num_words_m_1}] >> {shift}) & 1) - 1, {diff_num_words}*sizeof(uint64_t));"
                )?;
                writeln!(
                    f,
                    "{INDENT}memset({d}+{sum_num_words}, !(({s}[{src_arr_size_m_1}] >> {shift}) & 1) - 1, {diff_num_words}*sizeof(uint64_t));"
                )?;
            }
            if dst.ty().size.get() % 64 != 0 {
                let dst_num_words_m_1 = dst_num_words - 1;
                let dst_arr_size_m_1 = dst_arr_size - 1;
                let mask = mask(dst.ty().size.get() % 64);
                writeln!(f, "{INDENT}{d}[{dst_num_words_m_1}] &= 0x{mask:x};")?;
                writeln!(f, "{INDENT}{d}[{dst_arr_size_m_1}] &= 0x{mask:x};")?;
            }
        }

        (_, None, Some(_)) => unreachable!(),
    }

    Ok(())
}
