use std::cmp;
use std::fmt::{self, Write};
use std::ops::RangeInclusive;

use vogls_bits::set_subslice::set_with_mask;
use vogls_codegen::{HeapAlignment, HeapOffset, SixBitSize};
use vogls_ir::{LogicMode, SCALAR_VSIZE, VSIZE_64, VectorSize};
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_utils::TableKey;

use crate::reg::{Reg, RegInfo, Regs};
use crate::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineAddrOffset, Schedule, write_padded_mnemonic,
};

/// Flags that define set instruction semantics.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct SetFlags(u8);

impl SetFlags {
    const EMPTY: Self = Self(0b0u8);

    /// Write the mask of which source bits let to updates into the `rd`. Otherwise, `rd` is
    /// untouched.
    const WRITE_MASK: Self = Self(0b0001u8);

    /// Watch indices are encoded and need to be armed if an update is required.
    const WATCH: Self = Self(0b0010u8);

    /// The last update time needs to be set.
    const LAST_UPDATE_TIME: Self = Self(0b0100u8);

    /// Plugins need to be poked for this signal.
    const PLUGIN_POKE: Self = Self(0b1000u8);

    #[inline(always)]
    pub fn encode(self) -> u32 {
        self.0.into()
    }

    #[inline(always)]
    pub fn new_masked(v: u32) -> Self {
        Self((v & 0xF) as u8)
    }

    #[inline(always)]
    fn contains(&self, other: SetFlags) -> bool {
        self.0 & other.0 == other.0
    }

    fn set(&mut self, flags: SetFlags, set: bool) {
        if set {
            self.0 |= flags.0;
        } else {
            self.0 &= !flags.0;
        }
    }

    #[inline(always)]
    fn num_additional_slots(self) -> u8 {
        let mut x = self.0;
        x &= !Self::WRITE_MASK.0;
        (x.count_ones() * 2) as u8
    }

    fn fmt(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('[')?;
        if self.contains(Self::WRITE_MASK) {
            f.write_char('M')?;
        }
        if self.contains(Self::LAST_UPDATE_TIME) {
            f.write_char('L')?;
        }
        if self.contains(Self::WATCH) {
            f.write_char('W')?;
        }
        if self.contains(Self::PLUGIN_POKE) {
            f.write_char('P')?;
        }
        f.write_char(']')?;
        Ok(())
    }
}

struct Set1 {
    rd: Reg,
    rs: Reg,
    flags: SetFlags,
    imm12: u16,
}
struct Set {
    rd: Reg,
    rs: Reg,
    flags: SetFlags,
    size: SixBitSize,
    imm6: u8,
}
struct SetRelative {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    flags: SetFlags,
    offset: InlineAddrOffset<8>,
}

/// A register-relative set which reads its value from, and writes its update mask to, a register.
///
/// The size fits a [`SixBitSize`] and is encoded inline, which leaves no room for an address
/// offset. The address is taken from `roff` as-is.
struct SetRegRelative {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    flags: SetFlags,
    /// Whether the write addresses something other than a signal, such as the stack.
    ///
    /// Nothing observes such a write, so none of the signal tables (e.g. the first-write table)
    /// are addressable and no slots are encoded for them.
    no_signal: bool,
    size: SixBitSize,
}
/// A set at a constant address which reads its value from, and writes its update mask to, the
/// heap.
///
/// The address is known at encoding time, so it needs neither an address register nor a runtime
/// bounds check.
struct SetHeap {
    rd: Reg,
    rs: Reg,
    flags: SetFlags,
    /// See [`SetRegRelative::no_signal`].
    no_signal: bool,
}
/// A register-relative set which reads its value from, and writes its update mask to, the heap.
///
/// The address comes from `roff` and is bounds checked against a range in the additional slots.
struct SetHeapRelative {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    flags: SetFlags,
    /// See [`SetRegRelative::no_signal`].
    no_signal: bool,
    offset: InlineAddrOffset<7>,
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
            flags: SetFlags::new_masked(v >> 16),
            imm12: (v >> 20) as u16,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.flags.encode() << 16)
                | ((self.imm12 as u32) << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            flags,
            imm12,
        } = self;
        flags.fmt(f)?;
        write!(f, "{rd}, {rs}, {imm12}")
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
            flags: SetFlags::new_masked(v >> 16),
            size: SixBitSize::new_masked(v >> 20),
            imm6: (v >> 26) as u8,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.flags.encode() << 16)
                | (self.size.encode() << 20)
                | ((self.imm6 as u32) << 26),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            flags,
            size,
            imm6,
        } = self;
        flags.fmt(f)?;
        write!(f, "{rd}, {rs}, {imm6}, |{size}|")
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
            flags: SetFlags::new_masked(v >> 20),
            offset: InlineAddrOffset::new_shifted(v, 24),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.flags.encode() << 20)
                | (self.offset.encode() << 24),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            flags,
            offset,
        } = self;
        flags.fmt(f)?;
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
            flags: SetFlags::new_masked(v >> 20),
            no_signal: (v >> 24) & 1 != 0,
            size: SixBitSize::new_masked(v >> 25),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.flags.encode() << 20)
                | (u32::from(self.no_signal) << 24)
                | (self.size.encode() << 25),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            flags,
            no_signal,
            size,
        } = self;
        flags.fmt(f)?;
        if *no_signal {
            f.write_str("[N]")?;
        }
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
            flags: SetFlags::new_masked(v >> 16),
            no_signal: (v >> 20) & 1 != 0,
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.flags.encode() << 16)
                | (u32::from(self.no_signal) << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            flags,
            no_signal,
        } = self;
        flags.fmt(f)?;
        if *no_signal {
            f.write_str("[N]")?;
        }
        write!(f, "{rd}, {rs}")
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
            flags: SetFlags::new_masked(v >> 20),
            no_signal: (v >> 24) & 1 != 0,
            offset: InlineAddrOffset::new_shifted(v, 25),
        }
    }
    #[inline(always)]
    fn encode(&self, opcode: BytecodeOpcode) -> Bytecode {
        Bytecode(
            opcode as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.roff as u32) << 16)
                | (self.flags.encode() << 20)
                | (u32::from(self.no_signal) << 24)
                | (self.offset.encode() << 25),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            flags,
            no_signal,
            offset,
        } = self;
        flags.fmt(f)?;
        if *no_signal {
            f.write_str("[N]")?;
        }
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
fn fv_set_aligned(heap: &mut [u64], offset: u64, spc: u64, val: u64, size: SixBitSize) -> u64 {
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

    (prev_spc ^ spc) | (prev_val ^ val)
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
fn write_in_bounds(offset: u64, size: SixBitSize, lower: u64, upper: u64) -> bool {
    let end = offset.wrapping_add(size as u64 - 1);
    (offset >= lower) & (end <= upper) & (end >= offset)
}

#[inline(always)]
fn correct_first(
    updated: &mut bool,
    additional_slots: &[Bytecode; 16],
    slot_offset: &mut usize,
    state: &mut RuntimeState,
) {
    let correct_first = decode_bytecode_u64(&additional_slots[*slot_offset..]);

    let woff = (correct_first / 64) as usize;
    let boff = correct_first % 64;

    *updated |= (state.tvl_first_write[woff] & (1u64 << boff)) == 0;
    state.tvl_first_write[woff] |= 1u64 << boff;
    *slot_offset += 2;
}

#[inline(always)]
fn set_last_update_time(
    additional_slots: &[Bytecode; 16],
    slot_offset: &mut usize,
    state: &mut RuntimeState,
) {
    let lupdt_index = decode_bytecode_u64(&additional_slots[*slot_offset..]);
    state.last_active_time[lupdt_index as usize] = state.time;
    *slot_offset += 2;
}

#[inline(always)]
fn poke_watchers(
    additional_slots: &[Bytecode; 16],
    slot_offset: &mut usize,
    cldctx: &mut ColdContext,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
) {
    let watch_index = decode_bytecode_u64(&additional_slots[*slot_offset..]);
    let watchers = cldctx.watchers.get(watch_index as usize);
    for &index in watchers {
        super::temporal::wake(index, schedule, listeners);
    }
    *slot_offset += 2;
}

#[inline(never)]
fn poke_plugins(additional_slots: &[Bytecode; 16], offset: &mut usize, cldctx: &mut ColdContext) {
    let rt_index = decode_bytecode_u64(&additional_slots[*offset..]);
    *offset += 2;

    let rt_index = RtSignalKey::from_usize(rt_index as usize).unwrap();
    for plugin in cldctx.plugins.iter_mut() {
        plugin.poke_signal(rt_index);
    }
}

impl BytecodeInstruction for TvSet1 {
    fn num_additional_slots(&self) -> u8 {
        let slots = 1 + 2 + self.0.flags.num_additional_slots();
        debug_assert!(slots <= 16);
        slots
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                Some(SCALAR_VSIZE),
            ));
        }
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
        let slots = 1 + self.0.flags.num_additional_slots();
        debug_assert!(slots <= 16);
        slots
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                Some(SCALAR_VSIZE),
            ));
        }
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
        let mut slots = self.0.flags.num_additional_slots();
        slots += 1; // Base.
        slots += 1; // Spread.
        debug_assert!(slots <= 16);
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                Some(SCALAR_VSIZE),
            ));
        }
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
        let Self(Set1 {
            rd,
            rs,
            flags,
            imm12,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
            .try_into()
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = additional_slots[0].0 as u64;
        let spread = VectorSize::new(additional_slots[1].0).unwrap();
        let base = HeapAlignment::new(spread, LogicMode::FourValue).from_elem_offset(base);
        let bit_offset = imm12 as u64;

        let heap = state.heap.0.as_mut();
        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let val_offset = HeapAlignment::spc_offset_to_val_offset(spread, base);
        let mut updated = false;
        updated |= set_unaligned_inbounds(heap, base + bit_offset, spc, SixBitSize::N1) != 0;
        updated |= set_unaligned_inbounds(heap, val_offset + bit_offset, val, SixBitSize::N1) != 0;

        if flags.contains(SetFlags::WRITE_MASK) {
            regs[rd] = u64::from(updated);
        }

        if !updated {
            return;
        }

        poke1(
            additional_slots,
            2,
            flags,
            regs,
            state,
            schedule,
            listeners,
            cldctx,
        );
    }
}
impl BytecodeInstruction for TvSet1Relative {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = self.0.flags.num_additional_slots();
        slots += 2; // Base.
        slots += 4; // Upper & Lower bound.
        slots += 2; // TV Correct first.
        debug_assert!(slots <= 16);
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                Some(SCALAR_VSIZE),
            ));
        }
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
        let Self(SetRelative {
            rd,
            rs,
            roff,
            flags,
            offset,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
            .try_into()
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[0..]);
        let lower_bound = decode_bytecode_u64(&additional_slots[2..]);
        let upper_bound = decode_bytecode_u64(&additional_slots[4..]);
        let heap_offset = offset.get(regs[roff]);

        if !write_in_bounds(heap_offset, SixBitSize::N1, lower_bound, upper_bound) {
            // @NOTE: A single bit is either fully in range or fully out, so there is nothing to
            // partially write here.
            std::hint::cold_path();
            if flags.contains(SetFlags::WRITE_MASK) {
                regs[rd] = 0;
            }
            return;
        }

        let heap = state.heap.0.as_mut();
        let value = regs[rs];
        let mut updated =
            set_unaligned_inbounds(heap, base + heap_offset, value, SixBitSize::N1) != 0;
        correct_first(&mut updated, additional_slots, &mut 6, state);

        if flags.contains(SetFlags::WRITE_MASK) {
            regs[rd] = u64::from(updated);
        }

        if !updated {
            return;
        }

        poke1(
            additional_slots,
            8,
            flags,
            regs,
            state,
            schedule,
            listeners,
            cldctx,
        );
    }
}
impl BytecodeInstruction for FvSet1Relative {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = self.0.flags.num_additional_slots();
        slots += 2; // Base.
        slots += 4; // Upper & Lower bound.
        slots += 1; // Spread.
        debug_assert!(slots <= 16);
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                Some(SCALAR_VSIZE),
            ));
        }
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
        let Self(SetRelative {
            rd,
            rs,
            roff,
            flags,
            offset,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
            .try_into()
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let lower_bound = decode_bytecode_u64(&additional_slots[2..]);
        let upper_bound = decode_bytecode_u64(&additional_slots[4..]);
        let spread = VectorSize::new(additional_slots[6].0).unwrap();
        let bit_offset = offset.get(regs[roff]);

        if !write_in_bounds(bit_offset, SixBitSize::N1, lower_bound, upper_bound) {
            // @NOTE: A single bit is either fully in range or fully out, so there is nothing to
            // partially write here.
            std::hint::cold_path();
            if flags.contains(SetFlags::WRITE_MASK) {
                regs[rd] = 0;
            }
            return;
        }

        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let base_val = HeapAlignment::spc_offset_to_val_offset(spread, base);
        let heap = state.heap.0.as_mut();

        let mut updated = false;
        updated |= set_unaligned_inbounds(heap, base + bit_offset, spc, SixBitSize::N1) != 0;
        updated |= set_unaligned_inbounds(heap, base_val + bit_offset, val, SixBitSize::N1) != 0;

        if flags.contains(SetFlags::WRITE_MASK) {
            regs[rd] = u64::from(updated);
        }

        if !updated {
            return;
        }

        poke1(
            additional_slots,
            7,
            flags,
            regs,
            state,
            schedule,
            listeners,
            cldctx,
        );
    }
}

#[inline(always)]
fn execute_set1<const FOUR_VALUE: bool>(
    args: Set1,
    code: &[Bytecode],
    regs: &mut Regs,
    pc: &mut u64,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    cldctx: &mut ColdContext,
) {
    let Set1 {
        rd,
        rs,
        flags,
        imm12,
    } = args;

    // @NOTE: We ensured the code buffer is padded with at least 16 elements.
    let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
        .try_into()
        .expect("Should be padded");

    *pc += 1; // Offset.
    *pc += if FOUR_VALUE { 0 } else { 2 }; // TV Correct first.
    *pc += flags.num_additional_slots() as u64; // Flag slots.

    let code_offset = additional_slots[0].0;
    let heap_offset = ((code_offset as u64) << 12) | (imm12 as u64);

    let mut slot_offset = 1;
    let heap = state.heap.0.as_mut();
    let mut updated = if FOUR_VALUE {
        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        fv_set_aligned(heap, heap_offset, spc, val, SixBitSize::N1) != 0
    } else {
        let src = regs[rs];
        tv_set_aligned(heap, heap_offset, src, SixBitSize::N1) != 0
    };

    if !FOUR_VALUE {
        correct_first(&mut updated, additional_slots, &mut slot_offset, state);
    }

    if flags.contains(SetFlags::WRITE_MASK) {
        regs[rd] = u64::from(updated);
    }

    if !updated {
        return;
    }

    poke1(
        additional_slots,
        slot_offset,
        flags,
        regs,
        state,
        schedule,
        listeners,
        cldctx,
    );
}

#[inline(always)]
fn poke1(
    additional_slots: &[Bytecode; 16],
    mut slot_offset: usize,
    flags: SetFlags,
    _regs: &mut Regs,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    cldctx: &mut ColdContext,
) {
    if flags.contains(SetFlags::LAST_UPDATE_TIME) {
        set_last_update_time(additional_slots, &mut slot_offset, state);
    }

    if flags.contains(SetFlags::WATCH) {
        poke_watchers(
            additional_slots,
            &mut slot_offset,
            cldctx,
            schedule,
            listeners,
        );
    }

    if flags.contains(SetFlags::PLUGIN_POKE) {
        std::hint::cold_path();
        poke_plugins(additional_slots, &mut slot_offset, cldctx);
    }
}

impl BytecodeInstruction for TvSetAligned {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = self.0.flags.num_additional_slots();
        slots += 1; // Base.
        slots += 2; // TV Correct first.
        debug_assert!(slots <= 16);
        slots
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                Some(self.0.size.into()),
            ));
        }
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
        let Self(Set {
            rd,
            rs,
            flags,
            size,
            imm6,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
            .try_into()
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = additional_slots[0].0 as u64;
        let base = HeapAlignment::new(size.into(), LogicMode::TwoValue).from_elem_offset(base);
        let offset = base + imm6 as u64;

        let heap = state.heap.0.as_mut();
        let update_mask = tv_set_aligned(heap, offset, regs[rs], size);
        let mut updated = update_mask != 0;

        correct_first(&mut updated, additional_slots, &mut 1, state);

        if flags.contains(SetFlags::WRITE_MASK) {
            regs[rd] = update_mask;
        }

        if !updated {
            return;
        }

        poke1(
            additional_slots,
            3,
            flags,
            regs,
            state,
            schedule,
            listeners,
            cldctx,
        );
    }
}

impl BytecodeInstruction for FvSetAligned {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = self.0.flags.num_additional_slots();
        slots += 1; // Base.
        slots += 1; // Spread.
        debug_assert!(slots <= 16);
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                Some(self.0.size.into()),
            ));
        }
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
        let Self(Set {
            rd,
            rs,
            flags,
            size,
            imm6,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
            .try_into()
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = additional_slots[0].0 as u64;
        let spread = VectorSize::new(additional_slots[1].0).unwrap();
        let base = HeapAlignment::new(spread, LogicMode::FourValue).from_elem_offset(base);
        let offset = base + imm6 as u64;
        let val_offset = HeapAlignment::spc_offset_to_val_offset(spread, offset);

        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let heap = state.heap.0.as_mut();

        let update_mask = set_unaligned_inbounds(heap, offset, spc, size)
            | set_unaligned_inbounds(heap, val_offset, val, size);

        if flags.contains(SetFlags::WRITE_MASK) {
            regs[rd] = update_mask;
        }

        if update_mask == 0 {
            return;
        }

        poke1(
            additional_slots,
            2,
            flags,
            regs,
            state,
            schedule,
            listeners,
            cldctx,
        );
    }
}

impl BytecodeInstruction for TvSetUnaligned {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = self.0.flags.num_additional_slots();
        slots += 2; // Base.
        slots += 2; // TV Correct first.
        debug_assert!(slots <= 16);
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                Some(self.0.size.into()),
            ));
        }
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
        let Self(Set {
            rd,
            rs,
            flags,
            size,
            imm6,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
            .try_into()
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let offset = base + imm6 as u64;

        let heap = state.heap.0.as_mut();
        let update_mask = set_unaligned_inbounds(heap, offset, size.mask(regs[rs]), size);
        let mut updated = update_mask != 0;

        correct_first(&mut updated, additional_slots, &mut 2, state);

        if flags.contains(SetFlags::WRITE_MASK) {
            regs[rd] = update_mask;
        }

        if !updated {
            return;
        }

        poke1(
            additional_slots,
            4,
            flags,
            regs,
            state,
            schedule,
            listeners,
            cldctx,
        );
    }
}

impl BytecodeInstruction for FvSetUnaligned {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = self.0.flags.num_additional_slots();
        slots += 2; // Base.
        slots += 1; // Spread.
        debug_assert!(slots <= 16);
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                Some(self.0.size.into()),
            ));
        }
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
        let Self(Set {
            rd,
            rs,
            flags,
            size,
            imm6,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
            .try_into()
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let spread = VectorSize::new(additional_slots[2].0).expect("Expected non-zero size");
        let offset = base + imm6 as u64;
        let val_offset = HeapAlignment::spc_offset_to_val_offset(spread, offset);

        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let heap = state.heap.0.as_mut();

        let update_mask = set_unaligned_inbounds(heap, offset, size.mask(spc), size)
            | set_unaligned_inbounds(heap, val_offset, size.mask(val), size);

        if flags.contains(SetFlags::WRITE_MASK) {
            regs[rd] = update_mask;
        }

        if update_mask == 0 {
            return;
        }

        poke1(
            additional_slots,
            3,
            flags,
            regs,
            state,
            schedule,
            listeners,
            cldctx,
        );
    }
}

impl BytecodeInstruction for TvSetRelative {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 2; // Base.
        slots += 4; // Upper & Lower bound.
        if !self.0.no_signal {
            slots += 2; // TV Correct first.
            slots += self.0.flags.num_additional_slots();
        }
        debug_assert!(slots <= 16);
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                None,
            ));
        }
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
        let Self(SetRegRelative {
            rd,
            rs,
            roff,
            flags,
            no_signal,
            size,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
            .try_into()
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let lower_bound = decode_bytecode_u64(&additional_slots[2..]);
        let upper_bound = decode_bytecode_u64(&additional_slots[4..]);
        let bit_offset = regs[roff];

        let offset = base + bit_offset;
        let heap = state.heap.0.as_mut();
        // @NOTE: `set_unaligned_inbounds` expects the value to be premasked to `size`.
        let value = size.mask(regs[rs]);

        let update_mask = if write_in_bounds(offset, size, lower_bound, upper_bound) {
            set_unaligned_inbounds(heap, offset, value, size)
        } else {
            // @NOTE: A relative write is expected to land in bounds. A partially or fully
            // out-of-range index is rare, so keep it off the hot path. The in-range part of the
            // write is still performed.
            std::hint::cold_path();
            set_unaligned_oob(heap, offset, value, size, lower_bound, upper_bound)
        };

        // @NOTE: A write which does not address a signal has no first-write table to index into
        // and nothing to poke.
        if no_signal {
            return;
        }

        let mut updated = update_mask != 0;

        correct_first(&mut updated, additional_slots, &mut 6, state);

        if flags.contains(SetFlags::WRITE_MASK) {
            regs[rd] = update_mask;
        }

        if !updated {
            return;
        }

        poke1(
            additional_slots,
            8,
            flags,
            regs,
            state,
            schedule,
            listeners,
            cldctx,
        );
    }
}

impl BytecodeInstruction for FvSetRelative {
    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 2; // Base.
        slots += 4; // Upper & Lower bound.
        slots += 1; // Spread.
        if !self.0.no_signal {
            slots += self.0.flags.num_additional_slots();
        }
        debug_assert!(slots <= 16);
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
        if self.0.flags.contains(SetFlags::WRITE_MASK) {
            operands.push(RegInfo::register(
                "rd",
                self.0.rd,
                LogicMode::TwoValue,
                None,
            ));
        }
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
        let Self(SetRegRelative {
            rd,
            rs,
            roff,
            flags,
            no_signal,
            size,
        }) = self;

        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
            .try_into()
            .expect("Should be padded");

        *pc += self.num_additional_slots() as u64;

        let base = decode_bytecode_u64(&additional_slots[..]);
        let lower_bound = decode_bytecode_u64(&additional_slots[2..]);
        let upper_bound = decode_bytecode_u64(&additional_slots[4..]);
        let spread = VectorSize::new(additional_slots[6].0).expect("Expected non-zero size");
        let bit_offset = regs[roff];

        let base_val = HeapAlignment::spc_offset_to_val_offset(spread, base);
        let (rsspc, rsval) = rs.to_spc_and_val();
        let (spc, val) = (regs[rsspc], regs[rsval]);
        let heap = state.heap.0.as_mut();

        // @NOTE: `set_unaligned_inbounds` expects the value to be premasked to `size`.
        let (spc, val) = (size.mask(spc), size.mask(val));
        let offset = base + bit_offset;
        let val_offset = base_val + bit_offset;

        // @NOTE: Both planes sit at the same relative position within their own plane, so one
        // check covers both.
        let update_mask = if write_in_bounds(offset, size, lower_bound, upper_bound) {
            set_unaligned_inbounds(heap, offset, spc, size)
                | set_unaligned_inbounds(heap, val_offset, val, size)
        } else {
            // @NOTE: A relative write is expected to land in bounds. A partially or fully
            // out-of-range index is rare, so keep it off the hot path. The in-range part of the
            // write is still performed.
            std::hint::cold_path();
            let val_lower = HeapAlignment::spc_offset_to_val_offset(spread, lower_bound);
            let val_upper = HeapAlignment::spc_offset_to_val_offset(spread, upper_bound);
            set_unaligned_oob(heap, offset, spc, size, lower_bound, upper_bound)
                | set_unaligned_oob(heap, val_offset, val, size, val_lower, val_upper)
        };

        if flags.contains(SetFlags::WRITE_MASK) {
            regs[rd] = update_mask;
        }

        // @NOTE: A write which does not address a signal has nothing to poke.
        if no_signal || update_mask == 0 {
            return;
        }

        poke1(
            additional_slots,
            7,
            flags,
            regs,
            state,
            schedule,
            listeners,
            cldctx,
        );
    }
}

/// Perform a heap-to-heap set of `size` bits and write the update mask back onto the heap.
///
/// The source lives at the address in `rs` and the update mask is written to the address in `rd`
/// if the [`SetFlags::WRITE_MASK`] flag is set. For four-value, both planes are written with the
/// value plane a `spread` away from the special plane, and the mask is the OR sum of both.
///
/// Returns whether any bit was changed.
#[inline(always)]
fn set_heap_wide<const FOUR_VALUE: bool>(
    rd: Reg,
    rs: Reg,
    flags: SetFlags,
    offset: u64,
    size: VectorSize,
    spread: VectorSize,
    base: u64,
    base_size: VectorSize,
    regs: &Regs,
    state: &mut RuntimeState,
    cldctx: &mut ColdContext,
) -> bool {
    let src_num_words = size.get().div_ceil(64) as usize;
    let num_planes = if FOUR_VALUE { 2 } else { 1 };

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

    let dst = state
        .heap
        .get_mut_u64_slice(HeapOffset { bit_offset: 0usize }, state.heap.0.len());

    set_with_mask(
        update_mask,
        dst,
        &scratch_src[..src_num_words],
        offset,
        size,
        base,
        base_size,
    );
    if FOUR_VALUE {
        let val_offset = HeapAlignment::spc_offset_to_val_offset(spread, offset);
        let base_val = HeapAlignment::spc_offset_to_val_offset(spread, base);
        set_with_mask(
            update_mask,
            dst,
            &scratch_src[src_num_words..],
            val_offset,
            size,
            base_val,
            base_size,
        );
    }

    let updated = update_mask.iter().any(|w| *w != 0);

    if flags.contains(SetFlags::WRITE_MASK) {
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

    updated
}

/// Implement a heap set at a constant address.
///
/// `$partial` selects whether the write covers only part of the signal, which needs a base range
/// to trim against. A two-value whole-signal write needs no base at all; a four-value one still
/// encodes it, since the value plane sits a spread away from the base.
macro_rules! impl_set_heap {
    ($name:ident, $mnemonic:literal, $four_value:literal, $partial:literal) => {
        impl $name {
            /// Slot index of the trailing size field.
            const SIZE_SLOT: usize = 2;
            /// Slot index just past the fixed operands.
            const END_SLOT: usize = Self::SIZE_SLOT
                + 1
                + if $partial { 1 } else { 0 }
                + if $four_value { 1 } else { 0 };
        }

        impl BytecodeInstruction for $name {
            fn num_additional_slots(&self) -> u8 {
                let mut slots = Self::END_SLOT as u8;
                if !self.0.no_signal {
                    if !$four_value {
                        slots += 2; // TV Correct first.
                    }
                    slots += self.0.flags.num_additional_slots();
                }
                debug_assert!(slots <= 16);
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
                let size = code[pc as usize + Self::SIZE_SLOT].unwrap_size();
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
                if self.0.flags.contains(SetFlags::WRITE_MASK) {
                    let size = code[pc as usize + Self::SIZE_SLOT].unwrap_size();
                    operands.push(RegInfo::heap("rd", self.0.rd, LogicMode::TwoValue, size));
                }
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
                let Self(SetHeap {
                    rd,
                    rs,
                    flags,
                    no_signal,
                }) = self;

                // @NOTE: We ensured the code buffer is padded with at least 16 elements.
                let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
                    .try_into()
                    .expect("Should be padded");

                *pc += self.num_additional_slots() as u64;

                let size = additional_slots[Self::SIZE_SLOT].unwrap_size();
                // @NOTE: The base is the address the write lands at, and for a partial write also
                // the start of the region it is trimmed to.
                let base = decode_bytecode_u64(&additional_slots[..]);
                let base_size = if $partial {
                    additional_slots[Self::SIZE_SLOT + 1].unwrap_size()
                } else {
                    size
                };
                let spread = if $four_value {
                    additional_slots[Self::END_SLOT - 1].unwrap_size()
                } else {
                    size
                };

                let updated = set_heap_wide::<$four_value>(
                    rd, rs, flags, base, size, spread, base, base_size, regs, state, cldctx,
                );

                // @NOTE: A write which does not address a signal has no first-write table to
                // index into and nothing to poke.
                if no_signal {
                    return;
                }

                let mut updated = updated;
                let mut slot_offset = Self::END_SLOT;
                if !$four_value {
                    correct_first(&mut updated, additional_slots, &mut slot_offset, state);
                }

                if !updated {
                    return;
                }

                poke1(
                    additional_slots,
                    slot_offset,
                    flags,
                    regs,
                    state,
                    schedule,
                    listeners,
                    cldctx,
                );
            }
        }
    };
}

/// Implement a heap set at a register-relative address.
///
/// The address comes from `roff` and is bounds checked against an inclusive range; bits outside
/// it are not written.
macro_rules! impl_set_heap_relative {
    ($name:ident, $mnemonic:literal, $four_value:literal) => {
        impl $name {
            /// Slot index of the trailing size field.
            const SIZE_SLOT: usize = 4;
            /// Slot index just past the fixed operands.
            const END_SLOT: usize = Self::SIZE_SLOT + 1 + if $four_value { 1 } else { 0 };
        }

        impl BytecodeInstruction for $name {
            fn num_additional_slots(&self) -> u8 {
                let mut slots = Self::END_SLOT as u8;
                if !self.0.no_signal {
                    if !$four_value {
                        slots += 2; // TV Correct first.
                    }
                    slots += self.0.flags.num_additional_slots();
                }
                debug_assert!(slots <= 16);
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
                let size = code[pc as usize + Self::SIZE_SLOT].unwrap_size();
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
                    Some(VSIZE_64),
                ));
            }

            fn dest_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>) {
                if self.0.flags.contains(SetFlags::WRITE_MASK) {
                    let size = code[pc as usize + Self::SIZE_SLOT].unwrap_size();
                    operands.push(RegInfo::heap("rd", self.0.rd, LogicMode::TwoValue, size));
                }
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
                let Self(SetHeapRelative {
                    rd,
                    rs,
                    roff,
                    flags,
                    no_signal,
                    offset,
                }) = self;

                // @NOTE: We ensured the code buffer is padded with at least 16 elements.
                let additional_slots: &[Bytecode; 16] = &code[*pc as usize..*pc as usize + 16]
                    .try_into()
                    .expect("Should be padded");

                *pc += self.num_additional_slots() as u64;

                let lower_bound = decode_bytecode_u64(&additional_slots[..]);
                let upper_bound = decode_bytecode_u64(&additional_slots[2..]);
                let size = additional_slots[Self::SIZE_SLOT].unwrap_size();
                let spread = if $four_value {
                    additional_slots[Self::END_SLOT - 1].unwrap_size()
                } else {
                    size
                };

                let offset = offset.get(regs[roff]);

                // @NOTE: `set_with_mask` trims to the base range itself, so a partially in-range
                // write still lands its in-range bits.
                let base_size = VectorSize::new(
                    (upper_bound.saturating_sub(lower_bound) + 1).min(u32::MAX as u64) as u32,
                )
                .expect("Expected non-zero range");

                let updated = set_heap_wide::<$four_value>(
                    rd,
                    rs,
                    flags,
                    offset,
                    size,
                    spread,
                    lower_bound,
                    base_size,
                    regs,
                    state,
                    cldctx,
                );

                // @NOTE: A write which does not address a signal has no first-write table to
                // index into and nothing to poke.
                if no_signal {
                    return;
                }

                let mut updated = updated;
                let mut slot_offset = Self::END_SLOT;
                if !$four_value {
                    correct_first(&mut updated, additional_slots, &mut slot_offset, state);
                }

                if !updated {
                    return;
                }

                poke1(
                    additional_slots,
                    slot_offset,
                    flags,
                    regs,
                    state,
                    schedule,
                    listeners,
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
fn encode_bytecode_u64(bce: &mut BytecodeEncoder, v: u64) {
    bce.data.push(Bytecode((v & 0xFFFF_FFFF) as u32));
    bce.data.push(Bytecode((v >> 32) as u32));
}

#[inline(always)]
fn decode_bytecode_u64(slots: &[Bytecode]) -> u64 {
    u64::from(slots[0].0) | (u64::from(slots[1].0) << 32)
}

impl BytecodeEncoder {
    fn set1(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        offset: u64,
        tv_correct_index: Option<u64>,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
        f: impl FnOnce(Set1) -> Bytecode,
    ) {
        let mut flags = SetFlags::EMPTY;
        flags.set(SetFlags::WRITE_MASK, rd.is_some());
        flags.set(SetFlags::LAST_UPDATE_TIME, lupdt_index.is_some());
        flags.set(SetFlags::WATCH, watch_index.is_some());
        flags.set(SetFlags::PLUGIN_POKE, plugin_rt_index.is_some());

        self.data.push(f(Set1 {
            rd: rd.unwrap_or(rs),
            rs,
            flags,
            imm12: (offset & 0xFFF) as u16,
        }));
        self.data
            .push(Bytecode(((offset >> 12) & 0xFFFF_FFFF) as u32));
        if let Some(tv_correct_index) = tv_correct_index {
            encode_bytecode_u64(self, tv_correct_index);
        }
        if let Some(lupdt_index) = lupdt_index {
            encode_bytecode_u64(self, lupdt_index);
        }
        if let Some(watch_index) = watch_index {
            encode_bytecode_u64(self, watch_index);
        }
        if let Some(plugin_rt_index) = plugin_rt_index {
            encode_bytecode_u64(self, plugin_rt_index);
        }
    }

    pub fn tv_set1(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        offset: u64,
        tv_correct_index: u64,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
        scratch: Reg,
    ) {
        if offset >= 1u64 << 44 {
            self.load_u64(scratch, offset);
            self.tv_set1rel(
                rd,
                rs,
                scratch,
                0,
                InlineAddrOffset::ZERO,
                0..=u64::MAX,
                tv_correct_index,
                lupdt_index,
                watch_index,
                plugin_rt_index,
            );
            return;
        }

        self.set1(
            rd,
            rs,
            offset,
            Some(tv_correct_index),
            lupdt_index,
            watch_index,
            plugin_rt_index,
            |args| TvSet1(args).encode(),
        );
    }
    pub fn fv_set1(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        offset: u64,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
        scratch: Reg,
    ) {
        if offset >= 1u64 << 44 {
            self.load_u64(scratch, 0);
            self.fv_set1rel(
                rd,
                rs,
                scratch,
                offset,
                InlineAddrOffset::ZERO,
                0..=u64::MAX,
                SCALAR_VSIZE,
                lupdt_index,
                watch_index,
                plugin_rt_index,
            );
            return;
        }

        self.set1(
            rd,
            rs,
            offset,
            None,
            lupdt_index,
            watch_index,
            plugin_rt_index,
            |args| FvSet1(args).encode(),
        );
    }
    pub fn fv_set1spread(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        base: u64,
        offset: u64,
        spread: VectorSize,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
        scratch: Reg,
    ) {
        let alignment = HeapAlignment::new(spread, LogicMode::FourValue);
        let base_elem = alignment.to_elem_offset(base);

        if base_elem >= 1u64 << 32 || offset >= 1u64 << 12 {
            self.load_u64(scratch, 0);
            let (roff, offset) = InlineAddrOffset::new(offset as i64, self, scratch, scratch);
            self.fv_set1rel(
                rd,
                rs,
                roff,
                base,
                offset,
                0..=u64::MAX,
                spread,
                lupdt_index,
                watch_index,
                plugin_rt_index,
            );
            return;
        }

        let base = base_elem;
        let mut flags = SetFlags::EMPTY;
        flags.set(SetFlags::WRITE_MASK, rd.is_some());
        flags.set(SetFlags::LAST_UPDATE_TIME, lupdt_index.is_some());
        flags.set(SetFlags::WATCH, watch_index.is_some());
        flags.set(SetFlags::PLUGIN_POKE, plugin_rt_index.is_some());

        self.data.push(
            FvSet1Spread(Set1 {
                rd: rd.unwrap_or(rs),
                rs,
                flags,
                imm12: (offset & 0xFFF) as u16,
            })
            .encode(),
        );
        self.data.push(Bytecode((base & 0xFFFF_FFFF) as u32));
        self.data.push(Bytecode(spread.get()));
        if let Some(lupdt_index) = lupdt_index {
            encode_bytecode_u64(self, lupdt_index);
        }
        if let Some(watch_index) = watch_index {
            encode_bytecode_u64(self, watch_index);
        }
        if let Some(plugin_rt_index) = plugin_rt_index {
            encode_bytecode_u64(self, plugin_rt_index);
        }
    }

    pub fn tv_set1rel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        base: u64,
        offset: InlineAddrOffset<8>,
        range: RangeInclusive<u64>,
        tv_correct_index: u64,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let mut flags = SetFlags::EMPTY;
        flags.set(SetFlags::WRITE_MASK, rd.is_some());
        flags.set(SetFlags::LAST_UPDATE_TIME, lupdt_index.is_some());
        flags.set(SetFlags::WATCH, watch_index.is_some());
        flags.set(SetFlags::PLUGIN_POKE, plugin_rt_index.is_some());

        self.data.push(
            TvSet1Relative(SetRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                flags,
                offset,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        encode_bytecode_u64(self, *range.start());
        encode_bytecode_u64(self, *range.end());
        encode_bytecode_u64(self, tv_correct_index);
        if let Some(lupdt_index) = lupdt_index {
            encode_bytecode_u64(self, lupdt_index);
        }
        if let Some(watch_index) = watch_index {
            encode_bytecode_u64(self, watch_index);
        }
        if let Some(plugin_rt_index) = plugin_rt_index {
            encode_bytecode_u64(self, plugin_rt_index);
        }
    }
    pub fn fv_set1rel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        base: u64,
        offset: InlineAddrOffset<8>,
        range: RangeInclusive<u64>,
        spread: VectorSize,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let mut flags = SetFlags::EMPTY;
        flags.set(SetFlags::WRITE_MASK, rd.is_some());
        flags.set(SetFlags::LAST_UPDATE_TIME, lupdt_index.is_some());
        flags.set(SetFlags::WATCH, watch_index.is_some());
        flags.set(SetFlags::PLUGIN_POKE, plugin_rt_index.is_some());

        self.data.push(
            FvSet1Relative(SetRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                flags,
                offset,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        encode_bytecode_u64(self, *range.start());
        encode_bytecode_u64(self, *range.end());
        self.data.push(Bytecode(spread.get()));
        if let Some(lupdt_index) = lupdt_index {
            encode_bytecode_u64(self, lupdt_index);
        }
        if let Some(watch_index) = watch_index {
            encode_bytecode_u64(self, watch_index);
        }
        if let Some(plugin_rt_index) = plugin_rt_index {
            encode_bytecode_u64(self, plugin_rt_index);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn flags_for(
        rd: Option<Reg>,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) -> SetFlags {
        let mut flags = SetFlags::EMPTY;
        flags.set(SetFlags::WRITE_MASK, rd.is_some());
        flags.set(SetFlags::LAST_UPDATE_TIME, lupdt_index.is_some());
        flags.set(SetFlags::WATCH, watch_index.is_some());
        flags.set(SetFlags::PLUGIN_POKE, plugin_rt_index.is_some());
        flags
    }

    fn encode_poke_slots(
        &mut self,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        if let Some(lupdt_index) = lupdt_index {
            encode_bytecode_u64(self, lupdt_index);
        }
        if let Some(watch_index) = watch_index {
            encode_bytecode_u64(self, watch_index);
        }
        if let Some(plugin_rt_index) = plugin_rt_index {
            encode_bytecode_u64(self, plugin_rt_index);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn tv_set_aligned(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        at: u64,
        size: SixBitSize,
        tv_correct_index: u64,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
        scratch: Reg,
    ) {
        let alignment = HeapAlignment::new(size.into(), LogicMode::TwoValue);
        let base = alignment.next_aligned(at.saturating_sub(0x3F));
        let base_elem = alignment.to_elem_offset(base);

        if base_elem >= 1u64 << 32 || at - base >= 1u64 << 6 {
            self.load_u64(scratch, at);
            self.tv_setrel(
                rd,
                rs,
                scratch,
                0,
                0..=u64::MAX,
                size,
                Some(tv_correct_index),
                lupdt_index,
                watch_index,
                plugin_rt_index,
            );
            return;
        }

        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            TvSetAligned(Set {
                rd: rd.unwrap_or(rs),
                rs,
                flags,
                size,
                imm6: (at - base) as u8,
            })
            .encode(),
        );
        self.data.push(Bytecode(base_elem as u32));
        encode_bytecode_u64(self, tv_correct_index);
        self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fv_set_aligned(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        at: u64,
        size: SixBitSize,
        spread: VectorSize,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
        scratch: Reg,
    ) {
        let alignment = HeapAlignment::new(spread, LogicMode::FourValue);
        let base = alignment.next_aligned(at.saturating_sub(0x3F));
        let base_elem = alignment.to_elem_offset(base);

        if base_elem >= 1u64 << 32 || at - base >= 1u64 << 6 {
            self.load_u64(scratch, at);
            self.fv_setrel(
                rd,
                rs,
                scratch,
                0,
                0..=u64::MAX,
                size,
                spread,
                false,
                lupdt_index,
                watch_index,
                plugin_rt_index,
            );
            return;
        }

        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            FvSetAligned(Set {
                rd: rd.unwrap_or(rs),
                rs,
                flags,
                size,
                imm6: (at - base) as u8,
            })
            .encode(),
        );
        self.data.push(Bytecode(base_elem as u32));
        self.data.push(Bytecode(spread.get()));
        self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn tv_set_unaligned(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        at: u64,
        size: SixBitSize,
        base: u64,
        base_size: VectorSize,
        tv_correct_index: u64,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
        scratch: Reg,
    ) {
        if at.saturating_sub(base) >= 1u64 << 6 {
            self.load_u64(scratch, at);
            self.tv_setrel(
                rd,
                rs,
                scratch,
                0,
                base..=base + base_size.get() as u64 - 1,
                size,
                Some(tv_correct_index),
                lupdt_index,
                watch_index,
                plugin_rt_index,
            );
            return;
        }

        // @NOTE: The address is known here, so a write which does not fit the base range is
        // trimmed at encoding time rather than checked at runtime.
        debug_assert!(at + size as u64 <= base + base_size.get() as u64);

        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            TvSetUnaligned(Set {
                rd: rd.unwrap_or(rs),
                rs,
                flags,
                size,
                imm6: (at - base) as u8,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        encode_bytecode_u64(self, tv_correct_index);
        self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fv_set_unaligned(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        at: u64,
        size: SixBitSize,
        base: u64,
        base_size: VectorSize,
        spread: VectorSize,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
        scratch: Reg,
    ) {
        if at.saturating_sub(base) >= 1u64 << 6 {
            self.load_u64(scratch, at);
            self.fv_setrel(
                rd,
                rs,
                scratch,
                0,
                base..=base + base_size.get() as u64 - 1,
                size,
                spread,
                false,
                lupdt_index,
                watch_index,
                plugin_rt_index,
            );
            return;
        }

        // @NOTE: The address is known here, so a write which does not fit the base range is
        // trimmed at encoding time rather than checked at runtime.
        debug_assert!(at + size as u64 <= base + base_size.get() as u64);

        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            FvSetUnaligned(Set {
                rd: rd.unwrap_or(rs),
                rs,
                flags,
                size,
                imm6: (at - base) as u8,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(spread.get()));
        self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
    }

    #[allow(clippy::too_many_arguments)]
    /// Encode a two-value register-relative set.
    ///
    /// A `tv_correct_index` of [`None`] marks the write as not addressing a signal, which skips
    /// the first-write correction and every poke.
    pub fn tv_setrel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        base: u64,
        range: RangeInclusive<u64>,
        size: SixBitSize,
        tv_correct_index: Option<u64>,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            TvSetRelative(SetRegRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                flags,
                no_signal: tv_correct_index.is_none(),
                size,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        encode_bytecode_u64(self, *range.start());
        encode_bytecode_u64(self, *range.end());
        if let Some(tv_correct_index) = tv_correct_index {
            encode_bytecode_u64(self, tv_correct_index);
            self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Encode a four-value register-relative set.
    ///
    /// `no_signal` marks the write as not addressing a signal, which skips every poke.
    #[allow(clippy::too_many_arguments)]
    pub fn fv_setrel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        base: u64,
        range: RangeInclusive<u64>,
        size: SixBitSize,
        spread: VectorSize,
        no_signal: bool,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            FvSetRelative(SetRegRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                flags,
                no_signal,
                size,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        encode_bytecode_u64(self, *range.start());
        encode_bytecode_u64(self, *range.end());
        self.data.push(Bytecode(spread.get()));
        if !no_signal {
            self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
        }
    }

    /// Encode a two-value heap set which covers the whole signal.
    ///
    /// The write is exactly the signal, so no base range is encoded.
    pub fn tv_set_whole_heap(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        base: u64,
        size: VectorSize,
        tv_correct_index: Option<u64>,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            TvSetWholeHeap(SetHeap {
                rd: rd.unwrap_or(rs),
                rs,
                flags,
                no_signal: tv_correct_index.is_none(),
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(size.get()));
        if let Some(tv_correct_index) = tv_correct_index {
            encode_bytecode_u64(self, tv_correct_index);
            self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
        }
    }

    /// Encode a four-value heap set which covers the whole signal.
    ///
    /// A base is still encoded, since the value plane sits a `spread` away from it.
    #[allow(clippy::too_many_arguments)]
    pub fn fv_set_whole_heap(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        base: u64,
        size: VectorSize,
        spread: VectorSize,
        no_signal: bool,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            FvSetWholeHeap(SetHeap {
                rd: rd.unwrap_or(rs),
                rs,
                flags,
                no_signal,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(size.get()));
        self.data.push(Bytecode(spread.get()));
        if !no_signal {
            self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
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
        tv_correct_index: Option<u64>,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            TvSetPartialHeap(SetHeap {
                rd: rd.unwrap_or(rs),
                rs,
                flags,
                no_signal: tv_correct_index.is_none(),
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(size.get()));
        self.data.push(Bytecode(base_size.get()));
        if let Some(tv_correct_index) = tv_correct_index {
            encode_bytecode_u64(self, tv_correct_index);
            self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
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
        spread: VectorSize,
        no_signal: bool,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            FvSetPartialHeap(SetHeap {
                rd: rd.unwrap_or(rs),
                rs,
                flags,
                no_signal,
            })
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(size.get()));
        self.data.push(Bytecode(base_size.get()));
        self.data.push(Bytecode(spread.get()));
        if !no_signal {
            self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
        }
    }

    /// Encode a two-value heap set at a register-relative address.
    #[allow(clippy::too_many_arguments)]
    pub fn tv_set_heaprel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        offset: InlineAddrOffset<7>,
        range: RangeInclusive<u64>,
        size: VectorSize,
        tv_correct_index: Option<u64>,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            TvSetHeapRelative(SetHeapRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                flags,
                no_signal: tv_correct_index.is_none(),
                offset,
            })
            .encode(),
        );
        encode_bytecode_u64(self, *range.start());
        encode_bytecode_u64(self, *range.end());
        self.data.push(Bytecode(size.get()));
        if let Some(tv_correct_index) = tv_correct_index {
            encode_bytecode_u64(self, tv_correct_index);
            self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
        }
    }

    /// Encode a four-value heap set at a register-relative address.
    #[allow(clippy::too_many_arguments)]
    pub fn fv_set_heaprel(
        &mut self,
        rd: Option<Reg>,
        rs: Reg,
        roff: Reg,
        offset: InlineAddrOffset<7>,
        range: RangeInclusive<u64>,
        size: VectorSize,
        spread: VectorSize,
        no_signal: bool,
        lupdt_index: Option<u64>,
        watch_index: Option<u64>,
        plugin_rt_index: Option<u64>,
    ) {
        let flags = Self::flags_for(rd, lupdt_index, watch_index, plugin_rt_index);
        self.data.push(
            FvSetHeapRelative(SetHeapRelative {
                rd: rd.unwrap_or(rs),
                rs,
                roff,
                flags,
                no_signal,
                offset,
            })
            .encode(),
        );
        encode_bytecode_u64(self, *range.start());
        encode_bytecode_u64(self, *range.end());
        self.data.push(Bytecode(size.get()));
        self.data.push(Bytecode(spread.get()));
        if !no_signal {
            self.encode_poke_slots(lupdt_index, watch_index, plugin_rt_index);
        }
    }
}
