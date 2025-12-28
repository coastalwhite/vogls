use std::collections::{HashMap, HashSet};

use slotmap::SlotMap;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, Bits, Instruction, UnaryOp, VariableKey,
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

    while let Some(bb_key) = scratch_stack.pop() {
        if let BasicBlockTerminator::Jump(target_bb) = &mut bbs[bb_key].terminator
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

            bb_instrs.extend_from_slice(tgt_instrs);
            std::mem::swap(bb_terminator, tgt_terminator);
            bbs.remove(target_bb);
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
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_mfr: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut HashSet<BasicBlockKey>,
    scratch_map: &mut HashMap<VariableKey, Bits>,
) {
    scratch_stack.clear();
    scratch_mfr.clear();
    scratch_seen.clear();
    scratch_map.clear();

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
                            UnaryOp::ReduceOr(_) => Bits::from(bits.count_ones() > 0),
                            UnaryOp::ReduceAnd(_) => {
                                Bits::from(bits.count_ones() == bits.size().get())
                            }
                            UnaryOp::ReduceXor(_) => Bits::from(bits.count_ones() % 2 == 1),
                            UnaryOp::ZeroExtend(new_size, _) => bits.zero_extend(*new_size),
                            UnaryOp::SignExtend(new_size, _) => bits.sign_extend(*new_size),
                            UnaryOp::Neg(_) | UnaryOp::Slice(_, _) => continue,
                        },
                    )
                }
                I::Binary(_, _, _, _) => continue,
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
    }
}
