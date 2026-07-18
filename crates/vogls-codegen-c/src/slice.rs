use std::{fmt, io};

use vogls_ir::{INTEGER_VSIZE, LogicMode};
use vogls_utils::saturating_rem;

use super::INDENT;
use crate::{CExpr, CVar};

pub fn slice(
    f: &mut impl io::Write,
    dst: CVar,
    src: CExpr<'_>,
    offset: CExpr<'_>,
) -> io::Result<()> {
    slice_with(f, dst, src, offset, true)
}

pub fn slice_with(
    f: &mut impl io::Write,
    dst: CVar,
    src: CExpr<'_>,
    offset: CExpr<'_>,
    fill_with_x: bool,
) -> io::Result<()> {
    assert_eq!(offset.ty().size, INTEGER_VSIZE);
    if fill_with_x {
        assert_eq!(dst.ty.mode, LogicMode::FourValue);
    } else {
        assert_eq!(src.ty().mode, dst.ty.mode);
    }
    assert!(src.ty().size >= dst.ty.size);

    use LogicMode as M;
    let (d, s, o) = (dst.ident, src, offset);
    let d_size = dst.ty.size;
    let mask = super::mask(saturating_rem(dst.ty.size.get(), 64));

    if offset.ty().mode == LogicMode::FourValue {
        writeln!(f, "{INDENT}if (({o} & 0xFFFFFFFF) != 0xFFFFFFFF) {{")?;
        match dst.ty.array_size() {
            None => {
                if fill_with_x || matches!(dst.ty.mode, LogicMode::TwoValue) {
                    writeln!(f, "{INDENT}{INDENT}{d} = 0;")?
                } else {
                    writeln!(f, "{INDENT}{INDENT}{d} = 0x{mask:X};")?
                }
            }
            Some(arr_size) => {
                if fill_with_x {
                    writeln!(
                        f,
                        "{INDENT}{INDENT}memset({d}, 0, {arr_size}*sizeof(uint64_t));"
                    )?;
                } else {
                    let dst_words = match dst.ty.mode {
                        LogicMode::TwoValue => arr_size,
                        LogicMode::FourValue => arr_size / 2,
                    };
                    writeln!(f, "{INDENT}{INDENT}set_no_special({d}, {d_size});")?;
                    writeln!(
                        f,
                        "{INDENT}{INDENT}memset({d}+{dst_words}, 0, {dst_words}*sizeof(uint64_t));"
                    )?;
                }
            }
        }
        writeln!(f, "{INDENT}}} else {{")?;
    }

    struct Offset<'a>(CExpr<'a>);
    impl<'a> fmt::Display for Offset<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.0.ty().mode {
                M::TwoValue => self.0.fmt(f),
                M::FourValue => write!(f, "({}>>32)", self.0),
            }
        }
    }

    let o = Offset(o);

    let s_size = src.ty().size;
    let d_elem_ty = dst.ty.element_type();
    match (src.ty().mode, dst.ty.array_size(), src.ty().array_size()) {
        (M::TwoValue, None, None) => {
            write!(f, "{INDENT}{d} = ({d_elem_ty})(({o} >= {s_size}) ? 0 : (")?;
            if fill_with_x {
                // spc
                write!(
                    f,
                    "(({diff}>={o}) ? (({d_elem_ty})0x{mask:x}) : ((({d_elem_ty})0x{mask:x}) >> ({o}-{diff}))) |",
                    diff = src.ty().size.get() - dst.ty.size.get()
                )?;
            }
            // val
            write!(f, "((({d_elem_ty})({s} >> {o}) & 0x{mask:x})")?;
            if fill_with_x {
                write!(f, " << {d_size}")?;
            }
            writeln!(f, ")));")?;
        }
        (M::TwoValue, None, Some(src_arr_size)) => {
            let src_words_m_1 = src_arr_size - 1;
            write!(f, "{INDENT}{d} = ({d_elem_ty})(({o} >= {s_size}) ? 0 : (")?;
            if fill_with_x {
                // spc
                write!(
                    f,
                    "(({diff}>={o}) ? (({d_elem_ty})0x{mask:x}) : ((({d_elem_ty})0x{mask:x}) >> ({o}-{diff}))) |",
                    diff = src.ty().size.get() - dst.ty.size.get()
                )?;
            }
            // val
            write!(
                f,
                "((({d_elem_ty})((({s}[{o}/64] >> ({o}%64)) | ((({o}/64)<{src_words_m_1} && ({o}%64)>0) ? ({s}[{o}/64+1]<<(64-{o}%64)) : 0))) & 0x{mask:x})"
            )?;
            if fill_with_x {
                write!(f, " << {d_size}")?;
            }
            writeln!(f, ")));")?;
        }
        (M::TwoValue, Some(_), Some(_)) => writeln!(
            f,
            "{INDENT}tv_ll_slice({d}, {s}, {o}, {d_size}, {s_size}, {fill_with_x});"
        )?,
        (M::FourValue, None, None) => {
            let src_mask = super::mask(s_size.get());
            write!(f, "{INDENT}{d} = ({d_elem_ty})(({o} >= {s_size}) ? 0 : (")?;
            // spc
            if fill_with_x {
                write!(
                    f,
                    "(({d_elem_ty})(({s} & 0x{src_mask:x}) >> {o}) & 0x{mask:x}) |",
                )?;
            }
            // val
            write!(
                f,
                "((({d_elem_ty})(({s} >> {s_size}) >> {o}) & 0x{mask:x}) << {d_size})"
            )?;
            writeln!(f, "));")?;
        }
        (M::FourValue, None, Some(src_arr_size)) => {
            let src_words = src_arr_size / 2;
            let src_words_m_1 = src_words - 1;
            write!(f, "{INDENT}{d} = ({d_elem_ty})(({o} >= {s_size}) ? 0 : (")?;
            // spc
            write!(
                f,
                "(({d_elem_ty})((({s}[{o}/64] >> ({o}%64)) | ((({o}/64)<{src_words_m_1} && ({o}%64)>0) ? ({s}[{o}/64+1]<<(64-{o}%64)) : 0))) & 0x{mask:x}) |"
            )?;
            // val
            write!(
                f,
                "((({d_elem_ty})((({s}[{src_words}+{o}/64] >> ({o}%64)) | ((({o}/64)<{src_words_m_1} && ({o}%64)>0) ? ({s}[{src_words}+{o}/64+1]<<(64-{o}%64)) : 0))) & 0x{mask:x}) << {d_size})"
            )?;
            writeln!(f, "));")?;
        }
        (M::FourValue, Some(dst_arr_size), Some(src_arr_size)) => {
            let dst_words = dst_arr_size / 2;
            let src_words = src_arr_size / 2;
            writeln!(
                f,
                "{INDENT}tv_part_ll_slice({d}, {s}, {o}, {d_size}, {s_size}, {});",
                !fill_with_x
            )?;
            writeln!(
                f,
                "{INDENT}tv_part_ll_slice({d}+{dst_words}, {s}+{src_words}, {o}, {d_size}, {s_size}, false);"
            )?;
        }

        (M::TwoValue, Some(_), None) => {
            writeln!(
                f,
                "{INDENT}tv_ll_slice({d}, &{s}, {o}, {d_size}, {s_size}, {fill_with_x});"
            )?;
        }
        (_, Some(_), None) => unreachable!(),
    }

    if offset.ty().mode == LogicMode::FourValue {
        writeln!(f, "{INDENT}}}")?;
    }

    Ok(())
}
