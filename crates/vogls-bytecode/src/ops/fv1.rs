use super::*;

pub fn fv_not1(op: BytecodeInstruction, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
    let (dst, src) = extract_2op(op, regs);
    let value = heap.get_fv_item(src);
    heap.set_fv_scalar(dst, !value);
}
macro_rules! bin1_op {
    ($name:ident, |$lhs:ident, $rhs:ident| $out:expr) => {
        pub fn $name(op: BytecodeInstruction, heap: &mut Heap, _pc: &mut usize, regs: &mut Regs) {
            let (dst, lhs, rhs) = extract_3op(op, regs);
            let $lhs = heap.get_fv_item(lhs);
            let $rhs = heap.get_fv_item(rhs);
            heap.set_fv_scalar(dst, $out);
        }
    };
}
bin1_op!(fv_and1, |l, r| l & r);
bin1_op!(fv_or1, |l, r| l | r);
bin1_op!(fv_xor1, |l, r| l ^ r);
