use std::mem::ManuallyDrop;

#[repr(C)]
pub struct InlineVec<T> {
    length: usize,
    data: InlineVecData<T>,
}

union InlineVecData<T> {
    ptr: ManuallyDrop<InlineVecDataPtr<T>>,
    inline: [u8; size_of::<usize>() * 2 ],
}

struct InlineVecDataPtr<T> {
    capacity: usize, data: *mut T
}

impl<T> InlineVec<T> {
    pub const fn new() -> Self {
        const {
            assert!(align_of::<T>() <= align_of::<usize>());
            assert!(size_of::<T>() <= size_of::<usize>() * 2);
        };
        Self {
            length: 0,
            data: InlineVecData { inline: [0u8; _] },
        }
    }

    pub const fn max_num_inline_elems() -> usize {
        (2 * size_of::<usize>()) / size_of::<T>().next_multiple_of(align_of::<T>())
    }

    fn is_inline(&self) -> bool {
        self.length >> (usize::BITS - 1) != 0
    }

    pub fn len(&self) -> usize {
        self.length & (usize::MAX >> 1)
    }
    pub fn capacity(&self) -> usize {
        if self.is_inline() {
            Self::max_num_inline_elems()
        } else {
            unsafe { &self.data.ptr }.capacity
        }
    }

    pub fn push(&mut self, item: T) {
        let new_length = self.length.checked_add(1).expect("Vec overflow");
        if self.capacity() < new_length {
            self.reserve(new_length - self.capacity());
        }
        if self.is_inline() {
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        todo!()
    }

    pub fn as_slice(&self) -> &[T] {
        if self.is_inline() {
        } else {
        }
    }
}
