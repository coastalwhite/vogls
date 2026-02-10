use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Alignment;
use std::num::NonZeroUsize;
use std::path::Path;

use slotmap::{SlotMap, new_key_type};
use vogls_bits::arithmetic::{FvLogicValue, fv_set_no_special};
use vogls_bits::format::{BitsFormatBase, BitsFormatOptions};
use vogls_bits::set_subslice::{tv_l_set, tv_s_set};
use vogls_ir::vcd::NetType;
use vogls_ir::{INTEGER_VSIZE, LogicMode, SCALAR_VSIZE, SignalKey, TIME_VSIZE, VectorSize};

mod execution;
mod heap;
mod instruction;

use heap::Heap;
pub use instruction::*;
use vogls_utils::NonMaxU64;

new_key_type! { pub struct ListenerKey; }

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VmProcessKey(pub u64);

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum DispatchKey {
    Signal(VmSignalKey),
    Process(VmProcessKey),
}

pub struct Regions {
    pub active: Vec<Event>,
    pub other_dispatched: Vec<HashMap<DispatchKey, usize>>,
    pub other: Vec<Vec<Event>>,
}

impl Regions {
    pub fn new(num_additional_regions: usize) -> Self {
        Self {
            active: Vec::new(),
            other_dispatched: vec![HashMap::new(); num_additional_regions],
            other: vec![Vec::new(); num_additional_regions],
        }
    }
}

pub type Timestamp = u64;
pub type InstanceId = u64;

pub struct Context {
    time: Timestamp,
    logic_mode: LogicMode,
    pub stdout: Box<dyn std::io::Write>,
    pub stderr: Box<dyn std::io::Write>,
    pub instruction_count: u64,
    pub event_count: u64,
    pub itrace: bool,
}

impl Context {
    pub fn new(
        logic_mode: LogicMode,
        stdout: Box<dyn std::io::Write>,
        stderr: Box<dyn std::io::Write>,
    ) -> Self {
        Self {
            time: 0,
            logic_mode,
            stdout,
            stderr,
            instruction_count: 0,
            event_count: 0,
            itrace: false,
        }
    }
}

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
    sig: VmSignalKey,
    stack: &Heap,
    signals: &[HeapRef],
    watches: &mut [Vec<ListenerKey>],
    listeners: &mut SlotMap<ListenerKey, Event>,
    regions: &mut Regions,
    trace: Option<&mut vogls_trace::Trace>,
) {
    let start = regions.active.len();
    let watchers = &mut watches[sig.0 as usize];
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
            sig.0,
            stack.load_tv_bits(signals[sig.0 as usize]),
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
    if partial.is_some() || dst.size != src.size {
        let partial = partial.unwrap_or(0);

        return match logic_mode {
            LogicMode::TwoValue if dst.size.get() <= 32 => {
                let (dst_s, src_s) = heap.get_disjoint_u8_dst_src(dst, src);
                tv_s_set(dst_s, src_s, dst.size, partial, src.size)
            }
            LogicMode::TwoValue => {
                let mut src_s = [0u64];
                let (dst_s, src_s) = if src.size.get() <= 32 {
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
            LogicMode::FourValue if dst.size.get() <= 16 => {
                let (src_spc, src_val) = heap.get_fv_u64(src);
                let (old_spc, old_val) = heap.get_fv_u64(dst);

                let mask = (1u64 << src.size.get()) - 1;
                let mask = mask << partial;
                let new_spc = (src_spc << partial) | (old_spc & !mask);
                let new_val = (src_val << partial) | (old_val & !mask);
                heap.set_fv_u64(dst, new_spc, new_val);
                old_spc != new_spc || old_val != new_val
            }
            _ => {
                let mut src_s = [0u64, 0u64];
                let dst_nwords = dst.size.get().div_ceil(64) as usize;
                let (dst_s, src_s) = if src.size.get() <= 16 {
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
        map: &HashMap<SignalKey, VmSignalKey>,
        signal_map: &HashMap<SignalKey, SignalKey>,
    ) -> VcdScope {
        VcdScope {
            name: v.name.clone(),
            items: v
                .items
                .iter()
                .map(|i| VcdScopeItem::lower(i, map, signal_map))
                .collect(),
        }
    }

    fn write_to(&self, f: &mut impl std::io::Write) -> std::io::Result<()> {
        let Self { name, items } = self;
        writeln!(f, "$scope module {name} $end")?;
        for item in items {
            item.write_to(f)?;
        }
        writeln!(f, "$upscope $end")?;
        Ok(())
    }

    fn extend_into(
        &self,
        tracked: &mut HashMap<VmSignalKey, Option<NonZeroUsize>>,
        values: &mut Vec<VmSignalKey>,
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
                let idx = k.signal.0;
                write!(f, "$var ")?;
                f.write_all(
                    match ty {
                        NetType::Integer => "integer",
                        NetType::Register => "reg",
                        NetType::Wire => "wire",
                    }
                    .as_bytes(),
                )?;
                write!(f, " {size} W{idx:X} {name} ")?;
                if size.get() > 1 {
                    write!(f, "[{msb}:{lsb}] ")?;
                }
                writeln!(f, "$end")
            }
        }
    }

    fn extend_into(
        &self,
        tracked: &mut HashMap<VmSignalKey, Option<NonZeroUsize>>,
        values: &mut Vec<VmSignalKey>,
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
        map: &HashMap<SignalKey, VmSignalKey>,
        signal_map: &HashMap<SignalKey, SignalKey>,
    ) -> Self {
        match v {
            vogls_ir::vcd::VcdScopeItem::Scope(v) => {
                Self::Scope(VcdScope::lower(v, map, signal_map))
            }
            vogls_ir::vcd::VcdScopeItem::Variable(v) => {
                let mut signal = v.signal;
                while let Some(ns) = signal_map.get(&signal) {
                    signal = *ns;
                }
                let signal = map[&signal];
                Self::Variable(VcdVariable {
                    name: v.name.clone(),
                    signal,
                    ty: v.ty,
                    msb: v.msb,
                    lsb: v.lsb,
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct VcdVariable {
    pub name: String,
    pub signal: VmSignalKey,
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
    tracked: HashMap<VmSignalKey, Option<NonZeroUsize>>,
    updated_this_time_step: Vec<VmSignalKey>,
    writer: Box<dyn std::io::Write>,
    time_scale: u64,
}
impl VcdOutput {
    fn dump_time_step(
        &mut self,
        ctx: &Context,
        heap: &Heap,
        signals: &[HeapRef],
        finish: bool,
    ) -> std::io::Result<()> {
        let f = &mut self.writer;
        if self.start_ts == ctx.time {
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
        show_for_timestamp &= self.last_ts != ctx.time;
        if !show_for_timestamp {
            return Ok(());
        }

        self.last_ts = ctx.time;
        writeln!(f, "#{}", ctx.time * self.time_scale)?;
        for signal in &self.updated_this_time_step {
            let bits = signals[signal.0 as usize];
            let idx = signal.0;
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

impl Event {
    fn evaluate(
        mut self,
        ctx: &mut Context,
        processes: &[VmProcess],
        schedule: &mut BTreeMap<Timestamp, Vec<Event>>,
        regions: &mut Regions,
        signals: &mut [HeapRef],
        last_active_time: &mut [NonMaxU64],
        listeners: &mut SlotMap<ListenerKey, Event>,
        watches: &mut [Vec<ListenerKey>],
        heap: &mut Heap,
        vcd: &mut Option<VcdOutput>,
        mut trace: Option<&mut vogls_trace::Trace>,
    ) -> EvalOutcome {
        let Event {
            process: process_key,
            ip,
        } = &mut self;

        let process = &processes[process_key.0 as usize];

        loop {
            let instr = &process.instructions[*ip];

            *ip += 1;
            ctx.instruction_count += 1;

            let outcome = 'instruction: {
                use VmInstruction as I;
                match instr {
                    I::Constant(dst, value) => execution::exec_constant(heap, *dst, value),

                    I::TvUnary(dst, op, src) => execution::tv::exec_tv_unary(heap, *dst, *op, *src),
                    I::TvResize(dst, op, src) => {
                        execution::tv::exec_tv_resize(heap, *dst, *op, *src)
                    }
                    I::TvBinaryArithmetic(dst, op, lhs, rhs) => {
                        execution::tv::exec_tv_bin_arith(heap, *dst, *op, *lhs, *rhs)
                    }
                    I::TvBinaryComparison(dst, op, lhs, rhs) => {
                        execution::tv::exec_tv_bin_cmp(heap, *dst, *op, *lhs, *rhs)
                    }
                    I::TvShift(dst, op, src, offset) => {
                        execution::tv::exec_tv_shift(heap, *dst, *op, *src, *offset)
                    }
                    I::TvSelectBit(dst, src, idx) => {
                        execution::tv::exec_tv_select_bit(heap, *dst, *src, *idx)
                    }
                    I::TvConcat(dst, lhs, rhs) => {
                        execution::tv::exec_tv_concat(heap, *dst, *lhs, *rhs)
                    }

                    I::FvUnary(dst, op, src) => execution::fv::exec_fv_unary(heap, *dst, *op, *src),
                    I::FvResize(dst, op, src) => {
                        execution::fv::exec_fv_resize(heap, *dst, *op, *src)
                    }
                    I::FvBinaryArithmetic(dst, op, lhs, rhs) => {
                        execution::fv::exec_fv_bin_arith(heap, *dst, *op, *lhs, *rhs)
                    }
                    I::FvBinaryComparison(dst, op, lhs, rhs) => {
                        execution::fv::exec_fv_bin_cmp(heap, *dst, *op, *lhs, *rhs)
                    }
                    I::FvShift(dst, op, src, offset) => {
                        execution::fv::exec_fv_shift(heap, *dst, *op, *src, *offset)
                    }
                    I::FvSelectBit(dst, src, idx) => {
                        execution::fv::exec_fv_select_bit(heap, *dst, *src, *idx)
                    }
                    I::FvConcat(dst, lhs, rhs) => {
                        execution::fv::exec_fv_concat(heap, *dst, *lhs, *rhs)
                    }

                    I::TvToFv(dst, src) => {
                        let size = dst.size;
                        if size.get() <= 32 {
                            let v = heap.get_tv_u64(src.to_ref(size));
                            heap.set_fv_u64(
                                *dst,
                                1u64.unbounded_shl(size.get()).wrapping_sub(1),
                                v,
                            );
                        } else {
                            let nwords = size.get().div_ceil(64) as usize;
                            let (dst, src) = heap
                                .get_disjoint_u64_dst_src((dst.offset, nwords * 2), (*src, nwords));
                            fv_set_no_special(dst, size);
                            dst[nwords..].copy_from_slice(src);
                        }
                    }
                    I::FvToTv(dst, src) => {
                        let size = dst.size;
                        if size.get() <= 32 {
                            let (spc, val) = heap.get_fv_u64(src.to_ref(size));
                            heap.set_tv_u64(*dst, spc & val);
                        } else {
                            let nwords = size.get().div_ceil(64) as usize;
                            let (dst, src) = heap
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
                                    &mut ctx.stdout,
                                    args.iter().map(|(sr, lm)| match lm {
                                        LogicMode::TwoValue => heap.load_tv_bits(*sr),
                                        LogicMode::FourValue => heap.load_fv_bits(*sr),
                                    }),
                                )
                                .unwrap();
                            }
                            O::Assert(f) => {
                                let (cond_sr, cond_lm) = args[0];
                                let condition = match cond_lm {
                                    LogicMode::TwoValue => heap.get_tv_bool(cond_sr.offset),
                                    LogicMode::FourValue => {
                                        heap.get_fv_item(cond_sr.offset) == FvLogicValue::L1
                                    }
                                };

                                if !condition {
                                    f.write_to(
                                        &mut ctx.stdout,
                                        args[1..].iter().map(|(sr, lm)| match lm {
                                            LogicMode::TwoValue => heap.load_tv_bits(*sr),
                                            LogicMode::FourValue => heap.load_fv_bits(*sr),
                                        }),
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }
                            }
                            O::VcdOpenFile(path) => {
                                if vcd.is_some() {
                                    writeln!(&mut ctx.stderr, "ERR! VCD opened a second file")
                                        .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }

                                *vcd = Some(VcdOutput {
                                    start_ts: ctx.time,
                                    last_ts: Timestamp::MAX,
                                    paused: false,
                                    scope: VcdScope {
                                        name: "top".to_string(),
                                        items: Vec::new(),
                                    },
                                    tracked: HashMap::new(),
                                    updated_this_time_step: Vec::new(),
                                    writer: Box::new(std::fs::File::create(path).unwrap()),
                                    time_scale: 1000,
                                });
                            }
                            O::VcdAppendModule(scope) => {
                                let Some(vcd) = vcd.as_mut() else {
                                    writeln!(
                                        &mut ctx.stderr,
                                        "ERR! Dumping vars without having a VCD file open"
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                };
                                if vcd.start_ts != ctx.time {
                                    writeln!(
                                        &mut ctx.stderr,
                                        "ERR! Dumping vars over several simulation times"
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }

                                scope
                                    .extend_into(&mut vcd.tracked, &mut vcd.updated_this_time_step);
                                vcd.scope = scope.clone();
                            }
                            O::VcdPause => _ = vcd.as_mut().map(|vcd| vcd.paused = true),
                            O::VcdResume => _ = vcd.as_mut().map(|vcd| vcd.paused = false),
                            O::Time => _ = heap.set_tv_u64(dst.to_ref(TIME_VSIZE), ctx.time),
                            O::Random => _ = heap.set_tv_u64(dst.to_ref(INTEGER_VSIZE), ctx.time),
                            O::Finish => {
                                writeln!(&mut ctx.stdout, "[FINISH]").unwrap();
                                break 'instruction Some(EvalOutcome::Exit);
                            }
                        }
                    }
                    I::Drive(sig, src, partial) => {
                        let partial = match (partial, ctx.logic_mode) {
                            (None, _) => None,
                            (Some(offset), LogicMode::TwoValue) => {
                                Some(heap.load_exact_tv_u32(*offset))
                            }
                            (Some(offset), LogicMode::FourValue) => {
                                let (spc, val) = heap.load_exact_fv_u32(*offset);
                                if !spc != 0 {
                                    break 'instruction None;
                                }
                                Some(val)
                            }
                        };

                        let updated = drive_bits(
                            heap,
                            signals[sig.0 as usize],
                            *src,
                            partial,
                            ctx.logic_mode,
                        );

                        if updated {
                            update_watchers(
                                *sig,
                                heap,
                                signals,
                                watches,
                                listeners,
                                regions,
                                trace.as_deref_mut(),
                            );
                            if let Some(vcd) = vcd.as_mut()
                                && !vcd.paused
                                && let Some(idx) = vcd.tracked.get_mut(sig)
                            {
                                idx.get_or_insert_with(|| {
                                    vcd.updated_this_time_step.push(*sig);
                                    NonZeroUsize::new(vcd.updated_this_time_step.len()).unwrap()
                                });
                            }
                            last_active_time[sig.0 as usize] = NonMaxU64::new(ctx.time).unwrap();
                        }
                    }
                    I::Wait(time) => {
                        schedule.entry(ctx.time + time.0).or_default().push(self);
                        if let Some(trace) = trace.as_deref_mut() {
                            let vogls_trace::Event::Evaluation(_, _, stop_reason) =
                                trace.events.last_mut().unwrap()
                            else {
                                unreachable!();
                            };
                            *stop_reason = vogls_trace::EventStopReason::Wait(ctx.time + time.0);
                        }
                        if ctx.itrace {
                            instr.itrace(heap, signals, ctx.logic_mode);
                        }
                        return EvalOutcome::Next;
                    }
                    I::WaitRegion(region) => {
                        if *region == 0 {
                            regions.active.push(self);
                        } else {
                            regions.other[*region as usize - 1].push(self);
                        }
                        if let Some(trace) = trace.as_deref_mut() {
                            let vogls_trace::Event::Evaluation(_, _, stop_reason) =
                                trace.events.last_mut().unwrap()
                            else {
                                unreachable!();
                            };
                            *stop_reason = vogls_trace::EventStopReason::WaitRegion(*region);
                        }
                        if ctx.itrace {
                            instr.itrace(heap, signals, ctx.logic_mode);
                        }
                        return EvalOutcome::Next;
                    }
                    I::Watch(watch_signals) => {
                        let listener_key = listeners.insert(self);
                        for signal in watch_signals {
                            watches[signal.0 as usize].push(listener_key);
                        }
                        if let Some(trace) = trace.as_mut() {
                            let watch_range_start = trace.watches.len() as u64;
                            trace.watches.extend(watch_signals.iter().map(|s| s.0));
                            let vogls_trace::Event::Evaluation(_, _, stop_reason) =
                                trace.events.last_mut().unwrap()
                            else {
                                unreachable!();
                            };
                            *stop_reason = vogls_trace::EventStopReason::WatchSignals(
                                watch_range_start..trace.watches.len() as u64,
                            );
                        }
                        if ctx.itrace {
                            instr.itrace(heap, signals, ctx.logic_mode);
                        }
                        return EvalOutcome::Next;
                    }

                    I::Jump(offset) => *ip = *offset,
                    I::Branch(cond, true_offset, false_offset) => {
                        let is_true = heap.get_tv_u64(cond.to_ref(SCALAR_VSIZE)) & 1 != 0;
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

            if ctx.itrace {
                instr.itrace(heap, signals, ctx.logic_mode);
            }

            if let Some(outcome) = outcome {
                return outcome;
            }
        }
    }
}

#[derive(Clone)]
pub struct SignalInfo {
    pub name: String,
}

pub fn run(
    ctx: &mut Context,
    processes: &[VmProcess],
    regions: &mut Regions,
    signals: &mut [HeapRef],
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut [Vec<ListenerKey>],
    mut trace: Option<&mut vogls_trace::Trace>,
    heap: &mut Heap,
    max_time: u64,
    vcd: Option<(&Path, VcdScope)>,
) -> Result<(), ()> {
    let mut schedule = BTreeMap::<Timestamp, Vec<Event>>::new();
    let mut last_active_time = vec![NonMaxU64::ZERO; signals.len()];
    let mut vcd = match vcd {
        None => None,
        Some((p, scope)) => {
            let mut tracked = HashMap::new();
            let mut updated_this_time_step = Vec::new();

            scope.extend_into(&mut tracked, &mut updated_this_time_step);

            Some(VcdOutput {
                start_ts: ctx.time,
                last_ts: Timestamp::MAX,
                paused: false,
                scope,
                tracked,
                updated_this_time_step,
                writer: Box::new(std::fs::File::create(p).unwrap()),
                time_scale: 1000,
            })
        }
    };

    'region_loop: loop {
        while let Some(event) = regions.active.pop() {
            if let Some(trace) = trace.as_deref_mut() {
                trace.events.push(vogls_trace::Event::Evaluation(
                    event.process.0 as u64,
                    trace.driven.len() as u64..trace.driven.len() as u64,
                    vogls_trace::EventStopReason::Halt,
                ));
            }
            if cfg!(vm_profile) {
                ctx.event_count += 1;
            }

            let outcome = event.evaluate(
                ctx,
                processes,
                &mut schedule,
                regions,
                signals,
                &mut last_active_time,
                listeners,
                watches,
                heap,
                &mut vcd,
                trace.as_deref_mut(),
            );

            if ctx.itrace {
                eprintln!();
            }

            if let Some(trace) = trace.as_deref_mut() {
                match trace.events.last_mut().unwrap() {
                    vogls_trace::Event::Drive(_, drive) => {
                        _ = drive.take_if(|d| *d == trace.driven.len() as u64)
                    }
                    vogls_trace::Event::Evaluation(_, driven, _) => {
                        driven.end = trace.driven.len() as u64
                    }
                    vogls_trace::Event::Time(_) => {}
                }
            }

            match outcome {
                EvalOutcome::Next => continue,
                EvalOutcome::Error => return Err(()),
                EvalOutcome::Exit => break 'region_loop,
            }
        }

        for (i, region) in regions.other.iter_mut().enumerate() {
            if !region.is_empty() {
                regions.other_dispatched[i].clear();
                std::mem::swap(&mut regions.active, region);
                continue 'region_loop;
            }
        }

        // Dump the VCD updates for this simulation time.
        if let Some(vcd) = vcd.as_mut() {
            vcd.dump_time_step(ctx, heap, signals, false).unwrap();
        }

        let Some((at, events)) = schedule.pop_first() else {
            break;
        };

        ctx.time = at;
        if let Some(trace) = trace.as_deref_mut() {
            trace.events.push(vogls_trace::Event::Time(ctx.time));
        }
        if ctx.time > max_time {
            break;
        }
        regions.active = events;
    }

    if let Some(vcd) = vcd.as_mut() {
        vcd.dump_time_step(ctx, heap, signals, true).unwrap();
        vcd.writer.flush().unwrap();
    }

    if cfg!(vm_profile) {
        writeln!(ctx.stdout, "Stats:",).unwrap();
        writeln!(ctx.stdout, "  # Instructions: {}", ctx.instruction_count).unwrap();
        writeln!(ctx.stdout, "  # Events:       {}", ctx.event_count).unwrap();
        writeln!(ctx.stdout, "  # Stack size:   {}", heap.0.len()).unwrap();
    }

    Ok(())
}
