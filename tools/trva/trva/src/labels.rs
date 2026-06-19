use std::hash::{BuildHasher, Hash};
use std::num::NonZeroU64;
use std::ops::{Index, IndexMut};

#[cfg(feature = "foldhash")]
use foldhash::fast::RandomState;
#[cfg(not(feature = "foldhash"))]
use std::hash::RandomState;


use hashbrown::HashTable;
use hashbrown::hash_table::Entry;

/// An unique identifier for an assembly label.
///
/// This is used in place of a string to allow for easier storage and cheap comparison, hashing and
/// look-up in a [`LabelMap`].
///
/// The [`LabelTable`] keeps track of which labels have been seen and allows recovering the
/// original string from a [`LabelId`]. 
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LabelId(NonZeroU64);

/// A table of seen labels.
#[derive(Clone)]
pub struct LabelTable {
    content: String,
    lut: HashTable<((usize, usize), LabelId)>,
    /// (offset, length) in `content` taken up by identifier
    refs: Vec<(usize, usize)>,

    hash_builder: RandomState,
}

/// A map from a [`LabelId`] to a value.
pub struct LabelMap<T>(Vec<Option<T>>);

impl Default for LabelTable {
    fn default() -> Self {
        Self {
            content: String::new(),
            lut: Default::default(),
            refs: Vec::new(),

            hash_builder: RandomState::default(),
        }
    }
}
impl<T> Default for LabelMap<T> {
    fn default() -> Self {
        Self(Vec::default())
    }
}

impl LabelTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or insert a specific identifier string.
    ///
    /// If it already exists in the table, its [`LabelId`] is returned. Otherwise, it is inserted
    /// and the newly created [`LabelId`] is returned.
    pub fn get_or_insert(&mut self, ident: &str) -> LabelId {
        let hash = self.hash_builder.hash_one(ident);
        match self.lut.entry(
            hash,
            |(r, _)| ident == &self.content[r.0..][..r.1],
            |(r, _)| self.hash_builder.hash_one(&self.content[r.0..][..r.1]),
        ) {
            Entry::Occupied(entry) => entry.get().1,
            Entry::Vacant(entry) => {
                let ident_ref = (self.content.len(), ident.len());
                self.refs.push(ident_ref);
                let ident_id = LabelId(NonZeroU64::new(self.refs.len() as u64).unwrap());
                self.content.push_str(ident);
                entry.insert((ident_ref, ident_id));
                ident_id
            }
        }
    }
}

impl<T> LabelMap<T> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    fn grow_to_key(&mut self, key: LabelId) {
        self.0
            .resize_with(self.0.len().max(key.0.get() as usize), || None);
    }

    pub fn set(&mut self, key: LabelId, value: T) -> Option<T> {
        self.grow_to_key(key);
        self.0[key.0.get() as usize - 1].replace(value)
    }

    pub fn with_len(length: usize) -> Self {
        Self(std::iter::repeat_with(|| None).take(length).collect())
    }
}

impl Index<LabelId> for LabelTable {
    type Output = str;
    fn index(&self, id: LabelId) -> &Self::Output {
        let ident_ref = self.refs[id.0.get() as usize - 1];
        &self.content[ident_ref.0..][..ident_ref.1]
    }
}

impl<T> Index<LabelId> for LabelMap<T> {
    type Output = Option<T>;
    fn index(&self, id: LabelId) -> &Self::Output {
        match self.0.get(id.0.get() as usize - 1) {
            None => &None,
            Some(v) => v,
        }
    }
}

impl<T> IndexMut<LabelId> for LabelMap<T> {
    fn index_mut(&mut self, id: LabelId) -> &mut Self::Output {
        self.grow_to_key(id);
        &mut self.0[id.0.get() as usize - 1]
    }
}
