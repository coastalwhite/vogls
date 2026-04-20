use std::sync::Arc;

use hashbrown::hash_map::Entry;
use vogls::VSymbol;
use vogls::codegen::HeapRef;
use vogls::utils::{NonMaxUsize, VgHashMap};
use vogls::{LogicMode, RtSignalKey, design::Design};

/// Simulation plugin to trace signals when they can updated.
#[derive(Default, Clone)]
pub struct TracePlugin {
    pub tracked: vogls::utils::VgHashMap<RtSignalKey, Option<NonMaxUsize>>,
    pub updated_this_time_step: Vec<RtSignalKey>,

    pub logic_mode: LogicMode,
    pub signal_to_heap: Arc<[HeapRef]>,
    pub trace: Vec<(RtSignalKey, vogls::Bits)>,
    pub time_offsets: Vec<(u64, usize)>,
}

impl TracePlugin {
    pub fn new(design: &Design) -> Self {
        let mut tracked = VgHashMap::default();
        let mut updated_this_time_step = Vec::new();

        for signal in design.elab_table.symbol_iter() {
            if let VSymbol::Net(n) = &signal.content {
                let (signal, _slice) = n.net.probe_signal();
                let rt_signal = design.get_rt_signal(signal);
                tracked.insert(
                    rt_signal,
                    Some(NonMaxUsize::new(updated_this_time_step.len()).unwrap()),
                );
                updated_this_time_step.push(rt_signal);
            }
        }

        Self {
            tracked,
            updated_this_time_step,
            logic_mode: design.gl.logic_mode,
            signal_to_heap: design.signal_to_heap.clone(),
            trace: Vec::new(),
            time_offsets: Vec::new(),
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

#[pyo3::pyclass(frozen)]
pub struct Trace {
    pub trace: Vec<(RtSignalKey, vogls::Bits)>,
    pub time_offsets: Vec<(u64, usize)>,
}

#[pyo3::pymethods]
impl Trace {
    pub fn hamming_distance(&self, py: pyo3::Python<'_>) -> pyo3::Py<pyo3::types::PyList> {
        let out = py.detach(|| {
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
                .collect::<Vec<(u64, u64)>>()
        });
        pyo3::types::PyList::new(py, out).unwrap().into()
    }
}
