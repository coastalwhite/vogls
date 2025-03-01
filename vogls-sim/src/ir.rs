use core::fmt;

use crate::{Delay, ProcessId, RegId, Value};

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
    Schedule(ProcessId, Delay),
    Yield(Delay),
    Watch(Vec<(WatchCondition, RegId)>),
    Finish,
}

pub struct IRDisplay<'a> {
    ir: &'a IR,
    indent: usize,
}

impl IR {
    pub fn display(&self) -> IRDisplay {
        IRDisplay { ir: self, indent: 0 }
    }
}

impl IRDisplay<'_> {
    pub fn add_indent(mut self, indent: usize) -> Self {
        self.indent += indent;
        self
    }
}

impl<'a> fmt::Display for IRDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:indent$}", "", indent = self.indent * 2)?;

        match self.ir {
            IR::Load(value) => write!(f, "lv           {value}"),
            IR::Update(reg) => write!(f, "update       ${reg}"),
            IR::Display(s) => write!(f, "display      \"{s}\""),
            IR::Schedule(e, at) => write!(f, "schedule     e{e}, #{at}"),
            IR::Yield(delay) => write!(f, "yield        #{delay}"),
            IR::Watch(conditions) => {
                write!(f, "watch        ")?;

                if conditions.len() == 1 {
                    match conditions[0].0 {
                        WatchCondition::None => {}
                        WatchCondition::Posedge => write!(f, "posedge ")?,
                        WatchCondition::Negedge => write!(f, "negedge ")?,
                    }
                    write!(f, "${}", conditions[0].1)?;
                } else {
                    todo!()
                }
                Ok(())
            }
            IR::Finish => write!(f, "finish"),
        }
    }
}
