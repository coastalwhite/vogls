use std::hash::{BuildHasher, Hasher};

use hashbrown::hash_table::Entry;
use vogls::design::DesignState;
use vogls::utils::{Table, TableKey};
use vogls_trace::Trace;

pub mod array;
pub mod compute;
pub mod design;
pub mod dsl;
pub mod output;
pub mod plan;
pub mod run;

pub struct TraceRef(usize);

impl TraceRef {
    pub fn extract(&self, state: &mut DesignState) -> Trace {
        let plugins = match &mut *state {
            DesignState::Interpretted(s) => &mut s.plugins,
            DesignState::Compiled(s) => &mut s.plugins,
        };
        let trace = plugins.remove(self.0);
        let trace = trace as Box<dyn std::any::Any>;
        let trace = trace.downcast::<vogls_trace::TracePlugin>().unwrap();
        Trace {
            trace: trace.trace,
            time_offsets: trace.time_offsets,
        }
    }
}

pub trait CspAble {
    fn csp_eq(&self, other: &Self) -> bool;
    fn csp_hash<H: Hasher>(&self, state: &mut H);
    fn csp_merge(&mut self, other: Self);
}

impl<T: std::hash::Hash + Eq> CspAble for T {
    fn csp_eq(&self, other: &Self) -> bool {
        self.eq(other)
    }
    fn csp_hash<H: Hasher>(&self, state: &mut H) {
        self.hash(state);
    }
    fn csp_merge(&mut self, _other: Self) {}
}

pub struct CspWrap<T>(pub T);
impl<T: CspAble> std::hash::Hash for CspWrap<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.csp_hash(state)
    }
}
impl<T: CspAble> PartialEq for CspWrap<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.csp_eq(&other.0)
    }
}
impl<T: CspAble> Eq for CspWrap<T> {}
impl<T: CspAble> CspWrap<T> {
    pub fn merge(&mut self, other: Self) {
        self.0.csp_merge(other.0)
    }
}

pub struct CspTable<K> {
    hash_table: hashbrown::HashTable<K>,
    random_state: foldhash::fast::RandomState,
}
impl<K> Default for CspTable<K> {
    fn default() -> Self {
        Self {
            hash_table: Default::default(),
            random_state: Default::default(),
        }
    }
}

impl<K: TableKey> CspTable<K> {
    pub fn insert<V: CspAble>(&mut self, table: &mut Table<K, V>, value: V) -> K {
        let mut hasher = self.random_state.build_hasher();
        value.csp_hash(&mut hasher);
        let hash = hasher.finish();

        match self.hash_table.entry(
            hash,
            |k| V::csp_eq(&table[*k], &value),
            |k| {
                let mut hasher = self.random_state.build_hasher();
                table[*k].csp_hash(&mut hasher);
                hasher.finish()
            },
        ) {
            Entry::Occupied(entry) => {
                V::csp_merge(&mut table[*entry.get()], value);
                *entry.get()
            }
            Entry::Vacant(entry) => {
                let key = table.insert(value);
                entry.insert(key);
                key
            }
        }
    }
}
