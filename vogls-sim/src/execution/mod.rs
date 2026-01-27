use vogls_bits::BitsDataRef;
use vogls_ir::Bits;

use crate::{Stack, StackOffset};

pub(super) mod fv;
pub(super) mod tv;

pub(crate) fn exec_constant(stack: &mut Stack, dst: StackOffset, value: &Bits) {
    let size = value.size();
    match value.as_data_ref() {
        BitsDataRef::InlineTv(v) => _ = stack.set_tv_u64(dst.to_ref(size), v),
        BitsDataRef::InlineFv(spc, val) => {
            _ = stack.set_fv_u64(dst.to_ref(size), spc as u64, val as u64)
        }
        BitsDataRef::SeparateTv(items) | BitsDataRef::SeparateFv(items) => {
            stack
                .get_mut_u64_slice(dst, items.len())
                .copy_from_slice(items);
        }
    }
}
