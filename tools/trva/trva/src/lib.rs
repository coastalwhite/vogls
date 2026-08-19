//! Tiny RISC-V Assembler
//!
//! A library that performs RISC-V assembly into sections with basic relaxation.
use self::cursor::{SourceCursor, parse_gp_register};
use self::encoding::{FenceMode, FenceOrder, XRegIdent};
use self::error::AssembleResult;
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

pub struct Assembler {
    label_table: LabelTable,

    symbols: LabelMap<UnresolvedOffset>,

    text: Section,
    data: Section,
    rodata: Section,
    bss: Section,

    positions: SectionPositions,
    current_section: SectionKind,

    #[expect(unused)]
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

    fn min_max_size_with_symbols(&self, symbols: &LabelMap<UnresolvedOffset>) -> (u32, u32) {
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
    pub fn next_multiple_of(self, rhs: u32) -> Self {
        Self {
            min: self.min.next_multiple_of(rhs),
            max: self.max.next_multiple_of(rhs),
        }
    }

    fn pad(self, space: u32) -> Self {
        Self {
            min: self.min + space,
            max: self.max + space,
        }
    }
}

struct Section {
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
    fn min_max_size(&self, symbols: &LabelMap<UnresolvedOffset>) -> (u32, u32) {
        match self {
            Event::Data(n) | Event::Pad(n) => (*n, *n),
            Event::Symbol(_) | Event::LocalLabel(_) => (0, 0),
            Event::Resolve(item) => item.min_max_size_with_symbols(symbols),
            Event::Align(n) => (0, *n),
        }
    }
}

impl Section {
    fn new(start_offset: u32) -> Self {
        Self {
            offset: UnresolvedOffset {
                min: start_offset,
                max: start_offset,
            },
            data: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn push_le_u32(&mut self, value: u32) {
        self.data.extend_from_slice(&value.to_le_bytes());
        match self.events.last_mut() {
            Some(Event::Data(size)) => *size += 4,
            _ => self.events.push(Event::Data(4)),
        }
        self.offset.min += 4;
        self.offset.max += 4;
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
        if self.offset.min == self.offset.max {
            self.pad(self.offset.min.next_multiple_of(n) - self.offset.min);
            return;
        }

        self.events.push(Event::Align(n));
        self.offset = self.offset.next_multiple_of(n);
    }
}

#[derive(Clone, Copy)]
pub enum SectionKind {
    Text,
    Data,
    ReadOnlyData,
    Bss,
}

#[derive(Clone)]
pub struct SectionPositions {
    pub text: u32,
    pub data: u32,
    pub rodata: u32,
    pub bss: u32,
}
impl SectionPositions {
    fn as_array(&self) -> [u32; 4] {
        [self.text, self.data, self.rodata, self.bss]
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

            text: Section::new(positions.text),
            data: Section::new(positions.data),
            rodata: Section::new(positions.rodata),
            bss: Section::new(positions.bss),

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
                let position = self.current_section().offset;
                self.symbols.set(label, position);
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

                    let position = self.current_section().offset;
                    let name = self.label_table.get_or_insert(name);
                    self.symbols.set(name, position);
                }
                ".align" => {
                    cursor.take_separator()?;
                    let align = cursor.take_u32()?;
                    self.current_section_mut().align(align);
                }

                ".space" => {
                    cursor.take_separator()?;
                    let space = cursor.take_u32()?;
                    self.current_section_mut().pad(space);
                }

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
        self.parse_itype_instr_base(cursor, new, |cursor| {
            Ok(cursor.take_shamt()? & 0x1F)
        })
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

    pub fn assemble(mut self) -> Assembled {
        let mut out_symbols = LabelMap::with_len(self.symbols.len());

        // Link and relax.
        let mut local_offsets = [const { Vec::<u32>::new() }; 4];
        let mut reloc_variants = [const { Vec::<u8>::new() }; 4];
        let mut offsets = [0u32; 4];
        let mut start_offsets = [0u32; 4];
        for ((i, section), start_offset) in [&self.text, &self.data, &self.rodata, &self.bss]
            .iter()
            .enumerate()
            .zip(self.positions.as_array())
        {
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
                        self.symbols[*label] = Some(UnresolvedOffset {
                            min: offset,
                            max: offset,
                        });
                        out_symbols.set(*label, (SectionKind::Text, offset));
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
                        out.extend_from_slice(
                            &self.text.data[data_ptr as usize..][..*size as usize],
                        );
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
                            out.extend_from_slice(
                                &encoding::Lui::new(*rd, lbl_offset & !0xFFF)
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
        Assembled {
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
        }
    }

    fn current_section(&self) -> &Section {
        match self.current_section {
            SectionKind::Text => &self.text,
            SectionKind::Data => &self.data,
            SectionKind::ReadOnlyData => &self.rodata,
            SectionKind::Bss => &self.bss,
        }
    }
    fn current_section_mut(&mut self) -> &mut Section {
        match self.current_section {
            SectionKind::Text => &mut self.text,
            SectionKind::Data => &mut self.data,
            SectionKind::ReadOnlyData => &mut self.rodata,
            SectionKind::Bss => &mut self.bss,
        }
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

            if let Some(imm) = signed_imm_to_imm12(imm) {
                asm.text
                    .push_le_u32(encoding::Addi::new(rd, XRegIdent::Zero, imm).encode_as_u32());
            } else {
                let imm = (imm as u64 & 0xFFFF_FFFF) as u32;
                // @TODO: Better instruction dispatch.
                asm.text
                    .push_le_u32(encoding::Lui::new(rd, imm & !0xFFF).encode_as_u32());
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
            let imm = c.take_imm20()?;
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
            let imm = c.take_imm20()?;
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
                |c| c.take_imm12_unsigned(),
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

fn resolve_min_max_target_label(
    lbl: &TargetLabel,
    offset: u32,
    events: &[Event],
    eid: usize,
    symbols: &LabelMap<UnresolvedOffset>,
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
        TargetLabel::Global(lbl) => *symbols[*lbl].as_ref().unwrap(),
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
