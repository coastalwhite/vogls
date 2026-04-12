use super::*;

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
macro_rules! bin1_op {
    ($name:ident, |$lhs:ident, $rhs:ident| $out:expr) => {
        pub fn $name(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
            let (dst, lhs, rhs) = extract_3op(op, regs);
            let $lhs = heap.get_tv_bool(lhs);
            let $rhs = heap.get_tv_bool(rhs);
            heap.set_tv_bool(dst, $out);
        }
    };
}
bin1_op!(and1, |l, r| l & r);
bin1_op!(or1, |l, r| l | r);
bin1_op!(xor1, |l, r| l ^ r);
bin1_op!(xnor1, |l, r| l == r);
bin1_op!(or_not1, |l, r| l | !r);
bin1_op!(and_not1, |l, r| l & !r);

pub fn zero_extend1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src, size) = extract_size(op, regs);
    let src = heap.get_tv_bool(src);
    let word_size = size.min(SIZE64).checked_next_power_of_two().unwrap();
    heap.set_tv_u64(dst.to_ref(word_size), src.into());
    if size > SIZE64 {
        let dst = HeapOffset {
            bit_offset: dst.bit_offset + 64,
        };
        heap.get_mut_u64_slice(dst, (size.get().div_ceil(64) - 1) as usize)
            .fill(0u64);
    }
}
pub fn sign_extend1(op: u32, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src, size) = extract_size(op, regs);
    let src = heap.get_tv_bool(src);
    let word_size = size.min(SIZE64).checked_next_power_of_two().unwrap();
    heap.set_tv_u64(dst.to_ref(word_size), u64::from(src) * u64::MAX);
    if size > SIZE64 {
        let dst = HeapOffset {
            bit_offset: dst.bit_offset + 64,
        };
        let slice = heap.get_mut_u64_slice(dst, (size.get().div_ceil(64) - 1) as usize);
        slice.fill(u64::from(src) * u64::MAX);
        if size.get() % 64 != 0 {
            *slice.last_mut().unwrap() &= (1 << (size.get() % 64)) - 1;
        }
    }
}
