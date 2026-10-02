use std::cmp;
use std::fmt;
use std::ops::RangeInclusive;

use vogls_bits::set_subslice::set_with_mask;
use vogls_codegen::{HeapAlignment, HeapOffset, SixBitSize};
use vogls_ir::{LogicMode, SCALAR_VSIZE, VSIZE_32, VSIZE_64, VectorSize};
use vogls_runtime::RuntimeState;

use crate::InlineNBitSize;
use crate::reg::{Reg, RegInfo, Regs};
use crate::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineAddrOffset, Schedule, write_padded_mnemonic,
};

struct Set1 {
    rd: Reg,
    rs: Reg,
    imm16: u16,
}
struct Set {
    rd: Reg,
    rs: Reg,
    size: SixBitSize,
    imm10: u16,
}
struct SetRelative {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    offset: InlineAddrOffset<12>,
}

/// A register-relative set which reads its value from, and writes its update mask to, a register.
///
/// The size fits a [`SixBitSize`] and is encoded inline, which leaves no room for an address
/// offset. The address is taken from `roff` as-is.
struct SetRegRelative {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    size: SixBitSize,
    // 6 bits unused
}
/// A set at a constant address which reads its value from, and writes its update mask to, the
/// heap.
///
/// The address is known at encoding time, so it needs neither an address register nor a runtime
/// bounds check.
struct SetHeap {
    rd: Reg,
    rs: Reg,
    src_size: InlineNBitSize<16>,
}
/// A register-relative set which reads its value from, and writes its update mask to, the heap.
///
/// The address comes from `roff` and is bounds checked against a range in the additional slots.
struct SetHeapRelative {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    offset: InlineAddrOffset<12>,
}

/// Set 1 two-valued logic heap bit and trigger all corresponding signal updates.
pub struct TvSet1(Set1);
/// Set 1 four-valued logic heap bit and trigger all corresponding signal updates.
pub struct FvSet1(Set1);
/// Set 1 four-valued logic heap bit and trigger all corresponding signal updates.
///
/// The four-value special plane and value plane are separated by a certain spread.
pub struct FvSet1Spread(Set1);

pub struct TvSet1Relative(SetRelative);
pub struct FvSet1Relative(SetRelative);

/// Set `size` two-valued logic heap bits at an aligned address and trigger all corresponding
/// signal updates.
pub struct TvSetAligned(Set);
/// Set `size` four-valued logic heap bits at an aligned address and trigger all corresponding
/// signal updates.
pub struct FvSetAligned(Set);
/// Set `size` two-valued logic heap bits at an unaligned address and trigger all corresponding
/// signal updates.
pub struct TvSetUnaligned(Set);
/// Set `size` four-valued logic heap bits at an unaligned address and trigger all corresponding
/// signal updates.
pub struct FvSetUnaligned(Set);

/// Set two-valued logic heap bits at a register-relative unaligned address and trigger all
/// corresponding signal updates.
///
/// The address is bounds checked against an inclusive range encoded in the instruction. An
/// out-of-range address is a no-op which writes an empty update mask.
pub struct TvSetRelative(SetRegRelative);
/// Set four-valued logic heap bits at a register-relative unaligned address and trigger all
/// corresponding signal updates.
///
/// The special plane and value plane are separated by a spread.
pub struct FvSetRelative(SetRegRelative);

/// Set two-valued logic heap bits wider than a register and trigger all corresponding signal
/// updates.
///
/// The write covers the signal exactly, so it needs no base range and never trims. Both the
/// source and the update mask live on the heap: `rs` holds the address of the source value and
/// `rd` the address the update mask is written to.
pub struct TvSetWholeHeap(SetHeap);
/// Set four-valued logic heap bits wider than a register and trigger all corresponding signal
/// updates.
///
/// The write covers the signal exactly. Both the source and the update mask live on the heap. A
/// base is still encoded, since the value plane sits a spread away from it.
pub struct FvSetWholeHeap(SetHeap);
/// Set two-valued logic heap bits wider than a register into part of a signal, and trigger all
/// corresponding signal updates.
pub struct TvSetPartialHeap(SetHeap);
/// Set four-valued logic heap bits wider than a register into part of a signal, and trigger all
/// corresponding signal updates.
///
/// The special plane and value plane are separated by a spread.
pub struct FvSetPartialHeap(SetHeap);
/// Set two-valued logic heap bits wider than a register at a register-relative address, and
/// trigger all corresponding signal updates.
///
/// The address is bounds checked against an inclusive range. Bits addressed outside the range are
/// not written and do not contribute to the update mask.
pub struct TvSetHeapRelative(SetHeapRelative);
/// Set four-valued logic heap bits wider than a register at a register-relative address, and
/// trigger all corresponding signal updates.
///
/// The special plane and value plane are separated by a spread. As with [`TvSetHeapRelative`],
/// the address is bounds checked.
pub struct FvSetHeapRelative(SetHeapRelative);

impl Set1 {
    #[inline(always)]
    fn extract(c: Bytecode, opcode: BytecodeOpcode) -> Self {
        debug_assert_eq!(c.opcode(), opcode as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            imm16: (v >> 16) as u16,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.imm16 as u32) << 16),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs, imm16 } = self;
        write!(f, "{rd}, {rs}, {imm16}")
    }
}

impl Set {
    #[inline(always)]
    fn extract(c: Bytecode, opcode: BytecodeOpcode) -> Self {
        debug_assert_eq!(c.opcode(), opcode as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
            imm10: (v >> 22) as u16,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
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
        write!(f, "{rd}, {rs}, {imm10}, |{size}|")
    }
}

impl SetRelative {
    #[inline(always)]
    fn extract(c: Bytecode, opcode: BytecodeOpcode) -> Self {
        debug_assert_eq!(c.opcode(), opcode as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            offset: InlineAddrOffset::new_shifted(v, 20),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.offset.encode() << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            offset,
        } = self;
        write!(f, "{rd}, {rs}, {roff}, {offset}")
    }
}

impl SetRegRelative {
    #[inline(always)]
    fn extract(c: Bytecode, opcode: BytecodeOpcode) -> Self {
        debug_assert_eq!(c.opcode(), opcode as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            size: SixBitSize::new_masked(v >> 20),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.size.encode() << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs, roff, size } = self;
        write!(f, "{rd}, {rs}, {roff}, |{size}|")
    }
}

impl SetHeap {
    #[inline(always)]
    fn extract(c: Bytecode, opcode: BytecodeOpcode) -> Self {
        debug_assert_eq!(c.opcode(), opcode as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            src_size: InlineNBitSize::new_masked(v >> 16),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.src_size.encode() << 16),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs, src_size } = self;
        write!(f, "{rd}, {rs}, {src_size}")
    }
}

impl SetHeapRelative {
    #[inline(always)]
    fn extract(c: Bytecode, opcode: BytecodeOpcode) -> Self {
        debug_assert_eq!(c.opcode(), opcode as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            offset: InlineAddrOffset::new_shifted(v, 20),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.offset.encode() << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            offset,
        } = self;
        write!(f, "{rd}, {rs}, {roff}, {offset}")
    }
}

/// Set `size` bits at an aligned `offset` in the heap.
///
/// Returns the mask of bits which changed.
#[inline(always)]
fn tv_set_aligned(heap: &mut [u64], offset: u64, value: u64, size: SixBitSize) -> u64 {
    debug_assert!(HeapAlignment::new(size.into(), LogicMode::TwoValue).is_aligned(offset));
    let mask = size.mask(u64::MAX);
    let word = &mut heap[(offset / 64) as usize];
    let boff = offset % 64;
    let prev_value = mask & (*word >> boff);
    *word &= !(mask << boff);
    *word |= value << boff;
    prev_value ^ value
}

/// Set `size` bits in both planes at an aligned `offset` in the heap.
///
/// The value plane is derived from `size`. Returns the OR sum of the changed bits in both planes.
#[inline(always)]
fn fv_set_aligned(
    heap: &mut [u64],
    offset: u64,
    spc: u64,
    val: u64,
    size: SixBitSize,
) -> (u64, u64) {
    debug_assert!(HeapAlignment::new(size.into(), LogicMode::TwoValue).is_aligned(offset));

    let spc_offset = offset;
    let val_offset = HeapAlignment::spc_offset_to_val_offset(size.into(), spc_offset);

    let mask = size.mask(u64::MAX);

    let spc_boff = offset % 64;
    let heap_spc_word = &mut heap[(spc_offset / 64) as usize];
    let prev_spc = mask & (*heap_spc_word >> spc_boff);
    *heap_spc_word &= !(mask << spc_boff);
    *heap_spc_word |= spc << spc_boff;

    let val_boff = val_offset % 64;
    let heap_val_word = &mut heap[(val_offset / 64) as usize];
    let prev_val = mask & (*heap_val_word >> val_boff);
    *heap_val_word &= !(mask << val_boff);
    *heap_val_word |= val << val_boff;

    (prev_spc ^ spc, prev_val ^ val)
}

/// Perform the part of a write of `size` bits at `offset` which falls within `lower..=upper`.
///
/// Bits addressed outside the range are not written and do not contribute to the returned update
/// mask. The mask is returned in the same bit positions as a full write would have used, so bit
/// `i` of the result still corresponds to bit `i` of `value`.
#[cold]
#[inline(never)]
fn set_unaligned_oob(
    heap: &mut [u64],
    offset: u64,
    value: u64,
    size: SixBitSize,
    lower: u64,
    upper: u64,
) -> u64 {
    let write_min = offset;
    let write_max = offset.saturating_add(size as u64 - 1);

    let trim_min = cmp::max(lower, write_min);
    let trim_max = cmp::min(upper, write_max);

    // The write may fall entirely outside of the range, in which case nothing is written.
    let Some(trim_size) = trim_max
        .checked_sub(trim_min)
        .and_then(|v| u8::try_from(v + 1).ok())
        .and_then(SixBitSize::new)
    else {
        return 0;
    };
    debug_assert!(trim_size as u64 <= size as u64);

    let min_shift = trim_min - offset;
    let trim_value = trim_size.mask(value >> min_shift);

    set_unaligned_inbounds(heap, trim_min, trim_value, trim_size) << min_shift
}

/// Set `size` bits at an unaligned `offset` in the heap.
///
/// Returns the mask of bits which changed.
///
/// # Invariants
/// - `value` should be premasked according to `size`.
/// - The full write should be in bounds of the heap.
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

/// Whether a write of `size` bits starting at `offset` fits entirely within the inclusive address
/// range `lower..=upper`.
///
/// Both ends need checking: [`set_unaligned_inbounds`] indexes two heap words for a write which
/// straddles a word boundary, so a write which merely *starts* in range can still run past the
/// end of the addressable region.
#[inline(always)]
fn is_offset_in_bounds(offset: u32, size: SixBitSize, lower: u32, upper: u32) -> bool {
    let end = offset.wrapping_add(size as u32 - 1);
    (offset >= lower) & (end <= upper) & (end >= offset)
}

#[inline(always)]
fn is_offset_in_bounds_vs(offset: u32, size: VectorSize, lower: u32, upper: u32) -> bool {
    let end = offset.wrapping_add(size.get() - 1);
    (offset >= lower) & (end <= upper) & (end >= offset)
}

impl BytecodeInstruction for TvSet1 {
    fn num_additional_slots(&self) -> u8 {
        1
    }

    fn extract(v: Bytecode) -> Self {
        Self(Set1::extract(v, BytecodeOpcode::TvSet1))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::TvSet1)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "tv.set1")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<crate::reg::RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
        ));
    }

    #[inline(always)]
    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        execute_set1::<false>(self.0, code, regs, pc, state, schedule, listeners, cldctx);
    }
}
impl BytecodeInstruction for FvSet1 {
    fn num_additional_slots(&self) -> u8 {
        1
    }

    fn extract(v: Bytecode) -> Self {
        Self(Set1::extract(v, BytecodeOpcode::FvSet1))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::FvSet1)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "fv.set1")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::FourValue,
            Some(SCALAR_VSIZE),
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<crate::reg::RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
        ));
    }

    #[inline(always)]
    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        execute_set1::<true>(self.0, code, regs, pc, state, schedule, listeners, cldctx);
    }
}
impl BytecodeInstruction for FvSet1Spread {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 1; // Base.
        slots += 1; // Spread.
        slots
    }

    fn extract(v: Bytecode) -> Self {
        Self(Set1::extract(v, BytecodeOpcode::FvSet1Spread))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::FvSet1Spread)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "fv.set1spread")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::FourValue,
            Some(SCALAR_VSIZE),
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<crate::reg::RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
        ));
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
        let Self(Set1 { rd, rs, imm16 }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = additional_slots[0].0 as u64;
        let spread = VectorSize::new(additional_slots[1].0).unwrap();
        let base = HeapAlignment::new(spread, LogicMode::FourValue).from_elem_offset(base);
        let bit_offset = imm16 as u64;

        let heap = state.heap.0.as_mut();
        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let val_offset = HeapAlignment::spc_offset_to_val_offset(spread, base);
        let spc_update = set_unaligned_inbounds(heap, base + bit_offset, spc, SixBitSize::N1);
        let val_update = set_unaligned_inbounds(heap, val_offset + bit_offset, val, SixBitSize::N1);
        let updated = (spc_update | val_update) != 0;

        regs[rd] = u64::from(updated);
    }
}
impl BytecodeInstruction for TvSet1Relative {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 2; // Base.
        slots += 1; // Lower bound.
        slots += 1; // Upper bound.
        slots
    }

    fn extract(v: Bytecode) -> Self {
        Self(SetRelative::extract(v, BytecodeOpcode::TvSet1Relative))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::TvSet1Relative)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "tv.set1rel")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.extend([
            RegInfo::register("rs", self.0.rs, LogicMode::TwoValue, Some(SCALAR_VSIZE)),
            RegInfo::register("roff", self.0.roff, LogicMode::TwoValue, Some(VSIZE_64)),
        ]);
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<crate::reg::RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
        ));
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
        let Self(SetRelative {
            rd,
            rs,
            roff,
            offset,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[0..]);
        let lower_bound = additional_slots[2].0;
        let upper_bound = additional_slots[3].0;
        let heap_offset = offset.get(regs[roff]) as u32;

        if !is_offset_in_bounds(heap_offset, SixBitSize::N1, lower_bound, upper_bound) {
            // @NOTE: A single bit is either fully in range or fully out, so there is nothing to
            // partially write here.
            std::hint::cold_path();
            regs[rd] = 0;
            return;
        }

        let heap = state.heap.0.as_mut();
        let value = regs[rs];
        let updated =
            set_unaligned_inbounds(heap, base + heap_offset as u64, value, SixBitSize::N1) != 0;

        regs[rd] = u64::from(updated);
    }
}
impl BytecodeInstruction for FvSet1Relative {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 2; // Base.
        slots += 1; // Lower bound.
        slots += 1; // Upper bound.
        slots += 1; // Spread.
        slots
    }

    fn extract(v: Bytecode) -> Self {
        Self(SetRelative::extract(v, BytecodeOpcode::FvSet1Relative))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::FvSet1Relative)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "fv.set1rel")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.extend([
            RegInfo::register("rs", self.0.rs, LogicMode::FourValue, Some(SCALAR_VSIZE)),
            RegInfo::register("roff", self.0.roff, LogicMode::TwoValue, Some(VSIZE_64)),
        ]);
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<crate::reg::RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(SCALAR_VSIZE),
        ));
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
        let Self(SetRelative {
            rd,
            rs,
            roff,
            offset,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let lower_bound = additional_slots[2].0;
        let upper_bound = additional_slots[3].0;
        let spread = additional_slots[4].unwrap_size();
        let bit_offset = offset.get(regs[roff]) as u32;

        if !is_offset_in_bounds(bit_offset, SixBitSize::N1, lower_bound, upper_bound) {
            std::hint::cold_path();
            regs[rd] = 0;
            return;
        }

        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let base_val = HeapAlignment::spc_offset_to_val_offset(spread, base);
        let heap = state.heap.0.as_mut();

        let spc_update =
            set_unaligned_inbounds(heap, base + bit_offset as u64, spc, SixBitSize::N1);
        let val_update =
            set_unaligned_inbounds(heap, base_val + bit_offset as u64, val, SixBitSize::N1);
        let updated = (spc_update | val_update) != 0;

        regs[rd] = u64::from(updated);
    }
}

#[inline(always)]
fn execute_set1<const FOUR_VALUE: bool>(
    args: Set1,
    code: &[Bytecode],
    regs: &mut Regs,
    pc: &mut u64,
    state: &mut RuntimeState,
    _schedule: &mut Schedule,
    _listeners: &mut BytecodeListeners,
    _cldctx: &mut ColdContext,
) {
    let Set1 { rd, rs, imm16 } = args;

    // @NOTE: We ensured the code buffer is padded with at least 16 elements.
    let additional_slots: &[Bytecode; 16] = code
        .get(*pc as usize..)
        .and_then(<[_]>::first_chunk)
        .expect("Should be padded");

    *pc += 1; // Address.

    let heap_offset = additional_slots[0].0;
    let heap_offset = ((heap_offset as u64) << 16) | (imm16 as u64);

    let heap = state.heap.0.as_mut();
    let updated = if FOUR_VALUE {
        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let (spc_update, val_update) = fv_set_aligned(heap, heap_offset, spc, val, SixBitSize::N1);
        (spc_update | val_update) != 0
    } else {
        let src = regs[rs];
        tv_set_aligned(heap, heap_offset, src, SixBitSize::N1) != 0
    };

    regs[rd] = u64::from(updated);
}

impl BytecodeInstruction for TvSetAligned {
    fn num_additional_slots(&self) -> u8 {
        1
    }

    fn extract(v: Bytecode) -> Self {
        Self(Set::extract(v, BytecodeOpcode::TvSetAligned))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::TvSetAligned)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "tv.set_aligned")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
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
        let Self(Set {
            rd,
            rs,
            size,
            imm10,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = additional_slots[0].0 as u64;
        let base = HeapAlignment::new(size.into(), LogicMode::TwoValue).from_elem_offset(base);
        let offset = base + imm10 as u64;

        let heap = state.heap.0.as_mut();
        let value = regs[rs];
        let update_mask = tv_set_aligned(heap, offset, value, size);

        regs[rd] = update_mask;
    }
}

impl BytecodeInstruction for FvSetAligned {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 1; // Base.
        slots += 1; // Spread.
        slots
    }

    fn extract(v: Bytecode) -> Self {
        Self(Set::extract(v, BytecodeOpcode::FvSetAligned))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::FvSetAligned)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "fv.set_aligned")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::FourValue,
            Some(self.0.size.into()),
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
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
        let Self(Set {
            rd,
            rs,
            size,
            imm10,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = additional_slots[0].0 as u64;
        let spread = VectorSize::new(additional_slots[1].0).unwrap();
        let base = HeapAlignment::new(spread, LogicMode::FourValue).from_elem_offset(base);
        let offset = base + imm10 as u64;
        let val_offset = HeapAlignment::spc_offset_to_val_offset(spread, offset);

        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let heap = state.heap.0.as_mut();

        let spc_update = set_unaligned_inbounds(heap, offset, spc, size);
        let val_update = set_unaligned_inbounds(heap, val_offset, val, size);
        let update_mask = spc_update | val_update;

        regs[rd] = update_mask;
    }
}

impl BytecodeInstruction for TvSetUnaligned {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 2; // Base.
        slots
    }

    fn extract(v: Bytecode) -> Self {
        Self(Set::extract(v, BytecodeOpcode::TvSetUnaligned))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::TvSetUnaligned)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "tv.set_unaligned")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
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
        let Self(Set {
            rd,
            rs,
            size,
            imm10,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let offset = base + imm10 as u64;

        let heap = state.heap.0.as_mut();
        let value = size.mask(regs[rs]);
        let update_mask = set_unaligned_inbounds(heap, offset, value, size);
        regs[rd] = update_mask;
    }
}

impl BytecodeInstruction for FvSetUnaligned {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 2; // Base.
        slots += 1; // Spread.
        slots
    }

    fn extract(v: Bytecode) -> Self {
        Self(Set::extract(v, BytecodeOpcode::FvSetUnaligned))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::FvSetUnaligned)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "fv.set_unaligned")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rs",
            self.0.rs,
            LogicMode::FourValue,
            Some(self.0.size.into()),
        ));
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            Some(self.0.size.into()),
        ));
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
        let Self(Set {
            rd,
            rs,
            size,
            imm10,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let spread = VectorSize::new(additional_slots[2].0).expect("Expected non-zero size");
        let offset = base + imm10 as u64;
        let val_offset = HeapAlignment::spc_offset_to_val_offset(spread, offset);

        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let heap = state.heap.0.as_mut();

        let spc_update = set_unaligned_inbounds(heap, offset, size.mask(spc), size);
        let val_update = set_unaligned_inbounds(heap, val_offset, size.mask(val), size);
        let update_mask = spc_update | val_update;

        regs[rd] = update_mask;
    }
}

impl BytecodeInstruction for TvSetRelative {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 2; // Base.
        slots += 1; // Lower bound.
        slots += 1; // Upper bound.
        slots
    }

    fn extract(v: Bytecode) -> Self {
        Self(SetRegRelative::extract(v, BytecodeOpcode::TvSetRelative))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::TvSetRelative)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "tv.setrel")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.extend([
            RegInfo::register("rs", self.0.rs, LogicMode::TwoValue, None),
            RegInfo::register("roff", self.0.roff, LogicMode::TwoValue, Some(VSIZE_64)),
        ]);
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            None,
        ));
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
        let Self(SetRegRelative { rd, rs, roff, size }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");
        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let lower_bound = additional_slots[2].0;
        let upper_bound = additional_slots[3].0;
        let bit_offset = regs[roff] as u32;

        let offset = base + bit_offset as u64;
        let heap = state.heap.0.as_mut();
        // @NOTE: `set_unaligned_inbounds` expects the value to be premasked to `size`.
        let value = size.mask(regs[rs]);

        let update_mask = if is_offset_in_bounds(bit_offset, size, lower_bound, upper_bound) {
            set_unaligned_inbounds(heap, offset, value, size)
        } else {
            // @NOTE: A relative write is expected to land in bounds. A partially or fully
            // out-of-range index is rare, so keep it off the hot path. The in-range part of the
            // write is still performed.
            std::hint::cold_path();
            set_unaligned_oob(
                heap,
                offset,
                value,
                size,
                base + lower_bound as u64,
                base + upper_bound as u64,
            )
        };

        regs[rd] = update_mask;
    }
}

impl BytecodeInstruction for FvSetRelative {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 2; // Base.
        slots += 1; // Lower bound.
        slots += 1; // Upper bound.
        slots += 1; // Spread.
        slots
    }

    fn extract(v: Bytecode) -> Self {
        Self(SetRegRelative::extract(v, BytecodeOpcode::FvSetRelative))
    }

    fn encode(&self) -> Bytecode {
        self.0.encode(BytecodeOpcode::FvSetRelative)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "fv.setrel")?;
        self.0.fmt(f)
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.extend([
            RegInfo::register("rs", self.0.rs, LogicMode::FourValue, None),
            RegInfo::register("roff", self.0.roff, LogicMode::TwoValue, Some(VSIZE_64)),
        ]);
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.push(RegInfo::register(
            "rd",
            self.0.rd,
            LogicMode::TwoValue,
            None,
        ));
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
        let Self(SetRegRelative { rd, rs, roff, size }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let lower_bound = additional_slots[2].0;
        let upper_bound = additional_slots[3].0;
        let spread = additional_slots[4].unwrap_size();
        let bit_offset = regs[roff] as u32;

        let base_val = HeapAlignment::spc_offset_to_val_offset(spread, base);
        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let heap = state.heap.0.as_mut();

        // @NOTE: `set_unaligned_inbounds` expects the value to be premasked to `size`.
        let (spc, val) = (size.mask(spc), size.mask(val));
        let offset = base + bit_offset as u64;
        let val_offset = base_val + bit_offset as u64;

        // @NOTE: Both planes sit at the same relative position within their own plane, so one
        // check covers both.
        let (spc_update, val_update) =
            if is_offset_in_bounds(bit_offset, size, lower_bound, upper_bound) {
                (
                    set_unaligned_inbounds(heap, offset, spc, size),
                    set_unaligned_inbounds(heap, val_offset, val, size),
                )
            } else {
                // @NOTE: A relative write is expected to land in bounds. A partially or fully
                // out-of-range index is rare, so keep it off the hot path. The in-range part of the
                // write is still performed.
                std::hint::cold_path();
                let val_lower =
                    HeapAlignment::spc_offset_to_val_offset(spread, base + lower_bound as u64);
                let val_upper =
                    HeapAlignment::spc_offset_to_val_offset(spread, base + upper_bound as u64);
                (
                    set_unaligned_oob(
                        heap,
                        offset,
                        spc,
                        size,
                        base + lower_bound as u64,
                        base + upper_bound as u64,
                    ),
                    set_unaligned_oob(heap, val_offset, val, size, val_lower, val_upper),
                )
            };
        let update_mask = spc_update | val_update;
        regs[rd] = update_mask;
    }
}

/// Perform a heap-to-heap set of `size` bits and write the update mask back onto the heap.
///
/// The source lives at the address in `rs` and the update mask is written to the address in `rd`
/// if the [`SetFlags::WRITE_MASK`] flag is set. For four-value, both planes are written with the
/// value plane a `spread` away from the special plane, and the mask is the OR sum of both.
///
/// Only bits within the inclusive `clip` range of `base` are written. Any part of the write outside
/// of `clip` is a no-op and leaves its update mask bits zero.
#[inline(always)]
fn set_heap_wide<const FOUR_VALUE: bool>(
    rd: Reg,
    rs: Reg,
    offset: u32,
    size: VectorSize,
    base: u64,
    base_size: VectorSize,
    base_spread: VectorSize,
    clip: RangeInclusive<u32>,
    regs: &Regs,
    state: &mut RuntimeState,
    cldctx: &mut ColdContext,
) {
    let src_num_words = size.get().div_ceil(64) as usize;
    let base_num_words = base_size.get().div_ceil(64) as usize;
    let num_planes = if FOUR_VALUE { 2 } else { 1 };

    let (clip_start, clip_end) = (*clip.start(), *clip.end());
    debug_assert!(clip_start <= clip_end);
    let clip_size = VectorSize::new((clip_end - clip_start).saturating_add(1)).unwrap();

    cldctx.heap_scratch.clear();
    cldctx
        .heap_scratch
        .resize(src_num_words * (num_planes + 1), 0);
    let (scratch_src, update_mask) = cldctx.heap_scratch.split_at_mut(src_num_words * num_planes);

    scratch_src.copy_from_slice(state.heap.get_u64_slice(
        HeapOffset {
            bit_offset: regs[rs] as usize,
        },
        src_num_words * num_planes,
    ));

    let dst = state.heap.get_mut_u64_slice(
        HeapOffset {
            bit_offset: base as usize,
        },
        base_num_words * num_planes,
    );

    set_with_mask(
        update_mask,
        dst,
        &scratch_src[..src_num_words],
        offset as u64,
        size,
        clip_start as u64,
        clip_size,
    );
    if FOUR_VALUE {
        let val_offset = HeapAlignment::spc_offset_to_val_offset(base_spread, offset as u64);
        let val_clip_start =
            HeapAlignment::spc_offset_to_val_offset(base_spread, clip_start as u64);
        set_with_mask(
            update_mask,
            dst,
            &scratch_src[src_num_words..],
            val_offset,
            size,
            val_clip_start,
            clip_size,
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
        .copy_from_slice(update_mask);
}

macro_rules! impl_set_heap {
    ($name:ident, $mnemonic:literal, $four_value:literal, $partial:literal) => {
        impl BytecodeInstruction for $name {
            #[inline(always)]
            fn num_additional_slots(&self) -> u8 {
                let mut slots = 0;
                slots += 2; // Base.
                if $partial {
                    slots += 1; // Base Size.
                    slots += 1; // Offset.
                }
                if !self.0.src_size.is_inline() {
                    slots += 1; // Source Size.
                }
                slots
            }

            fn extract(v: Bytecode) -> Self {
                Self(SetHeap::extract(v, BytecodeOpcode::$name))
            }

            fn encode(&self) -> Bytecode {
                self.0.encode(BytecodeOpcode::$name)
            }

            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write_padded_mnemonic(f, $mnemonic)?;
                self.0.fmt(f)
            }

            fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
                let size = code[pc as usize + 3].unwrap_size();
                operands.push(RegInfo::heap(
                    "rs",
                    self.0.rs,
                    if $four_value {
                        LogicMode::FourValue
                    } else {
                        LogicMode::TwoValue
                    },
                    size,
                ));
            }

            fn dest_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
                let size = code[pc as usize + 3].unwrap_size();
                operands.push(RegInfo::heap("rd", self.0.rd, LogicMode::TwoValue, size));
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
                let Self(SetHeap { rd, rs, src_size }) = self;

                // @NOTE: We ensured the code buffer is padded with at least 16 elements.
                let additional_slots: &[Bytecode; 16] = code
                    .get(*pc as usize..)
                    .and_then(<[_]>::first_chunk)
                    .expect("Should be padded");

                *pc += self.num_additional_slots() as u64;

                let base = decode_bytecode_u64(&additional_slots[..]);

                let src_size = src_size.get(&mut if $partial { 4 } else { 2 }, additional_slots);
                let base_size = if $partial {
                    additional_slots[2].unwrap_size()
                } else {
                    src_size
                };
                let offset = if $partial { additional_slots[3].0 } else { 0 };

                set_heap_wide::<$four_value>(
                    rd,
                    rs,
                    offset,
                    src_size,
                    base,
                    base_size,
                    base_size,
                    0..=base_size.get() - 1,
                    regs,
                    state,
                    cldctx,
                );
            }
        }
    };
}

macro_rules! impl_set_heap_relative {
    ($name:ident, $mnemonic:literal, $four_value:literal) => {
        impl BytecodeInstruction for $name {
            fn num_additional_slots(&self) -> u8 {
                let mut slots = 0;
                slots += 2; // Base.
                slots += 1; // Lower bound.
                slots += 1; // Upper bound.
                slots += 1; // Source Size.
                slots += 1; // Base Size.
                slots
            }

            fn extract(v: Bytecode) -> Self {
                Self(SetHeapRelative::extract(v, BytecodeOpcode::$name))
            }

            fn encode(&self) -> Bytecode {
                self.0.encode(BytecodeOpcode::$name)
            }

            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write_padded_mnemonic(f, $mnemonic)?;
                self.0.fmt(f)
            }

            fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
                let size = code[pc as usize + 6].unwrap_size();
                operands.push(RegInfo::heap(
                    "rs",
                    self.0.rs,
                    if $four_value {
                        LogicMode::FourValue
                    } else {
                        LogicMode::TwoValue
                    },
                    size,
                ));
                operands.push(RegInfo::register(
                    "roff",
                    self.0.roff,
                    LogicMode::TwoValue,
                    Some(VSIZE_32),
                ));
            }

            fn dest_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
                let size = code[pc as usize + 6].unwrap_size();
                operands.push(RegInfo::heap("rd", self.0.rd, LogicMode::TwoValue, size));
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
                let Self(SetHeapRelative {
                    rd,
                    rs,
                    roff,
                    offset,
                }) = self;

                // @NOTE: We ensured the code buffer is padded with at least 16 elements.
                let additional_slots: &[Bytecode; 16] = code
                    .get(*pc as usize..)
                    .and_then(<[_]>::first_chunk)
                    .expect("Should be padded");

                *pc += self.num_additional_slots() as u64;

                let base = decode_bytecode_u64(&additional_slots[..]);
                let lower_bound = additional_slots[2].0;
                let upper_bound = additional_slots[3].0;
                let src_size = additional_slots[4].unwrap_size();
                let base_size = additional_slots[5].unwrap_size();
                let offset = offset.get(regs[roff]) as u32;

                // @NOTE: Clipping the write to the bounds is the bounds check. A partially or
                // fully out-of-range write only performs its in-range part.
                set_heap_wide::<$four_value>(
                    rd,
                    rs,
                    offset,
                    src_size,
                    base,
                    base_size,
                    base_size,
                    lower_bound..=upper_bound,
                    regs,
                    state,
                    cldctx,
                );
            }
        }
    };
}

impl_set_heap!(TvSetWholeHeap, "tv.set_wholeheap", false, false);
impl_set_heap!(FvSetWholeHeap, "fv.set_wholeheap", true, false);
impl_set_heap!(TvSetPartialHeap, "tv.set_partialheap", false, true);
impl_set_heap!(FvSetPartialHeap, "fv.set_partialheap", true, true);
impl_set_heap_relative!(TvSetHeapRelative, "tv.set_heaprel", false);
impl_set_heap_relative!(FvSetHeapRelative, "fv.set_heaprel", true);

#[inline(always)]
pub fn encode_bytecode_u64(bce: &mut BytecodeEncoder, v: u64) {
    bce.data.push(Bytecode((v & 0xFFFF_FFFF) as u32));
    bce.data.push(Bytecode((v >> 32) as u32));
}

#[inline(always)]
pub fn decode_bytecode_u64(slots: &[Bytecode]) -> u64 {
    u64::from(slots[0].0) | (u64::from(slots[1].0) << 32)
}

impl BytecodeEncoder {
    fn set1(&mut self, rd: Reg, rs: Reg, offset: u64, f: impl FnOnce(Set1) -> Bytecode) {
        debug_assert!(
            offset < (1 << 48),
            "Caller must fall back on a wide partial"
        );
        self.data.push(f(Set1 {
            rd,
            rs,
            imm16: (offset & 0xFFFF) as u16,
        }));
        self.data.push(Bytecode((offset >> 16) as u32));
    }

    pub fn tv_set1(&mut self, rd: Reg, rs: Reg, offset: u64, scratch: Reg) {
        if offset >= 1u64 << 48 {
            self.load_u64(scratch, 0);
            self.tv_set1rel(rd, rs, scratch, offset, InlineAddrOffset::ZERO, 0..=0);
            return;
        }

        self.set1(rd, rs, offset, |args| TvSet1(args).encode());
    }
    /// Encode a four-value single-bit set.
    ///
    /// Only ever used where the signal *is* the bit, so there is no partial to carry.
    pub fn fv_set1(&mut self, rd: Reg, rs: Reg, offset: u64, scratch: Reg) {
        if offset >= 1u64 << 48 {
            self.load_u64(scratch, 0);
            self.fv_set1rel(
                rd,
                rs,
                scratch,
                offset,
                InlineAddrOffset::ZERO,
                0..=0,
                SCALAR_VSIZE,
            );
            return;
        }

        self.set1(rd, rs, offset, |args| FvSet1(args).encode());
    }
    pub fn fv_set1spread(
        &mut self,
        rd: Reg,
        rs: Reg,
        base: u64,
        offset: u64,
        spread: VectorSize,
        scratch: Reg,
    ) {
        let alignment = HeapAlignment::new(spread, LogicMode::FourValue);
        let base_elem = alignment.to_elem_offset(base);

        if base_elem >= 1u64 << 32 || offset >= 1u64 << 12 {
            self.load_u64(scratch, 0);
            let (roff, offset) = InlineAddrOffset::new(offset as i64, self, scratch, scratch);
            self.fv_set1rel(rd, rs, roff, base, offset, 0..=0, spread);
            return;
        }

        let base = base_elem;
        self.data.push(
            FvSet1Spread(Set1 {
                rd,
                rs,
                imm16: (offset & 0xFFFF) as u16,
            })
            .encode(),
        );
        self.data.push(Bytecode((base & 0xFFFF_FFFF) as u32));
        self.data.push(Bytecode(spread.get()));
    }

    pub fn tv_set1rel(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        base: u64,
        offset: InlineAddrOffset<12>,
        range: RangeInclusive<u32>,
    ) {
        self.data.push(
            TvSet1Relative(SetRelative {
                rd,
                rs,
                roff,
                offset,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(*range.start()));
        self.data.push(Bytecode(*range.end()));
    }
    pub fn fv_set1rel(
        &mut self,
        rd: Reg,
        rs: Reg,
        roff: Reg,
        base: u64,
        offset: InlineAddrOffset<12>,
        range: RangeInclusive<u32>,
        spread: VectorSize,
    ) {
        self.data.push(
            FvSet1Relative(SetRelative {
                rd,
                rs,
                roff,
                offset,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(*range.start()));
        self.data.push(Bytecode(*range.end()));
        self.data.push(Bytecode(spread.get()));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn tv_set_aligned(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        at: u64,
        size: SixBitSize,
        partial: u64,
        scratch: Reg,
    ) {
        let alignment = HeapAlignment::new(size.into(), LogicMode::TwoValue);
        let base = alignment.next_aligned(at.saturating_sub(0x3F));
        let base_elem = alignment.to_elem_offset(base);

        if base_elem >= 1u64 << 32 || at - base >= 1u64 << 6 {
            self.load_u64(scratch, partial);
            self.tv_setrel(
                rd,
                rs,
                scratch,
                at.saturating_sub(partial),
                0..=u32::MAX,
                size,
            );
            return;
        }

        self.data.push(
            TvSetAligned(Set {
                rd: rd.unwrap_or(rs),
                rs,
                size,
                imm10: (at - base) as u16,
            })
            .encode(),
        );
        self.data.push(Bytecode(base_elem as u32));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fv_set_aligned(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        at: u64,
        size: SixBitSize,
        spread: VectorSize,
        partial: u64,
        scratch: Reg,
    ) {
        let alignment = HeapAlignment::new(spread, LogicMode::FourValue);
        let base = alignment.next_aligned(at.saturating_sub(0x3F));
        let base_elem = alignment.to_elem_offset(base);

        if base_elem >= 1u64 << 32 || at - base >= 1u64 << 6 {
            self.load_u64(scratch, partial);
            self.fv_setrel(
                rd,
                rs,
                scratch,
                at.saturating_sub(partial),
                0..=u32::MAX,
                size,
                spread,
            );
            return;
        }

        self.data.push(
            FvSetAligned(Set {
                rd: rd.unwrap_or(rs),
                rs,
                size,
                imm10: (at - base) as u16,
            })
            .encode(),
        );
        self.data.push(Bytecode(base_elem as u32));
        self.data.push(Bytecode(spread.get()));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn tv_set_unaligned(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        size: SixBitSize,
        base: u64,
        partial: u64,
        scratch: Reg,
    ) {
        if partial >= 1u64 << 6 {
            self.load_u64(scratch, partial);
            self.tv_setrel(rd, rs, scratch, base, 0..=u32::MAX, size);
            return;
        }

        self.data.push(
            TvSetUnaligned(Set {
                rd: rd.unwrap_or(rs),
                rs,
                size,
                imm10: partial as u16,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fv_set_unaligned(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        size: SixBitSize,
        base: u64,
        spread: VectorSize,
        partial: u64,
        scratch: Reg,
    ) {
        if partial >= 1u64 << 10 {
            self.load_u64(scratch, partial);
            self.fv_setrel(rd, rs, scratch, base, 0..=u32::MAX, size, spread);
            return;
        }

        self.data.push(
            FvSetUnaligned(Set {
                rd: rd.unwrap_or(rs),
                rs,
                size,
                imm10: partial as u16,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(spread.get()));
    }

    #[allow(clippy::too_many_arguments)]
    /// Encode a two-value register-relative set.
    ///
    pub fn tv_setrel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        base: u64,
        range: RangeInclusive<u32>,
        size: SixBitSize,
    ) {
        self.data.push(
            TvSetRelative(SetRegRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                size,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(*range.start()));
        self.data.push(Bytecode(*range.end()));
    }

    #[allow(clippy::too_many_arguments)]
    /// Encode a four-value register-relative set.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn fv_setrel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        base: u64,
        range: RangeInclusive<u32>,
        size: SixBitSize,
        spread: VectorSize,
    ) {
        self.data.push(
            FvSetRelative(SetRegRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                size,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(*range.start()));
        self.data.push(Bytecode(*range.end()));
        self.data.push(Bytecode(spread.get()));
    }

    /// Encode a two-value heap set which covers the whole signal.
    ///
    /// The write is exactly the signal, so no base range is encoded.
    pub fn tv_set_whole_heap(&mut self, rd: Reg, rs: Reg, base: u64, size: VectorSize) {
        let inline_size = InlineNBitSize::new(size);
        self.data.push(
            TvSetWholeHeap(SetHeap {
                rd,
                rs,
                src_size: inline_size,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        if !inline_size.is_inline() {
            self.data.push(Bytecode(size.get()));
        }
    }

    /// Encode a four-value heap set which covers the whole signal.
    ///
    /// A base is still encoded, since the value plane sits a `spread` away from it.
    #[allow(clippy::too_many_arguments)]
    pub fn fv_set_whole_heap(&mut self, rd: Reg, rs: Reg, base: u64, size: VectorSize) {
        let inline_size = InlineNBitSize::new(size);
        self.data.push(
            FvSetWholeHeap(SetHeap {
                rd,
                rs,
                src_size: inline_size,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        if !inline_size.is_inline() {
            self.data.push(Bytecode(size.get()));
        }
    }

    /// Encode a two-value heap set into part of a signal.
    ///
    /// Bits outside of `base..base + base_size` are not written.
    #[allow(clippy::too_many_arguments)]
    pub fn tv_set_partial_heap(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        base: u64,
        size: VectorSize,
        base_size: VectorSize,
        partial: u32,
    ) {
        let inline_size = InlineNBitSize::new(size);
        self.data.push(
            TvSetPartialHeap(SetHeap {
                rd: rd.unwrap_or(rs),
                rs,
                src_size: inline_size,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(base_size.get()));
        self.data.push(Bytecode(partial));
        if !inline_size.is_inline() {
            self.data.push(Bytecode(size.get()));
        }
    }

    /// Encode a four-value heap set into part of a signal.
    #[allow(clippy::too_many_arguments)]
    pub fn fv_set_partial_heap(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        base: u64,
        size: VectorSize,
        base_size: VectorSize,
        partial: u32,
    ) {
        let inline_size = InlineNBitSize::new(size);
        self.data.push(
            FvSetPartialHeap(SetHeap {
                rd: rd.unwrap_or(rs),
                rs,
                src_size: inline_size,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(base_size.get()));
        self.data.push(Bytecode(partial));
        if !inline_size.is_inline() {
            self.data.push(Bytecode(size.get()));
        }
    }

    /// Encode a two-value heap set at a register-relative address.
    #[allow(clippy::too_many_arguments)]
    pub fn tv_set_heaprel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        offset: InlineAddrOffset<12>,
        range: RangeInclusive<u32>,
        base: u64,
        size: VectorSize,
        base_size: VectorSize,
    ) {
        self.data.push(
            TvSetHeapRelative(SetHeapRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                offset,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(*range.start()));
        self.data.push(Bytecode(*range.end()));
        self.data.push(Bytecode(size.get()));
        self.data.push(Bytecode(base_size.get()));
    }

    /// Encode a four-value heap set at a register-relative address.
    #[allow(clippy::too_many_arguments)]
    pub fn fv_set_heaprel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        offset: InlineAddrOffset<12>,
        range: RangeInclusive<u32>,
        base: u64,
        size: VectorSize,
        base_size: VectorSize,
    ) {
        self.data.push(
            FvSetHeapRelative(SetHeapRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                offset,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(*range.start()));
        self.data.push(Bytecode(*range.end()));
        self.data.push(Bytecode(size.get()));
        self.data.push(Bytecode(base_size.get()));
    }
}
