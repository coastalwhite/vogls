use vogls_codegen::Heap;
use vogls_utils::{TableKey, new_table_key};

pub mod plugins;
pub mod readmem;

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
    pub event_count: u64,
}

impl Clone for RuntimeState {
    fn clone(&self) -> Self {
        Self {
            heap: self.heap.clone(),
            time: self.time.clone(),
            last_active_time: self.last_active_time.clone(),
            event_count: self.event_count.clone(),
        }
    }
}

impl RuntimeState {
    pub fn new(heap: Heap, num_signals: usize) -> Self {
        Self {
            heap,
            time: 0,
            last_active_time: vec![0; num_signals],
            event_count: 0,
        }
    }
}

pub struct SimulationIo {
    pub stdout: Box<dyn std::io::Write + Send + Sync>,
    pub stderr: Box<dyn std::io::Write + Send + Sync>,
}

impl SimulationIo {
    pub fn new(
        stdout: Box<dyn std::io::Write + Send + Sync>,
        stderr: Box<dyn std::io::Write + Send + Sync>,
    ) -> SimulationIo {
        Self { stdout, stderr }
    }
}
