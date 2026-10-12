use hashbrown::hash_map::Entry;
use vogls_utils::{VgHashMap, VgHashSet};

use crate::{BasicBlockKey, BasicBlockTerminator, GlobalContext, Instruction, ProcessKey};

pub fn merge_lone_jump_targets(
    gl: &mut GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_num_pred: &mut VgHashMap<BasicBlockKey, u32>,
    scratch_map: &mut VgHashMap<BasicBlockKey, BasicBlockKey>,
) {
    for tr in &gl.processes[process].regions {
        // Create a map of basic blocks to the number of fan-in.
        scratch_stack.clear();
        scratch_num_pred.clear();
        scratch_stack.push(tr.entry());
        scratch_num_pred.insert(tr.entry(), 1);
        while let Some(bb_key) = scratch_stack.pop() {
            gl.bbs[bb_key].terminator.for_each_non_temporal_bb(|tgt| {
                match scratch_num_pred.entry(tgt) {
                    Entry::Occupied(mut entry) => *entry.get_mut() += 1,
                    Entry::Vacant(entry) => {
                        entry.insert(1);
                        scratch_stack.push(tgt);
                    }
                }
            });
        }

        // Find the lone jumps and merge successor into the predecessor.
        scratch_seen.clear();
        scratch_map.clear();
        scratch_stack.push(tr.entry());
        scratch_seen.insert(tr.entry());
        while let Some(bb_key) = scratch_stack.pop() {
            while let BasicBlockTerminator::Jump(tgt) = &gl.bbs[bb_key].terminator
                && let tgt = *tgt
                && scratch_num_pred[&tgt] == 1
            {
                let Some(suc) = gl.bbs.remove(tgt) else {
                    if cfg!(debug_assertions) {
                        panic!("This block should still exist");
                    }
                    break;
                };
                scratch_map.insert(tgt, bb_key);

                let pred = &mut gl.bbs[bb_key];
                pred.instrs.extend(suc.instrs);
                pred.terminator = suc.terminator;
            }

            gl.bbs[bb_key].terminator.for_each_non_temporal_bb(|tgt| {
                if scratch_seen.insert(tgt) {
                    scratch_stack.push(tgt);
                }
            });
        }

        // No lone jumps. Early exit.
        if scratch_map.is_empty() {
            continue;
        }

        // Fix-up the phi instructions.
        scratch_seen.clear();
        scratch_stack.push(tr.entry());
        scratch_seen.insert(tr.entry());
        while let Some(bb_key) = scratch_stack.pop() {
            let bb = &mut gl.bbs[bb_key];
            bb.terminator.for_each_non_temporal_bb(|tgt| {
                if scratch_seen.insert(tgt) {
                    scratch_stack.push(tgt);
                }
            });

            for instr in bb.instrs.iter_mut() {
                let Instruction::Phi(dst, srcs) = instr else {
                    continue;
                };

                for (pred_bb_key, _) in srcs.iter_mut() {
                    *pred_bb_key = scratch_map
                        .get(pred_bb_key)
                        .copied()
                        .unwrap_or(*pred_bb_key);
                }
                if let [(key, src)] = srcs.as_ref()
                    && *key == bb_key
                {
                    // If there is a phi instruction in the successor that only referenced the
                    // predecessor, just turn it into a copy.
                    *instr = Instruction::copy(&gl.vars, *dst, *src);
                }
            }
        }
    }
}
