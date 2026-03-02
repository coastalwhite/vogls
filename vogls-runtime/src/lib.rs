use vogls_codegen::Heap;

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
