use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

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
    Instruction, LogicMode, ProcessKey, ResizeOp, ShiftImmOp, SignalKey, TemporalRegionKey,
    UnaryOp, VariableKey,
};

#[derive(Default, Clone, Copy)]
pub struct OptFlags(u64);

impl OptFlags {
    pub const ALL: Self = Self(0xFu64);
    pub const EMPTY: Self = Self(0u64);

    pub const CONSTANT_PROPAGATION: Self = Self(1u64 << 0);
    pub const DEADCODE_ELIMINATION: Self = Self(1u64 << 1);
    pub const COMMON_SUBEXPR_ELIM: Self = Self(1u64 << 2);
    pub const PEEPHOLE: Self = Self(1u64 << 3);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn set(&mut self, flags: OptFlags, do_set: bool) {
        if do_set {
            *self |= flags;
        } else {
            *self &= !flags;
        }
    }
}

impl Not for OptFlags {
    type Output = Self;
    fn not(self) -> Self::Output {
        self ^ Self::ALL
    }
}
impl BitXor<OptFlags> for OptFlags {
    type Output = Self;
    fn bitxor(self, rhs: OptFlags) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl BitXorAssign<OptFlags> for OptFlags {
    fn bitxor_assign(&mut self, rhs: OptFlags) {
        self.0 ^= rhs.0;
    }
}
impl BitAnd<OptFlags> for OptFlags {
    type Output = Self;
    fn bitand(self, rhs: OptFlags) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign<OptFlags> for OptFlags {
    fn bitand_assign(&mut self, rhs: OptFlags) {
        self.0 &= rhs.0;
    }
}
impl BitOr<OptFlags> for OptFlags {
    type Output = Self;
    fn bitor(self, rhs: OptFlags) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign<OptFlags> for OptFlags {
    fn bitor_assign(&mut self, rhs: OptFlags) {
        self.0 |= rhs.0;
    }
}

#[derive(Default, Clone, Copy)]
pub struct Optimizations {
    pub rounds: u8,
    pub flags: OptFlags,
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

pub fn optimize_processes(gl: &mut GlobalContext, processes: &[ProcessKey], opts: Optimizations) {
    let mut scratch_stack = Vec::new();
    let mut scratch_seen = VgHashSet::default();
    let mut scratch_mfr = VgHashSet::default();
    let mut scratch_map = VgHashMap::default();
    for &process in processes {
        if !gl.processes.contains_key(process) {
            continue;
        }

        for _ in 0..opts.rounds {
            if opts.flags.contains(OptFlags::CONSTANT_PROPAGATION) {
                constant_propagation::constant_propagation(
                    gl,
                    process,
                    &mut scratch_stack,
                    &mut scratch_seen,
                    &mut scratch_mfr,
                    &mut scratch_map,
                );
            }
            crate::form::check_ir_form(&gl.processes[process].regions, gl);
            if opts.flags.contains(OptFlags::COMMON_SUBEXPR_ELIM) {
                common_subexpr_elim::common_subexpr_elim(
                    gl,
                    process,
                    &mut scratch_stack,
                    &mut scratch_seen,
                );
            }
            crate::form::check_ir_form(&gl.processes[process].regions, gl);
            if opts.flags.contains(OptFlags::PEEPHOLE) {
                peephole::peephole(gl, process, &mut scratch_stack, &mut scratch_seen);
            }
            crate::form::check_ir_form(&gl.processes[process].regions, gl);
            if opts.flags.contains(OptFlags::DEADCODE_ELIMINATION) {
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

/// Remove Basic Blocks from a process that are no longer reachable.
///
/// This patches up phi-instructions to also remove any references to these basic blocks.
pub fn remove_bbs(
    gl: &mut GlobalContext,
    process: ProcessKey,

    remove: &VgHashSet<BasicBlockKey>,

    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,

    scratch_fanin: &mut Vec<(BasicBlockKey, BasicBlockKey)>,
) {
    let process = &mut gl.processes[process];
    let tr = process.regions[0];
    if remove.is_empty() {
        return;
    }

    assert!(!remove.contains(&tr.entry()));

    scratch_stack.clear();
    scratch_seen.clear();
    scratch_fanin.clear();

    for bb in remove.iter() {
        gl.bbs.remove(*bb);
    }
    process.regions.retain(|tr| !remove.contains(&tr.entry()));

    let mut has_phi_instruction = false;
    scratch_stack.push(tr.entry());
    scratch_seen.insert(tr.entry());
    while let Some(bb_key) = scratch_stack.pop() {
        let bb = &gl.bbs[bb_key];
        bb.terminator.for_each_temporal_bb(|next| {
            scratch_fanin.push((bb_key, next));
            assert!(!remove.contains(&next));
            if scratch_seen.insert(next) {
                scratch_stack.push(next);
            }
        });

        has_phi_instruction |= bb.instrs.iter().any(|i| matches!(i, Instruction::Phi(..)));
    }

    // If there are no PHI instructions to patch up, what are we doing here? Stop early.
    if !has_phi_instruction {
        return;
    }

    scratch_fanin.sort_unstable_by_key(|(bb, next)| (*next, *bb));
    let Some((_, mut current)) = scratch_fanin.first().copied() else {
        unreachable!("A phi instruction implies there are BBs with a fanin");
    };

    // We patch up the PHI instructions by sorting both the fanin of the BB and PHI sources and
    // removing all items that no longer align.
    let mut start = 0;
    for i in 1..scratch_fanin.len() + 1 {
        let next = scratch_fanin.get(i);
        if next.is_none_or(|(_, next)| current != *next) {
            let slice = &scratch_fanin[start..i];

            gl.bbs[current].instrs.iter_mut().for_each(|instr| {
                if let Instruction::Phi(dst, srcs) = instr {
                    if srcs.len() == slice.len() {
                        return;
                    }

                    assert!(srcs.len() > slice.len());
                    srcs.sort_unstable_by_key(|(bb, _)| *bb);

                    let mut k = 0;
                    let mut new_srcs = srcs.to_vec();
                    new_srcs.retain(|(src_bb, _)| {
                        if slice.get(k).is_some_and(|(v, _)| v == src_bb) {
                            k += 1;
                            true
                        } else {
                            false
                        }
                    });
                    if new_srcs.len() == 1 {
                        *instr = Instruction::copy(&gl.vars, *dst, srcs[0].1);
                    } else {
                        *srcs = new_srcs.into_boxed_slice();
                    }
                }
            });

            if let Some((_, next)) = next {
                current = *next;
            }
            start = i;
        }
    }
}
