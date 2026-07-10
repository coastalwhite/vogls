use std::cell::Cell;
use std::fmt;

use vogls_bits::arithmetic::{
    fv_bin_u64_cell_bitwise_op, fv_bitwise_and_elem, fv_bitwise_andnot_elem, fv_bitwise_or_elem,
    fv_bitwise_ornot_elem, fv_bitwise_xor_elem, fv_cell_addition, fv_cell_contains_special,
    fv_cell_divmod, fv_cell_multiplication, fv_cell_power, fv_cell_subtraction,
    tv_bin_u64_cell_bitwise_mask_last_op, tv_bin_u64_cell_bitwise_op, tv_cell_addition,
    tv_cell_divmod, tv_cell_multiplication, tv_cell_power, tv_cell_subtraction,
};
use vogls_bits::comparison::{fv_cell_unsigned_leq, tv_cell_unsigned_leq};
use vogls_bits::copyxz::{copy_x, copy_z};
use vogls_bits::format::{BitsDisplay, BitsFormatOptions};
use vogls_bits::negate::{fv_cell_negate, tv_cell_negate};
use vogls_bits::reduce::{
    fv_l_reduce_and, fv_l_reduce_or, fv_l_reduce_xor, tv_reduce_and, tv_reduce_or, tv_reduce_xor,
};
use vogls_bits::shift::{
    fv_cell_arithmetic_shift_right, fv_cell_logical_shift_left, fv_cell_logical_shift_right,
    tv_cell_arithmetic_shift_right, tv_cell_logical_shift_left, tv_cell_logical_shift_right,
};
use vogls_bits::util::CellSlice;
use vogls_codegen::HeapOffset;
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RuntimeState;

use crate::bytecode::{write_padded_mnemonic, write_register};

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    EXEC_ITRACE_INDENT, InlineNBitSize, Schedule,
};
pub struct HeapBinaryBitwise {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    op: BitwiseOp,
    size: InlineNBitSize<8>,
}
pub struct HeapBinaryArithmetic {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    op: ArithmeticOp,
    size: InlineNBitSize<8>,
}
pub struct HeapBinaryDivMod {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    src_fv: bool,
    fill_x: bool,
    is_mod: bool,
    size: InlineNBitSize<9>,
}
pub struct HeapBinaryCmp {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    op: CompareOp,
    size: InlineNBitSize<10>,
}
pub struct HeapBinaryMinMax {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    is_fv: bool,
    is_max: bool,
    size: InlineNBitSize<10>,
}
pub struct HeapCaseEq {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    ne: bool,
    num_words: InlineNBitSize<13>,
}
pub struct HeapBinaryShift {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    op: ShiftOp,
    size: InlineNBitSize<9>,
}
pub struct HeapUnary {
    rd: Reg,
    rs: Reg,
    op: UnaryOp,
    size: InlineNBitSize<12>,
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
            size: InlineNBitSize::new_masked(v >> 24),
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
                | (self.size.encode() << 24),
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
            BitwiseOp::FvCopyX => "fv.heap_copyx",
            BitwiseOp::FvCopyZ => "fv.heap_copyz",
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs1}, {rs2}")
    }
    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        regs: &Regs,
        state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        let size = self.size.get(regs);
        let mode = if self.op.is_four_value() {
            LogicMode::FourValue
        } else {
            LogicMode::TwoValue
        };
        let rs1 = state
            .heap
            .load_bits(regs.get_as_addr(self.rs1).to_ref(size), mode);
        let rs2 = state
            .heap
            .load_bits(regs.get_as_addr(self.rs2).to_ref(size), mode);
        writeln!(
            f,
            "rs1 = {}, rs2 = {}",
            rs1.display(&BitsFormatOptions::default()),
            rs2.display(&BitsFormatOptions::default())
        )
    }
    fn post_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        regs: &Regs,
        state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        let size = self.size.get(regs);
        let mode = if self.op.is_four_value() {
            LogicMode::FourValue
        } else {
            LogicMode::TwoValue
        };
        let rd = state
            .heap
            .load_bits(regs.get_as_addr(self.rd).to_ref(size), mode);
        writeln!(f, "rd = {}", rd.display(&BitsFormatOptions::default()))
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
        let size = size.get(regs);
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
            O::FvCopyX => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
                copy_x(lspc, lval, rspc, rval)
            }),
            O::FvCopyZ => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
                copy_z(lspc, lval, rspc, rval)
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
            size: InlineNBitSize::new_masked(v >> 24),
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
                | (self.size.encode() << 24),
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
            ArithmeticOp::TvPow => "tv.heap_pow",
            ArithmeticOp::FvAdd => "fv.heap_add",
            ArithmeticOp::FvSub => "fv.heap_sub",
            ArithmeticOp::FvMul => "fv.heap_mul",
            ArithmeticOp::FvPow => "fv.heap_pow",
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
        let size = size.get(regs);
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
            O::TvPow => tv_cell_power(dst, src1, src2, size),
            O::FvAdd => fv_cell_addition(dst, src1, src2, size),
            O::FvSub => fv_cell_subtraction(dst, src1, src2, size),
            O::FvMul => fv_cell_multiplication(dst, src1, src2, size),
            O::FvPow => fv_cell_power(dst, src1, src2, size),
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
            size: InlineNBitSize::new_masked(v >> 23),
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
                | (self.size.encode() << 23),
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
        let size = size.get(regs);
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

impl BytecodeInstruction for HeapBinaryCmp {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapBinaryCmp as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            op: CompareOp::new_masked(v >> 20),
            size: InlineNBitSize::new_masked(v >> 22),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapBinaryCmp as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | ((self.op as u32) << 20)
                | (self.size.encode() << 22),
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
            CompareOp::TvUnsignedLeq => "tv.heap_uleq",
            CompareOp::TvUnsignedGt => "tv.heap_ugt",
            CompareOp::FvUnsignedLeq => "fv.heap_uleq",
            CompareOp::FvUnsignedGt => "fv.heap_ugt",
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
        let size = size.get(regs);
        let mut num_words = size.get().div_ceil(64) as usize;
        if op.is_four_value() {
            num_words *= 2;
        }
        let [src1, src2] = state.heap.get_u64_cell_slices([
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

        use CompareOp as O;
        match op {
            O::TvUnsignedLeq => regs[rd] = u64::from(tv_cell_unsigned_leq(src1, src2, size)),
            O::TvUnsignedGt => regs[rd] = u64::from(!tv_cell_unsigned_leq(src1, src2, size)),
            O::FvUnsignedLeq => {
                let is_leq = fv_cell_unsigned_leq(src1, src2, size);
                let (spc, val) = rd.to_spc_and_val();
                regs[spc] = is_leq.spc().into();
                regs[val] = is_leq.val().into();
            }
            O::FvUnsignedGt => {
                let is_leq = fv_cell_unsigned_leq(src1, src2, size);
                let is_leq = !is_leq;
                let (spc, val) = rd.to_spc_and_val();
                regs[spc] = is_leq.spc().into();
                regs[val] = is_leq.val().into();
            }
        }
    }
}

impl BytecodeInstruction for HeapBinaryShift {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapBinaryShift as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            op: ShiftOp::new_masked(v >> 20),
            size: InlineNBitSize::new_masked(v >> 23),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapBinaryShift as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | ((self.op as u32) << 20)
                | (self.size.encode() << 23),
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
            ShiftOp::TvSll => "tv.sll",
            ShiftOp::TvSlr => "tv.slr",
            ShiftOp::TvSar => "tv.sar",
            ShiftOp::FvSll => "fv.sll",
            ShiftOp::FvSlr => "fv.slr",
            ShiftOp::FvSar => "fv.sar",
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
        let size = size.get(regs);
        let mut num_words = size.get().div_ceil(64) as usize;
        if op.is_four_value() {
            num_words *= 2;
        }
        let [dst, src] = state.heap.get_u64_cell_slices([
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
        ]);

        let shift = if op.is_four_value() {
            let (spc, val) = rs2.to_spc_and_val();
            if regs[spc] != u32::MAX as u64 {
                dst.iter().for_each(|d| d.set(0));
            }
            regs[val] as u32
        } else {
            regs[rs2] as u32
        };

        use ShiftOp as O;
        match op {
            O::TvSll => tv_cell_logical_shift_left(dst, src, shift, size),
            O::TvSlr => tv_cell_logical_shift_right(dst, src, shift, size),
            O::TvSar => tv_cell_arithmetic_shift_right(dst, src, shift, size),
            O::FvSll => fv_cell_logical_shift_left(dst, src, shift, size),
            O::FvSlr => fv_cell_logical_shift_right(dst, src, shift, size),
            O::FvSar => fv_cell_arithmetic_shift_right(dst, src, shift, size),
        }
    }
}

impl BytecodeInstruction for HeapBinaryMinMax {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapBinaryMinMax as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            is_fv: (v >> 20) & 1 != 0,
            is_max: (v >> 21) & 1 != 0,
            size: InlineNBitSize::new_masked(v >> 22),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapBinaryMinMax as u32
                | ((self.rd as u32) << 8)
                | ((self.rs1 as u32) << 12)
                | ((self.rs2 as u32) << 16)
                | ((self.is_fv as u32) << 20)
                | ((self.is_max as u32) << 21)
                | (self.size.encode() << 22),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs1,
            rs2,
            is_fv,
            is_max,
            size: _,
        } = self;
        let mnemonic = match (*is_fv, *is_max) {
            (false, false) => "tv.heap_min",
            (false, true) => "tv.heap_max",
            (true, false) => "fv.heap_min",
            (true, true) => "fv.heap_max",
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
            is_fv,
            is_max,
            size,
        } = self;
        let size = size.get(regs);
        let mut num_words = size.get().div_ceil(64) as usize;
        let mut offset = 0;
        if is_fv {
            offset = num_words;
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

        if is_fv && (fv_cell_contains_special(src1, size) || fv_cell_contains_special(src2, size)) {
            dst.iter().for_each(|v| v.set(0));
            return;
        }

        let is_rhs_max = tv_cell_unsigned_leq(&src1[offset..], &src2[offset..], size);
        if is_max ^ is_rhs_max {
            dst.iter().zip(src1).for_each(|(d, s)| d.set(s.get()));
        } else {
            dst.iter().zip(src2).for_each(|(d, s)| d.set(s.get()));
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
            num_words: InlineNBitSize::new_masked(v >> 21),
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
                | (self.num_words.encode() << 21),
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
    fn post_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rd", self.rd, LogicMode::TwoValue)?;
        writeln!(f)
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
        let num_words = num_words.get(regs).get() as usize;
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

impl BytecodeInstruction for HeapUnary {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::HeapUnary as u8);
        let v = c.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            op: UnaryOp::new_masked(v >> 16),
            size: InlineNBitSize::new_masked(v >> 20),
        }
    }
    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapUnary as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.op as u32) << 16)
                | (self.size.encode() << 20),
        )
    }
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            op,
            size: _,
        } = self;
        let mnemonic = match op {
            UnaryOp::TvNeg => "tv.heap_neg",
            UnaryOp::TvCopy => "tv.heap_copy",
            UnaryOp::TvReduceOr => "tv.heap_reduce_or",
            UnaryOp::TvReduceAnd => "tv.heap_reduce_and",
            UnaryOp::TvReduceXor => "tv.heap_reduce_xor",
            UnaryOp::FvNeg => "fv.heap_neg",
            UnaryOp::FvCopy => "fv.heap_copy",
            UnaryOp::FvReduceOr => "fv.heap_reduce_or",
            UnaryOp::FvReduceAnd => "fv.heap_reduce_and",
            UnaryOp::FvReduceXor => "fv.heap_reduce_xor",
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs}")
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
        let Self { rd, rs, op, size } = self;
        let size = size.get(regs);
        let mut num_words = size.get().div_ceil(64) as usize;
        if op.is_four_value() {
            num_words *= 2;
        }

        use UnaryOp as O;
        match op {
            O::TvNeg => {
                let [dst, src] = state.heap.get_u64_cell_slices([
                    (regs.get_as_addr(rd), num_words),
                    (regs.get_as_addr(rs), num_words),
                ]);
                tv_cell_negate(dst, src, size);
            }
            O::TvCopy => {
                let [dst, src] = state.heap.get_u64_cell_slices([
                    (regs.get_as_addr(rd), num_words),
                    (regs.get_as_addr(rs), num_words),
                ]);
                dst.copy_from_slice(src);
            }
            O::TvReduceOr => {
                regs[rd] = u64::from(tv_reduce_or(
                    state.heap.get_u64_slice(regs.get_as_addr(rs), num_words),
                ));
            }
            O::TvReduceAnd => {
                regs[rd] = u64::from(tv_reduce_and(
                    state.heap.get_u64_slice(regs.get_as_addr(rs), num_words),
                    size,
                ));
            }
            O::TvReduceXor => {
                regs[rd] = u64::from(tv_reduce_xor(
                    state.heap.get_u64_slice(regs.get_as_addr(rs), num_words),
                ));
            }
            O::FvNeg => {
                let [dst, src] = state.heap.get_u64_cell_slices([
                    (regs.get_as_addr(rd), num_words),
                    (regs.get_as_addr(rs), num_words),
                ]);
                fv_cell_negate(dst, src, size);
            }
            O::FvCopy => {
                let [dst, src] = state.heap.get_u64_cell_slices([
                    (regs.get_as_addr(rd), num_words),
                    (regs.get_as_addr(rs), num_words),
                ]);
                dst.copy_from_slice(src);
            }
            O::FvReduceOr => {
                let (spc, val) = rd.to_spc_and_val();
                let value = fv_l_reduce_or(
                    state.heap.get_u64_slice(regs.get_as_addr(rs), num_words),
                    size,
                );
                regs[spc] = u64::from(value.spc());
                regs[val] = u64::from(value.val());
            }
            O::FvReduceAnd => {
                let (spc, val) = rd.to_spc_and_val();
                let value = fv_l_reduce_and(
                    state.heap.get_u64_slice(regs.get_as_addr(rs), num_words),
                    size,
                );
                regs[spc] = u64::from(value.spc());
                regs[val] = u64::from(value.val());
            }
            O::FvReduceXor => {
                let (spc, val) = rd.to_spc_and_val();
                let value = fv_l_reduce_xor(
                    state.heap.get_u64_slice(regs.get_as_addr(rs), num_words),
                    size,
                );
                regs[spc] = u64::from(value.spc());
                regs[val] = u64::from(value.val());
            }
        }
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
    FvCopyX,
    FvCopyZ,
}

impl BitwiseOp {
    pub fn is_four_value(self) -> bool {
        match self {
            Self::TvAnd | Self::TvOr | Self::TvXor | Self::TvAndNot | Self::TvOrNot => false,
            Self::FvAnd
            | Self::FvOr
            | Self::FvXor
            | Self::FvAndNot
            | Self::FvOrNot
            | Self::FvCopyX
            | Self::FvCopyZ => true,
        }
    }

    pub fn new_masked(v: u32) -> Self {
        match v & 0xF {
            0 => Self::TvAnd,
            1 => Self::TvOr,
            2 => Self::TvXor,
            3 => Self::TvAndNot,
            4 => Self::TvOrNot,
            5 => Self::FvAnd,
            6 => Self::FvOr,
            7 => Self::FvXor,
            8 => Self::FvAndNot,
            9 => Self::FvOrNot,
            10 => Self::FvCopyX,
            _ => Self::FvCopyZ,
        }
    }
}

#[derive(Clone, Copy)]
pub enum ArithmeticOp {
    TvAdd,
    TvSub,
    TvMul,
    TvPow,
    FvAdd,
    FvSub,
    FvMul,
    FvPow,
}

impl ArithmeticOp {
    pub fn is_four_value(self) -> bool {
        match self {
            Self::TvAdd | Self::TvSub | Self::TvMul | Self::TvPow => false,
            Self::FvAdd | Self::FvSub | Self::FvMul | Self::FvPow => true,
        }
    }

    pub fn new_masked(v: u32) -> Self {
        match v & 0xF {
            0 => Self::TvAdd,
            1 => Self::TvSub,
            2 => Self::TvMul,
            3 => Self::TvPow,
            4 => Self::FvAdd,
            5 => Self::FvSub,
            6 => Self::FvMul,
            _ => Self::FvPow,
        }
    }
}

#[derive(Clone, Copy)]
pub enum CompareOp {
    TvUnsignedLeq,
    TvUnsignedGt,
    FvUnsignedLeq,
    FvUnsignedGt,
}

impl CompareOp {
    pub fn is_four_value(self) -> bool {
        match self {
            Self::TvUnsignedLeq | Self::TvUnsignedGt => false,
            Self::FvUnsignedLeq | Self::FvUnsignedGt => true,
        }
    }

    pub fn new_masked(v: u32) -> Self {
        match v & 0x3 {
            0 => Self::TvUnsignedLeq,
            1 => Self::TvUnsignedGt,
            2 => Self::FvUnsignedLeq,
            _ => Self::FvUnsignedGt,
        }
    }
}

#[derive(Clone, Copy)]
pub enum ShiftOp {
    TvSll,
    TvSlr,
    TvSar,
    FvSll,
    FvSlr,
    FvSar,
}

impl ShiftOp {
    pub fn is_four_value(self) -> bool {
        match self {
            Self::TvSll | Self::TvSlr | Self::TvSar => false,
            Self::FvSll | Self::FvSlr | Self::FvSar => true,
        }
    }

    pub fn new_masked(v: u32) -> Self {
        match v & 0x7 {
            0 => Self::TvSll,
            1 => Self::TvSlr,
            2 => Self::TvSar,
            3 => Self::FvSll,
            4 => Self::FvSlr,
            _ => Self::FvSar,
        }
    }
}

#[derive(Clone, Copy)]
pub enum UnaryOp {
    TvNeg,
    TvCopy,
    TvReduceOr,
    TvReduceAnd,
    TvReduceXor,
    FvNeg,
    FvCopy,
    FvReduceOr,
    FvReduceAnd,
    FvReduceXor,
}

impl UnaryOp {
    pub fn is_four_value(self) -> bool {
        match self {
            Self::TvNeg
            | Self::TvCopy
            | Self::TvReduceOr
            | Self::TvReduceAnd
            | Self::TvReduceXor => false,
            Self::FvNeg
            | Self::FvCopy
            | Self::FvReduceOr
            | Self::FvReduceAnd
            | Self::FvReduceXor => true,
        }
    }

    pub fn new_masked(v: u32) -> Self {
        match v & 0xF {
            0 => Self::TvNeg,
            1 => Self::TvCopy,
            2 => Self::TvReduceOr,
            3 => Self::TvReduceAnd,
            4 => Self::TvReduceXor,
            5 => Self::FvNeg,
            6 => Self::FvCopy,
            7 => Self::FvReduceOr,
            8 => Self::FvReduceAnd,
            _ => Self::FvReduceXor,
        }
    }
}

impl BytecodeEncoder {
    fn heap_ceq_impl(&mut self, rd: Reg, rs1: Reg, rs2: Reg, ne: bool, num_words: u32) {
        assert_ne!(num_words, 0);
        let num_words = InlineNBitSize::new(VectorSize::new(num_words).unwrap(), self);
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
        let size = InlineNBitSize::new(size, self);
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
        let size = InlineNBitSize::new(size, self);
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
    fn heap_binary_cmp(&mut self, rd: Reg, rs1: Reg, rs2: Reg, op: CompareOp, size: VectorSize) {
        let size = InlineNBitSize::new(size, self);
        self.data.push(
            HeapBinaryCmp {
                rd,
                rs1,
                rs2,
                op,
                size,
            }
            .encode(),
        );
    }
    fn heap_binary_shift(&mut self, rd: Reg, rs1: Reg, rs2: Reg, op: ShiftOp, size: VectorSize) {
        let size = InlineNBitSize::new(size, self);
        self.data.push(
            HeapBinaryShift {
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
        let size = InlineNBitSize::new(size, self);
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
    fn heap_binary_minmax(
        &mut self,
        rd: Reg,
        rs1: Reg,
        rs2: Reg,
        is_fv: bool,
        is_max: bool,
        size: VectorSize,
    ) {
        let size = InlineNBitSize::new(size, self);
        self.data.push(
            HeapBinaryMinMax {
                rd,
                rs1,
                rs2,
                is_fv,
                is_max,
                size,
            }
            .encode(),
        );
    }
    fn heap_unary(&mut self, rd: Reg, rs: Reg, op: UnaryOp, size: VectorSize) {
        let size = InlineNBitSize::new(size, self);
        self.data.push(HeapUnary { rd, rs, op, size }.encode());
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
    pub fn heap_fv_copyx(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::FvCopyX, size);
    }
    pub fn heap_fv_copyz(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_bitwise(rd, rs1, rs2, BitwiseOp::FvCopyZ, size);
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
    pub fn heap_tv_pow(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_arith(rd, rs1, rs2, ArithmeticOp::TvPow, size);
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
    pub fn heap_fv_pow(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_arith(rd, rs1, rs2, ArithmeticOp::FvPow, size);
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

    pub fn heap_tv_unsigned_leq(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_cmp(rd, rs1, rs2, CompareOp::TvUnsignedLeq, size);
    }
    pub fn heap_tv_unsigned_gt(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_cmp(rd, rs1, rs2, CompareOp::TvUnsignedGt, size);
    }
    pub fn heap_fv_unsigned_leq(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_cmp(rd, rs1, rs2, CompareOp::FvUnsignedLeq, size);
    }
    pub fn heap_fv_unsigned_gt(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_cmp(rd, rs1, rs2, CompareOp::FvUnsignedGt, size);
    }

    pub fn heap_tv_unsigned_geq(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_unsigned_leq(rd, rs2, rs1, size);
    }
    pub fn heap_tv_unsigned_lt(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_unsigned_gt(rd, rs2, rs1, size);
    }
    pub fn heap_fv_unsigned_geq(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_cmp(rd, rs1, rs2, CompareOp::FvUnsignedLeq, size);
    }
    pub fn heap_fv_unsigned_lt(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_cmp(rd, rs1, rs2, CompareOp::FvUnsignedGt, size);
    }

    pub fn heap_tv_min(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_minmax(rd, rs1, rs2, false, false, size);
    }
    pub fn heap_tv_max(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_minmax(rd, rs1, rs2, false, true, size);
    }
    pub fn heap_fv_min(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_minmax(rd, rs1, rs2, true, false, size);
    }
    pub fn heap_fv_max(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_minmax(rd, rs1, rs2, true, true, size);
    }

    pub fn heap_tv_sll(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_shift(rd, rs1, rs2, ShiftOp::TvSll, size);
    }
    pub fn heap_tv_slr(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_shift(rd, rs1, rs2, ShiftOp::TvSlr, size);
    }
    pub fn heap_tv_sar(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_shift(rd, rs1, rs2, ShiftOp::TvSar, size);
    }
    pub fn heap_fv_sll(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_shift(rd, rs1, rs2, ShiftOp::FvSll, size);
    }
    pub fn heap_fv_slr(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_shift(rd, rs1, rs2, ShiftOp::FvSlr, size);
    }
    pub fn heap_fv_sar(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_binary_shift(rd, rs1, rs2, ShiftOp::FvSar, size);
    }

    pub fn heap_tv_neg(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::TvNeg, size);
    }
    pub fn heap_tv_copy(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::TvCopy, size);
    }
    pub fn heap_tv_reduce_or(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::TvReduceOr, size);
    }
    pub fn heap_tv_reduce_and(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::TvReduceAnd, size);
    }
    pub fn heap_tv_reduce_xor(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::TvReduceXor, size);
    }
    pub fn heap_fv_neg(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::FvNeg, size);
    }
    pub fn heap_fv_copy(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::FvCopy, size);
    }
    pub fn heap_fv_reduce_or(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::FvReduceOr, size);
    }
    pub fn heap_fv_reduce_and(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::FvReduceAnd, size);
    }
    pub fn heap_fv_reduce_xor(&mut self, rd: Reg, rs: Reg, size: VectorSize) {
        self.heap_unary(rd, rs, UnaryOp::FvReduceXor, size);
    }
}
