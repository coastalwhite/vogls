use vogls_bits::arithmetic::FvLogicValue;
use vogls_ir::{ResizeOp, SCALAR_VSIZE, UnaryOp, VectorSize};

use crate::{
    BinaryArithmeticOp, BinaryComparisonOp, EdgeOp, Heap, HeapOffset, HeapRef, ShiftOp, SliceFlags,
};

pub(crate) fn exec_tv_unary(stack: &mut Heap, dst: HeapOffset, op: UnaryOp, src: HeapRef) {
    use UnaryOp as O;
    match op {
        O::Neg if src.size <= Heap::TV_SUBBITS_MAX_SIZE => {
            let b = stack.get_subbit_byte(src);
            stack.set_aligned_raw_bits(dst.to_ref(src.size), !b);
        }
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
            stack.set_tv_bool(dst, result);
        }
        O::ReduceAnd => {
            let result = stack
                .get(src)
                .as_slice()
                .iter()
                .map(|b| b.count_ones())
                .sum::<u32>();
            stack.set_tv_bool(dst, result == src.size.get());
        }
        O::ReduceXor => {
            let result = stack
                .get(src)
                .as_slice()
                .iter()
                .map(|b| b.count_ones())
                .sum::<u32>();
            stack.set_tv_bool(dst, result % 2 == 1);
        }
    }
}

pub(crate) fn exec_tv_resize(stack: &mut Heap, dst: HeapRef, op: ResizeOp, src: HeapRef) {
    use ResizeOp as O;

    if dst.size <= Heap::TV_SUBBITS_MAX_SIZE && src.size <= Heap::TV_SUBBITS_MAX_SIZE {
        let src_b = &[stack.get_subbit_byte(src)];
        let mut dst_b = [0];
        match op {
            O::Truncate => vogls_bits::slice::tv_s_truncate(&mut dst_b, src_b, dst.size),
            O::ZeroExtend => {
                vogls_bits::extend::tv_s_zero_extend(&mut dst_b, src_b, dst.size, src.size)
            }
            O::SignExtend => {
                vogls_bits::extend::tv_s_sign_extend(&mut dst_b, src_b, dst.size, src.size)
            }
        }
        stack.set_aligned_raw_bits(dst, dst_b[0]);
        return;
    }

    let mut d_byte = [0];
    let s_byte;
    let (mut d, mut s) =
        stack.get_disjoint_u8_dst_src(dst.prev_byte_align(), src.prev_byte_align());
    if dst.size <= Heap::TV_SUBBITS_MAX_SIZE {
        d = &mut d_byte;
    }
    if src.size <= Heap::TV_SUBBITS_MAX_SIZE {
        s_byte = src.align_subbits(s[0]);
        s = std::slice::from_ref(&s_byte);
    }

    match op {
        O::ZeroExtend => {
            assert!(dst.size >= src.size);
            vogls_bits::extend::tv_s_zero_extend(d, s, dst.size, src.size);
        }
        O::SignExtend => {
            assert!(dst.size >= src.size);
            vogls_bits::extend::tv_s_sign_extend(d, s, dst.size, src.size);
        }
        O::Truncate => {
            vogls_bits::slice::tv_s_truncate(d, s, dst.size);
        }
    }
    if dst.size <= Heap::TV_SUBBITS_MAX_SIZE {
        stack.set_aligned_raw_bits(dst, d_byte[0]);
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
    if size >= Heap::TV_U64_MIN_SIZE
        && matches!(op, O::And | O::Or | O::Xor | O::Add | O::Sub | O::Multiply)
    {
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

        let lhs = lhs.to_ref(dst.size);
        let rhs = rhs.to_ref(dst.size);

        if dst.size <= Heap::TV_SUBBITS_MAX_SIZE {
            let mut dst_b = 0u8;
            let lhs = stack.get(lhs);
            let rhs = stack.get(rhs);
            f(
                std::slice::from_mut(&mut dst_b),
                lhs.as_slice(),
                rhs.as_slice(),
                dst.size,
            );
            stack.set_aligned_raw_bits(dst, dst_b);
        } else {
            let (dst_s, lhs_s, rhs_s) = stack.get_disjoint_u8_dst_s1_s2(dst, lhs, rhs);
            f(dst_s, lhs_s, rhs_s, dst.size);
        }
    }
}

pub(crate) fn exec_tv_bin_cmp(
    stack: &mut Heap,
    dst: HeapOffset,
    op: BinaryComparisonOp,
    lhs: HeapRef,
    rhs: HeapOffset,
) {
    fn tv_case_equality(lhs: &[u8], rhs: &[u8], _size: VectorSize) -> bool {
        lhs == rhs
    }

    use BinaryComparisonOp as O;
    let f = match op {
        O::UnsignedLessEqual => vogls_bits::comparison::tv_unsigned_leq,
        O::CaseEquality => tv_case_equality,
    };

    let size = lhs.size;
    let lhs = stack.get(lhs);
    let rhs = stack.get(rhs.to_ref(size));
    let result = f(lhs.as_slice(), rhs.as_slice(), size);
    stack.set_tv_u64(dst.to_ref(SCALAR_VSIZE), result as u64);
}

pub(crate) fn exec_tv_edge(
    heap: &mut Heap,
    dst: HeapOffset,
    op: EdgeOp,
    lhs: HeapOffset,
    rhs: HeapOffset,
) {
    use EdgeOp as O;
    let f = match op {
        O::Posedge => vogls_bits::edge::tv_posedge,
        O::Negedge => vogls_bits::edge::tv_negedge,
    };

    let lhs = heap.get_tv_bool(lhs);
    let rhs = heap.get_tv_bool(rhs);
    let result = f(lhs, rhs);
    heap.set_tv_bool(dst, result);
}

pub(crate) fn exec_tv_shift(
    stack: &mut Heap,
    dst: HeapRef,
    op: ShiftOp,
    src: HeapOffset,
    offset: HeapOffset,
) {
    let offset = stack.load_exact_tv_u32(offset);
    exec_tv_shift_imm(stack, dst, op, src, offset);
}

pub(crate) fn exec_tv_shift_imm(
    stack: &mut Heap,
    dst: HeapRef,
    op: ShiftOp,
    src: HeapOffset,
    offset: u32,
) {
    use ShiftOp as O;
    let size = dst.size;
    if size <= Heap::TV_SUBBITS_MAX_SIZE {
        let src_s = &[stack.get_subbit_byte(src.to_ref(size))];
        let mut dst_s = [0];
        let f = match op {
            O::LogicalLeft => vogls_bits::shift::tv_s_logical_shift_left,
            O::LogicalRight => vogls_bits::shift::tv_s_logical_shift_right,
            O::ArithmeticRight => vogls_bits::shift::tv_s_arithmetic_shift_right,
        };
        f(&mut dst_s, src_s, offset, size);
        stack.set_aligned_raw_bits(dst, dst_s[0]);
    } else if size < Heap::TV_U64_MIN_SIZE {
        let (dst, src) = stack.get_disjoint_u8_dst_src(dst, src.to_ref(size));
        let f = match op {
            O::LogicalLeft => vogls_bits::shift::tv_s_logical_shift_left,
            O::LogicalRight => vogls_bits::shift::tv_s_logical_shift_right,
            O::ArithmeticRight => vogls_bits::shift::tv_s_arithmetic_shift_right,
        };
        f(dst, src, offset, size);
    } else {
        let num_words = size.get().div_ceil(64) as usize;
        let (dst, src) = stack.get_disjoint_u64_dst_src((dst.offset, num_words), (src, num_words));
        let f = match op {
            O::LogicalLeft => vogls_bits::shift::tv_l_logical_shift_left,
            O::LogicalRight => vogls_bits::shift::tv_l_logical_shift_right,
            O::ArithmeticRight => vogls_bits::shift::tv_l_arithmetic_shift_right,
        };
        f(dst, src, offset, size);
    }
}

pub(crate) fn exec_tv_slice(
    stack: &mut Heap,
    dst: HeapRef,
    src: HeapRef,
    offset: HeapOffset,
    flags: SliceFlags,
) {
    let offset = if flags.offset_is_fv {
        let (spc, offset) = stack.load_exact_fv_u32(offset);
        if !spc != 0 {
            stack.set_unknown(dst);
            return;
        }
        offset
    } else {
        stack.load_exact_tv_u32(offset)
    };
    exec_tv_slice_imm(stack, dst, src, offset, flags.fill_with_x);
}

pub(crate) fn exec_tv_slice_imm(
    stack: &mut Heap,
    dst: HeapRef,
    src: HeapRef,
    offset: u32,
    fill_with_x: bool,
) {
    if dst.size < Heap::FV_U64_MIN_SIZE && src.size < Heap::TV_U64_MIN_SIZE {
        let val = stack.get_tv_u64(src);
        let (spc, val) = vogls_bits::slice::tv_s_slice(val, offset, dst.size, src.size);
        if fill_with_x {
            stack.set_fv_u64(dst, spc, val);
        } else {
            stack.set_tv_u64(dst, val);
        }
    } else if dst.size < Heap::FV_U64_MIN_SIZE && src.size >= Heap::TV_U64_MIN_SIZE {
        let src_size = src.size;
        let src = stack.get_mut_u64_slice(src.offset, src.size.get().div_ceil(64) as usize);
        let (spc, val) = vogls_bits::slice::tv_ls_slice(src, offset, dst.size, src_size);
        if fill_with_x {
            stack.set_fv_u64(dst, spc, val);
        } else {
            stack.set_tv_u64(dst, val);
        }
    } else if fill_with_x {
        let (dst_s, src_s) = stack.get_disjoint_u64_dst_src(
            (dst.offset, 2 * dst.size.get().div_ceil(64) as usize),
            (src.offset, src.size.get().div_ceil(64) as usize),
        );
        vogls_bits::slice::tv_ll_slice(dst_s, src_s, offset, dst.size, src.size, true);
    } else {
        let (dst_s, src_s) = stack.get_disjoint_u64_dst_src(
            (dst.offset, dst.size.get().div_ceil(64) as usize),
            (src.offset, src.size.get().div_ceil(64) as usize),
        );
        vogls_bits::slice::tv_part_ll_slice(dst_s, src_s, offset, dst.size, src.size, false);
    }
}

pub(crate) fn exec_tv_concat(stack: &mut Heap, dst: HeapOffset, lhs: HeapRef, rhs: HeapRef) {
    let (lhs_size, rhs_size) = (lhs.size, rhs.size);
    let dst_size = lhs_size.checked_add(rhs_size.get()).unwrap();

    if dst_size <= Heap::TV_SUBBITS_MAX_SIZE {
        let lhs_byte = stack.get_subbit_byte(lhs);
        let rhs_byte = stack.get_subbit_byte(rhs);
        let mut dst_byte = [0];

        vogls_bits::concat::tv_concat(&mut dst_byte, &[lhs_byte], &[rhs_byte], lhs_size, rhs_size);
        stack.set_aligned_raw_bits(dst.to_ref(dst_size), dst_byte[0]);
    } else {
        let lhs_byte;
        let rhs_byte;

        let (dst, mut lhs_slice, mut rhs_slice) = stack.get_disjoint_u8_dst_s1_s2(
            dst.to_ref(dst_size),
            lhs.prev_byte_align(),
            rhs.prev_byte_align(),
        );

        if lhs.size <= Heap::TV_SUBBITS_MAX_SIZE {
            lhs_byte = lhs.align_subbits(lhs_slice[0]);
            lhs_slice = std::slice::from_ref(&lhs_byte);
        }
        if rhs.size <= Heap::TV_SUBBITS_MAX_SIZE {
            rhs_byte = rhs.align_subbits(rhs_slice[0]);
            rhs_slice = std::slice::from_ref(&rhs_byte);
        }
        vogls_bits::concat::tv_concat(dst, lhs_slice, rhs_slice, lhs_size, rhs_size);
    }
}

pub(crate) fn exec_tv_select(
    heap: &mut Heap,
    dst: HeapRef,
    cond: HeapOffset,
    truthy: HeapOffset,
    falsy: HeapOffset,
    cond_is_fv: bool,
) {
    let src = if (cond_is_fv && heap.get_fv_item(cond) == FvLogicValue::L1) || (!cond_is_fv && heap.get_tv_bool(cond)) {
        truthy
    } else {
        falsy
    };

    heap.tv_copy(dst.size, dst.offset, src)
}
