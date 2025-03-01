use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub type Timestamp = usize;
pub type Delay = usize;
pub type RegId = usize;
pub type EventId = usize;
pub type Value = u32;

pub enum IR {
    Load(Value),
    Update(RegId),
    Display(String),
    Schedule(EventId, Delay),
    Finish,
}

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

pub struct Listeners {
    watch_list: Vec<EventId>,
}

impl IR {
    fn evaluate(
        &self,
        ctx: &Context,
        schedule: &mut BinaryHeap<ScheduledEvent>,
        _events: &[Event],
        stack: &mut Vec<Value>,
        listeners: &[Listeners],
        registers: &mut [Value],
    ) {
        match self {
            IR::Load(value) => {
                stack.push(*value);
            }
            IR::Update(reg) => {
                for event in &listeners[*reg].watch_list {
                    schedule.push(ScheduledEvent {
                        at: ctx.time,
                        id: *event,
                    });
                }
                registers[*reg] = stack.pop().unwrap();
            }
            IR::Schedule(event, delay) => {
                schedule.push(ScheduledEvent {
                    at: ctx.time + delay,
                    id: *event,
                });
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
    listeners: &[Listeners],
    registers: &mut [Value],
) {
    let mut ctx = Context { time: 0 };
    let mut stack = Vec::new();
    while let Some(scheduled_event) = schedule.pop() {
        ctx.time = scheduled_event.at;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut schedule = BinaryHeap::default();
        schedule.push(ScheduledEvent { at: 7, id: 0 });
        let events = vec![
            Event {
                ir: vec![
                    IR::Display("Event 0".into()),
                    IR::Load(5),
                    IR::Update(0),
                    IR::Schedule(2, 5),
                ],
            },
            Event {
                ir: vec![
                    IR::Display("Event 1".into()),
                    IR::Schedule(2, 3),
                ],
            },
            Event {
                ir: vec![
                    IR::Display("Event 2".into()),
                ],
            }
        ];
        let listeners = vec![
            Listeners {
                watch_list: vec![1],
            }
        ];
        let mut registers = vec![0];

        run(&mut schedule, &events, &listeners, &mut registers);
        assert!(false);
    }
}
