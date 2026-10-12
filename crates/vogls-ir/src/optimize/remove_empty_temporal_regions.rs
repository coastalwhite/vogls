use vogls_utils::{VgHashMap, VgHashSet};

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, GlobalContext, ProcessKey, TemporalRegionKey,
    Time,
};

pub fn remove_empty_temporal_regions(
    gl: &mut GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_map: &mut VgHashMap<BasicBlockKey, BasicBlockKey>,
    scratch_path: &mut VgHashSet<BasicBlockKey>,
) {
    let regions = &mut gl.processes[process].regions;

    // Leave the process alone if there is only one temporal region.
    if regions.len() == 1 {
        return;
    }

    // Find all temporal regions that are empty and redirect to another temporal region.
    scratch_map.clear();
    let start_tr = regions[0];
    regions.retain(|tr| {
        let bb = &gl.bbs[tr.entry()];

        if bb.instrs.is_empty()
            && let BasicBlockTerminator::Wait(tgt, Time(0)) = bb.terminator
        {
            gl.bbs.remove(tr.entry());
            scratch_map.insert(tr.entry(), tgt.entry());
            false
        } else {
            true
        }
    });

    // Fast path: no empty temporal regions.
    if scratch_map.is_empty() {
        return;
    }

    // If there was some loop, all the temporal regions in that loop were removed. Insert a looping
    // temporal region to fix the process again.
    {
        scratch_stack.clear();
        scratch_seen.clear();

        // Find all loops.
        for &pred in scratch_map.keys() {
            if scratch_seen.contains(&pred) {
                continue;
            }

            scratch_path.clear();
            let mut cur = pred;
            while scratch_seen.insert(cur)
                && let Some(&next) = scratch_map.get(&cur)
            {
                scratch_path.insert(cur);
                cur = next;
            }
            if scratch_map.contains_key(&cur) && scratch_path.contains(&cur) {
                scratch_stack.push(pred);
            }
        }

        // Map all temporal regions involved in loops to a new temporal region.
        for bb_key in scratch_stack.iter() {
            let new_start_key = gl.bbs.insert(BasicBlock {
                instrs: Vec::new(),

                // Overwritten when we have the key.
                region: TemporalRegionKey::default(),
                terminator: BasicBlockTerminator::Halt,
            });
            let new_tr_key = TemporalRegionKey::from_entry(new_start_key);
            let bb = &mut gl.bbs[new_start_key];
            bb.region = new_tr_key;
            bb.terminator = BasicBlockTerminator::Jump(new_start_key);
            regions.push(new_tr_key);

            let mut next = *bb_key;
            while let Some(suc) = scratch_map.get_mut(&next) {
                next = *suc;
                *suc = new_start_key;
                if next == *bb_key {
                    break;
                }
            }
        }
    }

    // Remap all the temporal region references.
    for tr in regions.as_slice() {
        scratch_stack.clear();
        scratch_seen.clear();

        scratch_seen.insert(tr.entry());
        scratch_stack.push(tr.entry());

        while let Some(bb_key) = scratch_stack.pop() {
            let terminator = &gl.bbs[bb_key].terminator;
            gl.bbs[bb_key].terminator.for_each_non_temporal_bb(|tgt| {
                if scratch_seen.insert(tgt) {
                    scratch_stack.push(tgt);
                }
            });

            if terminator.is_temporal() {
                gl.bbs[bb_key].terminator.map_temporal_bb(|tgt| {
                    let mut cur = tgt;
                    while let Some(&next) = scratch_map.get(&cur) {
                        cur = next;
                    }
                    cur
                });
            }
        }
    }

    // Fix-up the starting temporal region.
    if regions[0] != start_tr {
        let mut new_start_tr = start_tr.entry();
        while let Some(&next) = scratch_map.get(&new_start_tr) {
            new_start_tr = next;
        }

        let new_start_tr_idx = regions
            .iter()
            .position(|tr| tr.entry() == new_start_tr)
            .expect("This is the final TR in the chain, it should not have been removed.");
        regions.swap(0, new_start_tr_idx);
    }
}
