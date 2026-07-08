use std::ops::Range;

use slotmap::SlotMap;
use vogls_utils::VgHashMap;

use crate::{BasicBlock, BasicBlockKey, BasicBlockTerminator, SignalKey};

#[derive(Debug)]
pub struct WatchMap {
    num_watches: usize,
    watchers: Vec<(SignalKey, usize)>,
    map: VgHashMap<SignalKey, Range<usize>>,
    bb_lookup: VgHashMap<BasicBlockKey, usize>,
}

impl WatchMap {
    pub fn new(bbs: &SlotMap<BasicBlockKey, BasicBlock>) -> Self {
        let mut watchers = Vec::<(SignalKey, usize)>::new();
        let mut map = VgHashMap::default();
        let mut bb_lookup = VgHashMap::default();
        let mut next_watcher_index = 0usize;
        for (key, bb) in bbs.iter() {
            if let BasicBlockTerminator::Watch(_, signals) = &bb.terminator {
                bb_lookup.insert(key, next_watcher_index);
                watchers.extend(signals.iter().map(|s| (*s, next_watcher_index)));
                next_watcher_index += 1;
            }
        }

        watchers.sort_unstable_by_key(|(s, _)| *s);
        if let Some(&(fst, _)) = watchers.first() {
            let mut start = 0usize;
            let mut current = fst;

            for (i, (s, _)) in watchers.iter().enumerate().skip(1) {
                if current != *s {
                    map.insert(current, start..i);
                    start = i;
                    current = *s;
                }
            }
            map.insert(current, start..watchers.len());
        }

        Self {
            num_watches: next_watcher_index,
            watchers,
            map,
            bb_lookup,
        }
    }

    pub fn get_watch_index(&self, key: BasicBlockKey) -> usize {
        self.bb_lookup[&key]
    }
}

impl WatchMap {
    pub fn num_watches(&self) -> usize {
        self.num_watches
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
}
