use std::collections::{HashMap, HashSet};

use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, GlobalContext, Instruction, IntrinsicArg, SectionKey,
    SectionVariant, SignalKey, Type, VariableKey,
};

use crate::instruction::{StackRef, VmInstruction, VmIntrinsicArg, VmProcess};

use super::VmSignalKey;

fn get_stack_size(ty: &Type) -> usize {
    match ty {
        Type::Bit => 1,
    }
}

pub fn lower_process_to_vm(
    section_key: SectionKey,
    gl: &GlobalContext,
    io_signals: &HashMap<SignalKey, VmSignalKey>,
) -> VmProcess {
    let section = gl.sections.get(section_key).unwrap();
    assert_eq!(section.variant, SectionVariant::Process);

    let mut bb_stack = Vec::new();
    let mut bb_seen = HashSet::new();

    let mut stack_map = HashMap::new();
    let mut stack_top = 0;

    // Make a map of the stack.
    bb_stack.push(section.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get(bb_key).unwrap();

        for instr in &bb.instrs {
            if let Some(dst) = instr.get_destination_variable() {
                let var_stack_size = get_stack_size(&gl.vars.get(dst).unwrap().ty);
                stack_map.insert(
                    dst,
                    StackRef {
                        offset: stack_top,
                        size: var_stack_size,
                    },
                );
                stack_top += var_stack_size;
            }
        }

        bb_seen.insert(bb_key);
        bb.terminator.extend_next_rev(&mut bb_stack, &mut bb_seen);
    }

    let stack_size = stack_top;

    bb_stack.clear();
    bb_seen.clear();
    let mut bb_offsets = HashMap::<BasicBlockKey, usize>::new();
    let mut bb_transitions = Vec::new();

    let mut instructions = Vec::new();

    // Lower the IR instructions to VM instructions.
    let var = |var: VariableKey| *stack_map.get(&var).unwrap();
    bb_stack.push(section.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get(bb_key).unwrap();

        bb_offsets.insert(bb_key, instructions.len());

        for instr in &bb.instrs {
            use Instruction as I;
            use VmInstruction as VI;
            let instr = match instr {
                I::Constant(d, value) => VI::Constant(var(*d), value.clone()),
                I::Unary(d, op, s) => VI::Unary(var(*d), *op, var(*s)),
                I::Binary(d, op, s1, s2) => VI::Binary(var(*d), *op, var(*s1), var(*s2)),
                I::Intrinsic(op, args) => {
                    use IntrinsicArg as IA;
                    use VmIntrinsicArg as VIA;
                    let args = args
                        .iter()
                        .map(|arg| match arg {
                            IA::StringLiteral(s) => VIA::StringLiteral(s.clone()),
                            IA::Variable(v) => VIA::Variable(var(*v)),
                        })
                        .collect();

                    VI::Intrinsic(*op, args)
                }
                I::Probe(dst, signal) => VI::Probe(var(*dst), *io_signals.get(signal).unwrap()),
                I::Drive(signal, src) => VI::Drive(*io_signals.get(signal).unwrap(), var(*src)),
                I::Instantiate(_, _) | I::Signal(_) => unreachable!(),
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
                instructions.push(VI::Watch(
                    signals
                        .iter()
                        .map(|s| *io_signals.get(s).unwrap())
                        .collect(),
                ));
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
        stack_size,
        instructions,
    }
}
