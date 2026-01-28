use crate::Bits;
use crate::arithmetic::FvLogicValue;

pub struct ValueIter<'a> {
    pub(super) bits: &'a Bits,
    pub(super) start: u32,
    pub(super) end: u32,
}

impl<'a> Iterator for ValueIter<'a> {
    type Item = FvLogicValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }

        let value = self.bits.select_value(self.start);
        self.start += 1;
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = (self.end - self.start) as usize;
        (size, Some(size))
    }
}

impl<'a> DoubleEndedIterator for ValueIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start >= self.end || self.end == 0 {
            return None;
        }

        self.end -= 1;
        Some(self.bits.select_value(self.end))
    }
}
impl<'a> ExactSizeIterator for ValueIter<'a> {}
