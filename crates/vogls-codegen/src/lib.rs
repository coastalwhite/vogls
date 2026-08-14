mod alignment;
mod heap;
pub mod lsra;
mod six_bit_size;

pub use alignment::HeapAlignment;
pub use heap::{Heap, HeapBuilder, HeapOffset, HeapRef};
pub use six_bit_size::SixBitSize;
use vogls_ir::{
    BasicBlockKey, BinaryImmOp, BinaryOp, GlobalContext, Instruction, LogicMode, TemporalRegionKey,
    VariableKey,
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
