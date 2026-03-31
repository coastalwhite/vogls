use hashbrown::hash_map::Entry;
use vogls_bits::{Bits, VectorSize};
use vogls_utils::{TableMap, TableMapEntry, VgHashMap, VgHashSet, new_table_key};

use crate::{
    BasicBlockKey, BinaryImmOp, BinaryOp, GlobalContext, Instruction, ProcessKey, ResizeOp,
    ShiftImmOp, SignalKey, UnaryOp, VariableKey,
};
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
    Probe(SignalKey),
    LastUpdateTime(SignalKey),
}

pub fn common_subexpr_elim(
    gl: &mut GlobalContext,
    process: ProcessKey,

    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let entry = gl.processes[process].entry;

    struct SignalDirty {
        lupdt: bool,
        probe: bool,
    }

    let mut exprs = TableMap::<ExprKey, CSExpr, VariableKey>::new();
    let mut var_lookup = VgHashMap::<VariableKey, ExprKey>::default();
    let mut var_remap = VgHashMap::<VariableKey, VariableKey>::default();
    let mut signal_dirty = VgHashMap::<SignalKey, SignalDirty>::default();

    macro_rules! try_lookup {
        ($var:expr) => {
            match var_lookup.get($var) {
                None => continue,
                Some(e) => *e,
            }
        };
    }

    scratch_stack.clear();
    scratch_seen.clear();
    scratch_seen.insert(entry);
    scratch_stack.push(entry);
    while let Some(bb_key) = scratch_stack.pop() {
        exprs.clear();
        var_lookup.clear();
        let bb = &mut gl.bbs[bb_key];
        for i in bb.instrs.iter_mut() {
            use Instruction as I;
            let mut dirty = false;
            let (dst, csexpr) = match i {
                I::Constant(dst, bits) => (*dst, CSExpr::Constant(bits.clone())),
                I::Unary(dst, op, src) => (*dst, CSExpr::Unary(*op, try_lookup!(src))),
                I::Resize(dst, op, src) => (
                    *dst,
                    CSExpr::Resize(*op, gl.vars[*dst].size, try_lookup!(src)),
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
                    CSExpr::Slice(gl.vars[*dst].size, try_lookup!(src), try_lookup!(offset)),
                ),
                I::SliceImm(dst, src, offset) => (
                    *dst,
                    CSExpr::SliceImm(gl.vars[*dst].size, try_lookup!(src), *offset),
                ),
                I::ShiftImm(dst, op, src, amount) => {
                    (*dst, CSExpr::ShiftImm(*op, try_lookup!(src), *amount))
                }
                I::Intrinsic(..) => continue,
                I::LastUpdateTime(dst, signal) => {
                    match signal_dirty.entry(*signal) {
                        Entry::Vacant(_) => {}
                        Entry::Occupied(mut entry) => {
                            dirty = std::mem::replace(&mut entry.get_mut().lupdt, false)
                        }
                    }
                    (*dst, CSExpr::LastUpdateTime(*signal))
                }
                I::Probe(dst, signal) => {
                    match signal_dirty.entry(*signal) {
                        Entry::Vacant(_) => {}
                        Entry::Occupied(mut entry) => {
                            dirty = std::mem::replace(&mut entry.get_mut().probe, false)
                        }
                    }
                    (*dst, CSExpr::Probe(*signal))
                }
                I::Drive(signal, _, _) => {
                    signal_dirty.insert(
                        *signal,
                        SignalDirty {
                            lupdt: true,
                            probe: true,
                        },
                    );
                    continue;
                }
                I::Phi(_, _) => continue,
            };
            let expr_key = match exprs.entry(csexpr) {
                TableMapEntry::Occupied(mut entry) if dirty => {
                    entry.set(dst);
                    entry.get_table_key()
                }
                TableMapEntry::Occupied(entry) => {
                    var_remap.insert(dst, *entry.get());
                    entry.get_table_key()
                }
                TableMapEntry::Vacant(entry) => entry.insert(dst).get_table_key(),
            };
            var_lookup.insert(dst, expr_key);
        }
        bb.terminator.for_each_bb(|bb_key| {
            if scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
    }

    if !var_remap.is_empty() {
        scratch_stack.clear();
        scratch_seen.clear();
        scratch_seen.insert(entry);
        scratch_stack.push(entry);
        while let Some(bb_key) = scratch_stack.pop() {
            let bb = &mut gl.bbs[bb_key];
            bb.instrs.retain_mut(|i| {
                if i.get_destination_variable()
                    .is_some_and(|dst| var_remap.contains_key(&dst))
                {
                    false
                } else {
                    i.map_vars(|v| var_remap.get(&v).copied().unwrap_or(v));
                    true
                }
            });
            bb.instrs.shrink_to_fit();
            bb.terminator
                .map_vars(|v| var_remap.get(&v).copied().unwrap_or(v));

            bb.terminator.for_each_bb(|bb_key| {
                if scratch_seen.insert(bb_key) {
                    scratch_stack.push(bb_key);
                }
            });
        }
    }
}
