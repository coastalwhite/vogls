use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use slotmap::{SlotMap, new_key_type};
use vogls_ir::{BinaryOp, Bits, IntrinsicOp, UnaryOp, VectorSize};

mod bits;
mod instruction;

pub use instruction::*;

new_key_type! { pub struct ListenerKey; }
new_key_type! { pub struct VmProcessKey; }

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
}

impl Context {
    pub fn new(stdout: Box<dyn std::io::Write>, stderr: Box<dyn std::io::Write>) -> Self {
        Self {
            time: 0,
            stdout,
            stderr,
            tracing_level: TracingLevel::Events,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Drive(VmSignalKey, Bits, Option<(VectorSize, VectorSize)>),
    Evaluation(EvaluationEvent),
}

#[derive(Clone, Debug)]
pub struct EvaluationEvent {
    /// Which process is scheduled.
    pub process: VmProcessKey,
    /// The stack with which to execute.
    pub bit_stack: Vec<u8>,
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
            "drive woke up {} watchers",
            regions.active.len() - start
        )
        .unwrap();
    }
}

pub fn drive_bits(
    bits: &mut Bits,
    slice: &[u8],
    partial: Option<(VectorSize, VectorSize)>,
) -> bool {
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
                bits::set_subslice(signal_value, slice, *size, offset, length)
            }
        },
        Bits::Small(signal_value, size) => {
            let before = *signal_value;
            match partial {
                None => {
                    *signal_value = bits::store_to_u64(slice, 0, *size);
                }
                Some((offset, length)) => {
                    let value = bits::store_to_u64(slice, 0, length);
                    *signal_value &= !(((1u64 << length) - 1) << offset);
                    *signal_value |= value << offset;
                }
            }
            before != *signal_value
        }
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
        listeners: &mut SlotMap<ListenerKey, Event>,
        watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
    ) -> EvalOutcome {
        let EvaluationEvent {
            process,
            bit_stack,
            ip,
        } = match &mut self {
            Event::Drive(sig, bits, partial) => {
                let signal_bits = signals.get_mut(sig).unwrap();
                let updated = drive_bits(signal_bits, bits.as_slice(), *partial);

                if updated {
                    update_watchers(ctx, *sig, watches, listeners, regions);
                }

                return EvalOutcome::Next;
            }
            Event::Evaluation(e) => e,
        };

        let process = processes.get(*process).unwrap();

        use VmInstruction as I;
        loop {
            let instr = &process.instructions[*ip];
            *ip += 1;
            match instr {
                I::Constant(var, Bits::Small(value, size)) => {
                    bits::load_from_u64(bit_stack, var.offset, *size, *value);
                }
                I::Constant(var, Bits::Big(size, value)) => {
                    bit_stack[var.offset..][..size.div_ceil(8) as usize].copy_from_slice(value);
                }
                I::Unary(dst, op, src) => {
                    use UnaryOp as O;
                    match op {
                        O::Neg(size) => {
                            bit_stack[dst.offset] =
                                bit_stack[src.offset] ^ 1u8.unbounded_shl(size % 8).wrapping_sub(1);
                            for i in 1..size.div_ceil(8) as usize {
                                bit_stack[dst.offset + i] = !bit_stack[src.offset + i];
                            }
                        }
                        O::ReduceOr(size) => {
                            let result = bit_stack[src.offset..][..size.div_ceil(8) as usize]
                                .iter()
                                .any(|b| *b != 0);
                            bit_stack[dst.offset] = u8::from(result);
                        }
                        O::ReduceAnd(size) => {
                            let result = bit_stack[src.offset + 1..][..size.div_ceil(8) as usize]
                                .iter()
                                .all(|b| *b == 0xFF);
                            let mask = 1u8.unbounded_shl(size % 8).wrapping_sub(1);
                            let result = result & (bit_stack[src.offset] & mask == mask);
                            bit_stack[dst.offset] = u8::from(result);
                        }
                        O::ReduceXor(size) => {
                            let mut result = 0;
                            if *size > 0 {
                                result = bit_stack[src.offset..][..size.div_ceil(8) as usize]
                                    .iter()
                                    .map(|b| VectorSize::from(b.count_ones()))
                                    .sum::<VectorSize>();
                            }
                            bit_stack[dst.offset] = u8::from(result % 2 == 1);
                        }
                        O::Slice(n, width) => {
                            bits::slice(bit_stack, dst.offset, src.offset, *width, *n);
                        }
                    };
                }
                I::Binary(dst, op, lhs, rhs) => {
                    use BinaryOp as O;
                    match op {
                        O::And(n) => {
                            for i in 0..n.div_ceil(8) as usize {
                                bit_stack[dst.offset + i] =
                                    bit_stack[lhs.offset + i] & bit_stack[rhs.offset + i]
                            }
                        }
                        O::Or(n) => {
                            for i in 0..n.div_ceil(8) as usize {
                                bit_stack[dst.offset + i] =
                                    bit_stack[lhs.offset + i] | bit_stack[rhs.offset + i];
                            }
                        }
                        O::Xor(n) => {
                            for i in 0..n.div_ceil(8) as usize {
                                bit_stack[dst.offset + i] =
                                    bit_stack[lhs.offset + i] ^ bit_stack[rhs.offset + i];
                            }
                        }
                        O::Add(n) => {
                            let n = *n;
                            if n > 64 {
                                todo!()
                            }
                            let l = bits::store_to_u64(&bit_stack, lhs.offset, n);
                            let r = bits::store_to_u64(&bit_stack, rhs.offset, n);
                            let out = l.wrapping_add(r) & (1u64.unbounded_shl(n)).wrapping_sub(1);
                            bits::load_from_u64(bit_stack, dst.offset, n, out);
                        }
                        O::Sub(n) => {
                            let n = *n;
                            if n > 64 {
                                todo!()
                            }
                            let l = bits::store_to_u64(&bit_stack, lhs.offset, n);
                            let r = bits::store_to_u64(&bit_stack, rhs.offset, n);
                            let out = l.wrapping_sub(r) & (1u64.unbounded_shl(n)).wrapping_sub(1);
                            bits::load_from_u64(bit_stack, dst.offset, n, out);
                        }
                        O::Multiply(n) => {
                            let n = *n;
                            if n > 64 {
                                todo!()
                            }
                            let l = bits::store_to_u64(&bit_stack, lhs.offset, n);
                            let r = bits::store_to_u64(&bit_stack, rhs.offset, n);
                            let out = l.wrapping_mul(r) & (1u64.unbounded_shl(n)).wrapping_sub(1);
                            bits::load_from_u64(bit_stack, dst.offset, n, out);
                        }
                        O::Divide(n) => {
                            let n = *n;
                            if n > 64 {
                                todo!()
                            }
                            let l = bits::store_to_u64(&bit_stack, lhs.offset, n);
                            let r = bits::store_to_u64(&bit_stack, rhs.offset, n);
                            let out = l.wrapping_div(r) & (1u64.unbounded_shl(n)).wrapping_sub(1);
                            bits::load_from_u64(bit_stack, dst.offset, n, out);
                        }
                        O::Modulus(n) => {
                            let n = *n;
                            if n > 64 {
                                todo!()
                            }
                            let l = bits::store_to_u64(&bit_stack, lhs.offset, n);
                            let r = bits::store_to_u64(&bit_stack, rhs.offset, n);
                            let out = l.wrapping_rem(r) & 1u64.unbounded_shl(n).wrapping_sub(1);
                            bits::load_from_u64(bit_stack, dst.offset, n, out);
                        }
                        O::UnsignedLessEqual(n) => {
                            let mut set = false;
                            for i in 0..n.div_ceil(8) as usize {
                                let value = match bit_stack[lhs.offset + i]
                                    .cmp(&bit_stack[rhs.offset + i])
                                {
                                    Ordering::Less => true,
                                    Ordering::Greater => false,
                                    Ordering::Equal => continue,
                                };
                                set = true;
                                bit_stack[dst.offset] = u8::from(value);
                            }
                            if !set {
                                bit_stack[dst.offset] = u8::from(true);
                            }
                        }
                        O::SelectBit(n) => {
                            let idx = bits::store_to_u64(&bit_stack, rhs.offset, 32);
                            assert!(idx < *n as u64);
                            let idx = idx as VectorSize;

                            bit_stack[dst.offset] =
                                (bit_stack[lhs.offset + (idx / 8) as usize] >> (idx % 8)) & 1;
                        }
                        O::LogicalShiftLeft(..) => todo!(),
                        O::LogicalShiftRight(n, shift_n) => {
                            let shift = bits::store_to_u64(&bit_stack, rhs.offset, *shift_n);
                            if shift as VectorSize >= *n {
                                for i in 0..n.div_ceil(8) as usize {
                                    bit_stack[dst.offset + i] = 0;
                                }
                                continue;
                            }
                            bits::logical_shift_right(
                                bit_stack,
                                dst.offset,
                                lhs.offset,
                                shift as VectorSize,
                                *n,
                            );
                        }
                        O::ArithmeticShiftLeft(..) => todo!(),
                        O::ArithmeticShiftRight(..) => todo!(),
                        O::Concat(lhs_size, rhs_size) => bits::concat(
                            bit_stack, dst.offset, lhs.offset, rhs.offset, *lhs_size, *rhs_size,
                        ),
                    };
                }

                I::Cast(dst, dst_size, src, src_size) => {
                    assert!(dst_size >= src_size);
                    for i in 0..src_size.div_ceil(8) as usize {
                        bit_stack[dst.offset + i] = bit_stack[src.offset + i];
                    }
                    for i in src_size.div_ceil(8) as usize..dst_size.div_ceil(8) as usize {
                        bit_stack[dst.offset + i] = 0;
                    }
                }
                I::Move(dst, src, size) => {
                    for i in 0..size.div_ceil(8) as usize {
                        bit_stack[dst.offset + i] = bit_stack[src.offset + i];
                    }
                }

                I::Intrinsic(op, args) => {
                    use IntrinsicOp as O;

                    match op {
                        O::Display => {
                            assert_eq!(args.len(), 1);
                            let msg = match args.first().unwrap() {
                                VmIntrinsicArg::StringLiteral(s) => s.clone(),
                                VmIntrinsicArg::Variable(_, n) if *n >= 64 => todo!(),
                                VmIntrinsicArg::Variable(s, n) => {
                                    format!("{n}'d{}", bits::store_to_u64(bit_stack, s.offset, *n))
                                }
                            };
                            writeln!(&mut ctx.stdout, "[DISPLAY]: time = {}: {msg}", ctx.time)
                                .unwrap();
                        }
                        O::Assert => {
                            let value = match args.first() {
                                Some(VmIntrinsicArg::Variable(condition, size)) => {
                                    bit_stack[condition.offset..][..size.div_ceil(8) as usize]
                                        .iter()
                                        .any(|b| *b != 0)
                                }
                                _ => {
                                    panic!("Invalid assert argument");
                                }
                            };
                            if !value {
                                writeln!(&mut ctx.stderr, "Assert failed.").unwrap();
                                return EvalOutcome::Error;
                            }
                        }
                        O::AssertEq(eq) => {
                            use VmIntrinsicArg as A;
                            match (&args[0], &args[1]) {
                                (A::Variable(l, ls), A::Variable(r, rs)) => {
                                    if ls != rs {
                                        writeln!(
                                            &mut ctx.stderr,
                                            "assert_eq failed. sizes different {ls} != {rs}"
                                        )
                                        .unwrap();
                                        return EvalOutcome::Error;
                                    }
                                    let lhs = &bit_stack[l.offset..][..ls.div_ceil(8) as usize];
                                    let rhs = &bit_stack[r.offset..][..rs.div_ceil(8) as usize];
                                    if *eq != (lhs == rhs) {
                                        writeln!(
                                            &mut ctx.stderr,
                                            "assert_{{eq, ne}} failed. {lhs:?} != {rhs:?}"
                                        )
                                        .unwrap();
                                        return EvalOutcome::Error;
                                    }
                                }
                                _ => {
                                    writeln!(
                                        &mut ctx.stderr,
                                        "assert_eq({eq}) failed. {:?} != {:?}",
                                        args[0], args[1]
                                    )
                                    .unwrap();
                                    return EvalOutcome::Error;
                                }
                            }
                        }
                        O::Finish => {
                            writeln!(&mut ctx.stdout, "[FINISH]").unwrap();
                            return EvalOutcome::Exit;
                        }
                    }
                }
                I::Probe(var, sig) => match signals.get(&sig).unwrap() {
                    Bits::Small(value, size) => {
                        bits::load_from_u64(bit_stack, var.offset, *size, *value);
                    }
                    Bits::Big(size, value) => {
                        bit_stack[var.offset..][..size.div_ceil(8) as usize].copy_from_slice(value);
                    }
                },
                I::Drive(sig, var, region, partial) => {
                    let size = signals[sig].size();
                    let partial = partial.map(|(offset, width)| {
                        (
                            bits::store_to_u64(&bit_stack, offset.offset, 32) as VectorSize,
                            width,
                        )
                    });
                    if *region != 0 {
                        let value = Bits::load_from_slice(&bit_stack[var.offset..], size);
                        regions.other[(region - 1) as usize]
                            .push(Event::Drive(*sig, value, partial));
                        continue;
                    }

                    let signal = signals.get_mut(sig).unwrap();
                    let size = partial.map_or(size, |(_, s)| s);
                    let updated = drive_bits(
                        signal,
                        &bit_stack[var.offset..][..size.div_ceil(8) as usize],
                        partial,
                    );

                    if updated {
                        update_watchers(ctx, *sig, watches, listeners, regions);
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
                    let is_true = bit_stack[cond.offset] & 1 != 0;
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

pub fn run(
    ctx: &mut Context,
    processes: &SlotMap<VmProcessKey, VmProcess>,
    regions: &mut Regions,
    signals: &mut HashMap<VmSignalKey, Bits>,
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
    max_time: u64,
) -> Result<(), ()> {
    let mut schedule = BTreeMap::<Timestamp, Vec<Event>>::new();
    'region_loop: loop {
        while let Some(event) = regions.active.pop() {
            if ctx.tracing_level >= TracingLevel::Events {
                match &event {
                    Event::Drive(signal, _, _) => {
                        writeln!(&mut ctx.stdout, "drive {signal:?}").unwrap()
                    }
                    Event::Evaluation(eval) => {
                        writeln!(&mut ctx.stdout, "eval {:?}", eval.process).unwrap()
                    }
                }
            }

            let outcome = event.evaluate(
                ctx,
                processes,
                &mut schedule,
                regions,
                signals,
                listeners,
                watches,
            );

            match outcome {
                EvalOutcome::Next => continue,
                EvalOutcome::Error => return Err(()),
                EvalOutcome::Exit => return Ok(()),
            }
        }

        for region in &mut regions.other {
            if !region.is_empty() {
                std::mem::swap(&mut regions.active, region);
                continue 'region_loop;
            }
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

    Ok(())
}
