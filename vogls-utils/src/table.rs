use std::fmt;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

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

pub trait TableKey: Sized {
    fn get(self) -> usize;
    fn from_usize(value: usize) -> Option<Self>;
}

pub struct Table<K, V> {
    values: Vec<V>,
    _pd: PhantomData<K>,
}

impl<K, V> Default for Table<K, V> {
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

impl<K: TableKey, V> Index<K> for Table<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        &self.values[index.get()]
    }
}

impl<K: TableKey, V> IndexMut<K> for Table<K, V> {
    fn index_mut(&mut self, index: K) -> &mut <Self as Index<K>>::Output {
        &mut self.values[index.get()]
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
}

impl<K: TableKey, V> Table<K, V> {
    pub fn insert(&mut self, value: V) -> K {
        let key = K::from_usize(self.values.len()).expect("Table overflow");
        self.values.push(value);
        key
    }
}
