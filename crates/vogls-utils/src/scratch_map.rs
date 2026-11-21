//! Insertion-ordered hash containers optimized for being cleared and refilled many times.

use std::hash::{BuildHasher, Hash};

pub type DefaultHasher = foldhash::fast::RandomState;

/// A vacant slot from [`ScratchTable::find_or_vacant`].
pub struct Vacant(usize);

/// A HashTable with `O(1)` clear when `T` is `!Drop`.
pub struct ScratchTable<T> {
    /// Power-of-two length, or 0. Never shrinks. Invariant: `entries` non-empty implies
    /// `slots` non-empty, and every slot tagged with `epoch` has `idx < entries.len()`.
    slots: Box<[Slot]>,
    /// Dense; iteration is O(len).
    entries: Vec<T>,
    /// Never 0 while live; 0 = `Slot::default()` = empty.
    epoch: u16,
}

/// A HashMap with `O(1)` clear when `T` is `!Drop`.
pub struct ScratchMap<K, V, S = DefaultHasher> {
    table: ScratchTable<(K, V)>,
    hasher: S,
}

/// A HashSet with `O(1)` clear when `T` is `!Drop`.
pub struct ScratchSet<K, S = DefaultHasher> {
    table: ScratchTable<K>,
    hasher: S,
}

/// Packed slot: low 16 bits are the epoch, high 48 bits the entry index.
#[derive(Clone, Copy, Default)]
struct Slot(u64);

impl Slot {
    const IDX_BITS: u32 = 48;
    const MAX_IDX: u64 = (1 << Self::IDX_BITS) - 1;

    #[inline(always)]
    fn new(epoch: u16, idx: u64) -> Self {
        debug_assert!(idx <= Self::MAX_IDX);
        Self((idx << 16) | epoch as u64)
    }

    #[inline(always)]
    fn epoch(self) -> u16 {
        self.0 as u16
    }

    #[inline(always)]
    fn idx(self) -> u64 {
        self.0 >> 16
    }
}

impl<T> Default for ScratchTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ScratchTable<T> {
    pub fn new() -> Self {
        Self {
            slots: Box::default(),
            entries: Vec::new(),
            epoch: 1,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// O(1) if T: !Drop, else O(live). Never O(capacity) except on epoch wrap.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.slots.fill(Slot::default()); // once per 2^16 clears
            self.epoch = 1;
        }
    }

    /// Entries in insertion order.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.entries
    }

    /// Entries in insertion order. Mutating the parts of an entry that determine its hash or
    /// equality is a logic error.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.entries
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.entries.iter()
    }

    /// Probe for an entry. `Ok(entry index)` on a hit, `Err(slot index)` of the first empty slot
    /// on a miss. Requires a non-empty slot table.
    #[inline]
    fn probe(&self, hash: u64, mut eq: impl FnMut(&T) -> bool) -> Result<usize, usize> {
        debug_assert!(self.slots.len().is_power_of_two());
        let mask = self.slots.len() - 1;
        let mut i = hash as usize & mask;
        loop {
            // SAFETY: `slots.len()` is power of two and we just masked `i` to the size.
            let s = unsafe { self.slots.get_unchecked(i) };
            if s.epoch() != self.epoch {
                return Err(i); // stale == empty
            }
            // SAFETY: `s` is tagged with the current epoch, so by the struct invariant it points
            // at a live entry.
            let entry = unsafe { self.entries.get_unchecked(s.idx() as usize) };
            if eq(entry) {
                return Ok(s.idx() as usize);
            }
            i = (i + 1) & mask;
        }
    }

    /// Index of the entry matching `eq`, if any.
    #[inline]
    pub fn find(&self, hash: u64, eq: impl FnMut(&T) -> bool) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }
        self.probe(hash, eq).ok()
    }

    /// Index of the entry matching `eq`, or a vacant slot to pass to [`Self::fill`].
    /// Reserves room for one entry first; `hasher` must agree with `hash` and is used for
    /// regrowth. The vacant slot is invalidated by any other mutation of the table.
    pub fn find_or_vacant(
        &mut self,
        hash: u64,
        eq: impl FnMut(&T) -> bool,
        hasher: impl Fn(&T) -> u64,
    ) -> Result<usize, Vacant> {
        self.reserve(1, hasher);
        self.probe(hash, eq).map_err(Vacant)
    }

    /// Insert `value` into a slot returned by [`Self::find_or_vacant`]; returns its index.
    pub fn fill(&mut self, vacant: Vacant, value: T) -> usize {
        let i = self.entries.len();
        debug_assert!(self.slots[vacant.0].epoch() != self.epoch);
        // Push before tagging the slot: if `push` unwinds, no slot points past the end.
        self.entries.push(value);
        self.slots[vacant.0] = Slot::new(self.epoch, i as u64);
        i
    }

    /// Index of the entry matching `eq`, inserting `make()` if there is none. The bool is true
    /// iff an entry was inserted.
    pub fn find_or_insert_with(
        &mut self,
        hash: u64,
        eq: impl FnMut(&T) -> bool,
        hasher: impl Fn(&T) -> u64,
        make: impl FnOnce() -> T,
    ) -> (usize, bool) {
        match self.find_or_vacant(hash, eq, hasher) {
            Ok(i) => (i, false),
            Err(v) => (self.fill(v, make()), true),
        }
    }

    /// Make room for `additional` more entries without regrowing the slot table.
    pub fn reserve(&mut self, additional: usize, hasher: impl Fn(&T) -> u64) {
        // load ≤ 1/2: linear probing degrades fast above that; memory is the agreed trade.
        let total = self
            .entries
            .len()
            .checked_add(additional)
            .filter(|&t| t as u64 <= Slot::MAX_IDX)
            .expect("ScratchTable: too many entries");
        let needed = total
            .checked_mul(2)
            .expect("ScratchTable: too many entries");
        if needed <= self.slots.len() {
            return;
        }
        let n = needed
            .checked_next_power_of_two()
            .expect("ScratchTable: too many entries")
            .max(16);
        self.entries.reserve(additional);

        // Build the new table off to the side and swap it in, so a panic (e.g. in `hasher`)
        // leaves the old, still-valid table in place.
        const EPOCH: u16 = 1;
        let mut slots = vec![Slot::default(); n].into_boxed_slice();
        let mask = n - 1;
        for (idx, e) in self.entries.iter().enumerate() {
            let mut i = hasher(e) as usize & mask;
            while slots[i].epoch() == EPOCH {
                i = (i + 1) & mask;
            }
            slots[i] = Slot::new(EPOCH, idx as u64);
        }
        self.slots = slots;
        self.epoch = EPOCH;
    }
}

impl<K, V, S: Default> Default for ScratchMap<K, V, S> {
    fn default() -> Self {
        Self {
            table: ScratchTable::new(),
            hasher: S::default(),
        }
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> ScratchMap<K, V, S> {
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            table: ScratchTable::new(),
            hasher,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.table.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// See [`ScratchTable::clear`].
    pub fn clear(&mut self) {
        self.table.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        let hasher = &self.hasher;
        self.table.reserve(additional, |(k, _)| hasher.hash_one(k));
    }

    #[inline]
    fn index_of(&self, k: &K) -> Option<usize> {
        if self.table.is_empty() {
            return None;
        }
        self.table.find(self.hasher.hash_one(k), |(ek, _)| ek == k)
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.index_of(k).map(|i| &self.table.entries[i].1)
    }

    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.index_of(k).map(|i| &mut self.table.entries[i].1)
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.index_of(k).is_some()
    }

    pub fn get_or_insert_with(&mut self, k: K, f: impl FnOnce() -> V) -> &mut V {
        let hasher = &self.hasher;
        let hash = hasher.hash_one(&k);
        let i =
            match self
                .table
                .find_or_vacant(hash, |(ek, _)| *ek == k, |(ek, _)| hasher.hash_one(ek))
            {
                Ok(i) => i,
                Err(v) => self.table.fill(v, (k, f())),
            };
        &mut self.table.entries[i].1
    }

    /// Inserts `k -> v` if `k` is not present. Otherwise returns the rejected pair.
    pub fn try_insert(&mut self, k: K, v: V) -> Result<&mut V, (K, V)> {
        let hasher = &self.hasher;
        let hash = hasher.hash_one(&k);
        match self
            .table
            .find_or_vacant(hash, |(ek, _)| *ek == k, |(ek, _)| hasher.hash_one(ek))
        {
            Ok(_) => Err((k, v)),
            Err(vacant) => {
                let i = self.table.fill(vacant, (k, v));
                Ok(&mut self.table.entries[i].1)
            }
        }
    }

    /// Entries in insertion order.
    #[inline]
    pub fn as_slice(&self) -> &[(K, V)] {
        self.table.as_slice()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, (K, V)> {
        self.table.iter()
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> std::ops::Index<K> for ScratchMap<K, V, S> {
    type Output = V;

    #[track_caller]
    fn index(&self, k: K) -> &V {
        self.get(&k).expect("ScratchMap: key not present")
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> std::ops::IndexMut<K> for ScratchMap<K, V, S> {
    #[track_caller]
    fn index_mut(&mut self, k: K) -> &mut V {
        self.get_mut(&k).expect("ScratchMap: key not present")
    }
}

impl<K, S: Default> Default for ScratchSet<K, S> {
    fn default() -> Self {
        Self {
            table: ScratchTable::new(),
            hasher: S::default(),
        }
    }
}

impl<K: Hash + Eq, S: BuildHasher> ScratchSet<K, S> {
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            table: ScratchTable::new(),
            hasher,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.table.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// See [`ScratchTable::clear`].
    pub fn clear(&mut self) {
        self.table.clear();
    }

    pub fn reserve(&mut self, additional: usize) {
        let hasher = &self.hasher;
        self.table.reserve(additional, |k| hasher.hash_one(k));
    }

    pub fn contains(&self, k: &K) -> bool {
        if self.table.is_empty() {
            return false;
        }
        self.table
            .find(self.hasher.hash_one(k), |ek| ek == k)
            .is_some()
    }

    /// Returns true iff `k` was not present.
    pub fn insert(&mut self, k: K) -> bool {
        let hasher = &self.hasher;
        let hash = hasher.hash_one(&k);
        match self
            .table
            .find_or_vacant(hash, |ek| *ek == k, |ek| hasher.hash_one(ek))
        {
            Ok(_) => false,
            Err(v) => {
                self.table.fill(v, k);
                true
            }
        }
    }

    /// Elements in insertion order.
    #[inline]
    pub fn as_slice(&self) -> &[K] {
        self.table.as_slice()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, K> {
        self.table.iter()
    }
}
