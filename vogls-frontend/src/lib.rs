pub type VgHashSet<K> = hashbrown::HashSet<K, foldhash::fast::RandomState>;
pub type VgHashMap<K, V> = hashbrown::HashMap<K, V, foldhash::fast::RandomState>;

pub mod symbol_table;
pub mod ident_table;
