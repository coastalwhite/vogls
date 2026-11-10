use super::data_encoding::DataEncoding;
use super::errors::{ElfError::*, Result};

#[derive(Debug, Clone)]
pub struct ElfProgramHeader {
    pub p_type: u32,
    pub p_offset: u32,
    pub p_vaddr: u32,
    pub p_paddr: u32,
    pub p_filesz: u32,
    pub p_memsz: u32,
    pub p_flags: u32,
    pub p_align: u32,
}

impl ElfProgramHeader {
    pub fn read_from_bytes(bytes: &[u8], data_encoding: DataEncoding) -> Result<Self> {
        if bytes.len() < 32 {
            return Err(UnsufficientBytes {
                available_bytes: bytes.len(),
                needed_bytes: 32,
            });
        }

        let denc = data_encoding;

        let p_type = denc.read_u32(bytes)?;
        let p_offset = denc.read_u32(&bytes[4..])?;
        let p_vaddr = denc.read_u32(&bytes[8..])?;
        let p_paddr = denc.read_u32(&bytes[12..])?;
        let p_filesz = denc.read_u32(&bytes[16..])?;
        let p_memsz = denc.read_u32(&bytes[20..])?;
        let p_flags = denc.read_u32(&bytes[24..])?;
        let p_align = denc.read_u32(&bytes[28..])?;

        Ok(Self {
            p_type,
            p_offset,
            p_vaddr,
            p_paddr,
            p_filesz,
            p_memsz,
            p_flags,
            p_align,
        })
    }
}
