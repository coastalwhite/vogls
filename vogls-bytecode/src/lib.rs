mod lower;
mod ops;

pub use lower::lower_to_bytecode;
use vogls_codegen::{Heap, HeapOffset};
use vogls_ir::{LogicMode, VectorSize};

use self::ops::{BytecodeOp, Regs};

pub fn execute(ops: &[BytecodeInstruction], pc: &mut usize, heap: &mut Heap) {
    let mut regs = ops::Regs {
        size1offset: 0,
        size2offset: 0,
        size4offset: 0,
        size8offset: 0,
        size16offset: 0,
        size32offset: 0,
        size64offset: 0,
    };
    while let Some(&op) = ops.get(*pc) {
        *pc += 1;
        let opcode = op.opcode();
        let f = ops::OP_TABLE[opcode as usize];
        f(op, heap, ops, pc, &mut regs);
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct BytecodeInstruction {
    inner: u32,
}

macro_rules! bitwise_op {
    () => {
        fn and(
            ops: &mut Vec<BytecodeInstruction>,
            d: HeapOffset,
            s1: HeapOffset,
            s2: HeapOffset,
            mode: LogicMode,
            size: VectorSize,
        ) {
            match SizeVariant::new(size, mode) {
                SizeVariant::Tv1 => Self::encode_dlr(ops, d, s1, s2),
                SizeVariant::Tv2 => todo!(),
                SizeVariant::Tv4 => todo!(),
                SizeVariant::Tv8 => todo!(),
                SizeVariant::Tv16 => todo!(),
                SizeVariant::Tv32 => todo!(),
                SizeVariant::Tv64 => todo!(),
                SizeVariant::Tv64p => todo!(),
                SizeVariant::Fv1 => todo!(),
                SizeVariant::Fv2 => todo!(),
                SizeVariant::Fv4 => todo!(),
                SizeVariant::Fv8 => todo!(),
                SizeVariant::Fv16 => todo!(),
                SizeVariant::Fv32 => todo!(),
                SizeVariant::Fv32p => todo!(),
            }
        }
    };
}

impl BytecodeInstruction {
    #[inline(always)]
    pub fn opcode(self) -> u8 {
        (self.inner >> 22) as u8
    }

    #[inline(always)]
    fn encode_opcode(op: BytecodeOp) -> u32 {
        (op as u32) << 22
    }

    fn encode_ds(
        ops: &mut Vec<BytecodeInstruction>,
        op: BytecodeOp,
        dst: HeapOffset,
        src: HeapOffset,
    ) {
        let src = Self::offset_to::<11>(ops, src, 1);
        let dst = Self::offset_to::<11>(ops, dst, 1);
        ops.push(Self {
            inner: Self::encode_opcode(op) | (dst << 11) | src,
        });
    }
    fn decode_ds(
        self,
        ops: &[BytecodeInstruction],
        pc: &mut usize,
        regs: &Regs,
    ) -> (HeapOffset, HeapOffset) {
        let src = Self::offset_from::<11>(self, ops, pc, regs, 1, 0);
        let dst = Self::offset_from::<11>(self, ops, pc, regs, 1, 11);
        (dst, src)
    }

    fn encode_dlr(
        ops: &mut Vec<BytecodeInstruction>,
        op: BytecodeOp,
        dst: HeapOffset,
        lhs: HeapOffset,
        rhs: HeapOffset,
    ) {
        let lhs = Self::offset_to::<7>(ops, lhs, 1);
        let rhs = Self::offset_to::<7>(ops, rhs, 1);
        let dst = Self::offset_to::<8>(ops, dst, 1);
        ops.push(Self {
            inner: Self::encode_opcode(op) | (dst << 14) | (rhs << 7) | lhs,
        });
    }
    fn decode_dlr(
        self,
        ops: &[BytecodeInstruction],
        pc: &mut usize,
        regs: &Regs,
    ) -> (HeapOffset, HeapOffset, HeapOffset) {
        let lhs = Self::offset_from::<7>(self, ops, pc, regs, 1, 0);
        let rhs = Self::offset_from::<7>(self, ops, pc, regs, 1, 7);
        let dst = Self::offset_from::<8>(self, ops, pc, regs, 1, 14);
        (dst, lhs, rhs)
    }

    fn offset_to<const N: usize>(
        ops: &mut Vec<BytecodeInstruction>,
        offset: HeapOffset,
        align: usize,
    ) -> u32 {
        const {
            assert!(N <= 22);
        }
        debug_assert_eq!(offset.bit_offset % align, 0);
        let offset = offset.bit_offset / align;
        let max = (1usize << N) - 1;
        if offset >= max {
            Self::push_operand(ops, offset);
            max as u32
        } else {
            offset as u32
        }
    }

    #[inline(always)]
    fn offset_from<const N: usize>(
        op: BytecodeInstruction,
        ops: &[BytecodeInstruction],
        pc: &mut usize,
        regs: &Regs,
        align: usize,
        lsb: u32,
    ) -> HeapOffset {
        debug_assert!(lsb < 22);
        let max = (1u32 << N) - 1;
        let offset = (op.inner >> lsb) & max;
        let offset = if offset == max {
            Self::pop_operand(ops, pc)
        } else {
            offset
        };
        HeapOffset {
            bit_offset: regs.size1offset + offset as usize * align,
        }
    }

    fn push_operand(ops: &mut Vec<BytecodeInstruction>, operand: usize) -> u32 {
        todo!()
    }
    fn pop_operand(ops: &[BytecodeInstruction], pc: &mut usize) -> u32 {
        todo!()
    }

    fn negate(
        ops: &mut Vec<BytecodeInstruction>,
        d: HeapOffset,
        s: HeapOffset,
        mode: LogicMode,
        size: VectorSize,
    ) {
        match SizeVariant::new(size, mode) {
            SizeVariant::Tv1 => Self::not1(ops, d, s),
            SizeVariant::Tv2 => todo!(),
            SizeVariant::Tv4 => todo!(),
            SizeVariant::Tv8 => todo!(),
            SizeVariant::Tv16 => todo!(),
            SizeVariant::Tv32 => todo!(),
            SizeVariant::Tv64 => todo!(),
            SizeVariant::Tv64p => todo!(),
            SizeVariant::Fv1 => todo!(),
            SizeVariant::Fv2 => todo!(),
            SizeVariant::Fv4 => todo!(),
            SizeVariant::Fv8 => todo!(),
            SizeVariant::Fv16 => todo!(),
            SizeVariant::Fv32 => todo!(),
            SizeVariant::Fv32p => todo!(),
        }
    }

    fn reduce_or(
        ops: &mut Vec<BytecodeInstruction>,
        d: HeapOffset,
        s: HeapOffset,
        mode: LogicMode,
        size: VectorSize,
    ) {
        match SizeVariant::new(size, mode) {
            SizeVariant::Tv1 => Self::move1(ops, d, s),
            SizeVariant::Tv2 => todo!(),
            SizeVariant::Tv4 => todo!(),
            SizeVariant::Tv8 => todo!(),
            SizeVariant::Tv16 => todo!(),
            SizeVariant::Tv32 => todo!(),
            SizeVariant::Tv64 => todo!(),
            SizeVariant::Tv64p => todo!(),
            SizeVariant::Fv1 => todo!(),
            SizeVariant::Fv2 => todo!(),
            SizeVariant::Fv4 => todo!(),
            SizeVariant::Fv8 => todo!(),
            SizeVariant::Fv16 => todo!(),
            SizeVariant::Fv32 => todo!(),
            SizeVariant::Fv32p => todo!(),
        }
    }
    fn reduce_and(
        ops: &mut Vec<BytecodeInstruction>,
        d: HeapOffset,
        s: HeapOffset,
        mode: LogicMode,
        size: VectorSize,
    ) {
        match SizeVariant::new(size, mode) {
            SizeVariant::Tv1 => Self::move1(ops, d, s),
            SizeVariant::Tv2 => todo!(),
            SizeVariant::Tv4 => todo!(),
            SizeVariant::Tv8 => todo!(),
            SizeVariant::Tv16 => todo!(),
            SizeVariant::Tv32 => todo!(),
            SizeVariant::Tv64 => todo!(),
            SizeVariant::Tv64p => todo!(),
            SizeVariant::Fv1 => todo!(),
            SizeVariant::Fv2 => todo!(),
            SizeVariant::Fv4 => todo!(),
            SizeVariant::Fv8 => todo!(),
            SizeVariant::Fv16 => todo!(),
            SizeVariant::Fv32 => todo!(),
            SizeVariant::Fv32p => todo!(),
        }
    }
    fn reduce_xor(
        ops: &mut Vec<BytecodeInstruction>,
        d: HeapOffset,
        s: HeapOffset,
        mode: LogicMode,
        size: VectorSize,
    ) {
        match SizeVariant::new(size, mode) {
            SizeVariant::Tv1 => Self::move1(ops, d, s),
            SizeVariant::Tv2 => todo!(),
            SizeVariant::Tv4 => todo!(),
            SizeVariant::Tv8 => todo!(),
            SizeVariant::Tv16 => todo!(),
            SizeVariant::Tv32 => todo!(),
            SizeVariant::Tv64 => todo!(),
            SizeVariant::Tv64p => todo!(),
            SizeVariant::Fv1 => todo!(),
            SizeVariant::Fv2 => todo!(),
            SizeVariant::Fv4 => todo!(),
            SizeVariant::Fv8 => todo!(),
            SizeVariant::Fv16 => todo!(),
            SizeVariant::Fv32 => todo!(),
            SizeVariant::Fv32p => todo!(),
        }
    }

    fn move1(ops: &mut Vec<BytecodeInstruction>, dst: HeapOffset, src: HeapOffset) {
        Self::encode_ds(ops, BytecodeOp::Move1, dst, src)
    }
    fn not1(ops: &mut Vec<BytecodeInstruction>, dst: HeapOffset, src: HeapOffset) {
        Self::encode_ds(ops, BytecodeOp::Not1, dst, src)
    }
    fn and1(ops: &mut Vec<BytecodeInstruction>, dst: HeapOffset, lhs: HeapOffset, rhs: HeapOffset) {
        Self::encode_dlr(ops, BytecodeOp::And1, dst, lhs, rhs)
    }
    fn or1(ops: &mut Vec<BytecodeInstruction>, dst: HeapOffset, lhs: HeapOffset, rhs: HeapOffset) {
        Self::encode_dlr(ops, BytecodeOp::Or1, dst, lhs, rhs)
    }
    fn xor1(ops: &mut Vec<BytecodeInstruction>, dst: HeapOffset, lhs: HeapOffset, rhs: HeapOffset) {
        Self::encode_dlr(ops, BytecodeOp::Xor1, dst, lhs, rhs)
    }
    fn xnor1(
        ops: &mut Vec<BytecodeInstruction>,
        dst: HeapOffset,
        lhs: HeapOffset,
        rhs: HeapOffset,
    ) {
        Self::encode_dlr(ops, BytecodeOp::Xnor1, dst, lhs, rhs)
    }
    fn or_not1(
        ops: &mut Vec<BytecodeInstruction>,
        dst: HeapOffset,
        lhs: HeapOffset,
        rhs: HeapOffset,
    ) {
        Self::encode_dlr(ops, BytecodeOp::OrNot1, dst, lhs, rhs)
    }
    fn and_not1(
        ops: &mut Vec<BytecodeInstruction>,
        dst: HeapOffset,
        lhs: HeapOffset,
        rhs: HeapOffset,
    ) {
        Self::encode_dlr(ops, BytecodeOp::AndNot1, dst, lhs, rhs)
    }

    fn next_event(ops: &mut Vec<BytecodeInstruction>) {
        ops.push(Self {
            inner: Self::encode_opcode(BytecodeOp::NextEvent),
        })
    }
}

enum SizeVariant {
    Tv1,
    Tv2,
    Tv4,
    Tv8,
    Tv16,
    Tv32,
    Tv64,
    Tv64p,
    Fv1,
    Fv2,
    Fv4,
    Fv8,
    Fv16,
    Fv32,
    Fv32p,
}

impl SizeVariant {
    pub fn new(size: VectorSize, mode: LogicMode) -> Self {
        use LogicMode as M;
        match (mode, size.get()) {
            (M::TwoValue, 1) => Self::Tv1,
            (M::TwoValue, 2) => Self::Tv2,
            (M::TwoValue, 3..=4) => Self::Tv4,
            (M::TwoValue, 5..=8) => Self::Tv8,
            (M::TwoValue, 9..=16) => Self::Tv16,
            (M::TwoValue, 17..=32) => Self::Tv32,
            (M::TwoValue, 33..=64) => Self::Tv64,
            (M::TwoValue, _) => Self::Tv64p,

            (M::FourValue, 1) => Self::Fv1,
            (M::FourValue, 2) => Self::Fv2,
            (M::FourValue, 3..=4) => Self::Fv4,
            (M::FourValue, 5..=8) => Self::Fv8,
            (M::FourValue, 9..=16) => Self::Fv16,
            (M::FourValue, 17..=32) => Self::Fv32,
            (M::FourValue, _) => Self::Fv32p,
        }
    }
}
