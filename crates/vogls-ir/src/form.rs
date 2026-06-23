use vogls_utils::{VgHashMap, VgHashSet};

use crate::{BasicBlockKey, GlobalContext, TemporalRegionKey, VariableKey};

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
            }
        }
    }
}
