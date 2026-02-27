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
) {
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
                        if let (Some(m1), Some(m2)) =
                            (var_mode.get(lhs).copied(), var_mode.get(rhs).copied())
                        {
                            let m = if m1 == LogicMode::FourValue || m2 == LogicMode::FourValue {
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
                    I::Drive(_, _, _) => {}
                    I::Phi(dst, items) => {
                        for (_, v) in items {
                            if let Some(m) = var_mode.get(v).copied() {
                                if m == LogicMode::FourValue {
                                    var_mode.insert(*dst, LogicMode::FourValue);
                                    continue;
                                }
                            } else {
                                has_unresolved_variables = true;
                                continue;
                            }
                        }
                        var_mode.insert(*dst, LogicMode::TwoValue);
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
                    let size = gl.vars.get(dst).unwrap().size;
                    let mode = var_mode[&dst];

                    let mut num_bits = size.get();
                    if mode == LogicMode::FourValue {
                        num_bits = num_bits * 2;
                    }

                    if (min_bits..=max_bits).contains(&num_bits) {
                        let prev = heap_map.insert(dst, heap_builder.claim(mode, size).offset);
                        assert!(prev.is_none());
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
