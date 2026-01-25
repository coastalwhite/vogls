use std::collections::{HashMap, HashSet};

use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction, IntrinsicOp,
    LogicMode, ProcessKey, SignalKey, VariableKey,
};

use crate::instruction::{StackRef, VmInstruction, VmProcess};
use crate::{BinaryArithmeticOp, BinaryComparisonOp, ShiftOp, VmIntrinsicOp};

use super::VmSignalKey;

pub fn lower_process_to_vm(
    process: ProcessKey,
    gl: &GlobalContext,
    stack_top: &mut usize,
    io_signals: &HashMap<SignalKey, VmSignalKey>,
) -> VmProcess {
    use Instruction as I;
    use VmInstruction as VI;

    let process = &gl.processes[process];

    let mut bb_stack = Vec::new();
    let mut bb_seen = HashSet::new();
    let mut bb_phis = HashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::new();

    let mut stack_map = HashMap::new();

    // Make a map of the stack.
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

                // Most arithmetic operations are implemented on 64-bit chunks, by ensuring that
                // all variables with a size larger or equal to than 64 are allocated in chunks of
                // 64 we can efficiently dispatch to these kernels.
                let nbytes = if size.get() > 32 {
                    // @TODO: Allow this space to be used somehow. In general, we should use a
                    // slab allocator instead of this.
                    *stack_top += 8 - (*stack_top % 8); // pad to 8-bytes
                    if gl.logic_mode == LogicMode::TwoValue {
                        (size.get() as usize).div_ceil(64) * 8
                    } else {
                        2 * (size.get() as usize).div_ceil(64) * 8
                    }
                } else {
                    if gl.logic_mode == LogicMode::TwoValue {
                        (size.get() as usize).div_ceil(8)
                    } else {
                        (2 * size.get() as usize).div_ceil(8)
                    }
                };

                stack_map.insert(dst, StackRef { offset: *stack_top });
                *stack_top += nbytes;
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
        ($var:expr) => {{ stack_map[&$var] }};
    }

    // Lower the IR instructions to VM instructions.
    bb_stack.push(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get(bb_key).unwrap();

        bb_offsets.insert(bb_key, instructions.len());

        for instr in &bb.instrs {
            let instr = match instr {
                I::Constant(d, value) => VI::Constant(var!(*d), value.clone()),

                I::TvUnary(d, op, s) => VI::TvUnary(var!(*d), *op, gl.vars[*s].size, var!(*s)),
                I::TvResize(d, op, s) => {
                    VI::TvResize(var!(*d), *op, gl.vars[*d].size, gl.vars[*s].size, var!(*s))
                }
                I::TvBinary(d, op, s1, s2) => {
                    let d_size = gl.vars[*d].size;
                    let s1_size = gl.vars[*s1].size;
                    let s2_size = gl.vars[*s2].size;
                    let d = var!(*d);
                    let s1 = var!(*s1);
                    let s2 = var!(*s2);
                    use BinaryArithmeticOp as BA;
                    use BinaryComparisonOp as BC;
                    use BinaryOp as O;
                    use ShiftOp as S;
                    match *op {
                        O::And => VI::TvBinaryArithmetic(d, BA::And, d_size, s1, s2),
                        O::Or => VI::TvBinaryArithmetic(d, BA::Or, d_size, s1, s2),
                        O::Xor => VI::TvBinaryArithmetic(d, BA::Xor, d_size, s1, s2),
                        O::Add => VI::TvBinaryArithmetic(d, BA::Add, d_size, s1, s2),
                        O::Sub => VI::TvBinaryArithmetic(d, BA::Sub, d_size, s1, s2),
                        O::Multiply => VI::TvBinaryArithmetic(d, BA::Multiply, d_size, s1, s2),
                        O::Divide => VI::TvBinaryArithmetic(d, BA::Divide, d_size, s1, s2),
                        O::Modulus => VI::TvBinaryArithmetic(d, BA::Modulus, d_size, s1, s2),

                        O::UnsignedLessEqual => {
                            VI::TvBinaryComparison(d, BC::UnsignedLessEqual, s1_size, s1, s2)
                        }
                        O::SelectBit => VI::TvSelectBit(d, s1_size, s1, s2),
                        O::LogicalShiftLeft => VI::TvShift(d, S::LogicalLeft, d_size, s1, s2),
                        O::LogicalShiftRight => VI::TvShift(d, S::LogicalRight, d_size, s1, s2),
                        O::ArithmeticShiftRight => {
                            VI::TvShift(d, S::ArithmeticRight, d_size, s1, s2)
                        }
                        O::Concat => VI::TvConcat(d, s1_size, s1, s2_size, s2),
                    }
                }

                I::FvUnary(d, op, s) => VI::FvUnary(var!(*d), *op, gl.vars[*s].size, var!(*s)),
                I::FvResize(d, op, s) => {
                    VI::FvResize(var!(*d), *op, gl.vars[*d].size, gl.vars[*s].size, var!(*s))
                }
                I::FvBinary(d, op, s1, s2) => {
                    let d_size = gl.vars[*d].size;
                    let s1_size = gl.vars[*s1].size;
                    let s2_size = gl.vars[*s2].size;
                    let d = var!(*d);
                    let s1 = var!(*s1);
                    let s2 = var!(*s2);
                    use BinaryArithmeticOp as BA;
                    use BinaryComparisonOp as BC;
                    use BinaryOp as O;
                    use ShiftOp as S;
                    match *op {
                        O::And => VI::FvBinaryArithmetic(d, BA::And, d_size, s1, s2),
                        O::Or => VI::FvBinaryArithmetic(d, BA::Or, d_size, s1, s2),
                        O::Xor => VI::FvBinaryArithmetic(d, BA::Xor, d_size, s1, s2),
                        O::Add => VI::FvBinaryArithmetic(d, BA::Add, d_size, s1, s2),
                        O::Sub => VI::FvBinaryArithmetic(d, BA::Sub, d_size, s1, s2),
                        O::Multiply => VI::FvBinaryArithmetic(d, BA::Multiply, d_size, s1, s2),
                        O::Divide => VI::FvBinaryArithmetic(d, BA::Divide, d_size, s1, s2),
                        O::Modulus => VI::FvBinaryArithmetic(d, BA::Modulus, d_size, s1, s2),

                        O::UnsignedLessEqual => {
                            VI::FvBinaryComparison(d, BC::UnsignedLessEqual, s1_size, s1, s2)
                        }
                        O::SelectBit => VI::FvSelectBit(d, s1_size, s1, s2),
                        O::LogicalShiftLeft => VI::FvShift(d, S::LogicalLeft, d_size, s1, s2),
                        O::LogicalShiftRight => VI::FvShift(d, S::LogicalRight, d_size, s1, s2),
                        O::ArithmeticShiftRight => {
                            VI::FvShift(d, S::ArithmeticRight, d_size, s1, s2)
                        }
                        O::Concat => VI::FvConcat(d, s1_size, s1, s2_size, s2),
                    }
                }

                I::Intrinsic(dst, op, args) => {
                    let args = args.iter().map(|v| (var!(*v), gl.vars[*v].size)).collect();
                    use crate::VcdScope as VmVcdScope;
                    use IntrinsicOp as O;
                    use VmIntrinsicOp as VO;
                    let op = match op.as_ref() {
                        O::Time => VO::Time,
                        O::Finish => VO::Finish,
                        O::Display(f) => VO::Display(f.clone()),
                        O::Assert(f) => VO::Assert(f.clone()),
                        O::VcdOpenFile(f) => VO::VcdOpenFile(f.clone()),
                        O::VcdAppendModule(v) => {
                            VO::VcdAppendModule(VmVcdScope::lower(v, io_signals))
                        }
                        O::VcdPause => VO::VcdPause,
                        O::VcdResume => VO::VcdResume,
                    };
                    VI::Intrinsic(var!(*dst), Box::new(op), args)
                }
                I::Probe(dst, signal) => VI::Probe(var!(*dst), signal!(*signal)),
                I::Drive(signal, src, region, partial) => VI::Drive(
                    signal!(*signal),
                    var!(*src),
                    *region,
                    partial.map(|(o, l)| (var!(o), l)),
                ),
                I::Phi(..) => continue,
            };

            instructions.push(instr);
        }

        if let Some(phis) = bb_phis.get(&bb_key) {
            for (dst, src) in phis {
                let src_size = gl.vars[*src].size;
                let dst_size = gl.vars[*dst].size;
                let (dst, src) = (var!(*dst), var!(*src));
                let i = if gl.logic_mode == LogicMode::TwoValue {
                    VI::TvResize(dst, vogls_ir::ResizeOp::Truncate, dst_size, src_size, src)
                } else {
                    VI::FvResize(dst, vogls_ir::ResizeOp::Truncate, dst_size, src_size, src)
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
