use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;

use slotmap::{SlotMap, new_key_type};
use vogls_bits::get_disjoint_dst_src;
use vogls_ir::dyn_format_string::{Base, Padding, format_bits};
use vogls_ir::vcd::NetType;
use vogls_ir::{Bits, INTEGER_VSIZE, ResizeOp, SignalKey, TIME_VSIZE, UnaryOp, VectorSize};

mod bits;
mod instruction;

pub use instruction::*;

new_key_type! { pub struct ListenerKey; }
new_key_type! { pub struct VmProcessKey; }

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TracingLevel {
    None,
    Events,
}

pub struct Context {
    time: Timestamp,
    pub stdout: Box<dyn std::io::Write>,
    pub stderr: Box<dyn std::io::Write>,
    pub tracing_level: TracingLevel,
    pub instruction_count: u64,
    pub event_count: u64,
}

impl Context {
    pub fn new(stdout: Box<dyn std::io::Write>, stderr: Box<dyn std::io::Write>) -> Self {
        Self {
            time: 0,
            stdout,
            stderr,
            tracing_level: TracingLevel::Events,
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
    ctx: &mut Context,
    sig: VmSignalKey,
    watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
    listeners: &mut SlotMap<ListenerKey, Event>,
    regions: &mut Regions,
    signal_info: &[SignalInfo],
) {
    let start = regions.active.len();
    if let Some(watchers) = watches.remove(&sig) {
        for watcher in watchers {
            if let Some(event) = listeners.remove(watcher) {
                regions.active.push(event);
            }
        }
    }

    if ctx.tracing_level >= TracingLevel::Events {
        writeln!(
            ctx.stdout,
            "drive of {} woke up {:?}",
            signal_info[sig.0 as usize].name,
            regions.active[start..]
                .iter()
                .map(|e| match e {
                    Event::Drive(signal, _) =>
                        format!("drive {}", signal_info[signal.0 as usize].name),
                    Event::Evaluation(event) => format!("{:?}", event.process),
                })
                .collect::<Vec<_>>()
        )
        .unwrap();
    }
}

pub fn drive_bits(bits: &mut Bits, slice: &[u8], partial: Option<(u32, VectorSize)>) -> bool {
    match bits {
        Bits::Big(size, signal_value) => match partial {
            None => {
                if slice == signal_value.as_ref() {
                    return false;
                }
                signal_value.copy_from_slice(slice);
                true
            }
            Some((offset, length)) => {
                vogls_bits::set_subslice::set_subslice(signal_value, slice, *size, offset, length)
            }
        },
        Bits::Small(signal_value, size) => {
            let before = *signal_value;
            match partial {
                None => {
                    eprintln!("signal_value = {signal_value:08X}");
                    *signal_value = bits::store_to_u64(slice, 0, *size);
                    eprintln!("signal_value = {signal_value:08X}");
                }
                Some((offset, length)) => {
                    let value = bits::store_to_u64(slice, 0, length);
                    *signal_value &= !(((1u64 << length.get()) - 1) << offset);
                    *signal_value |= value << offset;
                }
            }
            before != *signal_value
        }
    }
}

#[derive(Debug, Clone)]
pub struct VcdScope {
    pub name: String,
    pub items: Vec<VcdScopeItem>,
}

impl VcdScope {
    fn lower(v: &vogls_ir::vcd::VcdScope, map: &mut HashMap<SignalKey, VmSignalKey>) -> VcdScope {
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
    fn lower(v: &vogls_ir::vcd::VcdScopeItem, map: &mut HashMap<SignalKey, VmSignalKey>) -> Self {
        match v {
            vogls_ir::vcd::VcdScopeItem::Scope(v) => Self::Scope(VcdScope::lower(v, map)),
            vogls_ir::vcd::VcdScopeItem::Variable(v) => {
                let next = map.len();
                Self::Variable(VcdVariable {
                    signal: *map.entry(v.signal).or_insert(VmSignalKey(next as _)),
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
        signals: &HashMap<VmSignalKey, Bits>,
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

        dbg!(self.updated_this_time_step.contains(&VmSignalKey(83)));

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
            let bits = &signals[&signal];
            if signal.0 == 83 {
                dbg!(&bits);
            }
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
        processes: &SlotMap<VmProcessKey, VmProcess>,
        schedule: &mut BTreeMap<Timestamp, Vec<Event>>,
        regions: &mut Regions,
        signals: &mut HashMap<VmSignalKey, Bits>,
        signal_info: &[SignalInfo],
        listeners: &mut SlotMap<ListenerKey, Event>,
        watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
        stack: &mut [u8],
        vcd: &mut Option<VcdOutput>,
    ) -> EvalOutcome {
        let EvaluationEvent {
            process: process_key,
            ip,
        } = match &mut self {
            Event::Drive(sig, assignments) => {
                let signal_bits = signals.get_mut(sig).unwrap();
                let mut updated = false;
                for (bits, partial) in assignments {
                    updated |= drive_bits(signal_bits, bits.as_slice(), *partial);
                }

                if updated {
                    update_watchers(ctx, *sig, watches, listeners, regions, signal_info);
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

        let process = processes.get(*process_key).unwrap();

        use VmInstruction as I;
        loop {
            let instr = &process.instructions[*ip];
            *ip += 1;
            ctx.instruction_count += 1;
            match instr {
                I::Constant(var, Bits::Small(value, size)) => {
                    bits::load_from_u64(stack, var.offset, *size, *value);
                }
                I::Constant(var, Bits::Big(size, value)) => {
                    stack[var.offset..][..size.get().div_ceil(8) as usize].copy_from_slice(value);
                }
                I::Unary(dst, op, size, src) => {
                    use UnaryOp as O;
                    match op {
                        O::Neg => {
                            let n_full_bytes = (size.get() / 8) as usize;
                            if size.get() % 8 == 0 {
                                for i in 0..n_full_bytes {
                                    stack[dst.offset + i] = !stack[src.offset + i];
                                }
                            } else {
                                stack[dst.offset + n_full_bytes] = stack[src.offset + n_full_bytes]
                                    ^ 1u8.unbounded_shl(size.get() % 8).wrapping_sub(1);
                                for i in 0..n_full_bytes {
                                    stack[dst.offset + i] = !stack[src.offset + i];
                                }
                            }
                        }
                        O::ReduceOr => {
                            let result = stack[src.offset..][..size.get().div_ceil(8) as usize]
                                .iter()
                                .any(|b| *b != 0);
                            stack[dst.offset] = u8::from(result);
                        }
                        O::ReduceAnd => {
                            let result = stack[src.offset + 1..][..size.get().div_ceil(8) as usize]
                                .iter()
                                .all(|b| *b == 0xFF);
                            let mask = 1u8.unbounded_shl(size.get() % 8).wrapping_sub(1);
                            let result = result & (stack[src.offset] & mask == mask);
                            stack[dst.offset] = u8::from(result);
                        }
                        O::ReduceXor => {
                            let mut result = 0;
                            if size.get() > 0 {
                                result = stack[src.offset..][..size.get().div_ceil(8) as usize]
                                    .iter()
                                    .map(|b| b.count_ones())
                                    .sum::<u32>();
                            }
                            stack[dst.offset] = u8::from(result % 2 == 1);
                        }
                    };
                }
                I::Resize(dst, op, dst_size, src_size, src) => {
                    use ResizeOp as O;
                    match op {
                        O::ZeroExtend => {
                            assert!(dst_size >= src_size);
                            for i in 0..src_size.get().div_ceil(8) as usize {
                                stack[dst.offset + i] = stack[src.offset + i];
                            }
                            for i in src_size.get().div_ceil(8) as usize
                                ..dst_size.get().div_ceil(8) as usize
                            {
                                stack[dst.offset + i] = 0;
                            }
                        }
                        O::SignExtend => {
                            assert!(dst_size >= src_size);
                            let sign_offset = src_size.get() - 1;
                            let sign = (stack[src.offset + (sign_offset / 8) as usize]
                                >> (sign_offset % 8))
                                & 1;
                            let mask = u8::from(sign == 0).wrapping_sub(1);
                            if src_size.get() % 8 == 0 {
                                for i in 0..(src_size.get() / 8) as usize {
                                    stack[dst.offset + i] = stack[src.offset + i];
                                }
                                for i in (src_size.get() / 8) as usize
                                    ..dst_size.get().div_ceil(8) as usize
                                {
                                    stack[dst.offset + i] = mask;
                                }
                            } else {
                                let sbytes = src_size.get().div_ceil(8) as usize;
                                for i in 0..sbytes - 1 {
                                    stack[dst.offset + i] = stack[src.offset + i];
                                }
                                stack[dst.offset + sbytes - 1] =
                                    stack[src.offset + sbytes - 1] | (mask << (src_size.get() % 8));
                                for i in sbytes..dst_size.get().div_ceil(8) as usize {
                                    stack[dst.offset + i] = mask;
                                }
                            }
                        }
                        O::Truncate => {
                            let (dst, src) = get_disjoint_dst_src(
                                stack,
                                dst.offset,
                                dst_size.get().div_ceil(8) as usize,
                                src.offset,
                                src_size.get().div_ceil(8) as usize,
                            );
                            vogls_bits::slice::tv_slice(dst, src, *dst_size);
                        }
                    };
                }
                I::BinaryArithmetic(dst, op, size, lhs, rhs) => {
                    use BinaryArithmeticOp as O;
                    let f = match op {
                        O::And => vogls_bits::arithmetic::tv_bitwise_and,
                        O::Or => vogls_bits::arithmetic::tv_bitwise_or,
                        O::Xor => vogls_bits::arithmetic::tv_bitwise_xor,
                        O::Add => vogls_bits::arithmetic::tv_addition,
                        O::Sub => vogls_bits::arithmetic::tv_subtraction,
                        O::Multiply => vogls_bits::arithmetic::tv_multiplication,
                        O::Divide => vogls_bits::arithmetic::tv_division,
                        O::Modulus => vogls_bits::arithmetic::tv_modulus,
                    };

                    let nbytes = size.get().div_ceil(8) as usize;

                    let (dst, lhs, rhs) = vogls_bits::get_disjoint_dst_s1_s2(
                        stack, dst.offset, nbytes, lhs.offset, nbytes, rhs.offset, nbytes,
                    );

                    f(dst, lhs, rhs, *size);
                }
                I::BinaryComparison(dst, op, size, lhs, rhs) => {
                    use BinaryComparisonOp as O;
                    let f = match op {
                        O::UnsignedLessEqual => vogls_bits::comparison::tv_unsigned_leq,
                    };
                    let nbytes = size.get().div_ceil(8) as usize;
                    let lhs = &stack[lhs.offset..][..nbytes];
                    let rhs = &stack[rhs.offset..][..nbytes];
                    let result = f(lhs, rhs, *size);
                    stack[dst.offset] = u8::from(result);
                }
                I::Shift(dst, op, size, src, offset) => {
                    use ShiftOp as O;
                    let f = match op {
                        O::LogicalLeft => vogls_bits::shift::tv_logical_shift_left,
                        O::LogicalRight => vogls_bits::shift::tv_logical_shift_right,
                        O::ArithmeticRight => vogls_bits::shift::tv_arithmetic_shift_right,
                    };

                    let offset = vogls_bits::load::load_full_u32(&stack[offset.offset..]);
                    let nbytes = size.get().div_ceil(8) as usize;

                    let (dst, src) = vogls_bits::get_disjoint_dst_src(
                        stack, dst.offset, nbytes, src.offset, nbytes,
                    );
                    f(dst, src, offset, *size);
                }
                I::SelectBit(dst, size, src, idx) => {
                    let idx = vogls_bits::load::load_full_u32(&stack[idx.offset..]);
                    let nbytes = size.get().div_ceil(8) as usize;

                    let src = &stack[src.offset..][..nbytes];
                    stack[dst.offset] =
                        u8::from(vogls_bits::select::tv_select_bit(src, idx, *size));
                }
                I::Concat(dst, lhs_size, lhs, rhs_size, rhs) => {
                    let lbytes = lhs_size.get().div_ceil(8) as usize;
                    let rbytes = rhs_size.get().div_ceil(8) as usize;
                    let dbytes =
                        (lhs_size.get().checked_add(rhs_size.get()).unwrap()).div_ceil(8) as usize;

                    let (dst, lhs, rhs) = vogls_bits::get_disjoint_dst_s1_s2(
                        stack, dst.offset, dbytes, lhs.offset, lbytes, rhs.offset, rbytes,
                    );

                    vogls_bits::concat::tv_concat(dst, lhs, rhs, *lhs_size, *rhs_size);
                }

                I::Move(dst, src, size) => {
                    for i in 0..size.get().div_ceil(8) as usize {
                        stack[dst.offset + i] = stack[src.offset + i];
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
                                        &stack[o.offset..][..s.get().div_ceil(8) as usize],
                                        *s,
                                    )
                                }),
                            )
                            .unwrap();
                        }
                        O::Assert(f) => {
                            let condition = stack[args[0].0.offset] != 0;
                            if !condition {
                                f.write_to(
                                    &mut ctx.stdout,
                                    args[1..].iter().map(|(o, s)| {
                                        Bits::load_from_slice(
                                            &stack[o.offset..][..s.get().div_ceil(8) as usize],
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
                            bits::load_from_u64(stack, dst.offset, TIME_VSIZE, ctx.time);
                        }
                        O::Finish => {
                            writeln!(&mut ctx.stdout, "[FINISH]").unwrap();
                            return EvalOutcome::Exit;
                        }
                    }
                }
                I::Probe(var, sig) => match signals.get(&sig).unwrap() {
                    Bits::Small(value, size) => {
                        bits::load_from_u64(stack, var.offset, *size, *value);
                    }
                    Bits::Big(size, value) => {
                        stack[var.offset..][..size.get().div_ceil(8) as usize]
                            .copy_from_slice(value);
                    }
                },
                I::Drive(sig, var, region, partial) => {
                    let size = signals[sig].size();
                    let partial = partial.map(|(offset, width)| {
                        (
                            bits::store_to_u64(&stack, offset.offset, INTEGER_VSIZE) as u32,
                            width,
                        )
                    });
                    if *region != 0 {
                        let region = (*region - 1) as usize;
                        let value = Bits::load_from_slice(&stack[var.offset..], size);
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


                    let signal = signals.get_mut(sig).unwrap();
                    if signal_info[sig.0 as usize].name == "mem_rdata_latched" {
                        dbg!(&signal);
                    }
                    let size = partial.map_or(size, |(_, s)| s);
                    let updated = drive_bits(
                        signal,
                        &stack[var.offset..][..size.get().div_ceil(8) as usize],
                        partial,
                    );

                    if signal_info[sig.0 as usize].name == "mem_rdata_latched" {
                        dbg!(*process_key, updated, signal);
                        if let Some(vcd) = vcd.as_mut() {
                            dbg!(vcd.paused, vcd.tracked.contains_key(sig));
                        }
                        dbg!(sig.0);
                    }

                    if updated {
                        update_watchers(ctx, *sig, watches, listeners, regions, signal_info);
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
                    if ctx.tracing_level >= TracingLevel::Events {
                        writeln!(ctx.stdout, "event waiting for time {}.", ctx.time + time.0)
                            .unwrap();
                    }
                    return EvalOutcome::Next;
                }
                I::WaitRegion(region) => {
                    if *region == 0 {
                        regions.active.push(self);
                    } else {
                        regions.other[*region as usize - 1].push(self);
                    }
                    if ctx.tracing_level >= TracingLevel::Events {
                        writeln!(ctx.stdout, "event waiting for region {region}.").unwrap();
                    }
                    return EvalOutcome::Next;
                }
                I::Watch(signals) => {
                    let listener_key = listeners.insert(self);
                    for signal in signals {
                        watches.entry(*signal).or_default().push(listener_key);
                    }
                    if ctx.tracing_level >= TracingLevel::Events {
                        writeln!(ctx.stdout, "event watching for {signals:?}.").unwrap();
                    }
                    return EvalOutcome::Next;
                }
                I::Jump(offset) => *ip = *offset,
                I::Branch(cond, true_offset, false_offset) => {
                    let is_true = stack[cond.offset] & 1 != 0;
                    if is_true {
                        *ip = *true_offset;
                    } else {
                        *ip = *false_offset;
                    }
                }
                I::Halt => {
                    if ctx.tracing_level >= TracingLevel::Events {
                        writeln!(ctx.stdout, "event halted.").unwrap();
                    }
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
    processes: &SlotMap<VmProcessKey, VmProcess>,
    regions: &mut Regions,
    signals: &mut HashMap<VmSignalKey, Bits>,
    signal_info: &[SignalInfo],
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
    stack: &mut [u8],
    max_time: u64,
) -> Result<(), ()> {
    let mut schedule = BTreeMap::<Timestamp, Vec<Event>>::new();
    let mut vcd = None;
    'region_loop: loop {
        while let Some(event) = regions.active.pop() {
            if ctx.tracing_level >= TracingLevel::Events {
                match &event {
                    Event::Drive(signal, _) => writeln!(
                        &mut ctx.stdout,
                        "drive {}",
                        signal_info[signal.0 as usize].name
                    )
                    .unwrap(),
                    Event::Evaluation(eval) => {
                        writeln!(&mut ctx.stdout, "eval {:?}", eval.process).unwrap()
                    }
                }
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
                signal_info,
                listeners,
                watches,
                stack,
                &mut vcd,
            );

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
