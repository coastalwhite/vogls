use std::{fmt, io};

use vogls_codegen::Heap;
use vogls_ir::time::TimeFormat;
use vogls_utils::{TableKey, new_table_key};
use vogls_world::WorldError;

pub mod plugins;
pub mod random;
pub mod readmem;

new_table_key! { pub struct RtSignalKey; }

/// An error that ends the simulation.
///
/// The backends store it in their cold context when it occurs, stop the simulation and report it
/// once the run returns.
#[derive(Debug)]
pub enum RuntimeError {
    ReadMem(readmem::ReadMemError),
    World(WorldError),
    /// An assertion failed. Its message was written when it failed.
    AssertionFailed,
    VcdSecondFile,
    VcdDumpVarsOverTime,
    /// An event was scheduled beyond the end of time.
    TimeOverflow,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadMem(err) => err.fmt(f),
            Self::World(err) => err.fmt(f),
            Self::AssertionFailed => f.write_str("assertion failed"),
            Self::VcdSecondFile => f.write_str("VCD opened a second file"),
            Self::VcdDumpVarsOverTime => f.write_str("dumping vars over several simulation times"),
            Self::TimeOverflow => f.write_str("time overflow"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<readmem::ReadMemError> for RuntimeError {
    fn from(err: readmem::ReadMemError) -> Self {
        Self::ReadMem(err)
    }
}
impl From<WorldError> for RuntimeError {
    fn from(err: WorldError) -> Self {
        Self::World(err)
    }
}
impl From<io::Error> for RuntimeError {
    fn from(err: io::Error) -> Self {
        Self::World(err.into())
    }
}

impl RtSignalKey {
    pub fn as_usize(self) -> usize {
        self.get()
    }

    pub fn as_u64(self) -> u64 {
        self.get() as u64
    }
}

#[derive(Clone)]
pub struct RuntimeState {
    pub heap: Heap,
    pub time: u64,
    pub last_active_time: Vec<u64>,
    pub event_count: u64,
    pub instruction_count: u64,
    pub time_format: TimeFormat,
    pub txs: Vec<RtSignalKey>,
}

impl RuntimeState {
    pub fn new(heap: Heap, lupdt_updated: &[bool], time_format: TimeFormat) -> Self {
        let last_active_time = lupdt_updated
            .iter()
            .map(|updated| if *updated { 0 } else { u64::MAX })
            .collect();
        Self {
            heap,
            time: 0,
            last_active_time,
            event_count: 0,
            instruction_count: 0,
            time_format,
            txs: Vec::new(),
        }
    }

    pub fn dump_stats(&self, f: &mut impl io::Write) -> io::Result<()> {
        writeln!(f, "Stats:",).unwrap();
        writeln!(f, "  # Instructions: {}", self.instruction_count).unwrap();
        writeln!(f, "  # Events:       {}", self.event_count).unwrap();
        Ok(())
    }
}
