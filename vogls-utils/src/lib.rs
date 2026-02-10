pub type VgHashSet<K> = hashbrown::HashSet<K, foldhash::fast::RandomState>;
pub type VgHashMap<K, V> = hashbrown::HashMap<K, V, foldhash::fast::RandomState>;

mod non_max_int;

pub use non_max_int::{NonMaxU32, NonMaxU64, NonMaxUsize};
