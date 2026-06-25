mod heap;
pub mod lsra;

use std::ops::Range;

use hashbrown::hash_map::Entry;
pub use heap::{Heap, HeapBuilder, HeapOffset, HeapRef};
use vogls_ir::{
    BasicBlockKey, BinaryImmOp, BinaryOp, Bits, GlobalContext, Instruction, LogicMode,
    TemporalRegionKey, VariableKey,
};
use vogls_utils::{VgHashMap, VgHashSet};

/// Fill `var_mode` with the `LogicMode` for each variable present in the control-flow graph.
pub fn resolve_var_logic_mode_map(
    regions: &[TemporalRegionKey],
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

    for tr in regions {
        graph_offsets.clear();
        graph_inputs.clear();

        bb_seen.clear();
        bb_seen.insert(tr.entry());
        bb_stack.clear();
        bb_stack.push(tr.entry());

        while let Some(bb_key) = bb_stack.pop() {
            let bb = &gl.bbs[bb_key];
            bb.terminator.for_each_non_temporal_bb(|bb| {
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
                    I::Unary(dst, _, src)
                    | I::Resize(dst, _, src)
                    | I::SliceImm(dst, src, _)
                    | I::ShiftImm(dst, _, src, _) => match var_mode.get(src).copied() {
                        Some(m) => _ = var_mode.insert(*dst, m),
                        None => {
                            graph_offsets.insert(*dst, graph_inputs.len()..graph_inputs.len() + 1);
                            graph_inputs.push(*src)
                        }
                    },
                    I::BinaryImm(dst, op, src, imm) => {
                        let m1 = var_mode.get(src).copied();
                        let m2 = if imm.contains_special() {
                            LogicMode::FourValue
                        } else {
                            LogicMode::TwoValue
                        };

                        use LogicMode as M;
                        match (m1, m2) {
                            _ if op.always_outputs_bool() => _ = var_mode.insert(*dst, M::TwoValue),
                            _ if op.always_outputs_four_value() => {
                                _ = var_mode.insert(*dst, M::FourValue)
                            }
                            (Some(M::TwoValue), M::TwoValue) => {
                                _ = var_mode.insert(*dst, M::TwoValue)
                            }
                            (Some(M::FourValue), _) | (_, M::FourValue) => {
                                _ = var_mode.insert(*dst, M::FourValue)
                            }
                            _ => {}
                        }

                        match (m1, m2) {
                            (Some(M::TwoValue), M::TwoValue) | (Some(M::FourValue), _) => {}

                            (None, M::FourValue) => {
                                maybe_mark_conv_later.push((*src, M::FourValue))
                            }
                            (Some(M::TwoValue), M::FourValue) => _ = mark_conv!(*src),

                            (None, _) => {
                                graph_offsets
                                    .insert(*dst, graph_inputs.len()..graph_inputs.len() + 1);
                                graph_inputs.push(*src);
                            }
                        }
                    }
                    I::Binary(dst, op, lhs, rhs) => {
                        let m1 = var_mode.get(lhs).copied();
                        let m2 = var_mode.get(rhs).copied();

                        use LogicMode as M;
                        match (m1, m2) {
                            _ if op.always_outputs_bool() => _ = var_mode.insert(*dst, M::TwoValue),
                            _ if op.always_outputs_four_value() => {
                                _ = var_mode.insert(*dst, M::FourValue)
                            }
                            (Some(M::TwoValue), Some(M::TwoValue)) => {
                                _ = var_mode.insert(*dst, M::TwoValue)
                            }
                            (Some(M::FourValue), _) | (_, Some(M::FourValue)) => {
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

                            (None, _) | (_, None) => {
                                graph_offsets
                                    .insert(*dst, graph_inputs.len()..graph_inputs.len() + 2);
                                graph_inputs.extend([*lhs, *rhs]);
                            }
                        }
                    }
                    I::Slice(dst, lhs, rhs) => {
                        let m1 = var_mode.get(lhs).copied();
                        let m2 = var_mode.get(rhs).copied();

                        use LogicMode as M;
                        var_mode.insert(*dst, M::FourValue);

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

                            (None, _) | (_, None) => {
                                graph_offsets
                                    .insert(*dst, graph_inputs.len()..graph_inputs.len() + 2);
                                graph_inputs.extend([*lhs, *rhs]);
                            }
                        }
                    }
                    I::Select(dst, _cond, truthy, falsy) => {
                        let m1 = var_mode.get(truthy).copied();
                        let m2 = var_mode.get(falsy).copied();

                        use LogicMode as M;
                        match (m1, m2) {
                            (Some(M::TwoValue), Some(M::TwoValue)) => {
                                _ = var_mode.insert(*dst, M::TwoValue)
                            }
                            (Some(M::FourValue), _) | (_, Some(M::FourValue)) => {
                                _ = var_mode.insert(*dst, M::FourValue)
                            }
                            _ => {}
                        }

                        match (m1, m2) {
                            (Some(M::TwoValue), Some(M::TwoValue))
                            | (Some(M::FourValue), Some(M::FourValue)) => {}

                            (Some(M::FourValue), None) => {
                                maybe_mark_conv_later.push((*falsy, M::FourValue))
                            }
                            (Some(M::FourValue), Some(M::TwoValue)) => _ = mark_conv!(*falsy),
                            (None, Some(M::FourValue)) => {
                                maybe_mark_conv_later.push((*truthy, M::FourValue))
                            }
                            (Some(M::TwoValue), Some(M::FourValue)) => _ = mark_conv!(*truthy),

                            (None, _) | (_, None) => {
                                graph_offsets
                                    .insert(*dst, graph_inputs.len()..graph_inputs.len() + 2);
                                graph_inputs.extend([*truthy, *falsy]);
                            }
                        }
                    }
                    I::Intrinsic(dst, _, _) => _ = var_mode.insert(*dst, LogicMode::TwoValue),
                    I::LastUpdateTime(dst, _) => _ = var_mode.insert(*dst, LogicMode::TwoValue),
                    I::Probe(dst, _, _) => {
                        var_mode.insert(*dst, gl.logic_mode);
                    }
                    I::ProbeSlice(dst, _, _) => {
                        var_mode.insert(*dst, LogicMode::FourValue);
                    }
                    I::Drive(_, src, offset) => {
                        match var_mode.get(src) {
                            Some(m) if *m != gl.logic_mode => _ = mark_conv!(*src),
                            Some(_) => {}
                            None => {
                                maybe_mark_conv_later.push((*src, gl.logic_mode));
                            }
                        }

                        // @Performance. This conversion is kinda useless, but lets keep it for now as
                        // the interpreter relies on it.
                        if let Some((offset, _)) = offset {
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
                                        None => {
                                            maybe_mark_conv_later.push((*v, LogicMode::FourValue))
                                        }
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
                    if *entry.get() != mode {
                        _ = mark_conv!(*k);
                    }
                }
            });
            graph_offsets.retain(|k, _| !seen.contains(k));
        }

        for (v, m) in maybe_mark_conv_later.drain(..) {
            if m != var_mode[&v] {
                mark_conv!(v);
            }
        }
    }
}

pub fn insert_bb_phis(
    regions: &[TemporalRegionKey],
    gl: &GlobalContext,
    bb_stack: &mut Vec<BasicBlockKey>,
    bb_seen: &mut VgHashSet<BasicBlockKey>,
    bb_phis: &mut VgHashMap<BasicBlockKey, Vec<(VariableKey, VariableKey)>>,
) {
    for tr in regions {
        bb_seen.clear();
        bb_seen.insert(tr.entry());
        bb_stack.push(tr.entry());
        while let Some(bb_key) = bb_stack.pop() {
            let bb = gl.bbs.get(bb_key).unwrap();

            for instr in &bb.instrs {
                if let Instruction::Phi(dst, srcs) = instr {
                    for (bb, var) in srcs {
                        bb_phis.entry(*bb).or_insert(Vec::new()).push((*dst, *var));
                    }
                }
            }

            bb.terminator.for_each_non_temporal_bb(|bb| {
                if bb_seen.insert(bb) {
                    bb_stack.push(bb);
                }
            });
        }
    }
}

pub fn resolve_heap_map(
    regions: &[TemporalRegionKey],
    gl: &GlobalContext,
    bb_stack: &mut Vec<BasicBlockKey>,
    bb_seen: &mut VgHashSet<BasicBlockKey>,
    heap_builder: &mut HeapBuilder,
    heap_map: &mut VgHashMap<VariableKey, HeapOffset>,
    mut bits_map: Option<&mut VgHashMap<(Bits, LogicMode), HeapOffset>>,
) {
    for tr in regions {
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
            bb_stack.push(tr.entry());
            while let Some(bb_key) = bb_stack.pop() {
                let bb = gl.bbs.get(bb_key).unwrap();

                for instr in &bb.instrs {
                    if let Some(bits_map) = bits_map.as_mut() {
                        use Instruction as I;
                        let bits = match instr {
                            I::Constant(dst, bits) => Some((dst.mode(), bits)),
                            I::BinaryImm(dst, op, src, imm) => {
                                let imm_mode = if imm.contains_special() {
                                    LogicMode::FourValue
                                } else {
                                    LogicMode::TwoValue
                                };
                                let (mtgt, _, conv_imm) = bin_imm_args_need_conversion(
                                    *op,
                                    dst.mode(),
                                    src.mode(),
                                    imm_mode,
                                );
                                Some(if conv_imm {
                                    (mtgt, imm)
                                } else {
                                    (imm_mode, imm)
                                })
                            }
                            _ => None,
                        };
                        if let Some((mode, bits)) = bits {
                            let mut num_bits = bits.size().get();
                            if mode == LogicMode::FourValue {
                                num_bits = num_bits * 2;
                            }
                            if (min_bits..=max_bits).contains(&num_bits) {
                                bits_map.entry((bits.clone(), mode)).or_insert_with(|| {
                                    heap_builder
                                        .claim_constant(mode, bits.clone_lowering_mode())
                                        .offset
                                });
                            }
                        }
                    }

                    if let Some(dst) = instr.get_destination_variable() {
                        let mode = dst.mode();
                        let size = gl.vars.size(dst);

                        let mut num_bits = size.get();
                        if mode == LogicMode::FourValue {
                            num_bits = num_bits * 2;
                        }

                        if (min_bits..=max_bits).contains(&num_bits) {
                            let heap_offset = if let Instruction::Constant(_, bits) = instr
                                && let Some(bits_map) = bits_map.as_mut()
                            {
                                bits_map[&(bits.clone(), mode)]
                            } else {
                                heap_builder.claim(mode, size).offset
                            };
                            let prev = heap_map.insert(dst, heap_offset);
                            assert!(prev.is_none());
                        }
                    }
                }

                bb_seen.insert(tr.entry());
                bb.terminator.for_each_non_temporal_bb(|bb| {
                    if bb_seen.insert(bb) {
                        bb_stack.push(bb);
                    }
                });
            }
        }
    }
}

pub fn bin_args_need_conversion(
    op: BinaryOp,
    mdst: LogicMode,
    mlhs: LogicMode,
    mrhs: LogicMode,
) -> (LogicMode, bool, bool) {
    use LogicMode as M;
    if op.always_outputs_bool() | op.always_outputs_four_value() {
        (
            M::FourValue, // Operand target mode. So not destination mode!
            mlhs == M::TwoValue && mrhs == M::FourValue,
            mlhs == M::FourValue && mrhs == M::TwoValue,
        )
    } else {
        (mdst, mdst != mlhs, mdst != mrhs)
    }
}

pub fn bin_imm_args_need_conversion(
    op: BinaryImmOp,
    mdst: LogicMode,
    mlhs: LogicMode,
    mrhs: LogicMode,
) -> (LogicMode, bool, bool) {
    use LogicMode as M;
    if op.always_outputs_bool() | op.always_outputs_four_value() {
        (
            M::FourValue, // Operand target mode. So not destination mode!
            mlhs == M::TwoValue && mrhs == M::FourValue,
            mlhs == M::FourValue && mrhs == M::TwoValue,
        )
    } else {
        (mdst, mdst != mlhs, mdst != mrhs)
    }
}
