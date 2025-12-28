use std::collections::{HashMap, HashSet};

use slotmap::SlotMap;

use crate::{BasicBlock, BasicBlockKey, BasicBlockTerminator};

pub fn remove_needless_jumps(
    bbs: &mut SlotMap<BasicBlockKey, BasicBlock>,
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_map: &mut HashMap<BasicBlockKey, BasicBlockKey>,
    scratch_seen: &mut HashSet<BasicBlockKey>,
) -> BasicBlockKey {
    scratch_stack.clear();
    scratch_map.clear();
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

    while let Some(bb_key) = scratch_stack.pop() {
        bbs[bb_key].map_bbs(&scratch_map);
        bbs[bb_key]
            .terminator
            .extend_next_rev(scratch_stack, scratch_seen);
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
