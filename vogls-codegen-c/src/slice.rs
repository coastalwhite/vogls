use std::io;

use vogls_ir::{INTEGER_VSIZE, LogicMode};

use super::INDENT;
use crate::CVar;

pub fn slice(f: &mut impl io::Write, dst: CVar, src: CVar, offset: CVar) -> io::Result<()> {
    assert_eq!(offset.ty.size, INTEGER_VSIZE);
    assert_eq!(dst.ty.mode, LogicMode::FourValue);
    assert!(src.ty.size >= dst.ty.size);

    use LogicMode as M;
    let (d, s, o) = (dst.ident, src.ident, offset.ident);
    let d_size = dst.ty.size;
    let s_size = src.ty.size;
    let d_elem_ty = dst.ty.element_type();
    let mask = super::mask(dst.ty.size.get() % 64);
    match (src.ty.mode, dst.ty.array_size(), src.ty.array_size()) {
        (M::TwoValue, None, None) => {
            write!(f, "{INDENT}{d} = ({d_elem_ty})(({o} >= {s_size}) ? 0 : (")?;
            // spc
            write!(
                f,
                "(({diff}>={o}) ? (({d_elem_ty})0x{mask:x}) : ((({d_elem_ty})0x{mask:x}) >> ({o}-{diff}))) |",
                diff = src.ty.size.get() - dst.ty.size.get()
            )?;
            // val
            write!(f, "((({d_elem_ty})({s} >> {o}) & 0x{mask:x}) << {d_size})")?;
            writeln!(f, "));")?;
        }
        (M::TwoValue, None, Some(src_arr_size)) => {
            let src_words_m_1 = src_arr_size - 1;
            write!(f, "{INDENT}{d} = ({d_elem_ty})(({o} >= {s_size}) ? 0 : (")?;
            // spc
            write!(
                f,
                "(({diff}>={o}) ? (({d_elem_ty})0x{mask:x}) : ((({d_elem_ty})0x{mask:x}) >> ({o}-{diff}))) |",
                diff = src.ty.size.get() - dst.ty.size.get()
            )?;
            // val
            write!(
                f,
                "((({d_elem_ty})((({s}[{o}/64] >> ({o}%64)) | ((({o}/64)<{src_words_m_1} && ({o}%64)>0) ? ({s}[{o}/64+1]<<(64-{o}%64)) : 0))) & 0x{mask:x}) << {d_size})"
            )?;
            writeln!(f, "));")?;
        }
        (M::TwoValue, Some(_), Some(_)) => {
            writeln!(f, "{INDENT}tv_ll_slice({d}, {s}, {o}, {d_size}, {s_size});")?
        }
        (M::FourValue, None, None) => {
            let src_mask = super::mask(s_size.get());
            write!(
                f,
                "{INDENT}{d} = ({d_elem_ty})((({o} & 0xFFFFFFFF) != 0xFFFFFFFF || ({o}>>32) >= {s_size}) ? 0 : ("
            )?;
            // spc
            write!(
                f,
                "(({d_elem_ty})(({s} & 0x{src_mask:x}) >> ({o}>>32)) & 0x{mask:x}) |",
            )?;
            // val
            write!(
                f,
                "((({d_elem_ty})(({s} >> {s_size}) >> ({o}>>32)) & 0x{mask:x}) << {d_size})"
            )?;
            writeln!(f, "));")?;
        }
        (M::FourValue, None, Some(src_arr_size)) => {
            let src_words = src_arr_size / 2;
            let src_words_m_1 = src_words - 1;
            write!(
                f,
                "{INDENT}{d} = ({d_elem_ty})((({o} & 0xFFFFFFFF) != 0xFFFFFFFF || ({o}>>32) >= {s_size}) ? 0 : ("
            )?;
            // spc
            write!(
                f,
                "(({d_elem_ty})((({s}[({o}>>32)/64] >> (({o}>>32)%64)) | (((({o}>>32)/64)<{src_words_m_1} && (({o}>>32)%64)>0) ? ({s}[({o}>>32)/64+1]<<(64-({o}>>32)%64)) : 0))) & 0x{mask:x}) |"
            )?;
            // val
            write!(
                f,
                "((({d_elem_ty})((({s}[{src_words}+({o}>>32)/64] >> (({o}>>32)%64)) | (((({o}>>32)/64)<{src_words_m_1} && (({o}>>32)%64)>0) ? ({s}[{src_words}+({o}>>32)/64+1]<<(64-({o}>>32)%64)) : 0))) & 0x{mask:x}) << {d_size})"
            )?;
            writeln!(f, "));")?;
        }
        (M::FourValue, Some(dst_arr_size), Some(src_arr_size)) => {
            let dst_words = dst_arr_size / 2;
            let src_words = src_arr_size / 2;
            writeln!(f, "{INDENT}if (({o}&0xFFFFFFFF)==0xFFFFFFFF) {{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_part_ll_slice({d}, {s}, {o}>>32, {d_size}, {s_size});"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_part_ll_slice({d}+{dst_words}, {s}+{src_words}, {o}>>32, {d_size}, {s_size});"
            )?;
            writeln!(f, "{INDENT}}} else memset({d}, 0, {dst_arr_size});")?;
        }

        (M::TwoValue, Some(_), None) => {
            writeln!(
                f,
                "{INDENT}tv_ll_slice({d}, &{s}, {o}, {d_size}, {s_size});"
            )?;
        }
        (_, Some(_), None) => unreachable!(),
    }

    Ok(())
}
