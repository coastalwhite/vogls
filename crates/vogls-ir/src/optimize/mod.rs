use slotmap::SlotMap;
use vogls_bits::{Bits, VectorSize};
use vogls_utils::{VgHashMap, VgHashSet, new_table_key};

pub mod common_subexpr_elim;
pub mod constant_propagation;
pub mod control_flow_graph_dot;
pub mod deadcode_elimination;
// pub mod dominator;
pub mod peephole;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, GlobalContext,
    LogicMode, ProcessKey, ResizeOp, ShiftImmOp, SignalKey, TemporalRegionKey, UnaryOp,
    VariableKey,
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
    Constant(LogicMode, Bits),
    Unary(UnaryOp, ExprKey),
    Resize(ResizeOp, VectorSize, ExprKey),
    Binary(BinaryOp, ExprKey, ExprKey),
    Slice(VectorSize, ExprKey, ExprKey),
    BinaryImm(BinaryImmOp, ExprKey, Bits),
    ShiftImm(ShiftImmOp, ExprKey, u32),
    Select(ExprKey, ExprKey, ExprKey),
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
        if !gl.processes.contains_key(process) {
            continue;
        }

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
            crate::form::check_ir_form(&gl.processes[process].regions, gl);
            if flags.common_subexpr_elim {
                common_subexpr_elim::common_subexpr_elim(
                    gl,
                    process,
                    &mut scratch_stack,
                    &mut scratch_seen,
                );
            }
            crate::form::check_ir_form(&gl.processes[process].regions, gl);
            if flags.peephole {
                peephole::peephole(gl, process, &mut scratch_stack, &mut scratch_seen);
            }
            crate::form::check_ir_form(&gl.processes[process].regions, gl);
            if flags.deadcode_elimination {
                deadcode_elimination::deadcode_elimination(
                    gl,
                    process,
                    &mut scratch_stack,
                    &mut scratch_seen,
                );
            }
            remove_needles_branches(gl, process, &mut scratch_stack, &mut scratch_seen);

            // Remove empty processes
            if gl.processes[process].regions.is_empty() {
                gl.processes.remove(process);
                break;
            }

            if gl.processes[process].regions.len() == 1
                && let Some(tr) = gl.processes[process].regions.first()
            {
                let bb = &gl.bbs[tr.entry()];
                if bb.instrs.is_empty() && matches!(bb.terminator, BasicBlockTerminator::Halt) {
                    gl.processes.remove(process);
                    break;
                }
            }

            crate::form::check_ir_form(&gl.processes[process].regions, gl);
        }
    }
}

pub fn remove_needles_branches(
    gl: &mut GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    for tr in &gl.processes[process].regions {
        scratch_stack.clear();
        scratch_seen.clear();

        scratch_stack.push(tr.entry());
        scratch_seen.insert(tr.entry());
        while let Some(bb_key) = scratch_stack.pop() {
            let terminator = &mut gl.bbs[bb_key].terminator;
            if let BasicBlockTerminator::Branch(_, bb1, bb2) = terminator
                && bb1 == bb2
            {
                *terminator = BasicBlockTerminator::Jump(*bb1);
            }
            terminator.for_each_non_temporal_bb(|bb_key| {
                if scratch_seen.insert(bb_key) {
                    scratch_stack.push(bb_key);
                }
            });
        }
    }
}

pub fn remap_vars(
    bbs: &mut SlotMap<BasicBlockKey, BasicBlock>,
    tr: TemporalRegionKey,
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

    scratch_stack.clear();
    scratch_seen.clear();
    scratch_seen.insert(tr.entry());
    scratch_stack.push(tr.entry());
    while let Some(bb_key) = scratch_stack.pop() {
        let bb = &mut bbs[bb_key];
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

        bb.terminator.for_each_non_temporal_bb(|bb_key| {
            if scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
    }
}
