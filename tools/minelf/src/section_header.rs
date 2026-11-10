use super::data_encoding::DataEncoding;
use super::errors::{ElfError::*, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElfSectionType {
    Null,
    Progbits,
    Symtab,
    Strtab,
    Rela,
    Hash,
    Dynamic,
    Note,
    Nobits,
    Rel,
    Shlib,
    Dynsym,
    Loproc,
    Hiproc,
    Louser,
    Hiuser,
    Other(u32),
}

#[derive(Debug, Clone)]
pub struct ElfSectionFlags {
    pub write: bool,
    pub alloc: bool,
    pub execinst: bool,
    pub maskproc: bool,
}

#[derive(Debug, Clone)]
pub struct ElfSectionHeader {
    pub sh_name: u32,
    pub sh_type: ElfSectionType,
    pub sh_flags: ElfSectionFlags,
    pub sh_addr: u32,
    pub sh_offset: u32,
    pub sh_size: u32,
    pub sh_link: u32,
    pub sh_info: u32,
    pub sh_addralign: u32,
    pub sh_entsize: u32,
}

impl ElfSectionType {
    fn to_u32(&self) -> u32 {
        use ElfSectionType::*;

        match self {
            Null => 0,
            Progbits => 1,
            Symtab => 2,
            Strtab => 3,
            Rela => 4,
            Hash => 5,
            Dynamic => 6,
            Note => 7,
            Nobits => 8,
            Rel => 9,
            Shlib => 10,
            Dynsym => 11,
            Loproc => 0x70000000,
            Hiproc => 0x7fffffff,
            Louser => 0x80000000,
            Hiuser => 0xffffffff,
            Other(v) => *v,
        }
    }

    fn from_u32(word: u32) -> Result<Self> {
        use ElfSectionType::*;

        Ok(match word {
            0 => Null,
            1 => Progbits,
            2 => Symtab,
            3 => Strtab,
            4 => Rela,
            5 => Hash,
            6 => Dynamic,
            7 => Note,
            8 => Nobits,
            9 => Rel,
            10 => Shlib,
            11 => Dynsym,
            0x70000000 => Loproc,
            0x7fffffff => Hiproc,
            0x80000000 => Louser,
            0xffffffff => Hiuser,
            v => Other(v),
        })
    }
}

impl ElfSectionFlags {
    fn from_u32(word: u32) -> Self {
        Self {
            write: word & 0x1 == 0x1,
            alloc: word & 0x2 == 0x2,
            execinst: word & 0x4 == 0x4,
            maskproc: word & 0xf000_0000 == 0xf000_0000,
        }
    }
    fn to_u32(&self) -> u32 {
        let mut word = 0x0;

        if self.write {
            word |= 0x1;
        }
        if self.alloc {
            word |= 0x2;
        }
        if self.execinst {
            word |= 0x4;
        }
        if self.maskproc {
            word |= 0xf000_0000;
        }

        word
    }
}

impl ElfSectionHeader {
    pub fn read_from_bytes(bytes: &[u8], data_encoding: DataEncoding) -> Result<Self> {
        if bytes.len() < 40 {
            return Err(UnsufficientBytes {
                available_bytes: bytes.len(),
                needed_bytes: 40,
            });
        }

        let denc = data_encoding;

        let sh_name = denc.read_u32(bytes)?;
        let sh_type = denc.read_u32(&bytes[4..])?;
        let sh_type = ElfSectionType::from_u32(sh_type)?;
        let sh_flags = denc.read_u32(&bytes[8..])?;
        let sh_flags = ElfSectionFlags::from_u32(sh_flags);
        let sh_addr = denc.read_u32(&bytes[12..])?;
        let sh_offset = denc.read_u32(&bytes[16..])?;
        let sh_size = denc.read_u32(&bytes[20..])?;
        let sh_link = denc.read_u32(&bytes[24..])?;
        let sh_info = denc.read_u32(&bytes[28..])?;
        let sh_addralign = denc.read_u32(&bytes[32..])?;
        let sh_entsize = denc.read_u32(&bytes[36..])?;

        Ok(Self {
            sh_name,
            sh_type,
            sh_flags,
            sh_addr,
            sh_offset,
            sh_size,
            sh_link,
            sh_info,
            sh_addralign,
            sh_entsize,
        })
    }
}
