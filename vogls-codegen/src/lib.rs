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

    bb_stack.clear();
    // Fill `var_mode` with the `LogicMode` for each variable.
    //
    // @Performance
    loop {
        bb_stack.push(entry);
        bb_seen.insert(entry);
        let mut has_unresolved_variables = false;

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
                        if let Some(m) = var_mode.get(src).copied() {
                            var_mode.insert(*dst, m);
                        }
                    }
                    I::Binary(dst, _, lhs, rhs) => {
                        let m1 = var_mode.get(lhs);
                        let m2 = var_mode.get(rhs);

                        if let (Some(&m1), Some(&m2)) = (m1, m2) {
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
                    }
                    I::Intrinsic(dst, _, _) => _ = var_mode.insert(*dst, LogicMode::TwoValue),
                    I::LastUpdateTime(dst, _) => _ = var_mode.insert(*dst, LogicMode::TwoValue),
                    I::Probe(dst, _) => {
                        var_mode.insert(*dst, gl.logic_mode);
                    }
                    I::Drive(_, src, partial) => {
                        if let Some(m) = var_mode.get(src)
                            && *m != gl.logic_mode
                        {
                            mark_conv!(*src);
                        }
                        if let Some((offset, _)) = partial
                            && let Some(m) = var_mode.get(offset)
                            && *m != gl.logic_mode
                        {
                            mark_conv!(*offset);
                        }
                    }
                    I::Phi(dst, items) => {
                        let mut logic_mode = Some(LogicMode::TwoValue);
                        for (_, v) in items {
                            match var_mode.get(v) {
                                None => {
                                    logic_mode = None;
                                    has_unresolved_variables = true;
                                    break;
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

        bb_seen.clear();
        if !has_unresolved_variables {
            break;
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
