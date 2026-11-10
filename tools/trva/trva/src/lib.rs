//! Tiny RISC-V Assembler
//!
//! A library that performs RISC-V assembly into sections with basic relaxation.
use self::cursor::{SourceCursor, parse_gp_register};
use self::encoding::{CXRegIdent, FenceMode, FenceOrder, XRegIdent};
use self::error::{AssembleResult, LayoutError};
use self::isa::{Ext, Isa, XLen};
use self::labels::{LabelId, LabelMap, LabelTable};

#[cfg(feature = "foldhash")]
use foldhash::fast::RandomState;
#[cfg(not(feature = "foldhash"))]
use std::hash::RandomState;

mod cursor;
pub mod encoding;
pub mod error;
mod insn;
pub mod isa;
pub mod labels;
pub mod placement;

pub use self::placement::{
    ParsePlacementError, Radix, SectionKind, SectionPlacement, SectionPositions, UnknownSectionName,
};

pub struct Assembler {
    label_table: LabelTable,

    /// Positions of global symbols, relative to the start of their own section.
    symbols: LabelMap<(SectionKind, UnresolvedOffset)>,

    text: Section,
    data: Section,
    rodata: Section,
    bss: Section,

    positions: SectionPositions,
    current_section: SectionKind,

    isa: Isa,
    handlers: hashbrown::HashMap<&'static str, MnemonicHandler, RandomState>,
}

enum BranchKind {
    Eq,
    NotEq,
    Lt,
    Ge,
    Ltu,
    Geu,
}

enum RelocationItem {
    LoadAddress(XRegIdent, TargetLabel),
    Branch(XRegIdent, XRegIdent, BranchKind, TargetLabel),
    Jump(XRegIdent, TargetLabel),
    PcRelJump(XRegIdent, XRegIdent, TargetLabel),
    /// A compressed branch or jump, which is 2 bytes whatever the distance.
    Compressed(CompressedJump, TargetLabel),
    /// A branch or jump written with `.insn`, which is never relaxed either.
    Insn(insn::InsnJump, TargetLabel),
}

#[derive(Clone, Copy)]
enum CompressedJump {
    Beqz(CXRegIdent),
    Bnez(CXRegIdent),
    J,
    Jal,
}

impl CompressedJump {
    fn mnemonic(self) -> &'static str {
        match self {
            Self::Beqz(_) => encoding::CBeqz::MNEMONIC,
            Self::Bnez(_) => encoding::CBnez::MNEMONIC,
            Self::J => encoding::CJ::MNEMONIC,
            Self::Jal => encoding::CJal::MNEMONIC,
        }
    }

    /// The offsets the instruction encodes.
    fn range(self) -> (i32, i32) {
        match self {
            Self::Beqz(_) | Self::Bnez(_) => (-256, 254),
            Self::J | Self::Jal => (-2048, 2046),
        }
    }

    fn encode(self, offset: i16) -> u16 {
        let bits = match self {
            Self::Beqz(rs1) => encoding::CBeqz::new(rs1, offset).encode_as_u32(),
            Self::Bnez(rs1) => encoding::CBnez::new(rs1, offset).encode_as_u32(),
            Self::J => encoding::CJ::new(offset).encode_as_u32(),
            Self::Jal => encoding::CJal::new(offset).encode_as_u32(),
        };
        bits as u16
    }
}
impl RelocationItem {
    fn min_max_size(&self) -> (u32, u32) {
        match self {
            Self::LoadAddress(..) => (8, 8),
            Self::Branch(..) => (4, 12),
            Self::Jump(..) => (4, 8),
            Self::PcRelJump(..) => (4, 8),
            Self::Compressed(..) => (2, 2),
            Self::Insn(jump, _) => (jump.format.size(), jump.format.size()),
        }
    }

    fn min_max_size_with_symbols(
        &self,
        symbols: &LabelMap<(SectionKind, UnresolvedOffset)>,
    ) -> (u32, u32) {
        _ = symbols;
        self.min_max_size()
    }
}

const fn in_imm12_range(value: u32, center: u32) -> bool {
    let left = center.saturating_sub(2 << 11);
    let right = center.saturating_add((2 << 11) - 1);
    left <= value && value <= right
}

const fn in_imm20_range(value: u32, center: u32) -> bool {
    let left = center.saturating_sub(2 << 19);
    let right = center.saturating_add((2 << 19) - 1);
    left <= value && value <= right
}

pub struct Assembled {
    pub labels: LabelTable,
    pub symbols: LabelMap<(SectionKind, u32)>,

    pub text_start: u32,
    pub data_start: u32,
    pub rodata_start: u32,
    pub bss_start: u32,

    pub text: Box<[u8]>,
    pub data: Box<[u8]>,
    pub rodata: Box<[u8]>,
    pub bss: usize,
}

#[derive(Default, Clone, Copy)]
struct UnresolvedOffset {
    min: u32,
    max: u32,
}

impl UnresolvedOffset {
    fn pad(self, space: u32) -> Self {
        Self {
            min: self.min + space,
            max: self.max + space,
        }
    }

    /// An exact offset, where the minimum and maximum coincide.
    const fn exact(offset: u32) -> Self {
        Self {
            min: offset,
            max: offset,
        }
    }

    /// Shift this offset by another, widening the range to cover both.
    ///
    /// Saturating, because this is used to turn section-relative symbol positions into absolute
    /// ones for range checks, where a saturated bound just means "too far to reach".
    fn add(self, rhs: Self) -> Self {
        Self {
            min: self.min.saturating_add(rhs.min),
            max: self.max.saturating_add(rhs.max),
        }
    }

    /// Shift this offset by another, failing if either bound leaves the address space.
    fn checked_add(self, rhs: Self) -> Option<Self> {
        Some(Self {
            min: self.min.checked_add(rhs.min)?,
            max: self.max.checked_add(rhs.max)?,
        })
    }

    /// Round both bounds up to a multiple of `rhs`, failing if either leaves the address space.
    ///
    /// An `rhs` of `0` or `1` leaves the offset alone.
    fn checked_next_multiple_of(self, rhs: u32) -> Option<Self> {
        if rhs <= 1 {
            return Some(self);
        }
        Some(Self {
            min: self.min.checked_next_multiple_of(rhs)?,
            max: self.max.checked_next_multiple_of(rhs)?,
        })
    }
}

struct Section {
    /// Running position of the end of the section, relative to its own base.
    offset: UnresolvedOffset,
    data: Vec<u8>,
    events: Vec<Event>,
}

enum Event {
    Data(u32),
    Symbol(LabelId),
    LocalLabel(u32),
    Resolve(RelocationItem),
    Pad(u32),
    Align(u32),
}
impl Event {
    fn min_max_size(&self, symbols: &LabelMap<(SectionKind, UnresolvedOffset)>) -> (u32, u32) {
        match self {
            Event::Data(n) | Event::Pad(n) => (*n, *n),
            Event::Symbol(_) | Event::LocalLabel(_) => (0, 0),
            Event::Resolve(item) => item.min_max_size_with_symbols(symbols),
            Event::Align(n) => (0, *n),
        }
    }
}

impl Section {
    fn new() -> Self {
        Self {
            offset: UnresolvedOffset { min: 0, max: 0 },
            data: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn push_le_u32(&mut self, value: u32) {
        self.push_bytes(&value.to_le_bytes());
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        let n = bytes.len() as u32;
        self.data.extend_from_slice(bytes);
        match self.events.last_mut() {
            Some(Event::Data(size)) => *size += n,
            _ => self.events.push(Event::Data(n)),
        }
        self.offset.min += n;
        self.offset.max += n;
    }

    pub fn push_relocation_item(&mut self, item: RelocationItem) {
        let (min, max) = item.min_max_size();
        self.events.push(Event::Resolve(item));
        self.offset.min += min;
        self.offset.max += max;
    }

    fn pad(&mut self, n: u32) {
        if n == 0 {
            return;
        }

        self.events.push(Event::Pad(n));
        self.offset = self.offset.pad(n);
    }

    fn align(&mut self, n: u32) {
        if n <= 1 {
            return;
        }

        self.events.push(Event::Align(n));
        self.offset.max = self.offset.max.saturating_add(n);
    }
}

enum LocalLabelDirection {
    Forward,
    Backward,
}
enum TargetLabel {
    Local(u32, LocalLabelDirection),
    Global(LabelId),
}

type MnemonicHandler = fn(&mut Assembler, &mut SourceCursor) -> AssembleResult<()>;

impl Assembler {
    pub fn new(isa: Isa, positions: SectionPositions) -> Self {
        let mut handlers = hashbrown::HashMap::default();

        _ = fill_handlers(isa, &mut handlers);

        Self {
            label_table: LabelTable::new(),
            symbols: LabelMap::default(),

            text: Section::new(),
            data: Section::new(),
            rodata: Section::new(),
            bss: Section::new(),

            positions,
            current_section: SectionKind::Text,

            isa,
            handlers,
        }
    }

    pub fn with_source(mut self, content: &str) -> AssembleResult<Self> {
        self.add_source(content)?;
        Ok(self)
    }

    pub fn add_source(&mut self, content: &str) -> AssembleResult<&mut Self> {
        let mut cursor = SourceCursor::new(content);
        while !cursor.is_empty() {
            cursor.trim_wsc();

            // Whitespace at the end is fine.
            if cursor.is_empty() {
                break;
            }

            if cursor.peek_byte().is_some_and(|b| b.is_ascii_digit()) {
                // Local label definition
                let label = cursor.take_u32()?;
                if cursor.next_byte() != Some(b':') {
                    return Err(Box::new(cursor.new_err("expected semicolon")));
                }
                self.current_section_mut()
                    .events
                    .push(Event::LocalLabel(label));
                continue;
            }

            let label = cursor.take_ident()?;
            if cursor.next_byte_if_equals(b':') {
                // Label definition
                let label = self.label_table.get_or_insert(label);
                let kind = self.current_section;
                let position = self.current_section().offset;
                self.symbols.set(label, (kind, position));
                self.current_section_mut().events.push(Event::Symbol(label));
                continue;
            }

            match label {
                ".section" => {
                    cursor.take_separator()?;
                    let section = cursor.take_ident()?;
                    match section {
                        ".text" => self.current_section = SectionKind::Text,
                        ".data" => self.current_section = SectionKind::Data,
                        ".rodata" => self.current_section = SectionKind::ReadOnlyData,
                        ".bss" => self.current_section = SectionKind::Bss,
                        _ => return Err(Box::new(cursor.new_err("unknown section"))),
                    }
                }
                ".global" | ".globl" => {
                    cursor.take_separator()?;
                    let name = cursor.take_ident()?;

                    let kind = self.current_section;
                    let position = self.current_section().offset;
                    let name = self.label_table.get_or_insert(name);
                    self.symbols.set(name, (kind, position));
                }
                ".align" => {
                    cursor.take_separator()?;
                    let align = cursor.take_uimm32()?;
                    self.current_section_mut().align(align);
                }

                ".space" => {
                    cursor.take_separator()?;
                    let space = cursor.take_uimm32()?;
                    self.current_section_mut().pad(space);
                }

                ".insn" => self.parse_insn(&mut cursor)?,
                ".byte" => self.parse_data_directive(&mut cursor, 1)?,
                ".half" | ".short" | ".2byte" => self.parse_data_directive(&mut cursor, 2)?,
                ".word" | ".long" | ".4byte" => self.parse_data_directive(&mut cursor, 4)?,

                ".text" => self.current_section = SectionKind::Text,
                ".data" => self.current_section = SectionKind::Data,
                ".rodata" => self.current_section = SectionKind::ReadOnlyData,
                ".bss" => self.current_section = SectionKind::Bss,

                _ => {
                    let Some(handler) = self.handlers.get(label) else {
                        return Err(Box::new(cursor.new_err("unknown mnemonic or directive")));
                    };
                    if self.current_section == SectionKind::Bss {
                        return Err(Box::new(cursor.new_err(
                            "'.bss' is zero-initialised and cannot hold instructions",
                        )));
                    }
                    (handler)(self, &mut cursor)?;
                }
            }
            cursor.take_stmt_end()?;
        }

        Ok(self)
    }

    fn parse_data_directive(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        width: usize,
    ) -> AssembleResult<()> {
        cursor.take_separator()?;
        loop {
            let start = cursor.offset;
            let value = cursor.take_data_value(width)?;
            if self.current_section == SectionKind::Bss {
                // `.bss` holds no bytes, only its size.
                if value != 0 {
                    cursor.offset = start;
                    return Err(Box::new(cursor.new_err(
                        "'.bss' is zero-initialised and cannot hold a non-zero value; \
                         move it to '.data' or use '.space'",
                    )));
                }
                self.bss.pad(width as u32);
            } else {
                let bytes = value.to_le_bytes();
                self.current_section_mut().push_bytes(&bytes[..width]);
            }

            cursor.take_opt_separator();
            if cursor.peek_byte() != Some(b',') {
                return Ok(());
            }
            cursor.take_comma_separator()?;
        }
    }

    fn parse_itype_instr_base<'a, T>(
        &mut self,
        cursor: &mut SourceCursor<'a>,
        new: impl Fn(XRegIdent, XRegIdent, T) -> u32,
        imm: impl Fn(&mut SourceCursor<'a>) -> AssembleResult<T>,
    ) -> AssembleResult<()> {
        cursor.take_separator()?;
        let rd = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let rs = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let imm = imm(cursor)?;
        self.current_section_mut().push_le_u32(new(rd, rs, imm));
        Ok(())
    }
    fn parse_itype_instr(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        new: fn(XRegIdent, XRegIdent, i16) -> u32,
    ) -> AssembleResult<()> {
        self.parse_itype_instr_base(cursor, new, |cursor| cursor.take_imm12())
    }
    fn parse_itype_shamt_xlen_instr(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        new: fn(XRegIdent, XRegIdent, u8) -> u32,
    ) -> AssembleResult<()> {
        let xlen = self.isa.xlen;
        self.parse_itype_instr_base(cursor, new, |cursor| {
            let shamt = cursor.take_shamt()?;
            // @TODO: Throw error.
            let shamt = match xlen {
                XLen::Rv32 => shamt & 0x1F,
                XLen::Rv64 => shamt & 0x3F,
            };
            Ok(shamt)
        })
    }
    fn parse_itype_shamt5_instr(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        new: fn(XRegIdent, XRegIdent, u8) -> u32,
    ) -> AssembleResult<()> {
        self.parse_itype_instr_base(cursor, new, |cursor| Ok(cursor.take_shamt()? & 0x1F))
    }

    fn parse_rtype_instr(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        new: fn(XRegIdent, XRegIdent, XRegIdent) -> u32,
    ) -> AssembleResult<()> {
        cursor.take_separator()?;
        let rd = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let rs1 = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let rs2 = cursor.take_gp_register()?;
        self.current_section_mut().push_le_u32(new(rd, rs1, rs2));
        Ok(())
    }

    fn parse_2op_branch(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        kind: BranchKind,
        swap_operands: bool,
    ) -> AssembleResult<()> {
        cursor.take_separator()?;
        let rs1 = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let rs2 = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let label = self.take_target_label(cursor)?;

        let (rs1, rs2) = if swap_operands {
            (rs2, rs1)
        } else {
            (rs1, rs2)
        };
        self.current_section_mut()
            .push_relocation_item(RelocationItem::Branch(rs1, rs2, kind, label));
        Ok(())
    }
    fn parse_zero_op_branch(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        kind: BranchKind,
        operand_is_left: bool,
    ) -> AssembleResult<()> {
        cursor.take_separator()?;
        let rs = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let label = self.take_target_label(cursor)?;

        let (rs1, rs2) = if operand_is_left {
            (rs, XRegIdent::Zero)
        } else {
            (XRegIdent::Zero, rs)
        };
        self.current_section_mut()
            .push_relocation_item(RelocationItem::Branch(rs1, rs2, kind, label));
        Ok(())
    }

    fn parse_load_op(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        new: fn(XRegIdent, XRegIdent, i16) -> u32,
    ) -> AssembleResult<()> {
        cursor.take_separator()?;
        let rd = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let imm = cursor.take_imm12()?;
        cursor.take_opt_separator();
        cursor.expect_byte(b'(')?;
        cursor.take_opt_separator();
        let rs = cursor.take_gp_register()?;
        cursor.take_opt_separator();
        cursor.expect_byte(b')')?;

        self.current_section_mut().push_le_u32(new(rd, rs, imm));
        Ok(())
    }
    fn parse_store_op(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        new: fn(XRegIdent, XRegIdent, i16) -> u32,
    ) -> AssembleResult<()> {
        cursor.take_separator()?;
        let rs2 = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let imm = cursor.take_imm12()?;
        cursor.take_opt_separator();
        cursor.expect_byte(b'(')?;
        cursor.take_opt_separator();
        let rs1 = cursor.take_gp_register()?;
        cursor.take_opt_separator();
        cursor.expect_byte(b')')?;

        self.current_section_mut().push_le_u32(new(rs1, rs2, imm));
        Ok(())
    }

    fn parse_rd_rs(
        &mut self,
        cursor: &mut SourceCursor<'_>,
        new: impl Fn(XRegIdent, XRegIdent) -> u32,
    ) -> AssembleResult<()> {
        cursor.take_separator()?;
        let rd = cursor.take_gp_register()?;
        cursor.take_comma_separator()?;
        let rs = cursor.take_gp_register()?;

        self.current_section_mut().push_le_u32(new(rd, rs));
        Ok(())
    }
    fn parse_fence_order(&mut self, cursor: &mut SourceCursor<'_>) -> AssembleResult<FenceOrder> {
        if cursor.peek_byte().is_some_and(|b| b == b'0') {
            cursor.offset += 1;
            return Ok(FenceOrder::empty());
        }

        let mut order = FenceOrder::empty();
        while let Some(b) = cursor.peek_byte() {
            match b {
                b'i' => order |= FenceOrder::DEVICE_INPUT,
                b'o' => order |= FenceOrder::DEVICE_OUTPUT,
                b'r' => order |= FenceOrder::MEMORY_READ,
                b'w' => order |= FenceOrder::MEMORY_WRITE,
                b' ' | b'\t' | b'\n' | b',' => break,
                _ => return Err(Box::new(cursor.new_err("invalid fence order"))),
            }
            cursor.offset += 1;
        }
        Ok(order)
    }

    /// Resolve the source into section data and addresses.
    ///
    /// # Errors
    ///
    /// Fails if the section placements cannot be resolved into addresses; see [`LayoutError`].
    pub fn assemble(mut self) -> Result<Assembled, LayoutError> {
        let mut out_symbols = LabelMap::with_len(self.symbols.len());

        let positions = self.positions;
        let order = positions.layout_order()?;

        // Sizes as they stand after parsing: exact where nothing can be relaxed, a range
        // otherwise. `Section::offset` is this accumulation, kept up to date as events are
        // pushed.
        let size_estimates: [UnresolvedOffset; 4] =
            SectionKind::ALL.map(|kind| self.section(kind).offset);

        // Conservative bases, used while relaxing for references into sections that have not
        // been laid out yet.
        let mut bases = resolve_bases(&positions, &order, &size_estimates)?;

        // Link and relax.
        let mut local_offsets = [const { Vec::<u32>::new() }; 4];
        let mut reloc_variants = [const { Vec::<u8>::new() }; 4];
        let mut offsets = [0u32; 4];
        let mut start_offsets = [0u32; 4];
        let mut sizes = size_estimates;

        // Laying sections out in placement order means every section a base is measured from has
        // already been walked, so its size is exact by the time it is needed.
        let sections = [&self.text, &self.data, &self.rodata, &self.bss];
        for &kind in &order {
            let i = kind.index();
            let section = sections[i];

            let base = resolve_base(&positions, kind, &bases, &sizes)?;
            // Whatever this base is measured from has already been walked, so it is exact and
            // the section lands on a single address.
            debug_assert_eq!(base.min, base.max, "{kind} base should be exact");
            bases[i] = base;

            let start_offset = base.min;
            let mut local_offset = Vec::new();
            let mut reloc_variant = Vec::new();
            let mut offset = start_offset;
            start_offsets[i] = start_offset;

            for (eid, event) in section.events.iter().enumerate() {
                match &event {
                    Event::Data(size) => offset += *size,
                    Event::Pad(padding) => offset += *padding,
                    Event::Align(n) => offset = offset.next_multiple_of(*n),

                    Event::Symbol(label) => {
                        let rel = offset - start_offset;
                        self.symbols[*label] =
                            Some((kind, UnresolvedOffset { min: rel, max: rel }));
                        out_symbols.set(*label, (kind, offset));
                    }
                    Event::LocalLabel(_) => local_offset.push(offset),

                    Event::Resolve(item) => {
                        let size = match item {
                            RelocationItem::LoadAddress(..) => 8,
                            RelocationItem::Compressed(..) => 2,
                            RelocationItem::Insn(jump, _) => jump.format.size(),
                            RelocationItem::Branch(_, _, _, lbl) => {
                                let lbl_offset = resolve_min_max_target_label(
                                    lbl,
                                    offset,
                                    &section.events,
                                    eid,
                                    &self.symbols,
                                    &bases,
                                );
                                let is_in_branch_range = in_imm12_range(lbl_offset.min, offset)
                                    && in_imm12_range(lbl_offset.max, offset);
                                reloc_variant.push(u8::from(is_in_branch_range));
                                if is_in_branch_range { 4 } else { 12 }
                            }
                            RelocationItem::Jump(_, lbl) => {
                                let lbl_offset = resolve_min_max_target_label(
                                    lbl,
                                    offset,
                                    &section.events,
                                    eid,
                                    &self.symbols,
                                    &bases,
                                );
                                let is_in_jump_range = in_imm20_range(lbl_offset.min, offset)
                                    && in_imm20_range(lbl_offset.max, offset);
                                reloc_variant.push(u8::from(is_in_jump_range));
                                if is_in_jump_range { 4 } else { 8 }
                            }
                            RelocationItem::PcRelJump(_, _, lbl) => {
                                let lbl_offset = resolve_min_max_target_label(
                                    lbl,
                                    offset,
                                    &section.events,
                                    eid,
                                    &self.symbols,
                                    &bases,
                                );
                                let is_in_jump_range = in_imm20_range(lbl_offset.min, offset)
                                    && in_imm20_range(lbl_offset.max, offset);
                                reloc_variant.push(u8::from(is_in_jump_range));
                                if is_in_jump_range { 4 } else { 8 }
                            }
                        };
                        offset += size;
                    }
                };
            }

            local_offsets[i] = local_offset;
            reloc_variants[i] = reloc_variant;
            offsets[i] = offset;
            // Relaxation and alignment are settled for this section, so its size is now exact
            // and any section placed after it lands on a single address.
            sizes[i] = UnresolvedOffset::exact(offset - start_offset);
        }

        let mut outs: [Box<[u8]>; 4] = std::array::from_fn(|_| [].into());
        for (i, section) in [&self.text, &self.data, &self.rodata, &self.bss]
            .iter()
            .enumerate()
        {
            let local_offsets = &local_offsets[i];
            let reloc_variants = &reloc_variants[i];
            let end_offset = offsets[i];
            let start_offset = start_offsets[i];
            let mut local_offset = 0usize;
            let mut reloc_offset = 0usize;
            let mut out = Vec::<u8>::with_capacity((end_offset - start_offset) as usize);
            let mut data_ptr = 0u32;
            for (eid, event) in section.events.iter().enumerate() {
                let offset = start_offset + out.len() as u32;
                match &event {
                    Event::Data(size) => {
                        out.extend_from_slice(&section.data[data_ptr as usize..][..*size as usize]);
                        data_ptr += *size;
                    }
                    Event::Pad(padding) => {
                        out.resize(out.len() + *padding as usize, 0);
                    }
                    Event::Align(n) => {
                        let padding = offset.next_multiple_of(*n) - offset;
                        out.resize(out.len() + padding as usize, 0);
                    }

                    Event::Symbol(_) => {}
                    Event::LocalLabel(_) => local_offset += 1,

                    Event::Resolve(item) => match item {
                        RelocationItem::LoadAddress(rd, lbl) => {
                            let lbl_offset = resolve_target_label(
                                lbl,
                                &section.events,
                                local_offsets,
                                eid,
                                local_offset,
                                &out_symbols,
                            );
                            // `addi` sign-extends its immediate, so round the
                            // upper part up when bit 11 is set.
                            out.extend_from_slice(
                                &encoding::Lui::new(*rd, lbl_offset.wrapping_add(0x800) & !0xFFF)
                                    .encode_as_u32()
                                    .to_le_bytes(),
                            );
                            out.extend_from_slice(
                                &encoding::Addi::new(*rd, *rd, (lbl_offset & 0xFFF) as i16)
                                    .encode_as_u32()
                                    .to_le_bytes(),
                            );
                        }
                        RelocationItem::Branch(rs1, rs2, kind, lbl) => {
                            let lbl_offset = resolve_target_label(
                                lbl,
                                &section.events,
                                local_offsets,
                                eid,
                                local_offset,
                                &out_symbols,
                            );

                            let is_in_branch_range = reloc_variants[reloc_offset] != 0;
                            reloc_offset += 1;
                            if is_in_branch_range {
                                assert!(in_imm12_range(lbl_offset, offset));
                                let imm = lbl_offset
                                    .checked_signed_diff(offset)
                                    .expect("Invariant: should be in range")
                                    as i16;

                                let instr = match kind {
                                    BranchKind::Eq => {
                                        encoding::Beq::new(*rs1, *rs2, imm).encode_as_u32()
                                    }
                                    BranchKind::NotEq => {
                                        encoding::Bne::new(*rs1, *rs2, imm).encode_as_u32()
                                    }
                                    BranchKind::Lt => {
                                        encoding::Blt::new(*rs1, *rs2, imm).encode_as_u32()
                                    }
                                    BranchKind::Ge => {
                                        encoding::Bge::new(*rs1, *rs2, imm).encode_as_u32()
                                    }
                                    BranchKind::Ltu => {
                                        encoding::Bltu::new(*rs1, *rs2, imm).encode_as_u32()
                                    }
                                    BranchKind::Geu => {
                                        encoding::Bgeu::new(*rs1, *rs2, imm).encode_as_u32()
                                    }
                                };

                                out.extend_from_slice(&instr.to_le_bytes());
                            } else {
                                todo!();
                            }
                        }
                        RelocationItem::Jump(rd, lbl) => {
                            let lbl_offset = resolve_target_label(
                                lbl,
                                &section.events,
                                local_offsets,
                                eid,
                                local_offset,
                                &out_symbols,
                            );
                            let is_in_jump_range = reloc_variants[reloc_offset] != 0;
                            reloc_offset += 1;
                            if is_in_jump_range {
                                assert!(in_imm20_range(lbl_offset, offset));
                                let imm = lbl_offset
                                    .checked_signed_diff(offset)
                                    .expect("Invariant: should be in range");
                                out.extend_from_slice(
                                    &encoding::Jal::new(*rd, imm).encode_as_u32().to_le_bytes(),
                                );
                            } else {
                                todo!();
                            }
                        }
                        RelocationItem::Compressed(jump, lbl) => {
                            let lbl_offset = resolve_target_label(
                                lbl,
                                &section.events,
                                local_offsets,
                                eid,
                                local_offset,
                                &out_symbols,
                            );
                            let distance = i64::from(lbl_offset) - i64::from(offset);
                            let (min, max) = jump.range();
                            if !(i64::from(min)..=i64::from(max)).contains(&distance)
                                || distance % 2 != 0
                            {
                                return Err(LayoutError::TargetOutOfRange {
                                    mnemonic: jump.mnemonic(),
                                    offset: distance,
                                    min,
                                    max,
                                });
                            }
                            out.extend_from_slice(&jump.encode(distance as i16).to_le_bytes());
                        }
                        RelocationItem::Insn(jump, lbl) => {
                            let lbl_offset = resolve_target_label(
                                lbl,
                                &section.events,
                                local_offsets,
                                eid,
                                local_offset,
                                &out_symbols,
                            );
                            let distance = i64::from(lbl_offset) - i64::from(offset);
                            let (min, max) = jump.format.range();
                            if !(i64::from(min)..=i64::from(max)).contains(&distance)
                                || distance % 2 != 0
                            {
                                return Err(LayoutError::TargetOutOfRange {
                                    mnemonic: jump.format.mnemonic(),
                                    offset: distance,
                                    min,
                                    max,
                                });
                            }
                            let bits = jump.template | jump.format.immediate(distance as i32);
                            let size = jump.format.size() as usize;
                            out.extend_from_slice(&bits.to_le_bytes()[..size]);
                        }
                        RelocationItem::PcRelJump(rd, _rt, lbl) => {
                            // @Q? Should `rt` be set anyway?
                            let lbl_offset = resolve_target_label(
                                lbl,
                                &section.events,
                                local_offsets,
                                eid,
                                local_offset,
                                &out_symbols,
                            );
                            let is_in_jump_range = reloc_variants[reloc_offset] != 0;
                            reloc_offset += 1;
                            let imm = lbl_offset
                                .checked_signed_diff(offset)
                                .expect("Invariant: should be in range");
                            if is_in_jump_range {
                                assert!(in_imm20_range(lbl_offset, offset));
                                out.extend_from_slice(
                                    &encoding::Jal::new(*rd, imm).encode_as_u32().to_le_bytes(),
                                );
                            } else {
                                todo!()
                            }
                        }
                    },
                };
            }
            outs[i] = out.into_boxed_slice();
        }

        let [text, data, rodata, bss] = outs;
        let [text_start, data_start, rodata_start, bss_start] = start_offsets;
        Ok(Assembled {
            labels: self.label_table,
            symbols: out_symbols,

            text_start,
            data_start,
            rodata_start,
            bss_start,

            text,
            data,
            rodata,
            bss: bss.len(),
        })
    }

    fn section(&self, kind: SectionKind) -> &Section {
        match kind {
            SectionKind::Text => &self.text,
            SectionKind::Data => &self.data,
            SectionKind::ReadOnlyData => &self.rodata,
            SectionKind::Bss => &self.bss,
        }
    }
    fn section_mut(&mut self, kind: SectionKind) -> &mut Section {
        match kind {
            SectionKind::Text => &mut self.text,
            SectionKind::Data => &mut self.data,
            SectionKind::ReadOnlyData => &mut self.rodata,
            SectionKind::Bss => &mut self.bss,
        }
    }
    fn current_section(&self) -> &Section {
        self.section(self.current_section)
    }
    fn current_section_mut(&mut self) -> &mut Section {
        self.section_mut(self.current_section)
    }

    fn take_target_label(&mut self, cursor: &mut SourceCursor<'_>) -> AssembleResult<TargetLabel> {
        let Some(b) = cursor.peek_byte() else {
            return Err(Box::new(cursor.new_err("expected target label")));
        };

        if b.is_ascii_digit() {
            let label = cursor.take_u32()?;
            let direction = match cursor.next_byte() {
                Some(b'f') => LocalLabelDirection::Forward,
                Some(b'b') => LocalLabelDirection::Backward,
                _ => return Err(Box::new(cursor.new_err("expected f/b for local label"))),
            };
            Ok(TargetLabel::Local(label, direction))
        } else {
            let label = cursor.take_ident()?;
            let label = self.label_table.get_or_insert(label);
            Ok(TargetLabel::Global(label))
        }
    }
}

fn fill_handlers(
    isa: Isa,
    handlers: &mut hashbrown::HashMap<&'static str, MnemonicHandler, RandomState>,
) -> Result<(), Ext> {
    macro_rules! handler {
        ($fn_name:ident, |$asm:ident, $cursor:ident| $handle:expr) => {{
            fn $fn_name(
                $asm: &mut Assembler,
                $cursor: &mut SourceCursor<'_>,
            ) -> AssembleResult<()> {
                $handle
                Ok(())
            }
            $fn_name
        }};
        ($fn_name:ident, @load $name:ty) => {
            handler!($fn_name, |asm, c| {
                asm.parse_load_op(c, |rd, rs, imm| <$name>::new(rd, rs, imm).encode_as_u32())?;
            })
        };
        ($fn_name:ident, @store $name:ty) => {
            handler!($fn_name, |asm, c| {
                asm.parse_store_op(c, |rd, rs, imm| <$name>::new(rd, rs, imm).encode_as_u32())?;
            })
        };
        ($fn_name:ident, @itype $name:ty) => {
            handler!($fn_name, |asm, c| {
                asm.parse_itype_instr(c, |rd, rs1, rs2| <$name>::new(rd, rs1, rs2).encode_as_u32())?;
            })
        };
        ($fn_name:ident, @itype_shamt5 $name:ty) => {
            handler!($fn_name, |asm, c| {
                asm.parse_itype_shamt5_instr(c, |rd, rs1, rs2| {
                    <$name>::new(rd, rs1, rs2).encode_as_u32()
                })?;
            })
        };
        ($fn_name:ident, @itype_shamt_xlen $name:ty) => {
            handler!($fn_name, |asm, c| {
                asm.parse_itype_shamt_xlen_instr(c, |rd, rs1, rs2| {
                    <$name>::new(rd, rs1, rs2).encode_as_u32()
                })?;
            })
        };
        ($fn_name:ident, @rtype $name:ty) => {
            handler!($fn_name, |asm, c| {
                asm.parse_rtype_instr(c, |rd, rs1, rs2| <$name>::new(rd, rs1, rs2).encode_as_u32())?;
            })
        };
        ($fn_name:ident, @rd_rs |$rd:ident, $rs:ident| $encode:expr) => {
            handler!($fn_name, |asm, c| {
                asm.parse_rd_rs(c, |$rd, $rs| $encode)?;
            })
        };
        ($fn_name:ident, @branch2 $kind:expr, $swap:expr) => {
            handler!($fn_name, |asm, c| { asm.parse_2op_branch(c, $kind, $swap)?; })
        };
        ($fn_name:ident, @branch_zero $kind:expr, $swap:expr) => {
            handler!($fn_name, |asm, c| {
                asm.parse_zero_op_branch(c, $kind, $swap)?;
            })
        };
    }

    handlers.clear();

    handlers.insert(
        "la",
        handler!(la, |asm, c| {
            c.take_separator()?;
            let rd = c.take_gp_register()?;
            c.take_comma_separator()?;
            let label = asm.take_target_label(c)?;
            asm.current_section_mut()
                .push_relocation_item(RelocationItem::LoadAddress(rd, label));
        }),
    );
    handlers.insert(
        "li",
        handler!(la, |asm, c| {
            c.take_separator()?;
            let rd = c.take_gp_register()?;
            c.take_comma_separator()?;
            let imm = c.take_signed_imm32()?;
            let imm = match asm.isa.xlen {
                XLen::Rv32 => i64::from(imm as u32 as i32),
                XLen::Rv64 => imm,
            };

            if let Some(imm) = signed_imm_to_imm12(imm) {
                asm.current_section_mut()
                    .push_le_u32(encoding::Addi::new(rd, XRegIdent::Zero, imm).encode_as_u32());
            } else {
                let imm = (imm as u64 & 0xFFFF_FFFF) as u32;
                // @TODO: Better instruction dispatch.
                // `addi` sign-extends its immediate, so round the upper part up
                // when bit 11 is set.
                asm.current_section_mut().push_le_u32(
                    encoding::Lui::new(rd, imm.wrapping_add(0x800) & !0xFFF).encode_as_u32(),
                );
                asm.current_section_mut()
                    .push_le_u32(encoding::Addi::new(rd, rd, (imm & 0xFFF) as i16).encode_as_u32());
            }
        }),
    );
    handlers.insert(
        "lui",
        handler!(lui, |asm, c| {
            c.take_separator()?;
            let rd = c.take_gp_register()?;
            c.take_comma_separator()?;
            let imm = c.take_imm20()? << 12;
            asm.current_section_mut()
                .push_le_u32(encoding::Lui::new(rd, imm).encode_as_u32());
        }),
    );
    handlers.insert(
        "auipc",
        handler!(auipc, |asm, c| {
            c.take_separator()?;
            let rd = c.take_gp_register()?;
            c.take_comma_separator()?;
            let imm = c.take_imm20()? << 12;
            asm.current_section_mut()
                .push_le_u32(encoding::Auipc::new(rd, imm).encode_as_u32());
        }),
    );
    handlers.insert(
        "jal",
        handler!(jal, |asm, c| {
            c.take_separator()?;
            let reg = if let Some(ident) = c.peek_ident()
                && let Some(reg) = parse_gp_register(ident)
            {
                c.offset += ident.len();
                c.take_comma_separator()?;
                reg
            } else {
                // jal   label  ~  jal x1, label
                XRegIdent::Ra
            };
            let label = asm.take_target_label(c)?;
            asm.current_section_mut()
                .push_relocation_item(RelocationItem::Jump(reg, label));
        }),
    );
    handlers.insert(
        "j",
        handler!(j, |asm, c| {
            // j label      ~     jal x0, label
            c.take_separator()?;
            let label = asm.take_target_label(c)?;
            asm.current_section_mut()
                .push_relocation_item(RelocationItem::Jump(XRegIdent::Zero, label));
        }),
    );
    handlers.insert(
        "jalr",
        handler!(jalr, |asm, c| {
            c.take_separator()?;
            let rd_or_rs = c.take_gp_register()?;
            c.take_opt_separator();
            let (rd, rs, imm) = if c.peek_byte().is_some_and(|b| b == b',') {
                c.take_comma_separator()?;
                let rs = c.take_gp_register()?;
                c.take_comma_separator()?;
                let imm = c.take_imm12()?;
                (rd_or_rs, rs, imm)
            } else {
                // jalr rs       ~     jalr x1, rs, 0
                (XRegIdent::Ra, rd_or_rs, 0)
            };
            asm.current_section_mut()
                .push_le_u32(encoding::Jalr::new(rd, rs, imm).encode_as_u32());
        }),
    );
    handlers.insert(
        "jr",
        handler!(jr, |asm, c| {
            // jr rs     ~     jalr x0, rs, 0
            c.take_separator()?;
            let rs = c.take_gp_register()?;
            asm.current_section_mut()
                .push_le_u32(encoding::Jalr::new(XRegIdent::Zero, rs, 0).encode_as_u32());
        }),
    );
    handlers.insert(
        "ret",
        handler!(ret, |asm, _c| {
            // ret     ~     jalr x0, x1, 0
            asm.current_section_mut().push_le_u32(
                encoding::Jalr::new(XRegIdent::Zero, XRegIdent::Ra, 0).encode_as_u32(),
            );
        }),
    );
    handlers.insert(
        "call",
        handler!(call, |asm, c| {
            // call symbol    ~      auipc ra, %hi_symbol(symbol)
            //                       jalr  ra, ra, %lo_symbol(symbol)
            c.take_separator()?;
            let label = asm.take_target_label(c)?;
            asm.current_section_mut()
                .push_relocation_item(RelocationItem::PcRelJump(
                    XRegIdent::Ra,
                    XRegIdent::Ra,
                    label,
                ));
        }),
    );
    handlers.insert(
        "tail",
        handler!(tail, |asm, c| {
            // tail symbol    ~      auipc t1, %hi_symbol(symbol)
            //                       jalr  t1, x0, %lo_symbol(symbol)
            c.take_separator()?;
            let label = asm.take_target_label(c)?;
            asm.current_section_mut()
                .push_relocation_item(RelocationItem::PcRelJump(
                    XRegIdent::T0,
                    XRegIdent::Zero,
                    label,
                ));
        }),
    );
    handlers.insert(
        "jump",
        handler!(jump, |asm, c| {
            // jump symbol, rt    ~     auipc rt, %hi_symbol(symbol)
            //                          jalr  rt, x0, %lo_symbol(symbol)
            c.take_separator()?;
            let label = asm.take_target_label(c)?;
            c.take_comma_separator()?;
            let rt = c.take_gp_register()?;
            asm.current_section_mut()
                .push_relocation_item(RelocationItem::PcRelJump(rt, XRegIdent::Zero, label));
        }),
    );
    handlers.insert("beq", handler!(beq, @branch2 BranchKind::Eq, false));
    handlers.insert("bne", handler!(bne, @branch2 BranchKind::NotEq, false));
    handlers.insert("blt", handler!(blt, @branch2 BranchKind::Lt, false));
    handlers.insert("bgt", handler!(bgt, @branch2 BranchKind::Lt, true));
    handlers.insert("ble", handler!(ble, @branch2 BranchKind::Ge, true));
    handlers.insert("bge", handler!(bge, @branch2 BranchKind::Ge, false));
    handlers.insert("bltu", handler!(bltu, @branch2 BranchKind::Ltu, false));
    handlers.insert("bgtu", handler!(bgtu, @branch2 BranchKind::Ltu, true));
    handlers.insert("bgeu", handler!(bgeu, @branch2 BranchKind::Geu, false));
    handlers.insert("bleu", handler!(bleu, @branch2 BranchKind::Geu, true));

    handlers.insert("beqz", handler!(beqz, @branch_zero BranchKind::Eq, false));
    handlers.insert(
        "bnez",
        handler!(bnez, @branch_zero BranchKind::NotEq, false),
    );
    handlers.insert("bltz", handler!(bltz, @branch_zero BranchKind::Lt, false));
    handlers.insert("bgtz", handler!(bgtz, @branch_zero BranchKind::Lt, true));
    handlers.insert("bgez", handler!(bgez, @branch_zero BranchKind::Ge, false));
    handlers.insert("blez", handler!(blez, @branch_zero BranchKind::Ge, true));

    handlers.insert("lb", handler!(lb, @load encoding::Lb));
    handlers.insert("lh", handler!(lh, @load encoding::Lb));
    handlers.insert("lw", handler!(lw, @load encoding::Lw));
    handlers.insert("lbu", handler!(lbu, @load encoding::Lbu));
    handlers.insert("lhu", handler!(lhu, @load encoding::Lhu));

    handlers.insert("sb", handler!(sb, @store encoding::Sb));
    handlers.insert("sh", handler!(sh, @store encoding::Sh));
    handlers.insert("sw", handler!(sw, @store encoding::Sw));

    handlers.insert(
        "nop",
        handler!(nop, |asm, _c| {
            asm.current_section_mut().push_le_u32(
                encoding::Addi::new(XRegIdent::Zero, XRegIdent::Zero, 0).encode_as_u32(),
            );
        }),
    );
    handlers.insert(
        "mv",
        handler!(mv, @rd_rs |rd, rs| encoding::Addi::new(rd, rs, 0).encode_as_u32()),
    );
    handlers.insert(
        "not",
        handler!(not, @rd_rs |rd, rs| encoding::Xori::new(rd, rs, -1).encode_as_u32()),
    );
    handlers.insert(
        "neg",
        handler!(neg, @rd_rs |rd, rs| {
            encoding::Sub::new(rd, XRegIdent::Zero, rs).encode_as_u32()
        }),
    );

    handlers.insert("addi", handler!(addi, @itype encoding::Addi));
    handlers.insert("slti", handler!(slti, @itype encoding::Slti));
    handlers.insert(
        "sltiu",
        handler!(sltiu, |asm, c| {
            asm.parse_itype_instr_base(
                c,
                |rd, rs, imm| encoding::Sltiu::new(rd, rs, imm).encode_as_u32(),
                // Sign-extended like any I-type immediate, only the comparison is unsigned.
                |c| c.take_imm12().map(|imm| i32::from(imm) as u32),
            )?;
        }),
    );
    handlers.insert("xori", handler!(xori, @itype encoding::Xori));
    handlers.insert("ori", handler!(ori, @itype encoding::Ori));
    handlers.insert("andi", handler!(andi, @itype encoding::Andi));
    handlers.insert("slli", handler!(slli, @itype_shamt_xlen encoding::Slli));
    handlers.insert("srli", handler!(srli, @itype_shamt_xlen encoding::Srli));
    handlers.insert("srai", handler!(srai, @itype_shamt_xlen encoding::Srai));

    handlers.insert("add", handler!(add, @rtype encoding::Add));
    handlers.insert("sub", handler!(sub, @rtype encoding::Sub));
    handlers.insert("sll", handler!(sll, @rtype encoding::Sll));
    handlers.insert("srl", handler!(srl, @rtype encoding::Srl));
    handlers.insert("sra", handler!(sra, @rtype encoding::Sra));
    handlers.insert("slt", handler!(slt, @rtype encoding::Slt));
    handlers.insert("sltu", handler!(sltu, @rtype encoding::Sltu));
    handlers.insert("xor", handler!(xor, @rtype encoding::Xor));
    handlers.insert("and", handler!(and, @rtype encoding::And));
    handlers.insert("or", handler!(or, @rtype encoding::Or));

    handlers.insert(
        "fence",
        handler!(fence, |asm, c| {
            c.take_opt_separator();

            let (pred, succ) = if c
                .peek_byte()
                .is_some_and(|b| matches!(b, b'i' | b'o' | b'r' | b'w'))
            {
                let pred = asm.parse_fence_order(c)?;
                c.take_comma_separator()?;
                let succ = asm.parse_fence_order(c)?;
                (pred, succ)
            } else {
                (FenceOrder::ALL, FenceOrder::ALL)
            };

            asm.current_section_mut()
                .push_le_u32(encoding::Fence::new(FenceMode(0b0000), pred, succ).encode_as_u32());
        }),
    );
    handlers.insert(
        "fence.tso",
        handler!(fence_tso, |asm, c| {
            c.take_opt_separator();

            let (pred, succ) = if c
                .peek_byte()
                .is_some_and(|b| matches!(b, b'i' | b'o' | b'r' | b'w'))
            {
                let pred = asm.parse_fence_order(c)?;
                c.take_comma_separator()?;
                let succ = asm.parse_fence_order(c)?;
                (pred, succ)
            } else {
                (FenceOrder::ALL, FenceOrder::ALL)
            };

            asm.current_section_mut()
                .push_le_u32(encoding::Fence::new(FenceMode(0b1000), pred, succ).encode_as_u32());
        }),
    );
    handlers.insert(
        "pause",
        handler!(pause, |asm, _c| {
            asm.current_section_mut().push_le_u32(
                encoding::Fence::new(
                    FenceMode(0b0000),
                    FenceOrder::MEMORY_WRITE,
                    FenceOrder::empty(),
                )
                .encode_as_u32(),
            );
        }),
    );
    handlers.insert(
        "ecall",
        handler!(ecall, |asm, _c| {
            asm.current_section_mut()
                .push_le_u32(encoding::Ecall::new().encode_as_u32());
        }),
    );
    handlers.insert(
        "ebreak",
        handler!(ebreak, |asm, _c| {
            asm.current_section_mut()
                .push_le_u32(encoding::Ebreak::new().encode_as_u32());
        }),
    );

    if isa.xlen == XLen::Rv64 {
        handlers.insert("lwu", handler!(lwu, @load encoding::Lwu));
        handlers.insert("ld", handler!(ld, @load encoding::Ld));
        handlers.insert("sd", handler!(sd, @store encoding::Sd));

        handlers.insert("addiw", handler!(addiw, @itype encoding::Addiw));
        handlers.insert("slliw", handler!(slliw, @itype_shamt5 encoding::Slliw));
        handlers.insert("srliw", handler!(srliw, @itype_shamt5 encoding::Srliw));
        handlers.insert("sraiw", handler!(sraiw, @itype_shamt5 encoding::Sraiw));

        handlers.insert("addw", handler!(addw, @rtype encoding::Addw));
        handlers.insert("subw", handler!(subw, @rtype encoding::Subw));
        handlers.insert("sllw", handler!(sllw, @rtype encoding::Sllw));
        handlers.insert("srlw", handler!(srlw, @rtype encoding::Srlw));
        handlers.insert("sraw", handler!(sraw, @rtype encoding::Sraw));

        handlers.insert(
            "negw",
            handler!(negw, @rd_rs |rd, rs| {
                encoding::Subw::new(rd, XRegIdent::Zero, rs).encode_as_u32()
            }),
        );
        handlers.insert(
            "sext.w",
            handler!(sext_w, @rd_rs |rd, rs| encoding::Addiw::new(rd, rs, 0).encode_as_u32()),
        );
    }

    use isa::Ext;
    for ext in isa.exts {
        match ext {
            Ext::Atomic => return Err(ext),
            Ext::BitManipulation => return Err(ext),
            Ext::Compressed => {
                // Only the instructions as written: plain instructions are
                // never compressed behind the programmer's back. The
                // floating-point ones (`c.flw`, ...) need F, which is not
                // supported.
                handlers.insert(
                    "c.addi4spn",
                    handler!(c_addi4spn, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_c_register()?;
                        c.take_comma_separator()?;
                        c.expect_gp_register(XRegIdent::Sp)?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm_scaled(0..=1020, 4, true, "4..=1020")?;
                        push_compressed(
                            asm,
                            encoding::CAddi4SpN::new(rd, imm as u32).encode_as_u32(),
                        );
                    }),
                );
                handlers.insert(
                    "c.lw",
                    handler!(c_lw, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_c_register()?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm_scaled(0..=124, 4, false, "0..=124")?;
                        let rs1 = take_c_base(c)?;
                        push_compressed(
                            asm,
                            encoding::CLw::new(rd, rs1, imm as u8).encode_as_u32(),
                        );
                    }),
                );
                handlers.insert(
                    "c.sw",
                    handler!(c_sw, |asm, c| {
                        c.take_separator()?;
                        let rs2 = c.take_c_register()?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm_scaled(0..=124, 4, false, "0..=124")?;
                        let rs1 = take_c_base(c)?;
                        push_compressed(
                            asm,
                            encoding::CSw::new(rs1, rs2, imm as u8).encode_as_u32(),
                        );
                    }),
                );
                handlers.insert(
                    "c.nop",
                    handler!(c_nop, |asm, _c| {
                        push_compressed(asm, encoding::CNop::new().encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.addi",
                    handler!(c_addi, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_gp_register_except(&[])?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm_scaled(-32..=31, 1, true, "-32..=31")?;
                        push_compressed(asm, encoding::CAddi::new(rd, imm as i8).encode_as_u32());
                    }),
                );
                if isa.xlen == XLen::Rv32 {
                    // On RV64, this encoding is `c.addiw`.
                    handlers.insert(
                        "c.jal",
                        handler!(c_jal, |asm, c| {
                            c.take_separator()?;
                            let label = asm.take_target_label(c)?;
                            asm.current_section_mut().push_relocation_item(
                                RelocationItem::Compressed(CompressedJump::Jal, label),
                            );
                        }),
                    );
                }
                handlers.insert(
                    "c.li",
                    handler!(c_li, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_gp_register_except(&[])?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm_scaled(-32..=31, 1, false, "-32..=31")?;
                        push_compressed(asm, encoding::CLi::new(rd, imm as i8).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.addi16sp",
                    handler!(c_addi16sp, |asm, c| {
                        c.take_separator()?;
                        c.expect_gp_register(XRegIdent::Sp)?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm_scaled(-512..=496, 16, true, "-512..=496")?;
                        push_compressed(asm, encoding::CAddi16Sp::new(imm as i16).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.lui",
                    handler!(c_lui, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_gp_register_except(&[XRegIdent::Sp])?;
                        c.take_comma_separator()?;
                        // The upper 20 bits, as for `lui`, of which only the
                        // sign-extended lowest 6 are encoded.
                        let start = c.offset;
                        let upper = c.take_int_in(0..=0xF_FFFF, "1..=31 or 0xfffe0..=0xfffff")?;
                        let upper = match upper {
                            1..=31 => upper,
                            0xF_FFE0..=0xF_FFFF => upper - 0x10_0000,
                            _ => {
                                c.offset = start;
                                return Err(Box::new(c.new_err(format!(
                                    "immediate {upper} is out of range, expected 1..=31 or \
                                     0xfffe0..=0xfffff"
                                ))));
                            }
                        };
                        push_compressed(
                            asm,
                            encoding::CLui::new(rd, (upper as i32) << 12).encode_as_u32(),
                        );
                    }),
                );
                handlers.insert(
                    "c.srli",
                    handler!(c_srli, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_c_register()?;
                        c.take_comma_separator()?;
                        let shamt = take_c_shamt(asm, c)?;
                        push_compressed(asm, encoding::CSrli::new(rd, shamt).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.srai",
                    handler!(c_srai, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_c_register()?;
                        c.take_comma_separator()?;
                        let shamt = take_c_shamt(asm, c)?;
                        push_compressed(asm, encoding::CSrai::new(rd, shamt).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.andi",
                    handler!(c_andi, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_c_register()?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm_scaled(-32..=31, 1, false, "-32..=31")?;
                        push_compressed(asm, encoding::CAndi::new(rd, imm as i8).encode_as_u32());
                    }),
                );
                macro_rules! c_arith {
                    ($mnemonic:literal, $fn_name:ident, $ty:ty) => {
                        handlers.insert(
                            $mnemonic,
                            handler!($fn_name, |asm, c| {
                                c.take_separator()?;
                                let rd = c.take_c_register()?;
                                c.take_comma_separator()?;
                                let rs2 = c.take_c_register()?;
                                push_compressed(asm, <$ty>::new(rd, rs2).encode_as_u32());
                            }),
                        );
                    };
                }
                c_arith!("c.sub", c_sub, encoding::CSub);
                c_arith!("c.xor", c_xor, encoding::CXor);
                c_arith!("c.or", c_or, encoding::COr);
                c_arith!("c.and", c_and, encoding::CAnd);
                handlers.insert(
                    "c.j",
                    handler!(c_j, |asm, c| {
                        c.take_separator()?;
                        let label = asm.take_target_label(c)?;
                        asm.current_section_mut()
                            .push_relocation_item(RelocationItem::Compressed(
                                CompressedJump::J,
                                label,
                            ));
                    }),
                );
                handlers.insert(
                    "c.beqz",
                    handler!(c_beqz, |asm, c| {
                        c.take_separator()?;
                        let rs1 = c.take_c_register()?;
                        c.take_comma_separator()?;
                        let label = asm.take_target_label(c)?;
                        asm.current_section_mut()
                            .push_relocation_item(RelocationItem::Compressed(
                                CompressedJump::Beqz(rs1),
                                label,
                            ));
                    }),
                );
                handlers.insert(
                    "c.bnez",
                    handler!(c_bnez, |asm, c| {
                        c.take_separator()?;
                        let rs1 = c.take_c_register()?;
                        c.take_comma_separator()?;
                        let label = asm.take_target_label(c)?;
                        asm.current_section_mut()
                            .push_relocation_item(RelocationItem::Compressed(
                                CompressedJump::Bnez(rs1),
                                label,
                            ));
                    }),
                );
                handlers.insert(
                    "c.slli",
                    handler!(c_slli, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_gp_register_except(&[])?;
                        c.take_comma_separator()?;
                        let shamt = take_c_shamt(asm, c)?;
                        push_compressed(asm, encoding::CSlli::new(rd, shamt).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.lwsp",
                    handler!(c_lwsp, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_gp_register_except(&[])?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm_scaled(0..=252, 4, false, "0..=252")?;
                        take_sp_base(c)?;
                        push_compressed(asm, encoding::CLwSp::new(rd, imm as u8).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.swsp",
                    handler!(c_swsp, |asm, c| {
                        c.take_separator()?;
                        let rs2 = c.take_gp_register()?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm_scaled(0..=252, 4, false, "0..=252")?;
                        take_sp_base(c)?;
                        push_compressed(asm, encoding::CSwSp::new(rs2, imm as u8).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.jr",
                    handler!(c_jr, |asm, c| {
                        c.take_separator()?;
                        let rs1 = c.take_gp_register_except(&[])?;
                        push_compressed(asm, encoding::CJr::new(rs1).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.jalr",
                    handler!(c_jalr, |asm, c| {
                        c.take_separator()?;
                        let rs1 = c.take_gp_register_except(&[])?;
                        push_compressed(asm, encoding::CJalr::new(rs1).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.mv",
                    handler!(c_mv, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_gp_register_except(&[])?;
                        c.take_comma_separator()?;
                        let rs2 = c.take_gp_register_except(&[])?;
                        push_compressed(asm, encoding::CMv::new(rd, rs2).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.add",
                    handler!(c_add, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_gp_register_except(&[])?;
                        c.take_comma_separator()?;
                        let rs2 = c.take_gp_register_except(&[])?;
                        push_compressed(asm, encoding::CAdd::new(rd, rs2).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "c.ebreak",
                    handler!(c_ebreak, |asm, _c| {
                        push_compressed(asm, encoding::CEBreak::new().encode_as_u32());
                    }),
                );
            }
            Ext::DoublePrecisionFp => return Err(ext),
            Ext::Rv32E => return Err(ext),
            Ext::SinglePrecisionFp => {
                // `name rd, rs1[, rs2[, rs3]][, rm]`: each operand read by its
                // own cursor method, so that it is a general-purpose or a
                // floating-point register as the instruction has it, and a
                // rounding mode last when the instruction has one.
                macro_rules! fp {
                    (
                        $mnemonic:literal, $fn_name:ident, $ty:ty,
                        [$first:ident $(, $rest:ident)*] $($rm:ident)?
                    ) => {
                        handlers.insert(
                            $mnemonic,
                            handler!($fn_name, |asm, c| {
                                c.take_separator()?;
                                let instr = <$ty>::new(
                                    c.$first()?,
                                    $({
                                        c.take_comma_separator()?;
                                        c.$rest()?
                                    },)*
                                    $(c.$rm()?)?
                                );
                                asm.current_section_mut().push_le_u32(instr.encode_as_u32());
                            }),
                        );
                    };
                }
                use encoding as e;

                handlers.insert(
                    "flw",
                    handler!(flw, |asm, c| {
                        c.take_separator()?;
                        let rd = c.take_fp_register()?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm12()?;
                        let rs1 = take_base(c)?;
                        asm.current_section_mut()
                            .push_le_u32(e::Flw::new(rd, rs1, imm).encode_as_u32());
                    }),
                );
                handlers.insert(
                    "fsw",
                    handler!(fsw, |asm, c| {
                        c.take_separator()?;
                        let rs2 = c.take_fp_register()?;
                        c.take_comma_separator()?;
                        let imm = c.take_imm12()?;
                        let rs1 = take_base(c)?;
                        asm.current_section_mut()
                            .push_le_u32(e::Fsw::new(rs1, rs2, imm).encode_as_u32());
                    }),
                );

                fp!("fmadd.s", fmadd_s, e::FmaddS,
                    [take_fp_register, take_fp_register, take_fp_register, take_fp_register]
                    take_rounding_mode);
                fp!("fmsub.s", fmsub_s, e::FmsubS,
                    [take_fp_register, take_fp_register, take_fp_register, take_fp_register]
                    take_rounding_mode);
                fp!("fnmsub.s", fnmsub_s, e::FnmsubS,
                    [take_fp_register, take_fp_register, take_fp_register, take_fp_register]
                    take_rounding_mode);
                fp!("fnmadd.s", fnmadd_s, e::FnmaddS,
                    [take_fp_register, take_fp_register, take_fp_register, take_fp_register]
                    take_rounding_mode);

                fp!("fadd.s", fadd_s, e::FaddS,
                    [take_fp_register, take_fp_register, take_fp_register] take_rounding_mode);
                fp!("fsub.s", fsub_s, e::FsubS,
                    [take_fp_register, take_fp_register, take_fp_register] take_rounding_mode);
                fp!("fmul.s", fmul_s, e::FmulS,
                    [take_fp_register, take_fp_register, take_fp_register] take_rounding_mode);
                fp!("fdiv.s", fdiv_s, e::FdivS,
                    [take_fp_register, take_fp_register, take_fp_register] take_rounding_mode);
                fp!("fsqrt.s", fsqrt_s, e::FsqrtS,
                    [take_fp_register, take_fp_register] take_rounding_mode);

                fp!(
                    "fsgnj.s",
                    fsgnj_s,
                    e::FsgnjS,
                    [take_fp_register, take_fp_register, take_fp_register]
                );
                fp!(
                    "fsgnjn.s",
                    fsgnjn_s,
                    e::FsgnjnS,
                    [take_fp_register, take_fp_register, take_fp_register]
                );
                fp!(
                    "fsgnjx.s",
                    fsgnjx_s,
                    e::FsgnjxS,
                    [take_fp_register, take_fp_register, take_fp_register]
                );
                fp!(
                    "fmin.s",
                    fmin_s,
                    e::FminS,
                    [take_fp_register, take_fp_register, take_fp_register]
                );
                fp!(
                    "fmax.s",
                    fmax_s,
                    e::FmaxS,
                    [take_fp_register, take_fp_register, take_fp_register]
                );

                fp!("fcvt.w.s", fcvt_w_s, e::FcvtWS,
                    [take_gp_register, take_fp_register] take_rounding_mode);
                fp!("fcvt.wu.s", fcvt_wu_s, e::FcvtWuS,
                    [take_gp_register, take_fp_register] take_rounding_mode);
                fp!("fcvt.s.w", fcvt_s_w, e::FcvtSW,
                    [take_fp_register, take_gp_register] take_rounding_mode);
                fp!("fcvt.s.wu", fcvt_s_wu, e::FcvtSWu,
                    [take_fp_register, take_gp_register] take_rounding_mode);

                // `fmv.x.s` and `fmv.s.x` are the names from before version
                // 2.2 of the F extension, which assemblers still accept.
                fp!(
                    "fmv.x.w",
                    fmv_x_w,
                    e::FmvXW,
                    [take_gp_register, take_fp_register]
                );
                fp!(
                    "fmv.x.s",
                    fmv_x_s,
                    e::FmvXW,
                    [take_gp_register, take_fp_register]
                );
                fp!(
                    "fmv.w.x",
                    fmv_w_x,
                    e::FmvWX,
                    [take_fp_register, take_gp_register]
                );
                fp!(
                    "fmv.s.x",
                    fmv_s_x,
                    e::FmvWX,
                    [take_fp_register, take_gp_register]
                );

                fp!(
                    "feq.s",
                    feq_s,
                    e::FeqS,
                    [take_gp_register, take_fp_register, take_fp_register]
                );
                fp!(
                    "flt.s",
                    flt_s,
                    e::FltS,
                    [take_gp_register, take_fp_register, take_fp_register]
                );
                fp!(
                    "fle.s",
                    fle_s,
                    e::FleS,
                    [take_gp_register, take_fp_register, take_fp_register]
                );
                fp!(
                    "fclass.s",
                    fclass_s,
                    e::FclassS,
                    [take_gp_register, take_fp_register]
                );

                // The sign-injection pseudo-instructions, with `rs` as both
                // sources.
                macro_rules! fp_sign {
                    ($mnemonic:literal, $fn_name:ident, $ty:ty) => {
                        handlers.insert(
                            $mnemonic,
                            handler!($fn_name, |asm, c| {
                                c.take_separator()?;
                                let rd = c.take_fp_register()?;
                                c.take_comma_separator()?;
                                let rs = c.take_fp_register()?;
                                asm.current_section_mut()
                                    .push_le_u32(<$ty>::new(rd, rs, rs).encode_as_u32());
                            }),
                        );
                    };
                }
                fp_sign!("fmv.s", fmv_s, e::FsgnjS);
                fp_sign!("fneg.s", fneg_s, e::FsgnjnS);
                fp_sign!("fabs.s", fabs_s, e::FsgnjxS);

                // On RV64, these encodings are `c.ld`, `c.sd`, `c.ldsp` and
                // `c.sdsp`.
                if isa.exts.contains(isa::ExtensionSet::COMPRESSED) && isa.xlen == XLen::Rv32 {
                    handlers.insert(
                        "c.flw",
                        handler!(c_flw, |asm, c| {
                            c.take_separator()?;
                            let rd = c.take_c_fp_register()?;
                            c.take_comma_separator()?;
                            let imm = c.take_imm_scaled(0..=124, 4, false, "0..=124")?;
                            let rs1 = take_c_base(c)?;
                            push_compressed(asm, e::CFlw::new(rd, rs1, imm as u8).encode_as_u32());
                        }),
                    );
                    handlers.insert(
                        "c.fsw",
                        handler!(c_fsw, |asm, c| {
                            c.take_separator()?;
                            let rs2 = c.take_c_fp_register()?;
                            c.take_comma_separator()?;
                            let imm = c.take_imm_scaled(0..=124, 4, false, "0..=124")?;
                            let rs1 = take_c_base(c)?;
                            push_compressed(asm, e::CFsw::new(rs1, rs2, imm as u8).encode_as_u32());
                        }),
                    );
                    handlers.insert(
                        "c.flwsp",
                        handler!(c_flwsp, |asm, c| {
                            c.take_separator()?;
                            let rd = c.take_fp_register()?;
                            c.take_comma_separator()?;
                            let imm = c.take_imm_scaled(0..=252, 4, false, "0..=252")?;
                            take_sp_base(c)?;
                            push_compressed(asm, e::CFlwSp::new(rd, imm as u8).encode_as_u32());
                        }),
                    );
                    handlers.insert(
                        "c.fswsp",
                        handler!(c_fswsp, |asm, c| {
                            c.take_separator()?;
                            let rs2 = c.take_fp_register()?;
                            c.take_comma_separator()?;
                            let imm = c.take_imm_scaled(0..=252, 4, false, "0..=252")?;
                            take_sp_base(c)?;
                            push_compressed(asm, e::CFswSp::new(rs2, imm as u8).encode_as_u32());
                        }),
                    );
                }
            }
            Ext::Hypervisor => return Err(ext),
            Ext::IntegerMuldiv => {
                handlers.insert("mul", handler!(mul, @rtype encoding::Mul));
                handlers.insert("mulh", handler!(mul, @rtype encoding::MulH));
                handlers.insert("mulhsu", handler!(mul, @rtype encoding::MulHsu));
                handlers.insert("mulhu", handler!(mul, @rtype encoding::MulHu));
                handlers.insert("div", handler!(rem, @rtype encoding::Div));
                handlers.insert("divu", handler!(rem, @rtype encoding::DivU));
                handlers.insert("rem", handler!(rem, @rtype encoding::Rem));
                handlers.insert("remu", handler!(rem, @rtype encoding::RemU));
            }
            Ext::PackedSimd => return Err(ext),
            Ext::QuadPrecisionFp => return Err(ext),
            Ext::SupervisorMode => return Err(ext),
            Ext::UserMode => return Err(ext),
            Ext::Vector => return Err(ext),
        }
    }
    Ok(())
}

/// Appends a 2-byte compressed instruction to the current section.
fn push_compressed(asm: &mut Assembler, bits: u32) {
    asm.current_section_mut()
        .push_bytes(&(bits as u16).to_le_bytes());
}

/// The `(rs1)` base of a load or store.
fn take_base(c: &mut SourceCursor<'_>) -> AssembleResult<XRegIdent> {
    c.take_opt_separator();
    c.expect_byte(b'(')?;
    c.take_opt_separator();
    let rs1 = c.take_gp_register()?;
    c.take_opt_separator();
    c.expect_byte(b')')?;
    Ok(rs1)
}

/// The `(rs1')` base of a compressed load or store.
fn take_c_base(c: &mut SourceCursor<'_>) -> AssembleResult<CXRegIdent> {
    c.take_opt_separator();
    c.expect_byte(b'(')?;
    c.take_opt_separator();
    let rs1 = c.take_c_register()?;
    c.take_opt_separator();
    c.expect_byte(b')')?;
    Ok(rs1)
}

/// The `(sp)` base of a compressed stack-pointer load or store.
fn take_sp_base(c: &mut SourceCursor<'_>) -> AssembleResult<()> {
    c.take_opt_separator();
    c.expect_byte(b'(')?;
    c.take_opt_separator();
    c.expect_gp_register(XRegIdent::Sp)?;
    c.take_opt_separator();
    c.expect_byte(b')')
}

/// A shift amount of a compressed shift. Zero is a hint, not a shift, and on
/// RV32 the amount has to fit in 5 bits.
fn take_c_shamt(asm: &Assembler, c: &mut SourceCursor<'_>) -> AssembleResult<u8> {
    let (max, expected) = match asm.isa.xlen {
        XLen::Rv32 => (31, "1..=31"),
        XLen::Rv64 => (63, "1..=63"),
    };
    Ok(c.take_imm_scaled(0..=max, 1, true, expected)? as u8)
}

fn signed_imm_to_imm12(imm: i64) -> Option<i16> {
    if !(-(1i64 << 11)..(1i64 << 11)).contains(&imm) {
        return None;
    }
    Some(imm as i16)
}

/// Resolve the base address of a single section.
///
/// `bases` and `sizes` must already hold resolved entries for whatever section this one is
/// placed after, which is what laying sections out in [`SectionPositions::layout_order`] order
/// guarantees.
fn resolve_base(
    positions: &SectionPositions,
    kind: SectionKind,
    bases: &[UnresolvedOffset; 4],
    sizes: &[UnresolvedOffset; 4],
) -> Result<UnresolvedOffset, LayoutError> {
    match positions.placement(kind) {
        SectionPlacement::Absolute(addr) => Ok(UnresolvedOffset::exact(addr)),
        SectionPlacement::After {
            section,
            gap,
            align,
        } => {
            let after = section.index();
            bases[after]
                .checked_add(sizes[after])
                .and_then(|end| end.checked_add(UnresolvedOffset::exact(gap)))
                .and_then(|end| end.checked_next_multiple_of(align))
                .ok_or(LayoutError::AddressOverflow { section: kind })
        }
    }
}

/// Resolve every section's base address, given each section's size.
///
/// Also rejects layouts where a section's *end* leaves the address space, so that a caller which
/// passes the widest possible sizes can afterwards walk each section without checking for
/// overflow on every event.
fn resolve_bases(
    positions: &SectionPositions,
    order: &[SectionKind; 4],
    sizes: &[UnresolvedOffset; 4],
) -> Result<[UnresolvedOffset; 4], LayoutError> {
    let mut bases = [UnresolvedOffset::default(); 4];
    for &kind in order {
        let base = resolve_base(positions, kind, &bases, sizes)?;
        base.checked_add(sizes[kind.index()])
            .ok_or(LayoutError::AddressOverflow { section: kind })?;
        bases[kind.index()] = base;
    }
    Ok(bases)
}

fn resolve_min_max_target_label(
    lbl: &TargetLabel,
    offset: u32,
    events: &[Event],
    eid: usize,
    symbols: &LabelMap<(SectionKind, UnresolvedOffset)>,
    bases: &[UnresolvedOffset; 4],
) -> UnresolvedOffset {
    match lbl {
        TargetLabel::Local(lbl, dir) => {
            let mut tgt = UnresolvedOffset {
                min: offset,
                max: offset,
            };

            match dir {
                LocalLabelDirection::Backward => {
                    for event in events[..eid].iter().rev() {
                        if matches!(event, Event::LocalLabel(elbl) if elbl == lbl) {
                            return tgt;
                        }

                        let (min_size, max_size) = event.min_max_size(symbols);
                        tgt.min -= max_size;
                        tgt.max -= min_size;
                    }
                }
                LocalLabelDirection::Forward => {
                    for event in events[eid..].iter() {
                        if matches!(event, Event::LocalLabel(elbl) if elbl == lbl) {
                            return tgt;
                        }

                        let (min_size, max_size) = event.min_max_size(symbols);
                        tgt.min += min_size;
                        tgt.max += max_size;
                    }
                }
            }

            panic!("local label not found");
        }
        TargetLabel::Global(lbl) => {
            // Recorded positions are relative to the symbol's own section, so lift them into
            // absolute addresses with that section's base.
            let (kind, rel) = symbols[*lbl].expect("global label not found");
            bases[kind.index()].add(rel)
        }
    }
}

fn resolve_target_label(
    lbl: &TargetLabel,
    events: &[Event],
    local_offsets: &[u32],
    eid: usize,
    mut local_offset: usize,
    symbols: &LabelMap<(SectionKind, u32)>,
) -> u32 {
    match lbl {
        TargetLabel::Local(lbl, dir) => {
            match dir {
                LocalLabelDirection::Backward => {
                    for event in events[..eid].iter().rev() {
                        if let Event::LocalLabel(elbl) = event {
                            local_offset -= 1;
                            if elbl == lbl {
                                return local_offsets[local_offset];
                            }
                        }
                    }
                }
                LocalLabelDirection::Forward => {
                    for event in &events[eid..] {
                        if let Event::LocalLabel(elbl) = event {
                            if elbl == lbl {
                                return local_offsets[local_offset];
                            }
                            local_offset += 1;
                        }
                    }
                }
            }

            panic!("local label not found");
        }
        TargetLabel::Global(lbl) => symbols[*lbl].unwrap().1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::{ExtensionSet, XLen};

    const RV32IM: Isa = Isa {
        exts: ExtensionSet::INTEGER_MULDIV,
        xlen: XLen::Rv32,
    };

    fn assemble(positions: &SectionPositions, source: &str) -> Assembled {
        Assembler::new(RV32IM, *positions)
            .with_source(source)
            .expect("source should assemble")
            .assemble()
            .expect("sections should lay out")
    }

    /// Resolve a global symbol by name into the section it lives in and its absolute address.
    fn symbol(asm: &mut Assembled, name: &str) -> (SectionKind, u32) {
        let id = asm.labels.get_or_insert(name);
        asm.symbols[id].expect("symbol should be defined")
    }

    /// A symbol reports the section it was actually defined in, not always `.text`.
    #[test]
    fn symbols_carry_their_own_section() {
        let mut asm = assemble(
            &SectionPositions::absolute(0x8000_0000, 0x8000_0100, 0x8000_0200, 0x8000_0300),
            "
            .section .text
            start:
                li a0, 1
                ecall
            .section .data
            d0:
                .space 4
            d1:
            .section .rodata
            r0:
            .section .bss
                .space 8
            b0:
            ",
        );

        assert_eq!(symbol(&mut asm, "start"), (SectionKind::Text, 0x8000_0000));
        assert_eq!(symbol(&mut asm, "d0"), (SectionKind::Data, 0x8000_0100));
        assert_eq!(symbol(&mut asm, "d1"), (SectionKind::Data, 0x8000_0104));
        assert_eq!(
            symbol(&mut asm, "r0"),
            (SectionKind::ReadOnlyData, 0x8000_0200)
        );
        assert_eq!(symbol(&mut asm, "b0"), (SectionKind::Bss, 0x8000_0308));
    }

    /// `.align` rounds up the *absolute* address, so it has to account for a section base that is
    /// not itself aligned.
    #[test]
    fn align_rounds_the_absolute_address() {
        let mut asm = assemble(
            &SectionPositions::absolute(0x8000_0000, 0x8000_0101, 0x8000_0200, 0x8000_0300),
            "
            .section .data
                .space 1
                .align 4
            aligned:
                .space 1
            ",
        );

        assert_eq!(
            symbol(&mut asm, "aligned"),
            (SectionKind::Data, 0x8000_0104)
        );
        // 1 byte of `.space`, 2 bytes of alignment padding, 1 byte of `.space`.
        assert_eq!(asm.data.len(), 4);
    }

    /// `.align 0` and `.align 1` are no-ops rather than a division by zero.
    #[test]
    fn degenerate_aligns_are_noops() {
        let mut asm = assemble(
            &SectionPositions::absolute(0x8000_0000, 0x8000_0101, 0x8000_0200, 0x8000_0300),
            "
            .section .data
                .space 1
                .align 0
                .align 1
            here:
            ",
        );

        assert_eq!(symbol(&mut asm, "here"), (SectionKind::Data, 0x8000_0102));
    }
    /// `.bss` after `.data`, aligned to 4 bytes - the motivating case.
    #[test]
    fn bss_follows_data_aligned() {
        let positions = SectionPositions {
            text: SectionPlacement::Absolute(0x8000_0000),
            data: SectionPlacement::Absolute(0x8000_0100),
            rodata: SectionPlacement::after(SectionKind::Text),
            bss: SectionPlacement::after_aligned(SectionKind::Data, 4),
        };

        let asm = assemble(
            &positions,
            "
            .section .text
                li a0, 1
                ecall
            .section .data
                .space 5
            .section .bss
                .space 8
            ",
        );

        assert_eq!(asm.text_start, 0x8000_0000);
        assert_eq!(asm.data_start, 0x8000_0100);
        // `.text` is 8 bytes, so `.rodata` sits right behind it.
        assert_eq!(asm.rodata_start, 0x8000_0008);
        // `.data` ends at 0x80000105, rounded up to a 4-byte boundary.
        assert_eq!(asm.bss_start, 0x8000_0108);
        assert_eq!(asm.bss, 8);
    }

    /// A chain of relative placements resolves transitively, in one pass.
    #[test]
    fn placements_chain() {
        let positions = SectionPositions {
            text: SectionPlacement::Absolute(0x1000),
            data: SectionPlacement::after(SectionKind::Text),
            rodata: SectionPlacement::after_aligned(SectionKind::Data, 16),
            bss: SectionPlacement::after_aligned(SectionKind::ReadOnlyData, 8),
        };

        let asm = assemble(
            &positions,
            "
            .section .text
                li a0, 1
            .section .data
                .space 1
            .section .rodata
                .space 1
            ",
        );

        assert_eq!(asm.text_start, 0x1000);
        // `li a0, 1` is one instruction here, so `.text` is 4 bytes.
        assert_eq!(asm.data_start, 0x1004);
        // `.data` ends at 0x1005, rounded up to 16.
        assert_eq!(asm.rodata_start, 0x1010);
        // `.rodata` ends at 0x1011, rounded up to 8.
        assert_eq!(asm.bss_start, 0x1018);
    }

    /// A `gap` is inserted before the alignment is applied.
    #[test]
    fn gap_is_applied_before_alignment() {
        let positions = SectionPositions {
            text: SectionPlacement::Absolute(0x1000),
            data: SectionPlacement::After {
                section: SectionKind::Text,
                gap: 5,
                align: 8,
            },
            rodata: SectionPlacement::Absolute(0x4000),
            bss: SectionPlacement::Absolute(0x5000),
        };

        let asm = assemble(&positions, ".section .text\n li a0, 1\n");

        // `.text` ends at 0x1004, plus a 5-byte gap is 0x1009, rounded up to 8 is 0x1010.
        assert_eq!(asm.data_start, 0x1010);
    }

    /// An empty section still has an address, and whatever follows it is not displaced.
    #[test]
    fn empty_sections_do_not_displace() {
        let positions = SectionPositions {
            text: SectionPlacement::Absolute(0x2000),
            data: SectionPlacement::after(SectionKind::Text),
            rodata: SectionPlacement::after(SectionKind::Data),
            bss: SectionPlacement::after(SectionKind::ReadOnlyData),
        };

        let asm = assemble(&positions, ".section .text\n li a0, 1\n");

        assert_eq!(asm.data_start, 0x2004);
        assert_eq!(asm.rodata_start, 0x2004);
        assert_eq!(asm.bss_start, 0x2004);
    }

    /// Placements are resolved in dependency order, not in the order sections are declared in.
    #[test]
    fn placements_resolve_against_declaration_order() {
        let positions = SectionPositions {
            // `.text` trails the data sections.
            text: SectionPlacement::after_aligned(SectionKind::Bss, 4),
            data: SectionPlacement::Absolute(0x3000),
            rodata: SectionPlacement::after(SectionKind::Data),
            bss: SectionPlacement::after(SectionKind::ReadOnlyData),
        };

        let asm = assemble(
            &positions,
            "
            .section .data
                .space 2
            .section .rodata
                .space 3
            .section .bss
                .space 4
            .section .text
                li a0, 1
            ",
        );

        assert_eq!(asm.data_start, 0x3000);
        assert_eq!(asm.rodata_start, 0x3002);
        assert_eq!(asm.bss_start, 0x3005);
        // `.bss` ends at 0x3009, rounded up to 4.
        assert_eq!(asm.text_start, 0x300c);
    }

    /// A relative base that is not aligned is still handled correctly by `.align`.
    #[test]
    fn align_inside_a_relatively_placed_section() {
        let positions = SectionPositions {
            text: SectionPlacement::Absolute(0x1002),
            data: SectionPlacement::after(SectionKind::Text),
            rodata: SectionPlacement::Absolute(0x4000),
            bss: SectionPlacement::Absolute(0x5000),
        };

        let mut asm = assemble(
            &positions,
            "
            .section .text
                li a0, 1
            .section .data
                .space 1
                .align 8
            aligned:
            ",
        );

        // `.text` is 4 bytes from 0x1002, so `.data` starts at an unaligned 0x1006.
        assert_eq!(asm.data_start, 0x1006);
        assert_eq!(symbol(&mut asm, "aligned"), (SectionKind::Data, 0x1008));
    }

    #[test]
    fn layout_order_follows_the_dependency_chain() {
        let positions = SectionPositions {
            text: SectionPlacement::after(SectionKind::Bss),
            data: SectionPlacement::Absolute(0),
            rodata: SectionPlacement::after(SectionKind::Data),
            bss: SectionPlacement::after(SectionKind::ReadOnlyData),
        };

        assert_eq!(
            positions.layout_order().unwrap(),
            [
                SectionKind::Data,
                SectionKind::ReadOnlyData,
                SectionKind::Bss,
                SectionKind::Text,
            ],
        );
    }

    #[test]
    fn a_section_after_itself_is_rejected() {
        let mut positions = SectionPositions::absolute(0, 0, 0, 0);
        positions.bss = SectionPlacement::after(SectionKind::Bss);

        assert_eq!(
            positions.layout_order(),
            Err(LayoutError::SelfReference {
                section: SectionKind::Bss
            }),
        );
    }

    #[test]
    fn a_cycle_of_placements_is_rejected() {
        let mut positions = SectionPositions::absolute(0, 0, 0, 0);
        positions.data = SectionPlacement::after(SectionKind::Bss);
        positions.bss = SectionPlacement::after(SectionKind::ReadOnlyData);
        positions.rodata = SectionPlacement::after(SectionKind::Data);

        let err = positions.layout_order().unwrap_err();
        let LayoutError::Cycle { cycle } = &err else {
            panic!("expected a cycle, got {err}");
        };
        assert_eq!(
            cycle.sections(),
            [
                SectionKind::Data,
                SectionKind::Bss,
                SectionKind::ReadOnlyData
            ],
        );
        // Displaying it closes the loop.
        assert_eq!(
            err.to_string(),
            "section placements form a cycle: .data -> .bss -> .rodata -> .data"
        );
    }

    /// A walk that reaches a cycle through a section outside it reports only the cycle.
    #[test]
    fn a_cycle_is_reported_from_where_it_closes() {
        let mut positions = SectionPositions::absolute(0, 0, 0, 0);
        // `.text` leads into a cycle between `.data` and `.bss` without being part of it.
        positions.text = SectionPlacement::after(SectionKind::Data);
        positions.data = SectionPlacement::after(SectionKind::Bss);
        positions.bss = SectionPlacement::after(SectionKind::Data);

        let err = positions.layout_order().unwrap_err();
        let LayoutError::Cycle { cycle } = &err else {
            panic!("expected a cycle, got {err}");
        };
        assert_eq!(cycle.sections(), [SectionKind::Data, SectionKind::Bss]);
    }

    #[test]
    fn a_layout_past_the_address_space_is_rejected() {
        let positions = SectionPositions {
            text: SectionPlacement::Absolute(0xffff_fff0),
            data: SectionPlacement::after_aligned(SectionKind::Text, 0x1000),
            rodata: SectionPlacement::Absolute(0),
            bss: SectionPlacement::Absolute(0),
        };

        let err = Assembler::new(RV32IM, positions)
            .with_source(".section .text\n li a0, 1\n")
            .unwrap()
            .assemble()
            .err()
            .expect("layout should be rejected");

        assert_eq!(
            err,
            LayoutError::AddressOverflow {
                section: SectionKind::Data
            },
        );
    }

    /// A `.space` that runs off the end of the address space is caught rather than wrapping.
    #[test]
    fn a_section_overrunning_the_address_space_is_rejected() {
        let positions = SectionPositions::absolute(0, 0, 0, 0xffff_fff0);

        let err = Assembler::new(RV32IM, positions)
            .with_source(".section .bss\n .space 64\n")
            .unwrap()
            .assemble()
            .err()
            .expect("layout should be rejected");

        assert_eq!(
            err,
            LayoutError::AddressOverflow {
                section: SectionKind::Bss
            },
        );
    }

    /// Assembles `source` into `.text`, with any error from parsing it.
    fn assemble_text(source: &str) -> AssembleResult<Box<[u8]>> {
        let positions = SectionPositions::absolute(0, 0x400, 0x800, 0xc00);
        let assembled = Assembler::new(RV32IM, positions)
            .with_source(source)?
            .assemble()
            .expect("sections should lay out");
        Ok(assembled.text)
    }

    fn words(source: &str) -> Vec<u32> {
        let text = assemble_text(source).unwrap_or_else(|err| panic!("{source:?}: {err}"));
        text.chunks(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect()
    }

    fn error(source: &str) -> String {
        match assemble_text(source) {
            Ok(text) => panic!("{source:?} assembled to {text:02x?}"),
            Err(err) => err.reason.into_owned(),
        }
    }

    #[test]
    fn signed_immediates() {
        // Checked against GNU as.
        assert_eq!(words("addi sp,sp,-16"), [0xff01_0113]);
        assert_eq!(words("addi a0,zero,-2048"), [0x8000_0513]);
        assert_eq!(words("addi a0,zero,2047"), [0x7ff0_0513]);
        assert_eq!(words("slti a0,a1,-1"), [0xfff5_a513]);
        assert_eq!(words("xori a0,a1,-1"), [0xfff5_c513]);
        assert_eq!(words("lw t0,-4(fp)"), [0xffc4_2283]);
        assert_eq!(words("sw t0,-4(fp)"), [0xfe54_2e23]);
        assert_eq!(words("jalr ra,t0,-8"), [0xff82_80e7]);
        assert_eq!(words("li a0,-1"), [0xfff0_0513]);
        // The same 32 bits as `-1` on RV32.
        assert_eq!(words("li a0,4294967295"), [0xfff0_0513]);
        assert_eq!(words("li a0,4294965248"), [0x8000_0513]);
        // Just past what `addi` sign-extends to.
        assert_eq!(words("li a0,4294965247"), [0xffff_f537, 0x7ff5_0513]);
    }

    #[test]
    fn sltiu_takes_a_sign_extended_immediate() {
        assert_eq!(words("sltiu a0,a1,1"), [0x0015_b513]);
        assert_eq!(words("sltiu a0,a1,-1"), [0xfff5_b513]);
        assert_eq!(
            error("sltiu a0,a1,4095"),
            "immediate 4095 is out of range, expected -2048..=2047"
        );
    }

    #[test]
    fn immediates_out_of_range() {
        assert_eq!(
            error("addi a0,a0,2048"),
            "immediate 2048 is out of range, expected -2048..=2047"
        );
        assert_eq!(
            error("lw a0,-2049(sp)"),
            "immediate -2049 is out of range, expected -2048..=2047"
        );
        assert_eq!(
            error("slli a0,a0,64"),
            "immediate 64 is out of range, expected 0..=63"
        );
        assert_eq!(
            error("slli a0,a0,-1"),
            "immediate -1 is out of range, expected 0..=63"
        );
        assert_eq!(
            error("li a0,-2147483649"),
            "immediate -2147483649 is out of range, expected a 32-bit value"
        );
        assert_eq!(
            error("li a0,4294967296"),
            "immediate 4294967296 is out of range, expected a 32-bit value"
        );
        assert_eq!(error("addi a0,a0,99999999999999999999"), "integer overflow");
    }

    #[test]
    fn hexadecimal_and_binary_immediates() {
        assert_eq!(words("addi a0,zero,0x7ff"), [0x7ff0_0513]);
        assert_eq!(words("addi a0,zero,0X7FF"), [0x7ff0_0513]);
        assert_eq!(words("addi a0,zero,-0x800"), [0x8000_0513]);
        assert_eq!(words("andi a0,a1,0b1010"), [0x00a5_f513]);
        assert_eq!(words("lw t0,-0x4(fp)"), [0xffc4_2283]);
        assert_eq!(words("slli a0,a1,0x1f"), [0x01f5_9513]);
        assert_eq!(words("li a0,0xffffffff"), [0xfff0_0513]);
        // Not octal: a leading zero is just a zero.
        assert_eq!(words("addi a0,zero,010"), [0x00a0_0513]);
        assert_eq!(
            words(".space 0x10\naddi a0,zero,1"),
            [0, 0, 0, 0, 0x0010_0513]
        );
    }

    #[test]
    fn malformed_radix_immediates() {
        assert_eq!(
            error("addi a0,a0,0x"),
            "expected hexadecimal digits after '0x'"
        );
        assert_eq!(
            error("addi a0,a0,-0bx"),
            "expected binary digits after '0b'"
        );
        assert_eq!(error("addi a0,a0,0b2"), "expected binary digits after '0b'");
        assert_eq!(
            error("addi a0,a0,0x800"),
            "immediate 0x800 is out of range, expected -2048..=2047"
        );
        assert_eq!(
            error("li a0,0x100000000"),
            "immediate 0x100000000 is out of range, expected a 32-bit value"
        );
        assert_eq!(error("addi a0,a0,0x1g"), "expected end of statement");
        assert_eq!(
            error(".space -1"),
            "immediate -1 is out of range, expected 0..=4294967295"
        );
    }

    #[test]
    fn upper_immediates_are_the_upper_20_bits() {
        assert_eq!(words("lui a0,0x10"), [0x0001_0537]);
        assert_eq!(words("lui a0,0xfffff"), [0xffff_f537]);
        assert_eq!(words("lui a0,0"), [0x0000_0537]);
        assert_eq!(words("auipc t0,0x12345"), [0x1234_5297]);
        assert_eq!(words("auipc t0,1"), [0x0000_1297]);
        assert_eq!(
            error("lui a0,0x100000"),
            "immediate 0x100000 is out of range, expected 0..=0xfffff"
        );
        assert_eq!(
            error("auipc a0,-1"),
            "immediate -1 is out of range, expected 0..=0xfffff"
        );
    }

    #[test]
    fn data_directives_fill_their_own_section() {
        let asm = assemble(
            &SectionPositions::absolute(0, 0x400, 0x800, 0xc00),
            "
            .section .text
                addi a0,zero,1
                .word 0xdeadbeef
            .section .data
                .byte 1, 0xff, -1
                .half 0x1234, -2
                .word -1,0x01020304
            .section .rodata
                .2byte 7
                .4byte 8
                .short 9
                .long 10
            ",
        );
        assert_eq!(*asm.text, [0x13, 0x05, 0x10, 0x00, 0xef, 0xbe, 0xad, 0xde]);
        assert_eq!(
            *asm.data,
            [
                1, 0xff, 0xff, 0x34, 0x12, 0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 4, 3, 2, 1
            ]
        );
        assert_eq!(*asm.rodata, [7, 0, 8, 0, 0, 0, 9, 0, 10, 0, 0, 0]);
    }

    #[test]
    fn data_is_not_aligned_and_labels_follow_it() {
        let mut asm = assemble(
            &SectionPositions::absolute(0, 0x400, 0x800, 0xc00),
            "
            .section .data
                .byte 1
            w:
                .word 2
                .align 4
            after:
                .half 3
            ",
        );
        assert_eq!(*asm.data, [1, 2, 0, 0, 0, 0, 0, 0, 3, 0]);
        assert_eq!(symbol(&mut asm, "w"), (SectionKind::Data, 0x401));
        assert_eq!(symbol(&mut asm, "after"), (SectionKind::Data, 0x408));
    }

    #[test]
    fn data_directives_in_bss_must_be_zero() {
        let asm = assemble(
            &SectionPositions::absolute(0, 0x400, 0x800, 0xc00),
            ".section .bss\n.word 0, 0\n.byte 0\n.half 0",
        );
        assert_eq!(asm.bss, 11);

        assert_eq!(
            error(".section .bss\n.word 0, 1"),
            "'.bss' is zero-initialised and cannot hold a non-zero value; move it to '.data' or \
             use '.space'"
        );
    }

    #[test]
    fn data_values_must_fit() {
        assert_eq!(words(".byte -128, 255, 0, 0"), [0x0000_ff80]);
        assert_eq!(
            error(".byte 256"),
            "immediate 256 is out of range, expected -128..=255"
        );
        assert_eq!(
            error(".byte -129"),
            "immediate -129 is out of range, expected -128..=255"
        );
        assert_eq!(
            error(".half 0x10000"),
            "immediate 0x10000 is out of range, expected -32768..=65535"
        );
        assert_eq!(
            error(".word 0x100000000"),
            "immediate 0x100000000 is out of range, expected -2147483648..=4294967295"
        );
        assert_eq!(error(".word"), "missing separator");
        assert_eq!(error(".word 1,"), "expected an integer");
        assert_eq!(error(".word 1 2"), "expected end of statement");
    }

    #[test]
    fn instructions_go_in_the_current_section() {
        let asm = assemble(
            &SectionPositions::absolute(0, 0x400, 0x800, 0xc00),
            "
            .section .text
                addi a0,zero,1
            .section .data
                addi a0,zero,2
            .section .text
                addi a0,zero,3
            .section .rodata
                addi a0,zero,4
            ",
        );
        assert_eq!(*asm.text, [0x13, 0x05, 0x10, 0x00, 0x13, 0x05, 0x30, 0x00]);
        assert_eq!(*asm.data, [0x13, 0x05, 0x20, 0x00]);
        assert_eq!(*asm.rodata, [0x13, 0x05, 0x40, 0x00]);
    }

    #[test]
    fn jumps_resolve_outside_text() {
        let asm = assemble(
            &SectionPositions::absolute(0, 0x400, 0x800, 0xc00),
            "
            .section .text
                .word 0
            start:
                ecall
            .section .data
            1:
                beq a0,a1,1b
                jal zero,1b
                bne a0,a1,2f
                jal ra,2f
            2:
                jal zero,start
            ",
        );
        let words: Vec<u32> = asm
            .data
            .chunks(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect();
        assert_eq!(
            words,
            [
                0x00b5_0063, // beq a0,a1,0
                0xffdf_f06f, // jal zero,-4
                0x00b5_1463, // bne a0,a1,8
                0x0040_00ef, // jal ra,4
                0xbf5f_f06f, // jal zero,start (0x4 - 0x410)
            ]
        );
    }

    #[test]
    fn instructions_are_refused_in_bss() {
        assert_eq!(
            error(".section .bss\naddi a0,zero,1"),
            "'.bss' is zero-initialised and cannot hold instructions"
        );
    }

    #[test]
    fn globl_is_global() {
        let mut asm = assemble(
            &SectionPositions::absolute(0, 0x400, 0x800, 0xc00),
            ".section .text\nnop\n.globl _start\n_start:\nnop",
        );
        assert_eq!(symbol(&mut asm, "_start"), (SectionKind::Text, 4));
    }

    #[test]
    fn local_labels_are_not_binary() {
        // `1b` is a backward reference, not a malformed binary number.
        assert_eq!(words("1:\nbeq a0,a1,1b"), [0x00b5_0063]);
    }

    #[test]
    fn immediates_need_digits() {
        assert_eq!(error("addi a0,a0,-"), "expected an integer");
        assert_eq!(error("addi a0,a0,x"), "expected an integer");
        assert_eq!(error("lw a0,(sp)"), "expected an integer");
    }

    #[test]
    fn disassembly_round_trips() {
        for source in [
            "addi sp,sp,-16",
            "lw t0,-4(fp)",
            "sw t0,-2048(fp)",
            "sltiu a0,a1,-1",
            "slti a0,a1,-2048",
            "jalr ra,t0,-8",
            "srai a0,a1,31",
            "lui a0,0x10",
            "lui a0,0xfffff",
            "auipc t0,0x12345",
        ] {
            let text = words(source);
            let printed = encoding::Instruction::decode(&mut &*assemble_text(source).unwrap())
                .unwrap()
                .expect("a valid instruction")
                .to_string();
            assert_eq!(words(&printed), text, "{source:?} printed as {printed:?}");
        }
    }

    const RV32IMC: Isa = Isa {
        exts: ExtensionSet::INTEGER_MULDIV.union(ExtensionSet::COMPRESSED),
        xlen: XLen::Rv32,
    };

    /// Assembles `source` for `isa` into `.text`, with any error, from parsing
    /// or from laying it out, as text.
    fn assemble_for(isa: Isa, source: &str) -> Result<Box<[u8]>, String> {
        let positions = SectionPositions::absolute(0, 0x400, 0x800, 0xc00);
        let asm = Assembler::new(isa, positions)
            .with_source(source)
            .map_err(|err| err.reason.into_owned())?;
        asm.assemble()
            .map(|asm| asm.text)
            .map_err(|err| err.to_string())
    }

    fn halves(source: &str) -> Vec<u16> {
        let text = assemble_for(RV32IMC, source).unwrap_or_else(|err| panic!("{source:?}: {err}"));
        text.chunks(2)
            .map(|h| u16::from_le_bytes(h.try_into().unwrap()))
            .collect()
    }

    fn c_error(source: &str) -> String {
        match assemble_for(RV32IMC, source) {
            Ok(text) => panic!("{source:?} assembled to {text:02x?}"),
            Err(err) => err,
        }
    }

    /// Every compressed integer instruction, with the encodings of an encoder
    /// written separately from the format tables of the specification.
    const COMPRESSED: &[(&str, u16)] = &[
        ("c.addi4spn a0,sp,4", 0x0048),
        ("c.addi4spn a5,sp,1020", 0x1ffc),
        ("c.lw a0,124(a1)", 0x5de8),
        ("c.sw a5,4(s0)", 0xc05c),
        ("c.nop", 0x0001),
        ("c.addi t0,-32", 0x1281),
        ("c.addi t6,31", 0x0ffd),
        ("c.addi sp,-16", 0x1141),
        ("c.li a0,0", 0x4501),
        ("c.li s11,-1", 0x5dfd),
        ("c.addi16sp sp,-512", 0x7101),
        ("c.addi16sp sp,496", 0x617d),
        ("c.addi16sp sp,-64", 0x7139),
        ("c.lui a0,1", 0x6505),
        ("c.lui t0,31", 0x62fd),
        ("c.lui a0,0xfffe0", 0x7501),
        ("c.lui a0,0xfffff", 0x757d),
        ("c.srli a0,1", 0x8105),
        ("c.srai a5,31", 0x87fd),
        ("c.andi s1,-32", 0x9881),
        ("c.andi a0,31", 0x897d),
        ("c.sub a0,a1", 0x8d0d),
        ("c.xor s0,a5", 0x8c3d),
        ("c.or a2,a3", 0x8e55),
        ("c.and s1,s0", 0x8ce1),
        ("c.slli t6,31", 0x0ffe),
        ("c.lwsp ra,252(sp)", 0x50fe),
        ("c.lwsp ra,12(sp)", 0x40b2),
        ("c.lwsp a0,0(sp)", 0x4502),
        ("c.swsp zero,252(sp)", 0xdf82),
        ("c.swsp ra,12(sp)", 0xc606),
        ("c.jr ra", 0x8082),
        ("c.jr t0", 0x8282),
        ("c.mv t6,ra", 0x8f86),
        ("c.mv a0,a1", 0x852e),
        ("c.ebreak", 0x9002),
        ("c.jalr a0", 0x9502),
        ("c.add sp,t6", 0x917e),
    ];

    #[test]
    fn compressed_instructions() {
        for &(source, encoding) in COMPRESSED {
            assert_eq!(halves(source), [encoding], "{source}");
        }
    }

    #[test]
    fn compressed_disassembly_round_trips() {
        for &(source, encoding) in COMPRESSED {
            let printed = encoding::Instruction::decode(&mut &encoding.to_le_bytes()[..])
                .unwrap()
                .unwrap_or_else(|| panic!("{source} does not decode"))
                .to_string();
            assert_eq!(
                halves(&printed),
                [encoding],
                "{source} printed as {printed:?}"
            );
        }
    }

    #[test]
    fn compressed_branches_and_jumps() {
        // Out to the furthest offset each one reaches, both ways.
        let far = |n: usize| ".space 2\n".repeat(n);
        assert_eq!(
            halves(&format!("1:\n{}c.j 1b", far(1024)))[1024],
            0xb001 // c.j -2048
        );
        assert_eq!(halves(&format!("c.j 1f\n{}1:", far(1022)))[0], 0xaffd); // c.j 2046
        assert_eq!(halves("1:\nc.nop\nc.jal 1b"), [0x0001, 0x3ffd]); // c.jal -2
        assert_eq!(
            halves(&format!("1:\n{}c.beqz a0,1b", far(128)))[128],
            0xd101 // c.beqz a0,-256
        );
        assert_eq!(
            halves(&format!("c.bnez a5,1f\n{}1:", far(126)))[0],
            0xeffd // c.bnez a5,254
        );
        // Mixed with full-size instructions, to a global label.
        assert_eq!(
            halves("c.beqz s0,end\naddi a0,zero,1\nend:\nc.nop"),
            [0xc019, 0x0513, 0x0010, 0x0001] // c.beqz s0,6
        );
    }

    #[test]
    fn compressed_targets_out_of_reach() {
        let far = |n: usize| ".space 2\n".repeat(n);
        assert_eq!(
            c_error(&format!("c.beqz a0,1f\n{}1:", far(127))),
            "the target of 'c.beqz' is 256 bytes away, but it can only reach an even offset in \
             -256..=254"
        );
        assert_eq!(
            c_error(&format!("1:\n{}c.j 1b", far(1025))),
            "the target of 'c.j' is -2050 bytes away, but it can only reach an even offset in \
             -2048..=2046"
        );
        assert_eq!(
            c_error("c.j 1f\n.byte 0\n1:"),
            "the target of 'c.j' is 3 bytes away, but it can only reach an even offset in \
             -2048..=2046"
        );
    }

    #[test]
    fn compressed_operands_are_checked() {
        for (source, error) in [
            (
                "c.lw a6,0(a0)",
                "register a6 cannot be used here, expected one of s0, s1 and a0-a5",
            ),
            (
                "c.sub a0,t0",
                "register t0 cannot be used here, expected one of s0, s1 and a0-a5",
            ),
            ("c.lw a0,2(a0)", "immediate 2 is not a multiple of 4"),
            (
                "c.lw a0,128(a0)",
                "immediate 128 is out of range, expected 0..=124",
            ),
            ("c.addi4spn a0,sp,0", "immediate must not be zero"),
            ("c.addi4spn a0,a1,4", "expected register sp, got a1"),
            ("c.addi zero,1", "register zero cannot be used here"),
            ("c.addi a0,0", "immediate must not be zero"),
            (
                "c.addi a0,32",
                "immediate 32 is out of range, expected -32..=31",
            ),
            ("c.li zero,1", "register zero cannot be used here"),
            ("c.addi16sp sp,8", "immediate 8 is not a multiple of 16"),
            ("c.lui sp,1", "register sp cannot be used here"),
            (
                "c.lui a0,0",
                "immediate 0 is out of range, expected 1..=31 or 0xfffe0..=0xfffff",
            ),
            (
                "c.lui a0,32",
                "immediate 32 is out of range, expected 1..=31 or 0xfffe0..=0xfffff",
            ),
            (
                "c.slli a0,32",
                "immediate 32 is out of range, expected 1..=31",
            ),
            ("c.srli a0,0", "immediate must not be zero"),
            ("c.lwsp zero,0(sp)", "register zero cannot be used here"),
            ("c.lwsp ra,0(a0)", "expected register sp, got a0"),
            ("c.jr zero", "register zero cannot be used here"),
            ("c.mv a0,zero", "register zero cannot be used here"),
            ("c.add zero,a0", "register zero cannot be used here"),
        ] {
            assert_eq!(c_error(source), error, "{source}");
        }
    }

    #[test]
    fn compressed_instructions_need_the_extension() {
        assert_eq!(
            assemble_for(RV32IM, "c.nop"),
            Err("unknown mnemonic or directive".to_owned())
        );
        let rv64ic = Isa {
            exts: ExtensionSet::COMPRESSED,
            xlen: XLen::Rv64,
        };
        // `c.jal` is RV32-only; on RV64 its encoding is `c.addiw`.
        assert_eq!(
            assemble_for(rv64ic, "1:\nc.jal 1b"),
            Err("unknown mnemonic or directive".to_owned())
        );
        assert_eq!(
            assemble_for(rv64ic, "c.slli a0,63").unwrap()[..],
            [0x7e, 0x15]
        );
    }

    #[test]
    fn compressed_mnemonics_are_their_own() {
        for (source, mnemonic) in [
            ("c.ebreak", "c.ebreak"),
            ("c.jalr a0", "c.jalr"),
            ("c.add a0,a1", "c.add"),
            ("c.swsp ra,12(sp)", "c.swsp"),
        ] {
            let bytes = assemble_for(RV32IMC, source).unwrap();
            let printed = encoding::Instruction::decode(&mut &bytes[..])
                .unwrap()
                .unwrap()
                .to_string();
            assert_eq!(
                printed.split_whitespace().next(),
                Some(mnemonic),
                "{source}"
            );
        }
    }

    const RV32IMF: Isa = Isa {
        exts: ExtensionSet::INTEGER_MULDIV.union(ExtensionSet::SINGLE_PRECISION_FP),
        xlen: XLen::Rv32,
    };
    const RV32IMFC: Isa = Isa {
        exts: RV32IMF.exts.union(ExtensionSet::COMPRESSED),
        xlen: XLen::Rv32,
    };

    fn words_for(isa: Isa, source: &str) -> Vec<u32> {
        let text = assemble_for(isa, source).unwrap_or_else(|err| panic!("{source:?}: {err}"));
        text.chunks(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect()
    }

    /// Every F instruction, with the encodings of an encoder written
    /// separately from the format tables of the specification.
    const SINGLE_PRECISION: &[(&str, u32)] = &[
        ("flw ft0,-2048(sp)", 0x8001_2007),
        ("flw ft11,2047(t6)", 0x7fff_af87),
        ("fsw fs11,-4(s0)", 0xffb4_2e27),
        ("fsw f31,2047(zero)", 0x7ff0_2fa7),
        ("fmadd.s fa0,fa1,fa2,fa3", 0x68c5_f543),
        ("fmsub.s ft0,ft1,ft2,ft3,rne", 0x1820_8047),
        ("fnmsub.s f31,f30,f29,f28,rmm", 0xe1df_4fcb),
        ("fnmadd.s fs2,fs3,fs4,fs5,rup", 0xa949_b94f),
        ("fadd.s fa0,fa1,fa2,rdn", 0x00c5_a553),
        ("fadd.s fa0,fa0,fa1", 0x00b5_7553),
        ("fsub.s ft8,ft9,ft10", 0x09ee_fe53),
        ("fmul.s fa7,fa6,fa5,rtz", 0x10f8_18d3),
        ("fdiv.s fs0,fs1,fs2", 0x1924_f453),
        ("fsqrt.s fa0,fa1", 0x5805_f553),
        ("fsqrt.s fa0,fa1,rne", 0x5805_8553),
        ("fsgnj.s fa0,fa1,fa2", 0x20c5_8553),
        ("fsgnjn.s ft0,ft1,ft2", 0x2020_9053),
        ("fsgnjx.s f1,f2,f3", 0x2031_20d3),
        ("fmin.s fa0,fa1,fa2", 0x28c5_8553),
        ("fmax.s fa0,fa1,fa2", 0x28c5_9553),
        ("fcvt.w.s a0,fa0", 0xc005_7553),
        ("fcvt.w.s a0,fa0,rtz", 0xc005_1553),
        ("fcvt.wu.s t6,ft11,rtz", 0xc01f_9fd3),
        ("fcvt.s.w fa0,a0,rne", 0xd005_0553),
        ("fcvt.s.wu f31,t6", 0xd01f_ffd3),
        ("fmv.x.w a0,fa0", 0xe005_0553),
        ("fmv.w.x fa0,a0", 0xf005_0553),
        ("feq.s a0,fa0,fa1", 0xa0b5_2553),
        ("flt.s t0,ft0,ft1", 0xa010_12d3),
        ("fle.s a5,fs11,ft11", 0xa1fd_87d3),
        ("fclass.s a0,fa0", 0xe005_1553),
    ];

    #[test]
    fn single_precision_instructions() {
        for &(source, encoding) in SINGLE_PRECISION {
            assert_eq!(words_for(RV32IMF, source), [encoding], "{source}");
        }
    }

    #[test]
    fn single_precision_disassembly_round_trips() {
        for &(source, encoding) in SINGLE_PRECISION {
            let printed = encoding::Instruction::decode(&mut &encoding.to_le_bytes()[..])
                .unwrap()
                .unwrap_or_else(|| panic!("{source} does not decode"))
                .to_string();
            assert_eq!(
                words_for(RV32IMF, &printed),
                [encoding],
                "{source} printed as {printed:?}"
            );
        }
        // The default rounding mode is left out, as assemblers take it.
        let printed = encoding::Instruction::decode(&mut &0x00b5_7553u32.to_le_bytes()[..])
            .unwrap()
            .unwrap()
            .to_string();
        assert_eq!(printed.trim_end(), "fadd.s      fa0,fa0,fa1");
    }

    #[test]
    fn single_precision_aliases() {
        for (alias, instruction) in [
            ("fmv.x.s a0,fa0", "fmv.x.w a0,fa0"),
            ("fmv.s.x fa0,a0", "fmv.w.x fa0,a0"),
            ("fmv.s fa0,fa1", "fsgnj.s fa0,fa1,fa1"),
            ("fneg.s fa0,fa1", "fsgnjn.s fa0,fa1,fa1"),
            ("fabs.s fa0,fa1", "fsgnjx.s fa0,fa1,fa1"),
            ("fadd.s fa0,fa0,fa1,dyn", "fadd.s fa0,fa0,fa1"),
            ("fadd.s f10 , f10 , f11", "fadd.s fa0,fa0,fa1"),
        ] {
            assert_eq!(
                words_for(RV32IMF, alias),
                words_for(RV32IMF, instruction),
                "{alias}"
            );
        }
    }

    #[test]
    fn single_precision_operands_are_checked() {
        for (source, error) in [
            ("fadd.s fa0,fa1,a2", "unknown floating-point register 'a2'"),
            (
                "fadd.s fa0,fa1,f32",
                "unknown floating-point register 'f32'",
            ),
            (
                "fadd.s fa0,fa1,f01",
                "unknown floating-point register 'f01'",
            ),
            ("fcvt.w.s fa0,fa0", "unknown register"),
            (
                "fadd.s fa0,fa1,fa2,rtn",
                "unknown rounding mode 'rtn', expected one of rne, rtz, rdn, rup, rmm and dyn",
            ),
            ("fsgnj.s fa0,fa1,fa2,rtz", "expected end of statement"),
            (
                "flw fa0,2048(a0)",
                "immediate 2048 is out of range, expected -2048..=2047",
            ),
        ] {
            let err = assemble_for(RV32IMF, source).expect_err(source);
            assert_eq!(err, error, "{source}");
        }
        assert_eq!(
            assemble_for(RV32IM, "fadd.s fa0,fa1,fa2"),
            Err("unknown mnemonic or directive".to_owned())
        );
    }

    const COMPRESSED_SINGLE_PRECISION: &[(&str, u16)] = &[
        ("c.flw fa5,124(s0)", 0x7c7c),
        ("c.flw fa0,0(a0)", 0x6108),
        ("c.fsw fs0,4(a5)", 0xe3c0),
        ("c.fsw fa0,0(a0)", 0xe108),
        ("c.flwsp ft0,252(sp)", 0x707e),
        ("c.flwsp fa0,0(sp)", 0x6502),
        ("c.fswsp f31,8(sp)", 0xe47e),
        ("c.fswsp fa0,0(sp)", 0xe02a),
    ];

    #[test]
    fn compressed_single_precision_instructions() {
        for &(source, encoding) in COMPRESSED_SINGLE_PRECISION {
            let bytes =
                assemble_for(RV32IMFC, source).unwrap_or_else(|err| panic!("{source}: {err}"));
            assert_eq!(*bytes, encoding.to_le_bytes(), "{source}");

            let printed = encoding::Instruction::decode(&mut &bytes[..])
                .unwrap()
                .unwrap()
                .to_string();
            assert_eq!(
                *assemble_for(RV32IMFC, &printed).unwrap(),
                encoding.to_le_bytes(),
                "{source} printed as {printed:?}"
            );
        }
        assert_eq!(
            assemble_for(RV32IMFC, "c.flw ft0,0(a0)"),
            Err(
                "register ft0 cannot be used here, expected one of fs0, fs1 and fa0-fa5".to_owned()
            )
        );
        assert_eq!(
            assemble_for(RV32IMFC, "c.fswsp fa0,2(sp)"),
            Err("immediate 2 is not a multiple of 4".to_owned())
        );
    }

    #[test]
    fn compressed_single_precision_needs_both_extensions_on_rv32() {
        for isa in [
            RV32IMF,
            RV32IMC,
            Isa {
                xlen: XLen::Rv64,
                ..RV32IMFC
            },
        ] {
            assert_eq!(
                assemble_for(isa, "c.flw fa0,0(a0)"),
                Err("unknown mnemonic or directive".to_owned())
            );
        }
    }

    /// `.insn` assembles to the same bytes as the instruction it spells out.
    #[test]
    fn insn_formats() {
        for (insn, instruction) in [
            // The examples of the GNU as manual.
            (".insn r 0x33, 0, 0, a0, a1, a2", "add a0,a1,a2"),
            (".insn 0x4, 0xc58533", "add a0,a1,a2"),
            (".insn 0xc58533", "add a0,a1,a2"),
            // Each 4-byte format, with opcodes by name and by number.
            (".insn r OP, 0, 0x20, a0, a1, a2", "sub a0,a1,a2"),
            (".insn r OP_FP, 7, 0, fa0, fa1, fa2", "fadd.s fa0,fa1,fa2"),
            (
                ".insn r MADD, 7, 0, fa0, fa1, fa2, fa3",
                "fmadd.s fa0,fa1,fa2,fa3",
            ),
            (
                ".insn r4 MADD, 1, 0, fa0, fa1, fa2, fa3",
                "fmadd.s fa0,fa1,fa2,fa3,rtz",
            ),
            (".insn r OP_FP, 0, 0x70, a0, fa0, x0", "fmv.x.w a0,fa0"),
            (".insn i OP_IMM, 0, a0, a1, -16", "addi a0,a1,-16"),
            (".insn i 0x13, 7, t0, t1, 2047", "andi t0,t1,2047"),
            (".insn i LOAD, 2, a0, -4(sp)", "lw a0,-4(sp)"),
            (".insn i JALR, 0, ra, 8(t0)", "jalr ra,t0,8"),
            (".insn s STORE, 2, a0, -4(sp)", "sw a0,-4(sp)"),
            (".insn s STORE_FP, 2, fa0, 2047(a0)", "fsw fa0,2047(a0)"),
            (".insn u LUI, a0, 0x12345", "lui a0,0x12345"),
            (".insn u LUI, a0, 0xfffff", "lui a0,0xfffff"),
            (".insn u AUIPC, t0, 0", "auipc t0,0"),
            ("1:\n.insn b BRANCH, 1, a0, a1, 1b", "1:\nbne a0,a1,1b"),
            (
                ".insn sb BRANCH, 0, a0, a1, 1f\nnop\n1:",
                "beq a0,a1,1f\nnop\n1:",
            ),
            ("1:\n.insn j JAL, ra, 1b", "1:\njal ra,1b"),
            (
                ".insn uj 0x6f, zero, end\nnop\nend:",
                "jal zero,end\nnop\nend:",
            ),
            // Each compressed format. The immediates of `ciw`, `css`, `cl`
            // and `cs` are the raw fields, not the offsets they encode.
            (".insn cr C2, 9, a0, a1", "c.add a0,a1"),
            (".insn ci C1, 0, a0, -1", "c.addi a0,-1"),
            (".insn ciw C0, 0, a0, 2", "c.addi4spn a0,sp,4"),
            (".insn css C2, 6, ra, 12", "c.swsp ra,12(sp)"),
            (".insn cl C0, 2, a0, 31(a1)", "c.lw a0,124(a1)"),
            (".insn cs C0, 6, a5, 2(s0)", "c.sw a5,4(s0)"),
            (".insn ca C1, 35, 0, a0, a1", "c.sub a0,a1"),
            (
                "1:\nc.nop\n.insn cb C1, 6, a0, 1b",
                "1:\nc.nop\nc.beqz a0,1b",
            ),
            (".insn cj 1, 5, 1f\nc.nop\n1:", "c.j 1f\nc.nop\n1:"),
            // The numeric forms for 2 bytes.
            (".insn 2, 0x952e", "c.add a0,a1"),
            (".insn 0x952e", "c.add a0,a1"),
        ] {
            let isa = Isa {
                exts: RV32IMFC.exts,
                xlen: XLen::Rv32,
            };
            let expected = assemble_for(isa, instruction)
                .unwrap_or_else(|err| panic!("{instruction:?}: {err}"));
            let got = assemble_for(isa, insn).unwrap_or_else(|err| panic!("{insn:?}: {err}"));
            assert_eq!(got, expected, "{insn}");
        }
    }

    #[test]
    fn insn_values_longer_than_4_bytes() {
        assert_eq!(
            *assemble_for(RV32IM, ".insn 0x123456789f").unwrap(),
            [0x9f, 0x78, 0x56, 0x34, 0x12, 0x00]
        );
        assert_eq!(
            *assemble_for(RV32IM, ".insn 8, 0xffffffffffffff3f").unwrap(),
            [0x3f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
        // Mixed with labels after them.
        let mut asm = assemble(
            &SectionPositions::absolute(0, 0x400, 0x800, 0xc00),
            ".insn 0x952e\n.insn 0x1f\nafter:\nnop",
        );
        assert_eq!(symbol(&mut asm, "after"), (SectionKind::Text, 8));
    }

    #[test]
    fn insn_errors() {
        let isa = Isa {
            exts: RV32IMFC.exts,
            xlen: XLen::Rv32,
        };
        for (source, error) in [
            (
                ".insn 4, 0x952e",
                "0x952e is not a valid 4-byte instruction",
            ),
            (
                ".insn 2, 0x10001",
                "0x10001 is not a valid 2-byte instruction",
            ),
            (
                ".insn 3, 0x13",
                "instruction length 3 is not one of 2, 4, 6 and 8 bytes",
            ),
            (
                ".insn 0x7f",
                "0x7f is the encoding of an instruction longer than 8 bytes",
            ),
            (
                ".insn 0x100000013",
                "0x100000013 is wider than the 4 bytes its encoding gives it",
            ),
            (
                ".insn -1",
                "immediate -1 is out of range, expected a 64-bit unsigned value",
            ),
            (
                ".insn x 1",
                "unknown instruction format 'x', expected one of r, r4, i, s, b, sb, u, j, uj, cr, ci, ciw, css, cl, cs, ca, cb and cj",
            ),
            (
                ".insn r 0x30, 0, 0, a0, a1, a2",
                "opcode 0x30 is not the opcode of a 4-byte instruction",
            ),
            (
                ".insn r C1, 0, 0, a0, a1, a2",
                "opcode 0x01 is not the opcode of a 4-byte instruction",
            ),
            (".insn r FOO, 0, 0, a0, a1, a2", "unknown opcode 'FOO'"),
            (".insn cr OP, 9, a0, a1", "opcode OP is not a 2-bit opcode"),
            (
                ".insn cr 3, 9, a0, a1",
                "opcode 3 is not the opcode of a 2-byte instruction",
            ),
            (
                ".insn r OP, 8, 0, a0, a1, a2",
                "immediate 8 is out of range, expected 0..=7",
            ),
            (
                ".insn r OP, 0, 128, a0, a1, a2",
                "immediate 128 is out of range, expected 0..=127",
            ),
            (
                ".insn r MADD, 0, 4, a0, a1, a2, a3",
                "immediate 4 is out of range, expected 0..=3 for the func2 of an instruction with 4 registers",
            ),
            (
                ".insn r4 MADD, 7, 0, fa0, fa1, fa2",
                "expected a fourth register, rs3",
            ),
            (".insn r OP, 0, 0, a0, a1, q2", "unknown register 'q2'"),
            (
                ".insn i OP_IMM, 0, a0, a1, 2048",
                "immediate 2048 is out of range, expected -2048..=2047",
            ),
            (
                ".insn u LUI, a0, 0x100000",
                "immediate 0x100000 is out of range, expected 0..=0xfffff",
            ),
            (
                ".insn ci C1, 0, a0, 32",
                "immediate 32 is out of range, expected -32..=31",
            ),
            (
                ".insn ciw C0, 0, a0, 256",
                "immediate 256 is out of range, expected 0..=255",
            ),
            (
                ".insn ca C1, 35, 0, t0, a1",
                "register t0 cannot be used here, expected one of x8-x15 and f8-f15 (s0, s1, a0-a5, fs0, fs1 and fa0-fa5)",
            ),
            (
                ".section .bss\n.insn 0x13",
                "'.bss' is zero-initialised and cannot hold instructions",
            ),
            (
                ".insn b BRANCH, 0, a0, a1, 1f\n.space 4096\n1:",
                "the target of '.insn b' is 4100 bytes away, but it can only reach an even offset in -4096..=4094",
            ),
            (
                ".insn cj C1, 5, 1f\n.byte 0\n1:",
                "the target of '.insn cj' is 3 bytes away, but it can only reach an even offset in -2048..=2046",
            ),
        ] {
            assert_eq!(assemble_for(isa, source), Err(error.to_owned()), "{source}");
        }
        assert_eq!(
            assemble_for(RV32IM, ".insn cr C2, 9, a0, a1"),
            Err("'.insn cr' is a compressed format, which needs the C extension".to_owned())
        );
    }
}
