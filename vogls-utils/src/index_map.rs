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
        f.debug_tuple("IndexSet")
            .field(&self.keys)
            .field(&self.values)
            .finish()
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
}

impl<K: Eq + hash::Hash> IndexSet<K> {
    pub fn insert(&mut self, key: K) -> bool {
        self.insert_new(key).is_ok()
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
}
impl<K: Eq + hash::Hash, V> IndexMap<K, V> {
    pub fn insert(&mut self, key: K, value: V) -> Option<usize> {
        let hash = self.random_state.hash_one(&key);
        match self.table.entry(
            hash,
            |i| K::eq(&key, unsafe { self.keys.get_unchecked(i.get()) }),
            |i| {
                let i = unsafe { self.keys.get_unchecked(i.get()) };
                self.random_state.hash_one(i)
            },
        ) {
            hashbrown::hash_table::Entry::Occupied(i) => Some(i.get().get()),
            hashbrown::hash_table::Entry::Vacant(i) => {
                i.insert(NonMaxUsize::new(self.keys.len()).unwrap());
                self.keys.push(key);
                self.values.push(value);
                None
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
            map.insert(k, v);
        }
        map
    }
}
