use std::{fmt, io};

use vogls_ir::{LogicMode, SCALAR_VSIZE};

use crate::{CExpr, CVar, INDENT};

pub fn cgc_select(
    f: &mut impl io::Write,
    dst: CVar,
    cond: CExpr<'_>,
    truthy: CExpr<'_>,
    falsy: CExpr<'_>,
) -> io::Result<()> {
    assert_eq!(cond.ty().size, SCALAR_VSIZE);
    assert_eq!(dst.ty.size, truthy.ty().size);
    assert_eq!(dst.ty.size, falsy.ty().size);

    struct Cond<'a>(CExpr<'a>);
    impl<'a> fmt::Display for Cond<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.0.ty().mode {
                LogicMode::TwoValue => self.0.fmt(f),
                LogicMode::FourValue => write!(f, "({}==0b11)", self.0),
            }
        }
    }
    let cond = Cond(cond);

    let (d, truthy, falsy) = (dst.ident, truthy, falsy);
    match dst.ty.array_size() {
        None => writeln!(f, "{INDENT}{d} = {cond} ? {truthy} : {falsy};"),
        Some(nwords) => writeln!(
            f,
            "{INDENT}memcpy({d}, {cond} ? {truthy} : {falsy}, {nwords}*sizeof(uint64_t));"
        ),
    }
}
