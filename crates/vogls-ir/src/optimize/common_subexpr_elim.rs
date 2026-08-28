use hashbrown::hash_map::Entry;
use vogls_utils::{TableMap, TableMapEntry, VgHashMap, VgHashSet};

use crate::optimize::{CSExpr, ExprKey, remap_vars};
use crate::{BasicBlockKey, GlobalContext, Instruction, ProcessKey, SignalKey, VariableKey};

pub fn common_subexpr_elim(
    gl: &mut GlobalContext,
    process: ProcessKey,

    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let mut exprs = TableMap::<ExprKey, CSExpr, (VariableKey, u64)>::new();
    let mut var_lookup = VgHashMap::<VariableKey, ExprKey>::default();
    let mut var_remap = VgHashMap::<VariableKey, VariableKey>::default();
    let mut signal_generation = VgHashMap::<SignalKey, u64>::default();

    for tr in &gl.processes[process].regions {
        exprs.clear();
        var_lookup.clear();
        var_remap.clear();
        signal_generation.clear();

        scratch_stack.clear();
        scratch_seen.clear();
        scratch_seen.insert(tr.entry());
        scratch_stack.push(tr.entry());
        while let Some(bb_key) = scratch_stack.pop() {
            exprs.clear();
            var_lookup.clear();
            let bb = &mut gl.bbs[bb_key];
            for i in bb.instrs.iter_mut() {
                use Instruction as I;
                let mut generation = 0u64;

                macro_rules! try_lookup {
                    ($var:expr) => {
                        match var_lookup.get($var) {
                            None => continue,
                            Some(e) => {
                                generation = generation.max(exprs[*e].1);
                                *e
                            }
                        }
                    };
                }

                let (dst, csexpr) = match i {
                    I::Constant(dst, bits) => (*dst, CSExpr::Constant(dst.mode(), bits.clone())),
                    I::Unary(dst, op, src) => (*dst, CSExpr::Unary(*op, try_lookup!(src))),
                    I::Resize(dst, op, src) => (
                        *dst,
                        CSExpr::Resize(*op, gl.vars.size(*dst), try_lookup!(src)),
                    ),
                    I::Binary(dst, op, lhs, rhs) => (
                        *dst,
                        CSExpr::Binary(*op, try_lookup!(lhs), try_lookup!(rhs)),
                    ),
                    I::BinaryImm(dst, op, lhs, imm) => {
                        (*dst, CSExpr::BinaryImm(*op, try_lookup!(lhs), imm.clone()))
                    }
                    I::Slice(dst, src, offset) => (
                        *dst,
                        CSExpr::Slice(gl.vars.size(*dst), try_lookup!(src), try_lookup!(offset)),
                    ),
                    I::SliceImm(dst, src, offset) => (
                        *dst,
                        CSExpr::SliceImm(gl.vars.size(*dst), try_lookup!(src), *offset),
                    ),
                    I::ShiftImm(dst, op, src, amount) => {
                        (*dst, CSExpr::ShiftImm(*op, try_lookup!(src), *amount))
                    }
                    I::Select(dst, cond, truthy, falsy, kind) => (
                        *dst,
                        CSExpr::Select(try_lookup!(cond), try_lookup!(truthy), try_lookup!(falsy), *kind),
                    ),
                    I::Intrinsic(..) => continue,
                    I::LastUpdateTime(dst, signal) => {
                        generation = signal_generation
                            .get(signal)
                            .map(|&v| v + 1)
                            .unwrap_or_default();

                        (*dst, CSExpr::LastUpdateTime(*signal))
                    }
                    I::Probe(dst, signal, offset) => {
                        generation = signal_generation
                            .get(signal)
                            .map(|&v| v + 1)
                            .unwrap_or_default();

                        (*dst, CSExpr::Probe(*signal, gl.vars.size(*dst), *offset))
                    }
                    I::ProbeSlice(dst, signal, offset) => {
                        generation = signal_generation
                            .get(signal)
                            .map(|&v| v + 1)
                            .unwrap_or_default();

                        (
                            *dst,
                            CSExpr::ProbeSlice(*signal, gl.vars.size(*dst), try_lookup!(offset)),
                        )
                    }
                    I::Drive(_, signal, _, _) | I::DriveSlice(_, signal, _, _) => {
                        match signal_generation.entry(*signal) {
                            Entry::Occupied(mut entry) => *entry.get_mut() += 1,
                            Entry::Vacant(entry) => _ = entry.insert(0),
                        };
                        continue;
                    }
                    I::Phi(_, _) => continue,
                };
                let expr_key = match exprs.entry(csexpr.clone()) {
                    TableMapEntry::Occupied(mut entry) if entry.get().1 != generation => {
                        entry.set((dst, generation));
                        entry.get_table_key()
                    }
                    TableMapEntry::Occupied(entry) => {
                        var_remap.insert(dst, entry.get().0);
                        entry.get_table_key()
                    }
                    TableMapEntry::Vacant(entry) => entry.insert((dst, generation)).get_table_key(),
                };
                var_lookup.insert(dst, expr_key);
            }
            bb.terminator.for_each_non_temporal_bb(|bb_key| {
                if scratch_seen.insert(bb_key) {
                    scratch_stack.push(bb_key);
                }
            });
        }

        if !var_remap.is_empty() {
            let mut var_stack = Vec::new();
            let mut var_done = VgHashSet::default();
            remap_vars(
                &mut gl.bbs,
                *tr,
                scratch_stack,
                scratch_seen,
                &mut var_remap,
                &mut var_stack,
                &mut var_done,
            );
        }
    }
}
