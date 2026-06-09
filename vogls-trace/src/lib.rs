use std::sync::Arc;

use hashbrown::hash_map::Entry;
use vogls::{Design, ElaboratedDesign, SignalHandle, VSymbolTable};
use vogls_codegen::HeapRef;
use vogls_ir::{Bits, LogicMode};
use vogls_runtime::RtSignalKey;

use vogls_utils::{NonMaxUsize, VgHashMap};

/// Simulation plugin to trace signals when they can updated.
#[derive(Default, Clone)]
pub struct TracePlugin {
    pub tracked: VgHashMap<RtSignalKey, Option<NonMaxUsize>>,
    pub updated_this_time_step: Vec<RtSignalKey>,

    pub handles: Vec<SignalHandle>,
    pub logic_mode: LogicMode,
    pub signal_to_heap: Arc<[HeapRef]>,
    pub trace: Vec<(RtSignalKey, Bits)>,
    pub time_offsets: Vec<(u64, usize)>,
}

pub struct Trace {
    pub trace: Vec<(RtSignalKey, vogls::Bits)>,
    pub time_offsets: Vec<(u64, usize)>,
}

impl TracePlugin {
    pub fn new() -> Self {
        Self::default()
    }
}

impl vogls::VoglsPlugin for TracePlugin {
    fn clone(&self) -> Box<dyn vogls::VoglsPlugin> {
        Box::new(Clone::clone(self))
    }

    fn register_handles(&mut self, design: &mut ElaboratedDesign<'_>, table: &VSymbolTable) {
        self.handles.extend(
            table
                .symbol_id_iter()
                .filter_map(|sid| design.get_signal_handle(sid)),
        );
    }

    fn finalize(&mut self, design: &Design) {
        for handle in self.handles.drain(..) {
            let signal = design.resolve_handle(handle);
            self.tracked.insert(
                signal.key(),
                Some(NonMaxUsize::new(self.updated_this_time_step.len()).unwrap()),
            );
            self.updated_this_time_step.push(signal.key());
        }
    }
}

impl vogls::runtime::plugins::RuntimePlugin for TracePlugin {
    fn clone(&self) -> vogls::runtime::plugins::RuntimePluginState {
        Box::new(Clone::clone(self))
    }

    fn poke_signal(&mut self, signal: RtSignalKey) {
        if let Some(idx) = self.tracked.get_mut(&signal) {
            idx.get_or_insert_with(|| {
                let idx = NonMaxUsize::new(self.updated_this_time_step.len()).unwrap();
                self.updated_this_time_step.push(signal);
                idx
            });
        }
    }

    fn timestep(&mut self, state: &mut vogls::runtime::RuntimeState) {
        self.time_offsets.push((state.time, self.trace.len()));
        self.trace
            .extend(self.updated_this_time_step.iter().map(|&s| {
                (
                    s,
                    state
                        .heap
                        .load_bits(self.signal_to_heap[s.as_usize()], self.logic_mode),
                )
            }));
        self.tracked.iter_mut().for_each(|t| *t.1 = None);
        self.updated_this_time_step.clear();
    }

    fn finish(&mut self, state: &mut vogls::runtime::RuntimeState) {
        if !self.updated_this_time_step.is_empty() {
            self.time_offsets.push((state.time, self.trace.len()));
        }
    }
}

impl Trace {
    pub fn hamming_distance(&self) -> (Vec<u64>, Vec<u64>) {
        let mut values = vogls::utils::VgHashMap::<vogls::RtSignalKey, usize>::default();
        (0..self.time_offsets.len() - 1)
            .map(|i| {
                let mut hd = 0;
                let (time, start) = self.time_offsets[i];
                let end = self.time_offsets[i + 1].1;

                for (i, (signal, value)) in self.trace[start..end].iter().enumerate() {
                    let i = start + i;
                    match values.entry(*signal) {
                        Entry::Vacant(entry) => _ = entry.insert(i),
                        Entry::Occupied(mut entry) => {
                            hd += vogls::Bits::u64_reduce_op(
                                value,
                                &self.trace[*entry.get()].1,
                                |l, r| (l ^ r).count_ones(),
                                |l, r| l + r,
                            ) as u64;
                            entry.insert(i);
                        }
                    }
                }
                (time, hd)
            })
            .collect::<(Vec<u64>, Vec<u64>)>()
    }
}
