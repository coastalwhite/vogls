use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use slotmap::{new_key_type, SlotMap};
use vogls_ir::{BinaryOp, IntrinsicOp, SignalKey, UnaryOp, Value};

mod instruction;

pub use instruction::*;

new_key_type! { pub struct ListenerKey; }
new_key_type! { pub struct VmProcessKey; }

pub type Timestamp = u64;
pub type InstanceId = u64;

pub struct Context {
    time: Timestamp,
}

impl Context {
    pub fn new() -> Self {
        Self { time: 0 }
    }
}

#[derive(Clone)]
pub struct Event {
    /// Which process is scheduled.
    pub process: VmProcessKey,
    /// The stack with which to execute.
    pub stack: Vec<u8>,
    /// Where to start execution.
    pub ip: usize,
}

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
        signals: &mut HashMap<SignalKey, Value>,
        listeners: &mut SlotMap<ListenerKey, Event>,
        watches: &mut HashMap<SignalKey, Vec<ListenerKey>>,
    ) -> EvalOutcome {
        let ip = &mut self.ip;
        let stack = &mut self.stack;
        let process = processes.get(self.process).unwrap();

        use VmInstruction as I;
        loop {
            let instr = &process.instructions[*ip];
            *ip += 1;
            match instr {
                I::Constant(var, val) => {
                    assert_eq!(var.size, 1);
                    stack[var.offset] = match val {
                        Value::Bit(v) => *v as u8,
                    };
                }
                I::Unary(dst, op, src) => {
                    assert_eq!(dst.size, 1);
                    assert_eq!(src.size, 1);

                    use UnaryOp as O;
                    stack[dst.offset] = match op {
                        O::Neg => !stack[src.offset],
                    };
                }
                I::Binary(dst, op, lhs, rhs) => {
                    assert_eq!(dst.size, 1);
                    assert_eq!(lhs.size, 1);
                    assert_eq!(rhs.size, 1);

                    let lhs = stack[lhs.offset];
                    let rhs = stack[rhs.offset];

                    use BinaryOp as O;
                    stack[dst.offset] = match op {
                        O::And => lhs & rhs,
                        O::Or => lhs | rhs,
                        O::Xor => lhs ^ rhs,
                    };
                }
                I::Intrinsic(op, args) => {
                    use IntrinsicOp as O;

                    match op {
                        O::Display => {
                            let Some(VmIntrinsicArg::StringLiteral(msg)) = args.first() else {
                                panic!("Invalid display argument");
                            };
                            eprintln!("[DISPLAY]: time = {}: {msg}", ctx.time);
                        }
                        O::Finish => {
                            eprintln!("[FINISH]");
                            return EvalOutcome::Exit;
                        }
                    }
                }
                I::Probe(var, sig) => {
                    assert_eq!(var.size, 1);
                    stack[var.offset] = match signals.get(&sig).unwrap() {
                        Value::Bit(v) => *v as u8,
                    };
                }
                I::Drive(sig, var) => {
                    assert_eq!(var.size, 1);
                    let var_value = stack[var.offset];
                    signals.insert(*sig, Value::Bit(var_value & 1 != 0));

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
                    let is_true = stack[cond.offset] & 1 != 0;
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
    signals: &mut HashMap<SignalKey, Value>,
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut HashMap<SignalKey, Vec<ListenerKey>>,
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
