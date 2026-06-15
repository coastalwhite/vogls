use slotmap::SlotMap;
use vogls_ir::{
    BasicBlock, BasicBlockKey, GlobalContext, Instruction, Process, ProcessKey, SignalKey,
};
use vogls_utils::VgHashSet;

pub fn insert_process_prb_drv(
    bbs: &SlotMap<BasicBlockKey, BasicBlock>,
    process_key: ProcessKey,
    entry: BasicBlockKey,

    bb_stack: &mut Vec<BasicBlockKey>,
    bb_seen: &mut VgHashSet<BasicBlockKey>,

    prb_set: &mut VgHashSet<SignalKey>,
    drv_set: &mut VgHashSet<SignalKey>,

    prb: &mut Vec<(ProcessKey, SignalKey)>,
    drv: &mut Vec<(ProcessKey, SignalKey)>,
) {
    prb_set.clear();
    drv_set.clear();
    bb_seen.clear();
    bb_stack.clear();

    bb_seen.insert(entry);
    bb_stack.push(entry);

    while let Some(bb_key) = bb_stack.pop() {
        let bb = &bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_seen.insert(bb) {
                bb_stack.push(bb);
            }
        });

        for i in &bb.instrs {
            use Instruction as I;
            match i {
                I::Probe(_, signal) if prb_set.insert(*signal) => {
                    prb.push((process_key, *signal));
                }
                I::Drive(signal, _, _) if drv_set.insert(*signal) => {
                    drv.push((process_key, *signal));
                }
                _ => {}
            }
        }
    }
}

pub fn inline_signals(
    gl: &mut GlobalContext,
    bb_stack: &mut Vec<BasicBlockKey>,
    bb_seen: &mut VgHashSet<BasicBlockKey>,
) {
    let mut prb_set = VgHashSet::default();
    let mut drv_set = VgHashSet::default();

    let mut prb = Vec::new();
    let mut drv = Vec::new();

    for (key, process) in &gl.processes {
        insert_process_prb_drv(
            &gl.bbs,
            key,
            process.entry,
            bb_stack,
            bb_seen,
            &mut prb_set,
            &mut drv_set,
            &mut prb,
            &mut drv,
        );
    }

    if drv.is_empty() {
        return;
    }

    prb.sort_unstable_by_key(|(_, s)| *s);
    drv.sort_unstable_by_key(|(_, s)| *s);

    let mut inlined_signals = Vec::<(SignalKey, BasicBlockKey)>::new();
    let mut num_drivers = 1u64;
    for i in 0..drv.len() - 1 {
        if drv[i].1 == drv[i + 1].1 {
            num_drivers += 1;
        } else {
            if num_drivers == 1 {
                // Determine eligibility.
                todo!();
            }
            num_drivers = 1;
        }
    }


}
