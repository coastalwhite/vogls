//! Turning a program into the sections a core loads.

use std::fmt;
use std::ops::Range;

use minelf::ElfFile;
use minelf::data_encoding::DataEncoding;
use minelf::errors::ElfError;
use minelf::header::ElfHeader;
use trva::error::{AssembleError, LayoutError};
use trva::isa::Isa;
use trva::{Assembler, SectionKind, SectionPlacement, SectionPositions};

/// `EM_RISCV`, the `e_machine` of a RISC-V program.
const EM_RISCV: u16 = 243;
/// `ELFCLASS32`, the `e_ident[EI_CLASS]` of a 32-bit ELF file.
const ELFCLASS32: u8 = 1;
/// `ET_EXEC`, the `e_type` of an executable (fully linked) ELF file.
const ET_EXEC: u16 = 2;

/// Alignment of the start of each data section the assembler lays out.
const DATA_SECTION_ALIGN: u32 = 4;

/// One of the two memories of a core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Memory {
    Instruction,
    Data,
}

impl fmt::Display for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Memory::Instruction => "instruction memory",
            Memory::Data => "data memory",
        })
    }
}

/// Where the memories of a core sit in its address space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryMap {
    pub boot: u32,
    pub imem: Range<u32>,
    pub dmem: Range<u32>,
}

impl MemoryMap {
    pub fn to_positions(&self) -> SectionPositions {
        SectionPositions {
            text: SectionPlacement::Absolute(self.boot),
            data: SectionPlacement::Absolute(self.dmem.start),
            rodata: SectionPlacement::after_aligned(SectionKind::Data, DATA_SECTION_ALIGN),
            bss: SectionPlacement::after_aligned(SectionKind::ReadOnlyData, DATA_SECTION_ALIGN),
        }
    }

    /// The addresses of `memory`.
    pub fn range(&self, memory: Memory) -> &Range<u32> {
        match memory {
            Memory::Instruction => &self.imem,
            Memory::Data => &self.dmem,
        }
    }
}

/// The contents of each section of a program, and the address it is linked
/// at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgramSections {
    pub text: Box<[u8]>,
    pub data: Box<[u8]>,
    pub rodata: Box<[u8]>,
    /// Size of `.bss`. The section carries no bytes of its own: it is the
    /// zero-initialised part of memory, and only its size matters.
    pub bss: usize,

    pub text_addr: u32,
    pub data_addr: u32,
    pub rodata_addr: u32,
    pub bss_addr: u32,
}

impl ProgramSections {
    /// The name, memory, address and size of each section.
    pub fn placed(&self) -> [(&'static str, Memory, u32, usize); 4] {
        [
            (
                ".text",
                Memory::Instruction,
                self.text_addr,
                self.text.len(),
            ),
            (".data", Memory::Data, self.data_addr, self.data.len()),
            (".rodata", Memory::Data, self.rodata_addr, self.rodata.len()),
            (".bss", Memory::Data, self.bss_addr, self.bss),
        ]
    }

    /// Checks that every section lies within the memory `map` gives it.
    fn check_fits(&self, map: &MemoryMap) -> Result<(), LoadError> {
        for (section, memory, addr, len) in self.placed() {
            // Where an empty section is linked does not matter.
            if len == 0 {
                continue;
            }
            let range = map.range(memory);
            let end = u64::from(addr) + len as u64;
            if addr < range.start || end > u64::from(range.end) {
                return Err(LoadError::OutsideMemory {
                    section,
                    memory,
                    addr,
                    len,
                    range: range.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Assembles RISC-V `source` into the sections `map` places, and checks that
/// they fit the memories of `map`.
pub fn assemble(source: &str, isa: Isa, map: &MemoryMap) -> Result<ProgramSections, LoadError> {
    let assembled = Assembler::new(isa, map.to_positions())
        .with_source(source)?
        .assemble()?;
    let sections = ProgramSections {
        text: assembled.text,
        data: assembled.data,
        rodata: assembled.rodata,
        bss: assembled.bss,
        text_addr: assembled.text_start,
        data_addr: assembled.data_start,
        rodata_addr: assembled.rodata_start,
        bss_addr: assembled.bss_start,
    };
    sections.check_fits(map)?;
    Ok(sections)
}

/// Reads the sections of the linked ELF executable `bytes`, and checks that
/// they fit the memories of `map`, as a core would load them.
pub fn load_elf(bytes: &[u8], map: &MemoryMap) -> Result<ProgramSections, LoadError> {
    let sections = sections_from_elf(bytes, map)?;
    sections.check_fits(map)?;
    Ok(sections)
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
    /// A section is linked, at least partly, outside the memory it belongs in.
    OutsideMemory {
        section: &'static str,
        memory: Memory,
        addr: u32,
        len: usize,
        range: Range<u32>,
    },
    /// `.text` does not start where the core boots. The loader accepts that,
    /// but assembly always places `.text` at the reset vector.
    TextNotAtBoot {
        text_addr: u32,
        boot: u32,
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
            LoadError::OutsideMemory {
                section,
                memory,
                addr,
                len,
                range,
            } => write!(
                f,
                "'{section}' ({len} bytes at 0x{addr:08x}) does not fit in the {memory}, \
                 which spans 0x{:08x}..0x{:08x}",
                range.start, range.end,
            ),
            LoadError::TextNotAtBoot { text_addr, boot } => write!(
                f,
                "'.text' starts at 0x{text_addr:08x} but the core boots at 0x{boot:08x}, \
                 so it cannot be turned into assembly",
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

/// Extracts the four sections of a linked ELF executable, with the address
/// each is linked at. [`load_elf`] checks those against `map`.
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

    // A section the file lacks is empty, and where it would go is irrelevant.
    let addr = |name: &[u8]| elf.get_section(name).map_or(0, |sect| sect.sh_addr);
    let sections = ProgramSections {
        text: read_section(&elf, bytes, ".text")?,
        data: read_section(&elf, bytes, ".data")?,
        rodata: read_section(&elf, bytes, ".rodata")?,
        bss: elf
            .get_section(b".bss")
            .map_or(0, |sect| sect.sh_size as usize),
        text_addr: addr(b".text"),
        data_addr: addr(b".data"),
        rodata_addr: addr(b".rodata"),
        bss_addr: addr(b".bss"),
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
pub(crate) mod tests {
    use super::*;

    const SHT_PROGBITS: u32 = 1;
    const SHT_SYMTAB: u32 = 2;
    const SHT_STRTAB: u32 = 3;
    const SHT_NOBITS: u32 = 8;

    pub(crate) const SHF_WRITE: u32 = 0x1;
    const SHF_ALLOC: u32 = 0x2;
    pub(crate) const SHF_EXECINSTR: u32 = 0x4;

    /// A core shaped like the testbenches.
    pub(crate) const TEST_MAP: MemoryMap = MemoryMap {
        boot: 0,
        imem: 0..0x400,
        dmem: 0x400..0x13a4,
    };

    pub(crate) struct Sect {
        name: &'static str,
        kind: u32,
        flags: u32,
        addr: u32,
        /// Bytes in the file; empty for a section that only takes up memory.
        bytes: Vec<u8>,
        /// `sh_size`, which for [`SHT_NOBITS`] is not the length of `bytes`.
        size: u32,
    }

    pub(crate) fn progbits(name: &'static str, addr: u32, flags: u32, bytes: &[u8]) -> Sect {
        Sect {
            name,
            kind: SHT_PROGBITS,
            flags: SHF_ALLOC | flags,
            addr,
            size: bytes.len() as u32,
            bytes: bytes.to_vec(),
        }
    }

    pub(crate) fn nobits(name: &'static str, addr: u32, size: u32) -> Sect {
        Sect {
            name,
            kind: SHT_NOBITS,
            flags: SHF_ALLOC | SHF_WRITE,
            addr,
            bytes: Vec::new(),
            size,
        }
    }

    /// Builds a 32-bit little-endian RISC-V executable holding `sects`, and
    /// nothing else: like a stripped file, it has no symbol table and no
    /// `.strtab` for the names in one.
    pub(crate) fn build_elf(entry: u32, sects: Vec<Sect>) -> Vec<u8> {
        build_elf_with_symbols(entry, sects, &[])
    }

    /// [`build_elf`] with a symbol table, and a `.strtab` for its names: each
    /// symbol is a name, its value, and the name of the section it is defined
    /// in.
    ///
    /// The names must not start with a dot. [`ElfFile::read_from_bytes`]
    /// recognises the section name table as the string table whose names all
    /// do.
    pub(crate) fn build_elf_with_symbols(
        entry: u32,
        sects: Vec<Sect>,
        symbols: &[(&str, u32, &str)],
    ) -> Vec<u8> {
        let mut sects = sects;
        if !symbols.is_empty() {
            let mut strtab = vec![0u8];
            // The null symbol, then the given ones as global functions.
            let mut symtab = vec![0u8; 16];
            for &(name, value, section) in symbols {
                let st_name = strtab.len() as u32;
                strtab.extend_from_slice(name.as_bytes());
                strtab.push(0);
                // Section headers are numbered from 1, after the null one.
                let shndx = 1 + sects
                    .iter()
                    .position(|sect| sect.name == section)
                    .expect("symbol in a known section") as u16;
                symtab.extend_from_slice(&st_name.to_le_bytes());
                symtab.extend_from_slice(&value.to_le_bytes());
                symtab.extend_from_slice(&0u32.to_le_bytes());
                symtab.push(0x12); // STB_GLOBAL, STT_FUNC
                symtab.push(0);
                symtab.extend_from_slice(&shndx.to_le_bytes());
            }
            sects.push(Sect {
                name: ".symtab",
                kind: SHT_SYMTAB,
                flags: 0,
                addr: 0,
                size: symtab.len() as u32,
                bytes: symtab,
            });
            sects.push(Sect {
                name: ".strtab",
                kind: SHT_STRTAB,
                flags: 0,
                addr: 0,
                size: strtab.len() as u32,
                bytes: strtab,
            });
        }
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
                // `sh_entsize`, which minelf insists on for a symbol table.
                if sect.kind == SHT_SYMTAB { 16 } else { 0 },
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

    /// The sieve-shaped program: all four sections, `.text` in the
    /// instruction memory and the rest after each other in the data memory.
    fn well_formed() -> Vec<Sect> {
        vec![
            progbits(".text", 0, SHF_EXECINSTR, &[1, 2, 3, 4, 5, 6, 7, 8]),
            progbits(".rodata", 0x404, 0, &[0xaa, 0xbb, 0xcc, 0xdd]),
            progbits(".data", 0x400, SHF_WRITE, &[0x11, 0x22, 0x33, 0x44]),
            nobits(".bss", 0x408, 400),
        ]
    }

    fn sections(bytes: Vec<u8>) -> Result<ProgramSections, LoadError> {
        load_elf(&bytes, &TEST_MAP)
    }

    #[test]
    fn extracts_every_section() {
        let sections = sections(build_elf(0, well_formed())).expect("a well-formed program");

        assert_eq!(*sections.text, [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(*sections.rodata, [0xaa, 0xbb, 0xcc, 0xdd]);
        assert_eq!(*sections.data, [0x11, 0x22, 0x33, 0x44]);
        assert_eq!(sections.bss, 400);
        assert_eq!(
            [
                sections.text_addr,
                sections.data_addr,
                sections.rodata_addr,
                sections.bss_addr
            ],
            [0, 0x400, 0x404, 0x408]
        );
    }

    #[test]
    fn loads_a_stripped_file() {
        // No symbol table, and no `.strtab`, which used to be a panic.
        let bytes = build_elf(0, well_formed());
        assert!(!bytes.windows(8).any(|w| w == b".strtab\0"));
        assert!(sections(bytes).is_ok());
    }

    #[test]
    fn accepts_sections_anywhere_in_their_memory() {
        // Gaps, and data sections in another order, are the linker's business.
        let mut sects = well_formed();
        sects[1].addr = 0x1000;
        sects[2].addr = 0x480;
        sects[3].addr = 0x1200;
        assert!(sections(build_elf(0, sects)).is_ok());
    }

    #[test]
    fn ignores_where_empty_sections_are_linked() {
        let mut sects = well_formed();
        sects[1] = progbits(".rodata", 0x123, 0, &[]);
        assert!(sections(build_elf(0, sects)).is_ok());
    }

    #[test]
    fn loads_the_sample_programs() {
        for (elf, boot) in [
            (&include_bytes!("../programs/sieve.elf")[..], 0),
            (&include_bytes!("../programs/sieve-ibex.elf")[..], 0x80),
        ] {
            let map = MemoryMap { boot, ..TEST_MAP };
            let sections = load_elf(elf, &map).expect("the sample program to load");
            assert_eq!(sections.text_addr, boot);
            assert_eq!(sections.data_addr, 0x400);
        }
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
    fn rejects_sections_outside_their_memory() {
        // Instructions in the data memory, starting execution there too.
        let mut sects = well_formed();
        sects[0].addr = 0x400;
        let map = MemoryMap {
            boot: 0x400,
            ..TEST_MAP
        };
        assert!(matches!(
            load_elf(&build_elf(0x400, sects), &map),
            Err(LoadError::OutsideMemory {
                section: ".text",
                memory: Memory::Instruction,
                addr: 0x400,
                ..
            })
        ));

        // Read-only data in the instruction memory.
        let mut sects = well_formed();
        sects[1].addr = 0x300;
        assert!(matches!(
            sections(build_elf(0, sects)),
            Err(LoadError::OutsideMemory {
                section: ".rodata",
                memory: Memory::Data,
                addr: 0x300,
                ..
            })
        ));
    }

    #[test]
    fn rejects_sections_that_do_not_fit() {
        let mut sects = well_formed();
        sects[3] = nobits(".bss", 0x408, 400_000);
        assert!(matches!(
            sections(build_elf(0, sects)),
            Err(LoadError::OutsideMemory {
                section: ".bss",
                len: 400_000,
                ..
            })
        ));

        // Starting inside the memory is not enough: it has to end there too.
        let mut sects = well_formed();
        sects[3] = nobits(".bss", 0x1300, 0x100);
        assert!(matches!(
            sections(build_elf(0, sects)),
            Err(LoadError::OutsideMemory {
                section: ".bss",
                addr: 0x1300,
                ..
            })
        ));

        let mut sects = well_formed();
        sects[0] = progbits(".text", 0, SHF_EXECINSTR, &[0; 0x404]);
        assert!(matches!(
            sections(build_elf(0, sects)),
            Err(LoadError::OutsideMemory {
                section: ".text",
                len: 0x404,
                ..
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
            nobits(".bss", 0x408, 400),
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
    fn assembles_data_sections_after_each_other() {
        let source = "    .section .data\nvalue: .space 4\n    .section .rodata\nconst: .space 3\n    \
                      .section .bss\nbuf: .space 8\n    .section .text\n    .global _start\n\
                      _start:\n    addi x1, x0, 7\n";
        let sections =
            assemble(source, crate::RV32IM, &TEST_MAP).expect("a one-instruction program");

        assert_eq!(*sections.text, [0x93, 0x00, 0x70, 0x00]);
        assert_eq!(*sections.data, [0, 0, 0, 0]);
        assert_eq!(*sections.rodata, [0, 0, 0]);
        assert_eq!(sections.bss, 8);
        assert_eq!(disassemble(&sections.text), ["addi        ra,zero,7"]);

        // `.text` at the reset vector; `.rodata` straight after `.data` at the
        // start of the data memory, and `.bss` after that, word-aligned.
        assert_eq!(
            [
                sections.text_addr,
                sections.data_addr,
                sections.rodata_addr,
                sections.bss_addr
            ],
            [0, 0x400, 0x404, 0x408]
        );
    }

    #[test]
    fn rejects_assembly_that_does_not_fit() {
        let source = "    .section .bss\nbuf: .space 400000\n    .section .text\n    nop\n";
        assert!(matches!(
            assemble(source, crate::RV32IM, &TEST_MAP),
            Err(LoadError::OutsideMemory {
                section: ".bss",
                ..
            })
        ));
    }
}
