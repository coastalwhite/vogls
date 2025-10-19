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
impl<T> Default for ArenaIdRange<T> {
    fn default() -> Self {
        Self {
            start: 0,
            length: 0,
            _pd: PhantomData::default(),
        }
    }
}

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
    pub fn iter(self) -> ArenaIdRangeIter<T> {
        ArenaIdRangeIter { inner: self }
    }
}

pub struct ArenaIdRangeIter<T> {
    inner: ArenaIdRange<T>,
}

impl<T> Iterator for ArenaIdRangeIter<T> {
    type Item = ArenaId<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.inner.first()?;
        self.inner.start += size_of::<T>().div_ceil(size_of::<u64>());
        self.inner.length -= 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.inner.len(), Some(self.inner.len()))
    }
}
impl<T> DoubleEndedIterator for ArenaIdRangeIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let value = self.inner.last()?;
        self.inner.length -= 1;
        Some(value)
    }
}
impl<T> ExactSizeIterator for ArenaIdRangeIter<T> {}
impl<T> IntoIterator for ArenaIdRange<T> {
    type Item = ArenaId<T>;
    type IntoIter = ArenaIdRangeIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
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

    pub fn add<T: Copy>(&mut self, item: T) -> ArenaId<T> {
        const { assert!(align_of::<T>() == align_of::<u64>()) }

        let ptr = self.items.len();
        let size = size_of::<T>().div_ceil(size_of::<u64>());
        if size > 0 {
            self.items.reserve(size);

            // SAFETY: This space is reserved.
            unsafe {
                self.items.as_mut_ptr().add(ptr).cast::<T>().write(item);
                self.items.set_len(ptr + size);
            }
        }

        ArenaId {
            ptr,
            _pd: PhantomData::default(),
        }
    }

    pub fn extend<T: Copy>(&mut self, items: impl IntoIterator<Item = T>) -> ArenaIdRange<T> {
        const { assert!(align_of::<T>() == align_of::<u64>()) }

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
        assert!(
            item.ptr
                .checked_add(size_of::<T>().div_ceil(size_of::<u64>()))
                .unwrap()
                <= self.items.len()
        );
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
        assert!(
            item.ptr
                .checked_add(size_of::<T>().div_ceil(size_of::<u64>()))
                .unwrap()
                <= self.items.len()
        );
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

    pub fn get_slice<T>(&self, item_range: ArenaIdRange<T>) -> &[T] {
        assert!(
            item_range
                .start
                .checked_add(
                    item_range
                        .length
                        .checked_mul(size_of::<T>().div_ceil(size_of::<u64>()))
                        .unwrap()
                )
                .unwrap()
                <= self.items.len()
        );
        unsafe {
            std::slice::from_raw_parts(
                self.items.as_ptr().add(item_range.start).cast::<T>(),
                item_range.len(),
            )
        }
    }
}

impl Arena {
    pub fn take<T: Default>(&mut self, item: ArenaId<T>) -> T {
        let x = self.get_mut(item);
        std::mem::take(x)
    }
}
