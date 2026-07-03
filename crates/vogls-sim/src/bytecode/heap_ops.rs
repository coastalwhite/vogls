use std::cell::Cell;
use std::fmt;
use std::num::NonZeroU16;

use vogls_bits::arithmetic::{
    fv_bin_u64_cell_bitwise_op, fv_bitwise_and_elem, fv_bitwise_andnot_elem, fv_bitwise_or_elem,
    fv_bitwise_ornot_elem, fv_bitwise_xor_elem, fv_cell_addition, fv_cell_divmod,
    fv_cell_multiplication, fv_cell_subtraction, tv_bin_u64_cell_bitwise_mask_last_op,
    tv_bin_u64_cell_bitwise_op, tv_cell_addition, tv_cell_divmod, tv_cell_multiplication,
    tv_cell_subtraction,
};
use vogls_codegen::HeapOffset;
use vogls_ir::VectorSize;
use vogls_runtime::RuntimeState;

use crate::bytecode::write_padded_mnemonic;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    Schedule,
};
pub struct HeapBinaryBitwise {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    op: BitwiseOp,
    size: Option<VectorSize>,
}
pub struct HeapBinaryArithmetic {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    op: ArithmeticOp,
    size: Option<VectorSize>,
}
pub struct HeapBinaryDivMod {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    src_fv: bool,
    fill_x: bool,
    is_mod: bool,
    size: Option<VectorSize>,
}

pub struct HeapCaseEq {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    ne: bool,
    num_words: Option<NonZeroU16>,
}

impl BytecodeInstruction for HeapBinaryBitwise {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapBinaryBitwise as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            op: BitwiseOp::new_masked(v >> 20),
            size: VectorSize::new(v >> 24),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapBinaryBitwise as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | ((self.op as u32) << 20)
                | (self.size.map_or(0, |v| v.get()) << 24),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs1,
            rs2,
            op,
            size: _,
        } = self;
        let mnemonic = match op {
            BitwiseOp::TvAnd => "tv.heap_and",
            BitwiseOp::TvOr => "tv.heap_or",
            BitwiseOp::TvXor => "tv.heap_xor",
            BitwiseOp::TvAndNot => "tv.heap_andnot",
            BitwiseOp::TvOrNot => "tv.heap_ornot",
            BitwiseOp::FvAnd => "fv.heap_and",
            BitwiseOp::FvOr => "fv.heap_or",
            BitwiseOp::FvXor => "fv.heap_xor",
            BitwiseOp::FvAndNot => "fv.heap_andnot",
            BitwiseOp::FvOrNot => "fv.heap_ornot",
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs1}, {rs2}")
    }
    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs1,
            rs2,
            op,
            size,
        } = self;
        let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
        let mut num_words = size.get().div_ceil(64) as usize;
        if op.is_four_value() {
            num_words *= 2;
        }
        let [dst, src1, src2] = state.heap.get_u64_cell_slices([
            (
                HeapOffset {
                    bit_offset: regs[rd] as usize,
                },
                num_words,
            ),
            (
                HeapOffset {
                    bit_offset: regs[rs1] as usize,
                },
                num_words,
            ),
            (
                HeapOffset {
                    bit_offset: regs[rs2] as usize,
                },
                num_words,
            ),
        ]);

        use BitwiseOp as O;
        match op {
            O::TvAnd => tv_bin_u64_cell_bitwise_op(dst, src1, src2, |l, r| l & r),
            O::TvOr => tv_bin_u64_cell_bitwise_op(dst, src1, src2, |l, r| l | r),
            O::TvXor => tv_bin_u64_cell_bitwise_op(dst, src1, src2, |l, r| l ^ r),
            O::TvAndNot => {
                tv_bin_u64_cell_bitwise_mask_last_op(dst, src1, src2, |l, r| l & !r, size);
            }
            O::TvOrNot => {
                tv_bin_u64_cell_bitwise_mask_last_op(dst, src1, src2, |l, r| l | !r, size);
            }

            O::FvAnd => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
                fv_bitwise_and_elem(lspc, lval, rspc, rval)
            }),
            O::FvOr => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
                fv_bitwise_or_elem(lspc, lval, rspc, rval)
            }),
            O::FvXor => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
                fv_bitwise_xor_elem(lspc, lval, rspc, rval)
            }),
            O::FvAndNot => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
                fv_bitwise_andnot_elem(lspc, lval, rspc, rval)
            }),
            O::FvOrNot => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
                fv_bitwise_ornot_elem(lspc, lval, rspc, rval)
            }),
        }
    }
}

impl BytecodeInstruction for HeapBinaryArithmetic {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapBinaryArithmetic as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            op: ArithmeticOp::new_masked(v >> 20),
            size: VectorSize::new(v >> 24),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapBinaryArithmetic as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | ((self.op as u32) << 20)
                | (self.size.map_or(0, |v| v.get()) << 24),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs1,
            rs2,
            op,
            size: _,
        } = self;
        let mnemonic = match op {
            ArithmeticOp::TvAdd => "tv.heap_add",
            ArithmeticOp::TvSub => "tv.heap_sub",
            ArithmeticOp::TvMul => "tv.heap_mul",
            ArithmeticOp::FvAdd => "fv.heap_add",
            ArithmeticOp::FvSub => "fv.heap_sub",
            ArithmeticOp::FvMul => "fv.heap_mul",
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs1}, {rs2}")
    }
    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs1,
            rs2,
            op,
            size,
        } = self;
        let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
        let mut num_words = size.get().div_ceil(64) as usize;
        if op.is_four_value() {
            num_words *= 2;
        }
        let [dst, src1, src2] = state.heap.get_u64_cell_slices([
            (
                HeapOffset {
                    bit_offset: regs[rd] as usize,
                },
                num_words,
            ),
            (
                HeapOffset {
                    bit_offset: regs[rs1] as usize,
                },
                num_words,
            ),
            (
                HeapOffset {
                    bit_offset: regs[rs2] as usize,
                },
                num_words,
            ),
        ]);

        use ArithmeticOp as O;
        match op {
            O::TvAdd => tv_cell_addition(dst, src1, src2, size),
            O::TvSub => tv_cell_subtraction(dst, src1, src2, size),
            O::TvMul => tv_cell_multiplication(dst, src1, src2, size),
            O::FvAdd => fv_cell_addition(dst, src1, src2, size),
            O::FvSub => fv_cell_subtraction(dst, src1, src2, size),
            O::FvMul => fv_cell_multiplication(dst, src1, src2, size),
        }
    }
}

impl BytecodeInstruction for HeapBinaryDivMod {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapBinaryArithmetic as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            src_fv: (v >> 20) & 1 != 0,
            fill_x: (v >> 21) & 1 != 0,
            is_mod: (v >> 22) & 1 != 0,
            size: VectorSize::new(v >> 23),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapBinaryArithmetic as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | ((self.src_fv as u32) << 20)
                | ((self.fill_x as u32) << 21)
                | ((self.is_mod as u32) << 22)
                | (self.size.map_or(0, |v| v.get()) << 23),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs1,
            rs2,
            src_fv,
            fill_x,
            is_mod,
            size: _,
        } = self;
        let mnemonic = match (*src_fv, *fill_x, *is_mod) {
            (false, false, false) => "tv.heap_divz",
            (false, false, true) => "tv.heap_modz",
            (false, true, false) => "tv.heap_divx",
            (false, true, true) => "tv.heap_modx",

            (true, false, false) => "fv.heap_divz",
            (true, false, true) => "fv.heap_modz",
            (true, true, false) => "fv.heap_divx",
            (true, true, true) => "fv.heap_modx",
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs1}, {rs2}")
    }
    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs1,
            rs2,
            src_fv,
            fill_x,
            is_mod,
            size,
        } = self;
        let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
        let num_words = size.get().div_ceil(64) as usize;
        let mut src_num_words = num_words;
        let mut dst_num_words = num_words;
        if src_fv {
            src_num_words *= 2;
            dst_num_words *= 2;
        } else if fill_x {
            dst_num_words *= 2;
        }

        let [dst, src1, src2] = state.heap.get_u64_cell_slices([
            (
                HeapOffset {
                    bit_offset: regs[rd] as usize,
                },
                dst_num_words,
            ),
            (
                HeapOffset {
                    bit_offset: regs[rs1] as usize,
                },
                src_num_words,
            ),
            (
                HeapOffset {
                    bit_offset: regs[rs2] as usize,
                },
                src_num_words,
            ),
        ]);

        let complement_buffer = vec![Cell::new(0); dst_num_words];

        match (src_fv, is_mod) {
            (false, false) => tv_cell_divmod(dst, &complement_buffer, src1, src2, size, fill_x),
            (false, true) => tv_cell_divmod(&complement_buffer, dst, src1, src2, size, fill_x),
            (true, false) => fv_cell_divmod(dst, &complement_buffer, src1, src2, size, fill_x),
            (true, true) => fv_cell_divmod(&complement_buffer, dst, src1, src2, size, fill_x),
        }
    }
}

impl BytecodeInstruction for HeapCaseEq {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapCaseEq as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            ne: (v >> 20) & 1 != 0,
            num_words: NonZeroU16::new((v >> 21) as u16),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapCaseEq as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | ((self.ne as u32) << 20)
                | (self.num_words.map_or(0, |v| v.get() as u32) << 21),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs1,
            rs2,
            ne,
            num_words: _,
        } = self;
        let mnemonic = if *ne { "heap_cne" } else { "heap_ceq" };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs1}, {rs2}")
    }
    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs1,
            rs2,
            ne,
            num_words,
        } = self;
        let num_words = match num_words {
            None => regs[Reg::X12],
            Some(n) => n.get() as u64,
        };
        let num_words = num_words as usize;
        let src1 = state.heap.get_u64_slice(
            HeapOffset {
                bit_offset: regs[rs1] as usize,
            },
            num_words,
        );
        let src2 = state.heap.get_u64_slice(
            HeapOffset {
                bit_offset: regs[rs2] as usize,
            },
            num_words,
        );

        let is_eq = src1 == src2;

        regs[rd] = u64::from(is_eq ^ ne);
    }
}

#[derive(Clone, Copy)]
pub enum BitwiseOp {
    TvAnd,
    TvOr,
    TvXor,
    TvAndNot,
    TvOrNot,
    FvAnd,
    FvOr,
    FvXor,
    FvAndNot,
    FvOrNot,
}

impl BitwiseOp {
    pub fn is_four_value(self) -> bool {
        match self {
            Self::TvAnd | Self::TvOr | Self::TvXor | Self::TvAndNot | Self::TvOrNot => false,
            Self::FvAnd | Self::FvOr | Self::FvXor | Self::FvAndNot | Self::FvOrNot => true,
        }
    }

    pub fn new_masked(v: u32) -> Self {
        match v & 0x7 {
            0 => Self::TvAnd,
            1 => Self::TvOr,
            2 => Self::TvXor,
            3 => Self::TvAndNot,
            4 => Self::TvOrNot,
            8 => Self::FvAnd,
            9 => Self::FvOr,
            10 => Self::FvXor,
            11 => Self::FvAndNot,
            _ => Self::FvOrNot,
        }
    }
}

#[derive(Clone, Copy)]
pub enum ArithmeticOp {
    TvAdd,
    TvSub,
    TvMul,
    FvAdd,
    FvSub,
    FvMul,
}

impl ArithmeticOp {
    pub fn is_four_value(self) -> bool {
        match self {
            Self::TvAdd | Self::TvSub | Self::TvMul => false,
            Self::FvAdd | Self::FvSub | Self::FvMul => true,
        }
    }

    pub fn new_masked(v: u32) -> Self {
        match v & 0xF {
            0 => Self::TvAdd,
            1 => Self::TvSub,
            2 => Self::TvMul,
            7 => Self::FvAdd,
            8 => Self::FvSub,
            _ => Self::FvMul,
        }
    }
}

impl BytecodeEncoder {
    fn heap_ceq_impl(&mut self, rd: Reg, rs1: Reg, rs2: Reg, ne: bool, num_words: u32) {
        assert_ne!(num_words, 0);
        let num_words = if num_words >= (1u32 << 11) {
            self.load_u64(Reg::X12, num_words as u64);
            None
        } else {
            Some(NonZeroU16::new(num_words as u16).unwrap())
        };
        self.data.push(
            HeapCaseEq {
                rd,
                rs1,
                rs2,
                ne,
                num_words,
            }
            .encode(),
        );
    }

    pub fn heap_ceq(&mut self, rd: Reg, rs1: Reg, rs2: Reg, num_words: u32) {
        self.heap_ceq_impl(rd, rs1, rs2, false, num_words);
    }
    pub fn heap_cne(&mut self, rd: Reg, rs1: Reg, rs2: Reg, num_words: u32) {
        self.heap_ceq_impl(rd, rs1, rs2, true, num_words);
    }

    fn heap_binary_bitwise(
        &mut self,
        rd: Reg,
        rs1: Reg,
        rs2: Reg,
        op: BitwiseOp,
        size: VectorSize,
    ) {
        let size = if size.get() >= (1u32 << 8) {
            self.load_u64(Reg::X12, size.get() as u64);
            None
        } else {
            Some(size)
        };
        self.data.push(
            HeapBinaryBitwise {
                rd,
                rs1,
                rs2,
                op,
                size,
            }
            .encode(),
        );
    }
    fn heap_binary_arith(
        &mut self,
        rd: Reg,
        rs1: Reg,
        rs2: Reg,
        op: ArithmeticOp,
        size: VectorSize,
    ) {
        let size = if size.get() >= (1u32 << 8) {
            self.load_u64(Reg::X12, size.get() as u64);
            None
        } else {
            Some(size)
        };
        self.data.push(
            HeapBinaryArithmetic {
                rd,
                rs1,
                rs2,
                op,
                size,
            }
            .encode(),
        );
    }
    fn heap_binary_divmod(
        &mut self,
        rd: Reg,
        rs1: Reg,
        rs2: Reg,
        src_fv: bool,
        fill_x: bool,
        is_mod: bool,
        size: VectorSize,
    ) {
        let size = if size.get() >= (1u32 << 8) {
            self.load_u64(Reg::X12, size.get() as u64);
            None
        } else {
            Some(size)
        };
        self.data.push(
            HeapBinaryDivMod {
                rd,
                rs1,
                rs2,
                src_fv,
                fill_x,
                is_mod,
                size,
            }
            .encode(),
        );
    }

    pub fn heap_tv_and(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::TvAnd, size);
    }
    pub fn heap_tv_or(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::TvOr, size);
    }
    pub fn heap_tv_xor(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::TvXor, size);
    }
    pub fn heap_tv_andnot(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::TvAndNot, size);
    }
    pub fn heap_tv_ornot(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::TvOrNot, size);
    }
    pub fn heap_fv_and(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::FvAnd, size);
    }
    pub fn heap_fv_or(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::FvOr, size);
    }
    pub fn heap_fv_xor(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::FvXor, size);
    }

    pub fn heap_tv_add(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_arith(rd, rs1, rs2, ArithmeticOp::TvAdd, size);
    }
    pub fn heap_tv_sub(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_arith(rd, rs1, rs2, ArithmeticOp::TvSub, size);
    }
    pub fn heap_tv_mul(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_arith(rd, rs1, rs2, ArithmeticOp::TvMul, size);
    }
    pub fn heap_fv_add(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_arith(rd, rs1, rs2, ArithmeticOp::FvAdd, size);
    }
    pub fn heap_fv_sub(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_arith(rd, rs1, rs2, ArithmeticOp::FvSub, size);
    }
    pub fn heap_fv_mul(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_arith(rd, rs1, rs2, ArithmeticOp::FvMul, size);
    }

    pub fn heap_tv_divx(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_divmod(rd, rs1, rs2, false, true, false, size);
    }
    pub fn heap_tv_div0(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_divmod(rd, rs1, rs2, false, false, false, size);
    }
    pub fn heap_fv_divx(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_divmod(rd, rs1, rs2, true, true, false, size);
    }
    pub fn heap_fv_div0(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_divmod(rd, rs1, rs2, true, false, false, size);
    }
    pub fn heap_tv_modx(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_divmod(rd, rs1, rs2, false, true, true, size);
    }
    pub fn heap_tv_mod0(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_divmod(rd, rs1, rs2, false, false, true, size);
    }
    pub fn heap_fv_modx(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_divmod(rd, rs1, rs2, true, true, true, size);
    }
    pub fn heap_fv_mod0(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_divmod(rd, rs1, rs2, true, false, true, size);
    }
}
