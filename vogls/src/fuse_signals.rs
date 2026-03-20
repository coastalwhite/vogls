use slotmap::SlotMap;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, INTEGER_VSIZE, Signal, SignalKey, SignalSlice,
    new_process,
};
use vogls_utils::{IndexMap, TableKey, TableSet, VgHashMap};

vogls_utils::new_table_key! { struct NodeKey; }
slotmap::new_key_type! { struct EdgeKey; }

struct Edge {
    driver: NodeKey,
    drivee: NodeKey,
    driver_slice: SignalSlice,
    drivee_slice: SignalSlice,
}
#[derive(Default)]
struct Node {
    fanin: Vec<EdgeKey>,
    fanout: Vec<EdgeKey>,
}
type Nodes = TableSet<NodeKey, SignalKey, Node>;
type Edges = SlotMap<EdgeKey, Edge>;

fn print(signals: &SlotMap<SignalKey, Signal>, nodes: &Nodes, edges: &Edges) {
    println!("digraph {{");
    for (n, s, _) in nodes.iter() {
        println!(r#"  n{} [label="{}"];"#, n.get(), &signals[s].name);
    }
    for edge in edges.values() {
        println!(
            r#"  n{} -> n{} [taillabel="{}", headlabel="{}"];"#,
            edge.driver.get(),
            edge.drivee.get(),
            if edge.driver_slice.lsb() > 0
                || edge.driver_slice.width() != signals[nodes.get_key(edge.driver)].size
            {
                format!("[{}:{}]", edge.driver_slice.msb(), edge.driver_slice.lsb())
            } else {
                String::new()
            },
            if edge.drivee_slice.lsb() > 0
                || edge.drivee_slice.width() != signals[nodes.get_key(edge.drivee)].size
            {
                format!("[{}:{}]", edge.drivee_slice.msb(), edge.drivee_slice.lsb())
            } else {
                String::new()
            },
        );
    }
    println!("}}");
}

/// Fuse signals for a given list of drivers & drivees pairs.
///
/// Given a list of signals that are seen as equivalent try to merge as many signals as possible.
/// This is done by creating a directed graph that represents relationships between signals. Each
/// node represents a signal and each edge represents a driver to drivee relationship. The driver
/// and drivee of an edge may also optionally have a slice associated with them, allowing partial
/// probes and drives.
///
/// A set of graph transformations is performed on this graph usually reducing the depth of the
/// graph.
///
/// | Property        | From                       | To               |
/// |-----------------|----------------------------|------------------|
/// | Transitive      | A -> B -> C                | A -> B, A -> C   |
/// | Neighbour Merge | A[0] -> B[0], A[1] -> B[1] | A[1:0] -> B[1:0] |
/// | Symmetric       | A -> B, B -> A             | A -> B           |
///
/// At the end, you have a graph where many nodes only have on fanin edge. The driver of this edge
/// is uses as an alias for the node's signal. The returned map is a map from the original signal
/// to the new alias. If there is a more complex relation between nodes, a marshalling process is
/// inserted to represent that relationship.
///
/// This is a important pass for optimization that removes:
/// - Redundant signals (reducing memory consumption)
/// - Marshalling processes (reducing scheduling overhead)
pub fn fuse_signals(
    gl: &mut GlobalContext,
    connections: &[vogls_verilog::lower::Edge],
) -> VgHashMap<SignalKey, (SignalKey, Option<SignalSlice>)> {
    let mut nodes = Nodes::default();
    let mut edges = Edges::default();

    // Form the graph.
    for edge in connections.iter() {
        let driver = nodes.get_or_default(edge.driver);
        let drivee = nodes.get_or_default(edge.drivee);

        let driver_slice = edge
            .driver_slice
            .unwrap_or_else(|| SignalSlice::with_end(gl.signals[edge.driver].size));
        let drivee_slice = edge
            .drivee_slice
            .unwrap_or_else(|| SignalSlice::with_end(gl.signals[edge.drivee].size));

        let edge = edges.insert(Edge {
            driver,
            drivee,
            driver_slice,
            drivee_slice,
        });
        nodes[driver].fanout.push(edge);
        nodes[drivee].fanin.push(edge);
    }

    print(&gl.signals, &nodes, &edges);

    // Transform the graph until a fixed-point.
    let mut changed = true;
    let mut cyclic = Vec::new();
    while changed {
        changed = false;
        for node in nodes.table_key_iter() {
            if nodes[node].fanin.len() > 1 {
                nodes[node].fanin.sort_unstable_by_key(|&e| {
                    (edges[e].drivee_slice.lsb(), edges[e].drivee_slice.width())
                });
                let mut offset = 0;
                for &e in &nodes[node].fanin {
                    let s = &edges[e].drivee_slice;
                    if offset > s.lsb() {
                        print(&gl.signals, &nodes, &edges);
                        // @TODO: Better error
                        panic!("multiple drivers for same wire");
                    }
                    offset = s.lsb() + s.width().get();
                }
            }

            // Transitive property.
            let mut i = 0;
            cyclic.clear();
            while i < nodes[node].fanout.len() {
                let e = nodes[node].fanout[i];
                i += 1;

                let Edge {
                    driver,
                    drivee,
                    driver_slice,
                    drivee_slice,
                } = edges[e];

                if driver == drivee {
                    cyclic.push(i);
                    continue;
                }

                if nodes[drivee].fanout.is_empty() {
                    continue;
                }

                changed = true;

                let mut drivee_fanout = std::mem::take(&mut nodes[drivee].fanout);
                nodes[node]
                    .fanout
                    .extend(drivee_fanout.extract_if(.., |&mut e| {
                        let edge = &mut edges[e];
                        let Some(slice) = drivee_slice.relative_slice(edge.driver_slice) else {
                            return false;
                        };
                        let Some(slice) = slice.shift(driver_slice.lsb()) else {
                            return false;
                        };

                        edges[e].driver = driver;
                        edges[e].driver_slice = slice;
                        true
                    }));
                nodes[drivee].fanout = drivee_fanout;
            }

            // Cyclic property.
            if !cyclic.is_empty() {
                for &i in cyclic.iter() {
                    let e = nodes[node].fanout[i];
                    if edges[e].driver_slice == edges[e].drivee_slice {
                        changed = true;
                        nodes[node].fanout.swap_remove(i);
                        // @Performance: Linear scan.
                        let idx = nodes[node].fanin.iter().position(|&ie| ie == e).unwrap();
                        nodes[node].fanin.swap_remove(idx);
                        edges.remove(e);
                    }
                }
                cyclic.clear();
            }

            // Merge wires that represent sequential slices on both sides.
            if nodes[node].fanout.len() > 1 {
                nodes[node].fanout.sort_unstable_by_key(|&e| {
                    let edge = &edges[e];
                    (
                        edge.drivee,
                        edge.drivee_slice.lsb(),
                        edge.drivee_slice.msb(),
                    )
                });

                let mut read = 1;
                let mut write = 0;
                while read < nodes[node].fanout.len() {
                    let ledgekey = nodes[node].fanout[write];
                    let redgekey = nodes[node].fanout[read];
                    let ledge = &edges[ledgekey];
                    let redge = &edges[redgekey];
                    if ledge.drivee == redge.drivee
                        && let Some(fslice) = ledge.driver_slice.concat(redge.driver_slice)
                        && let Some(tslice) = ledge.drivee_slice.concat(redge.drivee_slice)
                    {
                        changed = true;
                        let edge = &mut edges[ledgekey];
                        edge.driver_slice = fslice;
                        edge.drivee_slice = tslice;
                        // @Performance: Linear scan.
                        let idx = nodes[edge.drivee]
                            .fanin
                            .iter()
                            .position(|&ie| ie == redgekey)
                            .unwrap();
                        nodes[edge.drivee].fanin.swap_remove(idx);
                        edges.remove(redgekey);
                    } else {
                        write += 1;
                        nodes[node].fanout[write] = nodes[node].fanout[read];
                    }
                    read += 1;
                }
                nodes[node].fanout.truncate(write);
            }
        }
    }

    print(&gl.signals, &nodes, &edges);
    println!();

    let mut replacement_signals =
        VgHashMap::<SignalKey, (SignalKey, Option<SignalSlice>)>::default();

    for (_, signal, v) in nodes.iter() {
        if v.fanin.is_empty() {
            continue;
        }

        if let Some(edge) = v.fanin.first().map(|&e| &edges[e])
            && v.fanin.len() == 1
            && edge.drivee_slice.lsb() == 0
            && edge.drivee_slice.width() == gl.signals[signal].size
        {
            let driver_signal = nodes.get_key(edge.driver);
            let driver_slice = (edge.driver_slice.lsb() != 0
                || edge.driver_slice.width() != gl.signals[driver_signal].size)
                .then_some(edge.driver_slice);
            replacement_signals.insert(signal, (driver_signal, driver_slice));
            continue;
        }

        // If it is more complicated, we have to insert a process that propagated from driver to
        // drivee.
        let (_, mut builder) = new_process(gl, "fuse_signal".into(), TokenRange::default());
        let mut watch_signals = IndexMap::default();
        for &e in &v.fanin {
            let edge = &edges[e];
            let driver_signal = nodes.get_key(edge.driver);
            // @TODO: probe constant?
            let mut prb = *watch_signals
                .get_or_insert_with(driver_signal, || builder.probe(gl, driver_signal));

            if edge.driver_slice.lsb() != 0
                || edge.driver_slice.width() != gl.signals[driver_signal].size
            {
                prb = builder.slice_constant(
                    gl,
                    prb,
                    edge.driver_slice.lsb(),
                    edge.driver_slice.width(),
                );
            }
            builder.drive_partial_constant(
                gl,
                signal,
                prb,
                edge.drivee_slice.lsb(),
                edge.drivee_slice.width(),
            )
        }
        let entry = builder.key();
        builder.watch_to(gl, watch_signals.take_keys(), entry);
    }

    // TODO: This is stupid.
    let keys = gl.bbs.keys().collect::<Vec<_>>();
    for key in keys {
        let mut builder = BasicBlockBuilder::continue_from(Vec::new(), key);
        use vogls_ir::Instruction as I;
        for i in std::mem::take(&mut gl.bbs[key].instrs) {
            match &i {
                I::Probe(dst, signal) => {
                    if let Some((to, slice)) = replacement_signals.get(signal) {
                        match slice {
                            None => builder.push_raw_instruction(I::Probe(*dst, *to)),
                            Some(slice) => {
                                builder.probe_slice_constant(gl, *to, slice.lsb(), slice.width());
                                builder
                                    .instrs
                                    .last_mut()
                                    .unwrap()
                                    .get_destination_variable_mut()
                                    .map(|d| *d = *dst);
                            }
                        }
                        continue;
                    }
                }
                I::Drive(signal, src, partial) => {
                    if let Some((to, slice)) = replacement_signals.get(signal) {
                        match (*partial, slice) {
                            (partial, None) => builder.drive_opt_partial(gl, *to, *src, partial),
                            (None, Some(slice)) => builder.drive_partial_constant(
                                gl,
                                *to,
                                *src,
                                slice.lsb(),
                                slice.width(),
                            ),
                            (Some((offset, width)), Some(slice)) => {
                                let offset = builder.plus_constant(
                                    gl,
                                    offset,
                                    Bits::from_u64(INTEGER_VSIZE, slice.lsb() as u64),
                                );
                                builder.drive_partial(gl, *to, *src, offset, width);
                            }
                        }

                        continue;
                    }
                }

                // @TODO: VCD
                I::Intrinsic(_, _, _) => {}
                _ => {}
            }
            builder.push_raw_instruction(i);
        }
        gl.bbs[key].instrs = builder.into_instructions();
        gl.bbs[key]
            .terminator
            .map_signal(|s| replacement_signals.get(&s).map_or(s, |(s, _)| *s));
    }

    replacement_signals
}
