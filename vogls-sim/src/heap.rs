use vogls_bits::arithmetic::{FvLogicValue, fv_pack_u64, fv_set_no_special, fv_unpack_u64};
use vogls_bits::load::load_partial_u64;
use vogls_bits::store::store_partial_u64;
use vogls_bits::{BitsDataRef, get_disjoint_dst_s1_s2, get_disjoint_dst_src};
use vogls_ir::{Bits, LogicMode, SCALAR_VSIZE, VectorSize};

use crate::{StackOffset, StackRef};

pub struct Stack(pub Box<[u64]>);

impl Stack {
    pub fn get(&self, at: StackRef) -> &[u8] {
        &bytemuck::cast_slice::<u64, u8>(&self.0)[at.offset.0..]
            [..at.size.get().div_ceil(8) as usize]
    }
    pub fn get_mut(&mut self, at: StackRef) -> &mut [u8] {
        &mut bytemuck::cast_slice_mut::<u64, u8>(&mut self.0)[at.offset.0..]
            [..at.size.get().div_ceil(8) as usize]
    }

    pub fn get_u64_slice(&self, at: StackOffset, nwords: usize) -> &[u64] {
        debug_assert_eq!(at.0 % 8, 0);
        &self.0[at.0 / 8..][..nwords]
    }
    pub fn get_mut_u64_slice(&mut self, at: StackOffset, nwords: usize) -> &mut [u64] {
        debug_assert_eq!(at.0 % 8, 0);
        &mut self.0[at.0 / 8..][..nwords]
    }

    pub fn get_u64(&self, at: StackOffset) -> u64 {
        self.get_u64_slice(at, 1)[0]
    }
    pub fn get_mut_u64(&mut self, at: StackOffset) -> &mut u64 {
        &mut self.get_mut_u64_slice(at, 1)[0]
    }

    pub fn load_exact_tv_u32(&self, at: StackOffset) -> u32 {
        self.get_tv_u64(at.to_ref(VectorSize::new(32).unwrap())) as u32
    }
    pub fn load_exact_fv_u32(&self, at: StackOffset) -> (u32, u32) {
        let (spc, val) = self.get_fv_u64(at.to_ref(VectorSize::new(32).unwrap()));
        (spc as u32, val as u32)
    }

    pub fn get_tv_u64(&self, at: StackRef) -> u64 {
        debug_assert!(at.size.get() <= 64);
        if at.size.get() <= 32 {
            load_partial_u64(self.get(at), at.size)
        } else {
            self.get_u64(at.offset)
        }
    }
    pub fn set_tv_u64(&mut self, at: StackRef, value: u64) -> u64 {
        debug_assert!(at.size.get() <= 64);
        if at.size.get() <= 32 {
            let old = load_partial_u64(self.get(at), at.size);
            store_partial_u64(self.get_mut(at), value, at.size);
            old
        } else {
            std::mem::replace(self.get_mut_u64(at.offset), value)
        }
    }
    pub fn set_tv_bool(&mut self, at: StackOffset, value: bool) {
        self.set_tv_u64(at.to_ref(SCALAR_VSIZE), value.into());
    }
    pub fn get_tv_bool(&self, at: StackOffset) -> bool {
        let val = self.get_tv_u64(at.to_ref(SCALAR_VSIZE));
        val & 1 != 0
    }

    pub fn get_fv_item(&self, at: StackOffset) -> FvLogicValue {
        let (spc, val) = self.get_fv_u64(at.to_ref(SCALAR_VSIZE));
        FvLogicValue::from_repr(((spc as u8) << 1) | (val as u8))
    }
    pub fn get_fv_u64(&self, at: StackRef) -> (u64, u64) {
        debug_assert!(at.size.get() <= 64);
        if at.size.get() <= 16 {
            let dsize = at.size.checked_mul(VectorSize::new(2).unwrap()).unwrap();
            let src = self.get(at.offset.to_ref(dsize));
            fv_unpack_u64(load_partial_u64(src, dsize), at.size)
        } else {
            let [spc, val] = self.get_u64_slice(at.offset, 2) else {
                unreachable!()
            };
            (*spc, *val)
        }
    }
    pub fn set_fv_u64(&mut self, at: StackRef, spc: u64, val: u64) -> (u64, u64) {
        debug_assert!(at.size.get() <= 64);
        if at.size.get() <= 16 {
            let dsize = at.size.checked_mul(VectorSize::new(2).unwrap()).unwrap();
            let dst = self.get_mut(at.offset.to_ref(dsize));
            let old = load_partial_u64(dst, dsize);
            store_partial_u64(dst, fv_pack_u64(spc, val, at.size), dsize);
            fv_unpack_u64(old, at.size)
        } else {
            let s = self.get_mut_u64_slice(at.offset, 2);
            (
                std::mem::replace(&mut s[0], spc),
                std::mem::replace(&mut s[1], val),
            )
        }
    }

    pub fn get_disjoint_u64_dst_src(
        &mut self,
        dst: (StackOffset, usize),
        src: (StackOffset, usize),
    ) -> (&mut [u64], &[u64]) {
        debug_assert_eq!(dst.0.0 % 8, 0);
        debug_assert_eq!(src.0.0 % 8, 0);
        get_disjoint_dst_src(&mut self.0, dst.0.0 / 8, dst.1, src.0.0 / 8, src.1)
    }

    pub fn get_disjoint_u64_dst_s1_s2(
        &mut self,
        dst: (StackOffset, usize),
        src1: (StackOffset, usize),
        src2: (StackOffset, usize),
    ) -> (&mut [u64], &[u64], &[u64]) {
        debug_assert_eq!(dst.0.0 % 8, 0);
        debug_assert_eq!(src1.0.0 % 8, 0);
        debug_assert_eq!(src2.0.0 % 8, 0);
        get_disjoint_dst_s1_s2(
            &mut self.0,
            dst.0.0 / 8,
            dst.1,
            src1.0.0 / 8,
            src1.1,
            src2.0.0 / 8,
            src2.1,
        )
    }

    pub fn get_disjoint_u8_dst_src(&mut self, dst: StackRef, src: StackRef) -> (&mut [u8], &[u8]) {
        let dst_bytes = dst.size.get().div_ceil(8) as usize;
        let src_bytes = src.size.get().div_ceil(8) as usize;
        get_disjoint_dst_src(
            bytemuck::cast_slice_mut(&mut self.0),
            dst.offset.0,
            dst_bytes,
            src.offset.0,
            src_bytes,
        )
    }

    pub fn get_disjoint_u8_dst_s1_s2(
        &mut self,
        dst: StackRef,
        src1: StackRef,
        src2: StackRef,
    ) -> (&mut [u8], &[u8], &[u8]) {
        get_disjoint_dst_s1_s2(
            bytemuck::cast_slice_mut(&mut self.0),
            dst.offset.0,
            dst.size.get().div_ceil(8) as usize,
            src1.offset.0,
            src1.size.get().div_ceil(8) as usize,
            src2.offset.0,
            src2.size.get().div_ceil(8) as usize,
        )
    }

    pub fn load_tv_bits(&self, at: StackRef) -> Bits {
        // @Performance: We should make a specialized path for u64
        Bits::load_from_slice(self.get(at), at.size)
    }
    pub fn load_fv_bits(&self, at: StackRef) -> Bits {
        if at.size.get() <= 32 {
            let (spc, val) = self.get_fv_u64(at);
            Bits::from_four_value_u64(at.size, spc as u32, val as u32)
        } else {
            Bits::from_boxed_slice(
                vogls_ir::Mode::FourValue,
                at.size,
                self.get_u64_slice(at.offset, 2 * at.size.get().div_ceil(64) as usize)
                    .into(),
            )
        }
    }

    pub fn store_bits(&mut self, dst: StackRef, logic_mode: LogicMode, value: &Bits) {
        match (value.as_data_ref(), logic_mode) {
            (BitsDataRef::InlineTv(v), LogicMode::TwoValue) => _ = self.set_tv_u64(dst, v),
            (BitsDataRef::InlineTv(v), LogicMode::FourValue) => {
                _ = self.set_fv_u64(dst, 1u64.unbounded_shl(dst.size.get()).wrapping_sub(1), v)
            }
            (BitsDataRef::InlineFv(..), LogicMode::TwoValue) => unreachable!(),
            (BitsDataRef::InlineFv(spc, val), LogicMode::FourValue) => {
                _ = self.set_fv_u64(dst, spc as u64, val as u64);
            }
            (BitsDataRef::SeparateTv(items), LogicMode::TwoValue)
            | (BitsDataRef::SeparateFv(items), LogicMode::FourValue) => {
                self.get_mut_u64_slice(dst.offset, items.len())
                    .copy_from_slice(items);
            }
            (BitsDataRef::SeparateTv(items), LogicMode::FourValue) => {
                let target = self.get_mut_u64_slice(dst.offset, items.len() * 2);
                fv_set_no_special(target, dst.size);
                target[items.len()..].copy_from_slice(items);
            }
            (BitsDataRef::SeparateFv(..), LogicMode::TwoValue) => unreachable!(),
        }
    }

    pub fn set_fv_scalar(&mut self, at: StackOffset, value: FvLogicValue) {
        let (spc, val) = ((value as u64) >> 1, (value as u64) & 1);
        self.set_fv_u64(at.to_ref(SCALAR_VSIZE), spc, val);
    }
}
