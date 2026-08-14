use std::mem::offset_of;
use std::ptr::NonNull;

/// An FFI safe vector wrapper.
#[repr(C)]
pub struct FfiVec<T> {
    ptr: *mut T,
    length: usize,
    capacity: usize,
    grow: extern "C" fn(NonNull<Self>),
}

impl<T> Default for FfiVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> FfiVec<T> {
    pub const PTR_OFFSET: usize = offset_of!(Self, ptr);
    pub const LEN_OFFSET: usize = offset_of!(Self, length);
    pub const CAP_OFFSET: usize = offset_of!(Self, capacity);
    pub const GROW_OFFSET: usize = offset_of!(Self, grow);

    pub const fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    pub const fn from_vec(mut vec: Vec<T>) -> Self {
        extern "C" fn grow<T>(mut slf: NonNull<FfiVec<T>>) {
            let slf = unsafe { slf.as_mut() };
            let mut v = unsafe { Vec::from_raw_parts(slf.ptr, slf.length, slf.capacity) };
            v.reserve(slf.capacity.max(1));
            slf.ptr = v.as_mut_ptr();
            slf.length = v.len();
            slf.capacity = v.capacity();
            std::mem::forget(v);
        }

        let slf = Self {
            ptr: vec.as_mut_ptr(),
            length: vec.len(),
            capacity: vec.capacity(),
            grow,
        };
        std::mem::forget(vec);
        slf
    }

    /// Inline always
    pub fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            None
        } else {
            unsafe {
                self.length -= 1;
                core::hint::assert_unchecked(self.length < self.capacity);
                Some(std::ptr::read(self.ptr.add(self.length)))
            }
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.length
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        if std::mem::needs_drop::<T>() {
            for i in 0..self.length {
                drop(unsafe { self.ptr.add(i).read() });
            }
        }
        self.length = 0;
    }

    /// Extend with items from the `values`.
    ///
    /// # Safety
    ///
    /// No capacity is reserved for values and thus it is assumed that enough capacity is
    /// available.
    pub unsafe fn extend_from_slice_unchecked(&mut self, values: &[T]) {
        debug_assert!(self.capacity() - self.len() >= values.len());

        // SAFETY: Precondition on the function.
        unsafe {
            std::ptr::copy_nonoverlapping(values.as_ptr(), self.ptr.add(self.length), values.len());
        }
        self.length += values.len();
    }
}

impl<T> From<Vec<T>> for FfiVec<T> {
    #[inline(always)]
    fn from(value: Vec<T>) -> Self {
        Self::from_vec(value)
    }
}

impl<T> From<FfiVec<T>> for Vec<T> {
    fn from(value: FfiVec<T>) -> Vec<T> {
        let v = unsafe { Vec::from_raw_parts(value.ptr, value.length, value.capacity) };
        std::mem::forget(value);
        v
    }
}

impl<T> Drop for FfiVec<T> {
    fn drop(&mut self) {
        _ = unsafe { Vec::from_raw_parts(self.ptr, self.length, self.capacity) }
    }
}

impl<T> AsRef<[T]> for FfiVec<T> {
    fn as_ref(&self) -> &[T] {
        if self.length == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, self.length) }
        }
    }
}
impl<T> AsMut<[T]> for FfiVec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        if self.length == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.ptr, self.length) }
        }
    }
}
