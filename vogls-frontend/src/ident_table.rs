use std::hash::BuildHasher;
use std::num::NonZeroU64;
use std::ops::Index;

use hashbrown::hash_table::Entry;

/// Table that allows interning strings.
///
/// Each unique string put into this table corresponds to a unique [`IdentRef`] or [`IdentId`].
///
/// This allows:
/// - Identifiers to represented by a `NonZeroU64` in other structures.
/// - Cheap hashing and equality checking on identifiers.
/// - Deduplication of string allocations.
/// - Identifiers are trivially `Copy`.
/// - Dropping identifiers (even in other structures) is a `O(1)` operation.
///
/// In general, this is a very common technique for compilers.
pub struct IdentTable {
    content: String,

    ident_id_lut: hashbrown::HashTable<((usize, usize), IdentId)>,
    /// (offset, length) in `content` taken up by identifier
    ident_refs: Vec<(usize, usize)>,

    random_state: foldhash::fast::RandomState,
}

impl Default for IdentTable {
    fn default() -> Self {
        let mut slf = Self {
            content: String::new(),
            ident_id_lut: Default::default(),
            ident_refs: Vec::new(),
            random_state: Default::default(),
        };
        slf.get_or_insert("");
        slf
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct IdentId(NonZeroU64);

impl IdentTable {
    pub const EMPTY_IDENT: IdentId = IdentId(NonZeroU64::new(1).unwrap());

    /// Get or insert a specific identifier string.
    ///
    /// If it already exists in the table, its [`IdentId`] is returned. Otherwise, it is inserted
    /// and the newly created [`IdentId`] is returned.
    pub fn get_or_insert(&mut self, ident: &str) -> IdentId {
        let hash = self.random_state.hash_one(ident);
        match self.ident_id_lut.entry(
            hash,
            |(r, _)| ident == &self.content[r.0..][..r.1],
            |(r, _)| self.random_state.hash_one(&self.content[r.0..][..r.1]),
        ) {
            Entry::Occupied(entry) => entry.get().1,
            Entry::Vacant(entry) => {
                let ident_ref = (self.content.len(), ident.len());
                self.ident_refs.push(ident_ref);
                let ident_id = IdentId(NonZeroU64::new(self.ident_refs.len() as u64).unwrap());
                self.content.push_str(ident);
                entry.insert((ident_ref, ident_id));
                ident_id
            }
        }
    }
}

impl Index<IdentId> for IdentTable {
    type Output = str;
    fn index(&self, id: IdentId) -> &Self::Output {
        let ident_ref = self.ident_refs[id.0.get() as usize - 1];
        &self.content[ident_ref.0..][..ident_ref.1]
    }
}
