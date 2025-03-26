use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use slotmap::{new_key_type, SlotMap};
use vogls_ir::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, IntrinsicArg,
    IntrinsicVariant, SignalKey, UnaryOp, Value, VariableKey,
};

new_key_type! { pub struct ListenerKey; }

pub type Timestamp = u64;
pub type Delay = usize;

pub struct Context {
    time: Timestamp,
}

#[derive(Clone, Copy)]
pub struct Event {
    /// Process ID
    pub bb: BasicBlockKey,
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
    Continue,
    Stop,
    Exit,
}

fn bb_evaluate(
    bb: &BasicBlock,
    ctx: &mut Context,
    gl: &GlobalContext,
    schedule: &mut BinaryHeap<ScheduledEvent>,
    variables: &mut HashMap<VariableKey, Value>,
    signals: &mut HashMap<SignalKey, Value>,
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut HashMap<SignalKey, Vec<ListenerKey>>,
) -> EvalOutcome {
    use vogls_ir::Instruction as I;

    for i in &bb.instrs {
        match i {
            I::Constant(var, val) => _ = variables.insert(*var, val.clone()),
            I::Unary(dst, op, src) => {
                use UnaryOp as O;

                let Value::Bit(src) = variables.get(&src).unwrap();

                variables.insert(
                    *dst,
                    match op {
                        O::Neg => Value::Bit(!src),
                    },
                );
            }
            I::Binary(dst, op, lhs, rhs) => {
                use BinaryOp as O;

                let Value::Bit(lhs) = variables.get(&lhs).unwrap();
                let Value::Bit(rhs) = variables.get(&rhs).unwrap();

                variables.insert(
                    *dst,
                    match op {
                        O::And => Value::Bit(lhs & rhs),
                        O::Or => Value::Bit(lhs | rhs),
                        O::Xor => Value::Bit(lhs ^ rhs),
                    },
                );
            }
            I::Intrinsic(op, args) => {
                use IntrinsicVariant as O;

                match op {
                    O::Display => {
                        let Some(IntrinsicArg::StringLiteral(msg)) = args.first() else {
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
                variables.insert(*var, signals.get(&sig).unwrap().clone());
            }
            I::Drive(sig, var) => {
                signals.insert(*sig, variables.get(&var).unwrap().clone());

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
        }
    }

    use BasicBlockTerminator as T;
    let next_bb = match &bb.terminator {
        T::Wait(bb, time) => {
            schedule.push(ScheduledEvent {
                at: ctx.time + time.0,
                event: Event { bb: *bb },
            });
            return EvalOutcome::Stop;
        }
        T::Watch(bb, signals) => {
            let listener_key = listeners.insert(Event { bb: *bb });
            for signal in signals {
                watches.entry(*signal).or_default().push(listener_key);
            }
            return EvalOutcome::Stop;
        }
        T::Jump(bb) => *bb,
        T::Branch(var, true_bb, false_bb) => {
            let is_true = match variables.get(&var).unwrap() {
                Value::Bit(v) => *v,
            };

            if is_true {
                *true_bb
            } else {
                *false_bb
            }
        }
        T::Halt => return EvalOutcome::Stop,
    };

    let bb = gl.bbs.get(next_bb).unwrap();
    bb_evaluate(
        bb, ctx, gl, schedule, variables, signals, listeners, watches,
    )
}

pub fn run(
    gl: &GlobalContext,
    schedule: &mut BinaryHeap<ScheduledEvent>,
    variables: &mut HashMap<VariableKey, Value>,
    signals: &mut HashMap<SignalKey, Value>,
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut HashMap<SignalKey, Vec<ListenerKey>>,
    max_time: u64,
) {
    let mut ctx = Context { time: 0 };
    while let Some(se) = schedule.pop() {
        ctx.time = se.at;

        if ctx.time > max_time {
            break;
        }

        let bb = gl.bbs.get(se.event.bb).unwrap();
        let outcome = bb_evaluate(
            bb, &mut ctx, gl, schedule, variables, signals, listeners, watches,
        );

        match outcome {
            EvalOutcome::Continue => {}
            EvalOutcome::Stop => continue,
            EvalOutcome::Exit => break,
        }
    }
}
