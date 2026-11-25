use std::sync::Arc;

use hashbrown::hash_map::Entry;
use vogls::{ElaboratedDesign, FinalizeContext, SignalHandle};
use vogls_codegen::{Heap, HeapRef, SixBitSize};
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
    fn record(&mut self, time: u64, heap: &Heap, txs: &[RtSignalKey]) {
        if self.shadow.is_none() {
            self.shadow = Some(ShadowHeap::new(heap.clone()));
            self.times.push(time);
            self.distances.push(0);
            return;
        }
        let shadow = self.shadow.as_mut().unwrap();

        let mut distance = 0u64;

        assert_eq!(shadow.heap.0.len(), heap.0.len());
        let heap_sha = shadow.heap.0.as_ref();
        let heap_cur = heap.0.as_ref();
        for tx in txs {
            let signal = tx.as_usize();
            if !self.tracked[signal] {
                continue;
            }
            let (at, mode) = (self.signal_to_heap[signal], self.signal_mode[signal]);

            let bit_offset = at.offset.bit_offset as u64;

            let woff = (bit_offset / 64) as usize;

            let mut bit_size = at.size.get();
            if mode.is_four_value() {
                bit_size *= 2;
            }

            let min_u64_size = match mode {
                LogicMode::TwoValue => Heap::TV_U64_MIN_SIZE,
                LogicMode::FourValue => Heap::FV_U64_MIN_SIZE,
            };

            if at.size < min_u64_size {
                let boff = bit_offset % 64;
                let whole_size = SixBitSize::new(bit_size as u8).unwrap();
                let whole_mask = whole_size.mask(u64::MAX);
                let wsha = whole_mask & (heap_sha[woff] >> boff);
                let wcur = whole_mask & (heap_cur[woff] >> boff);

                let delta_mask = match mode {
                    LogicMode::TwoValue => wcur ^ wsha,
                    LogicMode::FourValue => {
                        let plane_size = SixBitSize::from_vector_size(at.size).unwrap();
                        let plane_mask = plane_size.mask(u64::MAX);

                        let spc = (wcur & plane_mask) ^ (wsha & plane_mask);
                        let val = (wcur >> plane_size as u32) ^ (wsha >> plane_size as u32);
                        spc | val
                    }
                };
                distance += delta_mask.count_ones() as u64;
            } else if mode.is_two_value() {
                let nwords = at.size.get().div_ceil(64) as usize;
                distance += (woff..woff + nwords)
                    .map(|i| (heap_sha[i] ^ heap_cur[i]).count_ones() as u64)
                    .sum::<u64>();
            } else {
                let nwords = at.size.get().div_ceil(64) as usize;
                distance += (woff..woff + nwords)
                    .map(|i| {
                        let spc = heap_sha[i] ^ heap_cur[i];
                        let val = heap_sha[nwords + i] ^ heap_cur[nwords + i];
                        (spc | val).count_ones() as u64
                    })
                    .sum::<u64>();
            }

            let start = at.offset.bit_offset / 64;
            let end = (at.offset.bit_offset + bit_size as usize).div_ceil(64);
            for word in start..end {
                shadow.dirty[word / 64] |= 1 << (word % 64);
            }
        }

        shadow.update(heap);

        self.times.push(time);
        self.distances.push(distance);
    }
}

impl vogls::runtime::plugins::RuntimePlugin for RtHammingDistance {
    fn clone(&self) -> vogls::runtime::plugins::RuntimePluginState {
        Box::new(Clone::clone(self))
    }

    fn timestep(&mut self, state: &mut vogls::runtime::RuntimeState, txs: &[RtSignalKey]) {
        self.record(state.time, &state.heap, txs);
    }

    fn finish(&mut self, state: &mut vogls::runtime::RuntimeState, txs: &[RtSignalKey]) {
        if txs.is_empty() {
            return;
        }

        self.record(state.time, &state.heap, txs);
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
    txs: &[RtSignalKey],
    tracked: &mut VgHashMap<RtSignalKey, Option<NonMaxUsize>>,
    updated_this_time_step: &mut Vec<RtSignalKey>,
) {
    for &tx in txs {
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

    fn timestep(&mut self, state: &mut vogls::runtime::RuntimeState, txs: &[RtSignalKey]) {
        commit_transactions(txs, &mut self.tracked, &mut self.updated_this_time_step);

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

    fn finish(&mut self, state: &mut vogls::runtime::RuntimeState, txs: &[RtSignalKey]) {
        commit_transactions(txs, &mut self.tracked, &mut self.updated_this_time_step);

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

    fn timestep(&mut self, _state: &mut vogls::runtime::RuntimeState, txs: &[RtSignalKey]) {
        commit_transactions(txs, &mut self.tracked, &mut self.updated_this_time_step);

        self.tracked.iter_mut().for_each(|t| *t.1 = None);
        self.updated_this_time_step.clear();
    }

    fn finish(&mut self, _state: &mut vogls::runtime::RuntimeState, txs: &[RtSignalKey]) {
        commit_transactions(txs, &mut self.tracked, &mut self.updated_this_time_step);
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
