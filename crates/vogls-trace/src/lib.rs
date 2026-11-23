use std::sync::Arc;

use hashbrown::hash_map::Entry;
use vogls::{ElaboratedDesign, FinalizeContext, SignalHandle};
use vogls_codegen::{Heap, HeapRef};
use vogls_ir::{Bits, LogicMode};
use vogls_runtime::RtSignalKey;

use vogls_utils::{NonMaxUsize, VgHashMap};

pub struct ShadowHeap {
    heap: Heap,
    dirty: Vec<u64>,
}

fn clean_dirty_words(old: &mut [u64], new: &[u64], dirty: &mut [u64]) {
    assert_eq!(old.len(), new.len());
    assert_eq!(old.len().div_ceil(64), dirty.len());
    assert!(
        dirty
            .last()
            .is_none_or(|d| ((64 - d.trailing_zeros()) as usize) < (old.len() % 64))
    );

    for (i, dirty_word) in dirty.iter_mut().enumerate() {
        let mut idx = i * 64;
        let mut d = std::mem::take(dirty_word);
        while d != 0 {
            let lz = d.trailing_zeros();
            d >>= lz;
            idx += lz as usize;

            // SAFETY:
            // - `dirty.len().div_ceil(64) == old.len() == new.len()`
            // - All bits in the last word of `dirty` with index greater or equal to old.len() are
            //   zero.
            let old = unsafe { old.get_unchecked_mut(idx) };
            let new = unsafe { new.get_unchecked(idx) };
            *old = *new;
        }
    }
}

impl ShadowHeap {
    pub fn new(heap: Heap) -> Self {
        let dirty = vec![0u64; heap.0.len().div_ceil(64) as usize];
        Self { heap, dirty }
    }

    pub fn update(&mut self, new_heap: &Heap) {
        clean_dirty_words(
            self.heap.0.as_mut(),
            new_heap.0.as_ref(),
            self.dirty.as_mut_slice(),
        );
    }
}

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

/// Simulation plugin to trace all the signals for each changing timestep.
#[derive(Default, Clone)]
pub struct UpdateTrackerPlugin {
    pub tracked: VgHashMap<RtSignalKey, Option<NonMaxUsize>>,
    pub updated_this_time_step: Vec<RtSignalKey>,
    pub handles: Vec<SignalHandle>,
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

    fn register_handles(&mut self, design: &mut ElaboratedDesign<'_>) {
        self.handles.extend(
            design
                .table()
                .symbol_id_iter()
                .filter_map(|sid| design.get_signal_handle(sid)),
        );
    }

    fn finalize(
        mut self: Box<Self>,
        ctx: &FinalizeContext<'_>,
    ) -> Box<dyn vogls::runtime::plugins::RuntimePlugin> {
        for handle in self.handles.drain(..) {
            let signal = ctx.handle_map[&handle];
            self.tracked.insert(
                signal.key(),
                Some(NonMaxUsize::new(self.updated_this_time_step.len()).unwrap()),
            );
            self.updated_this_time_step.push(signal.key());
        }
        self.signal_to_heap = ctx.signal_to_heap.clone();
        self
    }
}

fn commit_transactions(
    state: &vogls::runtime::RuntimeState,
    tracked: &mut VgHashMap<RtSignalKey, Option<NonMaxUsize>>,
    updated_this_time_step: &mut Vec<RtSignalKey>,
) {
    for &tx in &state.txs {
        if let Some(idx) = tracked.get_mut(&tx) {
            idx.get_or_insert_with(|| {
                let idx = NonMaxUsize::new(updated_this_time_step.len()).unwrap();
                updated_this_time_step.push(tx);
                idx
            });
        }
    }
}

impl vogls::runtime::plugins::RuntimePlugin for TracePlugin {
    fn clone(&self) -> vogls::runtime::plugins::RuntimePluginState {
        Box::new(Clone::clone(self))
    }

    fn timestep(&mut self, state: &mut vogls::runtime::RuntimeState) {
        commit_transactions(state, &mut self.tracked, &mut self.updated_this_time_step);

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
        commit_transactions(state, &mut self.tracked, &mut self.updated_this_time_step);

        if !self.updated_this_time_step.is_empty() {
            self.time_offsets.push((state.time, self.trace.len()));
        }
    }
}

impl vogls::VoglsPlugin for UpdateTrackerPlugin {
    fn clone(&self) -> Box<dyn vogls::VoglsPlugin> {
        Box::new(Clone::clone(self))
    }

    fn register_handles(&mut self, design: &mut ElaboratedDesign<'_>) {
        self.handles.extend(
            design
                .table()
                .symbol_id_iter()
                .filter_map(|sid| design.get_signal_handle(sid)),
        );
    }

    fn finalize(
        mut self: Box<Self>,
        ctx: &FinalizeContext<'_>,
    ) -> Box<dyn vogls::runtime::plugins::RuntimePlugin> {
        for handle in self.handles.drain(..) {
            let signal = ctx.handle_map[&handle];
            self.tracked.insert(
                signal.key(),
                Some(NonMaxUsize::new(self.updated_this_time_step.len()).unwrap()),
            );
            self.updated_this_time_step.push(signal.key());
        }
        self
    }
}

impl vogls::runtime::plugins::RuntimePlugin for UpdateTrackerPlugin {
    fn clone(&self) -> vogls::runtime::plugins::RuntimePluginState {
        Box::new(Clone::clone(self))
    }

    fn timestep(&mut self, state: &mut vogls::runtime::RuntimeState) {
        commit_transactions(state, &mut self.tracked, &mut self.updated_this_time_step);

        self.tracked.iter_mut().for_each(|t| *t.1 = None);
        self.updated_this_time_step.clear();
    }

    fn finish(&mut self, state: &mut vogls::runtime::RuntimeState) {
        commit_transactions(state, &mut self.tracked, &mut self.updated_this_time_step);
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
