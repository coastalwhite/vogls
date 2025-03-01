use core::fmt;

use crate::{Delay, EventId, RegId, Value};

#[derive(Debug)]
pub enum WatchCondition {
    None,
    Posedge,
    Negedge,
}

#[derive(Debug)]
pub enum IR {
    Load(Value),
    Update(RegId),
    Display(String),
    Schedule(EventId, Delay),
    Watch(EventId, Vec<(WatchCondition, RegId)>),
    Finish,
}

pub struct IRDisplay<'a> {
    ir: &'a IR,
}

impl IR {
    pub fn display(&self) -> IRDisplay {
        IRDisplay { ir: self }
    }
}

impl<'a> fmt::Display for IRDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ir {
            IR::Load(value) => write!(f, "lv           {value}"),
            IR::Update(reg) => write!(f, "update       ${reg}"),
            IR::Display(s) => write!(f, "display      \"{s}\""),
            IR::Schedule(e, at) => write!(f, "schedule     e{e}, #{at}"),
            IR::Watch(e, conditions) => {
                write!(f, "watch        e{e}, ")?;

                if conditions.len() == 1 {
                    match conditions[0].0 {
                        WatchCondition::None => {},
                        WatchCondition::Posedge => write!(f, "posedge ")?,
                        WatchCondition::Negedge => write!(f, "negedge ")?,
                    }
                    write!(f, "${}", conditions[0].1)?;
                } else {
                    todo!()
                }
                Ok(())
            },
            IR::Finish => write!(f, "finish"),
        }
    }
}
