use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::Arc;

use slotmap::{SlotMap, new_key_type};
use vogls_bits::arithmetic::{FvLogicValue, fv_set_no_special};
use vogls_bits::set_subslice::{tv_l_set, tv_s_set};
use vogls_codegen::{Heap, HeapOffset, HeapRef};
use vogls_ir::{GlobalContext, INTEGER_VSIZE, LogicMode, Mode, TIME_VSIZE, VectorSize};
use vogls_runtime::plugins::RuntimePluginState;
use vogls_runtime::{RtSignalKey, SimulationIo};

mod execution;
mod instruction;
mod plugin;

pub use plugin::InstructionPlugin;

pub use instruction::*;
use vogls_utils::VgHashMap;

new_key_type! { pub struct ListenerKey; }

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VmProcessKey(pub u64);

#[derive(Clone)]
pub struct Regions {
    pub active: Vec<Event>,
    pub other: Vec<Vec<Event>>,
}

impl Regions {
    pub fn new(num_additional_regions: usize) -> Self {
        Self {
            active: Vec::new(),
            other: vec![Vec::new(); num_additional_regions],
        }
    }

    pub fn num_additional_regions(&self) -> usize {
        self.other.len()
    }
}

pub type Timestamp = u64;
pub type InstanceId = u64;

#[derive(Clone, Debug)]
pub struct Event {
    /// Which process is scheduled.
    pub process: VmProcessKey,
    /// Where to start execution.
    pub ip: usize,
}

#[derive(Debug)]
pub struct ScheduledEvent {
    pub at: Timestamp,
    pub event: Event,
}

impl PartialEq for ScheduledEvent {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at
    }
}
impl Eq for ScheduledEvent {}
impl PartialOrd for ScheduledEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.at.partial_cmp(&self.at)
    }
}
impl Ord for ScheduledEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other.at.cmp(&self.at)
    }
}

enum EvalOutcome {
    Next,
    Error,
    Exit,
}

fn update_watchers(
    sig: RtSignalKey,
    watches: &mut [Vec<ListenerKey>],
    listeners: &mut SlotMap<ListenerKey, Event>,
    regions: &mut Regions,
) {
    let watchers = &mut watches[sig.as_usize()];
    for watcher in watchers.iter() {
        if let Some(event) = listeners.remove(*watcher) {
            regions.active.push(event);
        }
    }
    watchers.clear();
}

pub fn drive_bits(
    heap: &mut Heap,
    dst: HeapRef,
    src: HeapRef,
    dst_limit: VectorSize,
    partial: Option<u32>,
    logic_mode: LogicMode,
) -> bool {
    if partial.is_some() || src.size < dst.size || dst_limit < dst.size {
        let partial = partial.unwrap_or(0);
        let Some(rem_dst_size) = VectorSize::new(dst_limit.get().saturating_sub(partial)) else {
            return false;
        };

        return match logic_mode {
            LogicMode::TwoValue if dst.size < Heap::TV_U64_MIN_SIZE => {
                let old_val = heap.get_tv_u64(dst);
                let mut src_val = heap.get_tv_u64(src);
                let size = src.size.min(rem_dst_size);
                if rem_dst_size < src.size {
                    src_val &= 1u64.unbounded_shl(rem_dst_size.get()).wrapping_sub(1);
                }
                let new_val = tv_s_set(old_val, src_val, dst.size, partial, size);
                heap.set_tv_u64(dst, new_val);
                old_val != new_val
            }
            LogicMode::TwoValue => {
                let mut src_s = [0u64];
                let (dst_s, src_s) = if src.size < Heap::TV_U64_MIN_SIZE {
                    src_s[0] = heap.get_tv_u64(src);
                    (
                        heap.get_mut_u64_slice(dst.offset, dst.size.get().div_ceil(64) as usize),
                        &src_s[..],
                    )
                } else {
                    let dst_nwords = dst.size.get().div_ceil(64) as usize;
                    let src_nwords = src.size.get().div_ceil(64) as usize;
                    heap.get_disjoint_u64_dst_src(
                        (dst.offset, dst_nwords),
                        (src.offset, src_nwords),
                    )
                };

                tv_l_set(dst_s, src_s, dst.size, partial, src.size)
            }
            LogicMode::FourValue if dst.size < Heap::FV_U64_MIN_SIZE => {
                let (old_spc, old_val) = heap.get_fv_u64(dst);
                let (src_spc, src_val) = heap.get_fv_u64(src);
                let new_spc = tv_s_set(old_spc, src_spc, dst.size, partial, src.size);
                let new_val = tv_s_set(old_val, src_val, dst.size, partial, src.size);
                heap.set_fv_u64(dst, new_spc, new_val);
                old_spc != new_spc || old_val != new_val
            }
            LogicMode::FourValue => {
                let mut src_s = [0u64, 0u64];
                let dst_nwords = dst.size.get().div_ceil(64) as usize;
                let (dst_s, src_s) = if src.size < Heap::FV_U64_MIN_SIZE {
                    (src_s[0], src_s[1]) = heap.get_fv_u64(src);
                    (
                        heap.get_mut_u64_slice(dst.offset, 2 * dst_nwords),
                        &src_s[..],
                    )
                } else {
                    let src_nwords = src.size.get().div_ceil(64) as usize;
                    heap.get_disjoint_u64_dst_src(
                        (dst.offset, 2 * dst_nwords),
                        (src.offset, 2 * src_nwords),
                    )
                };

                let mut updated = false;
                updated |= tv_l_set(
                    &mut dst_s[..dst_nwords],
                    &src_s[..src_s.len() / 2],
                    dst.size,
                    partial,
                    src.size,
                );
                updated |= tv_l_set(
                    &mut dst_s[dst_nwords..],
                    &src_s[src_s.len() / 2..],
                    dst.size,
                    partial,
                    src.size,
                );
                updated
            }
        };
    }

    let size = dst.size;
    match logic_mode {
        LogicMode::TwoValue if size.get() <= 32 => {
            let src = heap.get_tv_u64(src);
            let dst = heap.set_tv_u64(dst, src);
            dst != src
        }
        LogicMode::FourValue if size.get() <= 16 => {
            let (spc, val) = heap.get_fv_u64(src);
            let (dspc, dval) = heap.set_fv_u64(dst, spc, val);
            dspc != spc || val != dval
        }
        LogicMode::TwoValue | LogicMode::FourValue => {
            let mut nwords = size.get().div_ceil(64) as usize;
            if logic_mode == LogicMode::FourValue {
                nwords *= 2;
            }

            let (dst, src) =
                heap.get_disjoint_u64_dst_src((dst.offset, nwords), (src.offset, nwords));
            let mut updated = false;
            for i in 0..nwords {
                updated |= dst[i] != src[i];
                dst[i] = src[i];
            }
            updated
        }
    }
}

pub struct Simulation {
    pub processes: Vec<VmProcess>,
    pub signals: Arc<[HeapRef]>,
    pub lupdt_indexes: VgHashMap<RtSignalKey, u64>,
    pub logic_mode: LogicMode,
    pub itrace: bool,
}

impl Simulation {
    pub fn new(
        processes: Vec<VmProcess>,
        signals: Arc<[HeapRef]>,
        lupdt_indexes: VgHashMap<RtSignalKey, u64>,
        logic_mode: LogicMode,
    ) -> Self {
        Self {
            processes,
            signals,
            lupdt_indexes,
            logic_mode,
            itrace: false,
        }
    }

    pub fn new_state(
        &self,
        gl: &GlobalContext,
        regions: Regions,
        listeners: SlotMap<ListenerKey, Event>,
        watches: Vec<Vec<ListenerKey>>,
        heap: Heap,
        lupdt_updated: &[bool],
    ) -> SimulationState {
        SimulationState {
            schedule: BTreeMap::<Timestamp, Vec<Event>>::new(),
            runtime: vogls_runtime::RuntimeState::new(
                gl.logic_mode,
                heap,
                gl.signals.len(),
                lupdt_updated,
            ),
            regions,
            listeners,
            watches,
            plugins: Vec::new(),
            iplugins: Vec::new(),
        }
    }

    pub fn run(
        &self,
        state: &mut SimulationState,
        io: &mut SimulationIo,
        max_time: u64,
    ) -> Result<(), ()> {
        'region_loop: loop {
            while let Some(event) = state.regions.active.pop() {
                state.runtime.event_count += 1;
                let outcome = self.evaluate_event(io, state, event);
                if self.itrace {
                    eprintln!();
                }

                match outcome {
                    EvalOutcome::Next => continue,
                    EvalOutcome::Error => return Err(()),
                    EvalOutcome::Exit => break 'region_loop,
                }
            }

            for (i, region) in state.regions.other.iter_mut().enumerate() {
                if !region.is_empty() {
                    if self.itrace {
                        eprintln!("next region: {i}");
                    }
                    std::mem::swap(&mut state.regions.active, region);
                    continue 'region_loop;
                }
            }

            for plugin in state.plugins.iter_mut() {
                plugin.timestep(&mut state.runtime);
            }

            let Some((at, events)) = state.schedule.pop_first() else {
                break;
            };

            if at.wrapping_add(1) < state.runtime.time {
                eprintln!("Time overflow!");
                return Err(());
            }

            if at > max_time {
                state.runtime.time = max_time;
                state.schedule.insert(at, events);
                break;
            }

            if self.itrace {
                eprintln!("next timestep: {at}");
            }

            state.runtime.time = at;
            state.regions.active = events;
        }

        for plugin in state.plugins.iter_mut() {
            plugin.finish(&mut state.runtime);
        }

        Ok(())
    }

    fn evaluate_event(
        &self,
        io: &mut SimulationIo,
        state: &mut SimulationState,
        mut event: Event,
    ) -> EvalOutcome {
        let Event {
            process: process_key,
            ip,
        } = &mut event;

        let process = &self.processes[process_key.0 as usize];

        loop {
            let instr = &process.instructions[*ip];

            if !state.iplugins.is_empty() {
                let mut iplugins = std::mem::take(&mut state.iplugins);
                for p in iplugins.iter_mut() {
                    p.as_mut().instruction(self, state, instr);
                }
                state.iplugins = iplugins;
            }

            *ip += 1;
            state.runtime.instruction_count += 1;

            let outcome = 'instruction: {
                use VmInstruction as I;
                match instr {
                    I::TvMove1(dst, src) => {
                        execution::tv::exec_tv_mov1(&mut state.runtime.heap, *dst, *src)
                    }
                    I::TvDwDwMove(dst, src) => {
                        execution::tv::exec_tv_dwdwmov64m(&mut state.runtime.heap, *dst, *src)
                    }
                    I::TvDwSwMove(dst, src) => {
                        execution::tv::exec_tv_dwswmov64m(&mut state.runtime.heap, *dst, *src)
                    }
                    I::TvSwDwMove(dst, src) => {
                        execution::tv::exec_tv_swdwmov64m(&mut state.runtime.heap, *dst, *src)
                    }
                    I::TvSwSwMove(dst, src) => {
                        execution::tv::exec_tv_swswmov64m(&mut state.runtime.heap, *dst, *src)
                    }

                    I::TvNot1(dst, src) => {
                        execution::tv::exec_tv_not1(&mut state.runtime.heap, *dst, *src)
                    }
                    I::TvAnd1(dst, lhs, rhs) => {
                        execution::tv::exec_tv_and1(&mut state.runtime.heap, *dst, *lhs, *rhs)
                    }
                    I::TvOr1(dst, lhs, rhs) => {
                        execution::tv::exec_tv_or1(&mut state.runtime.heap, *dst, *lhs, *rhs)
                    }
                    I::TvXor1(dst, lhs, rhs) => {
                        execution::tv::exec_tv_xor1(&mut state.runtime.heap, *dst, *lhs, *rhs)
                    }
                    I::TvXnor1(dst, lhs, rhs) => {
                        execution::tv::exec_tv_xnor1(&mut state.runtime.heap, *dst, *lhs, *rhs)
                    }
                    I::TvOrNot1(dst, lhs, rhs) => {
                        execution::tv::exec_tv_ornot1(&mut state.runtime.heap, *dst, *lhs, *rhs)
                    }
                    I::TvAndNot1(dst, lhs, rhs) => {
                        execution::tv::exec_tv_andnot1(&mut state.runtime.heap, *dst, *lhs, *rhs)
                    }
                    I::TvZeroExtend1(dst, src) => {
                        execution::tv::exec_tv_zeroextend1(&mut state.runtime.heap, *dst, *src)
                    }
                    I::TvSignExtend1(dst, src) => {
                        execution::tv::exec_tv_signextend1(&mut state.runtime.heap, *dst, *src)
                    }
                    I::TvSelect1(dst, cond, truthy, falsy) => execution::tv::exec_tv_select1(
                        &mut state.runtime.heap,
                        *dst,
                        *cond,
                        *truthy,
                        *falsy,
                    ),

                    I::TvUnary(dst, op, src) => {
                        execution::tv::exec_tv_unary(&mut state.runtime.heap, *dst, *op, *src)
                    }
                    I::TvResize(dst, op, src) => {
                        execution::tv::exec_tv_resize(&mut state.runtime.heap, *dst, *op, *src)
                    }
                    I::TvBinaryArithmetic(dst, op, lhs, rhs) => execution::tv::exec_tv_bin_arith(
                        &mut state.runtime.heap,
                        *dst,
                        *op,
                        *lhs,
                        *rhs,
                    ),
                    I::TvBinaryComparison(dst, op, lhs, rhs) => execution::tv::exec_tv_bin_cmp(
                        &mut state.runtime.heap,
                        *dst,
                        *op,
                        *lhs,
                        *rhs,
                    ),
                    I::TvEdge(dst, op, lhs, rhs) => {
                        execution::tv::exec_tv_edge(&mut state.runtime.heap, *dst, *op, *lhs, *rhs)
                    }
                    I::TvShift(dst, op, src, offset) => execution::tv::exec_tv_shift(
                        &mut state.runtime.heap,
                        *dst,
                        *op,
                        *src,
                        *offset,
                    ),
                    I::TvShiftImm(dst, op, src, offset) => execution::tv::exec_tv_shift_imm(
                        &mut state.runtime.heap,
                        *dst,
                        *op,
                        *src,
                        *offset,
                    ),
                    I::TvSlice(dst, src, offset, fill_with_x) => execution::tv::exec_tv_slice(
                        &mut state.runtime.heap,
                        *dst,
                        *src,
                        *offset,
                        *fill_with_x,
                    ),
                    I::TvSliceImm(dst, src, offset) => execution::tv::exec_tv_slice_imm(
                        &mut state.runtime.heap,
                        *dst,
                        *src,
                        *offset,
                        false,
                    ),
                    I::TvConcat(dst, lhs, rhs) => {
                        execution::tv::exec_tv_concat(&mut state.runtime.heap, *dst, *lhs, *rhs)
                    }
                    I::TvSelect(dst, cond, truthy, falsy, cond_is_fv) => {
                        execution::tv::exec_tv_select(
                            &mut state.runtime.heap,
                            *dst,
                            *cond,
                            *truthy,
                            *falsy,
                            *cond_is_fv,
                        )
                    }

                    I::FvUnary(dst, op, src) => {
                        execution::fv::exec_fv_unary(&mut state.runtime.heap, *dst, *op, *src)
                    }
                    I::FvResize(dst, op, src) => {
                        execution::fv::exec_fv_resize(&mut state.runtime.heap, *dst, *op, *src)
                    }
                    I::FvBinaryArithmetic(dst, op, lhs, rhs) => execution::fv::exec_fv_bin_arith(
                        &mut state.runtime.heap,
                        *dst,
                        *op,
                        *lhs,
                        *rhs,
                    ),
                    I::FvBinaryComparison(dst, op, lhs, rhs) => execution::fv::exec_fv_bin_cmp(
                        &mut state.runtime.heap,
                        *dst,
                        *op,
                        *lhs,
                        *rhs,
                    ),
                    I::FvEdge(dst, op, lhs, rhs) => {
                        execution::fv::exec_fv_edge(&mut state.runtime.heap, *dst, *op, *lhs, *rhs)
                    }
                    I::FvShift(dst, op, src, offset) => execution::fv::exec_fv_shift(
                        &mut state.runtime.heap,
                        *dst,
                        *op,
                        *src,
                        *offset,
                    ),
                    I::FvShiftImm(dst, op, src, offset) => execution::fv::exec_fv_shift_imm(
                        &mut state.runtime.heap,
                        *dst,
                        *op,
                        *src,
                        *offset,
                    ),
                    I::FvSlice(dst, src, offset, flags) => execution::fv::exec_fv_slice(
                        &mut state.runtime.heap,
                        *dst,
                        *src,
                        *offset,
                        *flags,
                    ),
                    I::FvSliceImm(dst, src, offset) => execution::fv::exec_fv_slice_imm(
                        &mut state.runtime.heap,
                        *dst,
                        *src,
                        *offset,
                        false,
                    ),
                    I::FvConcat(dst, lhs, rhs) => {
                        execution::fv::exec_fv_concat(&mut state.runtime.heap, *dst, *lhs, *rhs)
                    }
                    I::FvSelect(dst, cond, truthy, falsy, cond_is_fv) => {
                        execution::fv::exec_fv_select(
                            &mut state.runtime.heap,
                            *dst,
                            *cond,
                            *truthy,
                            *falsy,
                            *cond_is_fv,
                        )
                    }

                    I::TvToFv(dst, src) => {
                        let size = dst.size;
                        if size.get() <= 32 {
                            let v = state.runtime.heap.get_tv_u64(src.to_ref(size));
                            state.runtime.heap.set_fv_u64(
                                *dst,
                                1u64.unbounded_shl(size.get()).wrapping_sub(1),
                                v,
                            );
                        } else {
                            let nwords = size.get().div_ceil(64) as usize;
                            let (dst, src) = state
                                .runtime
                                .heap
                                .get_disjoint_u64_dst_src((dst.offset, nwords * 2), (*src, nwords));
                            fv_set_no_special(dst, size);
                            dst[nwords..].copy_from_slice(src);
                        }
                    }
                    I::FvToTv(dst, src) => {
                        let size = dst.size;
                        if size.get() <= 32 {
                            let (spc, val) = state.runtime.heap.get_fv_u64(src.to_ref(size));
                            state.runtime.heap.set_tv_u64(*dst, spc & val);
                        } else {
                            let nwords = size.get().div_ceil(64) as usize;
                            let (dst, src) = state
                                .runtime
                                .heap
                                .get_disjoint_u64_dst_src((dst.offset, nwords), (*src, nwords * 2));
                            for i in 0..nwords {
                                dst[i] = src[i] & src[nwords + i];
                            }
                        }
                    }

                    I::Intrinsic(dst, op, args) => {
                        use VmIntrinsicOp as O;

                        match op.as_ref() {
                            O::Display(f) => {
                                f.write_to(
                                    &mut io.stdout,
                                    args.iter().map(|(sr, lm)| match lm {
                                        LogicMode::TwoValue => state.runtime.heap.load_tv_bits(*sr),
                                        LogicMode::FourValue => {
                                            state.runtime.heap.load_fv_bits(*sr)
                                        }
                                    }),
                                )
                                .unwrap();
                            }
                            O::Assert(f) => {
                                let (cond_sr, cond_lm) = args[0];
                                let condition = match cond_lm {
                                    LogicMode::TwoValue => {
                                        state.runtime.heap.get_tv_bool(cond_sr.offset)
                                    }
                                    LogicMode::FourValue => {
                                        state.runtime.heap.get_fv_item(cond_sr.offset)
                                            == FvLogicValue::L1
                                    }
                                };

                                if !condition {
                                    f.write_to(
                                        &mut io.stdout,
                                        args[1..].iter().map(|(sr, lm)| match lm {
                                            LogicMode::TwoValue => {
                                                state.runtime.heap.load_tv_bits(*sr)
                                            }
                                            LogicMode::FourValue => {
                                                state.runtime.heap.load_fv_bits(*sr)
                                            }
                                        }),
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }
                            }
                            O::VcdOpenFile(path) => {
                                let vcd = (state.plugins[0].as_mut() as &mut dyn std::any::Any)
                                    .downcast_mut::<vogls_vcd::RtVcdOutput>()
                                    .unwrap();
                                if !vcd.children.is_empty() {
                                    writeln!(&mut io.stderr, "ERR! VCD opened a second file")
                                        .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }

                                vcd.writer = Box::new(std::fs::File::create(path).unwrap());
                            }
                            O::VcdAppendModule(children, map) => {
                                let vcd = (state.plugins[0].as_mut() as &mut dyn std::any::Any)
                                    .downcast_mut::<vogls_vcd::RtVcdOutput>()
                                    .unwrap();

                                if vcd.start_ts != state.runtime.time {
                                    writeln!(
                                        &mut io.stderr,
                                        "ERR! Dumping vars over several simulation times"
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }

                                for child in children {
                                    child.extend_into(
                                        &mut vcd.tracked,
                                        &mut vcd.updated_this_time_step,
                                    );
                                }
                                vcd.children = children.clone();
                                vcd.map = map.clone();
                            }
                            O::VcdPause => {
                                let vcd = (state.plugins[0].as_mut() as &mut dyn std::any::Any)
                                    .downcast_mut::<vogls_vcd::RtVcdOutput>()
                                    .unwrap();
                                _ = vcd.paused = true;
                            }
                            O::VcdResume => {
                                let vcd = (state.plugins[0].as_mut() as &mut dyn std::any::Any)
                                    .downcast_mut::<vogls_vcd::RtVcdOutput>()
                                    .unwrap();
                                vcd.paused = false;
                            }
                            O::Time => {
                                _ = state
                                    .runtime
                                    .heap
                                    .set_tv_u64(dst.to_ref(TIME_VSIZE), state.runtime.time)
                            }
                            O::Random => {
                                _ = state
                                    .runtime
                                    .heap
                                    .set_tv_u64(dst.to_ref(INTEGER_VSIZE), state.runtime.time)
                            }
                            O::Finish => {
                                writeln!(&mut io.stdout, "[FINISH]").unwrap();
                                break 'instruction Some(EvalOutcome::Exit);
                            }
                            O::ReadMem(heap_ref, readmem) => vogls_runtime::readmem::read_mem(
                                &readmem.path,
                                state.runtime.heap.0.as_mut(),
                                *heap_ref,
                                if self.logic_mode == LogicMode::TwoValue {
                                    Mode::TwoValue
                                } else {
                                    Mode::FourValue
                                },
                                readmem.offset,
                                readmem.limit,
                                readmem.stride,
                                readmem.binary,
                            )
                            .unwrap(),
                        }
                    }
                    I::LastUpdateTime(dst, signal) => {
                        let idx = self.lupdt_indexes[signal];
                        let lupdt = state.runtime.last_active_time[idx as usize];
                        state.runtime.heap.set_tv_u64(dst.to_ref(TIME_VSIZE), lupdt);
                    }
                    I::Drive(sig, src, offset) => {
                        let dst = self.signals[sig.as_usize()];
                        let (dst_limit, partial) = match (offset, self.logic_mode) {
                            (None, _) => (dst.size, None),
                            (Some((offset, mask_size)), LogicMode::TwoValue) => (
                                *mask_size,
                                Some(state.runtime.heap.load_exact_tv_u32(*offset)),
                            ),
                            (Some((offset, mask_size)), LogicMode::FourValue) => {
                                let (spc, val) = state.runtime.heap.load_exact_fv_u32(*offset);
                                if !spc != 0 {
                                    break 'instruction None;
                                }
                                (*mask_size, Some(val))
                            }
                        };

                        let mut updated = drive_bits(
                            &mut state.runtime.heap,
                            dst,
                            *src,
                            dst_limit,
                            partial,
                            self.logic_mode,
                        );

                        if matches!(self.logic_mode, LogicMode::TwoValue)
                            && let w = &mut state.runtime.tvl_first_write[sig.as_usize() / 64]
                            && ((*w >> (sig.as_usize() % 64)) & 1) == 0
                        {
                            updated = true;
                            *w |= 1u64 << (sig.as_usize() % 64);
                        }

                        if updated {
                            self.update_signal(state, *sig);
                        }
                    }
                    I::TvVariableWait(time) | I::FvVariableWait(time) => {
                        let time = if matches!(instr, I::TvVariableWait(_)) {
                            state.runtime.heap.get_tv_u64(time.to_ref(TIME_VSIZE))
                        } else {
                            let (spc, val) = state.runtime.heap.get_fv_u64(time.to_ref(TIME_VSIZE));
                            assert_eq!(spc, u64::MAX, "variable wait with four-value logic");
                            val
                        };
                        if time > 0 {
                            state
                                .schedule
                                .entry(state.runtime.time.wrapping_add(time))
                                .or_default()
                                .push(event);
                            if self.itrace {
                                instr.itrace(
                                    &mut state.runtime.heap,
                                    &self.signals,
                                    self.logic_mode,
                                );
                            }
                            return EvalOutcome::Next;
                        }
                    }
                    I::Wait(time) => {
                        if time.0 > 0 {
                            state
                                .schedule
                                .entry(state.runtime.time.wrapping_add(time.0))
                                .or_default()
                                .push(event);
                            if self.itrace {
                                instr.itrace(
                                    &mut state.runtime.heap,
                                    &self.signals,
                                    self.logic_mode,
                                );
                            }
                            return EvalOutcome::Next;
                        }
                    }
                    I::WaitRegion(region) => {
                        if *region == 0 {
                            state.regions.active.push(event);
                        } else {
                            state.regions.other[*region as usize - 1].push(event);
                        }
                        if self.itrace {
                            instr.itrace(&mut state.runtime.heap, &self.signals, self.logic_mode);
                        }
                        return EvalOutcome::Next;
                    }
                    I::Watch(watch_signals) => {
                        let listener_key = state.listeners.insert(event);
                        for signal in watch_signals {
                            state.watches[signal.as_usize()].push(listener_key);
                        }
                        if self.itrace {
                            instr.itrace(&mut state.runtime.heap, &self.signals, self.logic_mode);
                        }
                        return EvalOutcome::Next;
                    }

                    I::Jump(offset) => *ip = *offset,
                    I::TvBranch(cond, true_offset, false_offset) => {
                        let is_true = state.runtime.heap.get_tv_bool(*cond);
                        if is_true {
                            *ip = *true_offset;
                        } else {
                            *ip = *false_offset;
                        }
                    }
                    I::FvBranch(cond, true_offset, false_offset) => {
                        let is_true = state.runtime.heap.get_fv_item(*cond) == FvLogicValue::L1;
                        if is_true {
                            *ip = *true_offset;
                        } else {
                            *ip = *false_offset;
                        }
                    }
                    I::Halt => {
                        break 'instruction Some(EvalOutcome::Next);
                    }
                }

                None
            };

            if self.itrace {
                instr.itrace(&mut state.runtime.heap, &self.signals, self.logic_mode);
            }

            if let Some(outcome) = outcome {
                return outcome;
            }
        }
    }

    pub fn update_signal(&self, state: &mut SimulationState, signal: RtSignalKey) {
        update_watchers(
            signal,
            &mut state.watches,
            &mut state.listeners,
            &mut state.regions,
        );
        for plugin in state.plugins.iter_mut() {
            plugin.poke_signal(signal);
        }
        if let Some(lupdt_idx) = self.lupdt_indexes.get(&signal) {
            state.runtime.last_active_time[*lupdt_idx as usize] = state.runtime.time;
        }
    }

    pub fn drive_bits(
        &self,
        state: &mut SimulationState,
        signal: RtSignalKey,
        value: &vogls_ir::Bits,
    ) {
        let heap_ref = self.signals[signal.as_usize()];
        let updated = &state.runtime.heap.load_bits(heap_ref, self.logic_mode) != value;

        if updated {
            state
                .runtime
                .heap
                .store_bits(heap_ref, self.logic_mode, value);
            self.update_signal(state, signal);
        }
    }

    pub fn poke_signal(&self, state: &mut SimulationState, signal: RtSignalKey) {
        self.update_signal(state, signal);
    }
}

pub struct SimulationState {
    pub schedule: BTreeMap<Timestamp, Vec<Event>>,
    pub runtime: vogls_runtime::RuntimeState,

    pub regions: Regions,
    pub listeners: SlotMap<ListenerKey, Event>,
    pub watches: Vec<Vec<ListenerKey>>,
    pub plugins: Vec<RuntimePluginState>,
    pub iplugins: Vec<plugin::InstructionPluginState>,
}

impl Clone for SimulationState {
    fn clone(&self) -> Self {
        Self {
            schedule: self.schedule.clone(),
            runtime: self.runtime.clone(),

            regions: self.regions.clone(),
            listeners: self.listeners.clone(),
            watches: self.watches.clone(),
            plugins: self.plugins.iter().map(|p| p.as_ref().clone()).collect(),
            iplugins: vec![],
        }
    }
}

impl SimulationState {}
