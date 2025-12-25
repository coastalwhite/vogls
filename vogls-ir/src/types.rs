use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArrayWidth(NonZeroU32);


impl ArrayWidth {
    pub fn new(v: u32) -> Self {
        Self(NonZeroU32::new(v + 1).unwrap())
    }

    pub fn get(self) -> u32 {
        self.0.get() - 1
    }
}
