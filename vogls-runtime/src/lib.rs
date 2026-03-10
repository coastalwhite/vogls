use vogls_codegen::Heap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RtSignalKey(pub u64);

impl RtSignalKey {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Clone)]
pub struct RuntimeState {
    pub heap: Heap,
    pub time: u64,
    pub last_active_time: Vec<u64>,
    pub event_count: u64,
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
