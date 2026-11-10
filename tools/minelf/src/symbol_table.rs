use super::data_encoding::DataEncoding;
use super::errors::{ElfError::UnsufficientBytes, Result};

#[derive(Debug, Clone, Copy)]
pub enum ElfSymbolBinding {
    Local,
    Global,
    Weak,
    Loproc,
    Hiproc,
    Other(u8),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElfSymbolType {
    Notype,
    Object,
    Func,
    Section,
    File,
    Loproc,
    Hiproc,
    Other(u8),
}

#[derive(Debug, Clone)]
pub struct ElfSymbolInfo {
    pub s_binding: ElfSymbolBinding,
    pub s_type: ElfSymbolType,
    pub s_info: u8,
}

#[derive(Debug, Clone)]
pub struct ElfSymbolTableEntry {
    pub st_name: u32,
    pub st_value: u32,
    pub st_size: u32,
    pub st_info: ElfSymbolInfo,
    pub st_other: u8,
    pub st_shndx: u16,
}

impl From<u8> for ElfSymbolBinding {
    fn from(word: u8) -> Self {
        use ElfSymbolBinding::*;

        match word {
            0 => Local,
            1 => Global,
            2 => Weak,
            13 => Loproc,
            15 => Hiproc,
            v => Other(v),
        }
    }
}

impl From<u8> for ElfSymbolType {
    fn from(word: u8) -> Self {
        use ElfSymbolType::*;

        match word {
            0 => Notype,
            1 => Object,
            2 => Func,
            3 => Section,
            4 => File,
            13 => Loproc,
            15 => Hiproc,
            v => Other(v),
        }
    }
}

impl From<u8> for ElfSymbolInfo {
    fn from(word: u8) -> Self {
        Self {
            s_binding: (word >> 4).into(),
            s_type: (word & 0xF).into(),
            s_info: word,
        }
    }
}

impl ElfSymbolTableEntry {
    pub fn read_from_bytes(bytes: &[u8], data_encoding: DataEncoding) -> Result<Self> {
        if bytes.len() < 8 {
            return Err(UnsufficientBytes {
                available_bytes: bytes.len(),
                needed_bytes: 8,
            });
        }

        let denc = data_encoding;

        let st_name = denc.read_u32(bytes)?;
        let st_value = denc.read_u32(&bytes[4..])?;
        let st_size = denc.read_u32(&bytes[8..])?;
        let st_info = bytes[12].into();
        let st_other = bytes[13];
        let st_shndx = denc.read_u16(&bytes[14..])?;

        Ok(Self {
            st_name,
            st_value,
            st_size,
            st_info,
            st_other,
            st_shndx,
        })
    }
}
