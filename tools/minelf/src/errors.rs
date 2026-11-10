use std::fmt::Display;

pub type Result<T> = std::result::Result<T, ElfError>;

#[derive(Debug, Clone)]
pub enum ElfError {
    UnsufficientBytes {
        available_bytes: usize,
        needed_bytes: usize,
    },
    UnexpectedByte {
        found_byte: u8,
        expected_byte: u8,
    },
    InvalidDataEncoding {
        found: u8,
    }
}

impl Display for ElfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ElfError::*;

        match self {
            UnsufficientBytes {
                available_bytes,
                needed_bytes,
            } => write!(
                f,
                "Expected {} bytes but got {} bytes",
                needed_bytes, available_bytes
            ),
            UnexpectedByte {
                found_byte,
                expected_byte,
            } => write!(
                f,
                "Expected '{:x}' but found '{:x}'",
                expected_byte, found_byte
            ),
            InvalidDataEncoding { found } => write!(f, "Data encoding '0x{:x}' is unknown", found),
        }
    }
}
