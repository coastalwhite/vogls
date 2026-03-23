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
pub struct TableSet<KK, K, V> {
    table: Table<KK, (K, V)>,
    set: hashbrown::HashTable<(KK, K)>,
    random_state: foldhash::fast::RandomState,
}

impl<K, V> Default for Table<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
impl<KK, K, V> Default for TableSet<KK, K, V> {
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
impl<KK: fmt::Debug, K: fmt::Debug, V: fmt::Debug> fmt::Debug for TableSet<KK, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TableSet<{}, {}, {}> ",
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
impl<KK: TableKey, K, V> Index<KK> for TableSet<KK, K, V> {
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
impl<KK: TableKey, K, V> IndexMut<KK> for TableSet<KK, K, V> {
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

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &V> + DoubleEndedIterator {
        self.values.iter()
    }

    pub fn into_iter(self) -> impl ExactSizeIterator<Item = V> + DoubleEndedIterator {
        self.values.into_iter()
    }
}
impl<KK, K, V> TableSet<KK, K, V> {
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
}

impl<KK, K: Hash + Eq, V> TableSet<KK, K, V> {
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
}

impl<KK: TableKey, K: Copy + Hash + Eq, V> TableSet<KK, K, V> {
    pub fn get_or_insert_with(&mut self, key: K, f: impl FnOnce() -> V) -> KK {
        let hash = self.random_state.hash_one(&key);
        self.set
            .entry(
                hash,
                |(_, k)| K::eq(k, &key),
                |(_, k)| self.random_state.hash_one(k),
            )
            .or_insert_with(|| {
                let kk = self.table.insert((key, f()));
                (kk, key)
            })
            .get()
            .0
    }
}

impl<KK: TableKey, K: Copy + Hash + Eq, V: Default> TableSet<KK, K, V> {
    pub fn get_or_default(&mut self, key: K) -> KK {
        self.get_or_insert_with(key, V::default)
    }
}

impl<KK: TableKey, K: Copy, V> TableSet<KK, K, V> {
    pub fn get_key(&self, key: KK) -> K {
        self.table[key].0
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (KK, K, &V)> + DoubleEndedIterator {
        self.table_key_iter()
            .zip(self.table.iter().map(|(k, v)| (*k, v)))
            .map(|(kk, (k, v))| (kk, k, v))
    }
}

impl<KK: TableKey, K, V> TableSet<KK, K, V> {
    pub fn table_key_iter(
        &self,
    ) -> impl ExactSizeIterator<Item = KK> + DoubleEndedIterator + 'static {
        (0..self.len()).map(|i| KK::from_usize(i).expect("out-of-bounds"))
    }
}
