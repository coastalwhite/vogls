mod heap;

pub use heap::{Heap, HeapBuilder, HeapOffset, HeapRef};
use vogls_ir::{BasicBlockKey, GlobalContext, Instruction, LogicMode, VariableKey};
use vogls_utils::{VgHashMap, VgHashSet};

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

    let mut unresolved = VgHashMap::<VariableKey, Vec<VariableKey>>::default();
    let mut need_modes = VgHashMap::<VariableKey, u8>::default();

    fn mode_to_flag(m: LogicMode) -> u8 {
        match m {
            LogicMode::TwoValue => 1,
            LogicMode::FourValue => 2,
        }
    }

    // Fill `var_mode` with the `LogicMode` for each variable.
    //
    // @Performance
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
                        None => unresolved.entry(*dst).or_default().push(*src),
                    }
                }
                I::Binary(dst, _, lhs, rhs) => {
                    let m1 = var_mode.get(lhs);
                    let m2 = var_mode.get(rhs);

                    use LogicMode as M;
                    match (m1, m2) {
                        (Some(&m1), Some(&m2)) => {
                            let m = if m1 == LogicMode::FourValue || m2 == LogicMode::FourValue {
                                if m1 == LogicMode::TwoValue {
                                    mark_conv!(*lhs);
                                }
                                if m2 == LogicMode::TwoValue {
                                    mark_conv!(*rhs);
                                }
                                LogicMode::FourValue
                            } else {
                                LogicMode::TwoValue
                            };
                            var_mode.insert(*dst, m);
                        }
                        (Some(M::FourValue), _) => {
                            *need_modes.entry(*rhs).or_default() |= mode_to_flag(M::FourValue);
                            var_mode.insert(*dst, M::FourValue);
                        }
                        (_, Some(M::FourValue)) => {
                            *need_modes.entry(*lhs).or_default() |= mode_to_flag(M::FourValue);
                            var_mode.insert(*dst, M::FourValue);
                        }
                        (Some(_), None) => unresolved.entry(*dst).or_default().push(*rhs),
                        (None, Some(_)) => unresolved.entry(*dst).or_default().push(*lhs),
                        (None, None) => unresolved.entry(*dst).or_default().extend([*lhs, *rhs]),
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
                        None => *need_modes.entry(*src).or_default() |= mode_to_flag(gl.logic_mode),
                    }

                    if let Some((offset, _)) = partial {
                        match var_mode.get(offset) {
                            Some(m) if *m != gl.logic_mode => _ = mark_conv!(*src),
                            Some(_) => {}
                            None => {
                                *need_modes.entry(*offset).or_default() |=
                                    mode_to_flag(gl.logic_mode)
                            }
                        }
                    }
                }
                I::Phi(dst, items) => {
                    let mut logic_mode = Some(LogicMode::TwoValue);
                    for (_, v) in items {
                        match var_mode.get(v) {
                            None => {
                                logic_mode = None;
                                unresolved.entry(*dst).or_default().push(*v);
                            }
                            Some(LogicMode::TwoValue) => {}
                            Some(LogicMode::FourValue) => {
                                logic_mode = Some(LogicMode::FourValue);
                            }
                        }
                    }
                    if let Some(logic_mode) = logic_mode {
                        if logic_mode == LogicMode::FourValue {
                            for (_, v) in items {
                                if var_mode.get(v) == Some(&LogicMode::TwoValue) {
                                    mark_conv!(*v);
                                }
                            }
                        }
                        var_mode.insert(*dst, logic_mode);
                    }
                }
            }
        }
    }

    while !unresolved.is_empty() {
        let start_length = unresolved.len();
        unresolved.retain(|k, v| {
            let mut num_fvs = 0usize;
            let mut num_tvs = 0usize;
            v.iter().for_each(|dep| {
                let m = var_mode.get(dep);
                num_tvs += usize::from(matches!(m, Some(LogicMode::TwoValue)));
                num_fvs += usize::from(matches!(m, Some(LogicMode::FourValue)));
            });

            if num_fvs > 0 {
                v.iter().for_each(|dep| {
                    *need_modes.entry(*dep).or_default() |= mode_to_flag(LogicMode::FourValue);
                });
                var_mode.insert(*k, LogicMode::FourValue);
                false
            } else if num_tvs == v.len() {
                v.iter().for_each(|dep| {
                    *need_modes.entry(*dep).or_default() |= mode_to_flag(LogicMode::TwoValue);
                });
                var_mode.insert(*k, LogicMode::TwoValue);
                false
            } else {
                true
            }
        });

        // @Hack. This is weird, there is probably a better solution here.
        //
        // There is a loop in the implication graph. This means no-one is hard wired to FV, so we
        // just set it to TV.
        if unresolved.len() == start_length {
            let &k = unresolved.keys().next().unwrap();
            let v = unresolved.remove(&k).unwrap();

            v.iter().for_each(|dep| {
                *need_modes.entry(*dep).or_default() |= mode_to_flag(LogicMode::TwoValue);
            });
            var_mode.insert(k, LogicMode::TwoValue);
        }
    }

    for (k, f) in need_modes {
        let m = var_mode[&k];
        if f != mode_to_flag(m) {
            mark_conv!(k);
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

            bb_seen.insert(entry);
            bb.terminator.for_each_bb(|bb| {
                if bb_seen.insert(bb) {
                    bb_stack.push(bb);
                }
            });
        }
    }
}
