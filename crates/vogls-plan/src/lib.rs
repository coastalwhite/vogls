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

/// Monomorphize `$body` once per `UInt` width, with `$T` naming the element type.
macro_rules! with_match_uint_type {(
    $uint_type:expr, | $_:tt $T:ident | $($body:tt)*
) => ({
    macro_rules! __with_uint_ty__ {( $_ $T:ident ) => ( $($body)* )}
    use $crate::typing::UIntType::*;
    match $uint_type {
        U8 => __with_uint_ty__! { u8 },
        U16 => __with_uint_ty__! { u16 },
        U32 => __with_uint_ty__! { u32 },
        U64 => __with_uint_ty__! { u64 },
    }
})}

/// [`with_match_uint_type`] for two independently-typed widths.
macro_rules! with_match_uint_type_pair {(
    $lhs_type:expr, $rhs_type:expr, | $_:tt $T:ident, $__:tt $U:ident | $($body:tt)*
) => ({
    macro_rules! __with_uint_ty2__ {( $_ $T:ident, $__ $U:ident ) => ( $($body)* )}
    use $crate::typing::UIntType::*;
    match ($lhs_type, $rhs_type) {
        (U8, U8) => __with_uint_ty2__! { u8, u8 },
        (U8, U16) => __with_uint_ty2__! { u8, u16 },
        (U8, U32) => __with_uint_ty2__! { u8, u32 },
        (U8, U64) => __with_uint_ty2__! { u8, u64 },
        (U16, U8) => __with_uint_ty2__! { u16, u8 },
        (U16, U16) => __with_uint_ty2__! { u16, u16 },
        (U16, U32) => __with_uint_ty2__! { u16, u32 },
        (U16, U64) => __with_uint_ty2__! { u16, u64 },
        (U32, U8) => __with_uint_ty2__! { u32, u8 },
        (U32, U16) => __with_uint_ty2__! { u32, u16 },
        (U32, U32) => __with_uint_ty2__! { u32, u32 },
        (U32, U64) => __with_uint_ty2__! { u32, u64 },
        (U64, U8) => __with_uint_ty2__! { u64, u8 },
        (U64, U16) => __with_uint_ty2__! { u64, u16 },
        (U64, U32) => __with_uint_ty2__! { u64, u32 },
        (U64, U64) => __with_uint_ty2__! { u64, u64 },
    }
})}

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
