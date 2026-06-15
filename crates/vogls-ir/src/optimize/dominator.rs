use std::ops::Range;

use vogls_utils::{SecondaryTable, Table, VgHashMap, VgHashSet};

use crate::{BasicBlockKey, GlobalContext, ProcessKey};

pub struct DominatorNode {
    parent: Option<BasicBlockKey>,
    children: Range<usize>,
}

vogls_utils::new_table_key! { struct DominatorKey; }

pub fn dominator(
    gl: &GlobalContext,
    process: ProcessKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
) -> (VgHashMap<BasicBlockKey, DominatorNode>, Vec<BasicBlockKey>) {
    let entry = gl.processes[process].entry;

    let mut dfs = Table::<DominatorKey, BasicBlockKey>::default();
    let mut ancestor = SecondaryTable::<DominatorKey, Option<DominatorKey>>::new();
    let mut parent = SecondaryTable::<DominatorKey, Option<DominatorKey>>::new();
    let mut semi = SecondaryTable::<DominatorKey, Option<DominatorKey>>::new();
    let mut label = SecondaryTable::<DominatorKey, DominatorKey>::new();
    let mut bucket = SecondaryTable::<DominatorKey, Vec<DominatorKey>>::new();

    let mut stack = Vec::<(BasicBlockKey, DominatorKey)>::new();

    let v = dfs.insert(entry);
    parent[v] = None;
    ancestor[v] = None;
    label[v] = v;
    bucket[v] = Vec::new();

    scratch_seen.clear();
    scratch_seen.insert(entry);
    stack.push((entry, v));

    while let Some((bb_key, v)) = stack.pop() {
        let bb = &gl.bbs[bb_key];
        bb.terminator.for_each_bb(|next| {
            if scratch_seen.insert(next) {
                let w = dfs.insert(entry);
                parent[w] = Some(v);
                ancestor[w] = None;
                label[w] = w;
                bucket[w] = Vec::new();

                stack.push((next, w));
            }
        });
    }

    fn compress(
        v: DominatorKey,
        path: &mut Vec<DominatorKey>,
        ancestor: &mut SecondaryTable<DominatorKey, Option<DominatorKey>>,
        semi: &mut SecondaryTable<DominatorKey, Option<DominatorKey>>,
        label: &mut SecondaryTable<DominatorKey, DominatorKey>,
    ) {
        path.clear();
        let mut u = v;
        while let Some(uu) = ancestor[u]
            && ancestor[uu].is_some()
        {
            path.push(u);
            u = uu;
        }

        for &x in path.iter().rev() {
            let a = ancestor[x].unwrap();
            if semi[label[a]] < semi[label[x]] {
                label[x] = label[a];
            }
            ancestor[x] = ancestor[a];
        }
    }
    fn eval_(
        v: DominatorKey,
        path: &mut Vec<DominatorKey>,
        ancestor: &mut SecondaryTable<DominatorKey, Option<DominatorKey>>,
        semi: &mut SecondaryTable<DominatorKey, Option<DominatorKey>>,
        label: &mut SecondaryTable<DominatorKey, DominatorKey>,
    ) -> DominatorKey {
        if ancestor[v].is_none() {
            return v;
        }
        compress(v, path, ancestor, semi, label);
        return label[v];
    }

    for w in dfs.table_key_iter().rev() {
        // Step 2: compute the semidominator of w.
        for v in pred.get(w, ()) {
            if semi.contains(v) {
                continue;  // v is unreachable from root; ignore it
            }
            let u = eval_(v);
            if semi[u] < semi[w] {
                semi[w] = semi[u]
            }
        }
        // semi[w] is now the DFS number of sdom(w).
        bucket[semi[w]].push(w);
        link(parent[w], w);
 
        // Step 3: for each v whose semidominator is parent(w), we can now
        //  decide its immediate dominator (possibly only implicitly).
        let pw = parent[w];
        // assert pw is not None  # w != root, so it has a parent
        let pw_bucket = bucket[pw];
        while let Some(v) = pw_bucket.pop() {
            let v = pw_bucket.pop();
            let u = eval_(v);
            // If the minimum-sdom ancestor u has the same sdom as v, then
            // sdom(v) dominates v (Theorem 2) and is its immediate dominator.
            // Otherwise u and v share an immediate dominator (Theorem 3);
            // we record u as a proxy and resolve it in step 4.
            dom[v] = u if semi[u] < semi[v] else pw;
        }
    }

    todo!()
}
