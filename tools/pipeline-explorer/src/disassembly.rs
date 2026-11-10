//! Turning a linked ELF executable back into assembly, so that it can be
//! edited like any other program.
//!
//! The assembly is laid out to give the same memory image as loading the ELF
//! file: `.text` at the reset vector, and one `.data` section holding the
//! whole data memory from its first address up to the end of the last
//! section, gaps included. Branches and jumps get labels, named after the
//! symbol table where it has a name for their target. Whatever the assembler
//! cannot reproduce exactly is kept as raw `.word`s.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write;
use std::hash::BuildHasher;
use std::ops::Range;

use hashbrown::{DefaultHashBuilder, HashTable};

use minelf::ElfFile;
use minelf::symbol_table::ElfSymbolType;
use trva::Assembler;
use trva::encoding::Instruction;
use trva::isa::Isa;

use crate::program::{LoadError, MemoryMap, ProgramSections, load_elf, parse_elf};

const INDENT: &str = "    ";
/// Number of values on one `.word` line of the data image.
const WORDS_PER_LINE: usize = 4;
/// `SHN_UNDEF`, the section index of a symbol the file does not define.
const SHN_UNDEF: u16 = 0;
/// `SHN_LORESERVE`: section indices from here on are not sections, such as
/// `SHN_ABS` for symbols that are not addresses.
const SHN_LORESERVE: u16 = 0xff00;

/// Disassembles the linked ELF executable `bytes` into assembly that `isa`
/// assembles, with the positions of `map`, into the memory image the ELF file
/// loads.
pub fn elf_to_assembly(bytes: &[u8], isa: Isa, map: &MemoryMap) -> Result<String, LoadError> {
    // Exactly what loading the file accepts...
    let sections = load_elf(bytes, map)?;
    // ...except that the assembler always puts `.text` at the reset vector.
    if sections.text_addr != map.boot {
        return Err(LoadError::TextNotAtBoot {
            text_addr: sections.text_addr,
            boot: map.boot,
        });
    }
    let elf = parse_elf(bytes)?;

    let text = decode(&sections.text, sections.text_addr);
    let text_end = sections.text_addr + sections.text.len() as u32;
    let starts: BTreeSet<u32> = text.iter().map(|item| item.addr).collect();

    let data = data_image(&sections, map);
    let data_end = data.last().map_or(map.dmem.start, Span::end);

    // A label can only go where the assembly has a line boundary: before an
    // instruction, between data values, or at the end of either memory.
    let mut labels = Labels::from_symbols(&elf, |addr| {
        starts.contains(&addr)
            || addr == text_end
            || (!data.is_empty() && (map.dmem.start..=data_end).contains(&addr))
    });
    for item in &text {
        if let Some(target) = item.target()
            && starts.contains(&target)
        {
            labels.ensure(target);
        }
    }

    let mut out = String::new();
    out.push_str(
        "# Disassembled from an ELF executable. Instructions that do not\n\
         # reassemble to the same bytes are kept as raw values.\n\n",
    );
    out.push_str(".section .text\n");
    for item in &text {
        labels.write_at(item.addr, &mut out);
        write_instruction(item, &labels, &starts, isa, map, &mut out);
    }
    labels.write_at(text_end, &mut out);

    if !data.is_empty() {
        out.push_str("\n.section .data\n");
        for span in &data {
            write_span(span, &mut labels, &mut out);
        }
        labels.write_at(data_end, &mut out);
    }

    Ok(out)
}

/// One instruction of `.text`, or the bytes where one could not be decoded.
struct Decoded<'a> {
    addr: u32,
    bytes: &'a [u8],
    instr: Option<Instruction>,
    /// The section ends partway through the instruction.
    truncated: bool,
}

impl Decoded<'_> {
    /// Where a branch or `jal` goes.
    fn target(&self) -> Option<u32> {
        let offset = match self.instr? {
            Instruction::Beq(i) => i32::from(i.imm()),
            Instruction::Bne(i) => i32::from(i.imm()),
            Instruction::Blt(i) => i32::from(i.imm()),
            Instruction::Bge(i) => i32::from(i.imm()),
            Instruction::Bltu(i) => i32::from(i.imm()),
            Instruction::Bgeu(i) => i32::from(i.imm()),
            Instruction::Jal(i) => i.imm(),
            _ => return None,
        };
        Some(self.addr.wrapping_add_signed(offset))
    }
}

fn decode(text: &[u8], base: u32) -> Vec<Decoded<'_>> {
    let mut items = Vec::new();
    let mut offset = 0;
    while offset < text.len() {
        let rest = &text[offset..];
        let mut reader = rest;
        let (len, instr, truncated) = match Instruction::decode(&mut reader) {
            // An unknown encoding still has a length: 2 or 4 bytes.
            Ok(instr) => (rest.len() - reader.len(), instr, false),
            // Part of an instruction at the end of the section.
            Err(_) => (rest.len(), None, true),
        };
        items.push(Decoded {
            addr: base + offset as u32,
            bytes: &rest[..len],
            instr,
            truncated,
        });
        offset += len;
    }
    items
}

fn write_instruction(
    item: &Decoded<'_>,
    labels: &Labels,
    starts: &BTreeSet<u32>,
    isa: Isa,
    map: &MemoryMap,
    out: &mut String,
) {
    let Some(instr) = item.instr else {
        let why = if item.truncated {
            "truncated instruction"
        } else {
            "unknown instruction"
        };
        return write_raw(item.bytes, why, out);
    };
    let printed = instr.to_string();
    let printed = printed.trim_end();

    if let Some(target) = item.target() {
        // The offset is the last operand; name it instead. A target that is
        // not an instruction of `.text` cannot have a label in it.
        let label = starts
            .contains(&target)
            .then(|| labels.first_at(target))
            .flatten();
        match (label, printed.rsplit_once(',')) {
            (Some(label), Some((operands, _))) => {
                _ = writeln!(out, "{INDENT}{operands},{label}");
            }
            _ => write_raw(item.bytes, printed, out),
        }
        return;
    }

    if reassembles(isa, map, printed, item.bytes) {
        _ = writeln!(out, "{INDENT}{printed}");
    } else {
        write_raw(item.bytes, printed, out);
    }
}

/// Whether assembling `line` on its own gives `bytes`. This rules out both
/// mnemonics the assembler lacks and operands it would read differently.
fn reassembles(isa: Isa, map: &MemoryMap, line: &str, bytes: &[u8]) -> bool {
    Assembler::new(isa, map.to_positions())
        .with_source(line)
        .ok()
        .and_then(|asm| asm.assemble().ok())
        .is_some_and(|asm| *asm.text == *bytes)
}

/// `bytes` as a directive of their own width, with `comment` after them.
fn write_raw(bytes: &[u8], comment: &str, out: &mut String) {
    _ = match *bytes {
        [a, b, c, d] => write!(
            out,
            "{INDENT}.word 0x{:08x}",
            u32::from_le_bytes([a, b, c, d])
        ),
        [a, b] => write!(out, "{INDENT}.half 0x{:04x}", u16::from_le_bytes([a, b])),
        _ => write!(out, "{INDENT}.byte {}", hex_bytes(bytes)),
    };
    _ = writeln!(out, "  # {comment}");
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("0x{b:02x}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A run of the data memory image.
enum Span<'a> {
    /// Bytes of `.data` or `.rodata`, starting at `addr`.
    Bytes { addr: u32, bytes: &'a [u8] },
    /// Zeroes: a gap between sections, or `.bss`.
    Zero { addr: u32, len: u32 },
}

impl Span<'_> {
    fn start(&self) -> u32 {
        match *self {
            Span::Bytes { addr, .. } | Span::Zero { addr, .. } => addr,
        }
    }
    fn end(&self) -> u32 {
        match *self {
            Span::Bytes { addr, bytes } => addr + bytes.len() as u32,
            Span::Zero { addr, len } => addr + len,
        }
    }
}

/// The data memory from its first address to the end of the last section,
/// in address order. Empty when the program has no data at all.
fn data_image<'a>(sections: &'a ProgramSections, map: &MemoryMap) -> Vec<Span<'a>> {
    let mut filled: Vec<(u32, &[u8])> = [
        (sections.data_addr, &*sections.data),
        (sections.rodata_addr, &*sections.rodata),
    ]
    .into_iter()
    .filter(|(_, bytes)| !bytes.is_empty())
    .collect();
    filled.sort_by_key(|&(addr, _)| addr);

    let end = filled
        .iter()
        .map(|&(addr, bytes)| addr + bytes.len() as u32)
        .chain((sections.bss != 0).then(|| sections.bss_addr + sections.bss as u32))
        .max();
    let Some(end) = end else {
        return Vec::new();
    };

    let mut spans = Vec::new();
    let mut cursor = map.dmem.start;
    for (addr, bytes) in filled {
        if addr > cursor {
            spans.push(Span::Zero {
                addr: cursor,
                len: addr - cursor,
            });
        }
        // Overlapping sections: the later one is only written past the
        // earlier one.
        let skip = (cursor.saturating_sub(addr) as usize).min(bytes.len());
        let bytes = &bytes[skip..];
        if !bytes.is_empty() {
            let addr = addr.max(cursor);
            spans.push(Span::Bytes { addr, bytes });
            cursor = addr + bytes.len() as u32;
        }
    }
    if end > cursor {
        spans.push(Span::Zero {
            addr: cursor,
            len: end - cursor,
        });
    }
    spans
}

/// Writes `span`, broken up wherever a label falls inside it.
fn write_span(span: &Span<'_>, labels: &mut Labels, out: &mut String) {
    let (start, end) = (span.start(), span.end());
    let cuts: Vec<u32> = labels.addrs_in(start..end).chain([end]).collect();
    let mut addr = start;
    for cut in cuts {
        if cut > addr {
            match *span {
                Span::Bytes { addr: base, bytes } => {
                    let from = (addr - base) as usize;
                    let to = (cut - base) as usize;
                    write_bytes(addr, &bytes[from..to], out);
                }
                Span::Zero { .. } => _ = writeln!(out, "{INDENT}.space {}", cut - addr),
            }
        }
        if cut < end {
            labels.write_at(cut, out);
        }
        addr = cut;
    }
}

/// `bytes` at `addr` as `.word`s where they are word-aligned, and `.byte`s
/// around them.
fn write_bytes(addr: u32, bytes: &[u8], out: &mut String) {
    let head = ((4 - addr % 4) % 4) as usize;
    let (head, rest) = bytes.split_at(head.min(bytes.len()));
    let (words, tail) = rest.split_at(rest.len() / 4 * 4);
    if words.is_empty() {
        _ = writeln!(out, "{INDENT}.byte {}", hex_bytes(bytes));
        return;
    }

    if !head.is_empty() {
        _ = writeln!(out, "{INDENT}.byte {}", hex_bytes(head));
    }
    for line in words.chunks(4 * WORDS_PER_LINE) {
        let values: Vec<String> = line
            .chunks(4)
            .map(|w| format!("0x{:08x}", u32::from_le_bytes(w.try_into().unwrap())))
            .collect();
        _ = writeln!(out, "{INDENT}.word {}", values.join(", "));
    }
    if !tail.is_empty() {
        _ = writeln!(out, "{INDENT}.byte {}", hex_bytes(tail));
    }
}

/// The labels of the program by address, each name used once.
#[derive(Default)]
struct Labels {
    names: String,
    at: BTreeMap<u32, Vec<Range<usize>>>,
    /// The names in use, hashed and compared by their text in `names`.
    taken: HashTable<Range<usize>>,
    hasher: DefaultHashBuilder,
    /// Addresses whose labels are written out already. `.text` can end
    /// exactly where the data memory starts.
    written: HashSet<u32>,
}

impl Labels {
    /// The names the symbol table gives to addresses `wanted` accepts.
    fn from_symbols(elf: &ElfFile, wanted: impl Fn(u32) -> bool) -> Self {
        let mut symbols: Vec<(u32, &str)> = elf
            .symbols()
            .iter()
            .filter(|sym| {
                matches!(
                    sym.st_info.s_type,
                    ElfSymbolType::Notype | ElfSymbolType::Object | ElfSymbolType::Func
                ) && sym.st_shndx != SHN_UNDEF
                    && sym.st_shndx < SHN_LORESERVE
                    && wanted(sym.st_value)
            })
            .filter_map(|sym| {
                let name = std::str::from_utf8(elf.get_symbol_string(sym)?).ok()?;
                is_label_name(name).then_some((sym.st_value, name))
            })
            .collect();
        // By address, so that of two symbols of the same name, the first one
        // in memory keeps it.
        symbols.sort();

        let mut labels = Self::default();
        for (addr, name) in symbols {
            if let Some(range) = labels.take(|names| names.push_str(name)) {
                labels.at.entry(addr).or_default().push(range);
            }
        }
        labels
    }

    /// Makes sure `addr` has a label, making up a name if it has none.
    fn ensure(&mut self, addr: u32) {
        if self.at.contains_key(&addr) {
            return;
        }
        let mut n = 0;
        let range = loop {
            let name = |names: &mut String| {
                _ = write!(names, "L_{addr:04x}");
                if n > 0 {
                    _ = write!(names, "_{n}");
                }
            };
            if let Some(range) = self.take(name) {
                break range;
            }
            n += 1;
        };
        self.at.insert(addr, vec![range]);
    }

    /// Appends the name `write` writes to `names` and takes it, unless it is
    /// taken already; then it is removed again.
    fn take(&mut self, write: impl FnOnce(&mut String)) -> Option<Range<usize>> {
        let start = self.names.len();
        write(&mut self.names);
        let range = start..self.names.len();

        let (names, hasher) = (&self.names, &self.hasher);
        let name = &names[range.clone()];
        let hash = hasher.hash_one(name);
        if self
            .taken
            .find(hash, |taken| names[taken.clone()] == *name)
            .is_some()
        {
            self.names.truncate(start);
            return None;
        }
        self.taken.insert_unique(hash, range.clone(), |taken| {
            hasher.hash_one(&names[taken.clone()])
        });
        Some(range)
    }

    fn first_at(&self, addr: u32) -> Option<&str> {
        self.at
            .get(&addr)
            .map(|ranges| &self.names[ranges[0].clone()])
    }

    fn addrs_in(&self, range: Range<u32>) -> impl Iterator<Item = u32> + use<> {
        self.at
            .range(range)
            .map(|(&addr, _)| addr)
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Writes the labels at `addr`, once.
    fn write_at(&mut self, addr: u32, out: &mut String) {
        if let Some(ranges) = self.at.get(&addr)
            && self.written.insert(addr)
        {
            for range in ranges {
                _ = writeln!(out, "{}:", &self.names[range.clone()]);
            }
        }
    }
}

/// Whether the assembler reads `name` as a label. Mapping symbols such as
/// `$x` mark code and data for disassemblers and are not names.
fn is_label_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '_' | '.'))
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '$'))
}

#[cfg(test)]
mod tests {
    use trva::encoding::{Beq, Jal, XRegIdent};

    use super::*;
    use crate::RV32IM;
    use crate::program::assemble;
    use crate::program::tests::{
        SHF_EXECINSTR, SHF_WRITE, TEST_MAP, build_elf, build_elf_with_symbols, nobits, progbits,
    };

    /// The instruction and data memory a program loads.
    fn image(sections: &ProgramSections, map: &MemoryMap) -> (Vec<u8>, Vec<u8>) {
        let mut imem = vec![0; map.imem.len()];
        let mut dmem = vec![0; map.dmem.len()];
        let at = |addr: u32, range: &Range<u32>| (addr - range.start) as usize;
        let text = at(sections.text_addr, &map.imem);
        imem[text..][..sections.text.len()].copy_from_slice(&sections.text);
        for (addr, bytes) in [
            (sections.data_addr, &sections.data),
            (sections.rodata_addr, &sections.rodata),
        ] {
            if !bytes.is_empty() {
                dmem[at(addr, &map.dmem)..][..bytes.len()].copy_from_slice(bytes);
            }
        }
        (imem, dmem)
    }

    /// Disassembles `elf` and checks that the assembly loads the same memory
    /// image as the file itself.
    fn round_trip(elf: &[u8], map: &MemoryMap) -> String {
        let asm = elf_to_assembly(elf, RV32IM, map).expect("the ELF file to disassemble");
        let assembled =
            assemble(&asm, RV32IM, map).unwrap_or_else(|err| panic!("{err} in:\n{asm}"));
        let loaded = load_elf(elf, map).expect("the ELF file to load");
        assert!(
            image(&assembled, map) == image(&loaded, map),
            "different memory image from:\n{asm}"
        );
        asm
    }

    fn word(instr: u32) -> [u8; 4] {
        instr.to_le_bytes()
    }

    fn text(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    #[test]
    fn sample_programs_round_trip() {
        for (elf, boot) in [
            (&include_bytes!("../programs/sieve.elf")[..], 0),
            (&include_bytes!("../programs/sieve-ibex.elf")[..], 0x80),
        ] {
            let asm = round_trip(elf, &MemoryMap { boot, ..TEST_MAP });
            // Labels come from the symbol table.
            assert!(asm.contains("\n_start:\n"), "{asm}");
            assert!(asm.contains("\n    beq         s1,s3,found\n"), "{asm}");
            assert!(asm.contains("\n    jal         zero,outer_loop\n"), "{asm}");
            assert!(asm.contains("\nprimes:\n    .space 400\n"), "{asm}");
        }
    }

    #[test]
    fn makes_up_labels_without_a_symbol_table() {
        let zero = XRegIdent::Zero;
        let nop = word(0x0000_0013);
        let code = text(&[
            &word(Beq::new(zero, zero, 8).encode_as_u32()),
            &nop,
            &nop,
            &word(Jal::new(zero, -12).encode_as_u32()),
        ]);
        let elf = build_elf(0, vec![progbits(".text", 0, SHF_EXECINSTR, &code)]);
        let asm = round_trip(&elf, &TEST_MAP);
        assert!(
            asm.ends_with(
                ".section .text\nL_0000:\n    beq         zero,zero,L_0008\n    \
                 addi        zero,zero,0\nL_0008:\n    addi        zero,zero,0\n    \
                 jal         zero,L_0000\n"
            ),
            "{asm}"
        );
    }

    #[test]
    fn made_up_labels_avoid_symbol_names() {
        let zero = XRegIdent::Zero;
        let code = text(&[
            &word(Beq::new(zero, zero, 4).encode_as_u32()),
            &word(0x0000_0013),
        ]);
        let elf = build_elf_with_symbols(
            0,
            vec![
                progbits(".text", 0, SHF_EXECINSTR, &code),
                progbits(".data", 0x400, SHF_WRITE, &[1, 2, 3, 4]),
            ],
            &[("L_0004", 0x400, ".data")],
        );
        let asm = round_trip(&elf, &TEST_MAP);
        assert!(
            asm.contains("beq         zero,zero,L_0004_1\nL_0004_1:\n"),
            "{asm}"
        );
        assert!(asm.contains("\nL_0004:\n    .word 0x04030201\n"), "{asm}");
    }

    #[test]
    fn keeps_what_does_not_reassemble_as_raw_values() {
        let zero = XRegIdent::Zero;
        let code = text(&[
            // 0x00: into the middle of the `csrrs` at 0x08.
            &word(Beq::new(zero, zero, 10).encode_as_u32()),
            // 0x04, 0x06: compressed, which RV32IM does not assemble.
            &[0x01, 0x00],
            &[0x01, 0x00],
            // 0x08: `csrrs a0,mcycle,zero`, which the assembler lacks.
            &word(0xb000_2573),
            // 0x0c: no instruction at all.
            &word(0xffff_ffff),
            // 0x10: past the end of `.text`.
            &word(Jal::new(zero, 0x100).encode_as_u32()),
            // 0x14: back to the `csrrs`, which does start an instruction.
            &word(Beq::new(zero, zero, -12).encode_as_u32()),
            // 0x18: half of an instruction.
            &[0x13, 0x00],
        ]);
        let elf = build_elf(0, vec![progbits(".text", 0, SHF_EXECINSTR, &code)]);
        let asm = round_trip(&elf, &TEST_MAP);
        let body = asm.split_once(".section .text\n").unwrap().1;
        assert_eq!(
            body,
            "    .word 0x00000563  # beq         zero,zero,10\n\
             \x20   .half 0x0001  # c.nop\n\
             \x20   .half 0x0001  # c.nop\n\
             L_0008:\n\
             \x20   .word 0xb0002573  # csrrs       a0,2816,zero\n\
             \x20   .word 0xffffffff  # unknown instruction\n\
             \x20   .word 0x1000006f  # jal         zero,256\n\
             \x20   beq         zero,zero,L_0008\n\
             \x20   .half 0x0013  # truncated instruction\n"
        );
    }

    #[test]
    fn data_memory_is_one_image() {
        let data: Vec<u8> = (0..20).collect();
        let elf = build_elf_with_symbols(
            0,
            vec![
                progbits(".text", 0, SHF_EXECINSTR, &word(0x0000_0013)),
                // In the opposite order of the assembler, with gaps.
                progbits(".data", 0x40c, SHF_WRITE, &data),
                progbits(".rodata", 0x400, 0, &[1, 2, 3, 4, 5, 6]),
                nobits(".bss", 0x420, 8),
            ],
            &[
                ("main", 0, ".text"),
                ("table", 0x402, ".rodata"),
                ("flag", 0x40c, ".data"),
                ("buf", 0x420, ".bss"),
                ("end", 0x428, ".bss"),
                // A mapping symbol, and a name that is taken already.
                ("$d", 0x400, ".rodata"),
                ("main", 0x404, ".rodata"),
            ],
        );
        let asm = round_trip(&elf, &TEST_MAP);
        let data = asm.split_once("\n.section .data\n").unwrap().1;
        assert_eq!(
            data,
            "    .byte 0x01, 0x02\n\
             table:\n\
             \x20   .byte 0x03, 0x04, 0x05, 0x06\n\
             \x20   .space 6\n\
             flag:\n\
             \x20   .word 0x03020100, 0x07060504, 0x0b0a0908, 0x0f0e0d0c\n\
             \x20   .word 0x13121110\n\
             buf:\n\
             \x20   .space 8\n\
             end:\n"
        );
        assert!(asm.contains("\nmain:\n    addi"), "{asm}");
    }

    #[test]
    fn no_data_section_without_data() {
        let elf = build_elf(
            0,
            vec![progbits(".text", 0, SHF_EXECINSTR, &word(0x0000_0013))],
        );
        assert!(!round_trip(&elf, &TEST_MAP).contains(".data"));
    }

    #[test]
    fn refuses_text_away_from_the_reset_vector() {
        // Execution starts at the reset vector, but `.text` starts before it.
        let code = text(&[&word(0x0000_0013), &word(0x0000_0013)]);
        let elf = build_elf(4, vec![progbits(".text", 0, SHF_EXECINSTR, &code)]);
        let map = MemoryMap {
            boot: 4,
            ..TEST_MAP
        };
        assert!(load_elf(&elf, &map).is_ok());
        assert!(matches!(
            elf_to_assembly(&elf, RV32IM, &map),
            Err(LoadError::TextNotAtBoot {
                text_addr: 0,
                boot: 4
            })
        ));
    }

    #[test]
    fn refuses_what_the_loader_refuses() {
        let elf = build_elf(0x40, vec![progbits(".text", 0, SHF_EXECINSTR, &word(0x13))]);
        assert!(matches!(
            elf_to_assembly(&elf, RV32IM, &TEST_MAP),
            Err(LoadError::WrongEntry { .. })
        ));
    }
}
