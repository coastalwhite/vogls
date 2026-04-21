use std::fmt;
use std::io::Write;
use std::ops::{BitOr, BitOrAssign};

use slotmap::SlotMap;
use vogls_bits::format::BitsFormatOptions;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, INTEGER_VSIZE, Instruction, IntrinsicOp, Signal,
    SignalKey, SignalSlice, VectorSize, new_process,
};
use vogls_utils::{IndexMap, Table, TableKey, VgHashMap, VgHashSet};
use vogls_verilog::lower::Driver;

vogls_utils::new_table_key! { struct NodeKey; }
slotmap::new_key_type! { struct EdgeKey; }

pub const LUPDT: u32 = 1u32;
pub const PROBE: u32 = 2u32;
pub const DRIVE: u32 = 4u32;
pub const WATCH: u32 = 8u32;

struct NodeDisplay<'a>(&'a Node, &'a SlotMap<SignalKey, Signal>);
impl<'a> fmt::Display for NodeDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.content {
            NodeContent::Signal(s) => f.write_str(&self.1[*s].name),
            NodeContent::Constant(bits) => bits.display(&BitsFormatOptions::default()).fmt(f),
        }
    }
}

impl Node {
    fn display<'a>(&'a self, signals: &'a SlotMap<SignalKey, Signal>) -> NodeDisplay<'a> {
        NodeDisplay(self, signals)
    }
}

#[allow(unused)]
fn print(signals: &SlotMap<SignalKey, Signal>, nodes: &Nodes, edges: &Edges) {
    use std::fmt::Write;
    let mut out = String::new();
    writeln!(&mut out, "digraph {{").unwrap();
    for (key, node) in nodes.key_value_iter() {
        writeln!(
            &mut out,
            r#"  n{} [label="{}"];"#,
            key.get(),
            node.display(signals)
        );
    }
    for edge in edges.values() {
        writeln!(
            &mut out,
            r#"  n{} -> n{} [taillabel="{}", headlabel="{}"];"#,
            edge.driver.get(),
            edge.drivee.get(),
            if edge.driver_slice.lsb() > 0 || edge.driver_slice.width() != nodes[edge.driver].size {
                format!("[{}:{}]", edge.driver_slice.msb(), edge.driver_slice.lsb())
            } else {
                String::new()
            },
            if edge.drivee_slice.lsb() > 0 || edge.drivee_slice.width() != nodes[edge.drivee].size {
                format!("[{}:{}]", edge.drivee_slice.msb(), edge.drivee_slice.lsb())
            } else {
                String::new()
            },
        );
    }
    writeln!(&mut out, "}}");
    println!("{out}");
}

#[allow(unused)]
fn print_edge(out: &mut String, edge: &Edge, nodes: &Nodes) {
    use std::fmt::Write;
    writeln!(
        out,
        r#"  n{} -> n{} [taillabel="{}", headlabel="{}"];"#,
        edge.driver.get(),
        edge.drivee.get(),
        if edge.driver_slice.lsb() > 0 || edge.driver_slice.width() != nodes[edge.driver].size {
            format!("[{}:{}]", edge.driver_slice.msb(), edge.driver_slice.lsb())
        } else {
            String::new()
        },
        if edge.drivee_slice.lsb() > 0 || edge.drivee_slice.width() != nodes[edge.drivee].size {
            format!("[{}:{}]", edge.drivee_slice.msb(), edge.drivee_slice.lsb())
        } else {
            String::new()
        },
    )
    .unwrap();
}

#[allow(unused)]
fn print_subgraph(
    out: &mut String,
    node: NodeKey,
    signals: &SlotMap<SignalKey, Signal>,
    nodes: &Nodes,
    edges: &Edges,
) {
    let mut seen = VgHashSet::<NodeKey>::default();
    let mut seen_edges = VgHashSet::<EdgeKey>::default();
    let mut stack = Vec::new();

    stack.push(node);
    seen.insert(node);

    while let Some(n) = stack.pop() {
        use std::fmt::Write;
        write!(
            out,
            r#"  n{} [label="{}"#,
            n.get(),
            nodes[n].display(signals)
        )
        .unwrap();

        let f = nodes[n].flags;
        if f != NodeFlags::EMPTY {
            write!(out, " [").unwrap();
            if f.contains(NodeFlags::DRIVE) {
                out.push('D');
            }
            if f.contains(NodeFlags::PROBE) {
                out.push('P');
            }
            if f.contains(NodeFlags::LUPDT) {
                out.push('L');
            }
            if f.contains(NodeFlags::DRIVE) {
                out.push('L');
            }
            write!(out, "]").unwrap();
        }

        writeln!(out, r#""];"#).unwrap();

        for e in nodes[n].fanin.iter().chain(nodes[n].fanout.iter()) {
            let edge = &edges[*e];
            if seen_edges.insert(*e) {
                print_edge(out, edge, nodes);
            }
            if seen.insert(edge.driver) {
                stack.push(edge.driver);
            }
            if seen.insert(edge.drivee) {
                stack.push(edge.drivee);
            }
        }
    }
}

#[allow(unused)]
fn open_dot(s: &str) -> std::io::Result<()> {
    use std::process::{Command, Stdio};
    let mut dot = Command::new("dot")
        .args(["-Nshape=box", "-Tsvg", "-ofuse-signals.svg"])
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dot");

    dot.stdin
        .take()
        .expect("Failed to take dot stdin")
        .write_all(s.as_bytes())
        .expect("Failed to write to dot stdin");

    dot.wait().expect("Failed to wait on dot");

    let mut imv = Command::new("imv")
        .arg("fuse-signals.svg")
        .spawn()
        .expect("Failed to spawn imv");

    imv.wait().expect("Failed to wait on imv");
    Ok(())
}

pub struct FuseSignalsContext {
    pub print_unoptimized_fuse_signals: bool,
    pub print_round_fuse_signals: bool,
    pub print_optimized_fuse_signals: bool,
}

struct Edge {
    driver: NodeKey,
    drivee: NodeKey,
    driver_slice: SignalSlice,
    drivee_slice: SignalSlice,
}

enum NodeContent {
    Signal(SignalKey),
    Constant(Bits),
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct NodeFlags(u8);
impl NodeFlags {
    const EMPTY: Self = Self(0u8);
    const DRIVE: Self = Self(0b0001u8);
    const PROBE: Self = Self(0b0010u8);
    const LUPDT: Self = Self(0b0100u8);
    const WATCH: Self = Self(0b1000u8);

    #[inline(always)]
    fn contains(self, flags: NodeFlags) -> bool {
        self.0 & flags.0 != 0
    }
}

impl BitOr for NodeFlags {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for NodeFlags {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

struct Node {
    content: NodeContent,
    flags: NodeFlags,
    size: VectorSize,
    fanin: Vec<EdgeKey>,
    fanout: Vec<EdgeKey>,
}
type Nodes = Table<NodeKey, Node>;
type Edges = SlotMap<EdgeKey, Edge>;

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
/// | Property         | From                       | To                   |
/// |------------------|----------------------------|----------------------|
/// | Transitive       | A -> B -> C                | A -> B, A -> C       |
/// | Neighbour Merge  | A[0] -> B[0], A[1] -> B[1] | A[1:0] -> B[1:0]     |
/// | Cyclic           | A -> A                     |                      |
/// | Subset Inversion | A -> B[0], C -> B[1]       | B[0] -> A, B[1] -> C |
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
    ctx: &FuseSignalsContext,
) -> VgHashMap<SignalKey, (SignalKey, Option<SignalSlice>)> {
    let mut nodes = Nodes::default();
    let mut edges = Edges::default();
    let mut signal_to_node = VgHashMap::<SignalKey, NodeKey>::default();

    // Form the graph.
    for edge in connections.iter() {
        let (driver, driver_slice) = match &edge.driver {
            Driver::Signal(driver_signal, driver_slice) => {
                let driver = *signal_to_node.entry(*driver_signal).or_insert_with(|| {
                    nodes.insert(Node {
                        content: NodeContent::Signal(*driver_signal),
                        flags: NodeFlags::EMPTY,
                        size: gl.signals[*driver_signal].size,
                        fanin: Vec::new(),
                        fanout: Vec::new(),
                    })
                });
                let driver_slice = driver_slice
                    .unwrap_or_else(|| SignalSlice::with_end(gl.signals[*driver_signal].size));

                (driver, driver_slice)
            }
            Driver::Constant(value) => (
                nodes.insert(Node {
                    content: NodeContent::Constant(value.clone()),
                    flags: NodeFlags::DRIVE,
                    size: value.size(),
                    fanin: Vec::new(),
                    fanout: Vec::new(),
                }),
                SignalSlice::with_end(value.size()),
            ),
        };
        let drivee = *signal_to_node.entry(edge.drivee).or_insert_with(|| {
            nodes.insert(Node {
                content: NodeContent::Signal(edge.drivee),
                flags: NodeFlags::EMPTY,
                size: gl.signals[edge.drivee].size,
                fanin: Vec::new(),
                fanout: Vec::new(),
            })
        });
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

    for bb in gl.bbs.values() {
        for i in &bb.instrs {
            use Instruction as I;
            match &i {
                I::LastUpdateTime(_, s) => {
                    _ = signal_to_node
                        .get(s)
                        .map(|n| nodes[*n].flags |= NodeFlags::LUPDT)
                }
                I::Probe(_, s, _) | I::ProbeSlice(_, s, _) => {
                    _ = signal_to_node
                        .get(s)
                        .map(|n| nodes[*n].flags |= NodeFlags::PROBE)
                }
                I::Drive(s, _, _) => {
                    _ = signal_to_node
                        .get(s)
                        .map(|n| nodes[*n].flags |= NodeFlags::DRIVE)
                }
                _ => {}
            }
        }
        bb.terminator.for_each_signal(|s| {
            _ = signal_to_node
                .get(&s)
                .map(|n| nodes[*n].flags |= NodeFlags::WATCH)
        });
    }

    if ctx.print_unoptimized_fuse_signals {
        println!("// Unoptimized Fuse Signals");
        print(&gl.signals, &nodes, &edges);
        println!();
    }

    // Transform the graph until a fixed-point.
    let mut changed = true;
    let mut cyclic = Vec::new();
    let mut round = 0;
    while changed {
        round += 1;
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

                debug_assert_eq!(node, driver);
                if driver == drivee {
                    cyclic.push(i);
                    continue;
                }

                // If we rely on accurately knowing the last update of a signal, we cannot fuse a
                // view of a signal as the driver of the observed signal.
                if (drivee_slice.lsb() != 0 || drivee_slice.width() != nodes[drivee].size)
                    && nodes[drivee].flags.contains(NodeFlags::LUPDT)
                {
                    continue;
                }

                if nodes[drivee].fanout.is_empty() {
                    continue;
                }

                let mut drivee_fanout = std::mem::take(&mut nodes[drivee].fanout);
                let start_length = drivee_fanout.len();
                nodes[driver]
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
                changed |= start_length != drivee_fanout.len();
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
                nodes[node].fanout.truncate(write + 1);
            }

            // Subset inversion
            if nodes[node].fanin.len() > 1 && !nodes[node].flags.contains(NodeFlags::DRIVE) {
                nodes[node].fanin.sort_unstable_by_key(|&e| {
                    let edge = &edges[e];
                    (edge.drivee_slice.lsb(), edge.drivee_slice.width())
                });

                let node_width = nodes[node].size;
                let mut offset = 0;
                let mut i = 0;

                while i < nodes[node].fanin.len() {
                    let edge = &edges[nodes[node].fanin[i]];
                    if edge.drivee_slice.lsb() != 0
                        || edge.driver_slice.width() != nodes[edge.driver].size
                    {
                        break;
                    }

                    let driver_irr = nodes[edge.driver].flags;

                    if driver_irr.contains(NodeFlags::WATCH | NodeFlags::LUPDT)
                        || edge.drivee_slice.lsb() != offset
                        || !nodes[edge.driver].fanin.is_empty()
                    {
                        break;
                    }

                    offset += edge.drivee_slice.width().get();
                    i += 1;
                }

                if i == nodes[node].fanin.len() && offset == node_width.get() {
                    changed = true;
                    let fanin = std::mem::take(&mut nodes[node].fanin);
                    for &e in &fanin {
                        let edge = &mut edges[e];

                        // @Performance: Linear scan.
                        let idx = nodes[edge.driver]
                            .fanout
                            .iter()
                            .position(|&ie| ie == e)
                            .unwrap();
                        nodes[edge.driver].fanout.swap_remove(idx);
                        nodes[edge.driver].fanin.push(e);

                        std::mem::swap(&mut edge.driver, &mut edge.drivee);
                        std::mem::swap(&mut edge.driver_slice, &mut edge.drivee_slice);
                    }
                    nodes[node].fanout.extend(fanin);
                }
            }
        }

        if ctx.print_round_fuse_signals {
            println!("// Round {round} Fuse Signals");
            print(&gl.signals, &nodes, &edges);
            println!();
        }
    }

    // dbg!(nodes.len());
    // for node in nodes.table_key_iter() {
    //     if gl.signals[*nodes.get_key(node)]
    //         .name
    //         .starts_with("aes.EC.MX3.a2/1131")
    //     {
    //         dbg!(&gl.signals[*nodes.get_key(node)].name);
    //         let mut s = String::new();
    //         print_subgraph(
    //             &mut s,
    //             node,
    //             &gl.signals,
    //             &nodes,
    //             &edges,
    //             &ir_signal_reference,
    //         );
    //         println!("{s}");
    //     }
    // }

    if ctx.print_optimized_fuse_signals {
        println!("// Optimized Fuse Signals");
        print(&gl.signals, &nodes, &edges);
        println!();
    }

    let mut replacement_signals =
        VgHashMap::<SignalKey, (SignalKey, Option<SignalSlice>)>::default();

    for v in nodes.iter() {
        let NodeContent::Signal(signal) = v.content else {
            continue;
        };

        if v.fanin.is_empty() {
            continue;
        }

        if let Some(edge) = v.fanin.first().map(|&e| &edges[e])
            && v.fanin.len() == 1
            && edge.drivee_slice.lsb() == 0
            && edge.drivee_slice.width() == gl.signals[signal].size
            && let NodeContent::Signal(driver) = &nodes[edge.driver].content
        {
            let driver_slice = (edge.driver_slice.lsb() != 0
                || edge.driver_slice.width() != nodes[edge.driver].size)
                .then_some(edge.driver_slice);
            replacement_signals.insert(signal, (*driver, driver_slice));
            continue;
        }

        // If it is more complicated, we have to insert a process that propagated from driver to
        // drivee.
        let (_, mut builder) = new_process(gl, "fuse_signal".into(), TokenRange::default());
        let mut watch_signals = IndexMap::default();
        for &e in &v.fanin {
            let edge = &edges[e];
            let mut value = match &nodes[edge.driver].content {
                NodeContent::Signal(driver) => {
                    *watch_signals.get_or_insert_with(*driver, || builder.probe(gl, *driver))
                }
                NodeContent::Constant(value) => builder.constant(gl, value.clone()),
            };

            if edge.driver_slice.lsb() != 0 || edge.driver_slice.width() != nodes[edge.driver].size
            {
                value = builder.slice_constant(
                    gl,
                    value,
                    edge.driver_slice.lsb(),
                    edge.driver_slice.width(),
                );
            }
            builder.drive_partial_constant(
                gl,
                signal,
                value,
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
                I::Probe(dst, signal, offset) => {
                    if let Some((to, slice)) = replacement_signals.get(signal) {
                        match slice {
                            None => builder.push_raw_instruction(I::Probe(*dst, *to, *offset)),
                            Some(slice) => {
                                builder.probe_slice_constant(
                                    gl,
                                    *to,
                                    offset + slice.lsb(),
                                    slice.width(),
                                );
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
                I::ProbeSlice(dst, signal, offset) => {
                    if let Some((to, slice)) = replacement_signals.get(signal) {
                        match slice {
                            None => builder.push_raw_instruction(I::ProbeSlice(*dst, *to, *offset)),
                            Some(slice) => {
                                let offset =
                                    builder.plus_constant(gl, *offset, Bits::new_u32(slice.lsb()));
                                builder.probe_slice(gl, *to, offset, slice.width());
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
                I::LastUpdateTime(dst, signal) => {
                    let signal = replacement_signals.get(signal).map_or(*signal, |(s, _)| *s);
                    builder.push_raw_instruction(I::LastUpdateTime(*dst, signal));
                    continue;
                }
                I::Intrinsic(dst, op, srcs) => match op.as_ref() {
                    IntrinsicOp::ReadMem(readmem) => {
                        if let Some((to, slice)) = replacement_signals.get(&readmem.signal) {
                            let mut readmem = readmem.clone();
                            readmem.signal = *to;
                            readmem.offset += slice.map_or(0, |s| s.lsb());
                            builder.push_raw_instruction(I::Intrinsic(
                                *dst,
                                Box::new(IntrinsicOp::ReadMem(readmem)),
                                srcs.clone(),
                            ));
                            continue;
                        }
                    }
                    IntrinsicOp::VcdAppendModule(module) => {
                        let mut module = module.clone();
                        for value in module.table.values_mut() {
                            if let Some((to, slice)) = replacement_signals.get(&value.signal) {
                                value.signal = *to;
                                value.signal_slice = match (value.signal_slice, *slice) {
                                    (None, None) => None,
                                    (Some(s), None) | (None, Some(s)) => Some(s),
                                    (Some(vs), Some(os)) => Some(os.subslice(vs).unwrap()),
                                };
                            }
                        }
                        module.signal_map = module
                            .signal_map
                            .into_iter()
                            .map(|(s, v)| (replacement_signals.get(&s).map_or(s, |(s, _)| *s), v))
                            .collect();
                        builder.push_raw_instruction(I::Intrinsic(
                            *dst,
                            Box::new(IntrinsicOp::VcdAppendModule(module)),
                            srcs.clone(),
                        ));
                        continue;
                    }
                    _ => {}
                },

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
