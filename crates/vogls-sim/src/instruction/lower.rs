use std::collections::HashMap;

use vogls_codegen::{
    HeapBuilder, HeapOffset, HeapRef, bin_imm_args_need_conversion, insert_bb_phis,
    resolve_heap_map,
};
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, GlobalContext, Instruction,
    IntrinsicOp, LogicMode, ProcessKey, ResizeOp, SCALAR_VSIZE, ShiftImmOp, SignalKey, UnaryOp,
    VariableKey, VectorSize,
};
use vogls_runtime::RtSignalKey;
use vogls_utils::{VgHashMap, VgHashSet};

use crate::instruction::{VmInstruction, VmProcess};
use crate::{BinaryArithmeticOp, BinaryComparisonOp, EdgeOp, ShiftOp, SliceFlags, VmIntrinsicOp};

pub fn lower_unary_op(
    instrs: &mut Vec<VmInstruction>,
    op: UnaryOp,
    dst: HeapOffset,
    src: HeapRef,
    src_mode: LogicMode,
) {
    use UnaryOp as O;
    use VmInstruction as VI;

    let i = match op {
        O::TvToFv => VI::TvToFv(dst.to_ref(src.size), src.offset),
        O::FvToTv => VI::FvToTv(dst.to_ref(src.size), src.offset),
        _ => {
            if src_mode == LogicMode::FourValue {
                VI::FvUnary(dst, op, src)
            } else {
                if src.size == SCALAR_VSIZE {
                    match op {
                        O::Neg => VI::TvNot1(dst, src.offset),
                        O::ReduceOr | O::ReduceAnd | O::ReduceXor => VI::TvMove1(dst, src.offset),
                        O::TvToFv => VI::TvToFv(dst.to_ref(src.size), src.offset),
                        O::FvToTv => VI::FvToTv(dst.to_ref(src.size), src.offset),
                        O::LeadingZeros => todo!(),
                    }
                } else {
                    match op {
                        O::TvToFv => VI::TvToFv(dst.to_ref(src.size), src.offset),
                        O::FvToTv => VI::FvToTv(dst.to_ref(src.size), src.offset),
                        _ => VI::TvUnary(dst, op, src),
                    }
                }
            }
        }
    };
    instrs.push(i);
}

pub fn lower_resize_op(
    instrs: &mut Vec<VmInstruction>,
    op: ResizeOp,
    dst: HeapRef,
    src: HeapRef,
    mode: LogicMode,
) {
    use VmInstruction as VI;
    if src.size == dst.size {
        lower_move_op(instrs, dst.offset, src.offset, src.size, mode);
        return;
    }
    let i = if mode == LogicMode::FourValue {
        VI::FvResize(dst, op, src)
    } else {
        if src.size == SCALAR_VSIZE {
            match op {
                ResizeOp::Truncate => VI::TvMove1(dst.offset, src.offset),
                ResizeOp::ZeroExtend => VI::TvZeroExtend1(dst, src.offset),
                ResizeOp::SignExtend => VI::TvSignExtend1(dst, src.offset),
            }
        } else {
            VI::TvResize(dst, op, src)
        }
    };
    instrs.push(i);
}

pub fn lower_move_op(
    instrs: &mut Vec<VmInstruction>,
    dst: HeapOffset,
    src: HeapOffset,
    size: VectorSize,
    mode: LogicMode,
) {
    use VmInstruction as VI;
    let i = if mode == LogicMode::FourValue {
        VI::FvResize(dst.to_ref(size), ResizeOp::Truncate, src.to_ref(size))
    } else {
        if size == SCALAR_VSIZE {
            VI::TvMove1(dst, src)
        } else {
            VI::TvResize(dst.to_ref(size), ResizeOp::Truncate, src.to_ref(size))
        }
    };
    instrs.push(i);
}

pub fn lower_convert_op(
    instrs: &mut Vec<VmInstruction>,
    dst: HeapOffset,
    src: HeapOffset,
    size: VectorSize,
    dst_mode: LogicMode,
    src_mode: LogicMode,
) {
    use LogicMode as M;
    use VmInstruction as VI;
    match (dst_mode, src_mode) {
        (M::TwoValue, M::TwoValue) | (M::FourValue, M::FourValue) => {
            lower_move_op(instrs, dst, src, size, dst_mode)
        }
        (M::TwoValue, M::FourValue) => instrs.push(VI::FvToTv(dst.to_ref(size), src)),
        (M::FourValue, M::TwoValue) => instrs.push(VI::TvToFv(dst.to_ref(size), src)),
    }
}

pub fn lower_slice_imm(
    instrs: &mut Vec<VmInstruction>,
    dst: HeapRef,
    src: HeapRef,
    offset: u32,
    mode: LogicMode,
) {
    use LogicMode as M;
    use VmInstruction as VI;
    let i = if mode == M::FourValue {
        VI::FvSliceImm(dst, src, offset)
    } else {
        let is_in_range = offset + dst.size.get() <= src.size.get();
        let src_is_dw =
            ((src.offset.bit_offset + offset as usize) % 64) + dst.size.get() as usize > 64;
        let dst_is_dw = (dst.offset.bit_offset % 64) + dst.size.get() as usize > 64;
        match dst.size.get() {
            1 if is_in_range => VI::TvMove1(
                dst.offset,
                HeapOffset {
                    bit_offset: src.offset.bit_offset + offset as usize,
                },
            ),
            2..=64 if is_in_range && src_is_dw && dst_is_dw => VI::TvDwDwMove(
                dst.offset,
                HeapOffset {
                    bit_offset: src.offset.bit_offset + offset as usize,
                }
                .to_ref(dst.size),
            ),
            2..=64 if is_in_range && src_is_dw => VI::TvSwDwMove(
                dst.offset,
                HeapOffset {
                    bit_offset: src.offset.bit_offset + offset as usize,
                }
                .to_ref(dst.size),
            ),
            2..=64 if is_in_range && dst_is_dw => VI::TvDwSwMove(
                dst.offset,
                HeapOffset {
                    bit_offset: src.offset.bit_offset + offset as usize,
                }
                .to_ref(dst.size),
            ),
            2..=64 if is_in_range => VI::TvSwSwMove(
                dst.offset,
                HeapOffset {
                    bit_offset: src.offset.bit_offset + offset as usize,
                }
                .to_ref(dst.size),
            ),
            _ => VI::TvSliceImm(dst, src, offset),
        }
    };
    instrs.push(i);
}

pub fn lower_bin_op(
    instrs: &mut Vec<VmInstruction>,
    op: BinaryOp,
    dst: HeapRef,
    lhs: HeapRef,
    rhs: HeapRef,
    dst_mode: LogicMode,
    lhs_mode: LogicMode,
    _rhs_mode: LogicMode,
) {
    use BinaryArithmeticOp as BA;
    use BinaryComparisonOp as BC;
    use BinaryOp as O;
    use LogicMode as M;
    use ShiftOp as S;
    use VmInstruction as VI;
    let i = if dst_mode == M::FourValue {
        match op {
            O::And => VI::FvBinaryArithmetic(dst, BA::And, lhs.offset, rhs.offset),
            O::Or => VI::FvBinaryArithmetic(dst, BA::Or, lhs.offset, rhs.offset),
            O::Xor => VI::FvBinaryArithmetic(dst, BA::Xor, lhs.offset, rhs.offset),
            O::Add => VI::FvBinaryArithmetic(dst, BA::Add, lhs.offset, rhs.offset),
            O::Sub => VI::FvBinaryArithmetic(dst, BA::Sub, lhs.offset, rhs.offset),
            O::Power => VI::FvBinaryArithmetic(dst, BA::Power, lhs.offset, rhs.offset),
            O::Multiply => VI::FvBinaryArithmetic(dst, BA::Multiply, lhs.offset, rhs.offset),
            O::DivideX => {
                if lhs_mode == M::TwoValue {
                    VI::FvTvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset)
                } else {
                    VI::FvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset)
                }
            }
            O::Divide0 => todo!(),
            O::ModulusX => {
                if lhs_mode == M::TwoValue {
                    VI::FvTvBinaryArithmetic(dst, BA::Modulus, lhs.offset, rhs.offset)
                } else {
                    VI::FvBinaryArithmetic(dst, BA::Modulus, lhs.offset, rhs.offset)
                }
            }
            O::Modulus0 => todo!(),
            O::Min => VI::FvBinaryArithmetic(dst, BA::Min, lhs.offset, rhs.offset),
            O::Max => VI::FvBinaryArithmetic(dst, BA::Max, lhs.offset, rhs.offset),

            O::UnsignedLessEqual => {
                VI::FvBinaryComparison(dst.offset, BC::UnsignedLessEqual, lhs, rhs.offset)
            }
            O::CaseEquality => unreachable!(),
            O::LogicalShiftLeft => VI::FvShift(dst, S::LogicalLeft, lhs.offset, rhs.offset),
            O::LogicalShiftRight => VI::FvShift(dst, S::LogicalRight, lhs.offset, rhs.offset),
            O::ArithmeticShiftRight => VI::FvShift(dst, S::ArithmeticRight, lhs.offset, rhs.offset),
            O::Concat => VI::FvConcat(dst.offset, lhs, rhs),

            O::CopyX => VI::FvBinaryArithmetic(dst, BA::CopyX, lhs.offset, rhs.offset),
            O::CopyZ => VI::FvBinaryArithmetic(dst, BA::CopyZ, lhs.offset, rhs.offset),
            O::Posedge => unreachable!(),
            O::Negedge => unreachable!(),
        }
    } else {
        if lhs.size == SCALAR_VSIZE {
            match op {
                O::And => VI::TvAnd1(dst.offset, lhs.offset, rhs.offset),
                O::Or => VI::TvOr1(dst.offset, lhs.offset, rhs.offset),
                O::Xor => VI::TvXor1(dst.offset, lhs.offset, rhs.offset),
                O::Add => VI::TvXor1(dst.offset, lhs.offset, rhs.offset),
                O::Sub => VI::TvXor1(dst.offset, lhs.offset, rhs.offset),
                O::Power => VI::TvOrNot1(dst.offset, rhs.offset, lhs.offset),
                O::Multiply => VI::TvAnd1(dst.offset, lhs.offset, rhs.offset),
                O::DivideX => VI::TvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset),
                O::Divide0 => VI::TvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset),
                O::ModulusX => VI::TvBinaryArithmetic(dst, BA::Modulus, lhs.offset, rhs.offset),
                O::Modulus0 => VI::TvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset),
                O::Min => VI::TvAnd1(dst.offset, lhs.offset, rhs.offset),
                O::Max => VI::TvOr1(dst.offset, lhs.offset, rhs.offset),
                O::UnsignedLessEqual => VI::TvOrNot1(dst.offset, rhs.offset, lhs.offset),
                O::CaseEquality if lhs_mode == M::TwoValue => {
                    VI::TvXnor1(dst.offset, lhs.offset, rhs.offset)
                }
                O::CaseEquality => {
                    VI::FvBinaryComparison(dst.offset, BC::CaseEquality, lhs, rhs.offset)
                }
                O::LogicalShiftLeft => VI::TvShift(dst, S::LogicalLeft, lhs.offset, rhs.offset),
                O::LogicalShiftRight => VI::TvShift(dst, S::LogicalRight, lhs.offset, rhs.offset),
                O::ArithmeticShiftRight => {
                    VI::TvShift(dst, S::ArithmeticRight, lhs.offset, rhs.offset)
                }
                O::Concat => VI::TvConcat(dst.offset, lhs, rhs),
                O::CopyX | O::CopyZ => VI::TvMove1(dst.offset, lhs.offset),
                O::Posedge if lhs_mode == M::TwoValue => {
                    VI::TvAndNot1(dst.offset, rhs.offset, lhs.offset)
                }
                O::Negedge if lhs_mode == M::TwoValue => {
                    VI::TvAndNot1(dst.offset, lhs.offset, rhs.offset)
                }
                O::Posedge => VI::FvEdge(dst.offset, EdgeOp::Posedge, lhs.offset, rhs.offset),
                O::Negedge => VI::FvEdge(dst.offset, EdgeOp::Negedge, lhs.offset, rhs.offset),
            }
        } else {
            match op {
                O::And => VI::TvBinaryArithmetic(dst, BA::And, lhs.offset, rhs.offset),
                O::Or => VI::TvBinaryArithmetic(dst, BA::Or, lhs.offset, rhs.offset),
                O::Xor => VI::TvBinaryArithmetic(dst, BA::Xor, lhs.offset, rhs.offset),
                O::Add => VI::TvBinaryArithmetic(dst, BA::Add, lhs.offset, rhs.offset),
                O::Sub => VI::TvBinaryArithmetic(dst, BA::Sub, lhs.offset, rhs.offset),
                O::Power => VI::TvBinaryArithmetic(dst, BA::Power, lhs.offset, rhs.offset),
                O::Multiply => VI::TvBinaryArithmetic(dst, BA::Multiply, lhs.offset, rhs.offset),
                O::DivideX => VI::TvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset),
                O::Divide0 => VI::TvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset),
                O::ModulusX => VI::TvBinaryArithmetic(dst, BA::Modulus, lhs.offset, rhs.offset),
                O::Modulus0 => VI::TvBinaryArithmetic(dst, BA::Modulus, lhs.offset, rhs.offset),
                O::Min => VI::TvBinaryArithmetic(dst, BA::Min, lhs.offset, rhs.offset),
                O::Max => VI::TvBinaryArithmetic(dst, BA::Max, lhs.offset, rhs.offset),

                O::UnsignedLessEqual => {
                    VI::TvBinaryComparison(dst.offset, BC::UnsignedLessEqual, lhs, rhs.offset)
                }
                O::CaseEquality if lhs_mode == M::TwoValue => {
                    VI::TvBinaryComparison(dst.offset, BC::CaseEquality, lhs, rhs.offset)
                }
                O::CaseEquality => {
                    VI::FvBinaryComparison(dst.offset, BC::CaseEquality, lhs, rhs.offset)
                }
                O::LogicalShiftLeft => VI::TvShift(dst, S::LogicalLeft, lhs.offset, rhs.offset),
                O::LogicalShiftRight => VI::TvShift(dst, S::LogicalRight, lhs.offset, rhs.offset),
                O::ArithmeticShiftRight => {
                    VI::TvShift(dst, S::ArithmeticRight, lhs.offset, rhs.offset)
                }
                O::Concat => VI::TvConcat(dst.offset, lhs, rhs),
                O::CopyX | O::CopyZ => VI::TvResize(dst, vogls_ir::ResizeOp::Truncate, lhs),
                O::Posedge if lhs_mode == M::TwoValue => {
                    VI::TvEdge(dst.offset, EdgeOp::Posedge, lhs.offset, rhs.offset)
                }
                O::Negedge if lhs_mode == M::TwoValue => {
                    VI::TvEdge(dst.offset, EdgeOp::Negedge, lhs.offset, rhs.offset)
                }
                O::Posedge => VI::FvEdge(dst.offset, EdgeOp::Posedge, lhs.offset, rhs.offset),
                O::Negedge => VI::FvEdge(dst.offset, EdgeOp::Negedge, lhs.offset, rhs.offset),
            }
        }
    };
    instrs.push(i);
}

pub fn lower_process_to_vm(
    process: ProcessKey,
    gl: &GlobalContext,
    heap_builder: &mut HeapBuilder,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
) -> VmProcess {
    use Instruction as I;
    use VmInstruction as VI;

    let process = &gl.processes[process];

    let mut bb_stack = Vec::new();
    let mut bb_seen = VgHashSet::<BasicBlockKey>::default();
    let mut bb_phis = VgHashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::default();

    let mut heap_map = VgHashMap::default();
    let mut bits_map = VgHashMap::default();

    insert_bb_phis(
        &process.regions,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        &mut bb_phis,
    );
    resolve_heap_map(
        &process.regions,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        heap_builder,
        &mut heap_map,
        Some(&mut bits_map),
    );

    bb_stack.clear();
    bb_seen.clear();
    let mut bb_offsets = HashMap::<BasicBlockKey, usize>::new();
    let mut bb_transitions = Vec::new();

    let mut instructions = Vec::new();

    macro_rules! signal {
        ($signal:expr) => {{ io_signals[&$signal] }};
    }
    // Lower the IR instructions to VM instructions.
    for tr in &process.regions {
        bb_stack.push(tr.entry());
        while let Some(bb_key) = bb_stack.pop() {
            let bb = gl.bbs.get(bb_key).unwrap();

            bb_offsets.insert(bb_key, instructions.len());

            for instr in &bb.instrs {
                let instr = match instr {
                    I::Constant(..) => continue,

                    I::Unary(d, op, s) => {
                        let sm = s.mode();
                        let size = gl.vars.size(*s);
                        let s = heap_map[s];
                        let d = heap_map[d];
                        lower_unary_op(&mut instructions, *op, d, s.to_ref(size), sm);
                        continue;
                    }
                    I::Resize(dst, op, src) => {
                        let d = heap_map[dst].to_ref(gl.vars.size(*dst));
                        let s = heap_map[src].to_ref(gl.vars.size(*src));
                        lower_resize_op(&mut instructions, *op, d, s, dst.mode());
                        continue;
                    }
                    I::BinaryImm(d, op, src, imm) => {
                        use LogicMode as M;

                        let d_size = gl.vars.size(*d);
                        let s1_size = gl.vars.size(*src);
                        let s2_size = imm.size();
                        let d_mode = d.mode();
                        let s1_mode = src.mode();
                        let imm_mode = if imm.contains_special() {
                            LogicMode::FourValue
                        } else {
                            LogicMode::TwoValue
                        };
                        let d = heap_map[d];
                        let (mtgt, _, conv_imm) =
                            bin_imm_args_need_conversion(*op, d_mode, s1_mode, imm_mode);
                        let imm_mode = if conv_imm { mtgt } else { imm_mode };
                        let src = heap_map[src];
                        let imm = bits_map[&(imm.clone(), imm_mode)];
                        use BinaryArithmeticOp as BA;
                        use BinaryComparisonOp as BC;
                        use BinaryImmOp as O;
                        if s1_mode == M::FourValue {
                            match *op {
                                O::And => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::And, src, imm)
                                }
                                O::Or => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Or, src, imm),
                                O::Xor => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Xor, src, imm)
                                }

                                O::Add => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Add, src, imm)
                                }
                                O::Sub => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Sub, src, imm)
                                }
                                O::Power => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Power, src, imm)
                                }
                                O::Multiply => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Multiply, src, imm)
                                }
                                O::Divide => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Divide, src, imm)
                                }
                                O::Modulus => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Modulus, src, imm)
                                }

                                O::RevSub => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Sub, imm, src)
                                }
                                O::RevPower => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Power, imm, src)
                                }
                                O::RevDivideX => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Divide, imm, src)
                                }
                                O::RevDivide0 => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Divide, imm, src)
                                }
                                O::RevModulusX => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Modulus, imm, src)
                                }
                                O::RevModulus0 => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Modulus, imm, src)
                                }

                                O::Min => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Min, src, imm)
                                }
                                O::Max => {
                                    VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Max, src, imm)
                                }

                                O::UnsignedLessEqual => VI::FvBinaryComparison(
                                    d,
                                    BC::UnsignedLessEqual,
                                    src.to_ref(s1_size),
                                    imm,
                                ),
                                O::UnsignedGreaterEqual => VI::FvBinaryComparison(
                                    d,
                                    BC::UnsignedLessEqual,
                                    imm.to_ref(s2_size),
                                    src,
                                ),

                                O::CaseEquality => VI::FvBinaryComparison(
                                    d,
                                    BC::CaseEquality,
                                    src.to_ref(s1_size),
                                    imm,
                                ),
                                O::ConcatLeft => {
                                    VI::FvConcat(d, imm.to_ref(s2_size), src.to_ref(s1_size))
                                }
                                O::ConcatRight => {
                                    VI::FvConcat(d, src.to_ref(s1_size), imm.to_ref(s2_size))
                                }
                                O::BitwiseCaseEquality => todo!(),
                            }
                        } else {
                            match *op {
                                O::And => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::And, src, imm)
                                }
                                O::Or => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Or, src, imm),
                                O::Xor => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Xor, src, imm)
                                }

                                O::Add => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Add, src, imm)
                                }
                                O::Sub => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Sub, src, imm)
                                }
                                O::Power => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Power, src, imm)
                                }
                                O::Multiply => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Multiply, src, imm)
                                }
                                O::Divide => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Divide, src, imm)
                                }
                                O::Modulus => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Modulus, src, imm)
                                }

                                O::RevSub => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Sub, imm, src)
                                }
                                O::RevPower => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Power, imm, src)
                                }
                                O::RevDivideX => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Divide, imm, src)
                                }
                                O::RevDivide0 => todo!(),
                                O::RevModulusX => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Modulus, imm, src)
                                }
                                O::RevModulus0 => todo!(),

                                O::Min => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Min, src, imm)
                                }
                                O::Max => {
                                    VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Max, src, imm)
                                }

                                O::UnsignedLessEqual => VI::TvBinaryComparison(
                                    d,
                                    BC::UnsignedLessEqual,
                                    src.to_ref(s1_size),
                                    imm,
                                ),
                                O::UnsignedGreaterEqual => VI::TvBinaryComparison(
                                    d,
                                    BC::UnsignedLessEqual,
                                    imm.to_ref(s2_size),
                                    src,
                                ),

                                O::CaseEquality => VI::TvBinaryComparison(
                                    d,
                                    BC::CaseEquality,
                                    src.to_ref(s1_size),
                                    imm,
                                ),
                                O::ConcatLeft => {
                                    VI::TvConcat(d, imm.to_ref(s2_size), src.to_ref(s1_size))
                                }
                                O::ConcatRight => {
                                    VI::TvConcat(d, src.to_ref(s1_size), imm.to_ref(s2_size))
                                }
                                O::BitwiseCaseEquality => todo!(),
                            }
                        }
                    }
                    I::Slice(d, s1, s2) => {
                        use LogicMode as M;

                        let d_size = gl.vars.size(*d);
                        let s1_size = gl.vars.size(*s1);
                        let s1_mode = s1.mode();
                        let s2_mode = s2.mode();
                        let d = heap_map[d];
                        let mode = match (s1_mode, s2_mode) {
                            (M::FourValue, _) | (_, M::FourValue) => M::FourValue,
                            _ => M::TwoValue,
                        };
                        let s1 = heap_map[s1];
                        let s2 = heap_map[s2];
                        if mode == M::FourValue {
                            VI::FvSlice(
                                d.to_ref(d_size),
                                s1.to_ref(s1_size),
                                s2,
                                SliceFlags {
                                    fill_with_x: true,
                                    offset_is_fv: s2_mode == LogicMode::FourValue,
                                },
                            )
                        } else {
                            VI::TvSlice(
                                d.to_ref(d_size),
                                s1.to_ref(s1_size),
                                s2,
                                SliceFlags {
                                    fill_with_x: true,
                                    offset_is_fv: s2_mode == LogicMode::FourValue,
                                },
                            )
                        }
                    }
                    I::SliceImm(d, src, offset) => {
                        use LogicMode as M;

                        let d_size = gl.vars.size(*d);
                        let s1_size = gl.vars.size(*src);
                        let s1_mode = src.mode();
                        let s2_mode = LogicMode::TwoValue;
                        let d = heap_map[d];
                        let mode = match (s1_mode, s2_mode) {
                            (M::FourValue, _) | (_, M::FourValue) => M::FourValue,
                            _ => M::TwoValue,
                        };
                        let src = heap_map[src];
                        lower_slice_imm(
                            &mut instructions,
                            d.to_ref(d_size),
                            src.to_ref(s1_size),
                            *offset,
                            mode,
                        );
                        continue;
                    }
                    I::ShiftImm(d, op, src, offset) => {
                        use LogicMode as M;

                        let d_size = gl.vars.size(*d);
                        let s1_mode = src.mode();
                        let s2_mode = LogicMode::TwoValue;
                        let d = heap_map[d];
                        let mode = match (s1_mode, s2_mode) {
                            (M::FourValue, _) | (_, M::FourValue) => M::FourValue,
                            _ => M::TwoValue,
                        };
                        let src = heap_map[src];
                        use ShiftImmOp as O;
                        use ShiftOp as S;
                        if mode == M::FourValue {
                            match op {
                                O::LogicalShiftLeft => {
                                    VI::FvShiftImm(d.to_ref(d_size), S::LogicalLeft, src, *offset)
                                }
                                O::LogicalShiftRight => {
                                    VI::FvShiftImm(d.to_ref(d_size), S::LogicalRight, src, *offset)
                                }
                                O::ArithmeticShiftRight => VI::FvShiftImm(
                                    d.to_ref(d_size),
                                    S::ArithmeticRight,
                                    src,
                                    *offset,
                                ),
                            }
                        } else {
                            match op {
                                O::LogicalShiftLeft => {
                                    VI::TvShiftImm(d.to_ref(d_size), S::LogicalLeft, src, *offset)
                                }
                                O::LogicalShiftRight => {
                                    VI::TvShiftImm(d.to_ref(d_size), S::LogicalRight, src, *offset)
                                }
                                O::ArithmeticShiftRight => VI::TvShiftImm(
                                    d.to_ref(d_size),
                                    S::ArithmeticRight,
                                    src,
                                    *offset,
                                ),
                            }
                        }
                    }
                    I::Select(dst, cond, truthy, falsy) => {
                        use LogicMode as M;

                        let dst_mode = dst.mode();
                        let size = gl.vars.size(*dst);
                        let cond_mode = cond.mode();
                        let d = heap_map[dst];
                        let c = heap_map[cond];
                        let t = heap_map[truthy];
                        let f = heap_map[falsy];

                        let cond_is_fv = cond_mode == M::FourValue;

                        if dst_mode == M::FourValue {
                            VI::FvSelect(d.to_ref(size), c, t, f, cond_is_fv)
                        } else {
                            if !cond_is_fv && size == SCALAR_VSIZE {
                                VI::TvSelect1(d, c, t, f)
                            } else {
                                VI::TvSelect(d.to_ref(size), c, t, f, cond_is_fv)
                            }
                        }
                    }
                    I::Binary(dst, op, lhs, rhs) => {
                        let d_size = gl.vars.size(*dst);
                        let s1_size = gl.vars.size(*lhs);
                        let s2_size = gl.vars.size(*rhs);
                        let d = heap_map[dst];
                        let s1 = heap_map[lhs];
                        let s2 = heap_map[rhs];
                        lower_bin_op(
                            &mut instructions,
                            *op,
                            d.to_ref(d_size),
                            s1.to_ref(s1_size),
                            s2.to_ref(s2_size),
                            dst.mode(),
                            lhs.mode(),
                            rhs.mode(),
                        );
                        continue;
                    }

                    I::Intrinsic(dst, op, args) => {
                        let vm_args = args
                            .iter()
                            .map(|v| (heap_map[v].to_ref(gl.vars.size(*v)), v.mode()))
                            .collect();
                        use IntrinsicOp as O;
                        use VmIntrinsicOp as VO;
                        let op = match op.as_ref() {
                            O::Time => VO::Time,
                            O::Finish => VO::Finish,
                            O::Random => VO::Random,
                            O::Display(f) => VO::Display(f.clone()),
                            O::Assert(f) => VO::Assert(f.clone()),
                            O::VcdOpenFile(f) => VO::VcdOpenFile(f.clone()),
                            O::VcdAppendModule(v) => {
                                let (children, map) = vogls_vcd::VcdScope::lower(v, io_signals);
                                VO::VcdAppendModule(children, map)
                            }
                            O::VcdPause => VO::VcdPause,
                            O::VcdResume => VO::VcdResume,
                            O::ReadMem(readmem) => {
                                VO::ReadMem(signal!(readmem.signal), readmem.clone())
                            }
                        };
                        VI::Intrinsic(heap_map[dst], Box::new(op), vm_args)
                    }
                    I::LastUpdateTime(dst, signal) => {
                        let signal = signal!(*signal);
                        VI::LastUpdateTime(heap_map[dst], signal)
                    }
                    I::Probe(dst, signal, offset) => {
                        let size = gl.vars.size(*dst);
                        let mode = gl.signals[*signal].mode;
                        let signal = signal!(*signal);
                        lower_slice_imm(
                            &mut instructions,
                            heap_map[dst].to_ref(size),
                            signals[signal.as_usize()],
                            *offset,
                            mode,
                        );
                        continue;
                    }
                    I::ProbeSlice(dst, signal, offset) => {
                        let size = gl.vars.size(*dst);
                        let mode = gl.signals[*signal].mode;
                        let signal = signal!(*signal);
                        match mode {
                            LogicMode::TwoValue => VI::TvSlice(
                                heap_map[dst].to_ref(size),
                                signals[signal.as_usize()],
                                heap_map[offset],
                                SliceFlags {
                                    fill_with_x: true,
                                    offset_is_fv: offset.mode() == LogicMode::FourValue,
                                },
                            ),
                            LogicMode::FourValue => VI::FvSlice(
                                heap_map[dst].to_ref(size),
                                signals[signal.as_usize()],
                                heap_map[offset],
                                SliceFlags {
                                    fill_with_x: true,
                                    offset_is_fv: offset.mode() == LogicMode::FourValue,
                                },
                            ),
                        }
                    }
                    I::Drive(signal, src, offset) => {
                        let src_size = gl.vars.size(*src);
                        VI::DriveCO(signal!(*signal), heap_map[src].to_ref(src_size), *offset)
                    }
                    I::DriveSlice(signal, src, offset) => {
                        let src_size = gl.vars.size(*src);
                        VI::Drive(
                            signal!(*signal),
                            heap_map[src].to_ref(src_size),
                            heap_map[offset],
                            offset.mode() == LogicMode::FourValue,
                        )
                    }
                    I::Phi(..) => continue,
                };

                instructions.push(instr);
            }

            if let Some(phis) = bb_phis.get(&bb_key) {
                for (dst, src) in phis {
                    let src_size = gl.vars.size(*src);
                    let dst_size = gl.vars.size(*dst);
                    assert_eq!(src_size, dst_size);
                    let size = src_size;
                    let src_mode = src.mode();
                    let dst_mode = dst.mode();
                    let (dst, src) = (heap_map[dst], heap_map[src]);
                    lower_convert_op(&mut instructions, dst, src, size, dst_mode, src_mode);
                }
            }

            use BasicBlockTerminator as T;
            let terminator_instr = match &bb.terminator {
                T::Wait(_, time) => {
                    instructions.push(VI::Wait(*time));
                    VI::Jump(0)
                }
                T::VariableWait(_, var) if var.mode() == LogicMode::TwoValue => {
                    instructions.push(VI::TvVariableWait(heap_map[var]));
                    VI::Jump(0)
                }
                T::VariableWait(_, var) => {
                    instructions.push(VI::FvVariableWait(heap_map[var]));
                    VI::Jump(0)
                }
                T::WaitRegion(_, region) => {
                    instructions.push(VI::WaitRegion(*region));
                    VI::Jump(0)
                }
                T::Watch(_, signals) => {
                    instructions.push(VI::Watch(signals.iter().map(|s| signal!(*s)).collect()));
                    VI::Jump(0)
                }
                T::Jump(_) => VI::Jump(0),
                T::Branch(cond, _, _) if cond.mode() == LogicMode::TwoValue => {
                    VI::TvBranch(heap_map[cond], 0, 0)
                }
                T::Branch(cond, _, _) => VI::FvBranch(heap_map[cond], 0, 0),
                T::Halt => VI::Halt,
            };

            bb_transitions.push((instructions.len(), bb_key));
            instructions.push(terminator_instr);

            bb_seen.insert(bb_key);
            bb.terminator.for_each_non_temporal_bb(|bb| {
                if bb_seen.insert(bb) {
                    bb_stack.push(bb);
                }
            });
        }
    }

    // Correct the offsets of the transitions between basic blocks.
    let bb_to_offset = |bb_key: BasicBlockKey| *bb_offsets.get(&bb_key).unwrap();
    for (offset, bb_key) in bb_transitions {
        let bb = gl.bbs.get(bb_key).unwrap();

        use BasicBlockTerminator as T;
        use VmInstruction as VI;
        match (&bb.terminator, &mut instructions[offset]) {
            (T::Wait(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(bb.entry()),
            (T::VariableWait(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(bb.entry()),
            (T::WaitRegion(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(bb.entry()),
            (T::Watch(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(bb.entry()),
            (T::Jump(bb), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
            (
                T::Branch(_, true_bb, false_bb),
                VI::TvBranch(_, true_offset, false_offset)
                | VI::FvBranch(_, true_offset, false_offset),
            ) => {
                *true_offset = bb_to_offset(*true_bb);
                *false_offset = bb_to_offset(*false_bb);
            }
            (T::Halt, VI::Halt) => {}
            _ => unreachable!("invalid terminator combination"),
        }
    }

    VmProcess { instructions }
}
