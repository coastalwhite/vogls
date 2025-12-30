use std::collections::{HashMap, HashSet};

use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction, ProcessKey,
    SignalKey, VariableKey,
};

use crate::instruction::{StackRef, VmInstruction, VmProcess};
use crate::{BinaryArithmeticOp, BinaryComparisonOp, ShiftOp};

use super::VmSignalKey;

pub fn lower_process_to_vm(
    process: ProcessKey,
    gl: &GlobalContext,
    io_signals: &mut HashMap<SignalKey, VmSignalKey>,
) -> VmProcess {
    use Instruction as I;
    use VmInstruction as VI;

    let process = &gl.processes[process];

    let mut bb_stack = Vec::new();
    let mut bb_seen = HashSet::new();
    let mut bb_phis = HashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::new();

    let mut stack_map = HashMap::new();
    let mut bit_stack_top = 0;

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
                stack_map.insert(
                    dst,
                    StackRef {
                        offset: bit_stack_top,
                    },
                );
                bit_stack_top += (size.get() as usize).div_ceil(8);
            }
        }

        bb_seen.insert(bb_key);
        bb.terminator.extend_next_rev(&mut bb_stack, &mut bb_seen);
    }

    let bit_stack_size = bit_stack_top;

    bb_stack.clear();
    bb_seen.clear();
    let mut bb_offsets = HashMap::<BasicBlockKey, usize>::new();
    let mut bb_transitions = Vec::new();

    let mut instructions = Vec::new();

    macro_rules! signal {
        ($signal:expr) => {{
            let next = io_signals.len();
            *io_signals.entry($signal).or_insert(VmSignalKey(next as _))
        }};
    }

    // Lower the IR instructions to VM instructions.
    let var = |var: VariableKey| *stack_map.get(&var).unwrap();
    bb_stack.push(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get(bb_key).unwrap();

        bb_offsets.insert(bb_key, instructions.len());

        for instr in &bb.instrs {
            let instr = match instr {
                I::Constant(d, value) => VI::Constant(var(*d), value.clone()),

                I::Unary(d, op, s) => VI::Unary(var(*d), *op, gl.vars[*s].size, var(*s)),
                I::Resize(d, op, s) => {
                    VI::Resize(var(*d), *op, gl.vars[*d].size, gl.vars[*s].size, var(*s))
                }
                I::Binary(d, op, s1, s2) => {
                    let d = var(*d);
                    let s1 = var(*s1);
                    let s2 = var(*s2);
                    use BinaryArithmeticOp as BA;
                    use BinaryComparisonOp as BC;
                    use BinaryOp as O;
                    use ShiftOp as S;
                    match *op {
                        O::And(n) => VI::BinaryArithmetic(d, BA::And, n, s1, s2),
                        O::Or(n) => VI::BinaryArithmetic(d, BA::Or, n, s1, s2),
                        O::Xor(n) => VI::BinaryArithmetic(d, BA::Xor, n, s1, s2),
                        O::Add(n) => VI::BinaryArithmetic(d, BA::Add, n, s1, s2),
                        O::Sub(n) => VI::BinaryArithmetic(d, BA::Sub, n, s1, s2),
                        O::Multiply(n) => VI::BinaryArithmetic(d, BA::Multiply, n, s1, s2),
                        O::Divide(n) => VI::BinaryArithmetic(d, BA::Divide, n, s1, s2),
                        O::Modulus(n) => VI::BinaryArithmetic(d, BA::Modulus, n, s1, s2),
                        O::UnsignedLessEqual(n) => {
                            VI::BinaryComparison(d, BC::UnsignedLessEqual, n, s1, s2)
                        }
                        O::SelectBit(n) => VI::SelectBit(d, n, s1, s2),
                        O::LogicalShiftLeft(n, _) => VI::Shift(d, S::LogicalLeft, n, s1, s2),
                        O::LogicalShiftRight(n, _) => VI::Shift(d, S::LogicalRight, n, s1, s2),
                        O::ArithmeticShiftLeft(n, _) => VI::Shift(d, S::LogicalLeft, n, s1, s2),
                        O::ArithmeticShiftRight(n, _) => {
                            VI::Shift(d, S::ArithmeticRight, n, s1, s2)
                        }
                        O::Concat(l, r) => VI::Concat(d, l, s1, r, s2),
                    }
                }

                I::Intrinsic(dst, op, args) => {
                    let args = args.iter().map(|v| (var(*v), gl.vars[*v].size)).collect();
                    VI::Intrinsic(var(*dst), op.clone(), args)
                }
                I::Probe(dst, signal) => VI::Probe(var(*dst), signal!(*signal)),
                I::Drive(signal, src, region, partial) => VI::Drive(
                    signal!(*signal),
                    var(*src),
                    *region,
                    partial.map(|(o, l)| (var(o), l)),
                ),
                I::Phi(..) => continue,
            };

            instructions.push(instr);
        }

        if let Some(phis) = bb_phis.get(&bb_key) {
            for (dst, src) in phis {
                instructions.push(VI::Move(var(*dst), var(*src), gl.vars[*src].size));
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
            T::Branch(cond, _, _) => VI::Branch(var(*cond), 0, 0),
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

    VmProcess {
        bit_stack_size,
        instructions,
    }
}
