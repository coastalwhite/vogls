use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Alignment;
use std::num::NonZeroUsize;
use std::path::Path;

use slotmap::{SlotMap, new_key_type};
use vogls_bits::arithmetic::{FvLogicValue, fv_set_no_special};
use vogls_bits::format::{BitsFormatBase, BitsFormatOptions};
use vogls_bits::set_subslice::{tv_l_set, tv_s_set};
use vogls_codegen::{Heap, HeapOffset, HeapRef};
use vogls_ir::vcd::NetType;
use vogls_ir::{INTEGER_VSIZE, LogicMode, SignalAlias, SignalKey, TIME_VSIZE, VectorSize};
use vogls_runtime::{RtSignalKey, SimulationIo};

mod execution;
mod instruction;
mod plugin;

pub use plugin::{InstructionPlugin, Plugin};

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
    stack: &Heap,
    signals: &[HeapRef],
    watches: &mut [Vec<ListenerKey>],
    listeners: &mut SlotMap<ListenerKey, Event>,
    regions: &mut Regions,
    trace: Option<&mut vogls_trace::Trace>,
) {
    let start = regions.active.len();
    let watchers = &mut watches[sig.as_usize()];
    for watcher in watchers.iter() {
        if let Some(event) = listeners.remove(*watcher) {
            regions.active.push(event);
        }
    }
    watchers.clear();

    if let Some(trace) = trace {
        let woken_start = trace.woken.len() as u64;
        trace
            .woken
            .extend(regions.active[start..].iter().map(|e| e.process.0));
        let woken_range = woken_start..trace.woken.len() as u64;
        trace.driven.push((
            sig.as_u64(),
            stack.load_tv_bits(signals[sig.as_usize()]),
            woken_range,
        ));
    }
}

pub fn drive_bits(
    heap: &mut Heap,
    dst: HeapRef,
    src: HeapRef,
    partial: Option<u32>,
    logic_mode: LogicMode,
) -> bool {
    debug_assert!(dst.size >= src.size);
    if partial.is_some() {
        let partial = partial.unwrap_or(0);

        return match logic_mode {
            LogicMode::TwoValue if dst.size < Heap::TV_U64_MIN_SIZE => {
                let old_val = heap.get_tv_u64(dst);
                let src_val = heap.get_tv_u64(src);
                let new_val = tv_s_set(old_val, src_val, dst.size, partial, src.size);
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

#[derive(Debug, Clone)]
pub struct VcdScope {
    pub name: String,
    pub items: Vec<VcdScopeItem>,
}

impl VcdScope {
    pub fn lower(
        v: &vogls_ir::vcd::VcdScope,
        map: &VgHashMap<SignalKey, RtSignalKey>,
        signal_aliases: &VgHashMap<SignalKey, SignalAlias>,
    ) -> VcdScope {
        VcdScope {
            name: v.name.clone(),
            items: v
                .items
                .iter()
                .map(|i| VcdScopeItem::lower(i, map, signal_aliases))
                .collect(),
        }
    }

    fn write_to(&self, f: &mut impl std::io::Write) -> std::io::Result<()> {
        let Self { name, items } = self;
        write!(f, "$scope module ")?;
        if name.contains(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            f.write_all(b"\\")?;
        }
        f.write_all(name.as_bytes())?;
        if name.trim().is_empty() {
            f.write_all(b"<anon>")?;
        }
        writeln!(f, " $end")?;
        for item in items {
            item.write_to(f)?;
        }
        writeln!(f, "$upscope $end")?;
        Ok(())
    }

    fn extend_into(
        &self,
        tracked: &mut VgHashMap<RtSignalKey, Option<NonZeroUsize>>,
        values: &mut Vec<RtSignalKey>,
    ) {
        for i in &self.items {
            i.extend_into(tracked, values);
        }
    }
}

impl VcdScopeItem {
    fn write_to(&self, f: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            VcdScopeItem::Scope(scope) => scope.write_to(f),
            VcdScopeItem::Variable(k) => {
                let VcdVariable {
                    name,
                    signal: _,
                    ty,
                    msb,
                    lsb,
                } = k;
                let size = VectorSize::new((msb.abs_diff(*lsb) + 1) as u32).unwrap();
                let idx = k.signal.as_u64();
                write!(f, "$var ")?;
                f.write_all(
                    match ty {
                        NetType::Integer => "integer",
                        NetType::Register => "reg",
                        NetType::Wire => "wire",
                    }
                    .as_bytes(),
                )?;
                write!(f, " {size} W{idx:X} ")?;
                if name.contains(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                    f.write_all(b"\\")?;
                }
                f.write_all(name.as_bytes())?;
                f.write_all(b" ")?;
                if size.get() > 1 {
                    write!(f, "[{msb}:{lsb}] ")?;
                }
                writeln!(f, "$end")
            }
        }
    }

    fn extend_into(
        &self,
        tracked: &mut VgHashMap<RtSignalKey, Option<NonZeroUsize>>,
        values: &mut Vec<RtSignalKey>,
    ) {
        match self {
            VcdScopeItem::Scope(s) => s.extend_into(tracked, values),
            VcdScopeItem::Variable(k) => {
                tracked.entry(k.signal).or_insert_with(|| {
                    values.push(k.signal);
                    Some(NonZeroUsize::new(values.len()).unwrap())
                });
            }
        }
    }
}

impl VcdScopeItem {
    fn lower(
        v: &vogls_ir::vcd::VcdScopeItem,
        map: &VgHashMap<SignalKey, RtSignalKey>,
        signal_aliases: &VgHashMap<SignalKey, SignalAlias>,
    ) -> Self {
        match v {
            vogls_ir::vcd::VcdScopeItem::Scope(v) => {
                Self::Scope(VcdScope::lower(v, map, signal_aliases))
            }
            vogls_ir::vcd::VcdScopeItem::Variable(v) => {
                let (mut msb, mut lsb) = (v.msb, v.lsb);
                let mut signal = v.signal;
                while let Some(ns) = signal_aliases.get(&signal) {
                    signal = ns.signal;
                    if let Some((s_msb, s_lsb)) = ns.range {
                        (msb, lsb) = (s_msb as i64, s_lsb as i64);
                    }
                }
                let signal = map[&signal];
                Self::Variable(VcdVariable {
                    name: v.name.clone(),
                    signal,
                    ty: v.ty,
                    msb,
                    lsb,
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct VcdVariable {
    pub name: String,
    pub signal: RtSignalKey,
    pub ty: NetType,
    pub msb: i64,
    pub lsb: i64,
}

#[derive(Debug, Clone)]
pub enum VcdScopeItem {
    Scope(VcdScope),
    Variable(VcdVariable),
}

pub struct VcdOutput {
    start_ts: Timestamp,
    last_ts: Timestamp,
    paused: bool,
    scope: VcdScope,
    tracked: VgHashMap<RtSignalKey, Option<NonZeroUsize>>,
    updated_this_time_step: Vec<RtSignalKey>,
    writer: Box<dyn std::io::Write + Send + Sync>,
    time_scale: u64,
}
impl VcdOutput {
    fn dump_time_step(
        &mut self,
        time: u64,
        heap: &Heap,
        signals: &[HeapRef],
        finish: bool,
    ) -> std::io::Result<()> {
        let f = &mut self.writer;
        if self.start_ts == time {
            writeln!(f, "$version Generated by VoGLS $end")?;
            // @TODO
            writeln!(f, "$date @TODO $end")?;
            writeln!(f, "$timescale 1ns $end")?;
            self.scope.write_to(f)?;
            writeln!(f, "$enddefinitions $end")?;
        }

        // Only print for the timestamp if something actually happened.
        let mut show_for_timestamp = !self.updated_this_time_step.is_empty();
        show_for_timestamp |= finish;
        show_for_timestamp &= self.last_ts != time;
        if !show_for_timestamp {
            return Ok(());
        }

        self.last_ts = time;
        writeln!(f, "#{}", time * self.time_scale)?;
        for signal in &self.updated_this_time_step {
            let bits = signals[signal.as_usize()];
            let idx = signal.as_usize();
            let bits = heap.load_tv_bits(bits);
            if bits.size().get() > 1 {
                f.write_all(&[b'b'])?;
            }
            write!(
                f,
                "{}",
                bits.display(&BitsFormatOptions {
                    prefix: false,
                    base: BitsFormatBase::Binary,
                    separator: None,
                    align: Some(Alignment::Right),
                    fill: '0',
                    width: vogls_bits::format::BitsFormatWidth::Expand
                })
            )?;
            if bits.size().get() > 1 {
                f.write_all(&[b' '])?;
            }
            writeln!(f, "W{idx:X}")?;
            *self.tracked.get_mut(signal).unwrap() = None;
        }

        self.updated_this_time_step.clear();
        Ok(())
    }
}

pub struct Simulation {
    pub processes: Vec<VmProcess>,
    pub signals: Vec<HeapRef>,
    pub logic_mode: LogicMode,
    pub itrace: bool,
}

impl Simulation {
    pub fn new(processes: Vec<VmProcess>, signals: Vec<HeapRef>, logic_mode: LogicMode) -> Self {
        Self {
            processes,
            signals,
            logic_mode,
            itrace: false,
        }
    }

    pub fn new_state(
        &self,
        regions: Regions,
        listeners: SlotMap<ListenerKey, Event>,
        watches: Vec<Vec<ListenerKey>>,
        heap: Heap,
    ) -> SimulationState {
        SimulationState {
            schedule: BTreeMap::<Timestamp, Vec<Event>>::new(),
            runtime: vogls_runtime::RuntimeState::new(heap, self.signals.len()),
            regions,
            listeners,
            watches,
            vcd: None,
            plugins: Vec::new(),
            iplugins: Vec::new(),
            instruction_count: 0,
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

            for region in state.regions.other.iter_mut() {
                if !region.is_empty() {
                    std::mem::swap(&mut state.regions.active, region);
                    continue 'region_loop;
                }
            }

            // Dump the VCD updates for this simulation time.
            if let Some(vcd) = state.vcd.as_mut() {
                vcd.dump_time_step(
                    state.runtime.time,
                    &state.runtime.heap,
                    &self.signals,
                    false,
                )
                .unwrap();
            }
            let mut plugins = std::mem::take(&mut state.plugins);
            for plugin in plugins.iter_mut() {
                plugin.timestep(self, state);
            }
            state.plugins = plugins;

            let Some((at, events)) = state.schedule.pop_first() else {
                break;
            };

            if at > max_time {
                state.runtime.time = max_time;
                state.schedule.insert(at, events);
                break;
            }

            state.runtime.time = at;
            state.regions.active = events;
        }

        if let Some(vcd) = state.vcd.as_mut() {
            vcd.dump_time_step(state.runtime.time, &state.runtime.heap, &self.signals, true)
                .unwrap();
            vcd.writer.flush().unwrap();
        }
        let mut plugins = std::mem::take(&mut state.plugins);
        for plugin in plugins.iter_mut() {
            plugin.finish(self, state);
        }
        state.plugins = plugins;

        if cfg!(vm_profile) {
            state.dump_profile_stats(io, state);
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

            let mut iplugins = std::mem::take(&mut state.iplugins);
            for p in iplugins.iter_mut() {
                p.as_mut().instruction(self, state, instr);
            }
            state.iplugins = iplugins;

            *ip += 1;
            state.instruction_count += 1;

            let outcome = 'instruction: {
                use VmInstruction as I;
                match instr {
                    I::Constant(dst, value) => {
                        execution::exec_constant(&mut state.runtime.heap, *dst, value)
                    }

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
                    I::TvSlice(dst, src, idx) => {
                        execution::tv::exec_tv_slice(&mut state.runtime.heap, *dst, *src, *idx)
                    }
                    I::TvConcat(dst, lhs, rhs) => {
                        execution::tv::exec_tv_concat(&mut state.runtime.heap, *dst, *lhs, *rhs)
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
                    I::FvSlice(dst, src, idx) => {
                        execution::fv::exec_fv_slice(&mut state.runtime.heap, *dst, *src, *idx)
                    }
                    I::FvConcat(dst, lhs, rhs) => {
                        execution::fv::exec_fv_concat(&mut state.runtime.heap, *dst, *lhs, *rhs)
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
                                if state.vcd.is_some() {
                                    writeln!(&mut io.stderr, "ERR! VCD opened a second file")
                                        .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }

                                state.vcd = Some(VcdOutput {
                                    start_ts: state.runtime.time,
                                    last_ts: Timestamp::MAX,
                                    paused: false,
                                    scope: VcdScope {
                                        name: "top".to_string(),
                                        items: Vec::new(),
                                    },
                                    tracked: VgHashMap::default(),
                                    updated_this_time_step: Vec::new(),
                                    writer: Box::new(std::fs::File::create(path).unwrap()),
                                    time_scale: 1000,
                                });
                            }
                            O::VcdAppendModule(scope) => {
                                let Some(vcd) = state.vcd.as_mut() else {
                                    writeln!(
                                        &mut io.stderr,
                                        "ERR! Dumping vars without having a VCD file open"
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                };
                                if vcd.start_ts != state.runtime.time {
                                    writeln!(
                                        &mut io.stderr,
                                        "ERR! Dumping vars over several simulation times"
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }

                                scope
                                    .extend_into(&mut vcd.tracked, &mut vcd.updated_this_time_step);
                                vcd.scope = scope.clone();
                            }
                            O::VcdPause => _ = state.vcd.as_mut().map(|vcd| vcd.paused = true),
                            O::VcdResume => _ = state.vcd.as_mut().map(|vcd| vcd.paused = false),
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
                        }
                    }
                    I::LastUpdateTime(dst, signal) => {
                        let lupdt = state.runtime.last_active_time[signal.as_usize()];
                        state.runtime.heap.set_tv_u64(dst.to_ref(TIME_VSIZE), lupdt);
                    }
                    I::Drive(sig, src, partial) => {
                        let partial = match (partial, self.logic_mode) {
                            (None, _) => None,
                            (Some(offset), LogicMode::TwoValue) => {
                                Some(state.runtime.heap.load_exact_tv_u32(*offset))
                            }
                            (Some(offset), LogicMode::FourValue) => {
                                let (spc, val) = state.runtime.heap.load_exact_fv_u32(*offset);
                                if !spc != 0 {
                                    break 'instruction None;
                                }
                                Some(val)
                            }
                        };

                        let updated = drive_bits(
                            &mut state.runtime.heap,
                            self.signals[sig.as_usize()],
                            *src,
                            partial,
                            self.logic_mode,
                        );

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
                                .entry(state.runtime.time + time)
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
                                .entry(state.runtime.time + time.0)
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
            &mut state.runtime.heap,
            &self.signals,
            &mut state.watches,
            &mut state.listeners,
            &mut state.regions,
            None,
        );
        if let Some(vcd) = state.vcd.as_mut()
            && !vcd.paused
            && let Some(idx) = vcd.tracked.get_mut(&signal)
        {
            idx.get_or_insert_with(|| {
                vcd.updated_this_time_step.push(signal);
                NonZeroUsize::new(vcd.updated_this_time_step.len()).unwrap()
            });
        }
        let mut plugins = std::mem::take(&mut state.plugins);
        for plugin in plugins.iter_mut() {
            plugin.update_signal(self, state, signal);
        }
        state.plugins = plugins;
        state.runtime.last_active_time[signal.as_usize()] = state.runtime.time;
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
    pub vcd: Option<VcdOutput>,
    pub plugins: Vec<plugin::PluginState>,
    pub iplugins: Vec<plugin::InstructionPluginState>,

    pub instruction_count: u64,
}

impl Clone for SimulationState {
    fn clone(&self) -> Self {
        Self {
            schedule: self.schedule.clone(),
            runtime: self.runtime.clone(),

            regions: self.regions.clone(),
            listeners: self.listeners.clone(),
            watches: self.watches.clone(),
            vcd: None,
            plugins: vec![],
            iplugins: vec![],

            instruction_count: self.instruction_count.clone(),
        }
    }
}

impl SimulationState {
    pub fn dump_profile_stats(&self, io: &mut SimulationIo, state: &SimulationState) {
        writeln!(io.stdout, "Stats:",).unwrap();
        writeln!(io.stdout, "  # Instructions: {}", state.instruction_count).unwrap();
        writeln!(io.stdout, "  # Events:       {}", state.runtime.event_count).unwrap();
        writeln!(
            io.stdout,
            "  # Stack size:   {}",
            state.runtime.heap.0.len() * size_of::<u64>()
        )
        .unwrap();
    }

    pub fn start_vcd(&mut self, path: &Path, scope: VcdScope) {
        self.start_vcd_raw(Box::new(std::fs::File::create(path).unwrap()), scope);
    }

    pub fn start_vcd_raw(
        &mut self,
        writer: Box<dyn std::io::Write + Send + Sync>,
        scope: VcdScope,
    ) {
        let mut tracked = VgHashMap::default();
        let mut updated_this_time_step = Vec::new();

        scope.extend_into(&mut tracked, &mut updated_this_time_step);

        self.vcd = Some(VcdOutput {
            start_ts: 0,
            last_ts: Timestamp::MAX,
            paused: false,
            scope,
            tracked,
            updated_this_time_step,
            writer,
            time_scale: 1000,
        });
    }
}
