use std::fmt;

pub type AssembleResult<T> = Result<T, Box<AssembleError>>;

#[derive(Debug, Clone)]
pub struct AssembleError {
    pub(crate) reason: std::borrow::Cow<'static, str>,
    pub(crate) line: usize,
    pub(crate) line_offset: usize,
    pub(crate) _offset: usize,
}

impl fmt::Display for AssembleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}]: {}", self.line, self.line_offset, self.reason)
    }
}
impl std::error::Error for AssembleError {}
