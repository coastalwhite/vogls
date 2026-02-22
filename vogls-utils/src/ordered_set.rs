use std::{fmt, hash};

use crate::VgHashSet;

/// A list of unique values that can be recalled in insertion order.
#[derive(Clone, Default)]
pub struct OrderedSet<T> {
    pub items: Vec<T>,
    pub set: VgHashSet<T>,
}

impl<T: fmt::Debug> fmt::Debug for OrderedSet<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OrderedSet").field(&self.items).finish()
    }
}

impl<T: PartialEq> PartialEq for OrderedSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}
impl<T: Eq> Eq for OrderedSet<T> {}
impl<T: hash::Hash> hash::Hash for OrderedSet<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.items.hash(state);
    }
}

impl<T> OrderedSet<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            set: VgHashSet::default(),
        }
    }
}

impl<T: Eq + hash::Hash + Clone> OrderedSet<T> {
    pub fn insert(&mut self, value: T) -> bool {
        let inserted = self.set.insert(value.clone());
        if inserted {
            self.items.push(value);
        }
        inserted
    }
}

impl<T: Eq + hash::Hash> OrderedSet<T> {
    pub fn reserve(&mut self, additional: usize) {
        self.items.reserve(additional);
        self.set.reserve(additional);
    }

    pub fn contains(&self, value: &T) -> bool {
        self.set.contains(value)
    }
}
