use super::*;

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
bin2_op!(remz2, |l, r| l.checked_rem(r).unwrap_or_default());
bin2_op!(min2, |l, r| l.min(r));
bin2_op!(max2, |l, r| l.max(r));

macro_rules! bin_bool2_op {
    ($name:ident, |$lhs:ident, $rhs:ident| $out:expr) => {
        pub fn $name(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
            let (dst, lhs, rhs) = extract_3op(op, regs);
            let $lhs = heap.get_tv_u64(lhs.to_ref(SIZE2));
            let $rhs = heap.get_tv_u64(rhs.to_ref(SIZE2));
            heap.set_tv_bool(dst, $out);
        }
    };
}
bin_bool2_op!(ule2, |l, r| l <= r);
bin_bool2_op!(ceq2, |l, r| l == r);

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
