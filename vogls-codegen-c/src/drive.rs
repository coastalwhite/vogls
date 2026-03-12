use std::io;

use vogls_codegen::HeapRef;
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RtSignalKey;

use super::INDENT;
use crate::{CType, CVar};

pub fn drive(
    f: &mut impl io::Write,
    signals: &[HeapRef],
    dst: RtSignalKey,
    src: CVar,
    partial: Option<(CVar, VectorSize)>,
) -> io::Result<()> {
    let s = src.ident;
    let Some((offset, partial_size)) = partial else {
        let dst_ref = signals[dst.as_usize()];
        let dst_wi = dst.as_u64() / 64;
        match src.ty.array_size() {
            None => {
                let s_elem_ty = src.ty.element_type();
                let dst_bi = dst.as_u64() % 64;
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
                    "{INDENT}if (memcmp({s}, &(heap+{dst_wi}), {arr_size}*sizeof(uint64_t)) == 0) {{"
                )?;
            }
        }
        writeln!(
            f,
            "{INDENT}{INDENT}drive_signal_{idx}(schedule, time, is_scheduled, listening, last_active_time);",
            idx = dst.as_u64()
        )?;
        write!(f, "{INDENT}")?;
        super::store(&mut f, signals[dst.as_usize()].offset, src)?;
        writeln!(f, "{INDENT}}}")?;
        return Ok(());
    };

    writeln!(f, "{INDENT}{{")?;

    let current_ty = CType {
        size: partial_size,
        mode: src.ty.mode,
    };
    super::write_var(f, "current", current_ty)?;

    // @TODO: offset > size
    // @TODO: offset contains special

    // Load the current value
    {
        if current_ty.mode == LogicMode::FourValue {
            todo!()
        }
        let src_ty = CType {
            size: src.ty.size,
            mode: dst.ty.mode,
        };
        if let Some(_) = src_ty.array_size() {
            match dst.ty.array_size() {
                None => {
                    write!(
                        b,
                        r#"
{INDENT}if (({offset}%64)+{dst_size} <= 64) {dst} = (heap[{word}+({offset}/64)] >> ({offset}%64)) & 0x{mask:x};
{INDENT}else {dst} = (heap[{word}+({offset}/64)] >> ({offset}%64)) | (heap[{word}+({offset}/64) + 1] >> (64 - {offset}%64)) & 0x{mask:x};
"#,
                        offset = offset.ident,
                        dst = dst.ident,
                        dst_size = dst.ty.size,
                        word = src.offset.bit_offset / 64,
                        mask = mask(dst.ty.size.get())
                    )?;
                }
                _ => todo!(),
            }

            return Ok(());
        }

        let mut num_bits = dst.ty.size.get();
        if dst.ty.mode == LogicMode::FourValue {
            num_bits *= 2;
        }

        let word = src.offset.bit_offset / 64;
        let shift = src.offset.bit_offset % 64;
        let mask = mask(num_bits);

        writeln!(
            b,
            "{INDENT}{t} = (heap[{word}] >> ({shift} + {offset})) & 0x{mask:x};",
            t = dst.ident,
            offset = offset.ident,
        )
    }

    let dst_wi = dst.as_u64() / 64;
    match current_ty.array_size() {
        None => {
            let s_elem_ty = src.ty.element_type();
            let dst_bi = dst.as_u64() % 64;
            let mask = super::mask(src.ty.size.get());
            writeln!(f, "{INDENT}if ({s} != current) {{")?;
        }
        Some(arr_size) => {
            writeln!(
                f,
                "{INDENT}if (memcmp({s}, current, {arr_size}*sizeof(uint64_t)) == 0) {{"
            )?;
        }
    }
    match current_t.ty.array_size() {
        None => {
            writeln!(
                f,
                "{INDENT}if ({t} != {current_t}) {{",
                t = t.ident,
                current_t = current_t.ident
            )?;
        }
        Some(_) => {
            let condition = CVar {
                ident: CIdent(temp_counter),
                ty: CType {
                    size: SCALAR_VSIZE,
                    mode: LogicMode::TwoValue,
                },
            };
            temp_counter += 1;
            write_cvar(&mut buffer, condition)?;
            binary::cgc_case_eq(&mut buffer, condition.ident, t, current_t)?;
            writeln!(buffer, "{INDENT}if (!{}) {{", condition.ident)?;
        }
    }
    writeln!(
        f,
        "{INDENT}{INDENT}drive_signal_{idx}(schedule, time, is_scheduled, listening, last_active_time);",
        idx = dst.as_u64()
    )?;
    store_slice(f, signals[dst.as_usize()], offset, src)?;
    writeln!(f, "{INDENT}}}")?;

    writeln!(f, "{INDENT}}}")?;
    Ok(())
}

fn load_slice(b: &mut impl io::Write, dst: CVar, offset: CVar, src: HeapRef) -> io::Result<()> {}

fn store_slice(f: &mut impl io::Write, dst: HeapRef, offset: CVar, src: CVar) -> io::Result<()> {
    if src.ty.mode == LogicMode::FourValue {
        todo!()
    }
    let dst_ty = CType {
        size: dst.size,
        mode: src.ty.mode,
    };
    if let Some(_) = dst_ty.array_size() {
        match src.ty.array_size() {
            None => {
                write!(
                    f,
                    r#"
{INDENT}if (({offset}%64)+{src_size} <= 64) heap[{word}+({offset}/64)] = (heap[{word}+({offset}/64)] & ~(((uint64_t)0x{mask:x}) << ({offset}%64))) | (((uint64_t){src}) << ({offset}%64));
{INDENT}else {{ printf("NYI [STORE SLICE]\n"); cldctx->exit = 2; return; }};
"#,
                    src_size = src.ty.size,
                    offset = offset.ident,
                    src = src.ident,
                    word = dst.offset.bit_offset / 64,
                    mask = mask(src.ty.size.get()),
                )?;
            }
            _ => todo!(),
        }
        return Ok(());
    }

    let mut num_bits = src.ty.size.get();
    if src.ty.mode == LogicMode::FourValue {
        num_bits *= 2;
    }

    let word = dst.offset.bit_offset / 64;
    let shift = dst.offset.bit_offset % 64;
    let mask = mask(num_bits);

    writeln!(
        f,
        "{INDENT}heap[{word}] = (heap[{word}] & ~(((uint64_t)0x{mask:x}) << ({shift}+{offset}))) | ((uint64_t){t} << ({shift} + {offset}));",
        t = src.ident,
        offset = offset.ident,
    )
}
