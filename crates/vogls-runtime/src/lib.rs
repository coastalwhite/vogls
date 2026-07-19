use std::io;

use vogls_codegen::Heap;
use vogls_ir::GlobalContext;
use vogls_utils::{TableKey, new_table_key};

pub mod plugins;
pub mod readmem;
pub mod random;

new_table_key! { pub struct RtSignalKey; }

impl RtSignalKey {
    pub fn as_usize(self) -> usize {
        self.get()
    }

    pub fn as_u64(self) -> u64 {
        self.get() as u64
    }
}

pub struct RuntimeState {
    pub heap: Heap,
    pub time: u64,
    pub last_active_time: Vec<u64>,
    pub tvl_first_write: Vec<u64>,
    pub event_count: u64,
    pub instruction_count: u64,
}

impl Clone for RuntimeState {
    fn clone(&self) -> Self {
        Self {
            heap: self.heap.clone(),
            time: self.time,
            last_active_time: self.last_active_time.clone(),
            tvl_first_write: self.tvl_first_write.clone(),
            event_count: self.event_count,
            instruction_count: self.instruction_count,
        }
    }
}

impl RuntimeState {
    pub fn new(gl: &GlobalContext, heap: Heap, lupdt_updated: &[bool]) -> Self {
        let tvl_first_write = vec![0u64; gl.signals.len().div_ceil(64)];
        Self {
            heap,
            time: 0,
            last_active_time: lupdt_updated
                .iter()
                .map(|updated| if *updated { 0 } else { u64::MAX })
                .collect(),
            tvl_first_write,
            event_count: 0,
            instruction_count: 0,
        }
    }

    pub fn dump_stats(&self, f: &mut impl io::Write) -> io::Result<()> {
        writeln!(f, "Stats:",).unwrap();
        writeln!(f, "  # Instructions: {}", self.instruction_count).unwrap();
        writeln!(f, "  # Events:       {}", self.event_count).unwrap();
        Ok(())
    }
}

pub struct SimulationIo {
    pub stdout: Box<dyn std::io::Write + Send + Sync>,
    pub stderr: Box<dyn std::io::Write + Send + Sync>,
}

impl Default for SimulationIo {
    fn default() -> Self {
        Self {
            stdout: Box::new(std::io::stdout()) as _,
            stderr: Box::new(std::io::stderr()) as _,
        }
    }
}

impl SimulationIo {
    pub fn new(
        stdout: Box<dyn std::io::Write + Send + Sync>,
        stderr: Box<dyn std::io::Write + Send + Sync>,
    ) -> SimulationIo {
        Self { stdout, stderr }
    }
}
