use std::fmt;
use std::ops;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    #[inline(always)]
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    #[inline(always)]
    pub fn with_length(start: usize, length: usize) -> Self {
        Self {
            start,
            end: start + length,
        }
    }

    #[inline(always)]
    pub fn start(&self) -> usize {
        self.start
    }

    #[inline(always)]
    pub fn end(&self) -> usize {
        self.end
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn as_range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

impl ops::BitOr for Span {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            start: usize::min(self.start, rhs.start),
            end: usize::max(self.end, rhs.end),
        }
    }
}

impl ops::BitOrAssign for Span {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.start = usize::min(self.start, rhs.start);
        self.end = usize::max(self.end, rhs.end);
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { start, end } = self;

        start.fmt(f)?;
        f.write_str("..")?;
        end.fmt(f)?;

        Ok(())
    }
}
