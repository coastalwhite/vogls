use vogls_bits::get_disjoint_dst_src;
use vogls_ir::{ResizeOp, UnaryOp, VectorSize};

use crate::{BinaryArithmeticOp, BinaryComparisonOp, ShiftOp};

pub(crate) fn exec_tv_unary(
    stack: &mut [u8],
    dst: usize,
    op: UnaryOp,
    size: VectorSize,
    src: usize,
) {
    use UnaryOp as O;
    match op {
        O::Neg => {
            let n_full_bytes = (size.get() / 8) as usize;
            if size.get() % 8 == 0 {
                for i in 0..n_full_bytes {
                    stack[dst + i] = !stack[src + i];
                }
            } else {
                stack[dst + n_full_bytes] =
                    stack[src + n_full_bytes] ^ 1u8.unbounded_shl(size.get() % 8).wrapping_sub(1);
                for i in 0..n_full_bytes {
                    stack[dst + i] = !stack[src + i];
                }
            }
        }
        O::ReduceOr => {
            let result = stack[src..][..size.get().div_ceil(8) as usize]
                .iter()
                .any(|b| *b != 0);
            stack[dst] = u8::from(result);
        }
        O::ReduceAnd => {
            let result = stack[src..][..size.get().div_ceil(8) as usize]
                .iter()
                .map(|b| b.count_ones())
                .sum::<u32>();
            stack[dst] = u8::from(result == size.get());
        }
        O::ReduceXor => {
            let result = stack[src..][..size.get().div_ceil(8) as usize]
                .iter()
                .map(|b| b.count_ones())
                .sum::<u32>();
            stack[dst] = u8::from(result % 2 == 1);
        }
    }
}

pub(crate) fn exec_tv_resize(
    stack: &mut [u8],
    dst: usize,
    dst_size: VectorSize,
    op: ResizeOp,
    src: usize,
    src_size: VectorSize,
) {
    use ResizeOp as O;
    match op {
        O::ZeroExtend => {
            assert!(dst_size >= src_size);
            for i in 0..src_size.get().div_ceil(8) as usize {
                stack[dst + i] = stack[src + i];
            }
            for i in src_size.get().div_ceil(8) as usize..dst_size.get().div_ceil(8) as usize {
                stack[dst + i] = 0;
            }
        }
        O::SignExtend => {
            assert!(dst_size >= src_size);
            let sign_offset = src_size.get() - 1;
            let sign = (stack[src + (sign_offset / 8) as usize] >> (sign_offset % 8)) & 1;
            let mask = u8::from(sign == 0).wrapping_sub(1);
            if src_size.get() % 8 == 0 {
                for i in 0..(src_size.get() / 8) as usize {
                    stack[dst + i] = stack[src + i];
                }
                for i in (src_size.get() / 8) as usize..dst_size.get().div_ceil(8) as usize {
                    stack[dst + i] = mask;
                }
            } else {
                let sbytes = src_size.get().div_ceil(8) as usize;
                for i in 0..sbytes - 1 {
                    stack[dst + i] = stack[src + i];
                }
                stack[dst + sbytes - 1] = stack[src + sbytes - 1] | (mask << (src_size.get() % 8));
                for i in sbytes..dst_size.get().div_ceil(8) as usize {
                    stack[dst + i] = mask;
                }
            }
        }
        O::Truncate => {
            let (dst, src) = get_disjoint_dst_src(
                stack,
                dst,
                dst_size.get().div_ceil(8) as usize,
                src,
                src_size.get().div_ceil(8) as usize,
            );
            vogls_bits::slice::tv_slice(dst, src, dst_size);
        }
    }
}

pub(crate) fn exec_tv_bin_arith(
    stack: &mut [u8],
    dst: usize,
    op: BinaryArithmeticOp,
    size: VectorSize,
    lhs: usize,
    rhs: usize,
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

    if size.get() > 32 && matches!(op, O::And | O::Or | O::Xor | O::Add | O::Sub | O::Multiply) {
        let f = match op {
            O::And => tv_u64_bitwise_and,
            O::Or => tv_u64_bitwise_or,
            O::Xor => tv_u64_bitwise_xor,
            O::Add => A::tv_addition,
            O::Sub => A::tv_subtraction,
            O::Multiply => A::tv_multiplication,
            O::Divide => tv_u64_division,
            O::Modulus => tv_u64_modulus,
        };

        let nwords = size.get().div_ceil(64) as usize;
        let nbytes = nwords * 8;
        let (dst, lhs, rhs) =
            vogls_bits::get_disjoint_dst_s1_s2(stack, dst, nbytes, lhs, nbytes, rhs, nbytes);

        let dst = bytemuck::cast_slice_mut(dst);
        let lhs = bytemuck::cast_slice(lhs);
        let rhs = bytemuck::cast_slice(rhs);

        f(dst, lhs, rhs, size);
    } else {
        let f = match op {
            O::And => tv_u8_bitwise_and,
            O::Or => tv_u8_bitwise_or,
            O::Xor => tv_u8_bitwise_xor,
            O::Add => A::tv_ltu64_addition,
            O::Sub => A::tv_ltu64_subtraction,
            O::Multiply => A::tv_ltu64_multiplication,
            O::Divide => A::tv_ltu64_division,
            O::Modulus => A::tv_ltu64_modulus,
        };

        let nbytes = size.get().div_ceil(8) as usize;
        let (dst, lhs, rhs) =
            vogls_bits::get_disjoint_dst_s1_s2(stack, dst, nbytes, lhs, nbytes, rhs, nbytes);

        f(dst, lhs, rhs, size);
    }
}

pub(crate) fn exec_tv_bin_cmp(
    stack: &mut [u8],
    dst: usize,
    op: BinaryComparisonOp,
    size: VectorSize,
    lhs: usize,
    rhs: usize,
) {
    use BinaryComparisonOp as O;
    let f = match op {
        O::UnsignedLessEqual => vogls_bits::comparison::tv_unsigned_leq,
    };
    let nbytes = size.get().div_ceil(8) as usize;
    let lhs = &stack[lhs..][..nbytes];
    let rhs = &stack[rhs..][..nbytes];
    let result = f(lhs, rhs, size);
    stack[dst] = u8::from(result);
}

pub(crate) fn exec_tv_shift(
    stack: &mut [u8],
    dst: usize,
    op: ShiftOp,
    size: VectorSize,
    src: usize,
    offset: usize,
) {
    use ShiftOp as O;
    let f = match op {
        O::LogicalLeft => vogls_bits::shift::tv_logical_shift_left,
        O::LogicalRight => vogls_bits::shift::tv_logical_shift_right,
        O::ArithmeticRight => vogls_bits::shift::tv_arithmetic_shift_right,
    };

    let offset = vogls_bits::load::load_full_u32(&stack[offset..]);
    let nbytes = size.get().div_ceil(8) as usize;

    let (dst, src) = vogls_bits::get_disjoint_dst_src(stack, dst, nbytes, src, nbytes);
    f(dst, src, offset, size);
}

pub(crate) fn exec_tv_select_bit(
    stack: &mut [u8],
    dst: usize,
    size: VectorSize,
    src: usize,
    idx: usize,
) {
    let idx = vogls_bits::load::load_full_u32(&stack[idx..]);
    let nbytes = size.get().div_ceil(8) as usize;

    let src = &stack[src..][..nbytes];
    stack[dst] = u8::from(vogls_bits::select::tv_select_bit(src, idx, size));
}

pub(crate) fn exec_tv_concat(
    stack: &mut [u8],
    dst: usize,
    lhs: usize,
    lhs_size: VectorSize,
    rhs: usize,
    rhs_size: VectorSize,
) {
    let lbytes = lhs_size.get().div_ceil(8) as usize;
    let rbytes = rhs_size.get().div_ceil(8) as usize;
    let dbytes = (lhs_size.get().checked_add(rhs_size.get()).unwrap()).div_ceil(8) as usize;

    let (dst, lhs, rhs) =
        vogls_bits::get_disjoint_dst_s1_s2(stack, dst, dbytes, lhs, lbytes, rhs, rbytes);

    vogls_bits::concat::tv_concat(dst, lhs, rhs, lhs_size, rhs_size);
}
