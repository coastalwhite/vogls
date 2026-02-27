use vogls_ir::{ResizeOp, SCALAR_VSIZE, UnaryOp, VectorSize};

use crate::{BinaryArithmeticOp, BinaryComparisonOp, Heap, HeapOffset, HeapRef, ShiftOp};

pub(crate) fn exec_tv_unary(stack: &mut Heap, dst: HeapOffset, op: UnaryOp, src: HeapRef) {
    use UnaryOp as O;
    match op {
        O::Neg => {
            let size = src.size;
            let (dst, src) = stack.get_disjoint_u8_dst_src(dst.to_ref(size), src);
            for (d, s) in dst.iter_mut().zip(src) {
                *d = !*s;
            }
            if size.get() % 8 != 0 {
                *dst.last_mut().unwrap() &= (1u8 << size.get() % 8) - 1;
            }
        }
        O::ReduceOr => {
            let result = stack.get(src).as_slice().iter().any(|b| *b != 0);
            stack.set_tv_u64(dst.to_ref(SCALAR_VSIZE), result as u64);
        }
        O::ReduceAnd => {
            let result = stack
                .get(src)
                .as_slice()
                .iter()
                .map(|b| b.count_ones())
                .sum::<u32>();
            stack.set_tv_u64(
                dst.to_ref(SCALAR_VSIZE),
                u64::from(result == src.size.get()),
            );
        }
        O::ReduceXor => {
            let result = stack
                .get(src)
                .as_slice()
                .iter()
                .map(|b| b.count_ones())
                .sum::<u32>();
            stack.set_tv_u64(dst.to_ref(SCALAR_VSIZE), u64::from(result % 2 == 1));
        }
        O::ContainsX => stack.set_tv_bool(dst, false),
    }
}

pub(crate) fn exec_tv_resize(stack: &mut Heap, dst: HeapRef, op: ResizeOp, src: HeapRef) {
    let (d, s) = stack.get_disjoint_u8_dst_src(dst, src);

    use ResizeOp as O;
    match op {
        O::ZeroExtend => {
            assert!(dst.size >= src.size);
            for i in 0..src.size.get().div_ceil(8) as usize {
                d[i] = s[i];
            }
            for i in src.size.get().div_ceil(8) as usize..dst.size.get().div_ceil(8) as usize {
                d[i] = 0;
            }
        }
        O::SignExtend => {
            assert!(dst.size >= src.size);
            let sign_offset = src.size.get() - 1;
            let sign = (s[(sign_offset / 8) as usize] >> (sign_offset % 8)) & 1;
            let mask = u8::from(sign == 0).wrapping_sub(1);
            if src.size.get() % 8 == 0 {
                for i in 0..(src.size.get() / 8) as usize {
                    d[i] = s[i];
                }
                for i in (src.size.get() / 8) as usize..dst.size.get().div_ceil(8) as usize {
                    d[i] = mask;
                }
            } else {
                let sbytes = src.size.get().div_ceil(8) as usize;
                for i in 0..sbytes - 1 {
                    d[i] = s[i];
                }
                d[sbytes - 1] = s[sbytes - 1] | (mask << (src.size.get() % 8));
                for i in sbytes..dst.size.get().div_ceil(8) as usize {
                    d[i] = mask;
                }
            }
        }
        O::Truncate => {
            vogls_bits::slice::tv_slice(d, s, dst.size);
        }
    }
}

pub(crate) fn exec_tv_bin_arith(
    stack: &mut Heap,
    dst: HeapRef,
    op: BinaryArithmeticOp,
    lhs: HeapOffset,
    rhs: HeapOffset,
) {
    use BinaryArithmeticOp as O;

    use vogls_bits::arithmetic as A;

    fn tv_u8_bitwise_and(dst: &mut [u8], lhs: &[u8], rhs: &[u8], _size: VectorSize) {
        A::tv_bin_bitwise_op(dst, lhs, rhs, |l, r| l & r);
    }
    fn tv_u8_bitwise_or(dst: &mut [u8], lhs: &[u8], rhs: &[u8], _size: VectorSize) {
        A::tv_bin_bitwise_op(dst, lhs, rhs, |l, r| l | r);
    }
    fn tv_u8_bitwise_xor(dst: &mut [u8], lhs: &[u8], rhs: &[u8], _size: VectorSize) {
        A::tv_bin_bitwise_op(dst, lhs, rhs, |l, r| l ^ r);
    }
    fn tv_u8_min(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        if vogls_bits::comparison::tv_unsigned_leq(lhs, rhs, size) {
            dst.copy_from_slice(lhs);
        } else {
            dst.copy_from_slice(rhs);
        }
    }
    fn tv_u8_max(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        if vogls_bits::comparison::tv_unsigned_leq(lhs, rhs, size) {
            dst.copy_from_slice(rhs);
        } else {
            dst.copy_from_slice(lhs);
        }
    }
    fn tv_u64_bitwise_and(dst: &mut [u64], lhs: &[u64], rhs: &[u64], _size: VectorSize) {
        A::tv_bin_u64_bitwise_op(dst, lhs, rhs, |l, r| l & r);
    }
    fn tv_u64_bitwise_or(dst: &mut [u64], lhs: &[u64], rhs: &[u64], _size: VectorSize) {
        A::tv_bin_u64_bitwise_op(dst, lhs, rhs, |l, r| l | r);
    }
    fn tv_u64_bitwise_xor(dst: &mut [u64], lhs: &[u64], rhs: &[u64], _size: VectorSize) {
        A::tv_bin_u64_bitwise_op(dst, lhs, rhs, |l, r| l ^ r);
    }
    fn tv_u64_division(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
        // @Performance: Scratchpad this somehow.
        let mut modulus = vec![0u64; dst.len()];
        A::tv_division(dst, &mut modulus, lhs, rhs, size);
    }
    fn tv_u64_modulus(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
        // @Performance: Scratchpad this somehow.
        let mut quotient = vec![0u64; dst.len()];
        A::tv_division(&mut quotient, dst, lhs, rhs, size);
    }
    fn tv_u64_min(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
        if vogls_bits::comparison::tv_gtu64_unsigned_leq(lhs, rhs, size) {
            dst.copy_from_slice(lhs);
        } else {
            dst.copy_from_slice(rhs);
        }
    }
    fn tv_u64_max(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
        if vogls_bits::comparison::tv_gtu64_unsigned_leq(lhs, rhs, size) {
            dst.copy_from_slice(rhs);
        } else {
            dst.copy_from_slice(lhs);
        }
    }

    let size = dst.size;
    if size.get() > 32 && matches!(op, O::And | O::Or | O::Xor | O::Add | O::Sub | O::Multiply) {
        let f = match op {
            O::And => tv_u64_bitwise_and,
            O::Or => tv_u64_bitwise_or,
            O::Xor => tv_u64_bitwise_xor,
            O::Add => A::tv_addition,
            O::Sub => A::tv_subtraction,
            O::Power => A::tv_power,
            O::Multiply => A::tv_multiplication,
            O::Divide => tv_u64_division,
            O::Modulus => tv_u64_modulus,
            O::Min => tv_u64_min,
            O::Max => tv_u64_max,
            O::CopyX | O::CopyZ => unreachable!(),
        };

        let nwords = size.get().div_ceil(64) as usize;
        let (dst, lhs, rhs) =
            stack.get_disjoint_u64_dst_s1_s2((dst.offset, nwords), (lhs, nwords), (rhs, nwords));

        f(dst, lhs, rhs, size);
    } else {
        let f = match op {
            O::And => tv_u8_bitwise_and,
            O::Or => tv_u8_bitwise_or,
            O::Xor => tv_u8_bitwise_xor,
            O::Add => A::tv_ltu64_addition,
            O::Sub => A::tv_ltu64_subtraction,
            O::Power => A::tv_ltu64_power,
            O::Multiply => A::tv_ltu64_multiplication,
            O::Divide => A::tv_ltu64_division,
            O::Modulus => A::tv_ltu64_modulus,
            O::Min => tv_u8_min,
            O::Max => tv_u8_max,
            O::CopyX | O::CopyZ => unreachable!(),
        };

        let (dst, lhs, rhs) =
            stack.get_disjoint_u8_dst_s1_s2(dst, lhs.to_ref(size), rhs.to_ref(size));
        f(dst, lhs, rhs, size);
    }
}

pub(crate) fn exec_tv_bin_cmp(
    stack: &mut Heap,
    dst: HeapOffset,
    op: BinaryComparisonOp,
    lhs: HeapRef,
    rhs: HeapOffset,
) {
    use BinaryComparisonOp as O;
    let f = match op {
        O::UnsignedLessEqual => vogls_bits::comparison::tv_unsigned_leq,
        O::CaseEquality => {
            fn tv_case_equality(lhs: &[u8], rhs: &[u8], _size: VectorSize) -> bool {
                lhs == rhs
            }
            tv_case_equality
        }
    };

    let size = lhs.size;
    let lhs = stack.get(lhs);
    let rhs = stack.get(rhs.to_ref(size));
    let result = f(lhs.as_slice(), rhs.as_slice(), size);
    stack.set_tv_u64(dst.to_ref(SCALAR_VSIZE), result as u64);
}

pub(crate) fn exec_tv_shift(
    stack: &mut Heap,
    dst: HeapRef,
    op: ShiftOp,
    src: HeapOffset,
    offset: HeapOffset,
) {
    use ShiftOp as O;
    let f = match op {
        O::LogicalLeft => vogls_bits::shift::tv_logical_shift_left,
        O::LogicalRight => vogls_bits::shift::tv_logical_shift_right,
        O::ArithmeticRight => vogls_bits::shift::tv_arithmetic_shift_right,
    };

    let size = dst.size;
    let offset = stack.load_exact_tv_u32(offset);
    let (dst, src) = stack.get_disjoint_u8_dst_src(dst, src.to_ref(size));
    f(dst, src, offset, size);
}

pub(crate) fn exec_tv_select_bit(stack: &mut Heap, dst: HeapOffset, src: HeapRef, idx: HeapOffset) {
    let size = src.size;
    let idx = stack.load_exact_tv_u32(idx);
    let src = stack.get(src);
    let result = vogls_bits::select::tv_select_bit(src.as_slice(), idx, size);
    stack.set_tv_bool(dst, result);
}

pub(crate) fn exec_tv_concat(stack: &mut Heap, dst: HeapOffset, lhs: HeapRef, rhs: HeapRef) {
    let (lhs_size, rhs_size) = (lhs.size, rhs.size);
    let dst_size = lhs_size.checked_add(rhs_size.get()).unwrap();
    let (dst, lhs, rhs) = stack.get_disjoint_u8_dst_s1_s2(dst.to_ref(dst_size), lhs, rhs);
    vogls_bits::concat::tv_concat(dst, lhs, rhs, lhs_size, rhs_size);
}
