use vogls_utils::{VgHashMap, VgHashSet};

use crate::{BasicBlockKey, GlobalContext, ProcessKey, VariableKey};

/// An optimization pass that removes unused instructions.
///
/// The optimization pass looks for variables that are unused in the process and then removes
/// instructions that generate those variables. A variable may transitively be unused when all the
/// variables that depend on it are unused.
///
/// The pass does this in the following way:
/// 1. Form a directed graph. Nodes are the variables and edges are the variables who depend on it.
/// 2. Perform depth-first traversals for all the possibly unused variables. Invariant: if all
///    neighbors are unused, the variable is unused.
/// 3. Filter out all instructions that create the unused instructions.
pub fn deadcode_elimination(
    gl: &mut GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let entry = gl.processes[process].entry;

    enum VariableState {
        Used,
        Unused,
        MaybeUnused(usize),
    }

    // @Performance. Scratchpad these?
    let mut nodes_to_visit = Vec::new();
    let mut nodes = VgHashMap::<VariableKey, VariableState>::default();
    let mut edges = Vec::new();

    // @Performance. It might be interesting to make the instruction iteration a post-visit action.
    // This would ensure all the fanout basic-blocks are added first. This would allow variables
    // that see use across basic-block barriers to be marked as MaybeUnused a lot less frequently.
    //
    // That way, if the code is linear you never have to go into the DFS part.
    scratch_seen.clear();
    scratch_stack.clear();
    scratch_stack.push(entry);
    while let Some(bb_key) = scratch_stack.pop() {
        let bb = &gl.bbs[bb_key];
        bb.terminator.for_each_bb(|next| {
            if scratch_seen.insert(next) {
                scratch_stack.push(next);
            }
        });
        bb.terminator
            .for_each_var(|v| _ = nodes.insert(v, VariableState::Used));
        for i in bb.instrs.iter().rev() {
            if !i.has_side_effects_on_call()
                && let Some(dst) = i.get_destination_variable()
                && nodes
                    .get(&dst)
                    .is_none_or(|s| matches!(s, VariableState::MaybeUnused(_)))
            {
                nodes.insert(dst, VariableState::MaybeUnused(0));
                nodes_to_visit.push(dst);
                i.for_each_src(|v| edges.push((v, dst)));
            } else {
                i.for_each_src(|v| _ = nodes.insert(v, VariableState::Used));
            }
        }
    }

    if nodes_to_visit.is_empty() {
        return;
    }

    // Make the edges continuous for all nodes and set their start point.
    if !edges.is_empty() {
        edges.sort_unstable_by_key(|(src, _)| *src);
        let mut current = edges[0].0;
        for (i, (src, _)) in edges.iter().enumerate() {
            if *src != current {
                if let Some(VariableState::MaybeUnused(offset)) = nodes.get_mut(src) {
                    *offset = i;
                }
                current = *src;
            }
        }
    }

    // Perform a depth-first search (DFS) on every potentially unused variable. If the variable is
    // a dependency of a used variable, the variable itself is also used. This is a post-visit
    // condition on the DFS.
    let mut var_stack = Vec::new();
    for var in nodes_to_visit {
        if !matches!(nodes[&var], VariableState::MaybeUnused(_)) {
            continue;
        }

        var_stack.push((false, var));
        'stack_loop: while let Some((post_visit, var)) = var_stack.pop() {
            let VariableState::MaybeUnused(start) = nodes[&var] else {
                continue;
            };

            // If the variable is a dependency of a used variable, the variable itself is also
            // used.
            let mut num_maybe_unused = 0;
            for &(src, dst) in &edges[start..] {
                if src != var {
                    break;
                }
                let dst_state = &nodes[&dst];
                num_maybe_unused += usize::from(matches!(dst_state, VariableState::MaybeUnused(_)));
                if matches!(dst_state, VariableState::Used) {
                    nodes.insert(var, VariableState::Used);
                    continue 'stack_loop;
                }
            }

            // If the variable is a dependency of only unused variables, the variable itself is
            // also unused.
            if num_maybe_unused == 0 || post_visit {
                _ = nodes.insert(var, VariableState::Unused);
                continue;
            }

            var_stack.push((true, var));
            var_stack.extend(
                edges[start..]
                    .iter()
                    .take_while(|(src, _)| *src == var)
                    .filter_map(|(_, dst)| {
                        matches!(nodes[dst], VariableState::MaybeUnused(_)).then_some((false, *dst))
                    }),
            );
        }
    }

    // Turn nodes into a hashset of unused variables.
    nodes.retain(|var, state| {
        if matches!(state, VariableState::Unused) {
            gl.vars.remove(*var);
            return true;
        }

        false
    });

    // Remove all instructions that generate unused variables.
    scratch_seen.clear();
    scratch_stack.clear();
    scratch_stack.push(entry);
    while let Some(bb_key) = scratch_stack.pop() {
        let bb = &mut gl.bbs[bb_key];
        bb.terminator.for_each_bb(|next| {
            if scratch_seen.insert(next) {
                scratch_stack.push(next);
            }
        });
        bb.instrs.retain(|i| {
            i.has_side_effects_on_call()
                || i.get_destination_variable()
                    .is_none_or(|n| !nodes.contains_key(&n))
        });
        bb.instrs.shrink_to_fit();
    }
}
