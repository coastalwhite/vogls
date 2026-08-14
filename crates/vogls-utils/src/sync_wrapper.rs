/// Utility wrapper to run types which are Send + !Sync, into types that are both Send + Sync.
///
/// This is done, by prohibiting access to the inner field through shared references.
pub struct SyncWrapper<T: ?Sized>(T);

// SAFETY:
//
// SyncWrapper has no way to turn a `&SyncWrapper<T>` into a `&T` or `T`. Therefore, a
// `&SyncWrapper` which is shared across threads cannot actually do anything to the inner value.
unsafe impl<T: ?Sized + Send> Sync for SyncWrapper<T> {}

impl<T> SyncWrapper<T> {
    pub const fn new(t: T) -> Self {
        Self(t)
    }
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: ?Sized> SyncWrapper<T> {
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
