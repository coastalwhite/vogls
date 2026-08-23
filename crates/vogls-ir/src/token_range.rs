use std::ops::{BitOr, BitOrAssign};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenRange {
    pub start: usize,
    pub end: usize,
}
impl TokenRange {
    pub fn at(tr: usize) -> TokenRange {
        TokenRange {
            start: tr,
            end: tr + 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

impl BitOr for TokenRange {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            start: self.start.min(rhs.start),
            end: self.end.max(rhs.end),
        }
    }
}
impl BitOrAssign for TokenRange {
    fn bitor_assign(&mut self, rhs: Self) {
        self.start = self.start.min(rhs.start);
        self.end = self.end.max(rhs.end);
    }
}
