use slotmap::SlotMap;
use vogls_bits::Bits;
use vogls_utils::{Table, VgHashMap, VgHashSet};

use crate::optimize::{CSExpr, ExprKey, remap_vars};
use crate::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryImmOpSimplification, GlobalContext,
    Instruction, LogicMode, ProcessKey, ResizeOp, ResizeOpSimplification, Signal, SignalKey,
    TvPushdownVariant, UnaryOp, UnaryOpSimplification, VariableKey, VariableMap,
};

pub fn peephole(
    gl: &mut GlobalContext,
    process: ProcessKey,

    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let mut exprs = Table::<ExprKey, (VariableKey, CSExpr)>::new();
    let mut var_lookup = VgHashMap::<VariableKey, ExprKey>::default();
    let mut var_remap = VgHashMap::<VariableKey, VariableKey>::default();

    for tr in &gl.processes[process].regions {
        exprs.clear();
        var_lookup.clear();
        var_remap.clear();

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
        scratch_seen.insert(tr.entry());
        scratch_stack.push(tr.entry());
        while let Some(bb_key) = scratch_stack.pop() {
            exprs.clear();
            var_lookup.clear();
            let bb = &mut gl.bbs[bb_key];

            let mut current_instr_idx = 0;
            while current_instr_idx < bb.instrs.len() {
                use Instruction as I;
                loop {
                    let result = peephole_instruction(
                        &mut bb.instrs,
                        current_instr_idx,
                        &mut gl.vars,
                        &mut gl.signals,
                        &exprs,
                        &var_lookup,
                    );
                    match result {
                        PeepholeResult::Unchanged => break,
                        PeepholeResult::Changed => {}

                        PeepholeResult::RemapVar { dst, src } => {
                            var_remap.insert(dst, src);
                            if let Some(&expr) = var_lookup.get(&src) {
                                var_lookup.insert(dst, expr);
                            }
                            break;
                        }
                        PeepholeResult::RemapExpr { dst, src } => {
                            var_remap.insert(dst, exprs[src].0);
                            var_lookup.insert(dst, src);
                            break;
                        }
                    }
                }

                let i = current_instr_idx;
                current_instr_idx += 1;
                let (dst, csexpr) = match &bb.instrs[i] {
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
                    I::Select(dst, cond, truthy, falsy) => (
                        *dst,
                        CSExpr::Select(try_lookup!(cond), try_lookup!(truthy), try_lookup!(falsy)),
                    ),
                    I::Intrinsic(..) => continue,
                    I::LastUpdateTime(dst, signal) => (*dst, CSExpr::LastUpdateTime(*signal)),
                    I::Probe(dst, signal, offset) => {
                        (*dst, CSExpr::Probe(*signal, gl.vars.size(*dst), *offset))
                    }
                    I::ProbeSlice(dst, signal, offset) => (
                        *dst,
                        CSExpr::ProbeSlice(*signal, gl.vars.size(*dst), try_lookup!(offset)),
                    ),
                    I::Drive(_, _, _) => continue,
                    I::Phi(_, _) => continue,
                };
                let expr_key = exprs.insert((dst, csexpr));
                var_lookup.insert(dst, expr_key);
            }
            bb.terminator.for_each_non_temporal_bb(|bb_key| {
                if scratch_seen.insert(bb_key) {
                    scratch_stack.push(bb_key);
                }
            });

            use BasicBlockTerminator as T;
            match &bb.terminator {
                T::Wait(_, _) => {}
                T::VariableWait(tr, src) => {
                    let src = try_lookup!(src);
                    if let CSExpr::Unary(UnaryOp::TvToFv, new_src) = exprs[src].1 {
                        bb.terminator = T::VariableWait(*tr, exprs[new_src].0);
                    }
                }
                T::WaitRegion(..) => {}
                T::Watch(..) => {}
                T::Jump(..) => {}
                T::Branch(..) => {}
                T::Halt => {}
            }
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

enum TwoValueResult {
    None,
    Source(VariableKey),
}

fn try_get_two_value(i: &CSExpr, exprs: &Table<ExprKey, (VariableKey, CSExpr)>) -> TwoValueResult {
    match i {
        CSExpr::Unary(UnaryOp::TvToFv, src) => TwoValueResult::Source(exprs[*src].0),
        _ => TwoValueResult::None,
    }
}

#[derive(Debug)]
enum PeepholeResult {
    Changed,
    Unchanged,
    RemapExpr { dst: VariableKey, src: ExprKey },
    RemapVar { dst: VariableKey, src: VariableKey },
}

fn peephole_instruction(
    instrs: &mut Vec<Instruction>,
    i: usize,
    vars: &mut VariableMap,
    _signals: &SlotMap<SignalKey, Signal>,
    exprs: &Table<ExprKey, (VariableKey, CSExpr)>,
    var_lookup: &VgHashMap<VariableKey, ExprKey>,
) -> PeepholeResult {
    use Instruction as I;
    let instr = &mut instrs[i];
    match instr {
        I::Constant(..) => {}
        I::Unary(dst, op, src) => {
            let (dst, op, src) = (*dst, *op, *src);
            let src_size = vars.size(src);
            match op.simplify(src_size, src.mode()) {
                UnaryOpSimplification::Keep => {}
                UnaryOpSimplification::Source => {
                    return PeepholeResult::RemapVar { dst, src };
                }
            }

            if let Some(csexpr) = var_lookup.get(&src) {
                let (_, expr) = &exprs[*csexpr];

                if src.mode().is_four_value() && op.supports_tv_pushdown() {
                    let src_tv = try_get_two_value(expr, exprs);

                    match src_tv {
                        TwoValueResult::None => {}
                        TwoValueResult::Source(src_tv) => {
                            match dst.mode() {
                                LogicMode::TwoValue => {
                                    *instr = I::Unary(dst, op, src_tv);
                                }
                                LogicMode::FourValue => {
                                    let dst_size = vars.size(dst);
                                    let dst_tv = vars.insert(LogicMode::TwoValue, dst_size);
                                    *instr = I::Unary(dst_tv, op, src_tv);
                                    instrs.insert(i + 1, I::Unary(dst, UnaryOp::TvToFv, dst_tv));
                                }
                            }
                            return PeepholeResult::Changed;
                        }
                    }
                }

                match (op, expr) {
                    (UnaryOp::TvToFv, CSExpr::Unary(UnaryOp::FvToTv, src)) => {
                        return PeepholeResult::RemapExpr { dst, src: *src };
                    }
                    (UnaryOp::FvToTv, CSExpr::Unary(UnaryOp::TvToFv, src)) => {
                        // @NOTE:
                        // TvToFv(FvToTv(X)) = {
                        //   0 -> 0,
                        //   1 -> 1,
                        //   x -> 0,
                        //   z -> 0,
                        // }
                        //
                        // This is the same as BitwiseCaseEq(X, -1)

                        let src = exprs[*src].0;

                        debug_assert_eq!(dst.mode(), LogicMode::TwoValue);
                        debug_assert_eq!(src.mode(), LogicMode::TwoValue);

                        let size = vars.size(dst);
                        *instr = I::BinaryImm(
                            dst,
                            BinaryImmOp::BitwiseCaseEquality,
                            src,
                            Bits::new_ones(size),
                        );
                        return PeepholeResult::Changed;
                    }

                    _ => {}
                }
            }
        }
        I::Resize(dst, _, src) if vars.size(*dst) == vars.size(*src) => {
            return PeepholeResult::RemapVar {
                dst: *dst,
                src: *src,
            };
        }
        I::Resize(dst, ResizeOp::Truncate, src) => {
            if let Some(csexpr) = var_lookup.get(src) {
                let (_, expr) = &exprs[*csexpr];
                match expr {
                    CSExpr::Resize(ResizeOp::Truncate, _, src) => {
                        *instr = I::Resize(*dst, ResizeOp::Truncate, exprs[*src].0);
                        return PeepholeResult::Changed;
                    }
                    CSExpr::SliceImm(_, src, offset) => {
                        *instr = I::SliceImm(*dst, exprs[*src].0, *offset);
                        return PeepholeResult::Changed;
                    }
                    CSExpr::Probe(signal, _, offset) => {
                        *instr = I::Probe(*dst, *signal, *offset);
                        return PeepholeResult::Changed;
                    }
                    _ => {}
                }
            }
        }
        I::Resize(dst, op, src) => {
            let (dst, op, src) = (*dst, *op, *src);
            let dst_size = vars.size(dst);
            let src_size = vars.size(src);
            match op.simplify(dst_size, src_size, dst.mode()) {
                ResizeOpSimplification::Keep => {}
                ResizeOpSimplification::Source => {
                    return PeepholeResult::RemapVar { dst, src };
                }
            }

            if let Some(csexpr) = var_lookup.get(&src) {
                let (_, expr) = &exprs[*csexpr];
                if src.mode().is_four_value()
                    && let TwoValueResult::Source(src_tv) = try_get_two_value(expr, exprs)
                {
                    let dst_size = vars.size(dst);
                    let dst_tv = vars.insert(LogicMode::TwoValue, dst_size);
                    *instr = I::Resize(dst_tv, op, src_tv);
                    instrs.insert(i + 1, I::Unary(dst, UnaryOp::TvToFv, dst_tv));
                    return PeepholeResult::Changed;
                }
            }
        }
        I::Binary(dst, op, lhs, rhs) => {
            let (dst, op, lhs, rhs) = (*dst, *op, *lhs, *rhs);
            if let (Some(lhs_ek), Some(rhs_ek)) = (var_lookup.get(&lhs), var_lookup.get(&rhs)) {
                if lhs.mode().is_four_value()
                    && let Some(tv_pushdown) = op.tv_pushdown_variant()
                {
                    let lhs_tv = try_get_two_value(&exprs[*lhs_ek].1, exprs);
                    let rhs_tv = try_get_two_value(&exprs[*rhs_ek].1, exprs);

                    use TwoValueResult as TVR;
                    match (lhs_tv, rhs_tv) {
                        (TVR::Source(lhs_tv), TVR::Source(rhs_tv)) => {
                            match tv_pushdown {
                                TvPushdownVariant::CastOutput => {
                                    let dst_size = vars.size(dst);
                                    let dst_tv = vars.insert(LogicMode::TwoValue, dst_size);
                                    *instr = I::Binary(dst_tv, op, lhs_tv, rhs_tv);
                                    instrs.insert(i + 1, I::Unary(dst, UnaryOp::TvToFv, dst_tv));
                                }
                                TvPushdownVariant::KeepOutput => {
                                    *instr = I::Binary(dst, op, lhs_tv, rhs_tv);
                                }
                            }
                            return PeepholeResult::Changed;
                        }
                        (TVR::None, _) | (_, TVR::None) => {}
                    }
                }

                match op {
                    _ => {}
                }
            }
        }
        I::BinaryImm(dst, op, src, imm) => {
            let (dst, op, src) = (*dst, *op, *src);
            match op.simplify(dst, src, imm) {
                BinaryImmOpSimplification::Keep => {}
                BinaryImmOpSimplification::Source => {
                    return PeepholeResult::RemapVar { dst, src };
                }
                BinaryImmOpSimplification::Immediate => {
                    *instr = I::Constant(dst, imm.clone_lowering_mode());
                    return PeepholeResult::Changed;
                }
                BinaryImmOpSimplification::Constant(bits) => {
                    *instr = I::Constant(dst, bits);
                    return PeepholeResult::Changed;
                }
                BinaryImmOpSimplification::Instruction(instruction) => {
                    *instr = instruction;
                    return PeepholeResult::Changed;
                }
            }

            if let Some(src_ek) = var_lookup.get(&src) {
                if src.mode().is_four_value()
                    && let Some(tv_pushdown) = op.tv_pushdown_variant()
                    && !imm.contains_special()
                {
                    let src_tv = try_get_two_value(&exprs[*src_ek].1, exprs);

                    use TwoValueResult as TVR;
                    match src_tv {
                        TVR::Source(src_tv) => {
                            match tv_pushdown {
                                TvPushdownVariant::CastOutput => {
                                    let dst_size = vars.size(dst);
                                    let dst_tv = vars.insert(LogicMode::TwoValue, dst_size);
                                    *instr =
                                        I::BinaryImm(dst_tv, op, src_tv, imm.clone_lowering_mode());
                                    instrs.insert(i + 1, I::Unary(dst, UnaryOp::TvToFv, dst_tv));
                                }
                                TvPushdownVariant::KeepOutput => {
                                    *instr =
                                        I::BinaryImm(dst, op, src_tv, imm.clone_lowering_mode());
                                }
                            }
                            return PeepholeResult::Changed;
                        }
                        TVR::None => {}
                    }
                }
            }
        }
        I::Slice(dst, src, offset) => {
            if let Some(csexpr) = var_lookup.get(src) {
                let (_, expr) = &exprs[*csexpr];
                match expr {
                    CSExpr::Resize(ResizeOp::Truncate, _, src) => {
                        *instr = I::Slice(*dst, exprs[*src].0, *offset);
                        return PeepholeResult::Changed;
                    }
                    CSExpr::Probe(signal, _, 0) => {
                        *instr = I::ProbeSlice(*dst, *signal, *offset);
                        return PeepholeResult::Changed;
                    }
                    _ => {}
                }
            }
        }
        I::SliceImm(dst, src, offset) => {
            if let Some(csexpr) = var_lookup.get(src) {
                let (_, expr) = &exprs[*csexpr];
                match expr {
                    CSExpr::Resize(ResizeOp::Truncate, _, src) => {
                        *instr = I::SliceImm(*dst, exprs[*src].0, *offset);
                        return PeepholeResult::Changed;
                    }
                    CSExpr::SliceImm(_, src, nested_offset) => {
                        *instr = I::SliceImm(*dst, exprs[*src].0, *nested_offset + *offset);
                        return PeepholeResult::Changed;
                    }
                    CSExpr::Probe(signal, _, nested_offset) => {
                        *instr = I::Probe(*dst, *signal, *nested_offset + *offset);
                        return PeepholeResult::Changed;
                    }
                    _ => {}
                }
            }
        }
        I::ShiftImm(..) => {}
        I::Select(dst, cond, truthy, falsy) => {
            if let (Some(truthy_ek), Some(falsy_ek)) =
                (var_lookup.get(truthy), var_lookup.get(falsy))
                && *truthy_ek == *falsy_ek
            {
                return PeepholeResult::RemapVar {
                    dst: *dst,
                    src: *truthy,
                };
            } else if let Some(cond_ek) = var_lookup.get(cond)
                && let CSExpr::Unary(UnaryOp::Neg, src_ek) = &exprs[*cond_ek].1
            {
                *instr = I::Select(*dst, exprs[*src_ek].0, *falsy, *truthy);
                return PeepholeResult::Changed;
            }
        }
        I::Intrinsic(..) => {}
        I::LastUpdateTime(..) => {}
        I::Probe(..) => {}
        I::ProbeSlice(..) => {}
        I::Drive(..) => {}
        I::Phi(..) => {}
    }

    PeepholeResult::Unchanged
}
