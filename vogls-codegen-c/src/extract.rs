use std::io;

use vogls_ir::{INTEGER_VSIZE, LogicMode};

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
    let (d, s, o) = (dst.ident, src.ident, offset.ident);
    let s_size = src.ty.size;
    let d_elem_ty = dst.ty.element_type();
    let mask = super::mask(dst.ty.size.get() % 64);
    match (dst.ty.mode, dst.ty.array_size(), src.ty.array_size()) {
        (M::TwoValue, None, None) => writeln!(
            f,
            "{d} = ({d_elem_ty})(({o} >= {s_size}) ? 0 : (({s} >> {o}) & 0x{mask:x}));"
        )?,
        (M::TwoValue, None, Some(arr_size)) => {
            let swords_m_1 = arr_size - 1;
            writeln!(
                f,
                "{d} = ({d_elem_ty})(({o} >= {s_size}) ? 0 : (({s}[{o}/64] >> ({o}%64)) | ({s}[min({swords_m_1},{o}/64+1)] << ((64-{o})%64))));"
            )?;
        }
        (M::TwoValue, Some(_), Some(_)) => todo!(),
        (M::FourValue, None, None) => todo!(),
        (M::FourValue, None, Some(_)) => todo!(),
        (M::FourValue, Some(_), Some(_)) => todo!(),

        (_, Some(_), None) => unreachable!(),
    }

    Ok(())
}
