use std::cmp::Ordering;
use std::collections::BinaryHeap;

use self::ir::{WatchCondition, IR};

pub mod ir;

pub type Timestamp = usize;
pub type Delay = usize;
pub type RegId = usize;
pub type EventId = usize;
pub type Value = u32;


#[derive(Debug)]
pub struct Event {
    pub ir: Vec<IR>,
}

pub struct Context {
    time: Timestamp,
}

pub struct ScheduledEvent {
    pub at: Timestamp,
    pub id: EventId,
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
    none: Vec<EventId>,
    posedge: Vec<EventId>,
    negedge: Vec<EventId>,
}

impl IR {
    fn evaluate(
        &self,
        ctx: &Context,
        schedule: &mut BinaryHeap<ScheduledEvent>,
        _events: &[Event],
        stack: &mut Vec<Value>,
        listeners: &mut [Listeners],
        registers: &mut [Value],
    ) {
        match self {
            IR::Load(value) => {
                stack.push(*value);
            }
            IR::Update(reg) => {
                let before = registers[*reg];
                let after = stack.pop().unwrap();

                // @Incomplete: 4-state logic
                let posedge = before == 0 && after == 1;
                let negedge = before == 1 && after == 0;

                for event in listeners[*reg].none.drain(..) {
                    schedule.push(ScheduledEvent {
                        at: ctx.time,
                        id: event,
                    });
                }
                if posedge {
                    for event in listeners[*reg].posedge.drain(..) {
                        schedule.push(ScheduledEvent {
                            at: ctx.time,
                            id: event,
                        });
                    }
                }
                if negedge {
                    for event in listeners[*reg].negedge.drain(..) {
                        schedule.push(ScheduledEvent {
                            at: ctx.time,
                            id: event,
                        });
                    }
                }
                registers[*reg] = after;
            }
            IR::Schedule(event, delay) => {
                schedule.push(ScheduledEvent {
                    at: ctx.time + delay,
                    id: *event,
                });
            }
            IR::Watch(event, conditions) => {
                // @Incomplete: This should only schedule the watch once over the whole list of
                // conditions.
                for (condition, reg) in conditions.iter() {
                    match condition {
                        WatchCondition::None => listeners[*reg].none.push(*event),
                        WatchCondition::Posedge => listeners[*reg].posedge.push(*event),
                        WatchCondition::Negedge => listeners[*reg].negedge.push(*event),
                    }
                }
            }
            IR::Display(msg) => {
                eprintln!("[DISPLAY]: time = {}: {msg}", ctx.time);
            }
            IR::Finish => {
                eprintln!("[FINISH]");
                std::process::exit(0);
            }
        }
    }
}

pub fn run(
    schedule: &mut BinaryHeap<ScheduledEvent>,
    events: &[Event],
    listeners: &mut [Listeners],
    registers: &mut [Value],
    max_time: usize,
) {
    let mut ctx = Context { time: 0 };
    let mut stack = Vec::new();
    while let Some(scheduled_event) = schedule.pop() {
        ctx.time = scheduled_event.at;
        if ctx.time > max_time {
            break;
        }

        eprintln!("Starting event {} at {}", scheduled_event.id, ctx.time);

        let event = &events[scheduled_event.id];

        for ir in &event.ir {
            ir.evaluate(
                &ctx,
                schedule,
                events,
                &mut stack,
                listeners,
                registers,
            );
        }
    }
}
