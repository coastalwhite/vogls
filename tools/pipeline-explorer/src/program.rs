//! Turning a program into the sections a core loads.
//!
//! A program arrives either as RISC-V assembly, which [`trva`] assembles, or as
//! a linked ELF executable, from which the `.text`, `.data`, `.rodata` and
//! `.bss` sections are extracted. Either way the result is a set of
//! [`ProgramSections`], each of which goes at the start of its range in the
//! [`MemoryMap`] of the core.
//!
//! The memories of the testbenches are tiny (a kilobyte of code and about four
//! kilobytes of data), so an ELF built for a real board does not fit and would
//! otherwise be loaded as a silently truncated or misplaced program. Every
//! section is therefore checked against the map.

use std::fmt;
use std::ops::Range;

use minelf::ElfFile;
use minelf::data_encoding::DataEncoding;
use minelf::errors::ElfError;
use minelf::header::ElfHeader;
use trva::error::{AssembleError, LayoutError};
use trva::isa::Isa;
use trva::{Assembler, SectionPositions};

/// `EM_RISCV`, the `e_machine` of a RISC-V program.
const EM_RISCV: u16 = 243;
/// `ELFCLASS32`, the `e_ident[EI_CLASS]` of a 32-bit ELF file.
const ELFCLASS32: u8 = 1;
/// `ET_EXEC`, the `e_type` of an executable (fully linked) ELF file.
const ET_EXEC: u16 = 2;

/// Start address and maximum allocated space for each program section for a
/// core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryMap {
    /// Address of the instruction fetched first after reset.
    pub boot: u32,
    pub text: Range<u32>,
    pub data: Range<u32>,
    pub rodata: Range<u32>,
    pub bss: Range<u32>,
}

impl MemoryMap {
    /// Where the assembler places each section: at the start of its range.
    ///
    /// Each range is fixed by the core, so these are absolute rather than relative to one
    /// another.
    pub fn to_positions(&self) -> SectionPositions {
        SectionPositions::absolute(
            self.text.start,
            self.data.start,
            self.rodata.start,
            self.bss.start,
        )
    }

    /// The name and range of each section, in the order of
    /// [`ProgramSections::lens`].
    pub fn sections(&self) -> [(&'static str, Range<u32>); 4] {
        [
            (".text", self.text.clone()),
            (".data", self.data.clone()),
            (".rodata", self.rodata.clone()),
            (".bss", self.bss.clone()),
        ]
    }
}

/// The contents of each section of a program, each to be placed at the start
/// of its range in the [`MemoryMap`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgramSections {
    pub text: Box<[u8]>,
    pub data: Box<[u8]>,
    pub rodata: Box<[u8]>,
    /// Size of `.bss`. The section carries no bytes of its own: it is the
    /// zero-initialised part of memory, and only its size matters.
    pub bss: usize,
}

impl ProgramSections {
    /// The size of each section, in the order of [`MemoryMap::sections`].
    pub fn lens(&self) -> [usize; 4] {
        [
            self.text.len(),
            self.data.len(),
            self.rodata.len(),
            self.bss,
        ]
    }

    /// Checks that every section fits in its range of `map`.
    fn check_fits(&self, map: &MemoryMap) -> Result<(), LoadError> {
        for ((section, range), len) in map.sections().into_iter().zip(self.lens()) {
            let max = range.len();
            if len > max {
                return Err(LoadError::TooLarge { section, len, max });
            }
        }
        Ok(())
    }
}

/// A program, as the user hands it over.
#[derive(Debug, Clone)]
pub enum Program {
    /// A linked ELF executable.
    Elf(Box<[u8]>),
    /// RISC-V assembly, assembled by [`trva`].
    Assembly(Box<str>),
}

impl Program {
    /// Converts the program to the sections `map` places.
    ///
    /// Assembly is assembled at the start of each range of `map`. An ELF must
    /// already be linked that way, and start executing at [`MemoryMap::boot`].
    /// Either way, every section has to fit in its range.
    pub fn to_sections(&self, isa: Isa, map: &MemoryMap) -> Result<ProgramSections, LoadError> {
        let sections = match self {
            Program::Assembly(source) => {
                let assembled = Assembler::new(isa, map.to_positions())
                    .with_source(source)?
                    .assemble()?;
                ProgramSections {
                    text: assembled.text,
                    data: assembled.data,
                    rodata: assembled.rodata,
                    bss: assembled.bss,
                }
            }
            Program::Elf(bytes) => sections_from_elf(bytes, map)?,
        };
        sections.check_fits(map)?;
        Ok(sections)
    }
}

#[derive(Debug)]
pub enum LoadError {
    Assemble(Box<AssembleError>),
    /// The sections could not be laid out in the address space.
    Layout(LayoutError),
    /// The ELF file could not be decoded.
    Elf(ElfError),
    /// The file is not a 32-bit little-endian RISC-V executable.
    NotRv32Exec {
        class: u8,
        encoding: DataEncoding,
        machine: u16,
        kind: u16,
    },
    /// The file has no `.text`, or it is empty.
    NoText,
    /// A section extends past the end of the file.
    Truncated {
        section: &'static str,
        end: usize,
        file_len: usize,
    },
    /// Execution does not start where the core fetches its first instruction.
    WrongEntry {
        entry: u32,
        boot: u32,
    },
    /// A section is not linked at the start of its range in the memory map.
    Misplaced {
        section: &'static str,
        addr: u32,
        expected: u32,
    },
    /// A section is larger than its range in the memory map.
    TooLarge {
        section: &'static str,
        len: usize,
        max: usize,
    },
    /// A section that takes up memory at run time but is not one of the four
    /// the loader knows how to place.
    UnknownSection {
        name: String,
        addr: u32,
        len: u32,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Assemble(err) => write!(f, "{err}"),
            LoadError::Layout(err) => write!(f, "{err}"),
            LoadError::Elf(err) => write!(f, "malformed ELF: {err}"),
            LoadError::NotRv32Exec {
                class,
                encoding,
                machine,
                kind,
            } => write!(
                f,
                "not a 32-bit little-endian RISC-V executable \
                 (class {class}, {encoding:?}, machine {machine}, type {kind})",
            ),
            LoadError::NoText => write!(f, "the ELF file holds no instructions in '.text'"),
            LoadError::Truncated {
                section,
                end,
                file_len,
            } => write!(
                f,
                "'{section}' ends at file offset {end} but the file is only {file_len} bytes",
            ),
            LoadError::WrongEntry { entry, boot } => write!(
                f,
                "entry point is 0x{entry:08x} but the core starts executing at 0x{boot:08x}",
            ),
            LoadError::Misplaced {
                section,
                addr,
                expected,
            } => write!(
                f,
                "'{section}' is linked at 0x{addr:08x} but the core expects it at 0x{expected:08x}",
            ),
            LoadError::TooLarge { section, len, max } => write!(
                f,
                "'{section}' is {len} bytes but the core has room for only {max}",
            ),
            LoadError::UnknownSection { name, addr, len } => write!(
                f,
                "section '{name}' ({len} bytes at 0x{addr:08x}) takes up memory but is not one of \
                 '.text', '.data', '.rodata' or '.bss'; merge it into one of those in the linker \
                 script",
            ),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<Box<AssembleError>> for LoadError {
    fn from(err: Box<AssembleError>) -> Self {
        Self::Assemble(err)
    }
}
impl From<LayoutError> for LoadError {
    fn from(err: LayoutError) -> Self {
        Self::Layout(err)
    }
}
impl From<ElfError> for LoadError {
    fn from(err: ElfError) -> Self {
        Self::Elf(err)
    }
}

/// Extracts the four sections of a linked ELF executable, checking that each
/// is linked where `map` places it.
fn sections_from_elf(bytes: &[u8], map: &MemoryMap) -> Result<ProgramSections, LoadError> {
    let elf = parse_elf(bytes)?;

    // Reset drives the program counter to the reset vector of the core, so
    // execution starts there whatever the ELF nominates as its entry point.
    let entry = elf.header().e_entry;
    if entry != map.boot {
        return Err(LoadError::WrongEntry {
            entry,
            boot: map.boot,
        });
    }

    // Anything else that takes up memory at run time would be dropped
    // silently, so refuse the file instead of running a program with holes in
    // it.
    const KNOWN: [&[u8]; 4] = [b".text", b".data", b".rodata", b".bss"];
    for sect in elf.section_headers() {
        let Some(name) = elf.get_section_name(sect) else {
            continue;
        };
        if !sect.sh_flags.alloc || sect.sh_size == 0 || KNOWN.contains(&name) {
            continue;
        }

        return Err(LoadError::UnknownSection {
            name: String::from_utf8_lossy(name).into_owned(),
            addr: sect.sh_addr,
            len: sect.sh_size,
        });
    }

    for (section, range) in map.sections() {
        check_placement(&elf, section, &range)?;
    }

    let sections = ProgramSections {
        text: read_section(&elf, bytes, ".text")?,
        data: read_section(&elf, bytes, ".data")?,
        rodata: read_section(&elf, bytes, ".rodata")?,
        bss: elf
            .get_section(b".bss")
            .map_or(0, |sect| sect.sh_size as usize),
    };
    if sections.text.is_empty() {
        return Err(LoadError::NoText);
    }
    Ok(sections)
}

/// Decodes a linked ELF executable.
///
/// The header is checked first: [`ElfFile::read_from_bytes`] panics on
/// anything that is not an executable, and a 64-bit or big-endian file would
/// otherwise be misread field by field.
pub fn parse_elf(bytes: &[u8]) -> Result<ElfFile, LoadError> {
    let header = ElfHeader::read_from_bytes(bytes)?;
    if header.e_ident.file_class != ELFCLASS32
        || header.e_ident.data_encoding != DataEncoding::LittleEndian
        || header.e_machine != EM_RISCV
        || header.e_type != ET_EXEC
    {
        return Err(LoadError::NotRv32Exec {
            class: header.e_ident.file_class,
            encoding: header.e_ident.data_encoding,
            machine: header.e_machine,
            kind: header.e_type,
        });
    }

    Ok(ElfFile::read_from_bytes(bytes)?)
}

/// Checks that `section`, if the file has it and it is not empty, is linked at
/// the start of `range`.
fn check_placement(
    elf: &ElfFile,
    section: &'static str,
    range: &Range<u32>,
) -> Result<(), LoadError> {
    match elf.get_section(section.as_bytes()) {
        Some(sect) if sect.sh_size != 0 && sect.sh_addr != range.start => {
            Err(LoadError::Misplaced {
                section,
                addr: sect.sh_addr,
                expected: range.start,
            })
        }
        _ => Ok(()),
    }
}

/// Reads the contents of one section, which is empty when the file has no such
/// section.
fn read_section(elf: &ElfFile, bytes: &[u8], name: &'static str) -> Result<Box<[u8]>, LoadError> {
    // `.bss` and friends hold no bytes in the file; `get_section_range` is
    // `None` for those.
    let Some(range) = elf.get_section_range(name.as_bytes()) else {
        return Ok(Box::default());
    };
    if range.end > bytes.len() {
        return Err(LoadError::Truncated {
            section: name,
            end: range.end,
            file_len: bytes.len(),
        });
    }
    Ok(bytes[range].into())
}

/// Disassembles `text` into one string per instruction, in program order.
///
/// The pipeline view numbers its slots by dividing the program counter by four,
/// so it only lines up with this list for a program of 32-bit instructions.
pub fn disassemble(text: &[u8]) -> Vec<String> {
    let mut instructions = Vec::new();
    let mut islice = text;
    while !islice.is_empty() {
        // A trailing partial instruction reads past the end of the slice.
        let Ok(i) = trva::encoding::Instruction::decode(&mut islice) else {
            instructions.push("unknown".into());
            break;
        };
        instructions.push(match i {
            None => "unknown".into(),
            Some(i) => i.to_string(),
        });
    }
    instructions
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHT_PROGBITS: u32 = 1;
    const SHT_STRTAB: u32 = 3;
    const SHT_NOBITS: u32 = 8;

    const SHF_WRITE: u32 = 0x1;
    const SHF_ALLOC: u32 = 0x2;
    const SHF_EXECINSTR: u32 = 0x4;

    /// A core shaped like the testbenches.
    const TEST_MAP: MemoryMap = MemoryMap {
        boot: 0,
        text: 0..0x300,
        rodata: 0x300..0x400,
        data: 0x400..0x800,
        bss: 0x800..0x13a4,
    };

    struct Sect {
        name: &'static str,
        kind: u32,
        flags: u32,
        addr: u32,
        /// Bytes in the file; empty for a section that only takes up memory.
        bytes: Vec<u8>,
        /// `sh_size`, which for [`SHT_NOBITS`] is not the length of `bytes`.
        size: u32,
    }

    fn progbits(name: &'static str, addr: u32, flags: u32, bytes: &[u8]) -> Sect {
        Sect {
            name,
            kind: SHT_PROGBITS,
            flags: SHF_ALLOC | flags,
            addr,
            size: bytes.len() as u32,
            bytes: bytes.to_vec(),
        }
    }

    fn nobits(name: &'static str, addr: u32, size: u32) -> Sect {
        Sect {
            name,
            kind: SHT_NOBITS,
            flags: SHF_ALLOC | SHF_WRITE,
            addr,
            bytes: Vec::new(),
            size,
        }
    }

    /// Builds a 32-bit little-endian RISC-V executable holding `sects`.
    ///
    /// Two string tables are appended, because [`ElfFile::read_from_bytes`]
    /// insists on a `.strtab` next to the section name table. The symbol names
    /// in it are deliberately not dot-prefixed, so that the section name table
    /// is the one recognised as such.
    fn build_elf(entry: u32, sects: Vec<Sect>) -> Vec<u8> {
        let mut sects = sects;
        sects.push(Sect {
            name: ".strtab",
            kind: SHT_STRTAB,
            flags: 0,
            addr: 0,
            bytes: b"\0_start\0".to_vec(),
            size: 8,
        });
        sects.push(Sect {
            name: ".shstrtab",
            kind: SHT_STRTAB,
            flags: 0,
            addr: 0,
            bytes: Vec::new(),
            size: 0,
        });

        // The name table holds the name of every section, itself included.
        let mut shstrtab = vec![0u8];
        let mut name_offsets = Vec::new();
        for sect in &sects {
            name_offsets.push(shstrtab.len() as u32);
            shstrtab.extend_from_slice(sect.name.as_bytes());
            shstrtab.push(0);
        }
        let last = sects.len() - 1;
        sects[last].size = shstrtab.len() as u32;
        sects[last].bytes = shstrtab;

        // The null section, then the contents, then the section header table.
        let mut file = vec![0u8; 52];
        let mut offsets = Vec::new();
        for sect in &sects {
            while !file.len().is_multiple_of(4) {
                file.push(0);
            }
            offsets.push(file.len() as u32);
            file.extend_from_slice(&sect.bytes);
        }
        while !file.len().is_multiple_of(4) {
            file.push(0);
        }

        let shoff = file.len() as u32;
        file.extend_from_slice(&[0u8; 40]);
        for (i, sect) in sects.iter().enumerate() {
            for word in [
                name_offsets[i],
                sect.kind,
                sect.flags,
                sect.addr,
                offsets[i],
                sect.size,
                0,
                0,
                4,
                0,
            ] {
                file.extend_from_slice(&word.to_le_bytes());
            }
        }

        let header: &mut [u8] = &mut file[..52];
        header[..16].copy_from_slice(&[0x7f, b'E', b'L', b'F', 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        header[16..18].copy_from_slice(&2u16.to_le_bytes()); // ET_EXEC
        header[18..20].copy_from_slice(&243u16.to_le_bytes()); // EM_RISCV
        header[20..24].copy_from_slice(&1u32.to_le_bytes());
        header[24..28].copy_from_slice(&entry.to_le_bytes());
        header[32..36].copy_from_slice(&shoff.to_le_bytes());
        header[40..42].copy_from_slice(&52u16.to_le_bytes());
        header[46..48].copy_from_slice(&40u16.to_le_bytes()); // e_shentsize
        header[48..50].copy_from_slice(&(sects.len() as u16 + 1).to_le_bytes());
        header[50..52].copy_from_slice(&(sects.len() as u16).to_le_bytes());

        file
    }

    /// The sieve-shaped program: all four sections, each where the core wants
    /// it.
    fn well_formed() -> Vec<Sect> {
        vec![
            progbits(".text", 0, SHF_EXECINSTR, &[1, 2, 3, 4, 5, 6, 7, 8]),
            progbits(".rodata", 0x300, 0, &[0xaa, 0xbb, 0xcc, 0xdd]),
            progbits(".data", 0x400, SHF_WRITE, &[0x11, 0x22, 0x33, 0x44]),
            nobits(".bss", 0x800, 400),
        ]
    }

    fn sections(bytes: Vec<u8>) -> Result<ProgramSections, LoadError> {
        Program::Elf(bytes.into()).to_sections(crate::RV32IM, &TEST_MAP)
    }

    #[test]
    fn extracts_every_section() {
        let sections = sections(build_elf(0, well_formed())).expect("a well-formed program");

        assert_eq!(*sections.text, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(*sections.rodata, [0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(*sections.data, [0x11, 0x22, 0x33, 0x44]);
        assert_eq!(sections.bss, 400);
    }

    #[test]
    fn ignores_where_empty_sections_are_linked() {
        let mut sects = well_formed();
        sects[1] = progbits(".rodata", 0x123, 0, &[]);
        assert!(sections(build_elf(0, sects)).is_ok());
    }

    #[test]
    fn rejects_a_program_that_starts_elsewhere() {
        // `.text` where the core boots, but execution starting past it.
        let bytes = build_elf(0x40, well_formed());
        assert!(matches!(
            sections(bytes),
            Err(LoadError::WrongEntry {
                entry: 0x40,
                boot: 0
            })
        ));
    }

    #[test]
    fn rejects_sections_linked_away_from_their_range() {
        // An entry point at the reset vector, but no instructions there.
        let mut sects = well_formed();
        sects[0].addr = 0x100;
        assert!(matches!(
            sections(build_elf(0, sects)),
            Err(LoadError::Misplaced {
                section: ".text",
                addr: 0x100,
                expected: 0
            })
        ));

        // Inside its range is not enough: a section starts at the start of it.
        let mut sects = well_formed();
        sects[2].addr = 0x480;
        assert!(matches!(
            sections(build_elf(0, sects)),
            Err(LoadError::Misplaced {
                section: ".data",
                addr: 0x480,
                expected: 0x400
            })
        ));
    }

    #[test]
    fn rejects_sections_that_do_not_fit() {
        let mut sects = well_formed();
        sects[3] = nobits(".bss", 0x800, 400_000);
        assert!(matches!(
            sections(build_elf(0, sects)),
            Err(LoadError::TooLarge {
                section: ".bss",
                len: 400_000,
                max: 0xba4,
            })
        ));

        let mut sects = well_formed();
        sects[1] = progbits(".rodata", 0x300, 0, &[0; 0x104]);
        assert!(matches!(
            sections(build_elf(0, sects)),
            Err(LoadError::TooLarge {
                section: ".rodata",
                len: 0x104,
                max: 0x100,
            })
        ));
    }

    #[test]
    fn rejects_a_section_it_cannot_place() {
        let mut sects = well_formed();
        sects.push(progbits(".sdata", 0x100, SHF_WRITE, &[1, 2, 3, 4]));
        match sections(build_elf(0, sects)) {
            Err(LoadError::UnknownSection { name, addr, len }) => {
                assert_eq!((name.as_str(), addr, len), (".sdata", 0x100, 4));
            }
            Err(other) => panic!("expected an unplaceable section, got {other:?}"),
            Ok(_) => panic!("expected an unplaceable section to be refused"),
        }

        // A section that takes up no memory at run time is ignored, as the
        // string tables of the file itself are.
        let mut sects = well_formed();
        sects.push(Sect {
            name: ".debug_info",
            kind: SHT_PROGBITS,
            flags: 0,
            addr: 0,
            bytes: vec![1, 2, 3, 4],
            size: 4,
        });
        assert!(sections(build_elf(0, sects)).is_ok());
    }

    #[test]
    fn rejects_files_that_are_not_rv32_executables() {
        // A relocatable object, which is not linked to any address yet.
        let mut bytes = build_elf(0, well_formed());
        bytes[16..18].copy_from_slice(&1u16.to_le_bytes());
        assert!(matches!(
            sections(bytes),
            Err(LoadError::NotRv32Exec { kind: 1, .. })
        ));

        // A 64-bit file, whose headers are laid out differently.
        let mut bytes = build_elf(0, well_formed());
        bytes[4] = 2;
        assert!(matches!(
            sections(bytes),
            Err(LoadError::NotRv32Exec { class: 2, .. })
        ));

        // A program for another architecture.
        let mut bytes = build_elf(0, well_formed());
        bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
        assert!(matches!(
            sections(bytes),
            Err(LoadError::NotRv32Exec { machine: 62, .. })
        ));

        // Not an ELF file at all.
        assert!(matches!(
            sections(b"#!/bin/sh\n".to_vec()),
            Err(LoadError::Elf(_))
        ));
    }

    #[test]
    fn rejects_a_program_without_instructions() {
        let sects = vec![
            progbits(".text", 0, SHF_EXECINSTR, &[]),
            nobits(".bss", 0x800, 400),
        ];
        assert!(matches!(
            sections(build_elf(0, sects)),
            Err(LoadError::NoText)
        ));
    }

    #[test]
    fn rejects_a_section_running_past_the_end_of_the_file() {
        let mut bytes = build_elf(0, well_formed());
        // `.text` is the first section header after the null one, and `sh_size`
        // is its sixth word.
        let shoff = u32::from_le_bytes(bytes[32..36].try_into().unwrap()) as usize;
        let size = shoff + 40 + 20;
        bytes[size..size + 4].copy_from_slice(&4096u32.to_le_bytes());
        assert!(matches!(
            sections(bytes),
            Err(LoadError::Truncated {
                section: ".text",
                ..
            })
        ));
    }

    #[test]
    fn assembles_each_section_at_the_start_of_its_range() {
        let source = "    .section .data\nvalue: .space 4\n    .section .bss\nbuf: .space 8\n    \
                      .section .text\n    .global _start\n_start:\n    addi x1, x0, 7\n";
        let sections = Program::Assembly(source.into())
            .to_sections(crate::RV32IM, &TEST_MAP)
            .expect("a one-instruction program");

        assert_eq!(*sections.text, [0x93, 0x00, 0x70, 0x00]);
        assert_eq!(*sections.data, [0, 0, 0, 0]);
        assert_eq!(sections.bss, 8);
        assert_eq!(disassemble(&sections.text), ["addi        ra,zero,7"]);
    }

    #[test]
    fn rejects_assembly_that_does_not_fit() {
        let source = "    .section .bss\nbuf: .space 400000\n    .section .text\n    nop\n";
        assert!(matches!(
            Program::Assembly(source.into()).to_sections(crate::RV32IM, &TEST_MAP),
            Err(LoadError::TooLarge {
                section: ".bss",
                ..
            })
        ));
    }
}
