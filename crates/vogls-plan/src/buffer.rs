use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Hash)]
pub struct Buffer<T> {
    inner: Arc<[T]>,
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
            inner: self.inner.clone(),
        }
    }
}

impl<T> Buffer<T> {
    pub fn from_vec(vec: Vec<T>) -> Self {
        Self { inner: vec.into() }
    }
    pub fn from_arc_slice(slice: Arc<[T]>) -> Self {
        Self { inner: slice }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }
}

impl<T> From<Vec<T>> for Buffer<T> {
    #[inline(always)]
    fn from(value: Vec<T>) -> Self {
        Self::from_vec(value)
    }
}
impl<T> From<Arc<[T]>> for Buffer<T> {
    #[inline(always)]
    fn from(value: Arc<[T]>) -> Self {
        Self::from_arc_slice(value)
    }
}
impl<T, const N: usize> From<[T; N]> for Buffer<T> {
    #[inline(always)]
    fn from(value: [T; N]) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

impl<V> FromIterator<V> for Buffer<V> {
    #[inline(always)]
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        Self {
            inner: FromIterator::from_iter(iter),
        }
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
