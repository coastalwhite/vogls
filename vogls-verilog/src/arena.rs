use std::fmt;
use std::marker::PhantomData;

pub struct ArenaId<T> {
    ptr: usize,
    _pd: std::marker::PhantomData<T>,
}
pub struct ArenaIdRange<T> {
    start: usize,
    length: usize,
    _pd: std::marker::PhantomData<T>,
}

impl<T> Clone for ArenaId<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            _pd: PhantomData::default(),
        }
    }
}
impl<T> Copy for ArenaId<T> {}

impl<T> fmt::Debug for ArenaId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArenaItem<{}>({})", std::any::type_name::<T>(), self.ptr)
    }
}
impl<T> Clone for ArenaIdRange<T> {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            length: self.length,
            _pd: PhantomData::default(),
        }
    }
}
impl<T> Copy for ArenaIdRange<T> {}

impl<T> ArenaIdRange<T> {
    const fn num_cells() -> usize {
        size_of::<T>().div_ceil(size_of::<u64>())
    }

    pub fn first(self) -> Option<ArenaId<T>> {
        (!self.is_empty()).then_some(ArenaId {
            ptr: self.start,
            _pd: PhantomData::default(),
        })
    }
    pub fn last(self) -> Option<ArenaId<T>> {
        (!self.is_empty()).then_some(ArenaId {
            ptr: self.start + (self.length - 1) * Self::num_cells(),
            _pd: PhantomData::default(),
        })
    }
    pub fn len(self) -> usize {
        self.length
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn iter(self) -> impl Iterator<Item = ArenaId<T>> {
        let start = self.start;
        (0..self.length).map(move |i| ArenaId {
            ptr: start + i * Self::num_cells(),
            _pd: PhantomData::default(),
        })
    }
}

#[derive(Default)]
pub struct Arena {
    items: Vec<u64>,
}

impl Arena {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    pub fn add<T>(&mut self, item: T) -> ArenaId<T> {
        const { assert!(align_of::<T>() <= 8) }

        let ptr = self.items.len();
        let size = size_of::<T>().div_ceil(8);
        if size > 0 {
            self.items.reserve(size);

            // SAFETY: This space is reserved.
            unsafe {
                self.items.as_mut_ptr().cast::<T>().write(item);
                self.items.set_len(ptr + size);
            }
        }

        ArenaId {
            ptr,
            _pd: PhantomData::default(),
        }
    }

    pub fn extend<T>(&mut self, items: impl IntoIterator<Item = T>) -> ArenaIdRange<T> {
        const { assert!(align_of::<T>() <= 8) }

        let start = self.items.len();
        let mut length = 0;
        for item in items.into_iter() {
            self.add(item);
            length += 1;
        }

        ArenaIdRange {
            start,
            length,
            _pd: PhantomData::default(),
        }
    }

    pub fn get<T>(&self, item: ArenaId<T>) -> &T {
        assert!(item.ptr.checked_add(size_of::<T>()).unwrap() < self.items.len());
        unsafe {
            self.items
                .as_ptr()
                .add(item.ptr)
                .cast::<T>()
                .as_ref()
                .unwrap()
        }
    }

    pub fn get_mut<T>(&mut self, item: ArenaId<T>) -> &mut T {
        assert!(item.ptr.checked_add(size_of::<T>()).unwrap() < self.items.len());
        unsafe {
            self.items
                .as_mut_ptr()
                .add(item.ptr)
                .cast::<T>()
                .as_mut()
                .unwrap()
        }
    }

    pub fn replace<T>(&mut self, item: ArenaId<T>, value: T) -> T {
        std::mem::replace(self.get_mut(item), value)
    }
}

impl Arena {
    pub fn take<T: Default>(&mut self, item: ArenaId<T>) -> T {
        let x = self.get_mut(item);
        std::mem::take(x)
    }
}
