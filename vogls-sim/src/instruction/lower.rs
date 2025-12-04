use std::collections::{HashMap, HashSet};

use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, GlobalContext, Instruction, IntrinsicArg, ProcessKey,
    SignalKey, Type, VariableKey,
};

use crate::instruction::{StackRef, VmInstruction, VmIntrinsicArg, VmProcess};

use super::VmSignalKey;

pub fn lower_process_to_vm(
    process: ProcessKey,
    gl: &GlobalContext,
    io_signals: &mut HashMap<SignalKey, VmSignalKey>,
) -> VmProcess {
    let process = &gl.processes[process];

    let mut bb_stack = Vec::new();
    let mut bb_seen = HashSet::new();

    let mut stack_map = HashMap::new();
    let mut bit_stack_top = 0;
    let mut decimal_stack_top = 0;

    // Make a map of the stack.
    bb_stack.push(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get(bb_key).unwrap();

        for instr in &bb.instrs {
            if let Some(dst) = instr.get_destination_variable() {
                match &gl.vars.get(dst).unwrap().ty {
                    Type::Bit => {
                        stack_map.insert(
                            dst,
                            StackRef {
                                offset: bit_stack_top,
                                size: 1,
                            },
                        );
                        bit_stack_top += 1;
                    }
                    Type::Decimal => {
                        stack_map.insert(
                            dst,
                            StackRef {
                                offset: decimal_stack_top,
                                size: 1,
                            },
                        );
                        decimal_stack_top += 1;
                    }
                }
            }
        }

        bb_seen.insert(bb_key);
        bb.terminator.extend_next_rev(&mut bb_stack, &mut bb_seen);
    }

    let bit_stack_size = bit_stack_top;
    let decimal_stack_size = decimal_stack_top;

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
            use Instruction as I;
            use VmInstruction as VI;
            let instr = match instr {
                I::ConstantBit(d, value) => VI::ConstantBit(var(*d), value.clone()),
                I::UnaryBit(d, op, s) => VI::UnaryBit(var(*d), *op, var(*s)),
                I::BinaryBit(d, op, s1, s2) => VI::BinaryBit(var(*d), *op, var(*s1), var(*s2)),

                I::ConstantDecimal(d, value) => VI::ConstantDecimal(var(*d), value.clone()),
                I::UnaryDecimal(d, op, s) => VI::UnaryDecimal(var(*d), *op, var(*s)),
                I::BinaryDecimal(d, op, s1, s2) => {
                    VI::BinaryDecimal(var(*d), *op, var(*s1), var(*s2))
                }

                I::Cast(d, s) => VI::Cast(
                    var(*d),
                    gl.vars[*d].ty.clone(),
                    var(*s),
                    gl.vars[*s].ty.clone(),
                ),

                I::Intrinsic(op, args) => {
                    use IntrinsicArg as IA;
                    use VmIntrinsicArg as VIA;
                    let args = args
                        .iter()
                        .map(|arg| match arg {
                            IA::StringLiteral(s) => VIA::StringLiteral(s.clone()),
                            IA::Variable(v) => match gl.vars[*v].ty {
                                Type::Bit => VIA::VariableBit(var(*v)),
                                Type::Decimal => VIA::VariableDecimal(var(*v)),
                            },
                        })
                        .collect();

                    VI::Intrinsic(*op, args)
                }
                I::Probe(dst, signal) => VI::Probe(var(*dst), signal!(*signal)),
                I::Drive(signal, src) => VI::Drive(signal!(*signal), var(*src)),
                I::Instantiate(_, _) | I::Spawn(_, _) | I::Signal(_) => unreachable!(),
            };

            instructions.push(instr);
        }

        use BasicBlockTerminator as T;
        use VmInstruction as VI;
        let terminator_instr = match &bb.terminator {
            T::Wait(_, time) => {
                instructions.push(VI::Wait(*time));
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
        decimal_stack_size,
        instructions,
    }
}
