mod heap;

use std::ops::Range;

use hashbrown::hash_map::Entry;
pub use heap::{Heap, HeapBuilder, HeapOffset, HeapRef};
use vogls_ir::{BasicBlockKey, GlobalContext, Instruction, LogicMode, VariableKey};
use vogls_utils::{VgHashMap, VgHashSet};

/// Fill `var_mode` with the `LogicMode` for each variable present in the control-flow graph.
pub fn resolve_var_logic_mode_map(
    entry: BasicBlockKey,
    gl: &GlobalContext,
    bb_stack: &mut Vec<BasicBlockKey>,
    bb_seen: &mut VgHashSet<BasicBlockKey>,
    var_mode: &mut VgHashMap<VariableKey, LogicMode>,
    conv_map: &mut VgHashMap<VariableKey, HeapOffset>,
) {
    macro_rules! mark_conv {
        ($var:expr) => {
            conv_map.insert($var, HeapOffset { bit_offset: 0 })
        };
    }

    // Variable need to need a converted equivalent if they don't match the specific mode. This is
    // used for variables which have not yet been seen in the linear traversal, but have been
    // assumed to be of a certain mode.
    let mut maybe_mark_conv_later = Vec::<(VariableKey, LogicMode)>::new();

    // A directed multi-graph of the variables for which the mode could not immediately resolved
    // with edges representing which variables their mode depends on. This can be case if phi
    // instructions are encountered that references basic blocks that have not yet been traversed
    // (e.g. in loops). Note, these might be cyclic. These need to be resolved separatedly
    // afterwards.
    //
    // Represented as a map from node to range in a continous list of edges.
    let mut graph_offsets = VgHashMap::<VariableKey, Range<usize>>::default();
    let mut graph_inputs = Vec::<VariableKey>::new();
    let mut is_fixed = VgHashSet::<VariableKey>::default();

    bb_seen.clear();
    bb_seen.insert(entry);
    bb_stack.clear();
    bb_stack.push(entry);

    while let Some(bb_key) = bb_stack.pop() {
        let bb = &gl.bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_seen.insert(bb) {
                bb_stack.push(bb);
            }
        });

        use Instruction as I;
        for instr in &bb.instrs {
            match instr {
                I::Constant(dst, bits) => {
                    var_mode.insert(
                        *dst,
                        if bits.contains_special() {
                            LogicMode::FourValue
                        } else {
                            LogicMode::TwoValue
                        },
                    );
                }
                I::Unary(dst, _, src) | I::Resize(dst, _, src) => {
                    match var_mode.get(src).copied() {
                        Some(m) => _ = var_mode.insert(*dst, m),
                        None => {
                            graph_offsets.insert(*dst, graph_inputs.len()..graph_inputs.len() + 1);
                            graph_inputs.push(*src)
                        }
                    }
                }
                I::Binary(dst, op, lhs, rhs) => {
                    let m1 = var_mode.get(lhs).copied();
                    let m2 = var_mode.get(rhs).copied();

                    use LogicMode as M;
                    match (m1, m2, op) {
                        (_, _, op) if op.always_outputs_bool() => {
                            if m1.is_none() || m2.is_none() {
                                is_fixed.insert(*dst);
                            }
                            _ = var_mode.insert(*dst, M::TwoValue)
                        }
                        (_, _, op) if op.always_outputs_four_value() => {
                            if m1.is_none() || m2.is_none() {
                                is_fixed.insert(*dst);
                            }
                            _ = var_mode.insert(*dst, M::FourValue)
                        }
                        (Some(M::TwoValue), Some(M::TwoValue), _) => {
                            _ = var_mode.insert(*dst, M::TwoValue)
                        }
                        (Some(M::FourValue), _, _) | (_, Some(M::FourValue), _) => {
                            _ = var_mode.insert(*dst, M::FourValue)
                        }
                        _ => {}
                    }

                    match (m1, m2) {
                        (Some(M::TwoValue), Some(M::TwoValue))
                        | (Some(M::FourValue), Some(M::FourValue)) => {}

                        (Some(M::FourValue), None) => {
                            maybe_mark_conv_later.push((*rhs, M::FourValue))
                        }
                        (Some(M::FourValue), Some(M::TwoValue)) => _ = mark_conv!(*rhs),
                        (None, Some(M::FourValue)) => {
                            maybe_mark_conv_later.push((*lhs, M::FourValue))
                        }
                        (Some(M::TwoValue), Some(M::FourValue)) => _ = mark_conv!(*lhs),

                        (Some(_), None) => {
                            graph_offsets.insert(*dst, graph_inputs.len()..graph_inputs.len() + 1);
                            graph_inputs.push(*rhs);
                        }
                        (None, Some(_)) => {
                            graph_offsets.insert(*dst, graph_inputs.len()..graph_inputs.len() + 1);
                            graph_inputs.push(*lhs);
                        }
                        (None, None) => {
                            graph_offsets.insert(*dst, graph_inputs.len()..graph_inputs.len() + 2);
                            graph_inputs.extend([*lhs, *rhs]);
                        }
                    }
                }
                I::Intrinsic(dst, _, _) => _ = var_mode.insert(*dst, LogicMode::TwoValue),
                I::LastUpdateTime(dst, _) => _ = var_mode.insert(*dst, LogicMode::TwoValue),
                I::Probe(dst, _) => {
                    var_mode.insert(*dst, gl.logic_mode);
                }
                I::Drive(_, src, partial) => {
                    match var_mode.get(src) {
                        Some(m) if *m != gl.logic_mode => _ = mark_conv!(*src),
                        Some(_) => {}
                        None => {
                            maybe_mark_conv_later.push((*src, gl.logic_mode));
                        }
                    }

                    if let Some((offset, _)) = partial {
                        match var_mode.get(offset) {
                            Some(m) if *m != gl.logic_mode => _ = mark_conv!(*offset),
                            Some(_) => {}
                            None => {
                                maybe_mark_conv_later.push((*offset, gl.logic_mode));
                            }
                        }
                    }
                }
                I::Phi(dst, items) => {
                    let mut logic_mode = Some(LogicMode::TwoValue);
                    for (_, v) in items {
                        match var_mode.get(v) {
                            None => logic_mode = None,
                            Some(LogicMode::TwoValue) => {}
                            Some(LogicMode::FourValue) => {
                                logic_mode = Some(LogicMode::FourValue);
                                break;
                            }
                        }
                    }
                    if let Some(logic_mode) = logic_mode {
                        if logic_mode == LogicMode::FourValue {
                            for (_, v) in items {
                                match var_mode.get(v) {
                                    None => maybe_mark_conv_later.push((*v, LogicMode::FourValue)),
                                    Some(&LogicMode::TwoValue) => _ = mark_conv!(*v),
                                    Some(&LogicMode::FourValue) => {}
                                }
                            }
                        }
                        var_mode.insert(*dst, logic_mode);
                    } else {
                        graph_offsets
                            .insert(*dst, graph_inputs.len()..graph_inputs.len() + items.len());
                        graph_inputs.extend(items.iter().map(|(_, v)| *v));
                    }
                }
            }
        }
    }

    // For all the variable which could not immediately
    //
    // @TODO: This is current pessimistic because if performs a depth-first search and marks all
    // reachable node with the same mode. It should perform a bread-first search and mark only
    // according to what is actually reachable from a node. This is a bit more complex and probably
    // is quite rare so I leave it as a future problem.
    let mut seen = VgHashSet::default();
    let mut stack = Vec::new();
    while let Some(&fst) = graph_offsets.keys().next() {
        seen.clear();

        seen.insert(fst);
        stack.push(fst);

        let mut seen_fv = false;
        while let Some(k) = stack.pop() {
            if let Some(m) = var_mode.get(&k) {
                seen_fv |= matches!(m, LogicMode::FourValue);
            }

            if let Some(neighbours_range) = graph_offsets.get(&k) {
                for i in neighbours_range.clone() {
                    let neighbour = graph_inputs[i];
                    if seen.insert(neighbour) {
                        stack.push(neighbour);
                    }
                }
            }
        }

        let mode = if seen_fv {
            LogicMode::FourValue
        } else {
            LogicMode::TwoValue
        };
        seen.iter().for_each(|k| match var_mode.entry(*k) {
            Entry::Vacant(entry) => _ = entry.insert(mode),
            Entry::Occupied(entry) => {
                if *entry.get() != mode && !is_fixed.contains(k) {
                    _ = mark_conv!(*k);
                }
            }
        });
        graph_offsets.retain(|k, _| !seen.contains(k));
    }

    for (v, m) in maybe_mark_conv_later {
        if m != var_mode[&v] {
            mark_conv!(v);
        }
    }
}

pub fn resolve_heap_map(
    entry: BasicBlockKey,
    gl: &GlobalContext,
    bb_stack: &mut Vec<BasicBlockKey>,
    bb_seen: &mut VgHashSet<BasicBlockKey>,
    var_mode: &VgHashMap<VariableKey, LogicMode>,
    conv_map: &mut VgHashMap<VariableKey, HeapOffset>,
    heap_builder: &mut HeapBuilder,
    heap_map: &mut VgHashMap<VariableKey, HeapOffset>,
    bb_phis: &mut VgHashMap<BasicBlockKey, Vec<(VariableKey, VariableKey)>>,
    temporal_variables: Option<&VgHashSet<VariableKey>>,
) {
    bb_stack.clear();

    // Make a map of the heap.
    for (min_bits, max_bits) in [
        (33, u32::MAX),
        (17, 32),
        (9, 16),
        (5, 8),
        (3, 4),
        (2, 2),
        (1, 1),
    ] {
        bb_seen.clear();
        bb_stack.push(entry);
        while let Some(bb_key) = bb_stack.pop() {
            let bb = gl.bbs.get(bb_key).unwrap();

            for instr in &bb.instrs {
                if let Instruction::Phi(dst, srcs) = instr {
                    for (bb, var) in srcs {
                        bb_phis.entry(*bb).or_insert(Vec::new()).push((*dst, *var));
                    }
                }

                if let Some(dst) = instr.get_destination_variable() {
                    let mode = var_mode[&dst];
                    let size = gl.vars[dst].size;

                    let mut num_bits = size.get();
                    if mode == LogicMode::FourValue {
                        num_bits = num_bits * 2;
                    }

                    if temporal_variables.is_none_or(|t| t.contains(&dst)) {
                        if (min_bits..=max_bits).contains(&num_bits) {
                            let prev = heap_map.insert(dst, heap_builder.claim(mode, size).offset);
                            assert!(prev.is_none());

                            if let Some(heap_ref) = conv_map.get_mut(&dst) {
                                let other_mode = match mode {
                                    LogicMode::TwoValue => LogicMode::FourValue,
                                    LogicMode::FourValue => LogicMode::TwoValue,
                                };
                                *heap_ref = heap_builder.claim(other_mode, size).offset;
                            }
                        }
                    }
                }
            }

            bb_seen.insert(entry);
            bb.terminator.for_each_bb(|bb| {
                if bb_seen.insert(bb) {
                    bb_stack.push(bb);
                }
            });
        }
    }
}
