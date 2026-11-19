use foldhash::fast::RandomState;
use std::hash::Hash;

use crate::VgHashSet;

pub struct SeenSet<T> {
    stack: Vec<T>,
    seen: VgHashSet<T>,
}

impl<T> SeenSet<T> {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            seen: VgHashSet::default(),
        }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            stack: Vec::with_capacity(capacity),
            seen: VgHashSet::with_capacity_and_hasher(capacity, RandomState::default()),
        }
    }
    pub fn clear(&mut self) {
        self.stack.clear();
        self.seen.clear();
    }
}
impl<T: Eq + Hash + Copy> SeenSet<T> {
    pub fn reserve(&mut self, additional: usize) {
        self.stack.reserve(additional);
        self.seen.reserve(additional);
    }

    pub fn insert(&mut self, item: T) -> bool {
        let seen = self.seen.insert(item);
        if seen {
            self.stack.push(item);
        }
        seen
    }
    pub fn pop(&mut self) -> Option<T> {
        self.stack.pop()
    }
    pub fn contains(&self, item: T) -> bool {
        self.seen.contains(&item)
    }
}
