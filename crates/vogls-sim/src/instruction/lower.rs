use std::collections::HashMap;

use vogls_codegen::{
    HeapBuilder, HeapOffset, HeapRef, insert_bb_phis, resolve_heap_map, resolve_var_logic_mode_map,
};
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, GlobalContext, INTEGER_VSIZE,
    Instruction, IntrinsicOp, LogicMode, ProcessKey, ResizeOp, SCALAR_VSIZE, ShiftImmOp, SignalKey,
    UnaryOp, VariableKey, VectorSize,
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
    mode: LogicMode,
) {
    use UnaryOp as O;
    use VmInstruction as VI;
    let i = if mode == LogicMode::FourValue {
        VI::FvUnary(dst, op, src)
    } else {
        if src.size == SCALAR_VSIZE {
            match op {
                O::Neg => VI::TvNot1(dst, src.offset),
                O::ReduceOr | O::ReduceAnd | O::ReduceXor => VI::TvMove1(dst, src.offset),
                O::LeadingZeros => todo!(),
            }
        } else {
            VI::TvUnary(dst, op, src)
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
    mode: LogicMode,
) {
    use BinaryArithmeticOp as BA;
    use BinaryComparisonOp as BC;
    use BinaryOp as O;
    use LogicMode as M;
    use ShiftOp as S;
    use VmInstruction as VI;
    let i = if mode == M::FourValue {
        match op {
            O::And => VI::FvBinaryArithmetic(dst, BA::And, lhs.offset, rhs.offset),
            O::Or => VI::FvBinaryArithmetic(dst, BA::Or, lhs.offset, rhs.offset),
            O::Xor => VI::FvBinaryArithmetic(dst, BA::Xor, lhs.offset, rhs.offset),
            O::Add => VI::FvBinaryArithmetic(dst, BA::Add, lhs.offset, rhs.offset),
            O::Sub => VI::FvBinaryArithmetic(dst, BA::Sub, lhs.offset, rhs.offset),
            O::Power => VI::FvBinaryArithmetic(dst, BA::Power, lhs.offset, rhs.offset),
            O::Multiply => VI::FvBinaryArithmetic(dst, BA::Multiply, lhs.offset, rhs.offset),
            O::Divide => VI::FvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset),
            O::Modulus => VI::FvBinaryArithmetic(dst, BA::Modulus, lhs.offset, rhs.offset),
            O::Min => VI::FvBinaryArithmetic(dst, BA::Min, lhs.offset, rhs.offset),
            O::Max => VI::FvBinaryArithmetic(dst, BA::Max, lhs.offset, rhs.offset),

            O::UnsignedLessEqual => {
                VI::FvBinaryComparison(dst.offset, BC::UnsignedLessEqual, lhs, rhs.offset)
            }
            O::CaseEquality => {
                VI::FvBinaryComparison(dst.offset, BC::CaseEquality, lhs, rhs.offset)
            }
            O::LogicalShiftLeft => VI::FvShift(dst, S::LogicalLeft, lhs.offset, rhs.offset),
            O::LogicalShiftRight => VI::FvShift(dst, S::LogicalRight, lhs.offset, rhs.offset),
            O::ArithmeticShiftRight => VI::FvShift(dst, S::ArithmeticRight, lhs.offset, rhs.offset),
            O::Concat => VI::FvConcat(dst.offset, lhs, rhs),

            O::CopyX => VI::FvBinaryArithmetic(dst, BA::CopyX, lhs.offset, rhs.offset),
            O::CopyZ => VI::FvBinaryArithmetic(dst, BA::CopyZ, lhs.offset, rhs.offset),
            O::Posedge => VI::FvEdge(dst.offset, EdgeOp::Posedge, lhs.offset, rhs.offset),
            O::Negedge => VI::FvEdge(dst.offset, EdgeOp::Negedge, lhs.offset, rhs.offset),
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
                O::Divide => VI::TvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset),
                O::Modulus => VI::TvBinaryArithmetic(dst, BA::Modulus, lhs.offset, rhs.offset),
                O::Min => VI::TvAnd1(dst.offset, lhs.offset, rhs.offset),
                O::Max => VI::TvOr1(dst.offset, lhs.offset, rhs.offset),
                O::UnsignedLessEqual => VI::TvOrNot1(dst.offset, rhs.offset, lhs.offset),
                O::CaseEquality => VI::TvXnor1(dst.offset, lhs.offset, rhs.offset),
                O::LogicalShiftLeft => VI::TvShift(dst, S::LogicalLeft, lhs.offset, rhs.offset),
                O::LogicalShiftRight => VI::TvShift(dst, S::LogicalRight, lhs.offset, rhs.offset),
                O::ArithmeticShiftRight => {
                    VI::TvShift(dst, S::ArithmeticRight, lhs.offset, rhs.offset)
                }
                O::Concat => VI::TvConcat(dst.offset, lhs, rhs),
                O::CopyX | O::CopyZ => VI::TvMove1(dst.offset, lhs.offset),
                O::Posedge => VI::TvAndNot1(dst.offset, rhs.offset, lhs.offset),
                O::Negedge => VI::TvAndNot1(dst.offset, lhs.offset, rhs.offset),
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
                O::Divide => VI::TvBinaryArithmetic(dst, BA::Divide, lhs.offset, rhs.offset),
                O::Modulus => VI::TvBinaryArithmetic(dst, BA::Modulus, lhs.offset, rhs.offset),
                O::Min => VI::TvBinaryArithmetic(dst, BA::Min, lhs.offset, rhs.offset),
                O::Max => VI::TvBinaryArithmetic(dst, BA::Max, lhs.offset, rhs.offset),

                O::UnsignedLessEqual => {
                    VI::TvBinaryComparison(dst.offset, BC::UnsignedLessEqual, lhs, rhs.offset)
                }
                O::CaseEquality => {
                    VI::TvBinaryComparison(dst.offset, BC::CaseEquality, lhs, rhs.offset)
                }
                O::LogicalShiftLeft => VI::TvShift(dst, S::LogicalLeft, lhs.offset, rhs.offset),
                O::LogicalShiftRight => VI::TvShift(dst, S::LogicalRight, lhs.offset, rhs.offset),
                O::ArithmeticShiftRight => {
                    VI::TvShift(dst, S::ArithmeticRight, lhs.offset, rhs.offset)
                }
                O::Concat => VI::TvConcat(dst.offset, lhs, rhs),
                O::CopyX | O::CopyZ => VI::TvResize(dst, vogls_ir::ResizeOp::Truncate, lhs),
                O::Posedge => VI::TvEdge(dst.offset, EdgeOp::Posedge, lhs.offset, rhs.offset),
                O::Negedge => VI::TvEdge(dst.offset, EdgeOp::Negedge, lhs.offset, rhs.offset),
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

    let mut var_mode = VgHashMap::<VariableKey, LogicMode>::default();
    let mut conv_map = VgHashMap::<VariableKey, HeapOffset>::default();
    let mut heap_map = VgHashMap::default();
    let mut bits_map = VgHashMap::default();

    resolve_var_logic_mode_map(
        process.entry,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        &mut var_mode,
        &mut conv_map,
    );
    insert_bb_phis(process.entry, gl, &mut bb_stack, &mut bb_seen, &mut bb_phis);
    resolve_heap_map(
        process.entry,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        &var_mode,
        &mut conv_map,
        heap_builder,
        &mut heap_map,
        None,
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
    macro_rules! var {
        ($var:expr$(, ($tgt_mode:expr, $src_mode:expr, $size:expr))?) => {{
            let r = heap_map[&$var];
            $(
            let r = match ($tgt_mode, $src_mode) {
                (LogicMode::TwoValue, LogicMode::TwoValue) | (LogicMode::FourValue, LogicMode::FourValue) => r,
                (LogicMode::TwoValue, LogicMode::FourValue) => {
                    let tgt = conv_map[&$var].to_ref($size);
                    instructions.push(VmInstruction::FvToTv(tgt, r));
                    tgt.offset
                },
                (LogicMode::FourValue, LogicMode::TwoValue) => {
                    let tgt = conv_map[&$var].to_ref($size);
                    instructions.push(VmInstruction::TvToFv(tgt, r));
                    tgt.offset
                },
            };
            )?
            r
        }};
    }

    // Lower the IR instructions to VM instructions.
    bb_stack.push(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get(bb_key).unwrap();

        bb_offsets.insert(bb_key, instructions.len());

        for instr in &bb.instrs {
            let instr = match instr {
                I::Constant(..) => continue,

                I::Unary(d, op, s) => {
                    let size = gl.vars[*s].size;
                    let m = var_mode[d];
                    let s = var!(*s, (m, var_mode[s], size));
                    let d = var!(*d);
                    lower_unary_op(&mut instructions, *op, d, s.to_ref(size), m);
                    continue;
                }
                I::Resize(d, op, s) => {
                    let d_size = gl.vars[*d].size;
                    let s_size = gl.vars[*s].size;
                    let m = var_mode[d];
                    let s = var!(*s, (m, var_mode[s], s_size)).to_ref(s_size);
                    let d = var!(*d).to_ref(d_size);
                    lower_resize_op(&mut instructions, *op, d, s, m);
                    continue;
                }
                I::BinaryImm(d, op, src, imm) => {
                    use LogicMode as M;

                    let d_size = gl.vars[*d].size;
                    let s1_size = gl.vars[*src].size;
                    let s2_size = imm.size();
                    let s1_mode = var_mode[src];
                    let s2_mode = if imm.contains_special() {
                        LogicMode::FourValue
                    } else {
                        LogicMode::TwoValue
                    };
                    let d = var!(*d);
                    let mode = match (s1_mode, s2_mode) {
                        (M::FourValue, _) | (_, M::FourValue) => M::FourValue,
                        _ => M::TwoValue,
                    };
                    let src = var!(*src, (mode, s1_mode, s1_size));
                    let imm = bits_map[&(imm.clone(), mode)];
                    use BinaryArithmeticOp as BA;
                    use BinaryComparisonOp as BC;
                    use BinaryImmOp as O;
                    if mode == M::FourValue {
                        match *op {
                            O::And => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::And, src, imm),
                            O::Or => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Or, src, imm),
                            O::Xor => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Xor, src, imm),

                            O::Add => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Add, src, imm),
                            O::Sub => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Sub, src, imm),
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
                            O::RevDivide => {
                                VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Divide, imm, src)
                            }
                            O::RevModulus => {
                                VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Modulus, imm, src)
                            }

                            O::Min => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Min, src, imm),
                            O::Max => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Max, src, imm),

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
                        }
                    } else {
                        match *op {
                            O::And => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::And, src, imm),
                            O::Or => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Or, src, imm),
                            O::Xor => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Xor, src, imm),

                            O::Add => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Add, src, imm),
                            O::Sub => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Sub, src, imm),
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
                            O::RevDivide => {
                                VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Divide, imm, src)
                            }
                            O::RevModulus => {
                                VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Modulus, imm, src)
                            }

                            O::Min => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Min, src, imm),
                            O::Max => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Max, src, imm),

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
                        }
                    }
                }
                I::Slice(d, s1, s2) => {
                    use LogicMode as M;

                    let d_size = gl.vars[*d].size;
                    let s1_size = gl.vars[*s1].size;
                    let s1_mode = var_mode[s1];
                    let s2_mode = var_mode[s2];
                    let d = var!(*d);
                    let mode = match (s1_mode, s2_mode) {
                        (M::FourValue, _) | (_, M::FourValue) => M::FourValue,
                        _ => M::TwoValue,
                    };
                    let s1 = var!(*s1, (mode, s1_mode, s1_size));
                    let s2 = var!(*s2);
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

                    let d_size = gl.vars[*d].size;
                    let s1_size = gl.vars[*src].size;
                    let s1_mode = var_mode[src];
                    let s2_mode = LogicMode::TwoValue;
                    let d = var!(*d);
                    let mode = match (s1_mode, s2_mode) {
                        (M::FourValue, _) | (_, M::FourValue) => M::FourValue,
                        _ => M::TwoValue,
                    };
                    let src = var!(*src, (mode, s1_mode, s1_size));
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

                    let d_size = gl.vars[*d].size;
                    let s1_size = gl.vars[*src].size;
                    let s1_mode = var_mode[src];
                    let s2_mode = LogicMode::TwoValue;
                    let d = var!(*d);
                    let mode = match (s1_mode, s2_mode) {
                        (M::FourValue, _) | (_, M::FourValue) => M::FourValue,
                        _ => M::TwoValue,
                    };
                    let src = var!(*src, (mode, s1_mode, s1_size));
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
                            O::ArithmeticShiftRight => {
                                VI::FvShiftImm(d.to_ref(d_size), S::ArithmeticRight, src, *offset)
                            }
                        }
                    } else {
                        match op {
                            O::LogicalShiftLeft => {
                                VI::TvShiftImm(d.to_ref(d_size), S::LogicalLeft, src, *offset)
                            }
                            O::LogicalShiftRight => {
                                VI::TvShiftImm(d.to_ref(d_size), S::LogicalRight, src, *offset)
                            }
                            O::ArithmeticShiftRight => {
                                VI::TvShiftImm(d.to_ref(d_size), S::ArithmeticRight, src, *offset)
                            }
                        }
                    }
                }
                I::Select(dst, cond, truthy, falsy) => {
                    use LogicMode as M;

                    let dst_mode = var_mode[dst];
                    let size = gl.vars[*dst].size;
                    let cond_mode = var_mode[cond];
                    let truthy_mode = var_mode[truthy];
                    let falsy_mode = var_mode[falsy];
                    let d = var!(*dst);
                    let c = var!(*cond);
                    let t = var!(*truthy, (dst_mode, truthy_mode, size));
                    let f = var!(*falsy, (dst_mode, falsy_mode, size));

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
                I::Binary(d, op, s1, s2) => {
                    use LogicMode as M;

                    let d_size = gl.vars[*d].size;
                    let s1_size = gl.vars[*s1].size;
                    let s2_size = gl.vars[*s2].size;
                    let s1_mode = var_mode[s1];
                    let s2_mode = var_mode[s2];
                    let d = var!(*d);
                    let mode = match (s1_mode, s2_mode) {
                        (M::FourValue, _) | (_, M::FourValue) => M::FourValue,
                        _ => M::TwoValue,
                    };
                    let s1 = var!(*s1, (mode, s1_mode, s1_size));
                    let s2 = var!(*s2, (mode, s2_mode, s2_size));
                    lower_bin_op(
                        &mut instructions,
                        *op,
                        d.to_ref(d_size),
                        s1.to_ref(s1_size),
                        s2.to_ref(s2_size),
                        mode,
                    );
                    continue;
                }

                I::Intrinsic(dst, op, args) => {
                    let vm_args = args
                        .iter()
                        .map(|v| (var!(*v).to_ref(gl.vars[*v].size), var_mode[v]))
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
                        O::ReadMem(readmem) => VO::ReadMem(
                            signals[signal!(readmem.signal).as_usize()],
                            readmem.clone(),
                        ),
                    };
                    VI::Intrinsic(var!(*dst), Box::new(op), vm_args)
                }
                I::LastUpdateTime(dst, signal) => {
                    let signal = signal!(*signal);
                    VI::LastUpdateTime(var!(*dst), signal)
                }
                I::Probe(dst, signal, offset) => {
                    let size = gl.vars[*dst].size;
                    let signal = signal!(*signal);
                    lower_slice_imm(
                        &mut instructions,
                        var!(*dst).to_ref(size),
                        signals[signal.as_usize()],
                        *offset,
                        gl.logic_mode,
                    );
                    continue;
                }
                I::ProbeSlice(dst, signal, offset) => {
                    let size = gl.vars[*dst].size;
                    let signal = signal!(*signal);
                    match gl.logic_mode {
                        LogicMode::TwoValue => VI::TvSlice(
                            var!(*dst).to_ref(size),
                            signals[signal.as_usize()],
                            var!(*offset),
                            SliceFlags {
                                fill_with_x: true,
                                offset_is_fv: var_mode[offset] == LogicMode::FourValue,
                            },
                        ),
                        LogicMode::FourValue => VI::FvSlice(
                            var!(*dst).to_ref(size),
                            signals[signal.as_usize()],
                            var!(*offset),
                            SliceFlags {
                                fill_with_x: true,
                                offset_is_fv: var_mode[offset] == LogicMode::FourValue,
                            },
                        ),
                    }
                }
                I::Drive(signal, src, offset) => {
                    let src_size = gl.vars[*src].size;
                    VI::Drive(
                        signal!(*signal),
                        var!(*src, (gl.logic_mode, var_mode[src], src_size)).to_ref(src_size),
                        offset.map(|(o, mask_size)| {
                            (
                                var!(o, (gl.logic_mode, var_mode[&o], INTEGER_VSIZE)),
                                mask_size,
                            )
                        }),
                    )
                }
                I::Phi(..) => continue,
            };

            instructions.push(instr);
        }

        if let Some(phis) = bb_phis.get(&bb_key) {
            for (dst, src) in phis {
                let src_size = gl.vars[*src].size;
                let dst_size = gl.vars[*dst].size;
                assert_eq!(src_size, dst_size);
                let size = src_size;
                let src_mode = var_mode[src];
                let dst_mode = var_mode[dst];
                let (dst, src) = (var!(*dst), var!(*src));
                lower_convert_op(&mut instructions, dst, src, size, dst_mode, src_mode);
            }
        }

        use BasicBlockTerminator as T;
        let terminator_instr = match &bb.terminator {
            T::Wait(_, time) => {
                instructions.push(VI::Wait(*time));
                VI::Jump(0)
            }
            T::VariableWait(_, var) if var_mode[var] == LogicMode::TwoValue => {
                instructions.push(VI::TvVariableWait(var!(*var)));
                VI::Jump(0)
            }
            T::VariableWait(_, var) => {
                instructions.push(VI::FvVariableWait(var!(*var)));
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
            T::Branch(cond, _, _) if var_mode[cond] == LogicMode::TwoValue => {
                VI::TvBranch(var!(*cond), 0, 0)
            }
            T::Branch(cond, _, _) => VI::FvBranch(var!(*cond), 0, 0),
            T::Halt => VI::Halt,
        };

        bb_transitions.push((instructions.len(), bb_key));
        instructions.push(terminator_instr);

        bb_seen.insert(bb_key);
        bb.terminator.for_each_bb(|bb| {
            if bb_seen.insert(bb) {
                bb_stack.push(bb);
            }
        });
    }

    // Correct the offsets of the transitions between basic blocks.
    let bb_to_offset = |bb_key: BasicBlockKey| *bb_offsets.get(&bb_key).unwrap();
    for (offset, bb_key) in bb_transitions {
        let bb = gl.bbs.get(bb_key).unwrap();

        use BasicBlockTerminator as T;
        use VmInstruction as VI;
        match (&bb.terminator, &mut instructions[offset]) {
            (T::Wait(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
            (T::VariableWait(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
            (T::WaitRegion(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
            (T::Watch(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
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
