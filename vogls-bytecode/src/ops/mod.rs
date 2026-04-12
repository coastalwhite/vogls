use vogls_codegen::{Heap, HeapOffset};
use vogls_ir::VectorSize;

mod fv1;
mod tv1;
mod tv2;

#[inline(always)]
fn extract_2op(op: u32, regs: &Regs) -> (HeapOffset, HeapOffset) {
    // OP:  [31:24]
    // DST: [23:12]
    // SRC: [11: 0]
    let mut dst = (op >> 12) & 0x3FF;
    let mut src = op & 0x3FF;

    if dst == 0x3FF {
        dst = regs.dst;
    }
    if src == 0x3FF {
        src = regs.lhs;
    }
    (
        HeapOffset {
            bit_offset: dst as usize,
        },
        HeapOffset {
            bit_offset: src as usize,
        },
    )
}

#[inline(always)]
fn extract_3op(op: u32, regs: &Regs) -> (HeapOffset, HeapOffset, HeapOffset) {
    // OP:  [31:24]
    // DST: [23:16]
    // LHS: [15: 8]
    // RHS: [ 7: 0]
    let mut dst = (op >> 16) & 0xFF;
    let mut lhs = (op >> 8) & 0xFF;
    let mut rhs = op & 0xFF;

    if dst == 0xFF {
        dst = regs.dst;
    }
    if lhs == 0xFF {
        lhs = regs.lhs;
    }
    if rhs == 0xFF {
        rhs = regs.rhs;
    }

    (
        HeapOffset {
            bit_offset: dst as usize,
        },
        HeapOffset {
            bit_offset: lhs as usize,
        },
        HeapOffset {
            bit_offset: rhs as usize,
        },
    )
}
#[inline(always)]
fn extract_size(op: u32, regs: &Regs) -> (HeapOffset, HeapOffset, VectorSize) {
    // OP:   [31:24]
    // DST:  [23:16]
    // SRC:  [15: 8]
    // SIZE: [ 7: 0]
    let mut dst = (op >> 16) & 0xFF;
    let mut src = (op >> 8) & 0xFF;
    let size = op & 0xFF;

    if dst == 0xFF {
        dst = regs.dst;
    }
    if src == 0xFF {
        src = regs.lhs;
    }
    let size = VectorSize::new(size).unwrap_or(regs.size);
    (
        HeapOffset {
            bit_offset: dst as usize,
        },
        HeapOffset {
            bit_offset: src as usize,
        },
        size,
    )
}
#[inline(always)]
fn extract_branch(op: u32, regs: &Regs) -> (HeapOffset, i8, i8) {
    // OP:  [31:24]
    // DST: [23:16]
    // LHS: [15: 8]
    // RHS: [ 7: 0]
    let mut dst = (op >> 16) & 0xFF;
    let mut lhs = (op >> 8) & 0xFF;
    let mut rhs = op & 0xFF;

    if dst == 0x3FF {
        dst = regs.dst;
    }
    if lhs == 0x3FF {
        lhs = regs.lhs;
    }
    if rhs == 0x3FF {
        rhs = regs.rhs;
    }

    (
        HeapOffset {
            bit_offset: dst as usize,
        },
        lhs as u8 as i8,
        rhs as u8 as i8,
    )
}

pub struct Regs {
    dst: u32,
    lhs: u32,
    rhs: u32,
    size: VectorSize,
    size1offset: usize,
    size2offset: usize,
    size4offset: usize,
    size8offset: usize,
    size16offset: usize,
    size32offset: usize,
    size64offset: usize,
}

const SIZE1: VectorSize = VectorSize::new(1).unwrap();
const SIZE2: VectorSize = VectorSize::new(2).unwrap();
const SIZE4: VectorSize = VectorSize::new(4).unwrap();
const SIZE8: VectorSize = VectorSize::new(8).unwrap();
const SIZE16: VectorSize = VectorSize::new(16).unwrap();
const SIZE32: VectorSize = VectorSize::new(32).unwrap();
const SIZE64: VectorSize = VectorSize::new(64).unwrap();

macro_rules! define_op {
    ($($name:ident = $f:path,)+) => {
        pub static OP_TABLE: [fn(op: u32, heap: &mut Heap, pc: &mut usize, regs: &mut Regs); 39] = [
            $($f,)+
        ];

        #[derive(Clone, Copy)]
        pub enum BytecodeOp {
            $(
            $name,
            )+
        }
    };
}

define_op! {
    // Two-Value
    //   Size = 1
    Move1       = tv1::move1,
    Not1        = tv1::not1,
    And1        = tv1::and1,
    Or1         = tv1::or1,
    Xor1        = tv1::xor1,
    Xnor1       = tv1::xnor1,
    OrNot1      = tv1::or_not1,
    AndNot1     = tv1::and_not1,
    ZeroExtend1 = tv1::zero_extend1,
    SignExtend1 = tv1::sign_extend1,

    //   Size = 2
    Move2                 = tv2::move2,
    Lsb2                  = tv2::lsb2,
    Neg2                  = tv2::neg2,
    ReduceOr2             = tv2::reduce_or2,
    ReduceAnd2            = tv2::reduce_and2,
    ReduceXor2            = tv2::reduce_xor2,
    ZeroExtend2           = tv2::zero_extend2,
    SignExtend2           = tv2::sign_extend2,
    And2                  = tv2::and2,
    Or2                   = tv2::or2,
    Xor2                  = tv2::xor2,
    Add2                  = tv2::add2,
    Sub2                  = tv2::sub2,
    Pow2                  = tv2::pow2,
    Mul2                  = tv2::mul2,
    // DivideX2              = tv2::divx2,
    DivideZ2              = tv2::divz2,
    // ModulusX2             = tv2::remx2,
    ModulusZ2             = tv2::remz2,
    Min2                  = tv2::min2,
    Max2                  = tv2::max2,
    UnsignedLessEqual     = tv2::ule2,
    CaseEquality          = tv2::ceq2,
    // LogicalShiftLeftX     = tv2::lslx2,
    // LogicalShiftRightX    = tv2::lsrx2,
    // ArithmeticShiftRightX = tv2::asrx2,
    // LogicalShiftLeftZ     = tv2::lslz2,
    // LogicalShiftRightZ    = tv2::lsrz2,
    // ArithmeticShiftRightZ = tv2::asrz2,
    // SliceX                = tv2::slicex2,
    // SliceZ                = tv2::slicez2,

    NotFv1      = fv1::fv_not1,
    AndFv1      = fv1::fv_and1,
    OrFv1       = fv1::fv_or1,
    XorFv1      = fv1::fv_xor1,

    PokeSignal  = poke_signal,
    Jump        = jump,
    Branch      = branch,
    NextEvent   = next_event,
}

pub fn poke_signal(op: u32, heap: &mut Heap, pc: &mut usize, regs: &mut Regs) {
    todo!()
}

pub fn branch(op: u32, heap: &mut Heap, pc: &mut usize, regs: &mut Regs) {
    let (condition, truthy, falsy) = extract_branch(op, regs);
    let condition = heap.get_tv_bool(condition);
    let offset = if condition { truthy } else { falsy };
    *pc += pc.wrapping_add_signed(offset.into());
}
pub fn jump(op: u32, _heap: &mut Heap, pc: &mut usize, _regs: &mut Regs) {
    // @TODO: Allow for regs.
    let offset = ((op << 8) as i32) >> 8;
    *pc += pc.wrapping_add_signed(offset as isize);
}
pub fn next_event(op: u32, _heap: &mut Heap, pc: &mut usize, _regs: &mut Regs) {
    // @TODO: Allow for regs.
    let offset = ((op << 8) as i32) >> 8;
    *pc += pc.wrapping_add_signed(offset as isize);
    todo!()
}
