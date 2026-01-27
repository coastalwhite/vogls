use vogls_bits::BitsDataRef;
use vogls_ir::Bits;

pub(super) mod fv;
pub(super) mod tv;

pub(crate) fn exec_constant(stack: &mut [u8], dst: usize, value: &Bits) {
    let size = value.size();
    match value.as_data_ref() {
        BitsDataRef::InlineTv(v) => {
            let nbytes = size.get().div_ceil(8) as usize;
            stack[dst..][..nbytes].copy_from_slice(&v.to_le_bytes()[..nbytes])
        }
        BitsDataRef::InlineFv(spc, val) => {
            let nbytes = (2 * size.get()).div_ceil(8) as usize;
            let v = ((spc as u64) << size.get()) | (val as u64);
            stack[dst..][..nbytes].copy_from_slice(&v.to_le_bytes()[..nbytes])
        }

        BitsDataRef::SeparateTv(items) => {
            let dst = bytemuck::cast_slice_mut::<u8, u64>(&mut stack[dst..][..items.len() * 8]);
            dst.copy_from_slice(items);
        }
        BitsDataRef::SeparateFv(items) => {
            let dst = bytemuck::cast_slice_mut::<u8, u64>(&mut stack[dst..][..items.len() * 8]);
            dst.copy_from_slice(items);
        }
    }
}
