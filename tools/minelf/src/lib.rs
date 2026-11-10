//! This is basic implementation which decodes ELF binaries.
#![allow(unused)]

pub mod data_encoding;
pub mod errors;
pub mod header;
pub mod ident;
pub mod program_header;
pub mod section_header;
pub mod string_table;
pub mod symbol_table;

use std::ops::Range;

use self::data_encoding::DataEncoding;
use self::errors::Result;
use self::header::ElfHeader;
use self::program_header::ElfProgramHeader;
use self::section_header::{ElfSectionHeader, ElfSectionType};
use self::string_table::ElfStringTable;
use self::symbol_table::ElfSymbolTableEntry;

#[derive(Debug, Clone)]
pub enum ElfFile {
    Linking(ElfLinkingFile),
    Executable(ElfExecutableFile),
}

#[derive(Debug, Clone)]
pub struct ElfExecutableFile {
    header: ElfHeader,
    program_headers: Vec<ElfProgramHeader>,
    section_headers: Vec<ElfSectionHeader>,
    symbols: Vec<ElfSymbolTableEntry>,
    section_string_table: ElfStringTable,
    string_table: ElfStringTable,
}

#[derive(Debug, Clone)]
pub struct ElfLinkingFile {
    header: ElfHeader,
    program_headers: Vec<ElfProgramHeader>,
    section_headers: Vec<ElfSectionHeader>,
    symbols: Vec<ElfSymbolTableEntry>,
    section_string_table: ElfStringTable,
    string_table: ElfStringTable,
    // ... TODO
}

fn read_program_headers(
    buffer: &[u8],
    data_encoding: DataEncoding,
    table_offset: u32,
    num_entries: u16,
    entry_size: u16,
) -> Result<Vec<ElfProgramHeader>> {
    assert!(std::mem::size_of::<usize>() >= 4);
    let table_offset = table_offset as usize;
    let num_entries = num_entries as usize;
    let entry_size = entry_size as usize;

    let mut program_headers = Vec::with_capacity(num_entries);

    if table_offset == 0 || num_entries == 0 {
        return Ok(program_headers);
    }

    for entry_offset in
        (table_offset..(table_offset + (num_entries * entry_size))).step_by(entry_size)
    {
        program_headers.push(ElfProgramHeader::read_from_bytes(
            &buffer[entry_offset..],
            data_encoding,
        )?);
    }

    Ok(program_headers)
}

fn read_section_headers(
    buffer: &[u8],
    data_encoding: DataEncoding,
    table_offset: u32,
    num_entries: u16,
    entry_size: u16,
) -> Result<Vec<ElfSectionHeader>> {
    assert!(std::mem::size_of::<usize>() >= 4);
    let table_offset = table_offset as usize;
    let num_entries = num_entries as usize;
    let entry_size = entry_size as usize;

    let mut section_headers = Vec::with_capacity(num_entries);

    if table_offset == 0 || num_entries == 0 {
        return Ok(section_headers);
    }

    for entry_offset in
        (table_offset..(table_offset + (num_entries * entry_size))).step_by(entry_size)
    {
        section_headers.push(ElfSectionHeader::read_from_bytes(
            &buffer[entry_offset..],
            data_encoding,
        )?);
    }

    Ok(section_headers)
}

fn read_symbols(
    bytes: &[u8],
    data_encoding: DataEncoding,
    section_headers: &Vec<ElfSectionHeader>,
) -> Result<Vec<ElfSymbolTableEntry>> {
    Ok(section_headers
        .iter()
        .filter(|sect| sect.sh_type == ElfSectionType::Symtab)
        .map(|sect| {
            assert!(sect.sh_entsize != 0);

            assert!(std::mem::size_of::<usize>() >= 4);
            let sh_size = sect.sh_size as usize;
            let sh_offset = sect.sh_offset as usize;
            let sh_entsize = sect.sh_entsize as usize;

            let num_entries = sh_size / sh_entsize;
            let mut symbols = Vec::with_capacity(num_entries);
            for i in 0..num_entries {
                let glob_offset = sh_offset + (i * sh_entsize);
                symbols.push(ElfSymbolTableEntry::read_from_bytes(
                    &bytes[glob_offset..],
                    data_encoding,
                )?);
            }

            Ok(symbols)
        })
        .collect::<Result<Vec<Vec<ElfSymbolTableEntry>>>>()?
        .into_iter()
        .flatten()
        .collect())
}

fn read_string_table<'a>(
    bytes: &'_ [u8],
    section_headers: &'a Vec<ElfSectionHeader>,
) -> Result<Vec<(&'a ElfSectionHeader, ElfStringTable)>> {
    section_headers
        .iter()
        .filter(|sect| sect.sh_type == ElfSectionType::Strtab)
        .map(|sect| Ok((sect, read_one_string_table(bytes, sect)?)))
        .collect()
}

/// The string table `sect` holds, which must lie within `bytes`.
fn read_one_string_table(bytes: &[u8], sect: &ElfSectionHeader) -> Result<ElfStringTable> {
    assert!(std::mem::size_of::<usize>() >= 4);
    let sh_size = sect.sh_size as usize;
    let sh_offset = sect.sh_offset as usize;
    let end = sh_offset.saturating_add(sh_size);
    let content = bytes
        .get(sh_offset..end)
        .ok_or(errors::ElfError::UnsufficientBytes {
            available_bytes: bytes.len(),
            needed_bytes: end,
        })?;
    Ok(ElfStringTable::new(content, sh_size))
}

impl ElfFile {
    pub fn read_from_bytes(bytes: &[u8]) -> Result<Self> {
        let header = ElfHeader::read_from_bytes(bytes)?;

        let data_encoding = header.e_ident.data_encoding;

        let phoff = header.e_phoff;
        let phnum = header.e_phnum;
        let phentsize = header.e_phentsize;

        let shoff = header.e_shoff;
        let shnum = header.e_shnum;
        let shentsize = header.e_shentsize;

        if header.e_type == 2 {
            let program_headers =
                read_program_headers(bytes, data_encoding, phoff, phnum, phentsize)?;
            let section_headers =
                read_section_headers(bytes, data_encoding, shoff, shnum, shentsize)?;
            let symbols = read_symbols(bytes, data_encoding, &section_headers)?;
            let string_tables = read_string_table(bytes, &section_headers)?;

            let mut shstr_table = None;

            for (_, str_table) in string_tables.iter() {
                if str_table.contains_section_headers() {
                    shstr_table = Some(str_table.clone());
                    break;
                }
            }
            let shstr_table = shstr_table.expect("No section header table found");
            let strtab = section_headers
                .iter()
                .find(|sect| shstr_table.get_from(sect.sh_name as usize) == b".strtab")
                .map(|sect| read_one_string_table(bytes, sect))
                .transpose()?
                .unwrap_or_else(ElfStringTable::empty);

            Ok(ElfFile::Executable(ElfExecutableFile {
                header,
                program_headers,
                section_headers,
                symbols,
                string_table: strtab,
                section_string_table: shstr_table,
            }))
        } else {
            unimplemented!("Non executable files are not yet implemented");
        }
    }

    pub fn section_headers(&self) -> &Vec<ElfSectionHeader> {
        match self {
            ElfFile::Executable(ElfExecutableFile {
                section_headers, ..
            }) => section_headers,
            ElfFile::Linking(ElfLinkingFile {
                section_headers, ..
            }) => section_headers,
        }
    }

    pub fn header(&self) -> &ElfHeader {
        match self {
            ElfFile::Executable(ElfExecutableFile { header, .. }) => header,
            ElfFile::Linking(ElfLinkingFile { header, .. }) => header,
        }
    }

    pub fn program_headers(&self) -> &Vec<ElfProgramHeader> {
        match self {
            ElfFile::Executable(ElfExecutableFile {
                program_headers, ..
            }) => program_headers,
            ElfFile::Linking(ElfLinkingFile {
                program_headers, ..
            }) => program_headers,
        }
    }

    /// The section header of the section called `name`.
    pub fn get_section(&self, name: &[u8]) -> Option<&ElfSectionHeader> {
        self.section_headers()
            .iter()
            .find(|sect| self.get_section_name(sect) == Some(name))
    }

    /// The range of file bytes holding the contents of the section `name`.
    ///
    /// This is [`None`] for a section without any bytes in the file: `.bss` and
    /// friends are [`ElfSectionType::Nobits`], so they take up memory at run
    /// time but no space in the file and their `sh_offset` does not point at a
    /// readable range.
    pub fn get_section_range(&self, name: &[u8]) -> Option<Range<usize>> {
        let sect = self.get_section(name)?;

        if sect.sh_type == ElfSectionType::Nobits {
            return None;
        }

        assert!(std::mem::size_of::<usize>() >= 4);
        let offset = sect.sh_offset as usize;
        let size = sect.sh_size as usize;

        Some(offset..(offset + size))
    }

    pub fn get_text_range(&self) -> Option<Range<usize>> {
        self.get_section_range(b".text")
    }

    pub fn get_text(&self) -> Option<&ElfSectionHeader> {
        self.get_section(b".text")
    }

    pub fn symbols(&self) -> &Vec<ElfSymbolTableEntry> {
        match self {
            ElfFile::Executable(ElfExecutableFile { symbols, .. }) => symbols,
            ElfFile::Linking(ElfLinkingFile { symbols, .. }) => symbols,
        }
    }

    pub fn string_table(&self) -> &ElfStringTable {
        match self {
            ElfFile::Executable(ElfExecutableFile { string_table, .. }) => string_table,
            ElfFile::Linking(ElfLinkingFile { string_table, .. }) => string_table,
        }
    }

    pub fn section_string_table(&self) -> &ElfStringTable {
        match self {
            ElfFile::Executable(ElfExecutableFile {
                section_string_table,
                ..
            }) => section_string_table,
            ElfFile::Linking(ElfLinkingFile {
                section_string_table,
                ..
            }) => section_string_table,
        }
    }

    pub fn get_section_name(&self, section: &ElfSectionHeader) -> Option<&[u8]> {
        if section.sh_name == 0 {
            return None;
        }

        assert!(std::mem::size_of::<usize>() >= 4);
        Some(
            self.section_string_table()
                .get_from(section.sh_name as usize),
        )
    }

    pub fn get_symbol_string(&self, symbol: &ElfSymbolTableEntry) -> Option<&[u8]> {
        if symbol.st_name == 0 {
            return None;
        }

        assert!(std::mem::size_of::<usize>() >= 4);
        Some(self.string_table().get_from(symbol.st_name as usize))
    }
}
