use std::collections::HashMap;

use vogls_codegen::{
    HeapBuilder, HeapOffset, HeapRef, insert_bb_phis, resolve_heap_map, resolve_var_logic_mode_map,
};
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, GlobalContext, Instruction, LogicMode, ProcessKey,
    SignalKey, UnaryOp, VariableKey, VectorSize,
};
use vogls_runtime::RtSignalKey;
use vogls_utils::{VgHashMap, VgHashSet};

use crate::ops::BytecodeOp;
use crate::BytecodeInstruction;

pub struct BytecodeProcess {
    start: usize,
    end: usize,
}

pub fn lower_to_bytecode(
    process: ProcessKey,
    gl: &GlobalContext,
    heap_builder: &mut HeapBuilder,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    ops: &mut Vec<BytecodeInstruction>,
) -> BytecodeProcess {
    use BytecodeInstruction as BI;
    use Instruction as I;

    let process = &gl.processes[process];
    let bytecode_start = ops.len();

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

    // Lower the IR instructions to Bytecode instructions.
    bb_stack.push(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get(bb_key).unwrap();

        bb_offsets.insert(bb_key, instructions.len());

        for instr in &bb.instrs {
            let instr = match instr {
                I::Constant(..) => continue,
                I::Unary(d, op, s) => {
                    let src_size = gl.vars.size(*s);
                    let (dm, mut sm) = (var_mode[d], var_mode[s]);
                    let d = heap_map[d];
                    let mut s = heap_map[s];
                    if let Some(mode) = vogls_codegen::unary_needs_convert(*op, dm, sm) {
                        s = convert_mode(gl, ops, &conv_map, mode, sm, s);
                        sm = mode;
                    }

                    use UnaryOp as O;
                    match op {
                        O::Neg => BI::negate(ops, d, s, dm, src_size),
                        O::ReduceOr => BI::reduce_or(ops, d, s, dm, src_size),
                        O::ReduceAnd => BI::reduce_and(ops, d, s, dm, src_size),
                        O::ReduceXor => BI::reduce_xor(ops, d, s, dm, src_size),
                    }
                }
                I::Resize(d, op, s) => todo!(),
                I::BinaryImm(d, op, src, imm) => todo!(),
                I::Slice(d, s1, s2) => todo!(),
                I::SliceImm(d, src, offset) => todo!(),
                I::ShiftImm(d, op, src, offset) => todo!(),
                I::Select(d, s, t, f) => todo!(),
                I::Binary(d, op, s1, s2) => todo!(),
                I::Intrinsic(dst, op, args) => todo!(),
                I::LastUpdateTime(dst, signal) => todo!(),
                I::Probe(dst, signal, offset) => todo!(),
                I::ProbeSlice(dst, signal, offset) => todo!(),
                I::Drive(signal, src, partial) => todo!(),
                I::Phi(..) => continue,
            };
        }

        if let Some(phis) = bb_phis.get(&bb_key) {
            for (dst, src) in phis {
                todo!();
            }
        }

        use BasicBlockTerminator as T;
        let terminator_instr = match &bb.terminator {
            T::Halt => BI::next_event(ops),
            _ => todo!(),
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
        todo!()
        // use VmInstruction as VI;
        // match (&bb.terminator, &mut instructions[offset]) {
        //     (T::Wait(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
        //     (T::VariableWait(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
        //     (T::WaitRegion(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
        //     (T::Watch(bb, _), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
        //     (T::Jump(bb), VI::Jump(offset)) => *offset = bb_to_offset(*bb),
        //     (
        //         T::Branch(_, true_bb, false_bb),
        //         VI::TvBranch(_, true_offset, false_offset)
        //         | VI::FvBranch(_, true_offset, false_offset),
        //     ) => {
        //         *true_offset = bb_to_offset(*true_bb);
        //         *false_offset = bb_to_offset(*false_bb);
        //     }
        //     (T::Halt, VI::Halt) => {}
        //     _ => unreachable!("invalid terminator combination"),
        // }
    }

    BytecodeProcess {
        start: bytecode_start,
        end: ops.len(),
    }
}

fn convert_mode(
    gl: &GlobalContext,
    ops: &mut Vec<BytecodeInstruction>,
    conv_map: &VgHashMap<VariableKey, HeapOffset>,
    tgt_mode: LogicMode,
    src_mode: LogicMode,
    s: HeapOffset,
) -> HeapOffset {
    todo!()
}
