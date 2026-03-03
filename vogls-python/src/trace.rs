use std::num::NonZeroUsize;

use hashbrown::hash_map::Entry;
use vogls::sim::VmSignalKey;

/// Simulation plugin to trace signals when they can updated.
#[derive(Default)]
pub struct TracePlugin {
    pub tracked: vogls::utils::VgHashMap<VmSignalKey, Option<NonZeroUsize>>,
    pub updated_this_time_step: Vec<VmSignalKey>,

    pub trace: Vec<(VmSignalKey, vogls::Bits)>,
    pub time_offsets: Vec<(u64, usize)>,
}

impl vogls::sim::Plugin for TracePlugin {
    fn update_signal(
        &mut self,
        _simulation: &vogls::sim::Simulation,
        _state: &mut vogls::SimulationState,
        signal: VmSignalKey,
    ) {
        if let Some(idx) = self.tracked.get_mut(&signal) {
            idx.get_or_insert_with(|| {
                self.updated_this_time_step.push(signal);
                NonZeroUsize::new(self.updated_this_time_step.len()).unwrap()
            });
        }
    }

    fn timestep(
        &mut self,
        simulation: &vogls::sim::Simulation,
        state: &mut vogls::SimulationState,
    ) {
        self.time_offsets.push((state.runtime.time, self.trace.len()));
        self.trace
            .extend(self.updated_this_time_step.iter().map(|&s| {
                (
                    s,
                    state
                        .runtime
                        .heap
                        .load_bits(simulation.signals[s.0 as usize], simulation.logic_mode),
                )
            }));
        self.tracked.iter_mut().for_each(|t| *t.1 = None);
        self.updated_this_time_step.clear();
    }

    fn finish(&mut self, _simulation: &vogls::sim::Simulation, state: &mut vogls::SimulationState) {
        self.time_offsets.push((state.runtime.time, self.trace.len()));
    }
}

#[pyo3::pyclass(frozen)]
pub struct Trace {
    pub trace: Vec<(VmSignalKey, vogls::Bits)>,
    pub time_offsets: Vec<(u64, usize)>,
}

#[pyo3::pymethods]
impl Trace {
    pub fn hamming_distance(&self, py: pyo3::Python<'_>) -> pyo3::Py<pyo3::types::PyList> {
        let mut out = Vec::<(u64, u64)>::new();
        py.detach(|| {
            let mut values = vogls::utils::VgHashMap::<vogls::sim::VmSignalKey, usize>::default();
            for i in 0..self.time_offsets.len() - 1 {
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

                out.push((time, hd));
            }
        });
        pyo3::types::PyList::new(py, out).unwrap().into()
    }
}
