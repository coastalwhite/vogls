use std::fmt;
use std::hash::BuildHasher;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use foldhash::fast::RandomState;

#[macro_export]
macro_rules! new_table_key {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ;
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(transparent)]
        $vis struct $name($crate::NonMaxUsize);

        impl $crate::TableKey for $name {
            fn get(self) -> usize {
                self.0.get()
            }
            fn from_usize(value: usize) -> Option<Self> {
                $crate::NonMaxUsize::new(value).map(Self)
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0.get()).finish()
            }
        }
    };
}

pub trait TableKey: Sized + Copy {
    fn get(self) -> usize;
    fn from_usize(value: usize) -> Option<Self>;
}

#[derive(Clone)]
pub struct Table<K, V> {
    values: Vec<V>,
    _pd: PhantomData<K>,
}

#[derive(Clone)]
pub struct SecondaryTable<K, V>(Table<K, Option<V>>);

#[derive(Clone)]
pub struct TableMap<KK, K, V> {
    table: Table<KK, (K, V)>,
    set: hashbrown::HashTable<(KK, K)>,
    random_state: foldhash::fast::RandomState,
}

pub enum TableMapEntry<'a, KK, K, V> {
    Occupied(OccupiedEntry<'a, KK, K, V>),
    Vacant(VacantEntry<'a, KK, K, V>),
}
pub struct OccupiedEntry<'a, KK, K, V> {
    table: &'a mut Table<KK, (K, V)>,
    ht_entry: hashbrown::hash_table::OccupiedEntry<'a, (KK, K)>,
}
pub struct VacantEntry<'a, KK, K, V> {
    key: K,
    table: &'a mut Table<KK, (K, V)>,
    ht_entry: hashbrown::hash_table::VacantEntry<'a, (KK, K)>,
}

impl<'a, KK: TableKey, K, V> OccupiedEntry<'a, KK, K, V> {
    pub fn get(&self) -> &V {
        &self.table[self.get_table_key()].1
    }
    pub fn get_table_key(&self) -> KK {
        self.ht_entry.get().0
    }
    pub fn key(&self) -> &K {
        &self.ht_entry.get().1
    }

    pub fn set(&mut self, value: V) {
        let tk = self.get_table_key();
        self.table[tk].1 = value;
    }
}
impl<'a, KK: TableKey, K: Clone, V> VacantEntry<'a, KK, K, V> {
    pub fn insert(self, value: V) -> OccupiedEntry<'a, KK, K, V> {
        let tk = self.table.insert((self.key.clone(), value));
        OccupiedEntry {
            table: self.table,
            ht_entry: self.ht_entry.insert((tk, self.key)),
        }
    }
}

impl<K, V> Default for Table<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K, V> Default for SecondaryTable<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
impl<KK, K, V> Default for TableMap<KK, K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Table<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Table<{}, {}> ",
            std::any::type_name::<K>(),
            std::any::type_name::<V>()
        )?;
        let mut map = f.debug_map();
        for (i, v) in self.values.iter().enumerate() {
            map.entry(&i, &v);
        }
        map.finish()
    }
}
impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for SecondaryTable<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SecondaryTable<{}, {}> ",
            std::any::type_name::<K>(),
            std::any::type_name::<V>()
        )?;
        let mut map = f.debug_map();
        for (i, v) in self.0.values.iter().enumerate() {
            map.entry(&i, &v);
        }
        map.finish()
    }
}
impl<KK: fmt::Debug, K: fmt::Debug, V: fmt::Debug> fmt::Debug for TableMap<KK, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TableMap<{}, {}, {}> ",
            std::any::type_name::<KK>(),
            std::any::type_name::<K>(),
            std::any::type_name::<V>()
        )?;
        let mut map = f.debug_map();
        for (i, v) in self.table.values.iter().enumerate() {
            map.entry(&i, &v);
        }
        map.finish()
    }
}

impl<K: TableKey, V> Index<K> for Table<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        &self.values[index.get()]
    }
}
impl<K: TableKey, V> Index<K> for SecondaryTable<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.0[index].as_ref().unwrap()
    }
}
impl<KK: TableKey, K, V> Index<KK> for TableMap<KK, K, V> {
    type Output = V;

    fn index(&self, index: KK) -> &Self::Output {
        &self.table[index].1
    }
}

impl<K: TableKey, V> IndexMut<K> for Table<K, V> {
    fn index_mut(&mut self, index: K) -> &mut <Self as Index<K>>::Output {
        &mut self.values[index.get()]
    }
}
impl<K: TableKey, V> IndexMut<K> for SecondaryTable<K, V> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.0[index].as_mut().unwrap()
    }
}
impl<KK: TableKey, K, V> IndexMut<KK> for TableMap<KK, K, V> {
    fn index_mut(&mut self, index: KK) -> &mut Self::Output {
        &mut self.table[index].1
    }
}

impl<K, V> Table<K, V> {
    pub const fn new() -> Self {
        Self {
            values: Vec::new(),
            _pd: PhantomData,
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.values.reserve(additional);
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn capacity(&self) -> usize {
        self.values.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &V> + DoubleEndedIterator {
        self.values.iter()
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn values(&self) -> &[V] {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut [V] {
        &mut self.values
    }
}

impl<K, V> IntoIterator for Table<K, V> {
    type Item = V;
    type IntoIter = std::vec::IntoIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<K, V> SecondaryTable<K, V> {
    pub const fn new() -> Self {
        Self(Table::new())
    }

    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<K: TableKey, V> SecondaryTable<K, V> {
    pub fn reserve_until(&mut self, key: K) {
        struct ExtendNone<V>(usize, PhantomData<V>);
        impl<V> Iterator for ExtendNone<V> {
            type Item = Option<V>;
            fn next(&mut self) -> Option<Self::Item> {
                if self.0 == 0 {
                    None
                } else {
                    self.0 -= 1;
                    Some(None)
                }
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.0, Some(self.0))
            }
        }
        impl<V> ExactSizeIterator for ExtendNone<V> {}
        self.0.values.extend(ExtendNone(
            (key.get() + 1).saturating_sub(self.len()),
            PhantomData,
        ));
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.reserve_until(key);
        self.0[key].replace(value)
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        if self.capacity() > key.get() {
            self.0[key].take()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.reserve_until(key);
        self.0[key].as_mut()
    }

    pub fn or_insert_with(&mut self, key: K, mut f: impl FnMut() -> V) {
        self.reserve_until(key);
        let v = &mut self.0[key];
        if v.is_none() {
            *v = Some(f());
        }
    }
}

impl<KK, K, V> TableMap<KK, K, V> {
    pub fn new() -> Self {
        Self {
            table: Table::new(),
            set: hashbrown::HashTable::new(),
            random_state: RandomState::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    pub fn key_value_iter(&self) -> impl ExactSizeIterator<Item = (&K, &V)> + DoubleEndedIterator {
        self.table.iter().map(|(k, v)| (k, v))
    }

    pub fn into_key_value_iter(
        self,
    ) -> impl ExactSizeIterator<Item = (K, V)> + DoubleEndedIterator {
        self.table.into_iter()
    }

    pub fn clear(&mut self) {
        self.table.clear();
        self.set.clear();
    }
}

impl<KK, K: Hash + Eq, V> TableMap<KK, K, V> {
    pub fn reserve(&mut self, additional: usize) {
        self.table.reserve(additional);
        self.set
            .reserve(additional, |(_, k)| self.random_state.hash_one(k));
    }
}

impl<K: TableKey, V> Table<K, V> {
    pub fn insert(&mut self, value: V) -> K {
        let key = K::from_usize(self.values.len()).expect("Table overflow");
        self.values.push(value);
        key
    }

    pub fn table_key_iter(
        &self,
    ) -> impl ExactSizeIterator<Item = K> + DoubleEndedIterator + 'static {
        (0..self.len()).map(|i| K::from_usize(i).expect("out-of-bounds"))
    }

    pub fn key_value_iter(&self) -> impl ExactSizeIterator<Item = (K, &V)> + DoubleEndedIterator {
        self.table_key_iter().zip(self.values.iter())
    }
}

impl<KK: TableKey, K: Clone + Hash + Eq, V> TableMap<KK, K, V> {
    pub fn insert_new(&mut self, key: K, value: V) -> Option<KK> {
        let hash = self.random_state.hash_one(&key);
        match self.set.entry(
            hash,
            |(_, k)| K::eq(k, &key),
            |(_, k)| self.random_state.hash_one(k),
        ) {
            hashbrown::hash_table::Entry::Occupied(_) => None,
            hashbrown::hash_table::Entry::Vacant(entry) => {
                let kk = self.table.insert((key.clone(), value));
                entry.insert((kk, key));
                Some(kk)
            }
        }
    }

    pub fn get_or_insert_with(&mut self, key: K, f: impl FnOnce() -> V) -> KK {
        let hash = self.random_state.hash_one(&key);
        self.set
            .entry(
                hash,
                |(_, k)| K::eq(k, &key),
                |(_, k)| self.random_state.hash_one(k),
            )
            .or_insert_with(|| {
                let kk = self.table.insert((key.clone(), f()));
                (kk, key)
            })
            .get()
            .0
    }
}

impl<KK: TableKey, K: Hash + Eq, V> TableMap<KK, K, V> {
    pub fn entry<'a>(&'a mut self, key: K) -> TableMapEntry<'a, KK, K, V> {
        let hash = self.random_state.hash_one(&key);
        let ht_entry = self.set.entry(
            hash,
            |(_, k)| K::eq(k, &key),
            |(_, k)| self.random_state.hash_one(k),
        );
        match ht_entry {
            hashbrown::hash_table::Entry::Occupied(ht_entry) => {
                TableMapEntry::Occupied(OccupiedEntry {
                    table: &mut self.table,
                    ht_entry,
                })
            }
            hashbrown::hash_table::Entry::Vacant(ht_entry) => TableMapEntry::Vacant(VacantEntry {
                table: &mut self.table,
                ht_entry,
                key,
            }),
        }
    }
}

impl<KK: TableKey, K: Clone + Hash + Eq, V: Default> TableMap<KK, K, V> {
    pub fn get_or_default(&mut self, key: K) -> KK {
        self.get_or_insert_with(key, V::default)
    }
}

impl<KK: TableKey, K, V> TableMap<KK, K, V> {
    pub fn get_key(&self, key: KK) -> &K {
        &self.table[key].0
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (KK, &K, &V)> + DoubleEndedIterator {
        self.table_key_iter()
            .zip(self.table.iter().map(|(k, v)| (k, v)))
            .map(|(kk, (k, v))| (kk, k, v))
    }
}

impl<KK: TableKey, K, V> TableMap<KK, K, V> {
    pub fn table_key_iter(
        &self,
    ) -> impl ExactSizeIterator<Item = KK> + DoubleEndedIterator + 'static {
        (0..self.len()).map(|i| KK::from_usize(i).expect("out-of-bounds"))
    }
}
