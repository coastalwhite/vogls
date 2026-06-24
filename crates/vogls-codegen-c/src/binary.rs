use std::{fmt, io};

use vogls_bits::arithmetic::FvLogicValue;
use vogls_ir::{INTEGER_VSIZE, LogicMode};

use crate::{CExpr, CIdent, CVar, INDENT, mask};

pub fn cgc_bin_and(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
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

pub fn cgc_bin_or(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
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

pub fn cgc_bin_xor<'a>(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'a>,
    rhs: CExpr<'a>,
) -> io::Result<()> {
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
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
    op: char,
) -> io::Result<()> {
    let size = dst.ty.size;
    let msbs_mask = mask(size.get());
    let (d, l, r) = (dst.ident, lhs, rhs);
    writeln!(
        f,
        "{INDENT}{d} = (({l} & 0x{msbs_mask:x}) != 0x{msbs_mask:x} || ({r} & 0x{msbs_mask:x}) != 0x{msbs_mask:x}) ? 0 : ((((({l} >> {size}) {op} ({r} >> {size})) & (uint64_t)0x{msbs_mask:x}) << {size}) | (uint64_t)0x{msbs_mask:x});"
    )
}

fn fv_inline_div_rem(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
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

pub fn cgc_bin_add(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
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

pub fn cgc_bin_sub(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
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

pub fn cgc_bin_pow(
    _f: &mut impl io::Write,
    _dst: CVar,
    _lhs: CExpr<'_>,
    _rhs: CExpr<'_>,
) -> io::Result<()> {
    todo!()
}

pub fn cgc_bin_mul(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
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

pub fn cgc_bin_div(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
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

pub fn cgc_bin_mod(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
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

fn tv_l_ule(
    f: &mut impl io::Write,
    d: impl fmt::Display,
    l: CExpr<'_>,
    r: CExpr<'_>,
    num_words: u32,
    arr_size: u32,
) -> io::Result<()> {
    writeln!(
        f,
        "{INDENT}{d} = 1; for (int i = 0; i < {num_words}; ++i) {{  if ({l}[{arr_size}-i-1] > {r}[{arr_size}-i-1]) {{ {d} = 0; break; }} else if ({l}[{arr_size}-i-1] < {r}[{arr_size}-i-1]) break; }}"
    )
}

pub fn cgc_bin_ule(
    f: &mut impl io::Write,
    dst: CIdent,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    let size = lhs.ty().size;
    let (d, l, r) = (dst, lhs, rhs);

    match (lhs.ty().mode, lhs.ty().array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = (uint8_t)({l} <= {r});")?,
        (LogicMode::TwoValue, Some(arr_size)) => tv_l_ule(f, d, l, r, arr_size, arr_size)?,
        (LogicMode::FourValue, None) => {
            let mask = mask(size.get());
            writeln!(
                f,
                "{INDENT}{d} = (({l} & 0x{mask:x}) == 0x{mask:x} && ({r} & 0x{mask:x}) == 0x{mask:x}) ? (((uint8_t)(({l} >> {size}) <= ({r} >> {size})) << 1) | 1) : 0;"
            )?;
        }
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(
                f,
                "{INDENT}if (contains_special({l}, {size}) || contains_special({r}, {size})) {d} = 0;"
            )?;
            writeln!(f, "{INDENT}else {{")?;
            write!(f, "{INDENT}")?;
            tv_l_ule(f, d, l, r, num_words, arr_size)?;
            writeln!(f, "{INDENT}{INDENT}{d} = ({d} << 1) | 1;")?;
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_lsl(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    assert_eq!(dst.ty, lhs.ty());
    assert_eq!(rhs.ty().size, INTEGER_VSIZE);

    let l_size = lhs.ty().size;
    let (d, l, r) = (dst.ident, lhs, rhs);
    use LogicMode as M;
    match (lhs.ty().mode, lhs.ty().array_size()) {
        (M::TwoValue, None) => writeln!(
            f,
            "{INDENT}{d} = ({r} >= {l_size}) ? 0 : (({l} << {r}) & 0x{:x});",
            mask(lhs.ty().size.get())
        )?,
        (M::TwoValue, Some(_)) => {
            writeln!(f, "{INDENT}tv_l_lsl_with({d}, {l}, {r}, {l_size}, 0);")?
        }
        (M::FourValue, None) => {
            writeln!(f, "{INDENT}if (({r} & 0xFFFFFFFF) == 0xFFFFFFFF) {{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}{d} = (({r} >> 32) >= {l_size}) ? 0x{mask:x}ULL : (({l} << ({r} >> 32)) & 0x{double_mask:x}ULL & ~(((1ULL << ({r} >> 32)) - 1) << {l_size})) | ((1ULL << ({r} >> 32)) - 1);",
                mask = mask(lhs.ty().size.get()),
                double_mask = mask(lhs.ty().size.get() * 2),
            )?;
            writeln!(f, "{INDENT}}} else {d} = 0;")?;
        }
        (M::FourValue, Some(arr_size)) => {
            let words = arr_size / 2;
            writeln!(f, "{INDENT}if (({r} & 0xFFFFFFFF) == 0xFFFFFFFF) {{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_l_lsl_with({d}, {l}, {r} >> 32, {l_size}, true);"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_l_lsl_with({d}+{words}, {l}+{words}, {r} >> 32, {l_size}, false);"
            )?;
            writeln!(
                f,
                "{INDENT}}} else memset({d}, 0, {arr_size}*sizeof(uint64_t));"
            )?;
        }
    }

    Ok(())
}

pub fn cgc_lsr(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    cgc_lsr_with(f, dst, lhs, rhs, true)
}

pub fn cgc_lsr_with(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
    valid: bool,
) -> io::Result<()> {
    assert_eq!(dst.ty, lhs.ty());
    assert_eq!(rhs.ty().size, INTEGER_VSIZE);

    let l_size = lhs.ty().size;
    let (d, l, r) = (dst.ident, lhs, rhs);
    use LogicMode as M;
    match (lhs.ty().mode, lhs.ty().array_size()) {
        (M::TwoValue, None) => writeln!(
            f,
            "{INDENT}{d} = ({r} >= {l_size}) ? 0 : (({l} >> {r}) & 0x{:x});",
            mask(lhs.ty().size.get())
        )?,
        (M::TwoValue, Some(_)) => {
            writeln!(f, "{INDENT}tv_l_lsr_with({d}, {l}, {r}, {l_size}, 0);")?
        }
        (M::FourValue, None) => {
            writeln!(f, "{INDENT}if (({r} & 0xFFFFFFFF) == 0xFFFFFFFF) {{")?;
            write!(
                f,
                "{INDENT}{INDENT}{d} = (({r} >> 32) >= {l_size}) ? 0x{mask:x}ULL : (({l} >> ({r} >> 32))",
                mask = mask(lhs.ty().size.get()),
            )?;
            if valid {
                write!(
                    f,
                    " | (((1ULL << ({r} >> 32)) - 1) << ({l_size} - ({r} >> 32)))"
                )?;
            }
            write!(f, ");")?;
            writeln!(f, "{INDENT}}} else {d} = 0;")?;
        }
        (M::FourValue, Some(arr_size)) => {
            let words = arr_size / 2;
            writeln!(f, "{INDENT}if (({r} & 0xFFFFFFFF) == 0xFFFFFFFF) {{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_l_lsr_with({d}, {l}, {r} >> 32, {l_size}, {valid});"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_l_lsr_with({d}+{words}, {l}+{words}, {r} >> 32, {l_size}, false);"
            )?;
            writeln!(
                f,
                "{INDENT}}} else memset({d}, 0, {arr_size}*sizeof(uint64_t));"
            )?;
        }
    }

    Ok(())
}

pub fn cgc_asr(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    let size = dst.ty.size;
    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => {
            let unsigned_elem_ty = dst.ty.element_type();
            let signed_elem_ty = dst.ty.element_type().signed_ty_str();
            let elem_size = dst.ty.element_type().size();
            let shift = elem_size.get() - size.get();
            let mask = mask(size.get());
            writeln!(
                f,
                "{INDENT}{d} = ((({unsigned_elem_ty})(((({signed_elem_ty})(({unsigned_elem_ty}){l} << {shift})) >> {shift}) >> (({r} >= {elem_size}) ? ({elem_size}-1) : {r})))) & 0x{mask:x};"
            )?;
        }
        (LogicMode::TwoValue, Some(_)) => {
            let wi = (size.get() - 1) / 64;
            let bi = (size.get() - 1) % 64;
            writeln!(
                f,
                "{INDENT}tv_l_lsr_with({d}, {l}, {r}, {size}, ({l}[{wi}] >> {bi}) & 1);"
            )?
        }
        (LogicMode::FourValue, None) => {
            let unsigned_elem_ty = dst.ty.element_type();
            let signed_elem_ty = dst.ty.element_type().signed_ty_str();
            let elem_size = dst.ty.element_type().size();
            let shift = elem_size.get() - size.get();
            let rhs_mask = mask(rhs.ty().size.get());
            let mask = mask(size.get());
            writeln!(
                f,
                "{INDENT}if (({r} & 0x{rhs_mask:x}) != 0x{rhs_mask:x}) {d} = 0;"
            )?;
            writeln!(f, "{INDENT}else {d} =")?;
            writeln!(
                f,
                "{INDENT}{INDENT}(((({unsigned_elem_ty})(((({signed_elem_ty})(({unsigned_elem_ty})({l} & 0x{mask:x}) << {shift})) >> {shift}) >> ((({r} >> 32) >= {elem_size}) ? ({elem_size}-1) : ({r} >> 32))))) & 0x{mask:x}) |"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}((((({unsigned_elem_ty})(((({signed_elem_ty})(({unsigned_elem_ty})({l} >> {size}) << {shift})) >> {shift}) >> ((({r} >> 32) >= {elem_size}) ? ({elem_size}-1) : ({r} >> 32))))) & 0x{mask:x}) << {size});"
            )?;
        }
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            let wi = (size.get() - 1) / 64;
            let bi = (size.get() - 1) % 64;
            writeln!(f, "{INDENT}if (({r} & 0xFFFFFFFF) == 0xFFFFFFFF) {{")?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_l_lsr_with({d}, {l}, {r} >> 32, {size}, ({l}[{wi}] >> {bi}) & 1);"
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}tv_l_lsr_with({d}+{num_words}, {l}+{num_words}, {r} >> 32, {size}, ({l}[{num_words}+{wi}] >> {bi}) & 1);"
            )?;
            writeln!(
                f,
                "{INDENT}}} else memset({d}, 0, {arr_size}*sizeof(uint64_t));"
            )?;
        }
    }
    Ok(())
}

pub fn cgc_concat(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    let (d, l, r) = (dst.ident, lhs, rhs);
    match (dst.ty.mode, dst.ty.array_size()) {
        (LogicMode::TwoValue, None) => writeln!(
            f,
            "{INDENT}{d} = ((({dst_elem_ty}){l}) << {r_size}) | (({dst_elem_ty}){r});",
            dst_elem_ty = dst.ty.element_type(),
            r_size = rhs.ty().size
        )?,
        (LogicMode::TwoValue, Some(_)) => {
            let l_size = lhs.ty().size;
            let r_size = rhs.ty().size;

            write!(f, "{INDENT}tv_l_concat({d}, ")?;
            match lhs.ty().array_size() {
                None => write!(f, "(uint64_t[1]){{{l} & 0x{:x}}}", mask(l_size.get()))?,
                Some(_) => write!(f, "{l}")?,
            }
            f.write_all(b", ")?;
            match rhs.ty().array_size() {
                None => write!(f, "(uint64_t[1]){{{r} & 0x{:x}}}", mask(r_size.get()))?,
                Some(_) => write!(f, "{r}")?,
            }
            writeln!(f, ", {l_size}, {r_size});")?;
        }
        (LogicMode::FourValue, None) => {
            let dst_elem_ty = dst.ty.element_type();
            let (l_size, r_size) = (lhs.ty().size, rhs.ty().size);

            write!(f, "{INDENT}{d} = ")?;
            write!(
                f,
                "(((({dst_elem_ty}){l}) & 0x{l_mask:x}) << {r_size}) | ((({dst_elem_ty}){r}) & 0x{r_mask:x}) |",
                l_mask = mask(lhs.ty().size.get()),
                r_mask = mask(rhs.ty().size.get()),
            )?;
            writeln!(
                f,
                "(((({dst_elem_ty}){l}) >> {l_size}) << {l_shift}) | (((({dst_elem_ty}){r}) >> {r_size}) << {r_shift});",
                l_shift = l_size.get() + r_size.get() * 2,
                r_shift = l_size.get() + r_size.get(),
            )?;
        }
        (LogicMode::FourValue, Some(dst_arr_size)) => {
            let dst_num_words = dst_arr_size / 2;
            let l_size = lhs.ty().size;
            let r_size = rhs.ty().size;

            write!(f, "{INDENT}tv_l_concat({d}, ")?;
            match lhs.ty().array_size() {
                None => write!(
                    f,
                    "(uint64_t[1]){{(uint64_t){l} & 0x{:x}}}",
                    mask(l_size.get())
                )?,
                Some(_) => write!(f, "{l}")?,
            }
            f.write_all(b", ")?;
            match rhs.ty().array_size() {
                None => write!(
                    f,
                    "(uint64_t[1]){{(uint64_t){r} & 0x{:x}}}",
                    mask(r_size.get())
                )?,
                Some(_) => write!(f, "{r}")?,
            }
            writeln!(f, ", {l_size}, {r_size});")?;

            write!(f, "{INDENT}tv_l_concat({d}+{dst_num_words}, ")?;
            match lhs.ty().array_size() {
                None => write!(f, "(uint64_t[1]){{(uint64_t){l} >> {l_size}}}")?,
                Some(arr_size) => write!(f, "{l}+{}", arr_size / 2)?,
            }
            f.write_all(b", ")?;
            match rhs.ty().array_size() {
                None => write!(f, "(uint64_t[1]){{(uint64_t){r} >> {r_size}}}")?,
                Some(arr_size) => write!(f, "{r}+{}", arr_size / 2)?,
            }
            writeln!(f, ", {l_size}, {r_size});")?;
        }
    }

    Ok(())
}

pub fn cgc_copy_x(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    let size = dst.ty.size;
    let (d, l, r) = (dst.ident, lhs, rhs);
    match (lhs.ty().mode, lhs.ty().array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = {l};")?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(f, "{INDENT}memcpy({d}, {l}, {arr_size}*sizeof(uint64_t));")?
        }
        (LogicMode::FourValue, None) => writeln!(
            f,
            "{INDENT}{d} = {l} & ({r} | ({r} >> {size}) | ({r} | ({r} >> {size})) << {size});"
        )?,
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {num_words}; ++i) {{ {d}[i] = {l}[i] & ({r}[i] | {r}[i+{num_words}]); {d}[i+{num_words}] = {l}[i+{num_words}] & ({r}[i] | {r}[i+{num_words}]); }}"
            )?
        }
    }

    Ok(())
}

pub fn cgc_copy_y(f: &mut impl io::Write, dst: CVar, lhs: CExpr, rhs: CExpr) -> io::Result<()> {
    let size = dst.ty.size;
    let (d, l, r) = (dst.ident, lhs, rhs);
    match (lhs.ty().mode, lhs.ty().array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = {l};")?,
        (LogicMode::TwoValue, Some(arr_size)) => {
            writeln!(f, "{INDENT}memcpy({d}, {l}, {arr_size}*sizeof(uint64_t));")?
        }
        (LogicMode::FourValue, None) => writeln!(
            f,
            "{INDENT}{d} = ({l} & ({r} | ~({r} >> {size}))) | ((({l} >> {size}) | (~{r} & ({r} >> {size}))) << {size});"
        )?,
        (LogicMode::FourValue, Some(arr_size)) => {
            let num_words = arr_size / 2;
            writeln!(
                f,
                "{INDENT}for (int i = 0; i < {num_words}; ++i) {{ {d}[i] = {l}[i] & ({r}[i] | ~{r}[i+{num_words}]); {d}[i+{num_words}] = {l}[i+{num_words}] | (~{r}[i] & {r}[i+{num_words}]); }}"
            )?
        }
    }

    Ok(())
}

pub fn cgc_bin_min(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    let size = lhs.ty().size;
    let (d, l, r) = (dst.ident, lhs, rhs);
    match (lhs.ty().mode, lhs.ty().array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = ({l} < {r}) ? {l} : {r};")?,
        (LogicMode::FourValue, None) => {
            let mask = mask(size.get());
            writeln!(
                f,
                "{INDENT}{d} = (({l} & 0x{mask:x}) == 0x{mask:x} && ({r} & 0x{mask:x}) == 0x{mask:x}) ? ((({l} >> {size}) < ({r} >> {size})) ? {l} : {r}) : 0;"
            )?;
        }
        (mode, Some(arr_size)) => {
            let num_words = match mode {
                LogicMode::TwoValue => arr_size,
                LogicMode::FourValue => arr_size / 2,
            };
            writeln!(f, "{INDENT}{{")?;
            if mode == LogicMode::FourValue {
                writeln!(
                    f,
                    "{INDENT}if (contains_special({l}, {size}) || contains_special({r}, {size})) memset({d}, 0, {arr_size}*sizeof(uint64_t));"
                )?;
                writeln!(f, "{INDENT}else {{")?;
            }
            writeln!(f, "{INDENT}uint8_t is_ule;")?;
            tv_l_ule(f, "is_ule", l, r, num_words, arr_size)?;
            writeln!(
                f,
                "{INDENT}if (is_ule) memcpy({d}, {l}, {arr_size}*sizeof(uint64_t));"
            )?;
            writeln!(
                f,
                "{INDENT}else memcpy({d}, {r}, {arr_size}*sizeof(uint64_t));"
            )?;
            if mode == LogicMode::FourValue {
                writeln!(f, "{INDENT}}}")?;
            }
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_bin_max(
    f: &mut impl io::Write,
    dst: CVar,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    assert_eq!(dst.ty, lhs.ty());
    assert_eq!(dst.ty, rhs.ty());

    let size = lhs.ty().size;
    let (d, l, r) = (dst.ident, lhs, rhs);
    match (lhs.ty().mode, lhs.ty().array_size()) {
        (LogicMode::TwoValue, None) => writeln!(f, "{INDENT}{d} = ({l} > {r}) ? {l} : {r};")?,
        (LogicMode::FourValue, None) => {
            let mask = mask(size.get());
            writeln!(
                f,
                "{INDENT}{d} = (({l} & 0x{mask:x}) == 0x{mask:x} && ({r} & 0x{mask:x}) == 0x{mask:x}) ? ((({l} >> {size}) > ({r} >> {size})) ? {l} : {r}) : 0;"
            )?;
        }
        (mode, Some(arr_size)) => {
            let num_words = match mode {
                LogicMode::TwoValue => arr_size,
                LogicMode::FourValue => arr_size / 2,
            };
            writeln!(f, "{INDENT}{{")?;
            if mode == LogicMode::FourValue {
                writeln!(
                    f,
                    "{INDENT}if (contains_special({l}, {size}) || contains_special({r}, {size})) memset({d}, 0, {arr_size}*sizeof(uint64_t));"
                )?;
                writeln!(f, "{INDENT}else {{")?;
            }
            writeln!(f, "{INDENT}uint8_t is_ule;")?;
            tv_l_ule(f, "is_ule", l, r, num_words, arr_size)?;
            writeln!(
                f,
                "{INDENT}if (is_ule) memcpy({d}, {r}, {arr_size}*sizeof(uint64_t));"
            )?;
            writeln!(
                f,
                "{INDENT}else memcpy({d}, {l}, {arr_size}*sizeof(uint64_t));"
            )?;
            if mode == LogicMode::FourValue {
                writeln!(f, "{INDENT}}}")?;
            }
            writeln!(f, "{INDENT}}}")?;
        }
    }

    Ok(())
}

pub fn cgc_case_eq(
    f: &mut impl io::Write,
    dst: CIdent,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    let (d, l, r) = (dst, lhs, rhs);
    match lhs.ty().array_size() {
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

pub fn cgc_posedge(
    f: &mut impl io::Write,
    dst: CIdent,
    lhs: CExpr<'_>,
    rhs: CExpr<'_>,
) -> io::Result<()> {
    let (d, l, r) = (dst, lhs, rhs);
    match lhs.ty().mode {
        LogicMode::TwoValue => writeln!(f, "{INDENT}{d} = {r} & ~{l};")?,
        LogicMode::FourValue => writeln!(
            f,
            "{INDENT}{d} = (0x{FV_POSEDGE_LUT:x} >> (({l} << 2) | {r})) & 1;"
        )?,
    }

    Ok(())
}

pub fn cgc_negedge(f: &mut impl io::Write, dst: CIdent, lhs: CExpr, rhs: CExpr) -> io::Result<()> {
    let (d, l, r) = (dst, lhs, rhs);
    match lhs.ty().mode {
        LogicMode::TwoValue => writeln!(f, "{INDENT}{d} = (uint8_t)({l} & ~{r});")?,
        LogicMode::FourValue => writeln!(
            f,
            "{INDENT}{d} = (0x{FV_NEGEDGE_LUT:x} >> (({l} << 2) | {r})) & 1;"
        )?,
    }

    Ok(())
}
