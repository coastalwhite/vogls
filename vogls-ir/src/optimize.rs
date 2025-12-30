use std::collections::{HashMap, HashSet};

use slotmap::SlotMap;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, Bits, Instruction, ResizeOp,
    SCALAR_VSIZE, UnaryOp, Variable, VariableKey,
};

pub fn remove_needless_jumps(
    bbs: &mut SlotMap<BasicBlockKey, BasicBlock>,
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_map: &mut HashMap<BasicBlockKey, BasicBlockKey>,
    scratch_bb_to_u32_map: &mut HashMap<BasicBlockKey, u32>,
    scratch_seen: &mut HashSet<BasicBlockKey>,
) -> BasicBlockKey {
    scratch_stack.clear();
    scratch_map.clear();
    scratch_bb_to_u32_map.clear();
    scratch_seen.clear();

    scratch_stack.push(entry);
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        let BasicBlock {
            name: _,
            instrs,
            terminator,
        } = &bbs[bb_key];
        terminator.extend_next_rev(scratch_stack, scratch_seen);
        if instrs.is_empty()
            && let BasicBlockTerminator::Jump(target_bb) = terminator
        {
            scratch_map.insert(bb_key, *target_bb);
        }
    }

    scratch_stack.push(entry);
    scratch_seen.clear();
    scratch_seen.insert(entry);

    while let Some(bb_key) = scratch_stack.pop() {
        bbs[bb_key]
            .terminator
            .extend_next_rev(scratch_stack, scratch_seen);
        if let Some(mut target_bb) = scratch_map.get(&bb_key).copied() {
            while let Some(new_target_bb) = scratch_map.get(&target_bb) {
                target_bb = *new_target_bb;
            }
            *scratch_map.get_mut(&bb_key).unwrap() = target_bb;
            bbs.remove(bb_key);
        }
    }

    let new_entry = scratch_map.get(&entry).copied().unwrap_or(entry);
    scratch_stack.push(new_entry);
    scratch_seen.clear();
    scratch_seen.insert(new_entry);
    scratch_bb_to_u32_map.insert(new_entry, 1);

    while let Some(bb_key) = scratch_stack.pop() {
        bbs[bb_key].map_bbs(&scratch_map);

        bbs[bb_key]
            .terminator
            .for_each_bb(|bb| *scratch_bb_to_u32_map.entry(bb).or_default() += 1);
        bbs[bb_key]
            .terminator
            .extend_next_rev(scratch_stack, scratch_seen);
    }

    scratch_stack.push(new_entry);
    scratch_seen.clear();
    scratch_seen.insert(new_entry);

    let mut remapped_jump = false;
    while let Some(bb_key) = scratch_stack.pop() {
        while let BasicBlockTerminator::Jump(target_bb) = &mut bbs[bb_key].terminator
            && scratch_bb_to_u32_map[target_bb] == 1
        {
            let target_bb = *target_bb;
            let [
                BasicBlock {
                    name: _,
                    instrs: bb_instrs,
                    terminator: bb_terminator,
                },
                BasicBlock {
                    name: _,
                    instrs: tgt_instrs,
                    terminator: tgt_terminator,
                },
            ] = &mut bbs.get_disjoint_mut([bb_key, target_bb]).unwrap();

            scratch_map.insert(target_bb, bb_key);
            bb_instrs.extend_from_slice(tgt_instrs);
            std::mem::swap(bb_terminator, tgt_terminator);

            remapped_jump = true;
            bbs.remove(target_bb);
        }
    }

    if remapped_jump {
        scratch_stack.push(new_entry);
        scratch_seen.clear();
        scratch_seen.insert(new_entry);

        while let Some(bb_key) = scratch_stack.pop() {
            bbs[bb_key].map_bbs(&scratch_map);
            bbs[bb_key]
                .terminator
                .extend_next_rev(scratch_stack, scratch_seen);
        }
    }

    new_entry
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
        let BasicBlock {
            name: _,
            instrs,
            terminator,
        } = &mut bbs[bb_key];

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

                    (
                        *dst,
                        match op {
                            UnaryOp::ReduceOr => Bits::from(bits.reduce_or()),
                            UnaryOp::ReduceAnd => Bits::from(bits.reduce_and()),
                            UnaryOp::ReduceXor => Bits::from(bits.reduce_xor()),
                            UnaryOp::Neg => bits.bitwise_negate(),
                        },
                    )
                }
                I::Resize(dst, op, src) => {
                    let Some(bits) = scratch_map.get(src) else {
                        continue;
                    };

                    let target_size = vars[*dst].size;
                    (
                        *dst,
                        match op {
                            ResizeOp::ZeroExtend => bits.zero_extend(target_size),
                            ResizeOp::SignExtend => bits.sign_extend(target_size),
                            ResizeOp::Truncate => bits.truncate(target_size),
                        },
                    )
                }
                I::Binary(dst, op, src1, src2) => {
                    let csrc1 = scratch_map.get(src1);
                    let csrc2 = scratch_map.get(src2);

                    use BinaryOp as O;
                    (
                        *dst,
                        match (csrc1, csrc2) {
                            (Some(src1), Some(src2)) => match op {
                                O::And => Bits::bitwise_and(src1, src2),
                                O::Or => Bits::bitwise_or(src1, src2),
                                O::Xor => Bits::bitwise_xor(src1, src2),
                                O::Add => Bits::add(src1, src2),
                                O::Sub => Bits::subtract(src1, src2),
                                O::Multiply => Bits::multiply(src1, src2),
                                O::Divide => Bits::divide(src1, src2),
                                O::Modulus => Bits::modulus(src1, src2),
                                O::UnsignedLessEqual => {
                                    Bits::from(Bits::is_unsigned_leq(src1, src2))
                                }
                                O::SelectBit
                                | O::LogicalShiftLeft
                                | O::LogicalShiftRight
                                | O::ArithmeticShiftRight => continue,
                                O::Concat => Bits::concatenate(src1, src2),
                            },
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
                                    O::UnsignedLessEqual
                                    | O::Modulus
                                    | O::SelectBit
                                    | O::LogicalShiftLeft
                                    | O::LogicalShiftRight
                                    | O::ArithmeticShiftRight
                                    | O::Concat => continue,
                                }
                            }
                            (None, None) => continue,
                        },
                    )
                }
                I::Intrinsic(_, _, _) => continue,
                I::Probe(_, _) => continue,
                I::Drive(_, _, _, _) => continue,
                I::Phi(_, _) => continue,
            };
            scratch_map.insert(dst, bits.clone());
            *i = I::Constant(dst, bits);

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
            }

            terminator.extend_next_rev(scratch_stack, scratch_seen);
        }
    }

    while let Some(bb_key) = scratch_mfr.pop() {
        if !scratch_seen.insert(bb_key) {
            continue;
        }
        bbs[bb_key]
            .terminator
            .extend_next_rev(scratch_mfr, scratch_seen);
        bbs.remove(bb_key);
        scratch_removed.insert(bb_key);
    }

    if !scratch_removed.is_empty() {
        scratch_seen.clear();
        scratch_stack.push(entry);
        scratch_seen.insert(entry);

        while let Some(bb_key) = scratch_stack.pop() {
            let BasicBlock {
                name: _,
                instrs,
                terminator,
            } = &mut bbs[bb_key];

            instrs.retain_mut(|i| {
                if let Instruction::Phi(dst, origins) = i {
                    let new_origins = origins
                        .iter()
                        .filter(|(bb, _)| !scratch_removed.contains(bb))
                        .copied()
                        .collect::<Box<_>>();
                    if new_origins.len() == 1
                        && let Some((_, src)) = new_origins.first()
                    {
                        scratch_var_to_var_map.insert(*dst, *src);
                        return false;
                    }
                    *origins = new_origins;
                }
                true
            });
            terminator.extend_next_rev(scratch_stack, scratch_seen);
        }
    }
    if !scratch_var_to_var_map.is_empty() {
        scratch_seen.clear();
        scratch_stack.push(entry);
        scratch_seen.insert(entry);

        while let Some(bb_key) = scratch_stack.pop() {
            let BasicBlock {
                name: _,
                instrs,
                terminator,
            } = &mut bbs[bb_key];

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
        let BasicBlock {
            name: _,
            instrs,
            terminator,
        } = &bbs[bb_key];
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
        let BasicBlock {
            name: _,
            instrs,
            terminator,
        } = &mut bbs[bb_key];
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
