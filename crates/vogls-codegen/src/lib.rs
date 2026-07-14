mod heap;
pub mod lsra;
mod alignment;

pub use alignment::HeapAlignment;
pub use heap::{Heap, HeapBuilder, HeapOffset, HeapRef};
use vogls_ir::{
    BasicBlockKey, BinaryImmOp, BinaryOp, Bits, GlobalContext, Instruction, LogicMode,
    TemporalRegionKey, VariableKey,
};
use vogls_utils::{VgHashMap, VgHashSet};

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
