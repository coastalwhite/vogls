use std::io;

use vogls_ir::{LogicMode, INTEGER_VSIZE};

use crate::CVar;

pub fn extract(f: &mut impl io::Write, dst: CVar, src: CVar, offset: CVar) -> io::Result<()> {
    assert_eq!(offset.ty.size, INTEGER_VSIZE);
    assert_eq!(src.ty.mode, dst.ty.mode);
    assert!(src.ty.size <= dst.ty.size);

    if dst.ty.size == src.ty.size {
        super::binary::cgc_lsr_with(f, dst, src, offset, false)?;
        return Ok(());
    }

    use LogicMode as M;
    match (dst.ty.mode, dst.ty.array_size(), src.ty.array_size()) {
        (M::TwoValue, None, None) => writeln!(f, ""),
        (M::TwoValue, None, Some(_)) => todo!(),
        (M::TwoValue, Some(_), Some(_)) => todo!(),
        (M::FourValue, None, None) => todo!(),
        (M::FourValue, None, Some(_)) => todo!(),
        (M::FourValue, Some(_), Some(_)) => todo!(),

        (_, Some(_), None) => unreachable!(),
    }

    Ok(())
}
