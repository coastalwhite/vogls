use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;

use slotmap::{SlotMap, new_key_type};
use vogls_bits::arithmetic::{fv_pack_u64, fv_separate_packed_u64, fv_set_no_special};
use vogls_bits::load::load_partial_u64;
use vogls_bits::store::store_partial_u64;
use vogls_bits::{BitsDataRef, get_disjoint_dst_src};
use vogls_ir::dyn_format_string::{Base, Padding, format_bits};
use vogls_ir::vcd::NetType;
use vogls_ir::{Bits, INTEGER_VSIZE, LogicMode, SignalKey, TIME_VSIZE, VectorSize};

mod bits;
mod execution;
mod instruction;

pub use instruction::*;

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
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Drive(VmSignalKey, Vec<(Bits, Option<(u32, VectorSize)>)>),
    Evaluation(EvaluationEvent),
}

#[derive(Clone, Debug)]
pub struct EvaluationEvent {
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
    signals: &mut [Bits],
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
        trace.woken.extend(regions.active[start..].iter().map(|e| {
            let Event::Evaluation(e) = &e else {
                panic!("only evaluation events are expected");
            };
            e.process.0
        }));
        let woken_range = woken_start..trace.woken.len() as u64;
        trace
            .driven
            .push((sig.0, signals[sig.0 as usize].clone(), woken_range));
    }
}

pub fn drive_bits(
    bits: &mut Bits,
    slice: &[u8],
    offset: usize,
    size: VectorSize,
    partial: Option<(u32, VectorSize)>,
    logic_mode: LogicMode,
) -> bool {
    if partial.is_some() {
        todo!()
    }

    let prev = bits.clone();
    match logic_mode {
        LogicMode::TwoValue if size.get() <= 32 => {
            let nbytes = size.get().div_ceil(8) as usize;
            let value = load_partial_u64(&slice[offset..][..nbytes], size);
            *bits = Bits::from_u64(size, value);
        }
        LogicMode::TwoValue => {
            let nwords = size.get().div_ceil(64) as usize;
            let slice = bytemuck::cast_slice::<u8, u64>(&slice[offset..][..nwords * 8]);
            *bits = Bits::from_boxed_slice(vogls_ir::Mode::TwoValue, size, slice.into())
        }
        LogicMode::FourValue if size.get() <= 16 => {
            let nbytes = (2 * size.get()).div_ceil(8) as usize;
            let value = load_partial_u64(
                &slice[offset..][..nbytes],
                VectorSize::new(2 * size.get()).unwrap(),
            );
            let (spc, val) = fv_separate_packed_u64(value, size);
            *bits = Bits::from_four_value_u64(size, spc as u32, val as u32);
        }
        LogicMode::FourValue => {
            let nwords = 2 * size.get().div_ceil(64) as usize;
            let slice = bytemuck::cast_slice::<u8, u64>(&slice[offset..][..nwords * 8]);
            *bits = Bits::from_boxed_slice(vogls_ir::Mode::FourValue, size, slice.into())
        }
    }
    &prev != bits
}

#[derive(Debug, Clone)]
pub struct VcdScope {
    pub name: String,
    pub items: Vec<VcdScopeItem>,
}

impl VcdScope {
    fn lower(v: &vogls_ir::vcd::VcdScope, map: &HashMap<SignalKey, VmSignalKey>) -> VcdScope {
        VcdScope {
            name: v.name.clone(),
            items: v
                .items
                .iter()
                .map(|i| VcdScopeItem::lower(i, map))
                .collect(),
        }
    }

    fn write_to(&self, f: &mut impl std::io::Write, info: &[SignalInfo]) -> std::io::Result<()> {
        let Self { name, items } = self;
        writeln!(f, "$scope module {name} $end")?;
        for item in items {
            item.write_to(f, info)?;
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
    fn write_to(&self, f: &mut impl std::io::Write, info: &[SignalInfo]) -> std::io::Result<()> {
        match self {
            VcdScopeItem::Scope(scope) => scope.write_to(f, info),
            VcdScopeItem::Variable(k) => {
                let VcdVariable {
                    signal,
                    ty,
                    msb,
                    lsb,
                } = k;
                let SignalInfo { name } = &info[signal.0 as usize];
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
    fn lower(v: &vogls_ir::vcd::VcdScopeItem, map: &HashMap<SignalKey, VmSignalKey>) -> Self {
        match v {
            vogls_ir::vcd::VcdScopeItem::Scope(v) => Self::Scope(VcdScope::lower(v, map)),
            vogls_ir::vcd::VcdScopeItem::Variable(v) => Self::Variable(VcdVariable {
                signal: map[&v.signal],
                ty: v.ty,
                msb: v.msb,
                lsb: v.lsb,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VcdVariable {
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
}
impl VcdOutput {
    fn dump_time_step(
        &mut self,
        ctx: &Context,
        signals: &[Bits],
        signal_info: &[SignalInfo],
        finish: bool,
    ) -> std::io::Result<()> {
        let f = &mut self.writer;
        if self.start_ts == ctx.time {
            writeln!(f, "$version Generated by VoGLS $end")?;
            // @TODO
            writeln!(f, "$date @TODO $end")?;
            writeln!(f, "$timescale 1ns $end")?;
            self.scope.write_to(f, signal_info)?;
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
        writeln!(f, "#{}", ctx.time)?;
        for signal in &self.updated_this_time_step {
            let bits = &signals[signal.0 as usize];
            let idx = signal.0;
            if bits.size().get() > 1 {
                f.write_all(&[b'b'])?;
            }
            format_bits(f, bits, Padding::ZeroPaddedToSize, Base::Binary).unwrap();
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
        signals: &mut [Bits],
        listeners: &mut SlotMap<ListenerKey, Event>,
        watches: &mut [Vec<ListenerKey>],
        stack: &mut [u8],
        vcd: &mut Option<VcdOutput>,
        mut trace: Option<&mut vogls_trace::Trace>,
    ) -> EvalOutcome {
        let EvaluationEvent {
            process: process_key,
            ip,
        } = match &mut self {
            Event::Drive(sig, assignments) => {
                let signal_bits = &mut signals[sig.0 as usize];
                let mut updated = false;
                for (bits, partial) in assignments {
                    updated |= drive_bits(
                        signal_bits,
                        bits.as_slice(),
                        0,
                        bits.size(),
                        *partial,
                        ctx.logic_mode,
                    );
                }

                if updated {
                    update_watchers(*sig, signals, watches, listeners, regions, trace);
                    if let Some(vcd) = vcd.as_mut()
                        && !vcd.paused
                        && let Some(idx) = vcd.tracked.get_mut(sig)
                    {
                        idx.get_or_insert_with(|| {
                            vcd.updated_this_time_step.push(*sig);
                            NonZeroUsize::new(vcd.updated_this_time_step.len()).unwrap()
                        });
                    }
                }

                return EvalOutcome::Next;
            }
            Event::Evaluation(e) => e,
        };

        let process = &processes[process_key.0 as usize];

        use VmInstruction as I;
        loop {
            let instr = &process.instructions[*ip];
            *ip += 1;
            ctx.instruction_count += 1;
            match instr {
                I::Constant(dst, value) => execution::exec_constant(stack, dst.0, value),

                I::TvUnary(dst, op, size, src) => {
                    execution::tv::exec_tv_unary(stack, dst.0, *op, *size, src.0)
                }
                I::TvResize(dst, op, dst_size, src_size, src) => {
                    execution::tv::exec_tv_resize(stack, dst.0, *dst_size, *op, src.0, *src_size)
                }
                I::TvBinaryArithmetic(dst, op, size, lhs, rhs) => {
                    execution::tv::exec_tv_bin_arith(stack, dst.0, *op, *size, lhs.0, rhs.0)
                }
                I::TvBinaryComparison(dst, op, size, lhs, rhs) => {
                    execution::tv::exec_tv_bin_cmp(stack, dst.0, *op, *size, lhs.0, rhs.0)
                }
                I::TvShift(dst, op, size, src, offset) => {
                    execution::tv::exec_tv_shift(stack, dst.0, *op, *size, src.0, offset.0)
                }
                I::TvSelectBit(dst, size, src, idx) => {
                    execution::tv::exec_tv_select_bit(stack, dst.0, *size, src.0, idx.0)
                }
                I::TvConcat(dst, lhs_size, lhs, rhs_size, rhs) => {
                    execution::tv::exec_tv_concat(stack, dst.0, lhs.0, *lhs_size, rhs.0, *rhs_size)
                }

                I::FvUnary(dst, op, size, src) => {
                    execution::fv::exec_fv_unary(stack, dst.0, *op, *size, src.0)
                }
                I::FvResize(dst, op, dst_size, src_size, src) => {
                    execution::fv::exec_fv_resize(stack, dst.0, *dst_size, *op, src.0, *src_size)
                }
                I::FvBinaryArithmetic(dst, op, size, lhs, rhs) => {
                    execution::fv::exec_fv_bin_arith(stack, dst.0, *op, *size, lhs.0, rhs.0)
                }
                I::FvBinaryComparison(dst, op, size, lhs, rhs) => {
                    execution::fv::exec_fv_bin_cmp(stack, dst.0, *op, *size, lhs.0, rhs.0)
                }
                I::FvShift(dst, op, size, src, offset) => {
                    execution::fv::exec_fv_shift(stack, dst.0, *op, *size, src.0, offset.0)
                }
                I::FvSelectBit(dst, size, src, idx) => {
                    execution::fv::exec_fv_select_bit(stack, dst.0, *size, src.0, idx.0)
                }
                I::FvConcat(dst, lhs_size, lhs, rhs_size, rhs) => {
                    execution::fv::exec_fv_concat(stack, dst.0, lhs.0, *lhs_size, rhs.0, *rhs_size)
                }

                I::TvToFv(dst, src, size) => {
                    if size.get() <= 16 {
                        let tv_nbytes = size.get().div_ceil(8) as usize;
                        let fv_nbytes = (size.get() * 2).div_ceil(8) as usize;
                        let v = load_partial_u64(&stack[src.0..][..tv_nbytes], *size);
                        store_partial_u64(
                            &mut stack[dst.0..][..fv_nbytes],
                            v | (((1u64 << size.get()) - 1) << size.get()),
                            VectorSize::new(2 * size.get()).unwrap(),
                        );
                    } else if size.get() <= 32 {
                        let tv_nbytes = size.get().div_ceil(8) as usize;
                        let v = load_partial_u64(&stack[src.0..][..tv_nbytes], *size);
                        let dst = bytemuck::cast_slice_mut::<u8, u64>(&mut stack[dst.0..][..16]);
                        dst[0] = (1u64 << size.get()) - 1;
                        dst[1] = v;
                    } else {
                        let tv_nwords = size.get().div_ceil(64) as usize;
                        let fv_nwords = 2 * size.get().div_ceil(64) as usize;
                        let (dst, src) =
                            get_disjoint_dst_src(stack, dst.0, fv_nwords * 8, src.0, tv_nwords * 8);
                        let dst = bytemuck::cast_slice_mut::<u8, u64>(dst);
                        let src = bytemuck::cast_slice::<u8, u64>(src);

                        fv_set_no_special(dst, *size);
                        dst[tv_nwords..].copy_from_slice(src);
                    }
                }
                I::FvToTv(dst, src, size) => {
                    if size.get() <= 16 {
                        let tv_nbytes = size.get().div_ceil(8) as usize;
                        let fv_nbytes = (size.get() * 2).div_ceil(8) as usize;
                        let v = load_partial_u64(
                            &stack[src.0..][..fv_nbytes],
                            VectorSize::new(2 * size.get()).unwrap(),
                        );
                        store_partial_u64(
                            &mut stack[dst.0..][..tv_nbytes],
                            v & ((1u64 << size.get()) - 1),
                            *size,
                        );
                    } else if size.get() <= 32 {
                        let tv_nbytes = size.get().div_ceil(8) as usize;
                        let v = bytemuck::cast_slice::<u8, u64>(&stack[dst.0..][..16])[1];
                        store_partial_u64(&mut stack[..tv_nbytes], v, *size);
                    } else {
                        let tv_nwords = size.get().div_ceil(64) as usize;
                        let fv_nwords = 2 * size.get().div_ceil(64) as usize;
                        let (dst, src) =
                            get_disjoint_dst_src(stack, src.0, tv_nwords * 8, dst.0, fv_nwords * 8);
                        let dst = bytemuck::cast_slice_mut::<u8, u64>(dst);
                        let src = bytemuck::cast_slice::<u8, u64>(src);

                        dst.copy_from_slice(&src[tv_nwords..]);
                    }
                }

                I::Intrinsic(dst, op, args) => {
                    use VmIntrinsicOp as O;

                    match op.as_ref() {
                        O::Display(f) => {
                            f.write_to(
                                &mut ctx.stdout,
                                args.iter().map(|(o, s)| {
                                    Bits::load_from_slice(
                                        &stack[o.0..][..s.get().div_ceil(8) as usize],
                                        *s,
                                    )
                                }),
                            )
                            .unwrap();
                        }
                        O::Assert(f) => {
                            let condition = stack[args[0].0.0] != 0;
                            if !condition {
                                f.write_to(
                                    &mut ctx.stdout,
                                    args[1..].iter().map(|(o, s)| {
                                        Bits::load_from_slice(
                                            &stack[o.0..][..s.get().div_ceil(8) as usize],
                                            *s,
                                        )
                                    }),
                                )
                                .unwrap();
                                return EvalOutcome::Error;
                            }
                        }
                        O::VcdOpenFile(path) => {
                            if vcd.is_some() {
                                writeln!(&mut ctx.stderr, "ERR! VCD opened a second file").unwrap();
                                return EvalOutcome::Error;
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
                            });
                        }
                        O::VcdAppendModule(scope) => {
                            let Some(vcd) = vcd.as_mut() else {
                                writeln!(
                                    &mut ctx.stderr,
                                    "ERR! Dumping vars without having a VCD file open"
                                )
                                .unwrap();
                                return EvalOutcome::Error;
                            };
                            if vcd.start_ts != ctx.time {
                                writeln!(
                                    &mut ctx.stderr,
                                    "ERR! Dumping vars over several simulation times"
                                )
                                .unwrap();
                                return EvalOutcome::Error;
                            }

                            scope.extend_into(&mut vcd.tracked, &mut vcd.updated_this_time_step);
                            vcd.scope = scope.clone();
                        }
                        O::VcdPause => _ = vcd.as_mut().map(|vcd| vcd.paused = true),
                        O::VcdResume => _ = vcd.as_mut().map(|vcd| vcd.paused = false),
                        O::Time => {
                            bits::load_from_u64(stack, dst.0, TIME_VSIZE, ctx.time);
                        }
                        O::Finish => {
                            writeln!(&mut ctx.stdout, "[FINISH]").unwrap();
                            return EvalOutcome::Exit;
                        }
                    }
                }
                I::Probe(var, sig) => {
                    let bits = &signals[sig.0 as usize];
                    match (bits.as_data_ref(), ctx.logic_mode) {
                        (BitsDataRef::InlineTv(v), LogicMode::TwoValue) => {
                            let nbytes = bits.size().get().div_ceil(8) as usize;
                            store_partial_u64(&mut stack[var.0..][..nbytes], v, bits.size());
                        }
                        (BitsDataRef::InlineFv(spc, v), LogicMode::FourValue) => {
                            let nbytes = (2 * bits.size().get()).div_ceil(8) as usize;
                            store_partial_u64(
                                &mut stack[var.0..][..nbytes],
                                fv_pack_u64(spc as u64, v as u64, bits.size()),
                                VectorSize::new(2 * bits.size().get()).unwrap(),
                            );
                        }
                        (BitsDataRef::SeparateTv(v), LogicMode::TwoValue) => {
                            bytemuck::cast_slice_mut::<u8, u64>(&mut stack[var.0..][..v.len() * 8])
                                .copy_from_slice(v);
                        }
                        (BitsDataRef::SeparateFv(v), LogicMode::FourValue) => {
                            bytemuck::cast_slice_mut::<u8, u64>(&mut stack[var.0..][..v.len() * 8])
                                .copy_from_slice(v);
                        }
                        _ => unreachable!(),
                    }
                }
                I::Drive(sig, var, region, partial) => {
                    let size = signals[sig.0 as usize].size();
                    let partial = partial.map(|(offset, width)| {
                        (
                            bits::store_to_u64(&stack, offset.0, INTEGER_VSIZE) as u32,
                            width,
                        )
                    });
                    if *region != 0 {
                        let region = (*region - 1) as usize;
                        let value = Bits::load_from_slice(
                            &stack[var.0..][..size.get().div_ceil(8) as usize],
                            size,
                        );
                        match regions.other_dispatched[region].entry(DispatchKey::Signal(*sig)) {
                            Entry::Occupied(entry) => {
                                let Event::Drive(_, assignments) =
                                    &mut regions.other[region][*entry.get()]
                                else {
                                    unreachable!();
                                };
                                assignments.push((value, partial));
                            }
                            Entry::Vacant(entry) => {
                                let event = Event::Drive(*sig, vec![(value, partial)]);
                                let i = regions.other[region].len();
                                regions.other[region].push(event);
                                entry.insert(i);
                            }
                        }

                        continue;
                    }

                    let signal = &mut signals[sig.0 as usize];
                    let size = partial.map_or(size, |(_, s)| s);
                    let updated = drive_bits(signal, stack, var.0, size, partial, ctx.logic_mode);

                    if updated {
                        update_watchers(
                            *sig,
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
                    return EvalOutcome::Next;
                }
                I::Watch(signals) => {
                    let listener_key = listeners.insert(self);
                    for signal in signals {
                        watches[signal.0 as usize].push(listener_key);
                    }
                    if let Some(trace) = trace.as_mut() {
                        let watch_range_start = trace.watches.len() as u64;
                        trace.watches.extend(signals.iter().map(|s| s.0));
                        let vogls_trace::Event::Evaluation(_, _, stop_reason) =
                            trace.events.last_mut().unwrap()
                        else {
                            unreachable!();
                        };
                        *stop_reason = vogls_trace::EventStopReason::WatchSignals(
                            watch_range_start..trace.watches.len() as u64,
                        );
                    }
                    return EvalOutcome::Next;
                }
                I::Jump(offset) => *ip = *offset,
                I::Branch(cond, true_offset, false_offset) => {
                    let is_true = stack[cond.0] & 1 != 0;
                    if is_true {
                        *ip = *true_offset;
                    } else {
                        *ip = *false_offset;
                    }
                }
                I::Halt => {
                    return EvalOutcome::Next;
                }
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
    signals: &mut [Bits],
    signal_info: &[SignalInfo],
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut [Vec<ListenerKey>],
    mut trace: Option<&mut vogls_trace::Trace>,
    stack: &mut [u8],
    max_time: u64,
) -> Result<(), ()> {
    let mut schedule = BTreeMap::<Timestamp, Vec<Event>>::new();
    let mut vcd = None;
    'region_loop: loop {
        while let Some(event) = regions.active.pop() {
            if let Some(trace) = trace.as_deref_mut() {
                trace.events.push(match &event {
                    Event::Drive(s, _) => {
                        vogls_trace::Event::Drive(s.0, Some(trace.driven.len() as u64))
                    }
                    Event::Evaluation(e) => vogls_trace::Event::Evaluation(
                        e.process.0 as u64,
                        trace.driven.len() as u64..trace.driven.len() as u64,
                        vogls_trace::EventStopReason::Halt,
                    ),
                });
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
                listeners,
                watches,
                stack,
                &mut vcd,
                trace.as_deref_mut(),
            );

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
            vcd.dump_time_step(ctx, signals, signal_info, false)
                .unwrap();
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
        vcd.dump_time_step(ctx, signals, signal_info, true).unwrap();
        vcd.writer.flush().unwrap();
    }

    if cfg!(vm_profile) {
        writeln!(ctx.stdout, "Stats:",).unwrap();
        writeln!(ctx.stdout, "  # Instructions: {}", ctx.instruction_count).unwrap();
        writeln!(ctx.stdout, "  # Events:       {}", ctx.event_count).unwrap();
        writeln!(ctx.stdout, "  # Stack size:   {}", stack.len()).unwrap();
    }

    Ok(())
}
