use std::io;

use vogls_bits::arithmetic::FvLogicValue;
use vogls_ir::{INTEGER_VSIZE, LogicMode};

use crate::{CIdent, CVar, INDENT, mask};

pub fn cgc_bin_and(f: &mut impl io::Write, dst: CVar, lhs: CIdent, rhs: CIdent) -> io::Result<()> {
    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = {l} & {r};")?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {arr_size}; ++i) {d}[i] = {l}[i] & {r}[i];",
            )?;
        }

        // value = xspc xvalue yspc yvalue
        // spc = xspc ~xvalue + yspc ~yvalue + xspc yspc
        (LogicMode::FourValue, None) => {
            let size = dst.ty.size;
            let mask = mask(size.get());
            write!(f, "{INDENT}{d} = ")?;
            write!(
                f,
                "((({l} & (~{l}) >> {size}) | ({r} & (~{r}) >> {size}) | ({l} & {r})) & 0x{mask:x}) | "
            )?;
            write!(
                f,
                "((({l} & ({l} >> {size}) & {r} & ({r} >> {size})) & 0x{mask:x}) << {size})"
            )?;

            writeln!(f, ";")?
        }
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;

            writeln!(f, "{INDENT}for (int i = 0; i < {num_words}; ++i) {{")?;
            // spc
            writeln!(
                f,
                "{INDENT}{INDENT}{d}[i] = ({l}[i] & ~{l}[i+{num_words}]) | ({r}[i] & ~{r}[i+{num_words}]) | ({l}[i] & {r}[i]);"
            )?;
            // value
            writeln!(
                f,
                "{INDENT}{INDENT}{d}[{num_words}+i] = {l}[i] & {l}[i+{num_words}] & {r}[i] & {r}[i+{num_words}];"
            )?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_bin_or(f: &mut impl io::Write, dst: CVar, lhs: CIdent, rhs: CIdent) -> io::Result<()> {
    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = {l} | {r};")?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {arr_size}; ++i) {d}[i] = {l}[i] | {r}[i];",
            )?;
        }

        // value = xspc xvalue + yspc yvalue
        // spc = xspc xvalue + yspc yvalue + xspc yspc
        (LogicMode::FourValue, None) => {
            let size = dst.ty.size;
            let mask = mask(size.get());
            write!(f, "{INDENT}{d} = ")?;
            write!(
                f,
                "((({l} & ({l}) >> {size}) | ({r} & ({r}) >> {size}) | ({l} & {r})) & 0x{mask:x}) | "
            )?;
            write!(
                f,
                "(((({l} & ({l} >> {size})) | ({r} & ({r} >> {size}))) & 0x{mask:x}) << {size})"
            )?;

            writeln!(f, ";")?
        }
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;

            writeln!(f, "{INDENT}for (int i = 0; i < {num_words}; ++i) {{")?;
            // spc
            writeln!(
                f,
                "{INDENT}{INDENT}{d}[i] = ({l}[i] & {l}[i+{num_words}]) | ({r}[i] & {r}[i+{num_words}]) | ({l}[i] & {r}[i]);"
            )?;
            // value
            writeln!(
                f,
                "{INDENT}{INDENT}{d}[{num_words}+i] = ({l}[i] & {l}[i+{num_words}]) | ({r}[i] & {r}[i+{num_words}]);"
            )?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_bin_xor(f: &mut impl io::Write, dst: CVar, lhs: CIdent, rhs: CIdent) -> io::Result<()> {
    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = {l} ^ {r};")?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {arr_size}; ++i) {d}[i] = {l}[i] ^ {r}[i];",
            )?;
        }

        // value = xspc yspc (xvalue ^ yvalue)
        // spc = xspc yspc
        (LogicMode::FourValue, None) => {
            let size = dst.ty.size;
            let mask = mask(size.get());
            write!(f, "{INDENT}{d} = ")?;
            write!(f, "({l} & {r} & 0x{mask:x}) | ")?;
            write!(
                f,
                "((({l} & {r} & (({l} >> {size}) ^ ({r} >> {size}))) & 0x{mask:x}) << {size})"
            )?;

            writeln!(f, ";")?
        }
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;

            writeln!(f, "{INDENT}for (int i = 0; i < {num_words}; ++i) {{")?;
            // spc
            writeln!(f, "{INDENT}{INDENT}{d}[i] = {l}[i] & {r}[i];")?;
            // value
            writeln!(
                f,
                "{INDENT}{INDENT}{d}[{num_words}+i] = {l}[i] & {r}[i] & ({l}[i+{num_words}] ^ {r}[i+{num_words}]);"
            )?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

fn fv_inline_arith(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CIdent,
    rhs: CIdent,
    op: char,
) -> io::Result<()> {
    let size = dst.ty.size;
    let msbs_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << dst.ty.size.get()) - 1
    };

    let (d, l, r) = (dst.ident, lhs, rhs);
    writeln!(
        f,
        "{INDENT}{d} = (({l} & 0x{msbs_mask:x}) != 0x{msbs_mask:x} || ({r} & 0x{msbs_mask:x}) != 0x{msbs_mask:x}) ? 0 : ((({l} {op} {r}) & (uint64_t)0x{msbs_mask:x}) | ((uint64_t)0x{msbs_mask:x} << {size}));"
    )
}

fn fv_inline_div_rem(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CIdent,
    rhs: CIdent,
    op: char,
) -> io::Result<()> {
    let size = dst.ty.size;
    let msbs_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << dst.ty.size.get()) - 1
    };

    let (d, l, r) = (dst.ident, lhs, rhs);
    writeln!(
        f,
        "{INDENT}{d} = (({l} & 0x{msbs_mask:x}) != 0x{msbs_mask:x} || ({r} & 0x{msbs_mask:x}) != 0x{msbs_mask:x} || ({r} >> {size}) == 0) ? 0 : ((({l} {op} {r}) & (uint64_t)0x{msbs_mask:x}) | ((uint64_t)0x{msbs_mask:x} << {size}));"
    )
}

pub fn cgc_bin_add(f: &mut impl io::Write, dst: CVar, lhs: CIdent, rhs: CIdent) -> io::Result<()> {
    let size = dst.ty.size;
    let msbs_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << (dst.ty.size.get() % 64)) - 1
    };

    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = ({l} + {r}) & 0x{msbs_mask:x};")?,
        (LogicMode::TwoValue, Some(_)) => {
            writeln!(
                f,
                "{INDENT}tv_bigint_add_sub({d}, {l}, {r}, {size}, false);"
            )?;
        }
        (LogicMode::FourValue, None) => fv_inline_arith(f, dst, lhs, rhs, '+')?,
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(
                f,
                "{INDENT}if ((contains_special({l}, {size})) | (contains_special({r}, {size})))"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}memset({d}, 0, {arr_size}*sizeof(uint64_t));"
            )?;
            writeln!(f, "{INDENT}else {{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_bigint_add_sub({d}+{num_words}, {l}+{num_words}, {r}+{num_words}, {size}, false);"
            )?;
            writeln!(f, "{INDENT}{INDENT}set_no_special({d}, {size});")?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_bin_sub(f: &mut impl io::Write, dst: CVar, lhs: CIdent, rhs: CIdent) -> io::Result<()> {
    let size = dst.ty.size;
    let msbs_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << (dst.ty.size.get() % 64)) - 1
    };

    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = ({l} - {r}) & 0x{msbs_mask:x};")?,
        (LogicMode::TwoValue, Some(_)) => {
            writeln!(f, "{INDENT}tv_bigint_add_sub({d}, {l}, {r}, {size}, true);")?;
        }
        (LogicMode::FourValue, None) => fv_inline_arith(f, dst, lhs, rhs, '-')?,
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(
                f,
                "{INDENT}if ((contains_special({l}, {size})) | (contains_special({r}, {size})))"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}memset({d}, 0, {arr_size}*sizeof(uint64_t));"
            )?;
            writeln!(f, "{INDENT}else {{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_bigint_add_sub({d}+{num_words}, {l}+{num_words}, {r}+{num_words}, {size}, true);"
            )?;
            writeln!(f, "{INDENT}{INDENT}set_no_special({d}, {size});")?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_bin_pow(f: &mut impl io::Write, dst: CVar, lhs: CIdent, rhs: CIdent) -> io::Result<()> {
    todo!()
}

pub fn cgc_bin_mul(f: &mut impl io::Write, dst: CVar, lhs: CIdent, rhs: CIdent) -> io::Result<()> {
    let size = dst.ty.size;
    let msbs_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << (dst.ty.size.get() % 64)) - 1
    };

    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = ({l} * {r}) & 0x{msbs_mask:x};")?,
        (LogicMode::TwoValue, Some(_)) => {
            writeln!(f, "{INDENT}tv_bigint_mul({d}, {l}, {r}, {size});")?;
        }
        (LogicMode::FourValue, None) => fv_inline_arith(f, dst, lhs, rhs, '*')?,
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(
                f,
                "{INDENT}if ((contains_special({l}, {size})) | (contains_special({r}, {size})))"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}memset({d}, 0, {arr_size}*sizeof(uint64_t));"
            )?;
            writeln!(f, "{INDENT}else {{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_bigint_mul({d}+{num_words}, {l}+{num_words}, {r}+{num_words}, {size});"
            )?;
            writeln!(f, "{INDENT}{INDENT}set_no_special({d}, {size});")?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_bin_div(f: &mut impl io::Write, dst: CVar, lhs: CIdent, rhs: CIdent) -> io::Result<()> {
    let size = dst.ty.size;
    let msbs_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << dst.ty.size.get()) - 1
    };

    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(
            f,
            "{INDENT}{d} = ({r} == 0) ? 0 : (({l} / {r}) & 0x{msbs_mask:x});"
        )?,
        (LogicMode::TwoValue, Some(_)) => todo!(),
        (LogicMode::FourValue, None) => fv_inline_div_rem(f, dst, lhs, rhs, '/')?,
        (LogicMode::FourValue, Some(_)) => todo!(),
    }

    Ok(())
}

pub fn cgc_bin_mod(f: &mut impl io::Write, dst: CVar, lhs: CIdent, rhs: CIdent) -> io::Result<()> {
    let size = dst.ty.size;
    let msbs_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        (1u64 << dst.ty.size.get()) - 1
    };

    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = ({l} % {r}) & 0x{msbs_mask:x};")?,
        (LogicMode::TwoValue, Some(_)) => todo!(),
        (LogicMode::FourValue, None) => fv_inline_div_rem(f, dst, lhs, rhs, '%')?,
        (LogicMode::FourValue, Some(_)) => todo!(),
    }

    Ok(())
}

pub fn cgc_bin_ule(f: &mut impl io::Write, dst: CIdent, lhs: CVar, rhs: CIdent) -> io::Result<()> {
    let (d, l, r) = (dst, lhs.ident, rhs);
    match (lhs.ty.mode, lhs.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = (uint8_t)({l} <= {r});")?,
        (LogicMode::TwoValue, Some(_)) => todo!(),
        (LogicMode::FourValue, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_select_bit(f: &mut impl io::Write, dst: CIdent, lhs: CVar, rhs: CVar) -> io::Result<()> {
    assert_eq!(rhs.ty.size, INTEGER_VSIZE);

    let (d, l, r) = (dst, lhs.ident, rhs.ident);
    match (lhs.ty.mode, lhs.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(
            f,
            "{INDENT}{d} = ({r} >= {l_size}) ? 0 : (uint8_t)(({l} >> {r}) & 1);",
            l_size = lhs.ty.size
        )?,
        (LogicMode::TwoValue, Some(_)) => writeln!(
            f,
            "{INDENT}{d} = ({r} >= {l_size}) ? 0 : (uint8_t)(({l}[{r} / 64] >> ({r} % 64)) & 1);",
            l_size = lhs.ty.size
        )?,
        (LogicMode::FourValue, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_lsl(f: &mut impl io::Write, dst: CVar, lhs: CVar, rhs: CVar) -> io::Result<()> {
    assert_eq!(dst.ty, lhs.ty);
    assert_eq!(rhs.ty.size, INTEGER_VSIZE);

    let (d, l, r) = (dst.ident, lhs.ident, rhs.ident);
    use LogicMode as M;
    match (lhs.ty.mode, rhs.ty.mode, lhs.ty.array_size()) {
        (M::TwoValue, M::TwoValue, None) => writeln!(f, "{INDENT}{d} = {l} << {r};")?,
        (_, _, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_lsr(f: &mut impl io::Write, dst: CVar, lhs: CVar, rhs: CVar) -> io::Result<()> {
    assert_eq!(dst.ty, lhs.ty);
    assert_eq!(rhs.ty.size, INTEGER_VSIZE);

    let (d, l, r) = (dst.ident, lhs.ident, rhs.ident);
    use LogicMode as M;
    match (lhs.ty.mode, rhs.ty.mode, lhs.ty.array_size()) {
        (M::TwoValue, M::TwoValue, None) => writeln!(f, "{INDENT}{d} = {l} >> {r};")?,
        (M::TwoValue, M::TwoValue, Some(arr_size)) => {
            writeln!(
                f,
                r#"{INDENT}if ({r} % 64 == 0) {{
{INDENT}{INDENT}for (int i = 0; i < {arr_size} - ({r}/64+({r}%64 != 0)); i++) {d}[i] = {l}[i + ({r}/64+({r}%64 != 0))];
{INDENT}{INDENT}for (int i = {arr_size} - ({r}/64+({r}%64 != 0)); i < {arr_size}; i++) {d}[i] = 0;
{INDENT}}} else {{
{INDENT}{INDENT}for (int i = 0; i < {arr_size} - ({r}/64+({r}%64 != 0)); i++) {d}[i] = ({l}[i + ({r}/64+({r}%64 != 0)) - 1] >> ({r}%64)) | ({l}[i + ({r}/64+({r}%64 != 0))] << (64 - {r}%64));
{INDENT}{INDENT}{d}[{arr_size} - ({r}/64+({r}%64 != 0))] = ({l}[{arr_size}-1] >> ({r}%64)) | (0 << (64 - {r}%64));
{INDENT}{INDENT}for (int i = {arr_size} - ({r}/64+({r}%64 != 0)) + 1; i < {arr_size}; i++) {d}[i] = 0;
{INDENT}}}"#
            )?;
        }
        (_, _, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_asr(f: &mut impl io::Write, dst: CVar, lhs: CVar, rhs: CVar) -> io::Result<()> {
    todo!()
}

pub fn cgc_concat(f: &mut impl io::Write, dst: CVar, lhs: CVar, rhs: CVar) -> io::Result<()> {
    let (d, l, r) = (dst.ident, lhs.ident, rhs.ident);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(
            f,
            "{INDENT}{d} = ((({dst_elem_ty}){l}) << {r_size}) | (({dst_elem_ty}){r});",
            dst_elem_ty = dst.ty.element_type(),
            r_size = rhs.ty.size
        )?,
        (LogicMode::TwoValue, Some(_)) => {
            if rhs.ty.size.get() % 64 == 0 {
                match rhs.ty.array_size() {
                    None => writeln!(f, "{INDENT}{d}[0] = (uint64_t){r};")?,
                    Some(r_arr_size) => writeln!(
                        f,
                        "{INDENT}memmove({d}, {r}, sizeof(uint64_t)*{r_arr_size});"
                    )?,
                }
                let rwords = rhs.ty.array_size().unwrap_or(1);
                match lhs.ty.array_size() {
                    None => writeln!(f, "{INDENT}{d}[{rwords}] = (uint64_t){l};")?,
                    Some(l_arr_size) => writeln!(
                        f,
                        "{INDENT}memmove({d}+{rwords}, {l}, sizeof(uint64_t)*{l_arr_size});"
                    )?,
                }
            } else {
                match (lhs.ty.array_size(), rhs.ty.array_size()) {
                    (None, None) => {
                        writeln!(
                            f,
                            "{INDENT}{d}[0] = (((uint64_t){l}) << {r_size}) | (uint64_t){r};",
                            r_size = rhs.ty.size
                        )?;
                        writeln!(
                            f,
                            "{INDENT}{d}[1] = ((uint64_t){l}) >> {shift};",
                            shift = 64 - rhs.ty.size.get()
                        )?
                    }
                    (Some(l), None) => todo!(),
                    (None, Some(r)) => todo!(),
                    (Some(l), Some(r)) => todo!(),
                }
            }
        }
        (LogicMode::FourValue, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_copy_x(f: &mut impl io::Write, dst: CVar, lhs: CVar, rhs: CVar) -> io::Result<()> {
    let (d, l, _r) = (dst.ident, lhs.ident, rhs.ident);
    match (lhs.ty.mode, lhs.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = {l};")?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(f, "{INDENT}memmove({d}, {l}, {arr_size}*sizeof(uint64_t));")?
        }
        (LogicMode::FourValue, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_copy_y(f: &mut impl io::Write, dst: CVar, lhs: CVar, rhs: CVar) -> io::Result<()> {
    let (d, l, _r) = (dst.ident, lhs.ident, rhs.ident);
    match (lhs.ty.mode, lhs.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = {l};")?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(f, "{INDENT}memmove({d}, {l}, {arr_size}*sizeof(uint64_t));")?
        }
        (LogicMode::FourValue, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_bin_min(f: &mut impl io::Write, dst: CVar, lhs: CVar, rhs: CVar) -> io::Result<()> {
    let (d, l, r) = (dst.ident, lhs.ident, rhs.ident);
    match (lhs.ty.mode, lhs.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = ({l} < {r}) ? {l} : {r};")?,
        (LogicMode::TwoValue, Some(_)) => todo!(),
        (LogicMode::FourValue, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_bin_max(f: &mut impl io::Write, dst: CVar, lhs: CVar, rhs: CVar) -> io::Result<()> {
    assert_eq!(dst.ty, lhs.ty);
    assert_eq!(dst.ty, rhs.ty);

    let (d, l, r) = (dst.ident, lhs.ident, rhs.ident);
    match (lhs.ty.mode, lhs.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = ({l} > {r}) ? {l} : {r};")?,
        (LogicMode::TwoValue, Some(_)) => todo!(),
        (LogicMode::FourValue, _) => todo!(),
    }

    Ok(())
}

pub fn cgc_case_eq(f: &mut impl io::Write, dst: CIdent, lhs: CVar, rhs: CVar) -> io::Result<()> {
    let (d, l, r) = (dst, lhs.ident, rhs.ident);
    match lhs.ty.array_size() {
        None => writeln!(f, "{INDENT}{d} = (uint8_t)({l} == {r});")?,
        Some(arr_size) => writeln!(
            f,
            "{INDENT}{d} = 1; for (int i = 0; i < {arr_size}; ++i) {d} &= {l}[i] == {r}[i];"
        )?,
    }

    Ok(())
}

const FV_POSEDGE_LUT: u16 = {
    let mut fv_lut = 0u16;
    let mut i = 0;
    while i < 16 {
        let before = FvLogicValue::from_repr(i >> 2);
        let after = FvLogicValue::from_repr(i & 0x3);
        fv_lut |= (vogls_bits::edge::fv_posedge(before, after) as u16) << i;
        i += 1;
    }
    fv_lut
};
const FV_NEGEDGE_LUT: u16 = {
    let mut fv_lut = 0u16;
    let mut i = 0;
    while i < 16 {
        let before = FvLogicValue::from_repr(i >> 2);
        let after = FvLogicValue::from_repr(i & 0x3);
        fv_lut |= (vogls_bits::edge::fv_negedge(before, after) as u16) << i;
        i += 1;
    }
    fv_lut
};

pub fn cgc_posedge(f: &mut impl io::Write, dst: CIdent, lhs: CVar, rhs: CVar) -> io::Result<()> {
    let (d, l, r) = (dst, lhs.ident, rhs.ident);
    match lhs.ty.mode {
        LogicMode::TwoValue => writeln!(f, "{INDENT}{d} = {r} & ~{l};")?,
        LogicMode::FourValue => writeln!(
            f,
            "{INDENT}{d} = (0x{FV_POSEDGE_LUT:x} >> (({l} << 2) | {r})) & 1;"
        )?,
    }

    Ok(())
}

pub fn cgc_negedge(f: &mut impl io::Write, dst: CIdent, lhs: CVar, rhs: CVar) -> io::Result<()> {
    let (d, l, r) = (dst, lhs.ident, rhs.ident);
    match lhs.ty.mode {
        LogicMode::TwoValue => writeln!(f, "{INDENT}{d} = (uint8_t)({l} & ~{r});")?,
        LogicMode::FourValue => writeln!(
            f,
            "{INDENT}{d} = (0x{FV_NEGEDGE_LUT:x} >> (({l} << 2) | {r})) & 1;"
        )?,
    }

    Ok(())
}
