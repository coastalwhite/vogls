use std::fmt;
use std::ops;
use std::path::Path;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: usize,
    end: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Location {
    span: Span,
    path: Option<Rc<Path>>,
}

impl Span {
    #[inline(always)]
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    #[inline(always)]
    pub fn with_length(start: usize, length: usize) -> Self {
        Self { start, end: start + length }
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
}

impl Location {
    #[inline(always)]
    pub fn new(span: Span, path: Option<Rc<Path>>) -> Self {
        Self { span, path }
    }

    #[inline(always)]
    pub fn span(&self) -> Span {
        self.span
    }

    #[inline(always)]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
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

impl ops::BitOr for Location {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(self.path, rhs.path);

        Self {
            span: self.span | rhs.span,
            path: self.path,
        }
    }
}

impl ops::BitOrAssign for Location {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        debug_assert_eq!(self.path, rhs.path);
        self.span |= rhs.span;
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

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { span, path } = self;

        match path {
            Some(p) => p.display().fmt(f)?,
            None => f.write_str("<HEADLESS>")?,
        }
        f.write_str(":")?;
        span.fmt(f)?;

        Ok(())
    }
}
