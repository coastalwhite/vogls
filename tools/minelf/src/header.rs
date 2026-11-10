use super::errors::{ElfError::*, Result};
use super::ident::ElfIdent;

#[derive(Debug, Clone)]
pub struct ElfHeader {
    pub e_ident: ElfIdent,
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u32,
    pub e_phoff: u32,
    pub e_shoff: u32,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

impl ElfHeader {
    pub fn read_from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 52 {
            return Err(UnsufficientBytes {
                available_bytes: bytes.len(),
                needed_bytes: 52,
            });
        }

        let e_ident = ElfIdent::read_from_bytes(bytes)?;
        let denc = e_ident.data_encoding;

        let e_type = denc.read_u16(&bytes[16..])?;
        let e_machine = denc.read_u16(&bytes[18..])?;
        let e_version = denc.read_u32(&bytes[20..])?;
        let e_entry = denc.read_u32(&bytes[24..])?;
        let e_phoff = denc.read_u32(&bytes[28..])?;
        let e_shoff = denc.read_u32(&bytes[32..])?;
        let e_flags = denc.read_u32(&bytes[36..])?;
        let e_ehsize = denc.read_u16(&bytes[40..])?;
        let e_phentsize = denc.read_u16(&bytes[42..])?;
        let e_phnum = denc.read_u16(&bytes[44..])?;
        let e_shentsize = denc.read_u16(&bytes[46..])?;
        let e_shnum = denc.read_u16(&bytes[48..])?;
        let e_shstrndx = denc.read_u16(&bytes[50..])?;

        Ok(Self {
            e_ident,
            e_type,
            e_machine,
            e_version,
            e_entry,
            e_phoff,
            e_shoff,
            e_flags,
            e_ehsize,
            e_phentsize,
            e_phnum,
            e_shentsize,
            e_shnum,
            e_shstrndx,
        })
    }
}
