use vogls_bits::arithmetic::{
    FvLogicValue, fv_gtu32_bitwise_inv, fv_l_reduce_and, fv_l_reduce_or, fv_l_reduce_xor,
    fv_leu32_bitwise_inv, fv_pack_u64, fv_s_reduce_and, fv_s_reduce_or, fv_s_reduce_xor,
    fv_unpack_u64,
};
use vogls_bits::concat::{fv_l_concat, fv_s_concat};
use vogls_bits::extend::{fv_l_sign_extend, fv_l_zero_extend, fv_s_sign_extend, fv_s_zero_extend};
use vogls_bits::load::load_partial_u64;
use vogls_bits::shift::{
    fv_l_arithmetic_shift_right, fv_l_logical_shift_left, fv_l_logical_shift_right,
    fv_s_arithmetic_shift_right, fv_s_logical_shift_left, fv_s_logical_shift_right,
};
use vogls_bits::truncate::{fv_l_truncate, fv_s_truncate};
use vogls_ir::{ResizeOp, UnaryOp, VectorSize};

use crate::{BinaryArithmeticOp, BinaryComparisonOp, EdgeOp, Heap, HeapOffset, HeapRef, ShiftOp};

pub(crate) fn exec_fv_unary(stack: &mut Heap, dst: HeapOffset, op: UnaryOp, src: HeapRef) {
    use UnaryOp as O;
    match op {
        O::Neg if src.size <= Heap::FV_SUBBITS_MAX_SIZE => {
            let b = stack.get_subbit_byte(src.to_fv_size());
            let (spc, value) = fv_unpack_u64(b as u64, src.size);
            let (spc, value) = vogls_bits::arithmetic::fv_bitwise_inv_elem(spc, value);
            stack.set_aligned_raw_bits(
                dst.to_ref(src.size).to_fv_size(),
                fv_pack_u64(spc, value, src.size) as u8,
            );
        }
        O::Neg if src.size >= Heap::FV_U64_MIN_SIZE => {
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

        O::ReduceOr | O::ReduceAnd | O::ReduceXor if src.size >= Heap::FV_U64_MIN_SIZE => {
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
            let src_s = stack.get(src.to_fv_size());
            let f = match op {
                O::Neg => unreachable!(),
                O::ReduceOr => fv_s_reduce_or,
                O::ReduceAnd => fv_s_reduce_and,
                O::ReduceXor => fv_s_reduce_xor,
            };
            let result = f(src_s.as_slice(), src.size);
            stack.set_fv_scalar(dst, result);
        }
    };
}

pub(crate) fn exec_fv_resize(stack: &mut Heap, dst: HeapRef, op: ResizeOp, src: HeapRef) {
    use ResizeOp as O;
    match op {
        O::Truncate | O::ZeroExtend | O::SignExtend
            if dst.size <= Heap::FV_SUBBITS_MAX_SIZE && src.size <= Heap::FV_SUBBITS_MAX_SIZE =>
        {
            let src_s = stack.get_subbit_byte(src.to_fv_size());
            let mut dst_s = [0];
            let f = match op {
                O::Truncate => fv_s_truncate,
                O::ZeroExtend => fv_s_zero_extend,
                O::SignExtend => fv_s_sign_extend,
            };
            f(&mut dst_s, &[src_s], dst.size, src.size);
            stack.set_aligned_raw_bits(dst.to_fv_size(), dst_s[0]);
        }
        O::Truncate
            if src.size < Heap::FV_U64_MIN_SIZE && dst.size <= Heap::FV_SUBBITS_MAX_SIZE =>
        {
            let src_s = stack.get(src.to_fv_size());
            let src_s = src_s.as_slice();
            let mut dst_s = [0];
            fv_s_truncate(&mut dst_s, src_s, dst.size, src.size);
            stack.set_aligned_raw_bits(dst.to_fv_size(), dst_s[0]);
        }
        O::ZeroExtend | O::SignExtend
            if dst.size >= Heap::FV_U64_MIN_SIZE && src.size <= Heap::FV_SUBBITS_MAX_SIZE =>
        {
            let mut src_s = [0, 0];
            (src_s[0], src_s[1]) = stack.get_fv_u64(src);
            let dst_s =
                stack.get_mut_u64_slice(dst.offset, 2 * dst.size.get().div_ceil(64) as usize);
            let f = match op {
                O::Truncate => unreachable!(),
                O::ZeroExtend => fv_l_zero_extend,
                O::SignExtend => fv_l_sign_extend,
            };
            f(dst_s, &src_s, dst.size, src.size);
        }
        O::ZeroExtend | O::SignExtend if src.size <= Heap::FV_SUBBITS_MAX_SIZE => {
            let src_s = stack.get_subbit_byte(src.to_fv_size());
            let dst_s = stack.get_mut(dst.to_fv_size());
            let f = match op {
                O::Truncate => unreachable!(),
                O::ZeroExtend => fv_s_zero_extend,
                O::SignExtend => fv_s_sign_extend,
            };
            f(dst_s, &[src_s], dst.size, src.size);
        }
        O::Truncate | O::ZeroExtend | O::SignExtend
            if dst.size < Heap::FV_U64_MIN_SIZE && src.size < Heap::FV_U64_MIN_SIZE =>
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
            if dst.size >= Heap::FV_U64_MIN_SIZE && src.size >= Heap::FV_U64_MIN_SIZE =>
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
    stack: &mut Heap,
    dst: HeapRef,
    op: BinaryArithmeticOp,
    lhs: HeapOffset,
    rhs: HeapOffset,
) {
    use BinaryArithmeticOp as O;

    use vogls_bits::arithmetic as A;
    use vogls_bits::copyxz as C;

    fn fv_u8_bitwise_and(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        A::fv_bin_bitwise_op(dst, lhs, rhs, size, A::fv_bitwise_and_elem)
    }
    fn fv_u8_bitwise_or(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        A::fv_bin_bitwise_op(dst, lhs, rhs, size, A::fv_bitwise_or_elem)
    }
    fn fv_u8_bitwise_xor(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        A::fv_bin_bitwise_op(dst, lhs, rhs, size, A::fv_bitwise_xor_elem)
    }
    fn fv_u8_min(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        match vogls_bits::comparison::fv_s_unsigned_leq(lhs, rhs, size) {
            FvLogicValue::L0 => dst.copy_from_slice(rhs),
            FvLogicValue::L1 => dst.copy_from_slice(lhs),
            FvLogicValue::X | FvLogicValue::Z => dst.fill(0),
        }
    }
    fn fv_u8_max(dst: &mut [u8], lhs: &[u8], rhs: &[u8], size: VectorSize) {
        match vogls_bits::comparison::fv_s_unsigned_leq(lhs, rhs, size) {
            FvLogicValue::L0 => dst.copy_from_slice(lhs),
            FvLogicValue::L1 => dst.copy_from_slice(rhs),
            FvLogicValue::X | FvLogicValue::Z => dst.fill(0),
        }
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
    fn fv_u64_copy_x(dst: &mut [u64], lhs: &[u64], rhs: &[u64], _size: VectorSize) {
        C::fv_l_copy_x(dst, lhs, rhs);
    }
    fn fv_u64_copy_z(dst: &mut [u64], lhs: &[u64], rhs: &[u64], _size: VectorSize) {
        C::fv_l_copy_z(dst, lhs, rhs);
    }
    fn fv_u64_min(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
        match vogls_bits::comparison::fv_l_unsigned_leq(lhs, rhs, size) {
            FvLogicValue::L1 => dst.copy_from_slice(lhs),
            FvLogicValue::L0 => dst.copy_from_slice(rhs),
            FvLogicValue::X | FvLogicValue::Z => dst.fill(0),
        }
    }
    fn fv_u64_max(dst: &mut [u64], lhs: &[u64], rhs: &[u64], size: VectorSize) {
        match vogls_bits::comparison::fv_l_unsigned_leq(lhs, rhs, size) {
            FvLogicValue::L1 => dst.copy_from_slice(rhs),
            FvLogicValue::L0 => dst.copy_from_slice(lhs),
            FvLogicValue::X | FvLogicValue::Z => dst.fill(0),
        }
    }

    if dst.size >= Heap::FV_U64_MIN_SIZE {
        let f = match op {
            O::And => fv_u64_bitwise_and,
            O::Or => fv_u64_bitwise_or,
            O::Xor => fv_u64_bitwise_xor,
            O::Add => A::fv_addition,
            O::Sub => A::fv_subtraction,
            O::Power => A::fv_power,
            O::Multiply => A::fv_multiplication,
            O::Divide => fv_u64_division,
            O::Modulus => fv_u64_modulus,
            O::CopyX => fv_u64_copy_x,
            O::CopyZ => fv_u64_copy_z,
            O::Min => fv_u64_min,
            O::Max => fv_u64_max,
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
            O::Power => A::fv_ltu32_power,
            O::Multiply => A::fv_ltu32_multiplication,
            O::Divide => A::fv_ltu32_division,
            O::Modulus => A::fv_ltu32_modulus,
            O::CopyX => C::fv_s_copy_x,
            O::CopyZ => C::fv_s_copy_z,
            O::Min => fv_u8_min,
            O::Max => fv_u8_max,
        };

        let size = dst.size;
        let dst = dst.to_fv_size();
        let lhs = lhs.to_ref(dst.size);
        let rhs = rhs.to_ref(dst.size);

        if size <= Heap::FV_SUBBITS_MAX_SIZE {
            let mut dst_b = 0u8;
            let lhs = stack.get(lhs);
            let rhs = stack.get(rhs);
            f(
                std::slice::from_mut(&mut dst_b),
                lhs.as_slice(),
                rhs.as_slice(),
                size,
            );
            stack.set_aligned_raw_bits(dst, dst_b);
        } else {
            let (dst_s, lhs_s, rhs_s) = stack.get_disjoint_u8_dst_s1_s2(dst, lhs, rhs);
            f(dst_s, lhs_s, rhs_s, size);
        }
    }
}

pub(crate) fn exec_fv_bin_cmp(
    stack: &mut Heap,
    dst: HeapOffset,
    op: BinaryComparisonOp,
    lhs: HeapRef,
    rhs: HeapOffset,
) {
    use BinaryComparisonOp as O;
    match op {
        O::UnsignedLessEqual if lhs.size < Heap::FV_U64_MIN_SIZE => {
            let lhs_s = stack.get(lhs.to_fv_size());
            let rhs_s = stack.get(rhs.to_ref(lhs.size).to_fv_size());
            let result = vogls_bits::comparison::fv_s_unsigned_leq(
                lhs_s.as_slice(),
                rhs_s.as_slice(),
                lhs.size,
            );
            stack.set_fv_scalar(dst, result);
        }
        O::UnsignedLessEqual => {
            let nwords = 2 * lhs.size.get().div_ceil(64) as usize;
            let lhs_s = stack.get_u64_slice(lhs.offset, nwords);
            let rhs_s = stack.get_u64_slice(rhs, nwords);
            let result = vogls_bits::comparison::fv_l_unsigned_leq(lhs_s, rhs_s, lhs.size);
            stack.set_fv_scalar(dst, result);
        }
        O::CaseEquality if lhs.size < Heap::FV_U64_MIN_SIZE => {
            let lhs_s = stack.get(lhs.to_fv_size());
            let rhs_s = stack.get(rhs.to_ref(lhs.size).to_fv_size());
            stack.set_tv_bool(dst, lhs_s.as_slice() == rhs_s.as_slice());
        }
        O::CaseEquality => {
            let nwords = 2 * lhs.size.get().div_ceil(64) as usize;
            let lhs_s = stack.get_u64_slice(lhs.offset, nwords);
            let rhs_s = stack.get_u64_slice(rhs, nwords);
            stack.set_tv_bool(dst, lhs_s == rhs_s);
        }
    }
}

pub(crate) fn exec_fv_edge(
    heap: &mut Heap,
    dst: HeapOffset,
    op: EdgeOp,
    lhs: HeapOffset,
    rhs: HeapOffset,
) {
    use EdgeOp as O;
    let f = match op {
        O::Posedge => vogls_bits::edge::fv_posedge,
        O::Negedge => vogls_bits::edge::fv_negedge,
    };

    let lhs = heap.get_fv_item(lhs);
    let rhs = heap.get_fv_item(rhs);
    let result = f(lhs, rhs);
    heap.set_tv_bool(dst, result);
}

pub(crate) fn exec_fv_shift(
    stack: &mut Heap,
    dst: HeapRef,
    op: ShiftOp,
    src: HeapOffset,
    offset: HeapOffset,
) {
    use ShiftOp as O;
    let (spc, offset) = stack.load_exact_fv_u32(offset);

    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 53
    // """
    // If the right operand has an x or z value, then the result shall be unknown.
    // """
    if !spc != 0 {
        stack.set_unknown(dst);
        return;
    }

    if dst.size.get() <= 2 {
        let src_s = &[stack.get_subbit_byte(src.to_ref(dst.size).to_fv_size())];
        let mut dst_s = [0];
        let f = match op {
            O::LogicalLeft => fv_s_logical_shift_left,
            O::LogicalRight => fv_s_logical_shift_right,
            O::ArithmeticRight => fv_s_arithmetic_shift_right,
        };
        f(&mut dst_s, src_s, offset, dst.size);
        stack.set_aligned_raw_bits(dst.to_fv_size(), dst_s[0]);
    } else if dst.size < Heap::FV_U64_MIN_SIZE {
        let (dst_s, src_s) =
            stack.get_disjoint_u8_dst_src(dst.to_fv_size(), src.to_ref(dst.size).to_fv_size());
        let f = match op {
            O::LogicalLeft => fv_s_logical_shift_left,
            O::LogicalRight => fv_s_logical_shift_right,
            O::ArithmeticRight => fv_s_arithmetic_shift_right,
        };
        f(dst_s, src_s, offset, dst.size);
    } else {
        let nwords = 2 * dst.size.get().div_ceil(64) as usize;
        let (dst_s, src_s) = stack.get_disjoint_u64_dst_src((dst.offset, nwords), (src, nwords));
        let f = match op {
            O::LogicalLeft => fv_l_logical_shift_left,
            O::LogicalRight => fv_l_logical_shift_right,
            O::ArithmeticRight => fv_l_arithmetic_shift_right,
        };
        f(dst_s, src_s, offset, dst.size);
    }
}

pub(crate) fn exec_fv_slice(
    stack: &mut Heap,
    dst: HeapRef,
    src: HeapRef,
    idx: HeapOffset,
    fill_with_x: bool,
) {
    let (spc, offset) = stack.load_exact_fv_u32(idx);
    if !spc != 0 {
        stack.set_unknown(dst);
        return;
    }

    if dst.size < Heap::FV_U64_MIN_SIZE && src.size < Heap::FV_U64_MIN_SIZE {
        let (spc, val) = stack.get_fv_u64(src);
        let (spc, val) = vogls_bits::slice::fv_s_slice(spc, val, offset, dst.size, src.size, fill_with_x);
        stack.set_fv_u64(dst, spc, val);
    } else if dst.size < Heap::FV_U64_MIN_SIZE && src.size >= Heap::FV_U64_MIN_SIZE {
        let src_size = src.size;
        let src = stack.get_mut_u64_slice(src.offset, 2 * src.size.get().div_ceil(64) as usize);
        let (spc, val) = vogls_bits::slice::fv_ls_slice(src, offset, dst.size, src_size, fill_with_x);
        stack.set_fv_u64(dst, spc, val);
    } else {
        let (dst_s, src_s) = stack.get_disjoint_u64_dst_src(
            (dst.offset, 2 * dst.size.get().div_ceil(64) as usize),
            (src.offset, 2 * src.size.get().div_ceil(64) as usize),
        );
        vogls_bits::slice::fv_ll_slice(dst_s, src_s, offset, dst.size, src.size, fill_with_x);
    }
}

pub(crate) fn exec_fv_concat(stack: &mut Heap, dst: HeapOffset, lhs: HeapRef, rhs: HeapRef) {
    let dst = dst.to_ref(VectorSize::new(lhs.size.get() + rhs.size.get()).unwrap());
    if dst.size <= Heap::FV_SUBBITS_MAX_SIZE {
        let lhs_byte = stack.get_subbit_byte(lhs.to_fv_size());
        let rhs_byte = stack.get_subbit_byte(rhs.to_fv_size());
        let mut dst_byte = [0];

        vogls_bits::concat::fv_s_concat(
            &mut dst_byte,
            &[lhs_byte],
            &[rhs_byte],
            lhs.size,
            rhs.size,
        );
        stack.set_aligned_raw_bits(dst.to_fv_size(), dst_byte[0]);
    } else if dst.size >= Heap::FV_U64_MIN_SIZE {
        let l_nbytes = if lhs.size < Heap::FV_U64_MIN_SIZE {
            (2 * lhs.size.get()).div_ceil(8)
        } else {
            2 * lhs.size.get().div_ceil(64) * 8
        };
        let r_nbytes = if rhs.size < Heap::FV_U64_MIN_SIZE {
            (2 * rhs.size.get()).div_ceil(8)
        } else {
            2 * rhs.size.get().div_ceil(64) * 8
        };
        let d_nbytes = 2 * dst.size.get().div_ceil(64) * 8;

        let (d, l, r) = stack.get_disjoint_u8_dst_s1_s2(
            dst.offset.to_ref(VectorSize::new(d_nbytes * 8).unwrap()),
            lhs.offset
                .to_ref(VectorSize::new(l_nbytes * 8).unwrap())
                .prev_byte_align(),
            rhs.offset
                .to_ref(VectorSize::new(r_nbytes * 8).unwrap())
                .prev_byte_align(),
        );
        let mut lhs_s = [0u64; 2];
        let mut rhs_s = [0u64; 2];
        let d = bytemuck::cast_slice_mut::<u8, u64>(d);
        let l = if lhs.size <= Heap::FV_SUBBITS_MAX_SIZE {
            let l = lhs.to_fv_size().align_subbits(l[0]);
            (lhs_s[0], lhs_s[1]) = fv_unpack_u64(l as u64, lhs.size);
            &lhs_s
        } else if lhs.size < Heap::FV_U64_MIN_SIZE {
            (lhs_s[0], lhs_s[1]) =
                fv_unpack_u64(load_partial_u64(l, lhs.to_fv_size().size), lhs.size);
            &lhs_s
        } else {
            bytemuck::cast_slice::<u8, u64>(l)
        };
        let r = if rhs.size <= Heap::FV_SUBBITS_MAX_SIZE {
            let r = rhs.to_fv_size().align_subbits(r[0]);
            (rhs_s[0], rhs_s[1]) = fv_unpack_u64(r as u64, rhs.size);
            &rhs_s
        } else if rhs.size < Heap::FV_U64_MIN_SIZE {
            (rhs_s[0], rhs_s[1]) =
                fv_unpack_u64(load_partial_u64(r, rhs.to_fv_size().size), rhs.size);
            &rhs_s
        } else {
            bytemuck::cast_slice::<u8, u64>(r)
        };
        fv_l_concat(d, l, r, lhs.size, rhs.size);
    } else {
        let lhs_byte;
        let rhs_byte;
        let (d, mut l, mut r) = stack.get_disjoint_u8_dst_s1_s2(
            dst.to_fv_size(),
            lhs.to_fv_size().prev_byte_align(),
            rhs.to_fv_size().prev_byte_align(),
        );

        if lhs.size <= Heap::FV_SUBBITS_MAX_SIZE {
            lhs_byte = lhs.to_fv_size().align_subbits(l[0]);
            l = std::slice::from_ref(&lhs_byte);
        }
        if rhs.size <= Heap::FV_SUBBITS_MAX_SIZE {
            rhs_byte = rhs.to_fv_size().align_subbits(r[0]);
            r = std::slice::from_ref(&rhs_byte);
        }
        fv_s_concat(d, l, r, lhs.size, rhs.size);
    }
}
