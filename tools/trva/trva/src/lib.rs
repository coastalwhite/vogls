//! Tiny RISC-V Assembler
//!
//! A library that performs RISC-V assembly into sections with basic relaxation.
use self::cursor::{SourceCursor, parse_gp_register};
use self::encoding::{FenceMode, FenceOrder, XRegIdent};
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
}
impl RelocationItem {
    fn min_max_size(&self) -> (u32, u32) {
        match self {
            Self::LoadAddress(..) => (8, 8),
            Self::Branch(..) => (4, 12),
            Self::Jump(..) => (4, 8),
            Self::PcRelJump(..) => (4, 8),
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
                ".global" | "globl" => {
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
        self.text.push_le_u32(new(rd, rs, imm));
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
        self.text.push_le_u32(new(rd, rs1, rs2));
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
        self.text
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
        self.text
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

        self.text.push_le_u32(new(rd, rs, imm));
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

        self.text.push_le_u32(new(rs1, rs2, imm));
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

        self.text.push_le_u32(new(rd, rs));
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
                                &self.text.events,
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
                        RelocationItem::PcRelJump(rd, _rt, lbl) => {
                            // @Q? Should `rt` be set anyway?
                            let lbl_offset = resolve_target_label(
                                lbl,
                                &self.text.events,
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
            asm.text
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
                asm.text
                    .push_le_u32(encoding::Addi::new(rd, XRegIdent::Zero, imm).encode_as_u32());
            } else {
                let imm = (imm as u64 & 0xFFFF_FFFF) as u32;
                // @TODO: Better instruction dispatch.
                // `addi` sign-extends its immediate, so round the upper part up
                // when bit 11 is set.
                asm.text.push_le_u32(
                    encoding::Lui::new(rd, imm.wrapping_add(0x800) & !0xFFF).encode_as_u32(),
                );
                asm.text
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
            asm.text
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
            asm.text
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
            asm.text
                .push_relocation_item(RelocationItem::Jump(reg, label));
        }),
    );
    handlers.insert(
        "j",
        handler!(j, |asm, c| {
            // j label      ~     jal x0, label
            c.take_separator()?;
            let label = asm.take_target_label(c)?;
            asm.text
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
            asm.text
                .push_le_u32(encoding::Jalr::new(rd, rs, imm).encode_as_u32());
        }),
    );
    handlers.insert(
        "jr",
        handler!(jr, |asm, c| {
            // jr rs     ~     jalr x0, rs, 0
            c.take_separator()?;
            let rs = c.take_gp_register()?;
            asm.text
                .push_le_u32(encoding::Jalr::new(XRegIdent::Zero, rs, 0).encode_as_u32());
        }),
    );
    handlers.insert(
        "ret",
        handler!(ret, |asm, _c| {
            // ret     ~     jalr x0, x1, 0
            asm.text.push_le_u32(
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
            asm.text.push_relocation_item(RelocationItem::PcRelJump(
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
            asm.text.push_relocation_item(RelocationItem::PcRelJump(
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
            asm.text
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
            asm.text.push_le_u32(
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

            asm.text
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

            asm.text
                .push_le_u32(encoding::Fence::new(FenceMode(0b1000), pred, succ).encode_as_u32());
        }),
    );
    handlers.insert(
        "pause",
        handler!(pause, |asm, _c| {
            asm.text.push_le_u32(
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
            asm.text.push_le_u32(encoding::Ecall::new().encode_as_u32());
        }),
    );
    handlers.insert(
        "ebreak",
        handler!(ebreak, |asm, _c| {
            asm.text
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
            Ext::Compressed => return Err(ext),
            Ext::DoublePrecisionFp => return Err(ext),
            Ext::Rv32E => return Err(ext),
            Ext::SinglePrecisionFp => return Err(ext),
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
}
