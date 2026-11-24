use std::sync::Arc;

use hashbrown::hash_map::Entry;
use vogls::{ElaboratedDesign, FinalizeContext, SignalHandle};
use vogls_codegen::{Heap, HeapRef};
use vogls_ir::{Bits, LogicMode};
use vogls_runtime::RtSignalKey;

use vogls_utils::{NonMaxUsize, VgHashMap};

/// A copy of the runtime heap as of the end of the previous timestep.
///
/// Only the words marked dirty are copied over on [`ShadowHeap::update`], so keeping the shadow
/// in sync costs time proportional to what changed, not to the size of the heap.
#[derive(Clone)]
pub struct ShadowHeap {
    heap: Heap,
    /// One bit per word of `heap`: whether to copy it on the next [`ShadowHeap::update`].
    dirty: Vec<u64>,
}

impl ShadowHeap {
    pub fn new(heap: Heap) -> Self {
        let dirty = vec![0u64; heap.0.len().div_ceil(64)];
        Self { heap, dirty }
    }

    /// The number of bits of the signal at `at` that differ between the shadow and `heap`.
    ///
    /// For a four-valued signal, a bit differs if either its special or its value bit does.
    pub fn distance(&self, heap: &Heap, at: HeapRef, mode: LogicMode) -> u64 {
        let size = at.size.get() as usize;
        let ones = match mode {
            LogicMode::TwoValue if size <= 64 => {
                let diff = self.heap.get_tv_u64(at) ^ heap.get_tv_u64(at);
                (diff & low_mask(size)).count_ones()
            }
            LogicMode::FourValue if size <= 64 => {
                let (old_spc, old_val) = self.heap.get_fv_u64(at);
                let (new_spc, new_val) = heap.get_fv_u64(at);
                (((old_spc ^ new_spc) | (old_val ^ new_val)) & low_mask(size)).count_ones()
            }
            // Wide signals are word aligned: `n` value words, or `n` special words followed by
            // `n` value words.
            LogicMode::TwoValue => {
                let n = size.div_ceil(64);
                let old = self.heap.get_u64_slice(at.offset, n);
                let new = heap.get_u64_slice(at.offset, n);
                (0..n)
                    .map(|i| ((old[i] ^ new[i]) & word_mask(size, i)).count_ones())
                    .sum()
            }
            LogicMode::FourValue => {
                let n = size.div_ceil(64);
                let old = self.heap.get_u64_slice(at.offset, 2 * n);
                let new = heap.get_u64_slice(at.offset, 2 * n);
                (0..n)
                    .map(|i| {
                        let diff = (old[i] ^ new[i]) | (old[n + i] ^ new[n + i]);
                        (diff & word_mask(size, i)).count_ones()
                    })
                    .sum()
            }
        };
        u64::from(ones)
    }

    /// Mark the words that store the signal at `at` to be copied on the next
    /// [`ShadowHeap::update`].
    pub fn mark_dirty(&mut self, at: HeapRef, mode: LogicMode) {
        let size = at.size.get() as usize;
        let num_bits = match mode {
            LogicMode::TwoValue if size <= 64 => size,
            LogicMode::TwoValue => size.next_multiple_of(64),
            LogicMode::FourValue if at.size < Heap::FV_U64_MIN_SIZE => 2 * size,
            LogicMode::FourValue => 2 * size.next_multiple_of(64),
        };
        let start = at.offset.bit_offset / 64;
        let end = (at.offset.bit_offset + num_bits).div_ceil(64);
        for word in start..end {
            self.dirty[word / 64] |= 1 << (word % 64);
        }
    }

    /// Copy the dirty words over from `heap` and clear them.
    pub fn update(&mut self, heap: &Heap) {
        let (old, new) = (&mut self.heap.0, &heap.0);
        assert_eq!(old.len(), new.len());

        for (i, dirty) in self.dirty.iter_mut().enumerate() {
            let mut d = std::mem::take(dirty);
            while d != 0 {
                let word = i * 64 + d.trailing_zeros() as usize;
                old[word] = new[word];
                d &= d - 1;
            }
        }
    }
}

/// A mask of the lowest `n` bits of a word, for `n <= 64`.
fn low_mask(n: usize) -> u64 {
    u64::MAX.unbounded_shr(64 - n as u32)
}

/// The mask of the bits of word `i` that belong to a wide signal of `size` bits.
fn word_mask(size: usize, i: usize) -> u64 {
    low_mask((size - 64 * i).min(64))
}

/// Computes the Hamming distance over all the signals in the design, per timestep.
///
/// The distance of a timestep is the number of signal bits whose value differs between the end
/// of the previous timestep and the end of this one. The first timestep is the baseline and has
/// a distance of 0.
///
/// Signals are registered as probes, which only read the design, so tracing them does not get in
/// the way of optimizing their drivers.
#[derive(Default, Clone)]
pub struct HammingDistancePlugin {
    handles: Vec<SignalHandle>,
}

impl HammingDistancePlugin {
    pub fn new() -> Self {
        Self::default()
    }
}

impl vogls::VoglsPlugin for HammingDistancePlugin {
    fn clone(&self) -> Box<dyn vogls::VoglsPlugin> {
        Box::new(Clone::clone(self))
    }

    fn register_handles(&mut self, design: &mut ElaboratedDesign<'_>) {
        let symbols = design.table().symbol_id_iter().collect::<Vec<_>>();
        self.handles.extend(
            symbols
                .into_iter()
                .filter_map(|sid| design.get_probe_handle(sid)),
        );
    }

    fn finalize(
        self: Box<Self>,
        ctx: &FinalizeContext<'_>,
    ) -> Box<dyn vogls::runtime::plugins::RuntimePlugin> {
        let mut tracked = vec![false; ctx.signal_to_heap.len()];
        for handle in &self.handles {
            // A signal that was folded into a constant has no runtime signal and never changes.
            if let Some(signal) = ctx.handle_map.get(handle) {
                tracked[signal.key().as_usize()] = true;
            }
        }

        Box::new(RtHammingDistance {
            tracked: tracked.into(),
            signal_to_heap: ctx.signal_to_heap.clone(),
            signal_mode: ctx.signal_mode.clone(),
            shadow: None,
            times: Vec::new(),
            distances: Vec::new(),
        })
    }
}

/// The runtime half of [`HammingDistancePlugin`].
#[derive(Clone)]
pub struct RtHammingDistance {
    /// Per runtime signal, whether it counts towards the distance.
    tracked: Arc<[bool]>,
    signal_to_heap: Arc<[HeapRef]>,
    signal_mode: Arc<[LogicMode]>,
    /// The heap at the end of the previous timestep, `None` before the first timestep.
    shadow: Option<ShadowHeap>,

    pub times: Vec<u64>,
    pub distances: Vec<u64>,
}

impl RtHammingDistance {
    /// Record the distance of the timestep at `time`, given the (deduplicated) signals that
    /// changed during it.
    fn record(&mut self, time: u64, heap: &Heap, txs: impl IntoIterator<Item = RtSignalKey>) {
        if self.shadow.is_none() {
            self.shadow = Some(ShadowHeap::new(heap.clone()));
            self.times.push(time);
            self.distances.push(0);
            return;
        }
        let shadow = self.shadow.as_mut().unwrap();

        let mut distance = 0;
        for tx in txs {
            let signal = tx.as_usize();
            if !self.tracked[signal] {
                continue;
            }
            let (at, mode) = (self.signal_to_heap[signal], self.signal_mode[signal]);
            distance += shadow.distance(heap, at, mode);
            shadow.mark_dirty(at, mode);
        }

        // Only copy once all distances are taken. Signals share heap words, so copying the
        // words of one signal early would also hide the change of another signal in them.
        shadow.update(heap);

        self.times.push(time);
        self.distances.push(distance);
    }
}

impl vogls::runtime::plugins::RuntimePlugin for RtHammingDistance {
    fn clone(&self) -> vogls::runtime::plugins::RuntimePluginState {
        Box::new(Clone::clone(self))
    }

    fn timestep(&mut self, state: &mut vogls::runtime::RuntimeState) {
        // The runtime deduplicates `txs` before handing it to `timestep`.
        self.record(state.time, &state.heap, state.txs.iter().copied());
    }

    fn finish(&mut self, state: &mut vogls::runtime::RuntimeState) {
        // A `$finish` ends the simulation without a `timestep` for the timestep it is in, so its
        // transactions are still pending and not deduplicated yet.
        if !state.txs.is_empty() {
            let mut txs = state.txs.clone();
            txs.sort_unstable();
            txs.dedup();
            self.record(state.time, &state.heap, txs);
        }
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
