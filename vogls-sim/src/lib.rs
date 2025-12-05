use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use slotmap::{SlotMap, new_key_type};
use vogls_ir::{BinaryOp, Bits, IntrinsicOp, UnaryOp, Value};

mod instruction;

pub use instruction::*;

new_key_type! { pub struct ListenerKey; }
new_key_type! { pub struct VmProcessKey; }

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
pub struct Event {
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
    Exit,
}

impl Event {
    fn evaluate(
        mut self,
        ctx: &mut Context,
        processes: &SlotMap<VmProcessKey, VmProcess>,
        schedule: &mut BinaryHeap<ScheduledEvent>,
        signals: &mut HashMap<VmSignalKey, Value>,
        listeners: &mut SlotMap<ListenerKey, Event>,
        watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
    ) -> EvalOutcome {
        let ip = &mut self.ip;
        let bit_stack = &mut self.bit_stack;
        let decimal_stack = &mut self.decimal_stack;
        let process = processes.get(self.process).unwrap();

        use VmInstruction as I;
        loop {
            let instr = &process.instructions[*ip];
            *ip += 1;
            match instr {
                I::ConstantBit(var, Bits::Small(val, size)) => {
                    for i in 0..size.div_ceil(8) {
                        let i = i as usize;
                        bit_stack[var.offset + i] = val.to_le_bytes()[i];
                    }
                }
                I::Unary(dst, op, src) => {
                    use UnaryOp as O;
                    match op {
                        O::DecimalNeg => decimal_stack[src.offset] = !decimal_stack[src.offset],
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
                    };
                }

                I::ConstantDecimal(var, val) => decimal_stack[var.offset] = *val,

                I::Cast(dst, dst_ty, src, src_ty) => {
                    use vogls_ir::Type as T;
                    match (dst_ty, src_ty) {
                        (T::Bits(x), T::Bits(y)) if x == y => {}
                        (T::Decimal, T::Decimal) => {}

                        (T::Bits(1), T::Decimal) => {
                            let src = decimal_stack[src.offset];
                            bit_stack[dst.offset] = (src != 0) as u8;
                        }
                        (T::Decimal, T::Bits(1)) => {
                            let src = bit_stack[src.offset];
                            decimal_stack[dst.offset] = src as i64;
                        }
                        _ => todo!("cast: {src_ty:?} -> {dst_ty:?}"),
                    }
                }

                I::Intrinsic(op, args) => {
                    use IntrinsicOp as O;

                    match op {
                        O::Display => {
                            let Some(VmIntrinsicArg::StringLiteral(msg)) = args.first() else {
                                panic!("Invalid display argument");
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
                            assert!(value, "failed assertion");
                        }
                        O::Finish => {
                            writeln!(&mut ctx.stdout, "[FINISH]").unwrap();
                            return EvalOutcome::Exit;
                        }
                    }
                }
                I::Probe(var, sig) => {
                    match signals.get(&sig).unwrap() {
                        Value::Bits(Bits::Small(val, size)) => {
                            for i in 0..size.div_ceil(8) as usize {
                                bit_stack[var.offset + i] = val.to_le_bytes()[i];
                            }
                        }
                        Value::Decimal(v) => decimal_stack[var.offset] = *v,
                    };
                }
                I::Drive(sig, var) => {
                    let signal = signals.get_mut(sig).unwrap();
                    match signal {
                        Value::Bits(Bits::Small(_, size)) => {
                            let mut value = [0u8; 8];
                            let nbytes = size.div_ceil(8) as usize;
                            value[..nbytes].copy_from_slice(&bit_stack[var.offset..][..nbytes]);
                            let value = u64::from_le_bytes(value);
                            *signal = Value::Bits(Bits::Small(value, *size))
                        }
                        Value::Decimal(_) => *signal = Value::Decimal(decimal_stack[var.offset]),
                    }

                    if let Some(watchers) = watches.remove(sig) {
                        for watcher in watchers {
                            if let Some(event) = listeners.remove(watcher) {
                                schedule.push(ScheduledEvent {
                                    at: ctx.time,
                                    event,
                                });
                            }
                        }
                    }
                }
                I::Wait(time) => {
                    schedule.push(ScheduledEvent {
                        at: ctx.time + time.0,
                        event: self,
                    });
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
    schedule: &mut BinaryHeap<ScheduledEvent>,
    signals: &mut HashMap<VmSignalKey, Value>,
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut HashMap<VmSignalKey, Vec<ListenerKey>>,
    max_time: u64,
) {
    while let Some(se) = schedule.pop() {
        ctx.time = se.at;

        if ctx.time > max_time {
            break;
        }

        let outcome = se
            .event
            .evaluate(ctx, processes, schedule, signals, listeners, watches);

        match outcome {
            EvalOutcome::Next => continue,
            EvalOutcome::Exit => break,
        }
    }
}
