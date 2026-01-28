use vogls_bits::arithmetic::{
    FvLogicValue, fv_gtu32_bitwise_inv, fv_l_reduce_and, fv_l_reduce_or, fv_l_reduce_xor,
    fv_l_select_bit, fv_leu32_bitwise_inv, fv_pack_u64, fv_s_reduce_and, fv_s_reduce_or,
    fv_s_reduce_xor, fv_s_select_bit, fv_unpack_u64,
};
use vogls_bits::concat::{fv_l_concat, fv_s_concat};
use vogls_bits::extend::{fv_l_sign_extend, fv_l_zero_extend, fv_s_sign_extend, fv_s_zero_extend};
use vogls_bits::load::load_partial_u64;
use vogls_bits::shift::{
    fv_l_arithmetic_shift_right, fv_l_logical_shift_left, fv_l_logical_shift_right,
    fv_s_arithmetic_shift_right, fv_s_logical_shift_left, fv_s_logical_shift_right,
};
use vogls_bits::truncate::{fv_l_truncate, fv_s_truncate};
use vogls_bits::{get_disjoint_dst_s1_s2, get_disjoint_dst_src};
use vogls_ir::{ResizeOp, UnaryOp, VectorSize};

use crate::{BinaryArithmeticOp, BinaryComparisonOp, ShiftOp, Stack, StackOffset, StackRef};

pub(crate) fn exec_fv_unary(stack: &mut Stack, dst: StackOffset, op: UnaryOp, src: StackRef) {
    use UnaryOp as O;
    match op {
        O::Neg if src.size.get() > 16 => {
            let nwords = 2 * src.size.get().div_ceil(64) as usize;
            let (dst_s, src_s) =
                stack.get_disjoint_u64_dst_src((dst, nwords), (src.offset, nwords));
            fv_gtu32_bitwise_inv(dst_s, src_s, src.size)
        }
        O::Neg => {
            let (dst_s, src_s) =
                stack.get_disjoint_u8_dst_src(dst.to_ref(src.size).to_fv_size(), src.to_fv_size());
            fv_leu32_bitwise_inv(dst_s, src_s, src.size)
        }

        O::ReduceOr | O::ReduceAnd | O::ReduceXor if src.size.get() > 16 => {
            let nwords = 2 * src.size.get().div_ceil(64) as usize;
            let src_s = stack.get_u64_slice(src.offset, nwords);
            let f = match op {
                O::Neg => unreachable!(),
                O::ReduceOr => fv_l_reduce_or,
                O::ReduceAnd => fv_l_reduce_and,
                O::ReduceXor => fv_l_reduce_xor,
            };
            let result = f(src_s, src.size);
            stack.set_fv_scalar(dst, result);
        }
        O::ReduceOr | O::ReduceAnd | O::ReduceXor => {
            let src_s = stack.get(src);
            let f = match op {
                O::Neg => unreachable!(),
                O::ReduceOr => fv_s_reduce_or,
                O::ReduceAnd => fv_s_reduce_and,
                O::ReduceXor => fv_s_reduce_xor,
            };
            let result = f(src_s, src.size);
            stack.set_fv_scalar(dst, result);
        }
    };
}

pub(crate) fn exec_fv_resize(stack: &mut Stack, dst: StackRef, op: ResizeOp, src: StackRef) {
    use ResizeOp as O;
    match op {
        O::Truncate | O::ZeroExtend | O::SignExtend
            if dst.size.get() <= 16 && src.size.get() <= 16 =>
        {
            let (dst_s, src_s) = stack.get_disjoint_u8_dst_src(dst.to_fv_size(), src.to_fv_size());
            let f = match op {
                O::Truncate => fv_s_truncate,
                O::ZeroExtend => fv_s_zero_extend,
                O::SignExtend => fv_s_sign_extend,
            };
            f(dst_s, src_s, dst.size, src.size);
        }
        O::Truncate | O::ZeroExtend | O::SignExtend
            if dst.size.get() > 16 && src.size.get() > 16 =>
        {
            let (dst_s, src_s) = stack.get_disjoint_u64_dst_src(
                (dst.offset, 2 * dst.size.get().div_ceil(64) as usize),
                (src.offset, 2 * src.size.get().div_ceil(64) as usize),
            );
            let f = match op {
                O::Truncate => fv_l_truncate,
                O::ZeroExtend => fv_l_zero_extend,
                O::SignExtend => fv_l_sign_extend,
            };
            f(dst_s, src_s, dst.size, src.size);
        }
        O::Truncate => {
            let mut pdst = [0u64; 2];
            let src_s = stack.get_u64_slice(src.offset, 2 * src.size.get().div_ceil(64) as usize);
            fv_l_truncate(&mut pdst, src_s, dst.size, src.size);
            stack.set_fv_u64(dst, pdst[0], pdst[1]);
        }
        O::ZeroExtend | O::SignExtend => {
            let mut psrc = [0u64; 2];
            (psrc[0], psrc[1]) = stack.get_fv_u64(src);
            let dst_s =
                stack.get_mut_u64_slice(dst.offset, 2 * dst.size.get().div_ceil(64) as usize);
            let f = match op {
                O::ZeroExtend => fv_l_zero_extend,
                O::SignExtend => fv_l_sign_extend,
                O::Truncate => unreachable!(),
            };
            f(dst_s, &psrc, dst.size, src.size);
        }
    }
}

pub(crate) fn exec_fv_bin_arith(
    stack: &mut Stack,
    dst: StackRef,
    op: BinaryArithmeticOp,
    lhs: StackOffset,
    rhs: StackOffset,
) {
    use BinaryArithmeticOp as O;

    use vogls_bits::arithmetic as A;

    fn fv_u8_bitwise_and(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        A::fv_bin_bitwise_op(dst, lhs, rhs, size, A::fv_bitwise_and_elem)
    }
    fn fv_u8_bitwise_or(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        A::fv_bin_bitwise_op(dst, lhs, rhs, size, A::fv_bitwise_or_elem)
    }
    fn fv_u8_bitwise_xor(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        A::fv_bin_bitwise_op(dst, lhs, rhs, size, A::fv_bitwise_xor_elem)
    }
    fn fv_u64_bitwise_and(dst: &mut [u64], lhs: &[u64], rhs: &[u64], _size: VectorSize) {
        A::fv_bin_u64_bitwise_op(dst, lhs, rhs, A::fv_bitwise_and_elem);
    }
    fn fv_u64_bitwise_or(dst: &mut [u64], lhs: &[u64], rhs: &[u64], _size: VectorSize) {
        A::fv_bin_u64_bitwise_op(dst, lhs, rhs, A::fv_bitwise_or_elem);
    }
    fn fv_u64_bitwise_xor(dst: &mut [u64], lhs: &[u64], rhs: &[u64], _size: VectorSize) {
        A::fv_bin_u64_bitwise_op(dst, lhs, rhs, A::fv_bitwise_xor_elem);
    }
    fn fv_u64_division(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
        // @Performance: Scratchpad this somehow.
        let mut modulus = vec![0u64; dst.len()];
        A::fv_division(dst, &mut modulus, lhs, rhs, size);
    }
    fn fv_u64_modulus(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
        // @Performance: Scratchpad this somehow.
        let mut quotient = vec![0u64; dst.len()];
        A::fv_division(&mut quotient, dst, lhs, rhs, size);
    }

    if dst.size.get() > 16 {
        let f = match op {
            O::And => fv_u64_bitwise_and,
            O::Or => fv_u64_bitwise_or,
            O::Xor => fv_u64_bitwise_xor,
            O::Add => A::fv_addition,
            O::Sub => A::fv_subtraction,
            O::Multiply => A::fv_multiplication,
            O::Divide => fv_u64_division,
            O::Modulus => fv_u64_modulus,
        };

        let nwords = 2 * dst.size.get().div_ceil(64) as usize;
        let (dst_s, lhs_s, rhs_s) =
            stack.get_disjoint_u64_dst_s1_s2((dst.offset, nwords), (lhs, nwords), (rhs, nwords));

        f(dst_s, lhs_s, rhs_s, dst.size);
    } else {
        let f = match op {
            O::And => fv_u8_bitwise_and,
            O::Or => fv_u8_bitwise_or,
            O::Xor => fv_u8_bitwise_xor,
            O::Add => A::fv_ltu32_addition,
            O::Sub => A::fv_ltu32_subtraction,
            O::Multiply => A::fv_ltu32_multiplication,
            O::Divide => A::fv_ltu32_division,
            O::Modulus => A::fv_ltu32_modulus,
        };

        let (dst_s, lhs_s, rhs_s) = stack.get_disjoint_u8_dst_s1_s2(
            dst.to_fv_size(),
            lhs.to_ref(dst.size).to_fv_size(),
            rhs.to_ref(dst.size).to_fv_size(),
        );

        f(dst_s, lhs_s, rhs_s, dst.size);
    }
}

pub(crate) fn exec_fv_bin_cmp(
    stack: &mut Stack,
    dst: StackOffset,
    op: BinaryComparisonOp,
    lhs: StackRef,
    rhs: StackOffset,
) {
    use BinaryComparisonOp as O;
    let result = match op {
        O::UnsignedLessEqual if lhs.size.get() <= 16 => {
            let lhs_s = stack.get(lhs.to_fv_size());
            let rhs_s = stack.get(rhs.to_ref(lhs.size).to_fv_size());
            vogls_bits::comparison::fv_s_unsigned_leq(lhs_s, rhs_s, lhs.size)
        }
        O::UnsignedLessEqual => {
            let nwords = 2 * lhs.size.get().div_ceil(64) as usize;
            let lhs_s = stack.get_u64_slice(lhs.offset, nwords);
            let rhs_s = stack.get_u64_slice(rhs, nwords);
            vogls_bits::comparison::fv_l_unsigned_leq(lhs_s, rhs_s, lhs.size)
        }
    };
    stack.set_fv_scalar(dst, result);
}

pub(crate) fn exec_fv_shift(
    stack: &mut [u8],
    dst: usize,
    op: ShiftOp,
    size: VectorSize,
    src: usize,
    offset: usize,
) {
    use ShiftOp as O;
    let offset = load_partial_u64(&stack[offset..][..8], VectorSize::new(32).unwrap());

    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 53
    // """
    // If the right operand has an x or z value, then the result shall be unknown.
    // """
    if offset >> 32 != 0xFFFF_FFFF {
        if size.get() > 16 {
            let nbytes = (2 * size.get()).div_ceil(8) as usize;
            stack[dst..][..nbytes].fill(0u8);
        } else {
            let nwords = 2 * size.get().div_ceil(64) as usize;
            bytemuck::cast_slice_mut::<u8, u64>(&mut stack[dst..][..nwords * 2]).fill(0u64);
        }
        return;
    }

    let offset = (offset & 0xFFFF_FFFF) as u32;
    if size.get() > 16 {
        let nbytes = size.get().div_ceil(8) as usize;
        let (dst, src) = get_disjoint_dst_src(stack, dst, nbytes, src, nbytes);
        let f = match op {
            O::LogicalLeft => fv_s_logical_shift_left,
            O::LogicalRight => fv_s_logical_shift_right,
            O::ArithmeticRight => fv_s_arithmetic_shift_right,
        };
        f(dst, src, offset, size);
    } else {
        let nwords = 2 * size.get().div_ceil(64) as usize;
        let (dst, src) = get_disjoint_dst_src(stack, dst, nwords * 8, src, nwords * 8);
        let dst = bytemuck::cast_slice_mut::<u8, u64>(dst);
        let src = bytemuck::cast_slice::<u8, u64>(src);
        let f = match op {
            O::LogicalLeft => fv_l_logical_shift_left,
            O::LogicalRight => fv_l_logical_shift_right,
            O::ArithmeticRight => fv_l_arithmetic_shift_right,
        };
        f(dst, src, offset, size);
    }
}

pub(crate) fn exec_fv_select_bit(
    stack: &mut Stack,
    dst: StackOffset,
    src: StackRef,
    idx: StackOffset,
) {
    let (spc, idx) = stack.load_exact_fv_u32(idx);
    if !spc != 0 || idx >= src.size.get() {
        stack.set_fv_scalar(dst, FvLogicValue::X);
        return;
    }

    if src.size.get() <= 16 {
        let result = fv_s_select_bit(stack.get(src.to_fv_size()), idx, src.size);
        stack.set_fv_scalar(dst, result);
    } else {
        let nwords = 2 * src.size.get().div_ceil(64) as usize;
        let result = fv_l_select_bit(stack.get_u64_slice(src.offset, nwords), idx, src.size);
        stack.set_fv_scalar(dst, result);
    }
}

pub(crate) fn exec_fv_concat(
    stack: &mut [u8],
    dst: usize,
    lhs: usize,
    lhs_size: VectorSize,
    rhs: usize,
    rhs_size: VectorSize,
) {
    if lhs_size.get() + rhs_size.get() > 16 {
        let (d, l, r) = get_disjoint_dst_s1_s2(
            stack,
            dst,
            2 * (lhs_size.get() + rhs_size.get()).div_ceil(64) as usize * 8,
            lhs,
            if lhs_size.get() > 32 {
                2 * lhs_size.get().div_ceil(64) as usize * 8
            } else {
                lhs_size.get().div_ceil(8) as usize
            },
            rhs,
            if rhs_size.get() > 32 {
                2 * rhs_size.get().div_ceil(64) as usize * 8
            } else {
                rhs_size.get().div_ceil(8) as usize
            },
        );
        let mut lhs_s = [0u64; 2];
        let mut rhs_s = [0u64; 2];
        let d = bytemuck::cast_slice_mut::<u8, u64>(d);
        let l = if lhs_size.get() <= 32 {
            (lhs_s[0], lhs_s[1]) = fv_unpack_u64(load_partial_u64(l, lhs_size), lhs_size);
            &lhs_s
        } else {
            bytemuck::cast_slice::<u8, u64>(l)
        };
        let r = if rhs_size.get() <= 32 {
            (rhs_s[0], rhs_s[1]) = fv_unpack_u64(load_partial_u64(r, rhs_size), rhs_size);
            &rhs_s
        } else {
            bytemuck::cast_slice::<u8, u64>(r)
        };
        fv_l_concat(d, l, r, lhs_size, rhs_size);
    } else {
        let (d, l, r) = get_disjoint_dst_s1_s2(
            stack,
            dst,
            (lhs_size.get() + rhs_size.get()).div_ceil(8) as usize,
            lhs,
            lhs_size.get().div_ceil(8) as usize,
            rhs,
            rhs_size.get().div_ceil(8) as usize,
        );
        fv_s_concat(d, l, r, lhs_size, rhs_size);
    }
}
