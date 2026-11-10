use crate::cursor::{SourceCursor, parse_fp_register, parse_gp_register};
use crate::encoding::{self, CXRegIdent, XRegIdent};
use crate::error::AssembleResult;
use crate::isa::ExtensionSet;
use crate::{Assembler, RelocationItem, SectionKind, push_compressed};

/// The names `.insn` takes for opcodes, and the opcode each stands for.
const OPCODES: &[(&str, u32)] = &[
    ("C0", 0x0),
    ("C1", 0x1),
    ("C2", 0x2),
    ("LOAD", 0x03),
    ("LOAD_FP", 0x07),
    ("CUSTOM_0", 0x0b),
    ("MISC_MEM", 0x0f),
    ("OP_IMM", 0x13),
    ("AUIPC", 0x17),
    ("OP_IMM_32", 0x1b),
    ("STORE", 0x23),
    ("STORE_FP", 0x27),
    ("CUSTOM_1", 0x2b),
    ("AMO", 0x2f),
    ("OP", 0x33),
    ("LUI", 0x37),
    ("OP_32", 0x3b),
    ("MADD", 0x43),
    ("MSUB", 0x47),
    ("NMSUB", 0x4b),
    ("NMADD", 0x4f),
    ("OP_FP", 0x53),
    ("OP_V", 0x57),
    ("CUSTOM_2", 0x5b),
    ("BRANCH", 0x63),
    ("JALR", 0x67),
    ("JAL", 0x6f),
    ("SYSTEM", 0x73),
    ("CUSTOM_3", 0x7b),
];

const FORMATS: &str = "r, r4, i, s, b, sb, u, j, uj, cr, ci, ciw, css, cl, cs, ca, cb and cj";

/// An instruction of a `.insn` format whose immediate is the offset to a label,
/// filled in once the label has an address.
#[derive(Clone, Copy)]
pub(crate) struct InsnJump {
    /// The instruction with its immediate left zero.
    pub(crate) template: u32,
    pub(crate) format: JumpFormat,
}

#[derive(Clone, Copy)]
pub(crate) enum JumpFormat {
    B,
    J,
    Cb,
    Cj,
}

impl JumpFormat {
    pub(crate) fn mnemonic(self) -> &'static str {
        match self {
            Self::B => ".insn b",
            Self::J => ".insn j",
            Self::Cb => ".insn cb",
            Self::Cj => ".insn cj",
        }
    }

    pub(crate) fn size(self) -> u32 {
        match self {
            Self::B | Self::J => 4,
            Self::Cb | Self::Cj => 2,
        }
    }

    /// The offsets the format encodes.
    pub(crate) fn range(self) -> (i32, i32) {
        match self {
            Self::B => (-4096, 4094),
            Self::J => (-(1 << 20), (1 << 20) - 2),
            Self::Cb => (-256, 254),
            Self::Cj => (-2048, 2046),
        }
    }

    /// The bits that put `offset` in the immediate of the format, taken from
    /// an instruction of that format whose other fields are all zero apart
    /// from those masked off.
    pub(crate) fn immediate(self, offset: i32) -> u32 {
        let zero = XRegIdent::Zero;
        match self {
            Self::B => encoding::Beq::new(zero, zero, offset as i16).encode_as_u32() & !0x7f,
            Self::J => encoding::Jal::new(zero, offset).encode_as_u32() & !0x7f,
            // `s0` is 0 in the 3-bit register field.
            Self::Cb => {
                encoding::CBeqz::new(CXRegIdent::Fp, offset as i16).encode_as_u32() & !0xe003
            }
            Self::Cj => encoding::CJ::new(offset as i16).encode_as_u32() & !0xe003,
        }
    }
}

/// The length in bytes that the lowest bits of an instruction give it, for
/// the lengths up to 8 bytes.
fn encoded_length(value: u64) -> Option<u64> {
    if value & 0b11 != 0b11 {
        Some(2)
    } else if value & 0b1_1100 != 0b1_1100 {
        Some(4)
    } else if value & 0b11_1111 == 0b01_1111 {
        Some(6)
    } else if value & 0b111_1111 == 0b011_1111 {
        Some(8)
    } else {
        None
    }
}

impl Assembler {
    /// Everything after `.insn`.
    pub(crate) fn parse_insn(&mut self, c: &mut SourceCursor<'_>) -> AssembleResult<()> {
        if self.current_section == SectionKind::Bss {
            return Err(Box::new(c.new_err(
                "'.bss' is zero-initialised and cannot hold instructions",
            )));
        }
        c.take_separator()?;
        if c.is_next_first_ident_char() {
            return self.parse_insn_format(c);
        }

        let start = c.offset;
        let first = c.take_u64()?;
        c.take_opt_separator();
        let (length, value) = if c.peek_byte() == Some(b',') {
            if !matches!(first, 2 | 4 | 6 | 8) {
                c.offset = start;
                return Err(Box::new(c.new_err(format!(
                    "instruction length {first} is not one of 2, 4, 6 and 8 bytes"
                ))));
            }
            c.take_comma_separator()?;
            let value_start = c.offset;
            let value = c.take_u64()?;
            let fits = first == 8 || value >> (8 * first) == 0;
            if encoded_length(value) != Some(first) || !fits {
                c.offset = value_start;
                return Err(Box::new(c.new_err(format!(
                    "0x{value:x} is not a valid {first}-byte instruction"
                ))));
            }
            (first, value)
        } else {
            let Some(length) = encoded_length(first) else {
                c.offset = start;
                return Err(Box::new(c.new_err(format!(
                    "0x{first:x} is the encoding of an instruction longer than 8 bytes"
                ))));
            };
            if length < 8 && first >> (8 * length) != 0 {
                c.offset = start;
                return Err(Box::new(c.new_err(format!(
                    "0x{first:x} is wider than the {length} bytes its encoding gives it"
                ))));
            }
            (length, first)
        };
        self.current_section_mut()
            .push_bytes(&value.to_le_bytes()[..length as usize]);
        Ok(())
    }

    /// `.insn type operand, ..., operand`.
    fn parse_insn_format(&mut self, c: &mut SourceCursor<'_>) -> AssembleResult<()> {
        let start = c.offset;
        let format = c.take_ident()?;
        let compressed = matches!(
            format,
            "cr" | "ci" | "ciw" | "css" | "cl" | "cs" | "ca" | "cb" | "cj"
        );
        if compressed && !self.isa.exts.contains(ExtensionSet::COMPRESSED) {
            c.offset = start;
            return Err(Box::new(c.new_err(format!(
                "'.insn {format}' is a compressed format, which needs the C extension"
            ))));
        }
        c.take_separator()?;

        let instruction = match format {
            "r" | "r4" => {
                let op = take_opcode7(c)?;
                let func3 = comma_field(c, 3)?;
                // `func7` with three registers, `func2` with four.
                c.take_comma_separator()?;
                let func_start = c.offset;
                let func = take_field(c, 7)?;
                let rd = comma_register(c)?;
                let rs1 = comma_register(c)?;
                let rs2 = comma_register(c)?;
                c.take_opt_separator();
                if c.peek_byte() == Some(b',') {
                    let rs3 = comma_register(c)?;
                    if func > 0b11 {
                        c.offset = func_start;
                        return Err(Box::new(c.new_err(format!(
                            "immediate {func} is out of range, expected 0..=3 for the func2 \
                             of an instruction with 4 registers"
                        ))));
                    }
                    rs3 << 27 | func << 25 | rs2 << 20 | rs1 << 15 | func3 << 12 | rd << 7 | op
                } else if format == "r4" {
                    return Err(Box::new(c.new_err("expected a fourth register, rs3")));
                } else {
                    func << 25 | rs2 << 20 | rs1 << 15 | func3 << 12 | rd << 7 | op
                }
            }
            "i" => {
                let op = take_opcode7(c)?;
                let func3 = comma_field(c, 3)?;
                let rd = comma_register(c)?;
                c.take_comma_separator()?;
                // `rd, rs1, simm12`, or `rd, simm12(rs1)` as for a load.
                let (rs1, imm) = if c.is_next_first_ident_char() {
                    let rs1 = take_register(c)?;
                    c.take_comma_separator()?;
                    (rs1, c.take_imm12()?)
                } else {
                    let imm = c.take_imm12()?;
                    (take_base_register(c)?, imm)
                };
                (imm as u32 & 0xfff) << 20 | rs1 << 15 | func3 << 12 | rd << 7 | op
            }
            "s" => {
                let op = take_opcode7(c)?;
                let func3 = comma_field(c, 3)?;
                let rs2 = comma_register(c)?;
                c.take_comma_separator()?;
                let imm = c.take_imm12()? as u32;
                let rs1 = take_base_register(c)?;
                (imm >> 5 & 0x7f) << 25
                    | rs2 << 20
                    | rs1 << 15
                    | func3 << 12
                    | (imm & 0x1f) << 7
                    | op
            }
            "b" | "sb" => {
                let op = take_opcode7(c)?;
                let func3 = comma_field(c, 3)?;
                let rs1 = comma_register(c)?;
                let rs2 = comma_register(c)?;
                c.take_comma_separator()?;
                let template = rs2 << 20 | rs1 << 15 | func3 << 12 | op;
                return self.push_insn_jump(c, template, JumpFormat::B);
            }
            "u" => {
                let op = take_opcode7(c)?;
                let rd = comma_register(c)?;
                c.take_comma_separator()?;
                // The upper 20 bits, as for `lui`.
                let imm = c.take_int_in(0..=0xf_ffff, "0..=0xfffff")? as u32;
                imm << 12 | rd << 7 | op
            }
            "j" | "uj" => {
                let op = take_opcode7(c)?;
                let rd = comma_register(c)?;
                c.take_comma_separator()?;
                return self.push_insn_jump(c, rd << 7 | op, JumpFormat::J);
            }

            "cr" => {
                let op = take_opcode2(c)?;
                let func4 = comma_field(c, 4)?;
                let rd = comma_register(c)?;
                let rs2 = comma_register(c)?;
                func4 << 12 | rd << 7 | rs2 << 2 | op
            }
            "ci" => {
                let op = take_opcode2(c)?;
                let func3 = comma_field(c, 3)?;
                let rd = comma_register(c)?;
                c.take_comma_separator()?;
                let imm = c.take_int_in(-32..=31, "-32..=31")? as u32;
                func3 << 13 | (imm >> 5 & 1) << 12 | rd << 7 | (imm & 0x1f) << 2 | op
            }
            "ciw" => {
                let op = take_opcode2(c)?;
                let func3 = comma_field(c, 3)?;
                let rd = comma_c_register(c)?;
                let imm = comma_field(c, 8)?;
                func3 << 13 | imm << 5 | rd << 2 | op
            }
            "css" => {
                let op = take_opcode2(c)?;
                let func3 = comma_field(c, 3)?;
                let rs2 = comma_register(c)?;
                let imm = comma_field(c, 6)?;
                func3 << 13 | imm << 7 | rs2 << 2 | op
            }
            "cl" | "cs" => {
                let op = take_opcode2(c)?;
                let func3 = comma_field(c, 3)?;
                let reg = comma_c_register(c)?;
                c.take_comma_separator()?;
                let imm = take_field(c, 5)?;
                let rs1 = take_c_base_register(c)?;
                func3 << 13 | (imm >> 2) << 10 | rs1 << 7 | (imm & 0b11) << 5 | reg << 2 | op
            }
            "ca" => {
                let op = take_opcode2(c)?;
                let func6 = comma_field(c, 6)?;
                let func2 = comma_field(c, 2)?;
                let rd = comma_c_register(c)?;
                let rs2 = comma_c_register(c)?;
                func6 << 10 | rd << 7 | func2 << 5 | rs2 << 2 | op
            }
            "cb" => {
                let op = take_opcode2(c)?;
                let func3 = comma_field(c, 3)?;
                let rs1 = comma_c_register(c)?;
                c.take_comma_separator()?;
                let template = func3 << 13 | rs1 << 7 | op;
                return self.push_insn_jump(c, template, JumpFormat::Cb);
            }
            "cj" => {
                let op = take_opcode2(c)?;
                let func3 = comma_field(c, 3)?;
                c.take_comma_separator()?;
                return self.push_insn_jump(c, func3 << 13 | op, JumpFormat::Cj);
            }
            _ => {
                c.offset = start;
                return Err(Box::new(c.new_err(format!(
                    "unknown instruction format '{format}', expected one of {FORMATS}"
                ))));
            }
        };

        if compressed {
            push_compressed(self, instruction);
        } else {
            self.current_section_mut().push_le_u32(instruction);
        }
        Ok(())
    }

    /// The label a `.insn` branch or jump goes to, filled in later.
    fn push_insn_jump(
        &mut self,
        c: &mut SourceCursor<'_>,
        template: u32,
        format: JumpFormat,
    ) -> AssembleResult<()> {
        let label = self.take_target_label(c)?;
        self.current_section_mut()
            .push_relocation_item(RelocationItem::Insn(InsnJump { template, format }, label));
        Ok(())
    }
}

/// An unsigned field of `bits` bits.
fn take_field(c: &mut SourceCursor<'_>, bits: u32) -> AssembleResult<u32> {
    let max = (1i64 << bits) - 1;
    Ok(c.take_int_in(0..=max, &format!("0..={max}"))? as u32)
}

fn comma_field(c: &mut SourceCursor<'_>, bits: u32) -> AssembleResult<u32> {
    c.take_comma_separator()?;
    take_field(c, bits)
}

/// An opcode by number or by name, which has to be one of `bits` bits.
fn take_opcode(c: &mut SourceCursor<'_>, bits: u32) -> AssembleResult<u32> {
    if !c.is_next_first_ident_char() {
        return take_field(c, bits);
    }
    let start = c.offset;
    let name = c.take_ident()?;
    match OPCODES.iter().find(|&&(known, _)| known == name) {
        Some(&(_, opcode)) if opcode < 1 << bits => Ok(opcode),
        Some(_) => {
            c.offset = start;
            Err(Box::new(c.new_err(format!(
                "opcode {name} is not a {bits}-bit opcode"
            ))))
        }
        None => {
            c.offset = start;
            Err(Box::new(c.new_err(format!("unknown opcode '{name}'"))))
        }
    }
}

/// The 7-bit opcode of a 4-byte instruction.
fn take_opcode7(c: &mut SourceCursor<'_>) -> AssembleResult<u32> {
    let start = c.offset;
    let opcode = take_opcode(c, 7)?;
    if encoded_length(u64::from(opcode)) != Some(4) {
        c.offset = start;
        return Err(Box::new(c.new_err(format!(
            "opcode 0x{opcode:02x} is not the opcode of a 4-byte instruction"
        ))));
    }
    Ok(opcode)
}

/// The 2-bit opcode of a 2-byte instruction.
fn take_opcode2(c: &mut SourceCursor<'_>) -> AssembleResult<u32> {
    let start = c.offset;
    let opcode = take_opcode(c, 2)?;
    if opcode == 0b11 {
        c.offset = start;
        return Err(Box::new(
            c.new_err("opcode 3 is not the opcode of a 2-byte instruction"),
        ));
    }
    Ok(opcode)
}

/// The number of a general-purpose or floating-point register.
fn take_register(c: &mut SourceCursor<'_>) -> AssembleResult<u32> {
    let start = c.offset;
    let name = c.take_ident().map_err(|mut err| {
        err.reason = "expected register".into();
        err
    })?;
    match parse_gp_register(name).map(|reg| reg as u32) {
        Some(reg) => Ok(reg),
        None => parse_fp_register(name)
            .map(|reg| reg as u32)
            .ok_or_else(|| {
                c.offset = start;
                Box::new(c.new_err(format!("unknown register '{name}'")))
            }),
    }
}

fn comma_register(c: &mut SourceCursor<'_>) -> AssembleResult<u32> {
    c.take_comma_separator()?;
    take_register(c)
}

/// A register of the 3-bit register fields of the compressed formats:
/// `x8`-`x15` or `f8`-`f15`.
fn take_c_register(c: &mut SourceCursor<'_>) -> AssembleResult<u32> {
    let start = c.offset;
    let reg = take_register(c)?;
    if !(8..16).contains(&reg) {
        let name = &c.content_between(start, c.offset).to_owned();
        c.offset = start;
        return Err(Box::new(c.new_err(format!(
            "register {name} cannot be used here, expected one of x8-x15 and f8-f15 \
             (s0, s1, a0-a5, fs0, fs1 and fa0-fa5)"
        ))));
    }
    Ok(reg - 8)
}

fn comma_c_register(c: &mut SourceCursor<'_>) -> AssembleResult<u32> {
    c.take_comma_separator()?;
    take_c_register(c)
}

/// The `(rs1)` after the offset of a load or store.
fn take_base_register(c: &mut SourceCursor<'_>) -> AssembleResult<u32> {
    c.take_opt_separator();
    c.expect_byte(b'(')?;
    c.take_opt_separator();
    let reg = take_register(c)?;
    c.take_opt_separator();
    c.expect_byte(b')')?;
    Ok(reg)
}

fn take_c_base_register(c: &mut SourceCursor<'_>) -> AssembleResult<u32> {
    c.take_opt_separator();
    c.expect_byte(b'(')?;
    c.take_opt_separator();
    let reg = take_c_register(c)?;
    c.take_opt_separator();
    c.expect_byte(b')')?;
    Ok(reg)
}
