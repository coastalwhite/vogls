use vogls_utils::{VgHashMap, VgHashSet};

use crate::{
    BasicBlockKey, GlobalContext, Instruction, LogicMode, TIME_VSIZE, TemporalRegionKey,
    VariableKey,
};

pub fn check_ir_form(regions: &[TemporalRegionKey], gl: &GlobalContext) {
    let mut var_region = VgHashMap::<VariableKey, TemporalRegionKey>::default();

    let mut stack = Vec::<BasicBlockKey>::new();
    let mut seen = VgHashSet::<BasicBlockKey>::default();

    for &tr in regions {
        seen.clear();

        stack.push(tr.entry());
        seen.insert(tr.entry());

        while let Some(bb_key) = stack.pop() {
            let bb = &gl.bbs[bb_key];

            bb.terminator.for_each_non_temporal_bb(|next_bb| {
                if seen.insert(next_bb) {
                    stack.push(next_bb);
                }
            });

            assert_eq!(bb.region, tr);

            for i in &bb.instrs {
                i.for_each_var(|v| {
                    assert_eq!(*var_region.entry(v).or_insert(tr), tr);
                });

                use Instruction as I;
                match i {
                    I::Constant(dst, bits) => {
                        if dst.mode() != bits.mode().into() {
                            dbg!(dst.mode(), LogicMode::from(bits.mode()));
                        }
                        assert_eq!(gl.vars.size(*dst), bits.size());
                        assert_eq!(dst.mode(), bits.mode().into());
                    }
                    I::Unary(dst, op, src) => {
                        assert_eq!(gl.vars.size(*dst), op.output_size(gl.vars.size(*src)));
                        assert_eq!(dst.mode(), op.output_mode(src.mode()));
                    }
                    I::Resize(dst, op, src) => {
                        assert_eq!(dst.mode(), op.output_mode(src.mode()));
                    }
                    I::Binary(dst, op, lhs, rhs) => {
                        if dst.mode() != op.output_mode(lhs.mode(), rhs.mode()) {
                            dbg!(&bb.instrs);
                            dbg!(op, dst.mode(), op.output_mode(lhs.mode(), rhs.mode()));
                        }
                        assert_eq!(
                            gl.vars.size(*dst),
                            op.output_size(gl.vars.size(*lhs), gl.vars.size(*rhs))
                                .unwrap()
                        );
                        assert_eq!(dst.mode(), op.output_mode(lhs.mode(), rhs.mode()));
                    }
                    I::BinaryImm(dst, op, src, imm) => {
                        assert_eq!(
                            gl.vars.size(*dst),
                            op.output_size(gl.vars.size(*src), imm.size()).unwrap()
                        );
                        assert_eq!(dst.mode(), op.output_mode(src.mode(), imm.mode().into()));
                    }
                    I::Slice(..) => {}
                    I::SliceImm(..) => {}
                    I::ShiftImm(..) => {}
                    I::Select(..) => {}
                    I::Intrinsic(..) => {}
                    I::LastUpdateTime(dst, _) => {
                        assert_eq!(dst.mode(), LogicMode::TwoValue);
                        assert_eq!(gl.vars.size(*dst), TIME_VSIZE);
                    }
                    I::Probe(dst, signal, _) => {
                        assert_eq!(dst.mode(), gl.signals[*signal].mode);
                    }
                    I::ProbeSlice(..) => {}
                    I::Drive(..) => {}
                    I::Phi(..) => {}
                }
            }
        }
    }
}
