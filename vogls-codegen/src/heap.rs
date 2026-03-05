use std::fmt;

use vogls_bits::arithmetic::{FvLogicValue, fv_pack_u64, fv_set_no_special, fv_unpack_u64};
use vogls_bits::load::load_partial_u64;
use vogls_bits::store::store_partial_u64;
use vogls_bits::{BitsDataRef, get_disjoint_dst_s1_s2, get_disjoint_dst_src};
use vogls_ir::{Bits, INTEGER_VSIZE, LogicMode, SCALAR_VSIZE, TIME_VSIZE, VectorSize};

pub struct HeapBuilder {
    top: usize,
    padding: usize,
}

impl HeapBuilder {
    pub fn new() -> Self {
        Self { top: 0, padding: 0 }
    }

    pub fn claim(&mut self, mode: LogicMode, size: VectorSize) -> HeapRef {
        let (bit_alignment, bit_size) = match mode {
            LogicMode::TwoValue => {
                let nbits = size.get() as usize;
                let alignment = nbits.min(64).next_power_of_two();
                (alignment, nbits.next_multiple_of(alignment))
            }
            LogicMode::FourValue if size.get() <= 16 => {
                let nbits = 2 * size.get() as usize;
                let alignment = nbits.next_power_of_two();
                (alignment, nbits.next_multiple_of(alignment))
            }
            LogicMode::FourValue => (64, 2 * (size.get() as usize).next_multiple_of(64)),
        };

        self.padding += self.top.next_multiple_of(bit_alignment) - self.top;
        self.top = self.top.next_multiple_of(bit_alignment);
        let heap_ref = HeapOffset {
            bit_offset: self.top,
        };
        self.top += bit_size;
        heap_ref.to_ref(size)
    }

    pub fn top(&self) -> usize {
        self.top
    }
    pub fn padding(&self) -> usize {
        self.padding
    }

    pub fn finish(self) -> Heap {
        Heap(vec![0u64; self.top.div_ceil(64)].into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapOffset {
    pub bit_offset: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapRef {
    pub offset: HeapOffset,
    pub size: VectorSize,
}
impl HeapRef {
    pub fn to_fv_size(mut self) -> HeapRef {
        self.size = self.size.checked_mul(VectorSize::new(2).unwrap()).unwrap();
        self
    }

    pub fn prev_byte_align(self) -> HeapRef {
        Self {
            offset: self.offset.prev_byte_align(),
            size: self.size,
        }
    }
    pub fn align_subbits(self, byte: u8) -> u8 {
        debug_assert!(self.size.get() <= 4);
        (byte >> (self.offset.bit_offset % 8)) & ((1u8 << self.size.get()) - 1)
    }
}

impl HeapOffset {
    pub fn to_ref(self, size: VectorSize) -> HeapRef {
        HeapRef { offset: self, size }
    }
    pub fn to_scalar_ref(self) -> HeapRef {
        self.to_ref(SCALAR_VSIZE)
    }
    pub fn to_32bit_ref(self) -> HeapRef {
        self.to_ref(INTEGER_VSIZE)
    }
    pub fn to_64bit_ref(self) -> HeapRef {
        self.to_ref(TIME_VSIZE)
    }
    pub fn prev_byte_align(self) -> Self {
        Self {
            bit_offset: self.bit_offset - self.bit_offset % 8,
        }
    }
}

impl fmt::Display for HeapOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.bit_offset)
    }
}

#[derive(Clone)]
pub struct Heap(pub Box<[u64]>);

pub enum HeapSlice<'a> {
    Bytes(&'a [u8]),
    Bits(u8),
}

impl<'a> HeapSlice<'a> {
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Bytes(s) => s,
            Self::Bits(b) => std::slice::from_ref(b),
        }
    }
}

impl Heap {
    pub const TV_SUBBITS_MAX_SIZE: VectorSize = VectorSize::new(4).unwrap();
    pub const TV_U64_MIN_SIZE: VectorSize = VectorSize::new(65).unwrap();

    pub const FV_SUBBITS_MAX_SIZE: VectorSize = VectorSize::new(2).unwrap();
    pub const FV_U64_MIN_SIZE: VectorSize = VectorSize::new(33).unwrap();

    pub fn get_subbit_byte<'a>(&'a self, at: HeapRef) -> u8 {
        debug_assert!(at.size <= Self::TV_SUBBITS_MAX_SIZE);
        let slice = bytemuck::cast_slice::<u64, u8>(&self.0);
        let start_byte = at.offset.bit_offset / 8;
        let b = slice[start_byte];
        let b = (b >> at.offset.bit_offset % 8) & ((1u8 << at.size.get()) - 1);
        b
    }

    pub fn get<'a>(&'a self, at: HeapRef) -> HeapSlice<'a> {
        if at.size > Self::TV_SUBBITS_MAX_SIZE {
            let slice = bytemuck::cast_slice::<u64, u8>(&self.0);
            let start_byte = at.offset.bit_offset / 8;
            HeapSlice::Bytes(&slice[start_byte..][..at.size.get().div_ceil(8) as usize])
        } else {
            HeapSlice::Bits(self.get_subbit_byte(at))
        }
    }
    pub fn get_mut(&mut self, at: HeapRef) -> &mut [u8] {
        debug_assert!(at.size > Self::TV_SUBBITS_MAX_SIZE);
        &mut bytemuck::cast_slice_mut::<u64, u8>(&mut self.0)[(at.offset.bit_offset / 8)..]
            [..at.size.get().div_ceil(8) as usize]
    }

    pub fn get_u64_slice(&self, at: HeapOffset, nwords: usize) -> &[u64] {
        debug_assert_eq!(at.bit_offset % 64, 0);
        &self.0[at.bit_offset / 64..][..nwords]
    }
    pub fn get_mut_u64_slice(&mut self, at: HeapOffset, nwords: usize) -> &mut [u64] {
        debug_assert_eq!(at.bit_offset % 64, 0);
        &mut self.0[at.bit_offset / 64..][..nwords]
    }

    pub fn get_u64(&self, at: HeapOffset) -> u64 {
        self.get_u64_slice(at, 1)[0]
    }
    pub fn get_mut_u64(&mut self, at: HeapOffset) -> &mut u64 {
        &mut self.get_mut_u64_slice(at, 1)[0]
    }

    pub fn load_exact_tv_u32(&self, at: HeapOffset) -> u32 {
        self.get_tv_u64(at.to_ref(VectorSize::new(32).unwrap())) as u32
    }
    pub fn load_exact_fv_u32(&self, at: HeapOffset) -> (u32, u32) {
        let (spc, val) = self.get_fv_u64(at.to_ref(VectorSize::new(32).unwrap()));
        (spc as u32, val as u32)
    }

    pub fn get_tv_u64(&self, at: HeapRef) -> u64 {
        debug_assert!(at.size.get() <= 64);
        if at.size.get() <= 32 {
            load_partial_u64(self.get(at).as_slice(), at.size)
        } else {
            self.get_u64(at.offset)
        }
    }
    pub fn set_tv_u64(&mut self, at: HeapRef, value: u64) -> u64 {
        debug_assert!(at.size.get() <= 64);
        if at.size <= Self::TV_SUBBITS_MAX_SIZE {
            let old = load_partial_u64(self.get(at).as_slice(), at.size);
            self.set_aligned_raw_bits(at, value as u8);
            old
        } else if at.size < Self::TV_U64_MIN_SIZE {
            let old = load_partial_u64(self.get(at).as_slice(), at.size);
            let dst = bytemuck::cast_slice_mut::<u64, u8>(&mut self.0);
            let dst = &mut dst[(at.offset.bit_offset / 8)..][..at.size.get().div_ceil(8) as usize];
            store_partial_u64(dst, value, at.size);
            old
        } else {
            std::mem::replace(self.get_mut_u64(at.offset), value)
        }
    }
    pub fn set_tv_bool(&mut self, at: HeapOffset, value: bool) {
        self.set_tv_u64(at.to_ref(SCALAR_VSIZE), value.into());
    }
    pub fn get_tv_bool(&self, at: HeapOffset) -> bool {
        let val = self.get_tv_u64(at.to_ref(SCALAR_VSIZE));
        val & 1 != 0
    }

    pub fn get_fv_item(&self, at: HeapOffset) -> FvLogicValue {
        let (spc, val) = self.get_fv_u64(at.to_ref(SCALAR_VSIZE));
        FvLogicValue::from_repr(((spc as u8) << 1) | (val as u8))
    }
    pub fn get_fv_u64(&self, at: HeapRef) -> (u64, u64) {
        debug_assert!(at.size.get() <= 64);
        if at.size < Self::FV_U64_MIN_SIZE {
            let src = self.get(at.to_fv_size());
            let src = src.as_slice();
            fv_unpack_u64(load_partial_u64(src, at.to_fv_size().size), at.size)
        } else {
            let [spc, val] = self.get_u64_slice(at.offset, 2) else {
                unreachable!()
            };
            (*spc, *val)
        }
    }
    pub fn set_fv_u64(&mut self, at: HeapRef, spc: u64, val: u64) -> (u64, u64) {
        debug_assert!(at.size.get() <= 64);
        let dat = at.to_fv_size();
        if at.size <= Self::FV_SUBBITS_MAX_SIZE {
            let src = self.get(dat);
            let old = load_partial_u64(src.as_slice(), dat.size);
            let result = fv_pack_u64(spc, val, at.size);
            self.set_aligned_raw_bits(dat, result as u8);
            fv_unpack_u64(old, at.size)
        } else if at.size < Self::FV_U64_MIN_SIZE {
            let dst = bytemuck::cast_slice_mut::<u64, u8>(&mut self.0);
            let dst = &mut dst[(at.offset.bit_offset / 8)..][..dat.size.get().div_ceil(8) as usize];
            let old = load_partial_u64(dst, dat.size);
            store_partial_u64(dst, fv_pack_u64(spc, val, at.size), dat.size);
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
        dst: (HeapOffset, usize),
        src: (HeapOffset, usize),
    ) -> (&mut [u64], &[u64]) {
        debug_assert_eq!(dst.0.bit_offset % 64, 0);
        debug_assert_eq!(src.0.bit_offset % 64, 0);
        get_disjoint_dst_src(
            &mut self.0,
            dst.0.bit_offset / 64,
            dst.1,
            src.0.bit_offset / 64,
            src.1,
        )
    }

    pub fn get_disjoint_u64_dst_s1_s2(
        &mut self,
        dst: (HeapOffset, usize),
        src1: (HeapOffset, usize),
        src2: (HeapOffset, usize),
    ) -> (&mut [u64], &[u64], &[u64]) {
        debug_assert_eq!(dst.0.bit_offset % 64, 0);
        debug_assert_eq!(src1.0.bit_offset % 64, 0);
        debug_assert_eq!(src2.0.bit_offset % 64, 0);
        get_disjoint_dst_s1_s2(
            &mut self.0,
            dst.0.bit_offset / 64,
            dst.1,
            src1.0.bit_offset / 64,
            src1.1,
            src2.0.bit_offset / 64,
            src2.1,
        )
    }

    pub fn get_disjoint_u8_dst_src(&mut self, dst: HeapRef, src: HeapRef) -> (&mut [u8], &[u8]) {
        debug_assert_eq!(dst.offset.bit_offset % 8, 0);
        debug_assert_eq!(src.offset.bit_offset % 8, 0);
        let dst_bytes = dst.size.get().div_ceil(8) as usize;
        let src_bytes = src.size.get().div_ceil(8) as usize;
        get_disjoint_dst_src(
            bytemuck::cast_slice_mut(&mut self.0),
            dst.offset.bit_offset / 8,
            dst_bytes,
            src.offset.bit_offset / 8,
            src_bytes,
        )
    }

    pub fn get_disjoint_u8_dst_s1_s2(
        &mut self,
        dst: HeapRef,
        src1: HeapRef,
        src2: HeapRef,
    ) -> (&mut [u8], &[u8], &[u8]) {
        debug_assert_eq!(dst.offset.bit_offset % 8, 0);
        debug_assert_eq!(src1.offset.bit_offset % 8, 0);
        debug_assert_eq!(src2.offset.bit_offset % 8, 0);
        get_disjoint_dst_s1_s2(
            bytemuck::cast_slice_mut(&mut self.0),
            dst.offset.bit_offset / 8,
            dst.size.get().div_ceil(8) as usize,
            src1.offset.bit_offset / 8,
            src1.size.get().div_ceil(8) as usize,
            src2.offset.bit_offset / 8,
            src2.size.get().div_ceil(8) as usize,
        )
    }

    pub fn load_tv_bits(&self, at: HeapRef) -> Bits {
        // @Performance: We should make a specialized path for u64
        Bits::load_from_slice(self.get(at).as_slice(), at.size)
    }
    pub fn load_fv_bits(&self, at: HeapRef) -> Bits {
        if at.size < Self::FV_U64_MIN_SIZE {
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

    pub fn load_bits(&self, at: HeapRef, logic_mode: LogicMode) -> Bits {
        match logic_mode {
            LogicMode::TwoValue => self.load_tv_bits(at),
            LogicMode::FourValue => self.load_fv_bits(at),
        }
    }

    pub fn store_bits(&mut self, dst: HeapRef, logic_mode: LogicMode, value: &Bits) {
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

    pub fn set_fv_scalar(&mut self, at: HeapOffset, value: FvLogicValue) {
        let (spc, val) = ((value as u64) >> 1, (value as u64) & 1);
        self.set_fv_u64(at.to_ref(SCALAR_VSIZE), spc, val);
    }

    pub fn set_aligned_raw_bits(&mut self, at: HeapRef, byte: u8) {
        debug_assert!(at.size <= Self::TV_SUBBITS_MAX_SIZE);
        debug_assert!(
            ((at.offset.bit_offset - (at.offset.bit_offset % 8))
                ..=(at.offset.bit_offset - (at.offset.bit_offset % 8) + 8))
                .contains(&(at.offset.bit_offset + at.size.get() as usize)),
        );
        let shift = at.offset.bit_offset % 64;
        let shifted_mask = ((1u64 << at.size.get()) - 1) << shift;
        self.0[at.offset.bit_offset / 64] &= !shifted_mask;
        self.0[at.offset.bit_offset / 64] |= ((byte as u64) << shift) & shifted_mask;
    }
    pub fn set_unaligned_raw_bits(&mut self, at: HeapRef, byte: u8) -> bool {
        debug_assert!(at.size <= Self::TV_SUBBITS_MAX_SIZE);

        // First word
        let shift = at.offset.bit_offset % 64;
        let mask = (1u64 << at.size.get()) - 1;
        let shifted_mask = mask << shift;
        let w = &mut self.0[at.offset.bit_offset / 64];
        let before = *w;
        *w &= !shifted_mask;
        *w |= ((byte as u64) << shift) & shifted_mask;
        let mut updated = before != *w;

        // Second word
        if (at.offset.bit_offset % 64) + at.size.get() as usize > 64 {
            let w = &mut self.0[(at.offset.bit_offset / 64) + 1];
            let before = *w;
            let shift = 64 - shift;
            let shifted_mask = mask >> shift;
            *w &= !shifted_mask;
            *w |= ((byte as u64) >> shift) & shifted_mask;
            updated |= before != *w;
        }

        updated
    }
}
