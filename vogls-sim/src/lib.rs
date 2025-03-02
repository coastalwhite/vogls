use std::cmp::Ordering;
use std::collections::BinaryHeap;

use self::instruction::{Instruction, WatchCondition};

pub mod instruction;

pub type Timestamp = usize;
pub type Delay = usize;
pub type RegId = usize;
pub type ProcessId = usize;
pub type Value = u32;

#[derive(Debug)]
pub struct Process {
    pub instrs: Vec<Instruction>,
}

pub struct Context {
    time: Timestamp,
    pid: ProcessId,
    ip: usize,
}

#[derive(Clone, Copy)]
pub struct Event {
    /// Process ID
    pub pid: ProcessId,
    /// Instruction Pointer
    pub ip: usize,
}

impl Event {
    pub fn at_pid_start(pid: ProcessId) -> Self {
        Self { pid, ip: 0 }
    }
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

#[derive(Default)]
pub struct Listeners {
    none: Vec<Event>,
    posedge: Vec<Event>,
    negedge: Vec<Event>,
}

enum EvalOutcome {
    Continue,
    Stop,
    Exit,
}

impl Instruction {
    fn evaluate(
        &self,
        ctx: &Context,
        schedule: &mut BinaryHeap<ScheduledEvent>,
        _processes: &[Process],
        stack: &mut Vec<Value>,
        listeners: &mut [Listeners],
        registers: &mut [Value],
    ) -> EvalOutcome {
        match self {
            Instruction::Load(value) => {
                stack.push(*value);
            }
            Instruction::Update(reg) => {
                let before = registers[*reg];
                let after = stack.pop().unwrap();

                // @Incomplete: 4-state logic
                let posedge = before == 0 && after == 1;
                let negedge = before == 1 && after == 0;

                for event in listeners[*reg].none.drain(..) {
                    schedule.push(ScheduledEvent {
                        at: ctx.time,
                        event,
                    });
                }
                if posedge {
                    for event in listeners[*reg].posedge.drain(..) {
                        schedule.push(ScheduledEvent {
                            at: ctx.time,
                            event,
                        });
                    }
                }
                if negedge {
                    for event in listeners[*reg].negedge.drain(..) {
                        schedule.push(ScheduledEvent {
                            at: ctx.time,
                            event,
                        });
                    }
                }
                registers[*reg] = after;
            }
            Instruction::Schedule(pid, delay) => {
                schedule.push(ScheduledEvent {
                    at: ctx.time + delay,
                    event: Event { pid: *pid, ip: 0 },
                });
            }
            Instruction::Yield(delay) => {
                schedule.push(ScheduledEvent {
                    at: ctx.time + *delay,
                    event: Event {
                        pid: ctx.pid,
                        ip: ctx.ip + 1,
                    },
                });

                return EvalOutcome::Stop;
            }
            Instruction::Watch(conditions) => {
                let event = Event {
                    pid: ctx.pid,
                    ip: ctx.ip + 1,
                };

                // @Incomplete: This should only schedule the watch once over the whole list of
                // conditions.
                for (condition, reg) in conditions.iter() {
                    match condition {
                        WatchCondition::None => listeners[*reg].none.push(event),
                        WatchCondition::Posedge => listeners[*reg].posedge.push(event),
                        WatchCondition::Negedge => listeners[*reg].negedge.push(event),
                    }
                }

                return EvalOutcome::Stop;
            }
            Instruction::Display(msg) => {
                eprintln!("[DISPLAY]: time = {}: {msg}", ctx.time);
            }
            Instruction::Finish => {
                eprintln!("[FINISH]");
                return EvalOutcome::Exit;
            }
        }

        EvalOutcome::Continue
    }
}

pub fn run(
    event_queue: &mut BinaryHeap<ScheduledEvent>,
    processes: &[Process],
    listeners: &mut [Listeners],
    registers: &mut [Value],
    max_time: usize,
) {
    let mut ctx = Context {
        time: 0,
        pid: 0,
        ip: 0,
    };
    let mut stack = Vec::new();
    'event_loop: while let Some(se) = event_queue.pop() {
        ctx.time = se.at;
        ctx.pid = se.event.pid;
        ctx.ip = se.event.ip;

        if ctx.time > max_time {
            break;
        }

        eprintln!(
            "[T={}] Starting process {} at {}",
            ctx.time, ctx.pid, ctx.ip,
        );

        let process = &processes[ctx.pid];
        for (ip, ir) in process.instrs.iter().enumerate().skip(ctx.ip) {
            ctx.ip = ip;
            let outcome = ir.evaluate(
                &ctx,
                event_queue,
                processes,
                &mut stack,
                listeners,
                registers,
            );

            match outcome {
                EvalOutcome::Continue => {}
                EvalOutcome::Stop => break,
                EvalOutcome::Exit => break 'event_loop,
            }
        }
    }
}
