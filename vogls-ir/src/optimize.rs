use std::collections::{HashMap, HashSet};

use slotmap::{SecondaryMap, SlotMap};

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, Bits, Instruction, SCALAR_VSIZE,
    UnaryOp, Variable, VariableKey,
};

pub fn get_fan_in<'a>(
    bbs: &mut SlotMap<BasicBlockKey, BasicBlock>,
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut HashSet<BasicBlockKey>,
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
        terminator.extend_next_rev(scratch_stack, scratch_seen);
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
        terminator.extend_next_rev(scratch_stack, scratch_seen);
        terminator.for_each_bb(|bb| scratch_fan_in[bb].push(bb_key));
    }

    if !cfg!(debug_assertions) {
        return;
    }

    scratch_seen.clear();
    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let BasicBlock { instrs, terminator } = &mut bbs[bb_key];
        terminator.extend_next_rev(scratch_stack, scratch_seen);
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
    bbs: &mut SlotMap<BasicBlockKey, BasicBlock>,
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut HashSet<BasicBlockKey>,
) {
    scratch_stack.clear();
    scratch_seen.clear();

    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let terminator = &mut bbs[bb_key].terminator;
        if let BasicBlockTerminator::Branch(_, bb1, bb2) = terminator
            && bb1 == bb2
        {
            *terminator = BasicBlockTerminator::Jump(*bb1);
        }
        terminator.extend_next_rev(scratch_stack, scratch_seen);
    }
}

pub fn propagate_constants<'a>(
    bbs: &mut SlotMap<BasicBlockKey, BasicBlock>,
    vars: &SlotMap<VariableKey, Variable>,
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_mfr: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut HashSet<BasicBlockKey>,
    scratch_removed: &mut HashSet<BasicBlockKey>,
    scratch_map: &mut HashMap<VariableKey, Bits>,
    scratch_var_to_var_map: &mut HashMap<VariableKey, VariableKey>,
    scratch_fan_in: &mut SecondaryMap<BasicBlockKey, Vec<BasicBlockKey>>,
) {
    scratch_stack.clear();
    scratch_mfr.clear();
    scratch_seen.clear();
    scratch_removed.clear();
    scratch_map.clear();
    scratch_var_to_var_map.clear();

    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let BasicBlock { instrs, terminator } = &mut bbs[bb_key];

        for i in instrs {
            use Instruction as I;
            let (dst, bits) = match i {
                I::Constant(key, bits) => {
                    scratch_map.insert(*key, bits.clone());
                    continue;
                }
                I::Unary(dst, op, src) => {
                    let Some(bits) = scratch_map.get(src) else {
                        continue;
                    };

                    (*dst, op.evaluate(bits))
                }
                I::Resize(dst, op, src) => {
                    let Some(bits) = scratch_map.get(src) else {
                        continue;
                    };

                    let target_size = vars[*dst].size;
                    (*dst, op.evaluate(bits, target_size))
                }
                I::Binary(dst, op, src1, src2) => {
                    let csrc1 = scratch_map.get(src1);
                    let csrc2 = scratch_map.get(src2);

                    use BinaryOp as O;
                    (
                        *dst,
                        match (csrc1, csrc2) {
                            (Some(src1), Some(src2)) => op.evaluate(src1, src2),
                            (Some(src), _) | (_, Some(src)) => {
                                let non_constant_src = if csrc1.is_none() { *src1 } else { *src2 };
                                macro_rules! set_eq_to_non_constant_src {
                                    () => {{
                                        _ = scratch_var_to_var_map.insert(*dst, non_constant_src);
                                        continue;
                                    }};
                                }

                                let num_ones = src.count_ones();
                                let size = src.size();
                                match op {
                                    O::And if num_ones == 0 => Bits::new_zeroed(size),
                                    O::And | O::Add | O::Sub if num_ones == size.get() => {
                                        set_eq_to_non_constant_src!()
                                    }
                                    O::And => continue,
                                    O::Or | O::Xor if num_ones == 0 => {
                                        set_eq_to_non_constant_src!()
                                    }
                                    O::Or if num_ones == size.get() => Bits::new_ones(size),
                                    O::Or => continue,
                                    O::Xor if num_ones == size.get() => src.bitwise_negate(),
                                    O::Xor => continue,
                                    O::Add | O::Sub => continue,
                                    O::Multiply if num_ones == 0 => Bits::new_zeroed(size),
                                    O::Multiply | O::Divide if src.is_one() => {
                                        set_eq_to_non_constant_src!()
                                    }
                                    O::Multiply | O::Divide => continue,
                                    O::UnsignedLessEqual if num_ones == 0 && csrc1.is_none() => {
                                        *i = Instruction::Unary(
                                            *dst,
                                            UnaryOp::ReduceOr,
                                            non_constant_src,
                                        );
                                        continue;
                                    }
                                    O::UnsignedLessEqual if num_ones == 0 && csrc2.is_none() => {
                                        Bits::new_ones(SCALAR_VSIZE)
                                    }
                                    O::UnsignedLessEqual
                                        if num_ones == size.get() && csrc1.is_none() =>
                                    {
                                        Bits::new_ones(SCALAR_VSIZE)
                                    }
                                    O::UnsignedLessEqual
                                        if num_ones == size.get() && csrc2.is_none() =>
                                    {
                                        *i = Instruction::Unary(
                                            *dst,
                                            UnaryOp::ReduceAnd,
                                            non_constant_src,
                                        );
                                        continue;
                                    }
                                    O::LogicalShiftLeft
                                    | O::LogicalShiftRight
                                    | O::ArithmeticShiftRight
                                        if csrc1.is_none() && num_ones == 0 =>
                                    {
                                        set_eq_to_non_constant_src!();
                                    }
                                    O::LogicalShiftLeft
                                    | O::LogicalShiftRight
                                    | O::ArithmeticShiftRight
                                        if csrc2.is_none() && num_ones == 0 =>
                                    {
                                        Bits::new_zeroed(size)
                                    }
                                    O::Power
                                    | O::UnsignedLessEqual
                                    | O::Min
                                    | O::Max
                                    | O::CaseEquality
                                    | O::Modulus
                                    | O::SelectBit
                                    | O::LogicalShiftLeft
                                    | O::LogicalShiftRight
                                    | O::ArithmeticShiftRight
                                    | O::Concat
                                    | O::CopyX
                                    | O::CopyZ => continue,
                                }
                            }
                            (None, None) => continue,
                        },
                    )
                }
                I::Intrinsic(_, _, _) => continue,
                I::LastUpdateTime(_, _) => continue,
                I::Probe(_, _) => continue,
                I::Drive(_, _, _) => continue,
                I::Phi(_, _) => continue,
            };
            scratch_map.insert(dst, bits.clone());
            *i = I::Constant(dst, bits);
        }

        // Simplify constant branches.
        if let BasicBlockTerminator::Branch(condition, truthy_bb, falsy_bb) = terminator
            && let Some(condition) = scratch_map.get(condition)
        {
            let (mfr, jump) = if condition.not_eq_zero() {
                (*falsy_bb, *truthy_bb)
            } else {
                (*truthy_bb, *falsy_bb)
            };
            if !scratch_seen.contains(&mfr) {
                scratch_mfr.push(mfr);
            }
            *terminator = BasicBlockTerminator::Jump(jump);

            bbs[mfr].remove_fan_in_edge(bb_key);
            scratch_fan_in[mfr].retain(|k| *k != bb_key);
        }
        bbs[bb_key]
            .terminator
            .extend_next_rev(scratch_stack, scratch_seen);
    }

    while let Some(bb_key) = scratch_mfr.pop() {
        if !scratch_seen.insert(bb_key) {
            continue;
        }

        let start_stack_len = scratch_mfr.len();
        bbs[bb_key]
            .terminator
            .extend_next_rev(scratch_mfr, scratch_seen);

        for &fan_out in &scratch_mfr[start_stack_len..] {
            if !scratch_seen.contains(&fan_out) {
                continue;
            }

            bbs[fan_out].remove_fan_in_edge(bb_key);
            scratch_fan_in[fan_out].retain(|k| *k != bb_key);
        }

        bbs.remove(bb_key);
        scratch_fan_in.remove(bb_key);
    }

    if !scratch_var_to_var_map.is_empty() {
        scratch_seen.clear();
        scratch_stack.push(entry);
        scratch_seen.insert(entry);

        while let Some(bb_key) = scratch_stack.pop() {
            let BasicBlock { instrs, terminator } = &mut bbs[bb_key];

            instrs.retain_mut(|i| {
                if i.get_destination_variable()
                    .is_some_and(|dst| scratch_var_to_var_map.contains_key(&dst))
                {
                    return false;
                }
                i.map_vars(|v| scratch_var_to_var_map.get(&v).copied().unwrap_or(v));
                true
            });
            terminator.map_vars(|v| scratch_var_to_var_map.get(&v).copied().unwrap_or(v));
            terminator.extend_next_rev(scratch_stack, scratch_seen);
        }
    }
}

pub fn deadcode_elimination<'a>(
    bbs: &mut SlotMap<BasicBlockKey, BasicBlock>,
    vars: &mut SlotMap<VariableKey, Variable>,
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut HashSet<BasicBlockKey>,
    scratch_var_seen: &mut HashSet<VariableKey>,
) {
    scratch_stack.clear();
    scratch_seen.clear();
    scratch_var_seen.clear();

    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let BasicBlock { instrs, terminator } = &bbs[bb_key];
        for i in instrs {
            i.for_each_var_src(|v| _ = scratch_var_seen.insert(v));
        }
        terminator.for_each_var_src(|v| _ = scratch_var_seen.insert(v));
        terminator.extend_next_rev(scratch_stack, scratch_seen);
    }

    scratch_seen.clear();
    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let BasicBlock { instrs, terminator } = &mut bbs[bb_key];
        terminator.extend_next_rev(scratch_stack, scratch_seen);
        instrs.retain(|i| {
            if i.has_side_effects_on_call() {
                return true;
            }

            let Some(dst) = i.get_destination_variable() else {
                return true;
            };
            if scratch_var_seen.contains(&dst) {
                return true;
            }

            vars.remove(dst);
            false
        });
    }
}
