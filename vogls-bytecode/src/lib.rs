#![allow(unused)]

use vogls_codegen::{Heap, HeapOffset};
use vogls_ir::VectorSize;

macro_rules! define_op {
    ($($name:ident = $f:path,)+) => {
        static OP_TABLE: [fn(op: u32, heap: &mut Heap, pc: &mut usize, regs: &mut Regs); 34] = [
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

pub fn not1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src) = extract_2op(op, regs);
    let value = heap.get_tv_bool(src);
    heap.set_tv_bool(dst, !value);
}
pub fn move1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src) = extract_2op(op, regs);
    let value = heap.get_tv_bool(src);
    heap.set_tv_bool(dst, value);
}
pub fn and1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_tv_bool(lhs);
    let rhs = heap.get_tv_bool(rhs);
    heap.set_tv_bool(dst, lhs & rhs);
}
pub fn or1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_tv_bool(lhs);
    let rhs = heap.get_tv_bool(rhs);
    heap.set_tv_bool(dst, lhs | rhs);
}
pub fn xor1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_tv_bool(lhs);
    let rhs = heap.get_tv_bool(rhs);
    heap.set_tv_bool(dst, lhs ^ rhs);
}
pub fn xnor1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_tv_bool(lhs);
    let rhs = heap.get_tv_bool(rhs);
    heap.set_tv_bool(dst, lhs == rhs);
}
pub fn or_not1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_tv_bool(lhs);
    let rhs = heap.get_tv_bool(rhs);
    heap.set_tv_bool(dst, lhs | !rhs);
}
pub fn and_not1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_tv_bool(lhs);
    let rhs = heap.get_tv_bool(rhs);
    heap.set_tv_bool(dst, lhs & !rhs);
}
pub fn zero_extend1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src, size) = extract_size(op, regs);
    let src = heap.get_tv_bool(src);
    todo!()
}
pub fn sign_extend1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src, size) = extract_size(op, regs);
    let src = heap.get_tv_bool(src);
    todo!()
}

pub fn fv_not1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src) = extract_2op(op, regs);
    let value = heap.get_fv_item(src);
    heap.set_fv_scalar(dst, !value);
}
pub fn fv_and1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_fv_item(lhs);
    let rhs = heap.get_fv_item(rhs);
    heap.set_fv_scalar(dst, lhs & rhs);
}
pub fn fv_or1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_fv_item(lhs);
    let rhs = heap.get_fv_item(rhs);
    heap.set_fv_scalar(dst, lhs | rhs);
}
pub fn fv_xor1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_fv_item(lhs);
    let rhs = heap.get_fv_item(rhs);
    heap.set_fv_scalar(dst, lhs ^ rhs);
}
pub fn fv_xnor1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, lhs, rhs) = extract_3op(op, regs);
    let lhs = heap.get_fv_item(lhs);
    let rhs = heap.get_fv_item(rhs);
    heap.set_fv_scalar(dst, !(lhs ^ rhs));
}

pub fn move2(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src) = extract_2op(op, regs);
    let value = heap.get_tv_u64(src.to_ref(SIZE2));
    heap.set_tv_u64(dst.to_ref(SIZE2), value);
}
pub fn lsb2(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src) = extract_2op(op, regs);
    let value = heap.get_tv_u64(src.to_ref(SIZE2));
    heap.set_tv_bool(dst, value & 1 != 0);
}
pub fn neg2(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src) = extract_2op(op, regs);
    let value = heap.get_tv_u64(src.to_ref(SIZE2));
    heap.set_tv_u64(dst.to_ref(SIZE2), !value);
}
macro_rules! reduce2_op {
    ($name:ident, |$value:ident| $out:expr) => {
        pub fn $name(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
            let (dst, src) = extract_2op(op, regs);
            let $value = heap.get_tv_u64(src.to_ref(SIZE2));
            heap.set_tv_bool(dst, $out);
        }
    };
}
reduce2_op!(reduce_or2, |value| value != 0);
reduce2_op!(reduce_and2, |value| value == 0b11);
reduce2_op!(reduce_xor2, |value| value.count_ones() == 1);

macro_rules! bin2_op {
    ($name:ident, |$lhs:ident, $rhs:ident| $out:expr) => {
        pub fn $name(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
            let (dst, lhs, rhs) = extract_3op(op, regs);
            let $lhs = heap.get_tv_u64(lhs.to_ref(SIZE2));
            let $rhs = heap.get_tv_u64(rhs.to_ref(SIZE2));
            heap.set_tv_u64(dst.to_ref(SIZE2), $out);
        }
    };
}
bin2_op!(and2, |l, r| l & r);
bin2_op!(or2, |l, r| l | r);
bin2_op!(xor2, |l, r| l ^ r);
bin2_op!(add2, |l, r| l.wrapping_add(r) & 0b11);
bin2_op!(sub2, |l, r| l.wrapping_sub(r) & 0b11);
bin2_op!(mul2, |l, r| l.wrapping_mul(r) & 0b11);
bin2_op!(pow2, |l, r| l.wrapping_pow(r as u32) & 0b11);
bin2_op!(divz2, |l, r| l.checked_div(r).unwrap_or_default());
bin2_op!(modz2, |l, r| l.checked_rem(r).unwrap_or_default());

pub fn zero_extend2(op: u32, heap: &mut Heap, pc: &mut usize, regs: &mut Regs) {
    let (dst, src, size) = extract_size(op, regs);
    let src = heap.get_tv_bool(src);
    todo!()
}
pub fn sign_extend2(op: u32, heap: &mut Heap, pc: &mut usize, regs: &mut Regs) {
    let (dst, src, size) = extract_size(op, regs);
    let src = heap.get_tv_bool(src);
    todo!()
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

define_op! {
    // Two-Value
    //   Size = 1
    Move1       = move1,
    Not1        = not1,
    And1        = and1,
    Or1         = or1,
    Xor1        = xor1,
    Xnor1       = xnor1,
    OrNot1      = or_not1,
    AndNot1     = and_not1,
    ZeroExtend1 = zero_extend1,
    SignExtend1 = sign_extend1,

    //   Size = 2
    Move2       = move2,
    Lsb2        = lsb2,
    Neg2        = neg2,
    ReduceOr2   = reduce_or2,
    ReduceAnd2  = reduce_and2,
    ReduceXor2  = reduce_xor2,
    ZeroExtend2 = zero_extend2,
    SignExtend2 = sign_extend2,
    And2        = and2,
    Or2         = or2,
    Xor2        = xor2,
    Add2        = add2,
    Sub2        = sub2,
    Pow2        = pow2,
    Mul2        = mul2,

    NotFv1 = fv_not1,
    AndFv1 = fv_and1,
    OrFv1 = fv_or1,
    XorFv1 = fv_xor1,
    XnorFv1 = fv_xnor1,

    PokeSignal = poke_signal,
    Jump = jump,
    Branch = branch,
    NextEvent = next_event,
}

pub fn execute(op: &[u32], pc: &mut usize, heap: &mut Heap) {
    let mut regs = Regs {
        dst: 0,
        lhs: 0,
        rhs: 0,
        size: SIZE1,
        size1offset: 0,
        size2offset: 0,
        size4offset: 0,
        size8offset: 0,
        size16offset: 0,
        size32offset: 0,
        size64offset: 0,
    };
    while let Some(&op) = op.get(*pc) {
        *pc += 1;
        let opcode = op >> 24;
        let f = OP_TABLE[opcode as usize];
        f(op, heap, pc, &mut regs);
    }
}
