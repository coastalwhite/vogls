use std::{cmp, fmt};

use vogls_bits::set_subslice::set_with_mask;
use vogls_codegen::{HeapOffset, SixBitSize};
use vogls_ir::{LogicMode, VSIZE_64, VectorSize};
use vogls_runtime::RuntimeState;

use crate::reg::{Reg, RegInfo, Regs};
use crate::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineNBitSize, Schedule, write_padded_mnemonic,
};

/// Set bits at given address in the heap without any aligned guarantees.
///
/// Write the `size` bits in `rs` into the heap at the address given by `(slot[0] << 10) | imm10`
/// and fills `rd` with a mask of the bits which were updated.
///
/// # Invariants
/// - `rs` is assumed to only contain bits in the least significant `size` bits.
/// - The calculated address is assumed to be in bounds of the target.
pub struct SetUnaligned {
    rd: Reg,
    rs: Reg,
    size: SixBitSize,
    imm10: u16,
}

/// Set bits at register-relative address in the heap without any aligned guarantees.
///
/// Write the `size` bits in `rs` into the heap at the address given by `raddr` and fills `rd` with
/// a mask of the bits which were updated. The written bits are generally assumed to be in
/// `base..base + base_size` where `base = (slot[1] << 32) | slot[0]`. A write to bits outside this
/// address range is a no-op and returns `0` in the destination register.
///
/// # Invariants
/// - `rs` is assumed to only contain bits in the least significant `size` bits.
pub struct RelSetUnaligned {
    rd: Reg,
    rs: Reg,
    raddr: Reg,
    size: SixBitSize,
    base_size: InlineNBitSize<6>,
}

/// Copy `size` bits on the heap from an aligned address given by `rs` to an unaligned address
/// given by `raddr` and write a mask of updated bits at the address given by `rd`.
///
/// The written bits are generally assumed to be in `base..base + base_size` where `base = (slot[1]
/// << 32) | slot[0]` and `base_size = slot[2]`. A write to bits outside this address range is a
/// no-op and returns `0` in the destination.
///
/// If `fv=true`, the same operation is performed in both the value and special plane with the
/// offset calculated from the `base` and `base_size`. The mask is the OR sum of the operation on
/// both planes.
///
/// # Invariants
/// - `rs` is assumed to only contain bits in the least significant `size` bits.
pub struct SetHeapUnaligned {
    rd: Reg,
    rs: Reg,
    raddr: Reg,
    fv: bool,
    size: InlineNBitSize<11>,
}

/// Set a word on the heap at a specific offset.
///
/// This overwrites all the bits in `offset..offset + size` with `value` and returns a mask of
/// which bits in value changed bits on the heap. If an addressed bit falls outside of
/// `base..base + base_size`, it cannot be changed and any write is considered a no-op.
///
/// # Invariants
///
/// - `value` should be premasked according to `size`.
/// - `base..base + base_size` should be a valid u64 range.
#[inline(always)]
fn set_unaligned_check_bounds(
    heap: &mut [u64],
    offset: u64,
    value: u64,
    size: SixBitSize,
    base: u64,
    base_size: VectorSize,
) -> u64 {
    debug_assert_eq!(value, size.mask(value));

    let base_min = base;
    let base_max = base + base_size.get() as u64;
    if (offset < base_min) | (offset.saturating_add(size as u64) > base_max) {
        return set_unaligned_oob(heap, offset, value, size, base, base_size);
    }

    set_unaligned_inbounds(heap, offset, value, size)
}

#[cold]
#[inline(never)]
fn set_unaligned_oob(
    heap: &mut [u64],
    offset: u64,
    value: u64,
    size: SixBitSize,
    base: u64,
    base_size: VectorSize,
) -> u64 {
    let base_min = base;
    let base_max = base + base_size.get() as u64;
    let write_min = offset;
    let write_max = offset.saturating_add(size as u64);

    let trim_min = cmp::max(base_min, write_min);
    let trim_max = cmp::min(base_max, write_max);

    let trim_size = trim_max.saturating_sub(trim_min);
    debug_assert!(trim_size < size as u64);
    let Some(trim_size) = SixBitSize::new(trim_size as u8) else {
        return 0;
    };

    let min_shift = trim_min - offset;
    let trim_value = trim_size.mask(value >> min_shift);

    set_unaligned_inbounds(heap, trim_min, trim_value, trim_size) << min_shift
}

#[inline(always)]
fn set_unaligned_inbounds(heap: &mut [u64], offset: u64, value: u64, size: SixBitSize) -> u64 {
    debug_assert_eq!(value, size.mask(value));

    let mask = size.mask(u64::MAX);
    let end_offset = offset + size as u64 - 1;

    let word = (offset / 64) as usize;
    let boff = offset % 64;
    let endword = (end_offset / 64) as usize;

    if word == endword {
        let word = &mut heap[word];
        let prev = mask & (*word >> boff);
        *word &= !(mask << boff);
        *word |= value << boff;
        return prev ^ value;
    }

    // Since word != endword, boff should never be 0.
    debug_assert_ne!(boff, 0);
    let tgt: &mut [u64; 2] = (&mut heap[word..word + 2])
        .try_into()
        .expect("Unable to take heap words");
    let prev = mask & ((tgt[0] >> boff) | (tgt[1] << (64 - boff)));
    tgt[0] &= !(mask << boff);
    tgt[0] |= value << boff;
    tgt[1] &= !(mask >> (64 - boff));
    tgt[1] |= value >> (64 - boff);
    prev ^ value
}

impl BytecodeInstruction for SetUnaligned {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::SetUnaligned as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
            imm10: (v >> 22) as u16,
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::SetUnaligned as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.size.encode() << 16)
                | ((self.imm10 as u32) << 22),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            size,
            imm10,
        } = self;
        write_padded_mnemonic(f, "set_unaligned")?;
        write!(f, "{rd}, {rs}, {imm10}, |{size}|")
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.rs,
            LogicMode::TwoValue,
            Some(self.size.into()),
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.rd,
            LogicMode::TwoValue,
            Some(self.size.into()),
        ));
    }

    fn num_additional_slots(&self) -> u8 {
        1
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
        let Self {
            rd,
            rs,
            size,
            imm10,
        } = self;

        let code_offset = code[*pc as usize].0;
        *pc += 1;
        let offset = ((code_offset as u64) << 10) | (imm10 as u64);
        let val = regs[rs];
        let heap = state.heap.0.as_mut();
        let updated = set_unaligned_inbounds(heap, offset, val, size);
        regs[rd] = updated;
    }
}

impl BytecodeInstruction for RelSetUnaligned {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::RelSetUnaligned as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            raddr: Reg::new_masked(v >> 16),
            size: SixBitSize::new_masked(v >> 20),
            base_size: InlineNBitSize::new_masked(v >> 26),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::RelSetUnaligned as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.raddr as u32) << 16)
                | (self.size.encode() << 20)
                | (self.base_size.encode() << 26),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            raddr,
            size,
            base_size,
        } = self;
        write_padded_mnemonic(f, "set_rel_unaligned")?;
        write!(f, "{rd}, {rs}, {raddr}, |{size}|, |{base_size}|")
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.rs,
            LogicMode::TwoValue,
            Some(self.size.into()),
        ));
        operands.push(RegInfo::register(
            "raddr",
            self.raddr,
            LogicMode::TwoValue,
            Some(VSIZE_64),
        ));
    }
    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.rd,
            LogicMode::TwoValue,
            Some(self.size.into()),
        ));
    }

    fn num_additional_slots(&self) -> u8 {
        2 + u8::from(self.base_size.0.is_none())
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
        let Self {
            rd,
            rs,
            raddr,
            size,
            base_size,
        } = self;

        let slots: &[Bytecode; 2] = (&code[*pc as usize..*pc as usize + 2])
            .try_into()
            .expect("Unable to grab expected slots");
        let base_addr = ((slots[1].0 as u64) << 32) | (slots[0].0 as u64);
        *pc += 2;
        let base_size = base_size.get(pc, code);

        let offset = regs[raddr];
        let value = regs[rs];
        let heap = state.heap.0.as_mut();
        let updated = set_unaligned_check_bounds(heap, offset, value, size, base_addr, base_size);
        regs[rd] = updated;
    }
}

impl BytecodeInstruction for SetHeapUnaligned {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::SetHeapUnaligned as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            raddr: Reg::new_masked(v >> 16),
            fv: (v >> 20) & 1 != 0,
            size: InlineNBitSize::new_masked(v >> 21),
        }
    }
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::SetHeapUnaligned as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.raddr as u32) << 16)
                | ((self.fv as u32) << 20)
                | (self.size.encode() << 21),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            raddr,
            fv,
            size,
        } = self;
        let mnemonic = if *fv {
            "fv_set_heap_unaligned"
        } else {
            "tv_set_heap_unaligned"
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs}, {raddr}, {size}")
    }
    fn num_additional_slots(&self) -> u8 {
        3 + if self.fv { 2 } else { 0 } + u8::from(self.size.0.is_none())
    }

    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc;
        if self.fv {
            pc += 6;
        } else {
            pc += 4;
        };
        let size = self.size.get(&mut pc, code);
        operands.push(RegInfo::heap(
            "rs",
            self.rs,
            if self.fv {
                LogicMode::FourValue
            } else {
                LogicMode::TwoValue
            },
            size,
        ));
        operands.push(RegInfo::heap(
            "raddr",
            self.raddr,
            LogicMode::TwoValue,
            size,
        ));
    }
    fn dest_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
        let mut pc = pc;
        if self.fv {
            pc += 6;
        } else {
            pc += 4;
        };
        let size = self.size.get(&mut pc, code);
        operands.push(RegInfo::heap("rd", self.rd, LogicMode::TwoValue, size));
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
        cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs,
            raddr,
            fv,
            size,
        } = self;
        let bit_offset = regs[raddr];
        let (spc_base_addr, val_base_addr, base_size) = if fv {
            let slots: &[Bytecode; 5] = (&code[*pc as usize..*pc as usize + 5])
                .try_into()
                .expect("Unable to grab expected slots");
            let spc_base_addr = ((slots[1].0 as u64) << 32) | (slots[0].0 as u64);
            let val_base_addr = ((slots[3].0 as u64) << 32) | (slots[2].0 as u64);
            let base_size = VectorSize::new(slots[4].0).expect("Expected non-zero size");
            *pc += 5;
            (spc_base_addr, val_base_addr, base_size)
        } else {
            let slots: &[Bytecode; 3] = (&code[*pc as usize..*pc as usize + 3])
                .try_into()
                .expect("Unable to grab expected slots");
            let base_addr = ((slots[1].0 as u64) << 32) | (slots[0].0 as u64);
            let base_size = VectorSize::new(slots[2].0).expect("Expected non-zero size");
            *pc += 3;
            (base_addr, base_addr, base_size)
        };

        let size = size.get(pc, code);

        let src_num_words = size.get().div_ceil(64) as usize;
        let fv_src_num_words = if fv { src_num_words * 2 } else { src_num_words };

        cldctx.heap_scratch.clear();
        cldctx
            .heap_scratch
            .resize(fv_src_num_words + src_num_words, 0);

        let (scratch_src, scratch_update_mask) = cldctx.heap_scratch.split_at_mut(fv_src_num_words);

        scratch_src.copy_from_slice(state.heap.get_u64_slice(
            HeapOffset {
                bit_offset: regs[rs] as usize,
            },
            fv_src_num_words,
        ));

        let dst = state
            .heap
            .get_mut_u64_slice(HeapOffset { bit_offset: 0usize }, state.heap.0.len());
        set_with_mask(
            scratch_update_mask,
            dst,
            &scratch_src[..src_num_words],
            bit_offset,
            size,
            spc_base_addr,
            base_size,
        );
        if fv {
            set_with_mask(
                scratch_update_mask,
                dst,
                &scratch_src[src_num_words..],
                bit_offset + (val_base_addr - spc_base_addr),
                size,
                val_base_addr,
                base_size,
            );
        }
        state
            .heap
            .get_mut_u64_slice(
                HeapOffset {
                    bit_offset: regs[rd] as usize,
                },
                src_num_words,
            )
            .copy_from_slice(scratch_update_mask);
    }
}

impl BytecodeEncoder {
    pub fn set_unaligned(&mut self, rd: Reg, rs: Reg, at: u64, size: SixBitSize) {
        if at < (1u64 << (10 + 32)) {
            let imm10 = (at & 0x3FF) as u16;
            let imm42_11 = (at >> 10) as u32;
            self.data.push(
                SetUnaligned {
                    rd,
                    rs,
                    size,
                    imm10,
                }
                .encode(),
            );
            self.data.push(Bytecode(imm42_11));
            return;
        }

        self.load_u64(rd, at);
        self.rel_set_unaligned(rd, rs, rd, size, at, size.into());
    }
    pub fn rel_set_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        raddr: Reg,
        size: SixBitSize,
        base_addr: u64,
        base_size: VectorSize,
    ) {
        let inline_base_size = InlineNBitSize::new(base_size);
        self.data.push(
            RelSetUnaligned {
                rd,
                rs,
                raddr,
                size,
                base_size: inline_base_size,
            }
            .encode(),
        );
        self.data.push(Bytecode((base_addr & 0xFFFF_FFFF) as u32));
        self.data.push(Bytecode((base_addr >> 32) as u32));
        if inline_base_size.0.is_none() {
            self.data.push(Bytecode(base_size.get()));
        }
    }
    pub fn tv_set_heap_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        raddr: Reg,
        size: VectorSize,
        base_addr: u64,
        base_size: VectorSize,
    ) {
        let inline_size = InlineNBitSize::new(size);
        self.data.push(
            SetHeapUnaligned {
                rd,
                rs,
                raddr,
                fv: false,
                size: inline_size,
            }
            .encode(),
        );
        self.data.push(Bytecode((base_addr & 0xFFFF_FFFF) as u32));
        self.data.push(Bytecode((base_addr >> 32) as u32));
        self.data.push(Bytecode(base_size.get()));
        if inline_size.0.is_none() {
            self.data.push(Bytecode(size.get()));
        }
    }
    pub fn fv_set_heap_unaligned(
        &mut self,
        rd: Reg,
        rs: Reg,
        raddr: Reg,
        size: VectorSize,
        spc_base_addr: u64,
        val_base_addr: u64,
        base_size: VectorSize,
    ) {
        let inline_size = InlineNBitSize::new(size);
        self.data.push(
            SetHeapUnaligned {
                rd,
                rs,
                raddr,
                fv: true,
                size: inline_size,
            }
            .encode(),
        );
        self.data
            .push(Bytecode((spc_base_addr & 0xFFFF_FFFF) as u32));
        self.data.push(Bytecode((spc_base_addr >> 32) as u32));
        self.data
            .push(Bytecode((val_base_addr & 0xFFFF_FFFF) as u32));
        self.data.push(Bytecode((val_base_addr >> 32) as u32));
        self.data.push(Bytecode(base_size.get()));
        if inline_size.0.is_none() {
            self.data.push(Bytecode(size.get()));
        }
    }
}
