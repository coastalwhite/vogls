use std::hash::{BuildHasher, Hasher};

use hashbrown::hash_table::Entry;
use vogls::design::DesignState;
use vogls::utils::{Table, TableKey};
use vogls_trace::Trace;

macro_rules! impl_dyn_eq_hash {
    ($ty:ident) => {
        fn csp_eq(&self, other: &dyn $ty) -> bool {
            let Some(other) = (other as &dyn std::any::Any).downcast_ref::<Self>() else {
                return false;
            };
            self == other
        }
        fn csp_hash(&self, mut state: &mut dyn std::hash::Hasher) {
            std::hash::Hash::hash(self, &mut state);
        }
    };
}

pub mod agg;
pub mod array;
pub mod buffer;
pub mod compute;
pub mod design;
pub mod dsl;
pub mod entropy;
pub mod expand;
pub mod map;
pub mod mutual_information;
#[cfg(feature = "python")]
pub mod numpy;
pub mod output;
pub mod pearson_corr;
pub mod plan;
pub mod random;
pub mod run;
pub mod run_vector;
pub mod ttest;
pub mod typing;
pub mod value;
pub mod window_sum;

pub struct TraceRef(usize);

impl TraceRef {
    pub fn extract(&self, state: &mut DesignState) -> Trace {
        let plugins = match &mut *state {
            DesignState::Bytecode(s) => &mut s.plugins,
            DesignState::Cranelift(s) => &mut s.plugins,
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
