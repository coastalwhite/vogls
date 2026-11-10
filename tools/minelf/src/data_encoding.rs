use super::errors::{ElfError::*, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataEncoding {
    LittleEndian,
    BigEndian,
}

impl DataEncoding {
    pub fn from_byte(byte: u8) -> Result<DataEncoding> {
        match byte {
            1 => Ok(DataEncoding::LittleEndian),
            2 => Ok(DataEncoding::BigEndian),
            _ => Err(InvalidDataEncoding { found: byte }),
        }
    }

    pub fn read_u16(self, buffer: &[u8]) -> Result<u16> {
        if buffer.len() < 2 {
            return Err(UnsufficientBytes {
                available_bytes: buffer.len(),
                needed_bytes: 2,
            });
        }

        let bytes = [buffer[0], buffer[1]];

        Ok(match self {
            DataEncoding::LittleEndian => u16::from_le_bytes(bytes),
            DataEncoding::BigEndian => u16::from_be_bytes(bytes),
        })
    }

    pub fn read_u32(self, buffer: &[u8]) -> Result<u32> {
        if buffer.len() < 4 {
            return Err(UnsufficientBytes {
                available_bytes: buffer.len(),
                needed_bytes: 4,
            });
        }

        let bytes = [buffer[0], buffer[1], buffer[2], buffer[3]];

        Ok(match self {
            DataEncoding::LittleEndian => u32::from_le_bytes(bytes),
            DataEncoding::BigEndian => u32::from_be_bytes(bytes),
        })
    }
}
