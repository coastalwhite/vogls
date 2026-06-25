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
                    I::Constant(dst, bits) => assert_eq!(gl.vars.size(*dst), bits.size()),
                    I::Unary(dst, op, src) => {
                        assert_eq!(gl.vars.size(*dst), op.output_size(gl.vars.size(*src)));
                        assert_eq!(dst.mode(), op.output_mode(src.mode()).unwrap());
                    }
                    I::Resize(dst, op, src) => {
                        assert_eq!(dst.mode(), op.output_mode(src.mode()));
                    }
                    I::Binary(dst, op, lhs, rhs) => {
                        let output_mode = op.output_mode(lhs.mode(), rhs.mode());
                        assert_eq!(lhs.mode(), output_mode.lhs);
                        assert_eq!(rhs.mode(), output_mode.rhs);
                        assert_eq!(dst.mode(), output_mode.dst);
                        assert_eq!(
                            gl.vars.size(*dst),
                            op.output_size(gl.vars.size(*lhs), gl.vars.size(*rhs))
                                .unwrap()
                        );
                    }
                    I::BinaryImm(dst, op, src, imm) => {
                        let output_mode = op.output_mode(src.mode(), imm.mode().into());
                        assert_eq!(src.mode(), output_mode.src);
                        assert_eq!(dst.mode(), output_mode.dst);
                        assert_eq!(
                            gl.vars.size(*dst),
                            op.output_size(gl.vars.size(*src), imm.size()).unwrap()
                        );
                    }
                    I::Slice(dst, _, _) => {
                        assert_eq!(dst.mode(), LogicMode::FourValue);
                    }
                    I::SliceImm(dst, src, _) => {
                        assert_eq!(dst.mode(), src.mode());
                    }
                    I::ShiftImm(dst, _, src, _) => {
                        assert_eq!(dst.mode(), src.mode());
                    }
                    I::Select(dst, _, truthy, falsy) => {
                        assert_eq!(dst.mode(), truthy.mode());
                        assert_eq!(dst.mode(), falsy.mode());
                    }
                    I::Intrinsic(..) => {}
                    I::LastUpdateTime(dst, _) => {
                        assert_eq!(dst.mode(), LogicMode::TwoValue);
                        assert_eq!(gl.vars.size(*dst), TIME_VSIZE);
                    }
                    I::Probe(dst, signal, _) => {
                        assert_eq!(dst.mode(), gl.signals[*signal].mode);
                    }
                    I::ProbeSlice(dst, _, _) => {
                        assert_eq!(dst.mode(), LogicMode::FourValue);
                    }
                    I::Drive(signal, src, _) => {
                        assert_eq!(gl.signals[*signal].mode, src.mode());
                    }
                    I::Phi(..) => {}
                }
            }
        }
    }
}
