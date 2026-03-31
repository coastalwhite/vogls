use vogls_utils::{Table, VgHashMap, VgHashSet};

use crate::optimize::{CSExpr, ExprKey, remap_vars};
use crate::{BasicBlockKey, GlobalContext, Instruction, ProcessKey, ResizeOp, VariableKey};

pub fn peephole(
    gl: &mut GlobalContext,
    process: ProcessKey,

    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let entry = gl.processes[process].entry;

    let mut exprs = Table::<ExprKey, (VariableKey, CSExpr)>::new();
    let mut var_lookup = VgHashMap::<VariableKey, ExprKey>::default();
    let mut var_remap = VgHashMap::<VariableKey, VariableKey>::default();

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
            loop {
                let mut was_changed = false;
                match i {
                    I::Constant(..) => {}
                    I::Unary(..) => {}
                    I::Resize(dst, _, src) if gl.vars[*dst].size == gl.vars[*src].size => {
                        _ = var_remap.insert(*dst, *src)
                    }
                    I::Resize(dst, ResizeOp::Truncate, src) => {
                        if let Some(csexpr) = var_lookup.get(src) {
                            let (_, expr) = &exprs[*csexpr];
                            match expr {
                                CSExpr::Resize(ResizeOp::Truncate, _, src) => {
                                    *i = I::Resize(*dst, ResizeOp::Truncate, exprs[*src].0);
                                    was_changed = true;
                                }
                                CSExpr::SliceImm(_, src, offset) => {
                                    *i = I::SliceImm(*dst, exprs[*src].0, *offset);
                                    was_changed = true;
                                }
                                CSExpr::Probe(signal, _, offset) => {
                                    *i = I::Probe(*dst, *signal, *offset);
                                    was_changed = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    I::Resize(..) => {}
                    I::Binary(..) => {}
                    I::BinaryImm(..) => {}
                    I::Slice(..) => {}
                    I::SliceImm(dst, src, offset) => {
                        if let Some(csexpr) = var_lookup.get(src) {
                            let (_, expr) = &exprs[*csexpr];
                            match expr {
                                CSExpr::Resize(ResizeOp::Truncate, _, src) => {
                                    *i = I::SliceImm(*dst, exprs[*src].0, *offset);
                                    was_changed = true;
                                }
                                CSExpr::SliceImm(_, src, nested_offset) => {
                                    *i = I::SliceImm(*dst, exprs[*src].0, *nested_offset + *offset);
                                    was_changed = true;
                                }
                                CSExpr::Probe(signal, _, nested_offset) => {
                                    *i = I::Probe(*dst, *signal, *nested_offset + *offset);
                                    was_changed = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    I::ShiftImm(..) => {}
                    I::Intrinsic(..) => {}
                    I::LastUpdateTime(..) => {}
                    I::Probe(..) => {}
                    I::ProbeSlice(..) => {}
                    I::Drive(..) => {}
                    I::Phi(..) => {}
                }

                if !was_changed {
                    break;
                }
            }

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
                I::LastUpdateTime(dst, signal) => (*dst, CSExpr::LastUpdateTime(*signal)),
                I::Probe(dst, signal, offset) => {
                    (*dst, CSExpr::Probe(*signal, gl.vars[*dst].size, *offset))
                }
                I::ProbeSlice(dst, signal, offset) => (
                    *dst,
                    CSExpr::ProbeSlice(*signal, gl.vars[*dst].size, try_lookup!(offset)),
                ),
                I::Drive(_, _, _) => continue,
                I::Phi(_, _) => continue,
            };
            let expr_key = exprs.insert((dst, csexpr));
            var_lookup.insert(dst, expr_key);
        }
        bb.terminator.for_each_bb(|bb_key| {
            if scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
    }

    if !var_remap.is_empty() {
        let mut var_stack = Vec::new();
        let mut var_done = VgHashSet::default();
        remap_vars(
            gl,
            process,
            scratch_stack,
            scratch_seen,
            &mut var_remap,
            &mut var_stack,
            &mut var_done,
        );
    }
}
