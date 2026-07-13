use std::fmt;

use vogls_codegen::HeapAlignment;
use vogls_ir::LogicMode;
use vogls_runtime::RuntimeState;

use crate::bytecode::{write_padded_mnemonic, write_register};

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    EXEC_ITRACE_INDENT, InlineNBitSize, Schedule, SixBitSize,
};

pub struct HeapRegSlice {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    dst_size: SixBitSize,
    src_size: InlineNBitSize<6>,
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

#[inline(always)]
pub fn execute<const SRC_FV: bool, const FILL_WITH_X: bool, const OFFSET_IS_FV: bool>(
    operands: HeapRegSlice,
    regs: &mut Regs,
    state: &mut RuntimeState,
) {
    let HeapRegSlice {
        rd,
        rs,
        roff,
        dst_size,
        src_size,
    } = operands;

    let src_size = src_size.get(regs);
    let offset = if OFFSET_IS_FV {
        let (spc, val) = roff.to_spc_and_val();
        if regs[spc] != u32::MAX as u64 {
            let (rdspc, rdval) = rd.to_spc_and_val();
            regs[rdspc] = 0;
            regs[rdval] = 0;
            return;
        }

        regs[val] as u32
    } else {
        regs[roff] as u32
    };

    let valid_mask_size = u32::min(dst_size as u32, src_size.get().saturating_sub(offset));
    if valid_mask_size == 0 {
        if SRC_FV | OFFSET_IS_FV | FILL_WITH_X {
            let (rdspc, rdval) = rd.to_spc_and_val();
            regs[rdspc] = 0;
            regs[rdval] = 0;
        } else {
            regs[rd] = 0;
        }
        return;
    }

    let valid_mask = 1u64.unbounded_shl(valid_mask_size).wrapping_sub(1);
    let start = offset;
    let end = offset + valid_mask_size - 1;

    let heap = state.heap.0.as_ref();
    let src = regs[rs];
    let fst = 'fst: {
        if OFFSET_IS_FV {
            break 'fst valid_mask;
        }

        let start_offset = src.wrapping_add(start as u64);
        let end_offset = src.wrapping_add(end as u64);
        let word = (start_offset / 64) as usize;
        let boff = start_offset % 64;
        let endword = (end_offset / 64) as usize;

        if word == endword {
            break 'fst heap[word] >> boff;
        }

        assert!(heap.len() > 0 && word < heap.len() - 1);
        let w1 = heap[word];
        let w2 = heap[word + 1];
        (w1 >> boff) | (w2 << (64 - boff))
    };

    if !SRC_FV & !FILL_WITH_X & !OFFSET_IS_FV {
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

        assert!(heap.len() > 0 && word < heap.len() - 1);
        let w1 = heap[word];
        let w2 = heap[word + 1];
        (w1 >> boff) | (w2 << (64 - boff))
    };

    let (rdspc, rdval) = rd.to_spc_and_val();
    regs[rdspc] = spc;
    regs[rdval] = val & valid_mask;
}

macro_rules! impl_op {
    ($(($variant:ident, $mnemonic:literal, $name:ident, $src_fv:expr, $fill_with_x:expr, $offset_is_fv:expr))+) => {
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

            fn pre_exec_itrace(
                &self,
                f: &mut fmt::Formatter<'_>,
                regs: &Regs,
                _state: &RuntimeState,
            ) -> fmt::Result {
                f.write_str(EXEC_ITRACE_INDENT)?;
                write_register(f, regs, "rs", self.0.rs, LogicMode::TwoValue)?;
                f.write_str(", ")?;
                write_register(f, regs, "roff", self.0.roff, if $offset_is_fv { LogicMode::FourValue } else { LogicMode::TwoValue })?;
                writeln!(f)?;
                Ok(())
            }
            fn post_exec_itrace(
                &self,
                f: &mut fmt::Formatter<'_>,
                regs: &Regs,
                _state: &RuntimeState,
            ) -> fmt::Result {
                f.write_str(EXEC_ITRACE_INDENT)?;
                write_register(f, regs, "rd", self.0.rd, if $src_fv | $fill_with_x | $offset_is_fv { LogicMode::FourValue } else { LogicMode::TwoValue })?;
                writeln!(f)?;
                Ok(())
            }

            fn execute(
                self,
                regs: &mut Regs,
                _pc: &mut u64,
                state: &mut RuntimeState,
                _schedule: &mut Schedule,
                _listeners: &mut BytecodeListeners,
                _cldctx: &mut ColdContext,
            ) {
                execute::<$src_fv, $fill_with_x, $offset_is_fv>(self.0, regs, state)
            }
        }
        )+

        impl BytecodeEncoder {
            $(
            pub fn $name(&mut self, rd: Reg, rs: Reg, roff: Reg, dst_size: SixBitSize, src_size: InlineNBitSize<6>) {
                self.data.push($variant(HeapRegSlice {
                    rd, rs, roff,
                    dst_size,
                    src_size
                }).encode());
            }
            )+
        }
    };
}

impl_op! {
    (TvTvHeapSlice0, "tvtv.heap_slice0", tvtv_heap_slice0, false, false, false)
    (TvTvHeapSliceX, "tvtv.heap_slicex", tvtv_heap_slicex, false, true, false)
    (TvFvHeapSlice0, "tvfv.heap_slice0", tvfv_heap_slice0, false, false, true)
    (TvFvHeapSliceX, "tvfv.heap_slicex", tvfv_heap_slicex, false, true, true)
    (FvTvHeapSlice0, "fvtv.heap_slice0", fvtv_heap_slice0, true, false, false)
    (FvTvHeapSliceX, "fvtv.heap_slicex", fvtv_heap_slicex, true, true, false)
    (FvFvHeapSlice0, "fvfv.heap_slice0", fvfv_heap_slice0, true, false, true)
    (FvFvHeapSliceX, "fvfv.heap_slicex", fvfv_heap_slicex, true, true, true)
}
