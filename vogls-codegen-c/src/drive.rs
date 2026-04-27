use std::io;

use vogls_codegen::HeapRef;
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RtSignalKey;

use super::INDENT;
use crate::{CIdent, CType, CVar, write_cvar};

pub fn drive(
    f: &mut impl io::Write,
    signals: &[HeapRef],
    dst: RtSignalKey,
    src: CVar,
    partial: Option<(CVar, VectorSize)>,
) -> io::Result<()> {
    let s = src.ident;
    let dst_ref = signals[dst.as_usize()];
    let Some((offset, partial_size)) = partial else {
        let dst_wi = dst_ref.offset.bit_offset / 64;
        match src.ty.array_size() {
            None => {
                let s_elem_ty = src.ty.element_type();
                let dst_bi = dst_ref.offset.bit_offset % 64;
                let mask = super::mask(
                    src.ty.size.get()
                        * if src.ty.mode == LogicMode::FourValue {
                            2
                        } else {
                            1
                        },
                );
                writeln!(
                    f,
                    "{INDENT}if ({s} != ({s_elem_ty})((heap[{dst_wi}] >> {dst_bi}) & 0x{mask:x})) {{"
                )?;
            }
            Some(arr_size) => {
                writeln!(
                    f,
                    "{INDENT}if (memcmp({s}, heap+{dst_wi}, {arr_size}*sizeof(uint64_t)) != 0) {{"
                )?;
            }
        }
        writeln!(
            f,
            "{INDENT}{INDENT}drive_signal_{idx}(schedule, time, listening, last_active_time, cldctx);",
            idx = dst.as_u64()
        )?;
        write!(f, "{INDENT}")?;
        super::store(f, dst_ref.offset, src)?;
        writeln!(f, "{INDENT}}}")?;
        return Ok(());
    };

    writeln!(f, "{INDENT}{{")?;

    let dst_ty = CType {
        size: dst_ref.size,
        mode: src.ty.mode,
    };
    let current = CVar {
        ident: CIdent::Scoped(0),
        ty: CType {
            size: src.ty.size,
            mode: src.ty.mode,
        },
    };
    super::write_cvar(f, current)?;

    // @TODO: offset > size
    // @TODO: offset contains special

    // Load the current value
    let s1 = if dst_ty.is_array() {
        CVar {
            ident: CIdent::HeapWords((dst_ref.offset.bit_offset / 64) as u64),
            ty: dst_ty,
        }
    } else {
        let v = CVar {
            ident: CIdent::Scoped(1),
            ty: dst_ty,
        };
        super::write_cvar(f, v)?;
        super::load(f, dst_ref.offset, v)?;
        v
    };
    super::slice::slice_with(f, current, s1.into(), offset.into(), false)?;
    let c = current.ident;
    let o = offset.ident;
    match src.ty.array_size() {
        None => write!(f, "{INDENT}if ({s} != {c}")?,
        Some(arr_size) => write!(
            f,
            "{INDENT}if (memcmp({s}, {c}, {arr_size}*sizeof(uint64_t)) != 0"
        )?,
    }
    if offset.ty.mode == LogicMode::FourValue {
        write!(f, " && ({o}&0xFFFFFFFF) ==0xFFFFFFFF")?;
    }
    writeln!(f, ") {{")?;

    use LogicMode as M;
    let o_s = match offset.ty.mode {
        M::TwoValue => "",
        M::FourValue => ">>32",
    };

    let d_size = dst_ty.size;
    let s_size = src.ty.size;
    match (dst_ty.mode, dst_ty.array_size()) {
        (M::TwoValue, None) => {
            f.write_all(INDENT.as_bytes())?;
            let current = CVar {
                ident: c,
                ty: dst_ty,
            };
            write_cvar(f, current)?;
            f.write_all(INDENT.as_bytes())?;
            super::load(f, dst_ref.offset, current)?;
            writeln!(
                f,
                "{INDENT}{INDENT}{c} = tv_s_set({c}, {s}, {d_size}, ({o}{o_s}), {s_size});"
            )?;
            f.write_all(INDENT.as_bytes())?;
            super::store(f, dst_ref.offset, current)?;
        }
        (M::TwoValue, Some(_)) => {
            let d_word = dst_ref.offset.bit_offset / 64;
            if src.ty.is_array() {
                writeln!(
                    f,
                    "{INDENT}{INDENT}tv_l_set(heap+{d_word}, {s}, {d_size}, ({o}{o_s}), {s_size});",
                )?;
            } else {
                writeln!(
                    f,
                    "{INDENT}{INDENT}tv_l_set(heap+{d_word}, (uint64_t[1]){{{s}}}, {d_size}, ({o}{o_s}), {s_size});",
                )?;
            }
        }
        (M::FourValue, None) => {
            let d_mask = super::mask(d_size.get());
            let s_mask = super::mask(s_size.get());
            f.write_all(INDENT.as_bytes())?;
            let current = CVar {
                ident: c,
                ty: dst_ty,
            };
            write_cvar(f, current)?;
            f.write_all(INDENT.as_bytes())?;
            super::load(f, dst_ref.offset, current)?;
            writeln!(
                f,
                "{INDENT}{INDENT}{c} = tv_s_set({c} & 0x{d_mask:x}, {s} & 0x{s_mask:x}, {d_size}, ({o}{o_s}), {s_size}) | (tv_s_set({c} >> {d_size}, {s} >> {s_size}, {d_size}, ({o}{o_s}), {s_size}) << {d_size});"
            )?;
            f.write_all(INDENT.as_bytes())?;
            super::store(f, dst_ref.offset, current)?;
            f.write_all(INDENT.as_bytes())?;
        }
        (M::FourValue, Some(dst_arr_size)) => {
            let dst_words = dst_arr_size / 2;
            let d_word = dst_ref.offset.bit_offset / 64;
            match src.ty.array_size() {
                None => {
                    let s_mask = super::mask(s_size.get());
                    writeln!(
                        f,
                        "{INDENT}{INDENT}tv_l_set(heap+{d_word}, (uint64_t[1]){{{s}&0x{s_mask:x}}}, {d_size}, ({o}{o_s}), {s_size});",
                    )?;
                    writeln!(
                        f,
                        "{INDENT}{INDENT}tv_l_set(heap+{dst_words}+{d_word}, (uint64_t[1]){{{s}>>{s_size}}}, {d_size}, ({o}{o_s}), {s_size});",
                    )?;
                }
                Some(src_arr_size) => {
                    let src_words = src_arr_size / 2;
                    writeln!(
                        f,
                        "{INDENT}{INDENT}tv_l_set(heap+{d_word}, {s}, {d_size}, ({o}{o_s}), {s_size});",
                    )?;
                    writeln!(
                        f,
                        "{INDENT}{INDENT}tv_l_set(heap+{dst_words}+{d_word}, {s}+{src_words}, {d_size}, ({o}{o_s}), {s_size});",
                    )?;
                }
            }
        }
    }
    writeln!(
        f,
        "{INDENT}{INDENT}drive_signal_{idx}(schedule, time, listening, last_active_time, cldctx);",
        idx = dst.as_u64()
    )?;

    writeln!(f, "{INDENT}}}")?;
    writeln!(f, "{INDENT}}}")?;
    Ok(())
}
