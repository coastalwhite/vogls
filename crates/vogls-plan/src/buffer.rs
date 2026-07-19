//! A zero-copy sliceable buffer. This is mostly adapted from Polars.
use std::any::Any;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Buffer<T> {
    ptr: *const T,
    length: usize,

    storage: SharedStorage<T>,
}

#[repr(transparent)]
pub struct SharedStorage<T> {
    inner: NonNull<SharedStorageInner<T>>,
    phantom: PhantomData<SharedStorageInner<T>>,
}

struct SharedStorageInner<T> {
    ref_count: AtomicU64,
    ptr: *mut T,
    length_in_bytes: usize,
    backing: BackingStorage,
    // https://github.com/rust-lang/rfcs/blob/master/text/0769-sound-generic-drop.md#phantom-data
    phantom: PhantomData<T>,
}

unsafe impl<T: Sync + Send> Sync for SharedStorageInner<T> {}

unsafe impl<T: Send + Sync> Sync for Buffer<T> {}
unsafe impl<T: Send + Sync> Send for Buffer<T> {}

impl<T: std::hash::Hash> std::hash::Hash for Buffer<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

enum BackingStorage {
    Vec {
        original_capacity: usize, // Elements, not bytes.
        vtable: &'static VecVTable,
    },
    ForeignOwner(Box<dyn Any + Send + 'static>),
    External,
    Leaked,
}

unsafe impl<T: Sync + Send> Send for SharedStorage<T> {}
unsafe impl<T: Sync + Send> Sync for SharedStorage<T> {}

// Allows us to transmute between types while also keeping the original
// stats and drop method of the Vec around.
#[expect(unused)]
struct VecVTable {
    size: usize,
    align: usize,
    drop_buffer: unsafe fn(*mut (), usize),
}

impl VecVTable {
    const fn new<T>() -> Self {
        unsafe fn drop_buffer<T>(ptr: *mut (), cap: usize) {
            unsafe { drop(Vec::from_raw_parts(ptr.cast::<T>(), 0, cap)) }
        }

        Self {
            size: size_of::<T>(),
            align: align_of::<T>(),
            drop_buffer: drop_buffer::<T>,
        }
    }

    fn new_static<T>() -> &'static Self {
        const { &Self::new::<T>() }
    }
}

impl<T> Drop for SharedStorageInner<T> {
    fn drop(&mut self) {
        match core::mem::replace(&mut self.backing, BackingStorage::External) {
            BackingStorage::ForeignOwner(o) => drop(o),
            BackingStorage::Vec {
                original_capacity,
                vtable,
            } => unsafe {
                // Drop the elements in our slice.
                if std::mem::needs_drop::<T>() {
                    core::ptr::drop_in_place(core::ptr::slice_from_raw_parts_mut(
                        self.ptr,
                        self.length_in_bytes / size_of::<T>(),
                    ));
                }

                // Free the buffer.
                if original_capacity > 0 {
                    (vtable.drop_buffer)(self.ptr.cast(), original_capacity);
                }
            },
            BackingStorage::External | BackingStorage::Leaked => {}
        }
    }
}

impl<T> SharedStorageInner<T> {
    pub fn from_vec(mut v: Vec<T>) -> Self {
        let length_in_bytes = v.len() * size_of::<T>();
        let original_capacity = v.capacity();
        let ptr = v.as_mut_ptr();
        core::mem::forget(v);
        Self {
            ref_count: AtomicU64::new(1),
            ptr,
            length_in_bytes,
            backing: BackingStorage::Vec {
                original_capacity,
                vtable: VecVTable::new_static::<T>(),
            },
            phantom: PhantomData,
        }
    }
}

impl<T> SharedStorage<T> {
    /// Creates an empty SharedStorage.
    pub const fn empty() -> Self {
        assert!(align_of::<T>() <= 1 << 30);
        static INNER: SharedStorageInner<()> = SharedStorageInner {
            ref_count: AtomicU64::new(1),
            ptr: core::ptr::without_provenance_mut(1 << 30), // Very overaligned for any T.
            length_in_bytes: 0,
            backing: BackingStorage::Leaked,
            phantom: PhantomData,
        };

        Self {
            inner: NonNull::new(&raw const INNER as *mut SharedStorageInner<T>).unwrap(),
            phantom: PhantomData,
        }
    }

    pub fn from_static(slice: &'static [T]) -> Self {
        // SAFETY: the slice has a static lifetime.
        unsafe { Self::from_slice_unchecked(slice) }
    }

    pub fn from_vec(v: Vec<T>) -> Self {
        Self {
            inner: NonNull::new(Box::into_raw(Box::new(SharedStorageInner::from_vec(v)))).unwrap(),
            phantom: PhantomData,
        }
    }

    /// # Safety
    ///
    /// Slice given by `std::slice::from_raw_parts(ptr, length)` should be valid until the owner is
    /// dropped. This function assumes a shared reference of this data.
    pub unsafe fn from_foreign(
        ptr: *mut T,
        length: usize,
        owner: Box<dyn Any + Send + 'static>,
    ) -> Self {
        let inner = SharedStorageInner {
            ref_count: AtomicU64::new(1),
            ptr,
            length_in_bytes: length * size_of::<T>(),
            backing: BackingStorage::ForeignOwner(owner),
            phantom: PhantomData,
        };
        Self {
            inner: NonNull::new(Box::into_raw(Box::new(inner))).unwrap(),
            phantom: PhantomData,
        }
    }

    /// # Safety
    ///
    /// Slice's lifetime should be valid for the time it used.
    pub unsafe fn from_slice_unchecked(slice: &[T]) -> Self {
        #[expect(clippy::manual_slice_size_calculation)]
        let length_in_bytes = slice.len() * size_of::<T>();
        let ptr = slice.as_ptr().cast_mut();
        let inner = SharedStorageInner {
            ref_count: AtomicU64::new(1),
            ptr,
            length_in_bytes,
            backing: BackingStorage::External,
            phantom: PhantomData,
        };
        Self {
            inner: NonNull::new(Box::into_raw(Box::new(inner))).unwrap(),
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    const fn inner(&self) -> &SharedStorageInner<T> {
        unsafe { &*self.inner.as_ptr() }
    }

    #[inline(always)]
    pub const fn as_ptr(&self) -> *const T {
        self.inner().ptr
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.inner().length_in_bytes / size_of::<T>()
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.inner().length_in_bytes == 0
    }

    /// # Safety
    /// May only be called once.
    #[cold]
    unsafe fn drop_slow(&mut self) {
        unsafe { drop(Box::from_raw(self.inner.as_ptr())) }
    }
}

impl<T> Drop for SharedStorage<T> {
    fn drop(&mut self) {
        let inner = self.inner();
        if matches!(inner.backing, BackingStorage::Leaked) {
            return;
        }

        // Ordering semantics copied from Arc<T>.
        if inner.ref_count.fetch_sub(1, Ordering::Release) == 1 {
            std::sync::atomic::fence(Ordering::Acquire);
            unsafe {
                self.drop_slow();
            }
        }
    }
}

impl<T> Clone for SharedStorage<T> {
    fn clone(&self) -> Self {
        let inner = self.inner();
        if !matches!(inner.backing, BackingStorage::Leaked) {
            // Ordering semantics copied from Arc<T>.
            inner.ref_count.fetch_add(1, Ordering::Relaxed);
        }
        Self {
            inner: self.inner,
            phantom: PhantomData,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Buffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Buffer").field(&self.as_slice()).finish()
    }
}

impl<T: PartialEq> PartialEq for Buffer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<T: Eq> Eq for Buffer<T> {}

impl<T> Clone for Buffer<T> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            ptr: self.ptr,
            length: self.length,
        }
    }
}

impl<T> Buffer<T> {
    pub const fn from_storage(storage: SharedStorage<T>) -> Self {
        let ptr = storage.as_ptr();
        let length = storage.len();
        Buffer {
            storage,
            ptr,
            length,
        }
    }

    pub fn from_static(data: &'static [T]) -> Self {
        Self::from_storage(SharedStorage::from_static(data))
    }

    pub fn from_vec(data: Vec<T>) -> Self {
        Self::from_storage(SharedStorage::from_vec(data))
    }

    #[inline]
    pub fn offset(&self) -> usize {
        unsafe {
            let ret = self.ptr.offset_from(self.storage.as_ptr()) as usize;
            debug_assert!(ret <= self.storage.len());
            ret
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: invariant of this struct `offset + length <= data.len()`.
        debug_assert!(self.offset() + self.length <= self.storage.len());
        unsafe { std::slice::from_raw_parts(self.ptr, self.length) }
    }
}

impl<T> From<Vec<T>> for Buffer<T> {
    #[inline(always)]
    fn from(value: Vec<T>) -> Self {
        Self::from_vec(value)
    }
}
impl<T, const N: usize> From<[T; N]> for Buffer<T> {
    #[inline(always)]
    fn from(value: [T; N]) -> Self {
        Self::from_vec(value.into())
    }
}

impl<V> FromIterator<V> for Buffer<V> {
    #[inline(always)]
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        Self::from_vec(FromIterator::from_iter(iter))
    }
}

impl<T> AsRef<[T]> for Buffer<T> {
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> Deref for Buffer<T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
