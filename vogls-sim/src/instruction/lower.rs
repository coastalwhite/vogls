use std::collections::{HashMap, HashSet};

use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, INTEGER_VSIZE, Instruction,
    IntrinsicOp, LogicMode, ProcessKey, SignalKey, VariableKey, VectorSize,
};

use crate::instruction::{HeapOffset, VmInstruction, VmProcess};
use crate::{BinaryArithmeticOp, BinaryComparisonOp, Heap, ShiftOp, VmIntrinsicOp};

use super::{HeapRef, VmSignalKey};

pub struct HeapBuilder {
    top: usize,
}

impl HeapBuilder {
    pub fn new() -> Self {
        Self { top: 0 }
    }

    pub fn claim(&mut self, mode: LogicMode, size: VectorSize) -> HeapRef {
        // Most arithmetic operations are implemented on 64-bit chunks, by ensuring that
        // all variables with a size larger or equal to than 64 are allocated in chunks of
        // 64 we can efficiently dispatch to these kernels.
        let nbytes = if mode == LogicMode::TwoValue && size.get() > 32 {
            // @TODO: Allow this space to be used somehow. In general, we should use a
            // slab allocator instead of this.
            self.top += 8 - (self.top % 8); // pad to 8-bytes
            (size.get() as usize).div_ceil(64) * 8
        } else if mode == LogicMode::FourValue && size.get() > 16 {
            self.top += 8 - (self.top % 8); // pad to 8-bytes
            2 * (size.get() as usize).div_ceil(64) * 8
        } else if mode == LogicMode::TwoValue {
            (size.get() as usize).div_ceil(8)
        } else {
            (2 * size.get() as usize).div_ceil(8)
        };
        let heap_ref = HeapOffset(self.top);
        self.top += nbytes;
        heap_ref.to_ref(size)
    }

    pub fn finish(self) -> Heap {
        Heap(vec![0u64; self.top.div_ceil(8)].into())
    }
}

pub fn lower_process_to_vm(
    process: ProcessKey,
    gl: &GlobalContext,
    builder: &mut HeapBuilder,
    signals: &[HeapRef],
    io_signals: &HashMap<SignalKey, VmSignalKey>,
    signal_map: &HashMap<SignalKey, SignalKey>,
) -> VmProcess {
    use Instruction as I;
    use VmInstruction as VI;

    let process = &gl.processes[process];

    let mut bb_stack = Vec::new();
    let mut bb_seen = HashSet::new();
    let mut bb_phis = HashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::new();

    let mut var_mode = HashMap::<VariableKey, LogicMode>::new();
    let mut heap_map = HashMap::new();

    // Fill `var_mode` with the `LogicMode` for each variable.
    //
    // @Performance
    loop {
        bb_stack.push(process.entry);
        bb_seen.insert(process.entry);
        let mut has_unresolved_variables = false;

        while let Some(bb_key) = bb_stack.pop() {
            let bb = &gl.bbs[bb_key];
            bb.terminator.extend_next_rev(&mut bb_stack, &mut bb_seen);

            use Instruction as I;
            for instr in &bb.instrs {
                match instr {
                    I::Constant(dst, bits) => {
                        var_mode.insert(
                            *dst,
                            if bits.contains_special() {
                                LogicMode::FourValue
                            } else {
                                LogicMode::TwoValue
                            },
                        );
                    }
                    I::Unary(dst, _, src) | I::Resize(dst, _, src) => {
                        if let Some(m) = var_mode.get(&src).copied() {
                            var_mode.insert(*dst, m);
                        }
                    }
                    I::Binary(dst, _, lhs, rhs) => {
                        if let (Some(m1), Some(m2)) =
                            (var_mode.get(&lhs).copied(), var_mode.get(&rhs).copied())
                        {
                            let m = if m1 == LogicMode::FourValue || m2 == LogicMode::FourValue {
                                LogicMode::FourValue
                            } else {
                                LogicMode::TwoValue
                            };
                            var_mode.insert(*dst, m);
                        }
                    }
                    I::Intrinsic(dst, _, _) => _ = var_mode.insert(*dst, LogicMode::TwoValue),
                    I::LastUpdateTime(dst, _) => _ = var_mode.insert(*dst, LogicMode::TwoValue),
                    I::Probe(dst, _) => {
                        var_mode.insert(*dst, gl.logic_mode);
                    }
                    I::Drive(_, _, _) => {}
                    I::Phi(dst, items) => {
                        for (_, v) in items {
                            if let Some(m) = var_mode.get(v).copied() {
                                if m == LogicMode::FourValue {
                                    var_mode.insert(*dst, LogicMode::FourValue);
                                    continue;
                                }
                            } else {
                                has_unresolved_variables = true;
                                continue;
                            }
                        }
                        var_mode.insert(*dst, LogicMode::TwoValue);
                    }
                }
            }
        }

        bb_seen.clear();
        if !has_unresolved_variables {
            break;
        }
    }

    // Make a map of the heap.
    bb_stack.push(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get(bb_key).unwrap();

        for instr in &bb.instrs {
            if let Instruction::Phi(dst, srcs) = instr {
                for (bb, var) in srcs {
                    bb_phis.entry(*bb).or_insert(Vec::new()).push((*dst, *var));
                }
            }

            if let Some(dst) = instr.get_destination_variable() {
                let size = gl.vars.get(dst).unwrap().size;
                let mode = var_mode[&dst];

                heap_map.insert(dst, builder.claim(mode, size).offset);
            }
        }

        bb_seen.insert(bb_key);
        bb.terminator.extend_next_rev(&mut bb_stack, &mut bb_seen);
    }

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
                    let tgt = builder.claim($tgt_mode, $size);
                    instructions.push(VmInstruction::FvToTv(tgt, r));
                    tgt.offset
                },
                (LogicMode::FourValue, LogicMode::TwoValue) => {
                    let tgt = builder.claim($tgt_mode, $size);
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
                I::Constant(d, value) => VI::Constant(var!(*d), value.clone()),

                I::Unary(d, op, s) => {
                    let size = gl.vars[*s].size;
                    let s = var!(*s, (var_mode[d], var_mode[s], size));
                    if var_mode[d] == LogicMode::FourValue {
                        VI::FvUnary(var!(*d), *op, s.to_ref(size))
                    } else {
                        VI::TvUnary(var!(*d), *op, s.to_ref(size))
                    }
                }
                I::Resize(d, op, s) => {
                    let d_size = gl.vars[*d].size;
                    let s_size = gl.vars[*s].size;
                    let s = var!(*s, (var_mode[d], var_mode[s], s_size));
                    if var_mode[d] == LogicMode::FourValue {
                        VI::FvResize(var!(*d).to_ref(d_size), *op, s.to_ref(s_size))
                    } else {
                        VI::TvResize(var!(*d).to_ref(d_size), *op, s.to_ref(s_size))
                    }
                }
                I::Binary(d, op, s1, s2) => {
                    let d_size = gl.vars[*d].size;
                    let s1_size = gl.vars[*s1].size;
                    let s2_size = gl.vars[*s2].size;
                    let d_mode = var_mode[d];
                    let s1_mode = var_mode[s1];
                    let s2_mode = var_mode[s2];
                    let d = var!(*d);
                    let s1 = var!(*s1, (d_mode, s1_mode, s1_size));
                    let s2 = var!(*s2, (d_mode, s2_mode, s2_size));
                    use BinaryArithmeticOp as BA;
                    use BinaryComparisonOp as BC;
                    use BinaryOp as O;
                    use ShiftOp as S;
                    if d_mode == LogicMode::FourValue {
                        match *op {
                            O::And => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::And, s1, s2),
                            O::Or => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Or, s1, s2),
                            O::Xor => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Xor, s1, s2),
                            O::Add => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Add, s1, s2),
                            O::Sub => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Sub, s1, s2),
                            O::Power => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Power, s1, s2),
                            O::Multiply => {
                                VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Multiply, s1, s2)
                            }
                            O::Divide => {
                                VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Divide, s1, s2)
                            }
                            O::Modulus => {
                                VI::FvBinaryArithmetic(d.to_ref(d_size), BA::Modulus, s1, s2)
                            }

                            O::UnsignedLessEqual => VI::FvBinaryComparison(
                                d,
                                BC::UnsignedLessEqual,
                                s1.to_ref(s1_size),
                                s2,
                            ),
                            O::CaseEquality => {
                                VI::FvBinaryComparison(d, BC::CaseEquality, s1.to_ref(s1_size), s2)
                            }
                            O::SelectBit => VI::FvSelectBit(d, s1.to_ref(s1_size), s2),
                            O::LogicalShiftLeft => {
                                VI::FvShift(d.to_ref(d_size), S::LogicalLeft, s1, s2)
                            }
                            O::LogicalShiftRight => {
                                VI::FvShift(d.to_ref(d_size), S::LogicalRight, s1, s2)
                            }
                            O::ArithmeticShiftRight => {
                                VI::FvShift(d.to_ref(d_size), S::ArithmeticRight, s1, s2)
                            }
                            O::Concat => VI::FvConcat(d, s1.to_ref(s1_size), s2.to_ref(s2_size)),

                            O::CopyX => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::CopyX, s1, s2),
                            O::CopyZ => VI::FvBinaryArithmetic(d.to_ref(d_size), BA::CopyZ, s1, s2),
                        }
                    } else {
                        match *op {
                            O::And => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::And, s1, s2),
                            O::Or => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Or, s1, s2),
                            O::Xor => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Xor, s1, s2),
                            O::Add => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Add, s1, s2),
                            O::Sub => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Sub, s1, s2),
                            O::Power => VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Power, s1, s2),
                            O::Multiply => {
                                VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Multiply, s1, s2)
                            }
                            O::Divide => {
                                VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Divide, s1, s2)
                            }
                            O::Modulus => {
                                VI::TvBinaryArithmetic(d.to_ref(d_size), BA::Modulus, s1, s2)
                            }

                            O::UnsignedLessEqual => VI::TvBinaryComparison(
                                d,
                                BC::UnsignedLessEqual,
                                s1.to_ref(s1_size),
                                s2,
                            ),
                            O::CaseEquality => {
                                VI::TvBinaryComparison(d, BC::CaseEquality, s1.to_ref(s1_size), s2)
                            }
                            O::SelectBit => VI::TvSelectBit(d, s1.to_ref(s1_size), s2),
                            O::LogicalShiftLeft => {
                                VI::TvShift(d.to_ref(d_size), S::LogicalLeft, s1, s2)
                            }
                            O::LogicalShiftRight => {
                                VI::TvShift(d.to_ref(d_size), S::LogicalRight, s1, s2)
                            }
                            O::ArithmeticShiftRight => {
                                VI::TvShift(d.to_ref(d_size), S::ArithmeticRight, s1, s2)
                            }
                            O::Concat => VI::TvConcat(d, s1.to_ref(s1_size), s2.to_ref(s2_size)),
                            O::CopyX | O::CopyZ => VI::TvResize(
                                d.to_ref(d_size),
                                vogls_ir::ResizeOp::Truncate,
                                s1.to_ref(s1_size),
                            ),
                        }
                    }
                }

                I::Intrinsic(dst, op, args) => {
                    let vm_args = args
                        .iter()
                        .map(|v| (var!(*v).to_ref(gl.vars[*v].size), var_mode[v]))
                        .collect();
                    use crate::VcdScope as VmVcdScope;
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
                            VO::VcdAppendModule(VmVcdScope::lower(v, io_signals, signal_map))
                        }
                        O::VcdPause => VO::VcdPause,
                        O::VcdResume => VO::VcdResume,
                    };
                    VI::Intrinsic(var!(*dst), Box::new(op), vm_args)
                }
                I::LastUpdateTime(dst, signal) => {
                    let signal = signal!(*signal);
                    VI::LastUpdateTime(var!(*dst), signal)
                }
                I::Probe(dst, signal) => {
                    let size = gl.vars[*dst].size;
                    let signal = signal!(*signal);
                    match gl.logic_mode {
                        LogicMode::TwoValue => VI::TvResize(
                            var!(*dst).to_ref(size),
                            vogls_ir::ResizeOp::Truncate,
                            signals[signal.0 as usize],
                        ),
                        LogicMode::FourValue => VI::FvResize(
                            var!(*dst).to_ref(size),
                            vogls_ir::ResizeOp::Truncate,
                            signals[signal.0 as usize],
                        ),
                    }
                }
                I::Drive(signal, src, partial) => {
                    let src_size = gl.vars[*src].size;
                    VI::Drive(
                        signal!(*signal),
                        var!(*src, (gl.logic_mode, var_mode[src], src_size)).to_ref(src_size),
                        partial.map(|(o, _)| var!(o, (gl.logic_mode, var_mode[&o], INTEGER_VSIZE))),
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
                use LogicMode as M;
                let i = match (dst_mode, src_mode) {
                    (M::TwoValue, M::TwoValue) => VI::TvResize(
                        dst.to_ref(size),
                        vogls_ir::ResizeOp::Truncate,
                        src.to_ref(size),
                    ),
                    (M::FourValue, M::FourValue) => VI::FvResize(
                        dst.to_ref(size),
                        vogls_ir::ResizeOp::Truncate,
                        src.to_ref(size),
                    ),
                    (M::TwoValue, M::FourValue) => VI::FvToTv(dst.to_ref(src_size), src),
                    (M::FourValue, M::TwoValue) => VI::TvToFv(dst.to_ref(src_size), src),
                };
                instructions.push(i);
            }
        }

        use BasicBlockTerminator as T;
        let terminator_instr = match &bb.terminator {
            T::Wait(_, time) => {
                instructions.push(VI::Wait(*time));
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
            T::Branch(cond, _, _) => VI::Branch(var!(*cond), 0, 0),
            T::Halt => VI::Halt,
        };

        bb_transitions.push((instructions.len(), bb_key));
        instructions.push(terminator_instr);

        bb_seen.insert(bb_key);
        bb.terminator.extend_next_rev(&mut bb_stack, &mut bb_seen);
    }

    // Correct the offsets of the transitions between basic blocks.
    let bb_to_offset = |bb_key: BasicBlockKey| *bb_offsets.get(&bb_key).unwrap();
    for (offset, bb_key) in bb_transitions {
        let bb = gl.bbs.get(bb_key).unwrap();

        use BasicBlockTerminator as T;
        use VmInstruction as VI;
        match (&bb.terminator, &mut instructions[offset]) {
            (T::Wait(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
            (T::WaitRegion(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
            (T::Watch(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
            (T::Jump(bb), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
            (T::Branch(_, true_bb, false_bb), VI::Branch(_, true_offset, false_offset)) => {
                *true_offset = bb_to_offset(*true_bb);
                *false_offset = bb_to_offset(*false_bb);
            }
            (T::Halt, VI::Halt) => {}
            _ => unreachable!("invalid terminator combination"),
        }
    }

    VmProcess { instructions }
}
