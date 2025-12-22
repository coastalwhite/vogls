use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use slotmap::{SlotMap, new_key_type};
use vogls_ir::{
    BinaryOp, Bits, IntrinsicOp, Type, TypeInfo, TypeTable, UnaryOp, Value, VectorSize,
};

#[derive(PartialEq, Eq)]
pub enum SignalValue {
    Bits(Bits),
    BitsArray(Box<[Bits]>),
    Decimal(i64),
    DecimalArray(Box<[i64]>),
}

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

pub struct Context {
    time: Timestamp,
    pub stdout: Box<dyn std::io::Write>,
    pub stderr: Box<dyn std::io::Write>,
}

impl Context {
    pub fn new(stdout: Box<dyn std::io::Write>, stderr: Box<dyn std::io::Write>) -> Self {
        Self {
            time: 0,
            stdout,
            stderr,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Drive(
        VmSignalKey,
        Value,
        Option<u32>,
        Option<(VectorSize, VectorSize)>,
    ),
    Evaluation(EvaluationEvent),
}

#[derive(Clone, Debug)]
pub struct EvaluationEvent {
    /// Which process is scheduled.
    pub process: VmProcessKey,
    /// The stack with which to execute.
    pub bit_stack: Vec<u8>,
    pub decimal_stack: Vec<i64>,
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
    watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
    listeners: &mut SlotMap<ListenerKey, Event>,
    regions: &mut Regions,
) {
    if let Some(watchers) = watches.remove(&sig) {
        for watcher in watchers {
            if let Some(event) = listeners.remove(watcher) {
                regions.active.push(event);
            }
        }
    }
}

pub fn drive_bits(
    bits: &mut Bits,
    slice: &[u8],
    partial: Option<(VectorSize, VectorSize)>,
) -> bool {
    match bits {
        Bits::Big(_, signal_value) => {
            if slice == signal_value.as_ref() {
                return false;
            }

            match partial {
                None => signal_value.copy_from_slice(slice),
                Some(_) => todo!(),
            }

            true
        }
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
        signals: &mut HashMap<VmSignalKey, SignalValue>,
        signal_ty: &HashMap<VmSignalKey, TypeInfo>,
        listeners: &mut SlotMap<ListenerKey, Event>,
        watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
        types: &TypeTable,
    ) -> EvalOutcome {
        let EvaluationEvent {
            process,
            bit_stack,
            decimal_stack,
            ip,
        } = match &mut self {
            Event::Drive(sig, value, idx, partial) => {
                let updated = match signals.get_mut(sig).unwrap() {
                    SignalValue::Bits(signal_bits) => {
                        assert!(idx.is_none());
                        let Value::Bits(bits) = value else {
                            unreachable!()
                        };
                        drive_bits(signal_bits, bits.as_slice(), *partial)
                    }
                    SignalValue::Decimal(signal_value) => {
                        assert!(idx.is_none());
                        assert!(partial.is_none());
                        let Value::Decimal(value) = value else {
                            unreachable!()
                        };
                        let before = *signal_value;
                        *signal_value = *value;
                        before != *signal_value
                    }
                    SignalValue::BitsArray(signal_bits) => {
                        let Some(idx) = idx else { unreachable!() };
                        let Value::Bits(bits) = value else {
                            unreachable!()
                        };
                        drive_bits(&mut signal_bits[*idx as usize], bits.as_slice(), *partial)
                    }
                    SignalValue::DecimalArray(signal_value) => {
                        assert!(partial.is_none());
                        let Some(idx) = idx else { unreachable!() };
                        let Value::Decimal(value) = value else {
                            unreachable!()
                        };
                        let before = signal_value[*idx as usize];
                        signal_value[*idx as usize] = *value;
                        before != signal_value[*idx as usize]
                    }
                };

                if updated {
                    update_watchers(*sig, watches, listeners, regions);
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
                I::ConstantBit(var, Bits::Small(value, size)) => {
                    bits::load_from_u64(bit_stack, var.offset, *size, *value);
                }
                I::ConstantBit(var, Bits::Big(size, value)) => {
                    bit_stack[var.offset..][..size.div_ceil(8) as usize].copy_from_slice(value);
                }
                I::Unary(dst, op, src) => {
                    use UnaryOp as O;
                    match op {
                        O::BitNeg(size) => {
                            if *size != 1 {
                                todo!()
                            }
                            bit_stack[dst.offset] = u8::from(bit_stack[src.offset] == 0)
                        }
                        O::BitReduceOr(size) => {
                            let result = bit_stack[src.offset..][..size.div_ceil(8) as usize]
                                .iter()
                                .any(|b| *b != 0);
                            bit_stack[dst.offset] = u8::from(result);
                        }
                        O::BitReduceAnd(size) => {
                            let num_bytes = size.div_ceil(8) as usize;
                            let result = bit_stack[src.offset..][..num_bytes - 1]
                                .iter()
                                .all(|b| *b == 0xFF);
                            let mask = (1u8 << size % 8).wrapping_sub(1);
                            let result = result & (bit_stack[num_bytes - 1] & mask == mask);
                            bit_stack[dst.offset] = u8::from(result);
                        }
                        O::BitReduceXor(size) => {
                            let mut result = 0;
                            if *size > 0 {
                                result = bit_stack[src.offset..][..size.div_ceil(8) as usize]
                                    .iter()
                                    .map(|b| VectorSize::from(b.count_ones()))
                                    .sum::<VectorSize>();
                            }
                            bit_stack[dst.offset] = u8::from(result % 2 == 1);
                        }
                        O::BitSlice(n, width) => {
                            bits::slice(bit_stack, dst.offset, src.offset, *width, *n);
                        }

                        O::DecimalNeg => decimal_stack[dst.offset] = !decimal_stack[src.offset],
                        O::DecimalReduceAnd => {
                            bit_stack[dst.offset] = u8::from(!decimal_stack[src.offset] == 0)
                        }
                        O::DecimalReduceOr => {
                            bit_stack[dst.offset] = u8::from(decimal_stack[src.offset] != 0)
                        }
                        O::DecimalReduceXor => {
                            bit_stack[dst.offset] =
                                u8::from(decimal_stack[src.offset].count_ones() % 2 != 0)
                        }
                    };
                }
                I::Binary(dst, op, lhs, rhs) => {
                    use BinaryOp as O;
                    match op {
                        O::BitAnd(n) => {
                            for i in 0..n.div_ceil(8) as usize {
                                bit_stack[dst.offset + i] =
                                    bit_stack[lhs.offset + i] & bit_stack[rhs.offset + i]
                            }
                        }
                        O::BitOr(n) => {
                            for i in 0..n.div_ceil(8) as usize {
                                bit_stack[dst.offset + i] =
                                    bit_stack[lhs.offset + i] | bit_stack[rhs.offset + i];
                            }
                        }
                        O::BitXor(n) => {
                            for i in 0..n.div_ceil(8) as usize {
                                bit_stack[dst.offset + i] =
                                    bit_stack[lhs.offset + i] ^ bit_stack[rhs.offset + i];
                            }
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
                        O::DecimalAnd => {
                            decimal_stack[dst.offset] =
                                decimal_stack[lhs.offset] & decimal_stack[rhs.offset]
                        }
                        O::DecimalOr => {
                            decimal_stack[dst.offset] =
                                decimal_stack[lhs.offset] | decimal_stack[rhs.offset]
                        }
                        O::DecimalXor => {
                            decimal_stack[dst.offset] =
                                decimal_stack[lhs.offset] ^ decimal_stack[rhs.offset]
                        }
                        O::DecimalAdd => {
                            decimal_stack[dst.offset] =
                                decimal_stack[lhs.offset] + decimal_stack[rhs.offset]
                        }
                        O::DecimalMultiply => {
                            decimal_stack[dst.offset] =
                                decimal_stack[lhs.offset] * decimal_stack[rhs.offset]
                        }
                        O::DecimalDivide => {
                            decimal_stack[dst.offset] =
                                decimal_stack[lhs.offset] / decimal_stack[rhs.offset]
                        }
                        O::DecimalModulus => {
                            decimal_stack[dst.offset] =
                                decimal_stack[lhs.offset] % decimal_stack[rhs.offset]
                        }
                        O::DecimalSub => {
                            decimal_stack[dst.offset] =
                                decimal_stack[lhs.offset] - decimal_stack[rhs.offset]
                        }
                        O::DecimalLessEqual => {
                            bit_stack[dst.offset] =
                                u8::from(decimal_stack[lhs.offset] <= decimal_stack[rhs.offset])
                        }
                        O::SelectBit(n) => {
                            let idx = decimal_stack[rhs.offset];
                            assert!(idx >= 0 && idx < *n as i64);
                            let idx = idx as VectorSize;

                            let byte_offset = n.div_ceil(8) - 1 - (idx / 8);
                            bit_stack[dst.offset] =
                                (bit_stack[lhs.offset + byte_offset as usize] >> (idx % 8)) & 1;
                        }
                        O::LogicalShiftRight(n) => {
                            let shift = decimal_stack[rhs.offset];
                            assert!(shift >= 0 && shift < *n as i64);
                            bits::logical_shift_right(
                                bit_stack,
                                dst.offset,
                                lhs.offset,
                                shift as VectorSize,
                                *n,
                            );
                        }
                        O::Concat(lhs_size, rhs_size) => bits::concat(
                            bit_stack, dst.offset, lhs.offset, rhs.offset, *lhs_size, *rhs_size,
                        ),
                    };
                }

                I::ConstantDecimal(var, val) => decimal_stack[var.offset] = *val,

                I::Cast(dst, dst_ty, src, src_ty) => {
                    use vogls_ir::Type as T;
                    match (types[*dst_ty], types[*src_ty]) {
                        (T::Bits(x), T::Bits(y)) if x == y => {}
                        (T::Decimal, T::Decimal) => {}

                        (T::Bits(m), T::Bits(n)) if n < m => {
                            for i in 0..n.div_ceil(8) as usize {
                                bit_stack[dst.offset + i] = bit_stack[src.offset + i];
                            }
                        }

                        (T::Bits(1), T::Decimal) => {
                            let src = decimal_stack[src.offset];
                            bit_stack[dst.offset] = (src != 0) as u8;
                        }
                        (T::Bits(n), T::Decimal) if n < 64 => {
                            let src = decimal_stack[src.offset];
                            assert!(src >= 0);
                            bits::load_from_u64(bit_stack, dst.offset, n, src as u64);
                        }
                        (T::Decimal, T::Bits(1)) => {
                            let src = bit_stack[src.offset];
                            decimal_stack[dst.offset] = src as i64;
                        }
                        _ => todo!("cast: {:?} -> {:?}", types[*src_ty], types[*dst_ty]),
                    }
                }
                I::Move(dst, src, ty) => {
                    use vogls_ir::Type as T;
                    match types[*ty] {
                        T::Bits(n) => {
                            for i in 0..n.div_ceil(8) as usize {
                                bit_stack[dst.offset + i] = bit_stack[src.offset + i];
                            }
                        }
                        T::Decimal => {
                            decimal_stack[dst.offset] = decimal_stack[src.offset];
                        }
                    }
                }

                I::Intrinsic(op, args) => {
                    use IntrinsicOp as O;

                    match op {
                        O::Display => {
                            assert_eq!(args.len(), 1);
                            let msg = match args.first().unwrap() {
                                VmIntrinsicArg::StringLiteral(s) => s.clone(),
                                VmIntrinsicArg::VariableBits(_, n) if *n >= 64 => todo!(),
                                VmIntrinsicArg::VariableBits(s, n) => {
                                    format!("{n}'d{}", bits::store_to_u64(bit_stack, s.offset, *n))
                                }
                                VmIntrinsicArg::VariableDecimal(s) => {
                                    decimal_stack[s.offset].to_string()
                                }
                            };
                            writeln!(&mut ctx.stdout, "[DISPLAY]: time = {}: {msg}", ctx.time)
                                .unwrap();
                        }
                        O::Assert => {
                            let value = match args.first() {
                                Some(VmIntrinsicArg::VariableBits(condition, size)) => {
                                    if *size != 1 {
                                        todo!()
                                    }
                                    bit_stack[condition.offset] != 0
                                }
                                Some(VmIntrinsicArg::VariableDecimal(condition)) => {
                                    decimal_stack[condition.offset] != 0
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
                                (A::VariableBits(l, ls), A::VariableBits(r, rs)) => {
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
                                (A::VariableDecimal(l), A::VariableDecimal(r)) => {
                                    let l = decimal_stack[l.offset];
                                    let r = decimal_stack[r.offset];
                                    if (l == r) != *eq {
                                        writeln!(
                                            &mut ctx.stderr,
                                            "assert_{{eq,ne}} failed. {l} == {r}"
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
                I::Probe(var, sig) => {
                    match signals.get(&sig).unwrap() {
                        SignalValue::Bits(Bits::Small(value, size)) => {
                            bits::load_from_u64(bit_stack, var.offset, *size, *value);
                        }
                        SignalValue::Bits(Bits::Big(size, value)) => {
                            bit_stack[var.offset..][..size.div_ceil(8) as usize]
                                .copy_from_slice(value);
                        }
                        SignalValue::Decimal(v) => decimal_stack[var.offset] = *v,
                        SignalValue::BitsArray(_) => unreachable!(),
                        SignalValue::DecimalArray(_) => unreachable!(),
                    };
                }
                I::Drive(sig, var, region, partial) => {
                    if *region != 0 {
                        let partial = partial.map(|(offset, width)| {
                            (decimal_stack[offset.offset] as VectorSize, width)
                        });
                        let value = match types[signal_ty[sig].key] {
                            Type::Decimal => Value::Decimal(decimal_stack[var.offset]),
                            Type::Bits(width) => {
                                let width = partial.map_or(width, |(_, s)| s);
                                Value::Bits(Bits::load_from_slice(&bit_stack[var.offset..], width))
                            }
                        };
                        regions.other[(region - 1) as usize]
                            .push(Event::Drive(*sig, value, None, partial));
                        continue;
                    }

                    let signal = signals.get_mut(sig).unwrap();
                    let updated = match signal {
                        SignalValue::Bits(Bits::Big(size, signal_value)) => {
                            if &bit_stack[var.offset..][..size.div_ceil(8) as usize]
                                == signal_value.as_ref()
                            {
                                false
                            } else {
                                match partial {
                                    None => {
                                        *signal_value = bit_stack[var.offset..]
                                            [..size.div_ceil(8) as usize]
                                            .iter()
                                            .copied()
                                            .collect();
                                    }
                                    Some(_) => todo!(),
                                }
                                true
                            }
                        }
                        SignalValue::Bits(Bits::Small(signal_value, size)) => {
                            let before = *signal_value;
                            match partial {
                                None => {
                                    *signal_value =
                                        bits::store_to_u64(bit_stack, var.offset, *size);
                                }
                                Some((offset, length)) => {
                                    let offset = decimal_stack[offset.offset];
                                    let value = bits::store_to_u64(bit_stack, var.offset, *length);
                                    *signal_value &= !(((1u64 << *length) - 1) << offset);
                                    *signal_value |= value << offset;
                                }
                            }
                            before != *signal_value
                        }
                        SignalValue::Decimal(signal_value) => {
                            assert!(partial.is_none());
                            let before = *signal_value;
                            *signal_value = decimal_stack[var.offset];
                            before != *signal_value
                        }
                        SignalValue::BitsArray(_) => unreachable!(),
                        SignalValue::DecimalArray(_) => unreachable!(),
                    };

                    if updated {
                        update_watchers(*sig, watches, listeners, regions);
                    }
                }
                I::ArrProbe(dst, signal, idx) => {
                    let signal = signals.get(signal).unwrap();
                    let idx = decimal_stack[idx.offset];
                    match signal {
                        SignalValue::BitsArray(arr) => match &arr[idx as usize] {
                            Bits::Small(value, size) => {
                                bits::load_from_u64(bit_stack, dst.offset, *size, *value);
                            }
                            Bits::Big(size, value) => bit_stack[dst.offset..]
                                [..size.div_ceil(8) as usize]
                                .copy_from_slice(&value),
                        },
                        SignalValue::DecimalArray(arr) => {
                            decimal_stack[dst.offset] = arr[idx as usize];
                        }
                        SignalValue::Bits(_) => unreachable!(),
                        SignalValue::Decimal(_) => unreachable!(),
                    };
                }
                I::ArrDrive(sig, src, idx, region, partial) => {
                    if *region != 0 {
                        let idx = decimal_stack[idx.offset] as u32;
                        let partial = partial.map(|(offset, width)| {
                            (decimal_stack[offset.offset] as VectorSize, width)
                        });
                        let value = match types[signal_ty[sig].key] {
                            Type::Decimal => Value::Decimal(decimal_stack[src.offset]),
                            Type::Bits(width) => {
                                let width = partial.map_or(width, |(_, s)| s);
                                Value::Bits(Bits::load_from_slice(&bit_stack[src.offset..], width))
                            }
                        };
                        regions.other[(region - 1) as usize].push(Event::Drive(
                            *sig,
                            value,
                            Some(idx),
                            partial,
                        ));
                        continue;
                    }

                    let signal = signals.get_mut(sig).unwrap();
                    let idx = decimal_stack[idx.offset];

                    let updated = match signal {
                        SignalValue::BitsArray(arr) => match &mut arr[idx as usize] {
                            Bits::Big(size, signal_value) => {
                                if &bit_stack[src.offset..][..size.div_ceil(8) as usize]
                                    == signal_value.as_ref()
                                {
                                    false
                                } else {
                                    match partial {
                                        None => {
                                            *signal_value = bit_stack[src.offset..]
                                                [..size.div_ceil(8) as usize]
                                                .iter()
                                                .copied()
                                                .collect();
                                        }
                                        Some(_) => todo!(),
                                    }
                                    true
                                }
                            }
                            Bits::Small(signal_value, size) => {
                                let before = *signal_value;
                                match partial {
                                    None => {
                                        *signal_value =
                                            bits::store_to_u64(bit_stack, src.offset, *size);
                                    }
                                    Some((offset, length)) => {
                                        let offset = decimal_stack[offset.offset];
                                        let value =
                                            bits::store_to_u64(bit_stack, src.offset, *length);
                                        *signal_value &= !(((1u64 << *length) - 1) << offset);
                                        *signal_value |= value << offset;
                                    }
                                }
                                before != *signal_value
                            }
                        },
                        SignalValue::DecimalArray(arr) => {
                            assert!(partial.is_none());
                            let before = arr[idx as usize];
                            arr[idx as usize] = decimal_stack[src.offset];
                            before != arr[idx as usize]
                        }
                        SignalValue::Bits(_) => unreachable!(),
                        SignalValue::Decimal(_) => unreachable!(),
                    };

                    if updated {
                        update_watchers(*sig, watches, listeners, regions);
                    }
                }
                I::Wait(time) => {
                    schedule.entry(ctx.time + time.0).or_default().push(self);
                    return EvalOutcome::Next;
                }
                I::WaitRegion(region) => {
                    if *region == 0 {
                        regions.active.push(self);
                    } else {
                        regions.other[*region as usize].push(self);
                    }
                    return EvalOutcome::Next;
                }
                I::Watch(signals) => {
                    let listener_key = listeners.insert(self);
                    for signal in signals {
                        watches.entry(*signal).or_default().push(listener_key);
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
                I::Halt => return EvalOutcome::Next,
            }
        }
    }
}

pub fn run(
    ctx: &mut Context,
    processes: &SlotMap<VmProcessKey, VmProcess>,
    regions: &mut Regions,
    signals: &mut HashMap<VmSignalKey, SignalValue>,
    signal_ty: &HashMap<VmSignalKey, TypeInfo>,
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
    types: &TypeTable,
    max_time: u64,
) -> Result<(), ()> {
    let mut schedule = BTreeMap::<Timestamp, Vec<Event>>::new();
    'region_loop: loop {
        while let Some(event) = regions.active.pop() {
            let outcome = event.evaluate(
                ctx,
                processes,
                &mut schedule,
                regions,
                signals,
                signal_ty,
                listeners,
                watches,
                types,
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
