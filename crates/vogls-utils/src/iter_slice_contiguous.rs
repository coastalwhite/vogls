pub struct IterSliceContinguous<'a, T, K, F: FnMut(&T) -> &K> {
    slice: &'a [T],
    to_key: F,
}

impl<'a, T, K, F: FnMut(&T) -> &K> IterSliceContinguous<'a, T, K, F> {
    pub fn new(slice: &'a [T], to_key: F) -> Self {
        Self { slice, to_key }
    }
}

impl<'a, T, K, F: FnMut(&T) -> &K> Iterator for IterSliceContinguous<'a, T, K, F>
where
    K: PartialEq,
{
    type Item = &'a [T];
    fn next(&mut self) -> Option<Self::Item> {
        let key = (self.to_key)(self.slice.first()?);
        Some(
            match self.slice[1..].iter().position(|v| (self.to_key)(v) != key) {
                None => std::mem::take(&mut self.slice),
                Some(next) => {
                    let current;
                    (current, self.slice) = self.slice.split_at(next + 1);
                    current
                }
            },
        )
    }
}
