use std::ops::{BitOr, BitOrAssign};

use hashbrown::hash_map::Entry;
use slotmap::SlotMap;
use vogls_ir::token_range::TokenRange;
use vogls_ir::vcd::VcdValue;
use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, Bits, GlobalContext, Instruction, IntrinsicOp, Signal,
    SignalKey, SignalSlice, VectorSize, new_process,
};
use vogls_utils::{IndexMap, Table, VgHashMap, VgHashSet};

vogls_utils::new_table_key! { struct NodeKey; }
vogls_utils::new_table_key! { struct EdgeKey; }

mod format;
#[cfg(test)]
mod tests;

#[cfg_attr(test, derive(PartialEq, Eq, Debug))]
pub enum FuseTarget {
    Signal(SignalKey, Option<SignalSlice>),
    Constant(Bits),
}

#[cfg_attr(test, derive(PartialEq, Eq, Debug, Clone))]
struct Edge {
    driver: NodeKey,
    drivee: NodeKey,
    driver_slice: SignalSlice,
    drivee_slice: SignalSlice,
}

#[derive(PartialEq, Eq, Debug)]
enum NodeContent {
    Signal(SignalKey),
    Constant(Bits),
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    fn remove(self, flags: NodeFlags) -> Self {
        Self(self.0 & !flags.0)
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

#[derive(Clone)]
pub enum Driver {
    Constant(Bits),
    Signal(SignalKey, Option<SignalSlice>),
}
impl Driver {
    pub fn size(&self, signals: &SlotMap<SignalKey, Signal>) -> VectorSize {
        match self {
            Driver::Constant(bits) => bits.size(),
            Driver::Signal(signal, slice) => {
                slice.map_or_else(|| signals[*signal].size, |s| s.width())
            }
        }
    }
}

#[derive(Clone)]
pub struct InputEdge {
    pub driver: Driver,
    pub drivee: SignalKey,
    pub drivee_slice: Option<SignalSlice>,
}

#[derive(Default)]
pub struct FuseGraph {
    nodes: Table<NodeKey, Node>,
    edges: Table<EdgeKey, Edge>,
    signal_to_node: VgHashMap<SignalKey, NodeKey>,
}

impl FuseGraph {
    pub fn from_connections(gl: &GlobalContext, connections: &[InputEdge]) -> Self {
        let mut g = FuseGraph::default();

        // Form the graph.
        for edge in connections.iter() {
            let (driver, driver_slice) = match &edge.driver {
                Driver::Signal(driver_signal, driver_slice) => {
                    let driver = *g.signal_to_node.entry(*driver_signal).or_insert_with(|| {
                        g.nodes.insert(Node {
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
                    g.nodes.insert(Node {
                        content: NodeContent::Constant(value.clone()),
                        flags: NodeFlags::DRIVE,
                        size: value.size(),
                        fanin: Vec::new(),
                        fanout: Vec::new(),
                    }),
                    SignalSlice::with_end(value.size()),
                ),
            };
            let drivee = *g.signal_to_node.entry(edge.drivee).or_insert_with(|| {
                g.nodes.insert(Node {
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

            let edge = g.edges.insert(Edge {
                driver,
                drivee,
                driver_slice,
                drivee_slice,
            });
            g.nodes[driver].fanout.push(edge);
            g.nodes[drivee].fanin.push(edge);
        }

        for bb in gl.bbs.values() {
            for i in &bb.instrs {
                use Instruction as I;
                match &i {
                    I::LastUpdateTime(_, s) => {
                        _ = g
                            .signal_to_node
                            .get(s)
                            .map(|n| g.nodes[*n].flags |= NodeFlags::LUPDT)
                    }
                    I::Probe(_, s, _) | I::ProbeSlice(_, s, _) => {
                        _ = g
                            .signal_to_node
                            .get(s)
                            .map(|n| g.nodes[*n].flags |= NodeFlags::PROBE)
                    }
                    I::Drive(s, _, _) => {
                        _ = g
                            .signal_to_node
                            .get(s)
                            .map(|n| g.nodes[*n].flags |= NodeFlags::DRIVE)
                    }
                    I::Intrinsic(_, i, _) => match i.as_ref() {
                        IntrinsicOp::Time
                        | IntrinsicOp::Finish
                        | IntrinsicOp::Random
                        | IntrinsicOp::Display(..)
                        | IntrinsicOp::Assert(..)
                        | IntrinsicOp::VcdOpenFile(..)
                        | IntrinsicOp::VcdAppendModule(..)
                        | IntrinsicOp::VcdPause
                        | IntrinsicOp::VcdResume => {}
                        IntrinsicOp::ReadMem(read_mem) => {
                            _ = g
                                .signal_to_node
                                .get(&read_mem.signal)
                                .map(|n| g.nodes[*n].flags |= NodeFlags::DRIVE);
                        }
                    },
                    _ => {}
                }
            }
            bb.terminator.for_each_signal(|s| {
                _ = g
                    .signal_to_node
                    .get(&s)
                    .map(|n| g.nodes[*n].flags |= NodeFlags::WATCH)
            });
        }
        g
    }
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
    connections: &[InputEdge],
) -> VgHashMap<SignalKey, FuseTarget> {
    let mut g = FuseGraph::from_connections(gl, connections);
    let mut marshalls = Vec::new();
    let mut fused_signals = VgHashMap::<SignalKey, FuseTarget>::default();
    let mut drive_map = VgHashMap::<SignalKey, (SignalKey, Option<SignalSlice>)>::default();

    g.optimize_till_fixed_point(&mut marshalls, &mut fused_signals, &mut drive_map);

    for n in marshalls {
        let NodeContent::Signal(signal) = &g.nodes[n].content else {
            unreachable!()
        };

        // If it is more complicated, we have to insert a process that propagated from driver to
        // drivee.
        let (_, mut builder) = new_process(gl, "fuse_signal".into(), TokenRange::default());
        let mut watch_signals = IndexMap::default();
        for &e in &g.nodes[n].fanin {
            let edge = &g.edges[e];
            let mut value = match &g.nodes[edge.driver].content {
                NodeContent::Signal(driver) => {
                    *watch_signals.get_or_insert_with(*driver, || builder.probe(gl, *driver))
                }
                NodeContent::Constant(value) => builder.constant(gl, value.clone()),
            };

            if edge.driver_slice.lsb() != 0
                || edge.driver_slice.width() != g.nodes[edge.driver].size
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
                *signal,
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
                    if let Some(tgt) = fused_signals.get(signal) {
                        match tgt {
                            FuseTarget::Signal(to, slice) => match slice {
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
                            },
                            FuseTarget::Constant(value) => {
                                builder.push_raw_instruction(I::Constant(*dst, value.clone()));
                            }
                        }
                        continue;
                    }
                }
                I::ProbeSlice(dst, signal, offset) => {
                    if let Some(tgt) = fused_signals.get(signal) {
                        match tgt {
                            FuseTarget::Signal(to, slice) => match slice {
                                None => {
                                    builder.push_raw_instruction(I::ProbeSlice(*dst, *to, *offset))
                                }
                                Some(slice) => {
                                    let offset = builder.plus_constant(
                                        gl,
                                        *offset,
                                        Bits::new_u32(slice.lsb()),
                                    );
                                    builder.probe_slice(gl, *to, offset, slice.width());
                                    builder
                                        .instrs
                                        .last_mut()
                                        .unwrap()
                                        .get_destination_variable_mut()
                                        .map(|d| *d = *dst);
                                }
                            },
                            FuseTarget::Constant(value) => {
                                builder.rev_imm_slice_x(
                                    gl,
                                    value.clone(),
                                    *offset,
                                    gl.vars[*dst].size,
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
                I::Drive(signal, src, partial) => {
                    if let Some((to, slice)) = drive_map.get(signal) {
                        let mut partial = *partial;
                        if let Some(slice) = slice {
                            partial = partial.map(|(o, m)| {
                                (
                                    builder.plus_constant(gl, o, Bits::new_u32(slice.lsb())),
                                    VectorSize::new(m.get() + slice.lsb()).unwrap(),
                                )
                            });
                        }
                        builder.push_raw_instruction(I::Drive(*to, *src, partial));
                    }
                }
                I::LastUpdateTime(dst, signal) => {
                    if let Some(tgt) = fused_signals.get(signal) {
                        match tgt {
                            FuseTarget::Signal(signal, slice) => {
                                // Fusing is only allowed when there is a LastUpdateTime
                                // instruction if the whole signal is fused.
                                assert!(slice.is_none());
                                builder.push_raw_instruction(I::LastUpdateTime(*dst, *signal));
                            }
                            FuseTarget::Constant(_) => {
                                // Constant signals are only assigned at the start.
                                builder.push_raw_instruction(I::Constant(*dst, Bits::new_u64(0)));
                            }
                        }
                        continue;
                    }
                }
                I::Intrinsic(dst, op, srcs) => match op.as_ref() {
                    IntrinsicOp::ReadMem(readmem) => {
                        if fused_signals.get(&readmem.signal).is_some() {
                            // You are only allowed to fuse items that don't have an external
                            // driver.
                            unreachable!("Implementation error");
                        }

                        if drive_map.get(&readmem.signal).is_some() {
                            todo!()
                        }
                    }
                    IntrinsicOp::VcdAppendModule(module) => {
                        let mut module = module.clone();
                        for vcd_var in module.table.values_mut() {
                            let VcdValue::Signal(vcd_signal, vcd_signal_slice) = &vcd_var.value
                            else {
                                continue;
                            };
                            if let Some(tgt) = fused_signals.get(vcd_signal) {
                                match tgt {
                                    FuseTarget::Signal(to, slice) => {
                                        vcd_var.value = VcdValue::Signal(
                                            *to,
                                            match (*vcd_signal_slice, *slice) {
                                                (None, None) => None,
                                                (Some(s), None) | (None, Some(s)) => Some(s),
                                                (Some(vs), Some(os)) => {
                                                    Some(os.subslice(vs).unwrap())
                                                }
                                            },
                                        );
                                    }
                                    FuseTarget::Constant(bits) => {
                                        vcd_var.value = VcdValue::Constant(bits.clone());
                                    }
                                }
                            }
                        }
                        for (s, tgt) in fused_signals.iter() {
                            let Some(items) = module.signal_map.remove(s) else {
                                continue;
                            };

                            match tgt {
                                FuseTarget::Signal(to, _) => match module.signal_map.entry(*to) {
                                    Entry::Occupied(mut e) => e.get_mut().extend(items.into_iter()),
                                    Entry::Vacant(e) => _ = e.insert(items),
                                },
                                FuseTarget::Constant(_) => {}
                            }
                        }
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
        if let BasicBlockTerminator::Watch(_, signals) = &mut gl.bbs[key].terminator {
            signals.retain_mut(|s| match fused_signals.get(s) {
                None => true,
                Some(FuseTarget::Signal(to, _)) => {
                    *s = *to;
                    true
                }
                Some(FuseTarget::Constant(_)) => false,
            });

            if signals.is_empty() {
                gl.bbs[key].terminator = BasicBlockTerminator::Halt;
            }
        }
    }

    fused_signals
}

impl FuseGraph {
    fn optimize_till_fixed_point(
        &mut self,
        marshalls: &mut Vec<NodeKey>,
        probe_fuse: &mut VgHashMap<SignalKey, FuseTarget>,
        drive_map: &mut VgHashMap<SignalKey, (SignalKey, Option<SignalSlice>)>,
    ) {
        let mut cyclic = Vec::new();
        let mut drive_inv_map = VgHashMap::<SignalKey, Vec<(SignalKey, SignalSlice)>>::default();
        let mut seen = VgHashSet::<NodeKey>::default();
        let mut active = Vec::new();
        let mut at;

        for n in self.nodes.table_key_iter() {
            if !seen.insert(n) {
                continue;
            }

            // Mark all nodes in a subgraph as active.
            //
            // We are assuming that there are many small subgraphs. Performing optimization
            // subgraph-by-subgraph, as opposed to node-by-node, should improve cache coherency
            // significantly and allows much easier observability since we can now see which
            // subgraphs contribute non-fusable signals.
            at = 0;
            active.push(n);
            while at != active.len() {
                for &e in &self.nodes[n].fanin {
                    let other = self.edges[e].driver;
                    if seen.insert(other) {
                        active.push(other);
                    }
                }
                for &e in &self.nodes[n].fanout {
                    let other = self.edges[e].drivee;
                    if seen.insert(other) {
                        active.push(other);
                    }
                }

                at += 1;
            }

            self.optimize_subgraph_until_fix_point(
                &active,
                drive_map,
                &mut drive_inv_map,
                &mut cyclic,
            );

            // Decide in this subgraph, what signals can be probe fused and what signals need to be
            // marshalled.
            {
                let FuseGraph {
                    nodes,
                    edges,
                    signal_to_node: _,
                } = self;

                for &k in active.iter() {
                    let v = &nodes[k];
                    let NodeContent::Signal(signal) = v.content else {
                        continue;
                    };

                    if v.fanin.is_empty() {
                        continue;
                    }

                    // C1: Node only has a single driver edge.
                    if v.flags.contains(NodeFlags::DRIVE) {
                        marshalls.push(k);
                        continue;
                    }
                    let &[e] = v.fanin.as_slice() else {
                        marshalls.push(k);
                        continue;
                    };

                    let edge = &edges[e];

                    // C2: Edge spans entire signal.
                    if edge.drivee_slice != SignalSlice::with_end(v.size) {
                        marshalls.push(k);
                        continue;
                    }

                    // C3: Either
                    // - signal is not observed for LastUpdateTime or Watched
                    // - or other the fused signal is used in its entirety
                    if v.flags.contains(NodeFlags::LUPDT | NodeFlags::WATCH)
                        && edge.driver_slice != SignalSlice::with_end(nodes[edge.driver].size)
                    {
                        marshalls.push(k);
                        continue;
                    }

                    let target = match &nodes[edge.driver].content {
                        NodeContent::Signal(driver) => FuseTarget::Signal(
                            *driver,
                            (edge.driver_slice != SignalSlice::with_end(nodes[edge.driver].size))
                                .then_some(edge.driver_slice),
                        ),
                        NodeContent::Constant(value) => FuseTarget::Constant(
                            value.slicez(edge.driver_slice.lsb(), edge.driver_slice.width()),
                        ),
                    };
                    probe_fuse.insert(signal, target);
                }
            }
        }
    }

    fn optimize_subgraph_until_fix_point(
        &mut self,
        active: &[NodeKey],
        drive_map: &mut VgHashMap<SignalKey, (SignalKey, Option<SignalSlice>)>,
        drive_inv_map: &mut VgHashMap<SignalKey, Vec<(SignalKey, SignalSlice)>>,
        cyclic: &mut Vec<usize>,
    ) {
        let Self {
            nodes,
            edges,
            signal_to_node: _,
        } = self;

        const MAX_ROUNDS: u32 = 1024;
        let mut reached_fixed_point = false;
        let mut round = 0;
        while !reached_fixed_point && round < MAX_ROUNDS {
            reached_fixed_point = true;
            round += 1;

            for &n in active {
                // Transitive + Cyclic Transform
                if !nodes[n].fanout.is_empty() {
                    let mut i = 0;
                    cyclic.clear();

                    while i < nodes[n].fanout.len() {
                        let e = nodes[n].fanout[i];
                        i += 1;

                        let Edge {
                            driver,
                            drivee,
                            driver_slice,
                            drivee_slice,
                        } = edges[e];

                        debug_assert_eq!(n, driver);
                        if driver == drivee {
                            cyclic.push(i - 1);
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
                                let Some(slice) = drivee_slice.relative_slice(edge.driver_slice)
                                else {
                                    return false;
                                };
                                let Some(slice) = slice.shift(driver_slice.lsb()) else {
                                    return false;
                                };

                                edges[e].driver = driver;
                                edges[e].driver_slice = slice;
                                true
                            }));
                        reached_fixed_point &= start_length == drivee_fanout.len();
                        nodes[drivee].fanout = drivee_fanout;
                    }

                    // Remove edges that point to the same node.
                    for &i in cyclic.iter() {
                        let e = nodes[n].fanout[i];
                        if edges[e].driver_slice == edges[e].drivee_slice {
                            reached_fixed_point = false;
                            nodes[n].fanout.swap_remove(i);
                            // @Performance: Linear scan.
                            let idx = nodes[n].fanin.iter().position(|&ie| ie == e).unwrap();
                            nodes[n].fanin.swap_remove(idx);
                        }
                    }
                }

                // Neighbour Merge Transform
                if nodes[n].fanout.len() > 1 {
                    nodes[n].fanout.sort_unstable_by_key(|&e| {
                        let edge = &edges[e];
                        (
                            edge.drivee,
                            edge.drivee_slice.lsb(),
                            edge.drivee_slice.msb(),
                        )
                    });

                    let mut read = 1;
                    let mut write = 0;
                    while read < nodes[n].fanout.len() {
                        let ledgekey = nodes[n].fanout[write];
                        let redgekey = nodes[n].fanout[read];
                        let ledge = &edges[ledgekey];
                        let redge = &edges[redgekey];
                        if ledge.drivee == redge.drivee
                            && let Some(fslice) = ledge.driver_slice.concat(redge.driver_slice)
                            && let Some(tslice) = ledge.drivee_slice.concat(redge.drivee_slice)
                        {
                            reached_fixed_point = false;
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
                        } else {
                            write += 1;
                            nodes[n].fanout[write] = nodes[n].fanout[read];
                        }
                        read += 1;
                    }
                    nodes[n].fanout.truncate(write + 1);
                }

                // Inversion
                if !nodes[n].flags.contains(NodeFlags::DRIVE)
                    && nodes[n].fanin.iter().any(|&e| {
                        edges[e].drivee_slice != SignalSlice::with_end(nodes[n].size)
                            && edges[e].driver_slice
                                == SignalSlice::with_end(nodes[edges[e].driver].size)
                    })
                {
                    let NodeContent::Signal(s) = nodes[n].content else {
                        unreachable!("Invariant: Constants don't have drivers");
                    };

                    let mut drive_inv = drive_inv_map.entry(s);
                    nodes[n].fanin.sort_unstable_by_key(|&e| {
                        let edge = &edges[e];
                        (edge.drivee_slice.lsb(), edge.drivee_slice.width())
                    });

                    let mut is_independent_from_prev = true;
                    let mut is_independent_from_next;
                    cyclic.clear();
                    for i in 0..nodes[n].fanin.len() {
                        let e = nodes[n].fanin[i];
                        let edge = &edges[e];
                        debug_assert_eq!(edge.drivee, n);

                        is_independent_from_next = nodes[n].fanin.get(i + 1).map_or(true, |&e| {
                            !edge.drivee_slice.overlaps(edges[e].drivee_slice)
                        });

                        let is_independent = is_independent_from_prev && is_independent_from_next;
                        is_independent_from_prev = is_independent_from_next;
                        if !is_independent {
                            continue;
                        }

                        let NodeContent::Signal(driver_signal) = nodes[edge.driver].content else {
                            continue;
                        };

                        if edge.driver_slice != SignalSlice::with_end(nodes[edge.driver].size) {
                            continue;
                        }

                        if let Entry::Occupied(i) = &drive_inv
                            && i.get()
                                .iter()
                                .any(|(_, slice)| edge.drivee_slice.overlaps(*slice))
                        {
                            continue;
                        }

                        let drivee_slice = edge.drivee_slice;

                        // 1. Remove edge from drivee fanin / driver fanout.
                        // 2. Add edge to drivee fanout / driver fanin
                        // 3. Drain driver fanin into drivee fanin.
                        // 4. Add drive map entry.
                        // 5. Copy Drv from driver.

                        drive_map.insert(driver_signal, (s, Some(edge.drivee_slice.clone())));
                        let mut occupied = match drive_inv {
                            Entry::Vacant(entry) => entry.insert_entry(Vec::new()),
                            Entry::Occupied(entry) => entry,
                        };
                        occupied
                            .get_mut()
                            .push((driver_signal, edge.drivee_slice.clone()));
                        drive_inv = Entry::Occupied(occupied);

                        let driver_fanin = std::mem::take(&mut nodes[edge.driver].fanin);
                        nodes[n].fanin.extend(driver_fanin.into_iter().map(|ek| {
                            edges[ek].drivee = n;
                            drivee_slice.subslice(edges[ek].drivee_slice);
                            ek
                        }));

                        let edge = &mut edges[e];

                        if nodes[edge.driver].flags.contains(NodeFlags::DRIVE) {
                            nodes[n].flags |= NodeFlags::DRIVE;
                            nodes[edge.driver].flags =
                                nodes[edge.driver].flags.remove(NodeFlags::DRIVE);
                        }

                        // A -> B     ~>      B -> A
                        dbg!(edge.driver);
                        dbg!(n);
                        nodes[edge.driver].fanin.push(e);
                        nodes[edge.driver].fanout.retain(|&ek| e != ek);
                        cyclic.push(i);
                        nodes[n].fanout.push(e);
                        std::mem::swap(&mut edge.driver, &mut edge.drivee);
                        std::mem::swap(&mut edge.driver_slice, &mut edge.drivee_slice);

                        dbg!(&nodes[edge.drivee].fanin);
                    }

                    if !cyclic.is_empty() {
                        let mut i = 0;
                        let mut j = 0;

                        nodes[n].fanin.retain(|_| {
                            let retain = cyclic.get(j).copied() != Some(i);

                            i += 1;
                            j += usize::from(!retain);

                            retain
                        });
                    }
                }
            }
        }
    }
}
