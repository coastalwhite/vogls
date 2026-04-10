use std::fmt::Write;
use vogls_utils::VgHashSet;

use crate::{BasicBlockKey, ContextFormat, DisplayContext, GlobalContext, ProcessKey};

pub fn control_flow_graph_dot(
    gl: &GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) -> String {
    let entry = gl.processes[process].entry;

    let mut f = String::new();
    let mut label = String::new();
    let mut ctx = DisplayContext::new(gl);
    ctx.prepare_process(entry);

    writeln!(f, "digraph cfg {{").unwrap();

    scratch_seen.clear();
    scratch_stack.clear();
    scratch_seen.insert(entry);
    scratch_stack.push(entry);
    while let Some(bb_key) = scratch_stack.pop() {
        let bb = &gl.bbs[bb_key];
        let idx = ctx.get_bb_idx(bb_key).unwrap();

        label.clear();
        for i in &bb.instrs {
            write!(label, "{}", i.display(&ctx)).unwrap();
            label.push_str("\\n");
        }
        write!(label, "{}", bb.terminator.display(&ctx)).unwrap();
        label.push_str("\\n");

        writeln!(f, "  P{idx}[label=\"{label}\"]").unwrap();

        bb.terminator.for_each_bb(|next| {
            writeln!(
                f,
                "  P{idx} -> P{next_idx}",
                next_idx = ctx.get_bb_idx(bb_key).unwrap()
            )
            .unwrap();
            if scratch_seen.insert(next) {
                scratch_stack.push(next);
            }
        });
    }
    writeln!(f, "}}").unwrap();

    f
}
