use std::fmt;

pub type SdfResult<T> = Result<T, Box<SdfError>>;

#[derive(Debug)]
pub struct SdfError {
    pub line: u64,
    pub msg: String,
}
impl fmt::Display for SdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}
impl std::error::Error for SdfError {}
