use hashbrown::hash_map::Entry;
use slotmap::SlotMap;

use vogls_ir::{BasicBlock, BasicBlockKey, VariableKey};
use vogls_utils::{IndexSet, VgHashMap, VgHashSet};

pub fn get_temporal_regions(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    bb_stack: &mut Vec<BasicBlockKey>,
    bb_seen: &mut VgHashSet<BasicBlockKey>,

    temporal_roots: &mut IndexSet<BasicBlockKey>,
    temporal: &mut VgHashMap<BasicBlockKey, usize>,
) {
    bb_stack.clear();
    bb_seen.clear();
    temporal_roots.clear();
    temporal.clear();

    temporal_roots.insert(entry);
    bb_seen.insert(entry);
    bb_stack.push(entry);

    // Find all initial temporal roots. These are the basic blocks that are a temporal terminator
    // points to.
    while let Some(bb_key) = bb_stack.pop() {
        let bb = &bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_seen.insert(bb) {
                bb_stack.push(bb);
            }
        });

        if bb.terminator.is_temporal() {
            bb.terminator.for_each_bb(|bb| {
                temporal_roots.insert(bb);
            });
        }
    }

    // Iterate all temporal roots and perform a graph traversal from them.
    // - If they find a BB that has not been assigned a temporal region, mark it as the current
    // region.
    // - If they find a BB that has been assigned a temporal region, mark it as a new root if it
    // was not already.
    let mut temporal_region = 0;
    while temporal_region < temporal_roots.len() {
        let &root = temporal_roots.get_at_index(temporal_region).unwrap();

        bb_seen.clear();
        bb_stack.push(root);
        bb_seen.insert(root);
        temporal.insert(root, temporal_region);

        while let Some(bb_key) = bb_stack.pop() {
            let bb = &bbs[bb_key];

            // Only traverse through non-temporal edges.
            if bb.terminator.is_temporal() {
                continue;
            }

            bb.terminator.for_each_bb(|bb| {
                if bb_seen.insert(bb) {
                    match temporal.entry(bb) {
                        Entry::Occupied(_) => _ = temporal_roots.insert(bb),
                        Entry::Vacant(entry) => {
                            entry.insert(temporal_region);
                            bb_stack.push(bb);
                        }
                    }
                }
            });
        }
        temporal_region += 1;
    }
}

pub fn get_temporal_variables(
    entry: BasicBlockKey,
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,

    bb_stack: &mut Vec<BasicBlockKey>,
    bb_seen: &mut VgHashSet<BasicBlockKey>,

    temporal_roots: &mut IndexSet<BasicBlockKey>,
    temporal: &mut VgHashMap<BasicBlockKey, usize>,

    variable_to_tmr_map: &mut VgHashMap<VariableKey, usize>,
    temporal_variables: &mut VgHashSet<VariableKey>,
) {
    get_temporal_regions(entry, bbs, bb_stack, bb_seen, temporal_roots, temporal);

    bb_stack.clear();
    bb_stack.push(entry);
    bb_seen.clear();
    bb_seen.insert(entry);

    // Assign all variables a temporal region in which they were assigned.
    while let Some(bb_key) = bb_stack.pop() {
        let bb = &bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_seen.insert(bb) {
                bb_stack.push(bb);
            }
        });

        let tmr = temporal[&bb_key];
        for i in &bb.instrs {
            if let Some(v) = i.get_destination_variable() {
                variable_to_tmr_map.insert(v, tmr);
            }
        }
    }

    bb_seen.clear();
    bb_seen.insert(entry);
    bb_stack.push(entry);

    // See if all variable uses are in the same region they were defined.
    while let Some(bb_key) = bb_stack.pop() {
        let bb = &bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_seen.insert(bb) {
                bb_stack.push(bb);
            }
        });

        let tmr = temporal[&bb_key];
        bb.for_each_var(|v| {
            if variable_to_tmr_map[&v] != tmr {
                temporal_variables.insert(v);
            }
        });
    }
}
