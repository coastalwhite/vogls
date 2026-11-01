use std::fmt;
use std::ops::RangeInclusive;

use vogls_codegen::{HeapAlignment, SixBitSize};
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RuntimeState;

use crate::write_padded_mnemonic;

use super::reg::{Reg, RegInfo, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineNBitSize, Schedule,
};

pub struct HeapRegSlice {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    dst_size: SixBitSize,
    src_size: InlineNBitSize<6>,
}

pub struct HeapRegSliceVarBounds {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    rlb: Reg,
    dst_size: SixBitSize,
}

impl HeapRegSlice {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            dst_size: SixBitSize::new_masked(v >> 20),
            src_size: InlineNBitSize::new_masked(v >> 26),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.dst_size.encode() << 20)
                | (self.src_size.encode() << 26),
        )
    }
}

impl HeapRegSliceVarBounds {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            rlb: Reg::new_masked(v >> 20),
            dst_size: SixBitSize::new_masked(v >> 24),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | ((self.rlb as u32) << 20)
                | (self.dst_size.encode() << 24),
        )
    }
}

#[inline(always)]
pub fn execute<const SRC_FV: bool, const FILL_WITH_X: bool, const USE_BOUNDS: bool>(
    rd: Reg,
    rs: Reg,
    roff: Reg,
    dst_size: SixBitSize,
    src_size: VectorSize,
    lower_bound: u32,
    upper_bound: u32,
    regs: &mut Regs,
    state: &mut RuntimeState,
) {
    let offset = regs[roff] as u32;

    let slice_end = offset.saturating_add(dst_size as u32);
    let (start, end) = if USE_BOUNDS {
        (
            u32::max(offset, lower_bound),
            slice_end
                .min(upper_bound.saturating_add(1))
                .min(src_size.get()),
        )
    } else {
        (offset, slice_end.min(src_size.get()))
    };

    if start >= end {
        if SRC_FV | FILL_WITH_X {
            let (rdspc, rdval) = rd.to_spc_and_val();
            regs[rdspc] = 0;
            regs[rdval] = 0;
        } else {
            regs[rd] = 0;
        }
        return;
    }

    let shift = start - offset;
    let valid_mask_size = end - start;
    let valid_mask = 1u64.unbounded_shl(valid_mask_size).wrapping_sub(1) << shift;
    let end = end - 1;

    let heap = state.heap.0.as_ref();
    let src = regs[rs];
    let fst = 'fst: {
        let start_offset = src.wrapping_add(start as u64);
        let end_offset = src.wrapping_add(end as u64);
        let word = (start_offset / 64) as usize;
        let boff = start_offset % 64;
        let endword = (end_offset / 64) as usize;

        if word == endword {
            break 'fst heap[word] >> boff;
        }

        assert!(!heap.is_empty() && word < heap.len() - 1);
        let w1 = heap[word];
        let w2 = heap[word + 1];
        (w1 >> boff) | (w2 << (64 - boff))
    } << shift;

    if !SRC_FV & !FILL_WITH_X {
        regs[rd] = fst & valid_mask;
        return;
    }

    if !SRC_FV {
        let (rdspc, rdval) = rd.to_spc_and_val();
        regs[rdspc] = if FILL_WITH_X {
            valid_mask
        } else {
            dst_size.mask(u64::MAX)
        };
        regs[rdval] = fst & valid_mask;
        return;
    }

    let spc = if FILL_WITH_X {
        fst & valid_mask
    } else {
        fst | !valid_mask
    };

    let val = 'val: {
        let val_offset = HeapAlignment::spc_offset_to_val_offset(src_size, src);
        let start_offset = val_offset.wrapping_add(start.into());
        let end_offset = val_offset.wrapping_add(end.into());
        let word = (start_offset / 64) as usize;
        let boff = start_offset % 64;
        let endword = (end_offset / 64) as usize;

        if word == endword {
            break 'val heap[word] >> boff;
        }

        assert!(!heap.is_empty() && word < heap.len() - 1);
        let w1 = heap[word];
        let w2 = heap[word + 1];
        (w1 >> boff) | (w2 << (64 - boff))
    } << shift;

    let (rdspc, rdval) = rd.to_spc_and_val();
    regs[rdspc] = spc;
    regs[rdval] = val & valid_mask;
}

macro_rules! impl_op {
    ($(($variant:ident, $mnemonic:literal, $name:ident, $src_fv:expr, $fill_with_x:expr$(, $bounds:ident = $has_bounds:expr)?))+) => {
        $(
        pub struct $variant(HeapRegSlice);

        impl BytecodeInstruction for $variant {
            fn encode(&self) -> Bytecode {
                self.0.encode(BytecodeOpcode::$variant)
            }

            fn extract(v: Bytecode) -> Self {
                debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
                Self(HeapRegSlice::extract(v))
            }

            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let HeapRegSlice {
                    rd,
                    rs,
                    roff,
                    dst_size,
                    src_size,
                } = self.0;
                write_padded_mnemonic(f, $mnemonic)?;
                write!(f, "{rd}, {rs}, {roff}, {dst_size}, {src_size}")
            }

            fn num_additional_slots(&self) -> u8 {
                let mut num_additional_slots = 0u8;
                $(
                if $has_bounds {
                    num_additional_slots += 2;
                }
                )?
                num_additional_slots += u8::from(self.0.src_size.0.is_none());
                num_additional_slots
            }

            fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
                let mut pc = pc + 1 + if false $(|| $has_bounds)? { 2 } else { 0 };
                operands.push(RegInfo::heap("rs", self.0.rs, if $src_fv { LogicMode::FourValue } else { LogicMode::TwoValue }, self.0.src_size.get(&mut pc, code)));
                operands.push(RegInfo::register("roff", self.0.roff, LogicMode::TwoValue, None));
            }
            fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
                operands.push(RegInfo::register("rd", self.0.rd, if $src_fv | $fill_with_x { LogicMode::FourValue } else { LogicMode::TwoValue }, Some(self.0.dst_size.into())));
            }

            #[inline(always)]
            fn execute(
                self,
                code: &[Bytecode],
                regs: &mut Regs,
                pc: &mut u64,
                state: &mut RuntimeState,
                _schedule: &mut Schedule,
                _listeners: &mut BytecodeListeners,
                _cldctx: &mut ColdContext,
            ) {
                let HeapRegSlice {
                    rd,
                    rs,
                    roff,
                    dst_size,
                    src_size,
                } = self.0;

                // @NOTE: We ensured the code buffer is padded with at least 16 elements.
                let additional_slots: &[Bytecode; 16] = code
                    .get(*pc as usize..)
                    .and_then(<[_]>::first_chunk)
                    .expect("Should be padded");

                *pc += self.num_additional_slots() as u64;

                let lower_bound = if false $(|| $has_bounds)? {
                    additional_slots[0].0
                } else {
                    0
                };
                let upper_bound = if false $(|| $has_bounds)? {
                    additional_slots[1].0
                } else {
                    0
                };

                let mut src_size_slot = if false $(|| $has_bounds)? { 2 } else { 0 };
                let src_size = src_size.get(&mut src_size_slot, additional_slots);
                execute::<$src_fv, $fill_with_x, { false $(| $has_bounds)? }>(rd, rs, roff, dst_size, src_size, lower_bound, upper_bound, regs, state)
            }
        }
        )+

        impl BytecodeEncoder {
            $(
            pub fn $name(&mut self, rd: Reg, rs: Reg, roff: Reg, dst_size: SixBitSize, src_size: VectorSize
                $(, $bounds: RangeInclusive<u32>)?

                ) {
                let inline_src_size = InlineNBitSize::new(src_size);
                self.data.push($variant(HeapRegSlice {
                    rd, rs, roff,
                    dst_size,
                    src_size: inline_src_size,
                }).encode());
                $(
                self.data.push(Bytecode(*$bounds.start()));
                self.data.push(Bytecode(*$bounds.end()));
                )?
                if inline_src_size.0.is_none() {
                    self.data.push(Bytecode(src_size.get()));
                }
            }
            )+
        }
    };
}

macro_rules! impl_varbounds_op {
    ($(($variant:ident, $mnemonic:literal, $name:ident, $src_fv:expr))+) => {
        $(
        pub struct $variant(HeapRegSliceVarBounds);

        impl BytecodeInstruction for $variant {
            fn encode(&self) -> Bytecode {
                self.0.encode(BytecodeOpcode::$variant)
            }

            fn extract(v: Bytecode) -> Self {
                debug_assert_eq!(v.opcode(), BytecodeOpcode::$variant as u8);
                Self(HeapRegSliceVarBounds::extract(v))
            }

            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let HeapRegSliceVarBounds {
                    rd,
                    rs,
                    roff,
                    rlb,
                    dst_size,
                } = self.0;
                write_padded_mnemonic(f, $mnemonic)?;
                write!(f, "{rd}, {rs}, {roff}, {rlb}, {dst_size}")
            }

            fn num_additional_slots(&self) -> u8 {
                2
            }

            fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
                let src_size = code[(pc + 1) as usize].unwrap_size();
                operands.push(RegInfo::heap("rs", self.0.rs, if $src_fv { LogicMode::FourValue } else { LogicMode::TwoValue }, src_size));
                operands.push(RegInfo::register("roff", self.0.roff, LogicMode::TwoValue, None));
            }
            fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
                operands.push(RegInfo::register("rd", self.0.rd, LogicMode::FourValue, Some(self.0.dst_size.into())));
            }

            #[inline(always)]
            fn execute(
                self,
                code: &[Bytecode],
                regs: &mut Regs,
                pc: &mut u64,
                state: &mut RuntimeState,
                _schedule: &mut Schedule,
                _listeners: &mut BytecodeListeners,
                _cldctx: &mut ColdContext,
            ) {
                let HeapRegSliceVarBounds {
                    rd,
                    rs,
                    roff,
                    rlb,
                    dst_size,
                } = self.0;

                // @NOTE: We ensured the code buffer is padded with at least 16 elements.
                let additional_slots: &[Bytecode; 16] = code
                    .get(*pc as usize..)
                    .and_then(<[_]>::first_chunk)
                    .expect("Should be padded");

                *pc += self.num_additional_slots() as u64;

                let src_size = additional_slots[0].unwrap_size();
                let bound_width = additional_slots[1].0;

                let lower_bound = regs[rlb] as u32;
                let upper_bound = lower_bound.saturating_add(bound_width - 1);

                execute::<$src_fv, true, true>(rd, rs, roff, dst_size, src_size, lower_bound, upper_bound, regs, state)
            }
        }
        )+

        impl BytecodeEncoder {
            $(
            pub fn $name(&mut self, rd: Reg, rs: Reg, roff: Reg, rlb: Reg, dst_size: SixBitSize, src_size: VectorSize, bound_width: VectorSize) {
                self.data.push($variant(HeapRegSliceVarBounds {
                    rd, rs, roff, rlb,
                    dst_size,
                }).encode());
                self.data.push(Bytecode(src_size.get()));
                self.data.push(Bytecode(bound_width.get()));
            }
            )+
        }
    };
}

impl_op! {
    (TvHeapSlice0, "tv.heap_slice0", tv_heap_slice0, false, false)
    (TvHeapSliceX, "tv.heap_slicex", tv_heap_slicex, false, true)
    (FvHeapSlice0, "fv.heap_slice0", fv_heap_slice0, true, false)
    (FvHeapSliceX, "fv.heap_slicex", fv_heap_slicex, true, true)

    (TvHeapSliceXBounds, "tv.heap_slicex_bounds", tv_heap_slicex_bounds, false, true, bounds = true)
    (FvHeapSliceXBounds, "fv.heap_slicex_bounds", fv_heap_slicex_bounds, true, true, bounds = true)
}

impl_varbounds_op! {
    (TvHeapSliceXVarBounds, "tv.heap_slicex_varbounds", tv_heap_slicex_varbounds, false)
    (FvHeapSliceXVarBounds, "fv.heap_slicex_varbounds", fv_heap_slicex_varbounds, true)
}
