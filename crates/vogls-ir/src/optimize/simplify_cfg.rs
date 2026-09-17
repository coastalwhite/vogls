//! Control-flow cleanups.
//!
//! The rest of the optimizer works on instructions: nothing else removes a *block* or a *temporal
//! region*, so the scaffolding left behind by structured lowering survives all the way into the
//! emitted code, where each leftover edge costs a jump (bytecode) or a tail call (Cranelift).
//!
//! Two shapes show up in every design:
//!
//! - An `if`/`else` leaves join blocks and empty arms: a block with no instructions whose only job
//!   is to `Jump` to the real successor.
//! - `temporal_jump_to` is `Wait(tr, 0)`, so a region that only exists to hand over to the next one
//!   is an entry block with no instructions and a zero wait.

use vogls_utils::{VgHashMap, VgHashSet};

use crate::{
    BasicBlockKey, BasicBlockTerminator, GlobalContext, Instruction, ProcessKey, TemporalRegionKey,
    Time,
};

/// Follow `redirect` to a fixed point. Returns `None` if the chain cycles, which would be a loop of
/// empty blocks; collapsing it would change an infinite loop into something else, so leave it be.
fn resolve<K: Copy + Eq + std::hash::Hash>(start: K, redirect: &VgHashMap<K, K>) -> Option<K> {
    let mut cur = start;
    for _ in 0..=redirect.len() {
        match redirect.get(&cur) {
            None => return Some(cur),
            Some(&next) => cur = next,
        }
    }
    None
}

/// Collect every basic block reachable in `process`, following non-temporal edges from each region
/// entry.
fn collect_blocks(
    gl: &GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) -> Vec<BasicBlockKey> {
    let mut blocks = Vec::new();
    scratch_seen.clear();
    for tr in &gl.processes[process].regions {
        scratch_stack.clear();
        let entry = tr.entry();
        if scratch_seen.insert(entry) {
            scratch_stack.push(entry);
        }
        while let Some(k) = scratch_stack.pop() {
            blocks.push(k);
            gl.bbs[k].terminator.for_each_non_temporal_bb(|s| {
                if scratch_seen.insert(s) {
                    scratch_stack.push(s);
                }
            });
        }
    }
    blocks
}

/// Thread jumps through empty basic blocks, then drop them.
pub fn thread_empty_blocks(
    gl: &mut GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let blocks = collect_blocks(gl, process, scratch_stack, scratch_seen);

    // A `Phi` names its predecessor blocks explicitly, so threading a block it refers to would need
    // the phi rewritten too. They are rare enough that skipping them costs essentially nothing.
    let mut blocked = VgHashSet::default();
    for &k in &blocks {
        for i in &gl.bbs[k].instrs {
            if let Instruction::Phi(_, srcs) = i {
                for (bb, _) in srcs.iter() {
                    blocked.insert(*bb);
                }
            }
        }
    }
    // Region entries are addressed by `TemporalRegionKey`, so they are not ours to remove here.
    for tr in &gl.processes[process].regions {
        blocked.insert(tr.entry());
    }

    let mut redirect = VgHashMap::<BasicBlockKey, BasicBlockKey>::default();
    for &k in &blocks {
        if blocked.contains(&k) || !gl.bbs[k].instrs.is_empty() {
            continue;
        }
        if let BasicBlockTerminator::Jump(target) = gl.bbs[k].terminator {
            redirect.insert(k, target);
        }
    }
    if redirect.is_empty() {
        return;
    }

    let mut final_map = VgHashMap::<BasicBlockKey, BasicBlockKey>::default();
    for (&from, _) in redirect.iter() {
        if let Some(to) = resolve(from, &redirect) {
            final_map.insert(from, to);
        }
    }
    if final_map.is_empty() {
        return;
    }

    for &k in &blocks {
        let remap = |bb: &mut BasicBlockKey| {
            if let Some(&to) = final_map.get(bb) {
                *bb = to;
            }
        };
        match &mut gl.bbs[k].terminator {
            BasicBlockTerminator::Jump(bb) => remap(bb),
            BasicBlockTerminator::Branch(_, truthy, falsy) => {
                remap(truthy);
                remap(falsy);
            }
            BasicBlockTerminator::Wait(..)
            | BasicBlockTerminator::VariableWait(..)
            | BasicBlockTerminator::WaitRegion(..)
            | BasicBlockTerminator::Watch(..)
            | BasicBlockTerminator::Halt => {}
        }
    }

    // Nothing refers to them any more: they were not region entries and every non-temporal edge was
    // just rewritten past them.
    for from in final_map.keys() {
        gl.bbs.remove(*from);
    }
}

/// Merge a block into its predecessor when that predecessor is its only one.
///
/// Where [`thread_empty_blocks`] rewrites edges *past* a block that carries nothing,
/// this absorbs a block that carries real instructions but is only ever entered one way: if `A`
/// ends in `Jump(B)` and `A` is `B`'s only predecessor, `B`'s instructions move into `A` and `A`
/// takes `B`'s terminator. The jump disappears, and the bigger block gives the per-block passes
/// (CSE, peephole) more to work with on the next round.
pub fn merge_lone_jump_targets(
    gl: &mut GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let blocks = collect_blocks(gl, process, scratch_stack, scratch_seen);

    let mut predecessors = VgHashMap::<BasicBlockKey, u32>::default();
    for &k in &blocks {
        gl.bbs[k].terminator.for_each_non_temporal_bb(|s| {
            *predecessors.entry(s).or_insert(0) += 1;
        });
    }

    let mut blocked = VgHashSet::default();
    for &k in &blocks {
        for i in &gl.bbs[k].instrs {
            if let Instruction::Phi(_, srcs) = i {
                // A phi inside the absorbed block would end up naming the block it now lives in,
                // and one in a successor would still name the block that is going away.
                blocked.insert(k);
                for (bb, _) in srcs.iter() {
                    blocked.insert(*bb);
                }
            }
        }
    }
    for tr in &gl.processes[process].regions {
        blocked.insert(tr.entry());
    }

    let mut removed = VgHashSet::default();
    for &a in &blocks {
        if removed.contains(&a) {
            continue;
        }
        // Absorbing `b` can expose another lone jump, so keep going until `a` ends in something
        // else. Only `a` can ever absorb `b`, since `b` has exactly one predecessor.
        while let BasicBlockTerminator::Jump(b) = gl.bbs[a].terminator {
            if b == a
                || removed.contains(&b)
                || blocked.contains(&b)
                || predecessors.get(&b).copied() != Some(1)
            {
                break;
            }
            let Some(block) = gl.bbs.remove(b) else {
                break;
            };
            gl.bbs[a].instrs.extend(block.instrs);
            gl.bbs[a].terminator = block.terminator;
            removed.insert(b);
        }
    }
}

/// Drop temporal regions that only hand over to the next one.
pub fn remove_passthrough_regions(
    gl: &mut GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let regions = gl.processes[process].regions.clone();

    let mut redirect = VgHashMap::<TemporalRegionKey, TemporalRegionKey>::default();
    for &tr in &regions {
        let entry = tr.entry();
        let bb = &gl.bbs[entry];
        if !bb.instrs.is_empty() {
            continue;
        }
        if let BasicBlockTerminator::Wait(next, Time(0)) = bb.terminator
            && next != tr
        {
            redirect.insert(tr, next);
        }
    }
    if redirect.is_empty() {
        return;
    }

    let mut final_map = VgHashMap::<TemporalRegionKey, TemporalRegionKey>::default();
    for (&from, _) in redirect.iter() {
        if let Some(to) = resolve(from, &redirect) {
            final_map.insert(from, to);
        }
    }
    if final_map.is_empty() {
        return;
    }

    // Rewrite every temporal edge in the process, including those inside the regions being kept.
    let blocks = collect_blocks(gl, process, scratch_stack, scratch_seen);
    for &k in &blocks {
        let remap = |tr: &mut TemporalRegionKey| {
            if let Some(&to) = final_map.get(tr) {
                *tr = to;
            }
        };
        match &mut gl.bbs[k].terminator {
            BasicBlockTerminator::Wait(tr, _)
            | BasicBlockTerminator::VariableWait(tr, _)
            | BasicBlockTerminator::WaitRegion(tr, _)
            | BasicBlockTerminator::Watch(tr, _) => remap(tr),
            BasicBlockTerminator::Jump(_)
            | BasicBlockTerminator::Branch(..)
            | BasicBlockTerminator::Halt => {}
        }
    }

    // `regions[0]` is where the process starts, so if the entry itself was a hand-over the region
    // it handed over to has to take its place at the front.
    let entry = final_map
        .get(&regions[0])
        .copied()
        .unwrap_or_else(|| regions[0]);
    let mut new_regions = Vec::with_capacity(regions.len());
    new_regions.push(entry);
    for tr in regions {
        if tr != entry && !final_map.contains_key(&tr) {
            new_regions.push(tr);
        }
    }
    for from in final_map.keys() {
        gl.bbs.remove(from.entry());
    }
    gl.processes[process].regions = new_regions;
}
