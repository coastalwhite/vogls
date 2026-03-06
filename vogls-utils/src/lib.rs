pub type VgHashSet<K> = hashbrown::HashSet<K, foldhash::fast::RandomState>;
pub type VgHashMap<K, V> = hashbrown::HashMap<K, V, foldhash::fast::RandomState>;

mod index_map;
mod non_max_int;
mod ordered_set;

pub use index_map::{IndexMap, IndexSet};
pub use non_max_int::{NonMaxU8, NonMaxU32, NonMaxU64, NonMaxUsize};
pub use ordered_set::OrderedSet;

/// Remainder that results the divisor if the remainder is zero.
///
/// `(a % b == 0) ? b : (a % b)`
#[inline(always)]
pub fn saturating_rem<T: std::ops::Rem<T, Output = T> + Copy + PartialEq + Default>(
    a: T,
    b: T,
) -> T {
    let r = a % b;
    if r == T::default() { b } else { r }
}
