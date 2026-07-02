use std::fmt;
use std::num::NonZeroU16;

use vogls_bits::arithmetic::{
    fv_bin_u64_cell_bitwise_op, fv_bitwise_and_elem, fv_bitwise_andnot_elem, fv_bitwise_or_elem,
    fv_bitwise_ornot_elem, fv_bitwise_xor_elem, tv_bin_u64_cell_bitwise_mask_last_op,
    tv_bin_u64_cell_bitwise_op,
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
        matches!(
            self,
            Self::FvAnd | Self::FvOr | Self::FvXor | Self::FvAndNot | Self::FvOrNot
        )
    }

    pub fn new_masked(v: u32) -> Self {
        match v & 0x7 {
            0 => Self::TvAnd,
            1 => Self::TvOr,
            2 => Self::TvXor,
            3 => Self::TvAndNot,
            4 => Self::TvOrNot,
            5 => Self::FvAnd,
            6 => Self::FvOr,
            7 => Self::FvXor,
            8 => Self::FvAndNot,
            _ => Self::FvOrNot,
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
        self.data.push(HeapBinaryBitwise { rd, rs1, rs2, op, size }.encode());
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
}
