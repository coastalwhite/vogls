//! Linear Scan Register Allocation

use core::fmt;
use std::collections::VecDeque;

use slotmap::SlotMap;
use vogls_ir::{
    BasicBlock, BasicBlockKey, Instruction, LogicMode, VSIZE_32, VSIZE_64, VariableKey,
    VariableMap, VectorSize,
};
use vogls_utils::{Bitset, IndexSet, VgHashMap};

use crate::{HeapAlignment, HeapBuilder};

#[derive(Default)]
pub struct StackTracker {
    b1: Bitset,
    b2: Bitset,
    b4: Bitset,
    b8: Bitset,
    b16: Bitset,
    b32: Bitset,
    b64: Bitset,
}

pub struct StackOffsets {
    pub b2: usize,
    pub b4: usize,
    pub b8: usize,
    pub b16: usize,
    pub b32: usize,
    pub b64: usize,
}

impl StackTracker {
    fn get_bitset_for_size(&mut self, size: VectorSize, mode: LogicMode) -> &mut Bitset {
        match mode {
            LogicMode::TwoValue => match size.get() {
                1 => &mut self.b1,
                2 => &mut self.b2,
                3..=4 => &mut self.b4,
                5..=8 => &mut self.b8,
                9..=16 => &mut self.b16,
                17..=32 => &mut self.b32,
                _ => &mut self.b64,
            },
            LogicMode::FourValue => match size.get() {
                1 => &mut self.b2,
                2 => &mut self.b4,
                3..=4 => &mut self.b8,
                5..=8 => &mut self.b16,
                9..=16 => &mut self.b32,
                _ => &mut self.b64,
            },
        }
    }

    pub fn offsets(&self) -> StackOffsets {
        let mut bit_offset = 0usize;
        bit_offset += self.b1.len();
        bit_offset = bit_offset.next_multiple_of(2);

        let b2 = bit_offset.div_ceil(2);
        bit_offset += 2 * self.b2.len();
        bit_offset = bit_offset.next_multiple_of(4);

        let b4 = bit_offset.div_ceil(4);
        bit_offset += 4 * self.b4.len();
        bit_offset = bit_offset.next_multiple_of(8);

        let b8 = bit_offset.div_ceil(8);
        bit_offset += 8 * self.b8.len();
        bit_offset = bit_offset.next_multiple_of(8);

        let b16 = bit_offset.div_ceil(16);
        bit_offset += 16 * self.b16.len();
        bit_offset = bit_offset.next_multiple_of(16);

        let b32 = bit_offset.div_ceil(32);
        bit_offset += 32 * self.b32.len();
        bit_offset = bit_offset.next_multiple_of(32);

        let b64 = bit_offset.div_ceil(64);
        StackOffsets {
            b2,
            b4,
            b8,
            b16,
            b32,
            b64,
        }
    }

    pub fn num_words(&self) -> usize {
        self.offsets().b64 + self.b64.len()
    }

    fn clear(&mut self) {
        let Self { b1, b2, b4, b8, b16, b32, b64 } = self;
        b1.set_all_zero();
        b2.set_all_zero();
        b4.set_all_zero();
        b8.set_all_zero();
        b16.set_all_zero();
        b32.set_all_zero();
        b64.set_all_zero();
    }
}

#[derive(Clone, Copy)]
struct Interval {
    start: u64,
    end: u64,
    size: VectorSize,
    mode: LogicMode,
}

impl fmt::Debug for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Interval")
            .field(&format!("{}..{}", self.start, self.end))
            .finish()
    }
}

impl Interval {
    pub fn empty(size: VectorSize, mode: LogicMode) -> Self {
        Self {
            start: 0,
            end: 0,
            size,
            mode,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn add_range(&mut self, other: Interval) {
        debug_assert_eq!(self.size, other.size);
        debug_assert_eq!(self.mode, other.mode);
        if other.is_empty() {
            return;
        }

        if self.is_empty() {
            *self = other;
        } else {
            self.start = self.start.min(other.start);
            self.end = self.end.max(other.end);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Slot {
    Heap(u64),
    Stack(HeapAlignment, u32),
    Register(u32),
}

pub fn linear_scan_register_allocation(
    post_order_bbs: &[BasicBlockKey],
    var_map: &VariableMap,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,
    heap_builder: &mut HeapBuilder,
    assignment: &mut VgHashMap<VariableKey, Slot>,
    stack_tracker: &mut StackTracker,
    num_registers: u32,
) {
    let mut vars = IndexSet::<VariableKey>::default();

    for &bb in post_order_bbs {
        for i in &bbs[bb].instrs {
            if let Some(dst) = i.get_destination_variable() {
                vars.insert(dst);
            }
        }
    }

    let mut genset = VgHashMap::<BasicBlockKey, Bitset>::default();
    let mut killset = VgHashMap::<BasicBlockKey, Bitset>::default();
    let mut live_in = VgHashMap::<BasicBlockKey, Bitset>::default();
    genset.reserve(post_order_bbs.len());
    killset.reserve(post_order_bbs.len());
    live_in.reserve(post_order_bbs.len());

    // @Performance. We could alternatively make the bitset one large allocation and then make a
    // HashMap from BBKey -> PostOrderIndex.
    for &bb in post_order_bbs {
        let mut g = genset.entry(bb).insert(Bitset::zeroed(vars.len()));
        let mut k = killset.entry(bb).insert(Bitset::zeroed(vars.len()));
        live_in.insert(bb, Bitset::zeroed(vars.len()));

        for i in bbs[bb].instrs.iter().rev() {
            // @Performance: Never put constants smaller than 64-bits on the stack.
            if let Instruction::Constant(dst, value) = i
                && value.size() > VSIZE_64
            {
                let offset = heap_builder.claim_constant(dst.mode(), value.clone_lowering_mode());
                assignment.insert(*dst, Slot::Heap(offset.offset.bit_offset as u64));
            }
            if let Some(dst) = i.get_destination_variable() {
                k.get_mut().set(vars.get_index(&dst).unwrap(), true);
            }
            i.for_each_src(|src| {
                // Constant variables might be None here.
                if let Some(src) = vars.get_index(&src) {
                    g.get_mut().set(src, true);
                }
            });
        }
    }

    let mut changed = true;
    let mut block_live = Bitset::zeroed(vars.len());
    while changed {
        changed = false;
        for &bb in post_order_bbs {
            // @Performance: First succesors should copy into block_live instead of `fill(false)`
            // into `bitor`.
            block_live.fill(false);
            bbs[bb].terminator.for_each_non_temporal_bb(|succ| {
                if let Some(live_in) = live_in.get(&succ) {
                    block_live |= live_in;
                }
            });
            block_live |= &genset[&bb];
            block_live.andnot_mut(&killset[&bb]);
            let live_in = live_in.get_mut(&bb).unwrap();
            if live_in != &block_live {
                changed = true;
                std::mem::swap(live_in, &mut block_live);
            }
        }
    }

    drop(genset);
    drop(killset);

    let mut bb_inums = VgHashMap::<BasicBlockKey, u64>::default();
    bb_inums.reserve(post_order_bbs.len());

    let mut inum = 16u64;
    for &bb in post_order_bbs.iter().rev() {
        bb_inums.insert(bb, inum);

        inum += bbs[bb].instrs.len() as u64 * 2;
        // Add an instruction for the terminator.
        inum += 2;
    }

    let mut intervals = VgHashMap::<VariableKey, Interval>::default();
    intervals.reserve(vars.len());

    let mut live = block_live;
    for &bb in post_order_bbs {
        // @Performance: First succesors should copy into block_live instead of `fill(false)`
        // into `bitor`.
        live.fill(false);
        bbs[bb].terminator.for_each_non_temporal_bb(|succ| {
            if let Some(live_in) = live_in.get(&succ) {
                live |= live_in;
            }
        });

        let block_from = bb_inums[&bb];
        let block_to = block_from + bbs[bb].instrs.len() as u64 * 2 + 2;

        // @Q? Reference implementation contains this. I don't think this applies to us.
        // live |= block.out_vregs.map { |vreg| 1 << vreg.num }.reduce(0, :|)

        for idx in live.true_idx_iter() {
            let var = vars.get_at_index(idx).unwrap();
            let size = var_map.size(*var);
            intervals
                .entry(*var)
                .or_insert(Interval::empty(size, var.mode()))
                .add_range(Interval {
                    start: block_from,
                    end: block_to,
                    size,
                    mode: var.mode(),
                });
        }

        for (i, instr) in bbs[bb].instrs.iter().enumerate().rev() {
            let inum = block_from + i as u64 * 2;
            if let Some(dst) = instr.get_destination_variable()
 
                // Don't insert intervals for large constants.
                && !assignment.contains_key(&dst) {

                let size = var_map.size(dst);
                let interval = intervals
                    .entry(dst)
                    .or_insert(Interval::empty(size, dst.mode()));
                interval.start = inum;
                if interval.is_empty() {
                    interval.end = inum;
                }
            }
            instr.for_each_src(|src| {
                // Don't handle temporal variables.
                if assignment.contains_key(&src) {
                    return;
                }

                let size = var_map.size(src);
                let interval = intervals
                    .entry(src)
                    .or_insert(Interval::empty(size, src.mode()));
                interval.add_range(Interval {
                    start: block_from,
                    end: inum,
                    size,
                    mode: src.mode(),
                });
            });
        }

        let terminator_inum = block_to - 2;
        bbs[bb].terminator.for_each_var_src(|src| {
            // Don't handle temporal variables.
            if assignment.contains_key(&src) {
                return;
            }

            let size = var_map.size(src);
            let interval = intervals
                .entry(src)
                .or_insert(Interval::empty(size, src.mode()));
            interval.add_range(Interval {
                start: block_from,
                end: terminator_inum,
                size,
                mode: src.mode(),
            });
        });
    }

    drop(live_in);
    drop(live);

    assert!(num_registers > 0 && num_registers <= 64);

    // @Performance:
    // 1. Use a better queue here.
    // 2. Add a fast-path for the interval end being greater than the last active interval.
    let mut active_registers = 0u64;
    let mut active = VecDeque::<(usize, LogicMode, Slot)>::with_capacity(num_registers as usize);
    assignment.reserve(intervals.len());

    let mut intervals = intervals.into_iter().collect::<Vec<_>>();
    intervals.sort_unstable_by_key(|(_, i)| i.start);

    for (i, (var, interval)) in intervals.iter().enumerate() {
        // @TODO: Change for `pop_front_if`
        while active
            .front()
            .is_some_and(|&(active_interval, mode, slot)| {
                let active_interval = intervals[active_interval].1;
                if active_interval.end > interval.start {
                    false
                } else {
                    match slot {
                        Slot::Heap(_) => debug_assert!(false),
                        Slot::Register(r) => {
                            let mask = 1u64 | (u64::from(mode == LogicMode::FourValue) << 1);
                            active_registers ^= mask << r
                        }
                        Slot::Stack(kind, offset) => {
                            let offset = offset as usize;

                            use HeapAlignment as K;
                            match kind {
                                K::B1 => stack_tracker.b1.set(offset, false),
                                K::B2 => stack_tracker.b2.set(offset, false),
                                K::B4 => stack_tracker.b4.set(offset, false),
                                K::B8 => stack_tracker.b8.set(offset, false),
                                K::B16 => stack_tracker.b16.set(offset, false),
                                K::B32 => stack_tracker.b32.set(offset, false),
                                K::B64 => {
                                    let mut num_words =
                                        active_interval.size.get().div_ceil(64) as usize;
                                    if mode == LogicMode::FourValue {
                                        num_words *= 2;
                                    }
                                    stack_tracker
                                        .b64
                                        .set_slice_constant(offset, num_words, false);
                                }
                            }
                        }
                    }
                    true
                }
            })
        {
            _ = active.pop_front();
        }

        // Everything that is larger than 64-bits (both two-value and four-value) lives on the
        // stack only. We never put it in a register! The address to the stack-address will live in
        // a temporary register.
        if interval.size > VSIZE_64 {
            let bitset = stack_tracker.get_bitset_for_size(interval.size, interval.mode);
            let num_bitset_slots = num_bitset_slots(interval.size, interval.mode);
            let offset = match bitset.find_n_contiguous_zeros(num_bitset_slots) {
                Ok(offset) => offset,
                Err(offset) => {
                    debug_assert!(offset <= bitset.len());
                    bitset.extend_zeroed(num_bitset_slots - (bitset.len() - offset));
                    offset
                }
            };
            bitset.set_slice_constant(offset, num_bitset_slots, true);
            let offset = offset.try_into().expect("Too large");
            let slot = Slot::Stack(HeapAlignment::new(interval.size, interval.mode), offset);
            assignment.insert(*var, slot);
            continue;
        }

        if let Some(register) =
            claim_register_slot(num_registers, &mut active_registers, interval.mode)
        {
            let slot = Slot::Register(register);
            assignment.insert(*var, slot);

            let insert_idx =
                active.binary_search_by_key(&interval.end, |(i, _, _)| intervals[*i].1.end);
            let insert_idx = insert_idx.unwrap_or_else(|i| i);
            active.insert(insert_idx, (i, interval.mode, slot));
        } else {
            let (spill_i, _mode, _spill) = active.back().unwrap();

            // @Performance. Better spilling policy.
            let (spill_var, spill_interval) = intervals[*spill_i];
            if spill_interval.end > interval.end {
                let bitset = stack_tracker.get_bitset_for_size(spill_interval.size, spill_interval.mode);
                let num_bitset_slots = num_bitset_slots(spill_interval.size, spill_interval.mode);
                let offset = match bitset.find_n_contiguous_zeros(num_bitset_slots) {
                    Ok(offset) => offset,
                    Err(offset) => {
                        debug_assert!(offset <= bitset.len());
                        bitset.extend_zeroed(bitset.len() - offset + num_bitset_slots);
                        offset
                    }
                };
                bitset.set_slice_constant(offset as usize, num_bitset_slots, true);
                let offset = offset.try_into().expect("Too large");
                let slot = Slot::Stack(HeapAlignment::new(spill_interval.size, spill_interval.mode), offset);
                let slot = assignment.insert(spill_var, slot).unwrap();
                assignment.insert(*var, slot);
                active.pop_back();

                let insert_idx =
                    active.binary_search_by_key(&interval.end, |(i, _, _)| intervals[*i].1.end);
                let insert_idx = insert_idx.unwrap_or_else(|i| i);
                active.insert(insert_idx, (i, interval.mode, slot));
            } else {
                let bitset = stack_tracker.get_bitset_for_size(interval.size, interval.mode);
                let num_bitset_slots = num_bitset_slots(interval.size, interval.mode);
                let offset = match bitset.find_n_contiguous_zeros(num_bitset_slots) {
                    Ok(offset) => offset,
                    Err(offset) => {
                        debug_assert!(offset <= bitset.len());
                        bitset.extend_zeroed(bitset.len() - offset + num_bitset_slots);
                        offset
                    }
                };
                bitset.set_slice_constant(offset as usize, num_bitset_slots, true);
                let offset = offset.try_into().expect("Too large");
                let slot = Slot::Stack(HeapAlignment::new(interval.size, interval.mode), offset);
                assignment.insert(*var, slot);
            }
        }
    }

    stack_tracker.clear();
}

fn claim_register_slot(
    num_registers: u32,
    active_registers: &mut u64,
    mode: LogicMode,
) -> Option<u32> {
    if active_registers.count_ones() == num_registers {
        return None;
    }

    match mode {
        LogicMode::TwoValue => {
            let register = active_registers.trailing_ones();
            *active_registers |= 1u64 << register;
            Some(register)
        }
        LogicMode::FourValue => {
            // Each four-value logic variable is put into two subsequent registers.
            let subsequent_inactive_register_mask = !(*active_registers | (*active_registers >> 1));
            let register = subsequent_inactive_register_mask.trailing_zeros();
            if register >= num_registers - 1 {
                return None;
            }
            *active_registers |= 3u64 << register;
            Some(register)
        }
    }
}

fn num_bitset_slots(size: VectorSize, mode: LogicMode) -> usize {
    match mode {
        LogicMode::TwoValue if size <= VSIZE_64 => 1,
        LogicMode::TwoValue => size.get().div_ceil(64) as usize,
        LogicMode::FourValue if size <= VSIZE_32 => 1,
        LogicMode::FourValue => 2 * size.get().div_ceil(64) as usize,
    }
}
