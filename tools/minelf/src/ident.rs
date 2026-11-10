use super::data_encoding::DataEncoding;
use super::errors::{ElfError::*, Result};

#[derive(Debug, Clone)]
pub struct ElfIdent {
    pub file_class: u8,
    pub data_encoding: DataEncoding,
    pub file_version: u8,
}

impl ElfIdent {
    pub fn read_from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes[0] != 0x7f {
            return Err(UnexpectedByte {
                expected_byte: 0x7f,
                found_byte: bytes[0],
            });
        }
        if bytes[1] != b'E' {
            return Err(UnexpectedByte {
                expected_byte: b'E',
                found_byte: bytes[1],
            });
        }
        if bytes[2] != b'L' {
            return Err(UnexpectedByte {
                expected_byte: b'L',
                found_byte: bytes[2],
            });
        }
        if bytes[3] != b'F' {
            return Err(UnexpectedByte {
                expected_byte: b'F',
                found_byte: bytes[3],
            });
        }

        let file_class = bytes[4];
        let data_encoding = DataEncoding::from_byte(bytes[5])?;
        let file_version = bytes[6];

        for i in 7..16 {
            if bytes[i] != 0 {
                return Err(UnexpectedByte {
                    found_byte: bytes[i],
                    expected_byte: 0,
                });
            }
        }

        Ok(Self {
            file_class,
            data_encoding,
            file_version,
        })
    }
}
