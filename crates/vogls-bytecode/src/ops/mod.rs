use crate::BytecodeInstruction;
use vogls_codegen::{Heap, HeapOffset};
use vogls_ir::VectorSize;

// mod fv1;
mod tv1;
// mod tv2;

pub struct Regs {
    pub size1offset: usize,
    pub size2offset: usize,
    pub size4offset: usize,
    pub size8offset: usize,
    pub size16offset: usize,
    pub size32offset: usize,
    pub size64offset: usize,
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
        pub static OP_TABLE: [fn(op: BytecodeInstruction, heap: &mut Heap, ops: &[BytecodeInstruction], pc: &mut usize, regs: &mut Regs); { 0 $(+ { _ = BytecodeOp::$name; 1 })+ }] = [
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
    // ZeroExtend1 = tv1::zero_extend1,
    // SignExtend1 = tv1::sign_extend1,

    //   Size = 2
    // Move2                 = tv2::move2,
    // Lsb2                  = tv2::lsb2,
    // Neg2                  = tv2::neg2,
    // ReduceOr2             = tv2::reduce_or2,
    // ReduceAnd2            = tv2::reduce_and2,
    // ReduceXor2            = tv2::reduce_xor2,
    // ZeroExtend2           = tv2::zero_extend2,
    // SignExtend2           = tv2::sign_extend2,
    // And2                  = tv2::and2,
    // Or2                   = tv2::or2,
    // Xor2                  = tv2::xor2,
    // Add2                  = tv2::add2,
    // Sub2                  = tv2::sub2,
    // Pow2                  = tv2::pow2,
    // Mul2                  = tv2::mul2,
    // DivideX2              = tv2::divx2,
    // DivideZ2              = tv2::divz2,
    // ModulusX2             = tv2::remx2,
    // ModulusZ2             = tv2::remz2,
    // Min2                  = tv2::min2,
    // Max2                  = tv2::max2,
    // UnsignedLessEqual     = tv2::ule2,
    // CaseEquality          = tv2::ceq2,
    // LogicalShiftLeftX     = tv2::lslx2,
    // LogicalShiftRightX    = tv2::lsrx2,
    // ArithmeticShiftRightX = tv2::asrx2,
    // LogicalShiftLeftZ     = tv2::lslz2,
    // LogicalShiftRightZ    = tv2::lsrz2,
    // ArithmeticShiftRightZ = tv2::asrz2,
    // SliceX                = tv2::slicex2,
    // SliceZ                = tv2::slicez2,

    // NotFv1      = fv1::fv_not1,
    // AndFv1      = fv1::fv_and1,
    // OrFv1       = fv1::fv_or1,
    // XorFv1      = fv1::fv_xor1,

    // PokeSignal  = poke_signal,
    // Jump        = jump,
    // Branch      = branch,
    NextEvent   = next_event,
}

// pub fn poke_signal(op: BytecodeInstruction, heap: &mut Heap, pc: &mut usize, regs: &mut Regs) {
//     todo!()
// }
//
// pub fn branch(op: BytecodeInstruction, heap: &mut Heap, pc: &mut usize, regs: &mut Regs) {
//     let (condition, truthy, falsy) = extract_branch(op, regs);
//     let condition = heap.get_tv_bool(condition);
//     let offset = if condition { truthy } else { falsy };
//     *pc += pc.wrapping_add_signed(offset.into());
// }
// pub fn jump(op: BytecodeInstruction, _heap: &mut Heap, pc: &mut usize, _regs: &mut Regs) {
//     // @TODO: Allow for regs.
//     let offset = ((op << 8) as i32) >> 8;
//     *pc += pc.wrapping_add_signed(offset as isize);
// }
pub fn next_event(op: BytecodeInstruction, _heap: &mut Heap, ops: &[BytecodeInstruction], pc: &mut usize, _regs: &mut Regs) {
    panic!("next event");
}
