use std::ops::Range;

use slotmap::SlotMap;
use vogls_utils::VgHashMap;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, SignalKey, TemporalRegionKey, WatchCondition,
};

#[derive(Debug)]
pub struct WatchMap {
    watchers: Vec<(WatchCondition, usize)>,
    map: VgHashMap<SignalKey, Range<usize>>,
    bb_lookup: VgHashMap<BasicBlockKey, usize>,

    /// The watch index of every distinct watch.
    watch_lookup: VgHashMap<(TemporalRegionKey, Box<[WatchCondition]>), usize>,
    /// The temporal region each watch resumes into, by watch index.
    targets: Vec<TemporalRegionKey>,
}

impl WatchMap {
    pub fn new(bbs: &SlotMap<BasicBlockKey, BasicBlock>) -> Self {
        let mut watchers = Vec::<(WatchCondition, usize)>::new();
        let mut map = VgHashMap::default();
        let mut bb_lookup = VgHashMap::default();
        let mut watch_lookup = VgHashMap::default();
        let mut targets = Vec::new();
        for (key, bb) in bbs.iter() {
            if let BasicBlockTerminator::Watch(tr, conditions) = &bb.terminator {
                let key_conditions: Box<[WatchCondition]> = conditions.as_slice().into();
                let index = *watch_lookup
                    .entry((*tr, key_conditions))
                    .or_insert_with(|| {
                        let index = targets.len();
                        watchers.extend(conditions.iter().map(|c| (*c, index)));
                        targets.push(*tr);
                        index
                    });
                bb_lookup.insert(key, index);
            }
        }

        watchers.sort_unstable_by_key(|(c, _)| c.to_ord());
        if let Some(&(fst, _)) = watchers.first() {
            let mut start = 0usize;
            let mut current = fst;

            // @NOTE: `to_ord` leads with the signal, so every watch on one signal is one run.
            for (i, (s, _)) in watchers.iter().enumerate().skip(1) {
                if current.signal != s.signal {
                    map.insert(current.signal, start..i);
                    start = i;
                    current = *s;
                }
            }
            map.insert(current.signal, start..watchers.len());
        }

        Self {
            watchers,
            map,
            bb_lookup,
            watch_lookup,
            targets,
        }
    }

    pub fn get_watch_index(&self, key: BasicBlockKey) -> usize {
        self.bb_lookup[&key]
    }

    pub fn find_watch_index(
        &self,
        tr: TemporalRegionKey,
        conditions: &[WatchCondition],
    ) -> Option<usize> {
        // @Performance: Remove this allocation.
        self.watch_lookup.get(&(tr, conditions.into())).copied()
    }

    /// The temporal region watch `index` resumes into when it triggers.
    ///
    /// A backend needs this to turn a watch into something callable; the index alone only says
    /// *which* watch fired, not where it goes.
    pub fn watch_target(&self, index: usize) -> TemporalRegionKey {
        self.targets[index]
    }

    pub fn num_watches(&self) -> usize {
        self.targets.len()
    }

    pub fn watch_indices(&self, signal: SignalKey) -> impl Iterator<Item = usize> {
        match self.map.get(&signal) {
            None => &[],
            Some(range) => &self.watchers[range.clone()],
        }
        .iter()
        .map(|(_, i)| *i)
    }

    pub fn num_watch_indices(&self, signal: SignalKey) -> usize {
        match self.map.get(&signal) {
            None => 0,
            Some(range) => range.clone().len(),
        }
    }

    pub fn map(&self) -> &VgHashMap<SignalKey, Range<usize>> {
        &self.map
    }

    pub fn watchers(&self) -> &[(WatchCondition, usize)] {
        &self.watchers
    }

    pub fn signal_watchers(&self, signal: SignalKey) -> &[(WatchCondition, usize)] {
        match self.map.get(&signal) {
            None => &[],
            Some(range) => &self.watchers[range.clone()],
        }
    }
}
