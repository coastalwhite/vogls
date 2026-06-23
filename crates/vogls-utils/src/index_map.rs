use std::hash::BuildHasher;
use std::ops::{Index, IndexMut};
use std::{fmt, hash};

use crate::NonMaxUsize;

#[derive(Clone)]
pub struct IndexSet<K> {
    keys: Vec<K>,
    table: hashbrown::HashTable<NonMaxUsize>,
    random_state: foldhash::fast::RandomState,
}

#[derive(Clone)]
pub struct IndexMap<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
    table: hashbrown::HashTable<NonMaxUsize>,
    random_state: foldhash::fast::RandomState,
}

impl<V: fmt::Debug> fmt::Debug for IndexSet<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("IndexSet").field(&self.keys).finish()
    }
}
impl<K> Default for IndexSet<K> {
    fn default() -> Self {
        Self {
            keys: Default::default(),
            table: Default::default(),
            random_state: Default::default(),
        }
    }
}
impl<K: PartialEq> PartialEq for IndexSet<K> {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys
    }
}
impl<K: Eq> Eq for IndexSet<K> {}
impl<K: hash::Hash> hash::Hash for IndexSet<K> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.keys.hash(state);
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for IndexMap<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IndexMap<{}, {}> ",
            std::any::type_name::<K>(),
            std::any::type_name::<V>()
        )?;
        let mut map = f.debug_map();
        for (k, v) in self.keys.iter().zip(&self.values) {
            map.entry(k, v);
        }
        map.finish()
    }
}
impl<K, V> Default for IndexMap<K, V> {
    fn default() -> Self {
        Self {
            keys: Default::default(),
            values: Default::default(),
            table: Default::default(),
            random_state: Default::default(),
        }
    }
}
impl<K: PartialEq, V: PartialEq> PartialEq for IndexMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.keys == other.keys && self.values == other.values
    }
}
impl<K: Eq, V: Eq> Eq for IndexMap<K, V> {}
impl<K: hash::Hash, V: hash::Hash> hash::Hash for IndexMap<K, V> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.keys.hash(state);
        self.values.hash(state);
    }
}

impl<K> IndexSet<K> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.table.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &K> {
        self.keys.iter()
    }
    pub fn into_iter(self) -> impl Iterator<Item = K> {
        self.keys.into_iter()
    }

    pub fn get_at_index(&self, index: usize) -> Option<&K> {
        self.keys.get(index)
    }

    pub fn take_keys(self) -> Vec<K> {
        self.keys
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
impl<K, V> IndexMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
        self.table.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.values.iter())
    }
    pub fn into_iter(self) -> impl Iterator<Item = (K, V)> {
        self.keys.into_iter().zip(self.values.into_iter())
    }

    pub fn at(&self, index: usize) -> (&K, &V) {
        (&self.keys[index], &self.values[index])
    }
    pub fn at_mut(&mut self, index: usize) -> (&K, &mut V) {
        (&self.keys[index], &mut self.values[index])
    }

    pub fn take(self) -> (Vec<K>, Vec<V>) {
        (self.keys, self.values)
    }
    pub fn take_keys(self) -> Vec<K> {
        self.keys
    }
    pub fn take_values(self) -> Vec<V> {
        self.values
    }

    pub fn iter_values(&self) -> impl DoubleEndedIterator + ExactSizeIterator<Item = &V> {
        self.values.iter()
    }
}

pub enum Entry<'a, K, V> {
    Occuppied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

pub struct OccupiedEntry<'a, K, V> {
    entry: hashbrown::hash_table::OccupiedEntry<'a, NonMaxUsize>,
    _keys: &'a mut Vec<K>,
    values: &'a mut Vec<V>,
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {}

pub struct VacantEntry<'a, K, V> {
    entry: hashbrown::hash_table::VacantEntry<'a, NonMaxUsize>,
    key: K,
    keys: &'a mut Vec<K>,
    values: &'a mut Vec<V>,
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    pub fn insert(self, value: V) -> OccupiedEntry<'a, K, V> {
        let idx = NonMaxUsize::new(self.keys.len()).unwrap();
        self.keys.push(self.key);
        self.values.push(value);
        OccupiedEntry {
            entry: self.entry.insert(idx),
            _keys: self.keys,
            values: self.values,
        }
    }
}

impl<'a, K, V: Default> Entry<'a, K, V> {
    pub fn or_default(self) -> &'a mut V {
        let entry = match self {
            Self::Vacant(e) => e.insert(V::default()),
            Self::Occuppied(e) => e,
        };
        entry.values.get_mut(entry.entry.get().get()).unwrap()
    }
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    pub fn get(&mut self) -> &mut V {
        &mut self.values[self.entry.get().get()]
    }

    pub fn index(&self) -> usize {
        self.entry.get().get()
    }
}

impl<K: Eq + hash::Hash> IndexSet<K> {
    pub fn reserve(&mut self, additional: usize) {
        self.keys.reserve(additional);
        self.table.reserve(additional, |i| {
            let i = unsafe { self.keys.get_unchecked(i.get()) };
            self.random_state.hash_one(i)
        });
    }

    pub fn insert(&mut self, key: K) -> bool {
        self.insert_new(key).is_ok()
    }

    pub fn insert_index(&mut self, key: K) -> usize {
        match self.insert_new(key) {
            Ok(v) => v,
            Err(v) => v,
        }
    }

    pub fn insert_new(&mut self, key: K) -> Result<usize, usize> {
        let hash = self.random_state.hash_one(&key);
        match self.table.entry(
            hash,
            |i| K::eq(&key, unsafe { self.keys.get_unchecked(i.get()) }),
            |i| {
                let i = unsafe { self.keys.get_unchecked(i.get()) };
                self.random_state.hash_one(i)
            },
        ) {
            hashbrown::hash_table::Entry::Occupied(i) => Err(i.get().get()),
            hashbrown::hash_table::Entry::Vacant(i) => {
                let idx = self.keys.len();
                i.insert(NonMaxUsize::new(idx).unwrap());
                self.keys.push(key);
                Ok(idx)
            }
        }
    }

    #[inline(always)]
    fn get_index_ref(&self, key: &K) -> Option<&NonMaxUsize> {
        let hash = self.random_state.hash_one(key);
        self.table.find(hash, |i| {
            K::eq(&key, unsafe { self.keys.get_unchecked(i.get()) })
        })
    }

    pub fn contains(&self, key: &K) -> bool {
        self.get_index_ref(key).is_some()
    }
    pub fn get_index(&self, key: &K) -> Option<usize> {
        self.get_index_ref(key).map(|i| i.get())
    }

    pub fn remove_with_gap(&mut self, key: &K) -> bool {
        let hash = self.random_state.hash_one(key);
        match self
            .table
            .find_entry(hash, |k| K::eq(&self.keys[k.get()], key))
        {
            Ok(entry) => {
                entry.remove();
                true
            }
            Err(_) => false,
        }
    }
}
impl<K: Eq + hash::Hash, V> IndexMap<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> Result<usize, usize> {
        match self.entry(key) {
            Entry::Occuppied(entry) => {
                let idx = entry.index();
                self.values[idx] = value;
                Err(idx)
            }
            Entry::Vacant(entry) => {
                let entry = entry.insert(value);
                Ok(entry.index())
            }
        }
    }

    pub fn get_or_insert_with(&mut self, key: K, mut f: impl FnMut() -> V) -> &mut V {
        let idx = match self.entry(key) {
            Entry::Occuppied(entry) => entry.index(),
            Entry::Vacant(entry) => entry.insert(f()).index(),
        };
        &mut self.values[idx]
    }

    #[inline(always)]
    fn get_index_ref(&self, key: &K) -> Option<&NonMaxUsize> {
        let hash = self.random_state.hash_one(key);
        self.table.find(hash, |i| {
            K::eq(&key, unsafe { self.keys.get_unchecked(i.get()) })
        })
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.get_index_ref(key).is_some()
    }
    pub fn get_index(&self, key: &K) -> Option<usize> {
        self.get_index_ref(key).map(|i| i.get())
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        self.get_index(key)
            .map(|idx| unsafe { self.values.get_unchecked(idx) })
    }
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.get_index(key)
            .map(|idx| unsafe { self.values.get_unchecked_mut(idx) })
    }

    pub fn entry<'a>(&'a mut self, key: K) -> Entry<'a, K, V> {
        let hash = self.random_state.hash_one(&key);
        let entry = self.table.entry(
            hash,
            |i| K::eq(&key, unsafe { self.keys.get_unchecked(i.get()) }),
            |i| {
                let i = unsafe { self.keys.get_unchecked(i.get()) };
                self.random_state.hash_one(i)
            },
        );

        match entry {
            hashbrown::hash_table::Entry::Occupied(entry) => Entry::Occuppied(OccupiedEntry {
                entry,
                _keys: &mut self.keys,
                values: &mut self.values,
            }),
            hashbrown::hash_table::Entry::Vacant(entry) => Entry::Vacant(VacantEntry {
                entry,
                key,
                keys: &mut self.keys,
                values: &mut self.values,
            }),
        }
    }
}

impl<K: Eq + hash::Hash> Extend<K> for IndexSet<K> {
    fn extend<T: IntoIterator<Item = K>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        if let Some(size) = iter.size_hint().1 {
            self.reserve(size);
        }
        for k in iter {
            _ = self.insert_new(k);
        }
    }
}

impl<K: Eq + hash::Hash, V> Index<&K> for IndexMap<K, V> {
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        self.get(index).expect("index failed")
    }
}
impl<K: Eq + hash::Hash, V> IndexMut<&K> for IndexMap<K, V> {
    fn index_mut(&mut self, index: &K) -> &mut Self::Output {
        self.get_mut(&index).expect("index failed")
    }
}
impl<K: Eq + hash::Hash + Copy, V> Index<K> for IndexMap<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(&index).expect("index failed")
    }
}
impl<K: Eq + hash::Hash + Copy, V> IndexMut<K> for IndexMap<K, V> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut(&index).expect("index failed")
    }
}

impl<K: Eq + hash::Hash> FromIterator<K> for IndexSet<K> {
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut set = Self::new();
        for i in iter {
            set.insert(i);
        }
        set
    }
}
impl<K: Eq + hash::Hash, V> FromIterator<(K, V)> for IndexMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut map = Self::new();
        for (k, v) in iter {
            _ = map.insert(k, v);
        }
        map
    }
}
