use std::collections::HashSet;

use slotmap::{SecondaryMap, SlotMap};
use vogls_bits::{Bits, VectorSize};
use vogls_utils::{VgHashMap, VgHashSet, new_table_key};

pub mod common_subexpr_elim;
pub mod constant_propagation;
pub mod deadcode_elimination;
pub mod peephole;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, GlobalContext,
    Instruction, ProcessKey, ResizeOp, ShiftImmOp, SignalKey, UnaryOp, VariableKey,
};

#[derive(Default, Clone, Copy)]
pub struct OptFlags {
    pub opt_rounds: u8,
    pub constant_propagation: bool,
    pub deadcode_elimination: bool,
    pub common_subexpr_elim: bool,
    pub peephole: bool,
}

new_table_key! {
    struct ExprKey;
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum CSExpr {
    Constant(Bits),
    Unary(UnaryOp, ExprKey),
    Resize(ResizeOp, VectorSize, ExprKey),
    Binary(BinaryOp, ExprKey, ExprKey),
    Slice(VectorSize, ExprKey, ExprKey),
    BinaryImm(BinaryImmOp, ExprKey, Bits),
    ShiftImm(ShiftImmOp, ExprKey, u32),
    SliceImm(VectorSize, ExprKey, u32),
    Probe(SignalKey, VectorSize, u32),
    ProbeSlice(SignalKey, VectorSize, ExprKey),
    LastUpdateTime(SignalKey),
}

pub fn optimize_processes(gl: &mut GlobalContext, processes: &[ProcessKey], flags: OptFlags) {
    let mut scratch_stack = Vec::new();
    let mut scratch_seen = VgHashSet::default();
    let mut scratch_mfr = VgHashSet::default();
    let mut scratch_map = VgHashMap::default();
    let mut scratch_dep = VgHashMap::default();
    let mut scratch_dep_edges = Vec::new();
    for &process in processes {
        for _ in 0..flags.opt_rounds {
            if flags.constant_propagation {
                constant_propagation::constant_propagation(
                    gl,
                    process,
                    &mut scratch_stack,
                    &mut scratch_seen,
                    &mut scratch_mfr,
                    &mut scratch_map,
                    &mut scratch_dep,
                    &mut scratch_dep_edges,
                );
            }
            if flags.common_subexpr_elim {
                common_subexpr_elim::common_subexpr_elim(
                    gl,
                    process,
                    &mut scratch_stack,
                    &mut scratch_seen,
                );
            }
            if flags.peephole {
                peephole::peephole(gl, process, &mut scratch_stack, &mut scratch_seen);
            }
            if flags.deadcode_elimination {
                deadcode_elimination::deadcode_elimination(
                    gl,
                    process,
                    &mut scratch_stack,
                    &mut scratch_seen,
                );
            }
            remove_needles_branches(gl, process, &mut scratch_stack, &mut scratch_seen);
        }
    }
}

pub fn get_fan_in<'a>(
    bbs: &mut SlotMap<BasicBlockKey, BasicBlock>,
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_fan_in: &mut SecondaryMap<BasicBlockKey, Vec<BasicBlockKey>>,
) {
    scratch_stack.clear();
    scratch_seen.clear();
    scratch_fan_in.clear();

    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let BasicBlock {
            instrs: _,
            terminator,
        } = &mut bbs[bb_key];
        terminator.for_each_bb(|bb_key| {
            if scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
        scratch_fan_in.insert(bb_key, Vec::new());
    }

    scratch_seen.clear();

    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let BasicBlock {
            instrs: _,
            terminator,
        } = &bbs[bb_key];
        terminator.for_each_bb(|next| {
            if scratch_seen.insert(next) {
                scratch_stack.push(next);
            }
            scratch_fan_in[next].push(bb_key)
        });
    }

    if !cfg!(debug_assertions) {
        return;
    }

    scratch_seen.clear();
    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let BasicBlock { instrs, terminator } = &mut bbs[bb_key];
        terminator.for_each_bb(|bb_key| {
            if scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
        for i in instrs {
            if let Instruction::Phi(_, srcs) = i {
                for (bb, _) in srcs {
                    assert!(scratch_fan_in[bb_key].contains(bb));
                }
            }
        }
    }
}

pub fn remove_needless_jumps(
    bbs: &mut SlotMap<BasicBlockKey, BasicBlock>,
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut HashSet<BasicBlockKey>,
    scratch_fan_in: &mut SecondaryMap<BasicBlockKey, Vec<BasicBlockKey>>,
) {
    scratch_stack.clear();
    scratch_seen.clear();
    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let BasicBlockTerminator::Jump(target_bb) = bbs[bb_key].terminator else {
            bbs[bb_key]
                .terminator
                .extend_next_rev(scratch_stack, scratch_seen);
            continue;
        };

        let [bb, target] = bbs.get_disjoint_mut([bb_key, target_bb]).unwrap();
        let [bb_fan_in, target_fan_in] = scratch_fan_in
            .get_disjoint_mut([bb_key, target_bb])
            .unwrap();

        if !bb.instrs.is_empty() && target_fan_in.len() != 1 {
            bb.terminator.extend_next_rev(scratch_stack, scratch_seen);
            continue;
        }

        for i in bb.instrs.iter_mut() {
            if let Instruction::Phi(_, srcs) = i {
                let mut new_srcs = Vec::with_capacity(bb_fan_in.len() + target_fan_in.len() - 1);
                for (b, v) in srcs.iter() {
                    if *b == bb_key {
                        new_srcs.extend(target_fan_in.iter().map(|t| (*t, *v)));
                    } else {
                        new_srcs.push((*b, *v));
                    }
                }
                *srcs = new_srcs.into();
            }
        }

        if bb.instrs.is_empty() {
            std::mem::swap(&mut bb.instrs, &mut target.instrs);
        } else {
            bb.instrs.extend(std::mem::take(&mut target.instrs));
        }
        std::mem::swap(&mut bb.terminator, &mut target.terminator);

        bb_fan_in.reserve(target_fan_in.len() - 1);
        bb_fan_in.extend(target_fan_in.iter().copied().filter(|k| *k != bb_key));

        for b in target_fan_in.iter().copied().filter(|k| *k != bb_key) {
            bbs[b].map_bb(|bb| if bb == target_bb { bb_key } else { bb });
        }

        let start_stack_len = scratch_stack.len();
        bbs[bb_key]
            .terminator
            .for_each_bb(|b| scratch_stack.push(b));
        for &b in &scratch_stack[start_stack_len..] {
            for f in scratch_fan_in[b].iter_mut() {
                if *f == target_bb {
                    *f = bb_key;
                }
            }
            bbs[b].map_bb(|bb| if bb == target_bb { bb_key } else { bb });
        }
        scratch_stack.truncate(start_stack_len);
        scratch_stack.push(bb_key);
    }
}

pub fn remove_needles_branches(
    gl: &mut GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let entry = gl.processes[process].entry;

    scratch_stack.clear();
    scratch_seen.clear();

    scratch_stack.push(entry);
    scratch_seen.insert(entry);
    while let Some(bb_key) = scratch_stack.pop() {
        let terminator = &mut gl.bbs[bb_key].terminator;
        if let BasicBlockTerminator::Branch(_, bb1, bb2) = terminator
            && bb1 == bb2
        {
            *terminator = BasicBlockTerminator::Jump(*bb1);
        }
        terminator.for_each_bb(|bb_key| {
            if scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
    }
}

pub fn remap_vars(
    gl: &mut GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    var_map: &mut VgHashMap<VariableKey, VariableKey>,
    var_stack: &mut Vec<VariableKey>,
    var_done: &mut VgHashSet<VariableKey>,
) {
    var_done.clear();
    var_stack.clear();
    var_stack.extend(var_map.keys());
    while let Some(src) = var_stack.pop() {
        if var_done.contains(&src) {
            continue;
        }

        let dst = var_map[&src];
        match var_map.get(&dst) {
            None => _ = var_done.insert(src),
            Some(&dst_dst) if var_done.contains(&dst) => {
                var_done.insert(src);
                *var_map.get_mut(&src).unwrap() = dst_dst;
            }
            Some(_) => var_stack.extend_from_slice(&[src, dst]),
        }
    }

    let entry = gl.processes[process].entry;

    scratch_stack.clear();
    scratch_seen.clear();
    scratch_seen.insert(entry);
    scratch_stack.push(entry);
    while let Some(bb_key) = scratch_stack.pop() {
        let bb = &mut gl.bbs[bb_key];
        bb.instrs.retain_mut(|i| {
            if i.get_destination_variable()
                .is_some_and(|dst| var_map.contains_key(&dst))
            {
                false
            } else {
                i.map_vars(|v| var_map.get(&v).copied().unwrap_or(v));
                true
            }
        });
        bb.instrs.shrink_to_fit();
        bb.terminator
            .map_vars(|v| var_map.get(&v).copied().unwrap_or(v));

        bb.terminator.for_each_bb(|bb_key| {
            if scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
    }
}
