#[derive(Default)]
pub struct Arena(bumpalo::Bump);

impl Arena {
    pub fn new() -> Self {
        Self(bumpalo::Bump::new())
    }

    pub fn add<T>(&self, val: T) -> &mut T {
        self.0.alloc(val)
    }

    pub fn extend<T, I: IntoIterator<Item = T>>(&self, iter: I) -> &mut [T]
    where
        I::IntoIter: ExactSizeIterator,
    {
        self.0.alloc_slice_fill_iter(iter)
    }
}
