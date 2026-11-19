//! Aliasing of signals in the drive graph.
//!
//! A set of graph transformations is performed on the drive graph usually reducing its depth.
//!
//! | Property         | From                       | To                   |
//! |------------------|----------------------------|----------------------|
//! | Transitive       | A -> B -> C                | A -> B, A -> C       |
//! | Neighbour Merge  | A[0] -> B[0], A[1] -> B[1] | A[1:0] -> B[1:0]     |
//! | Cyclic           | A -> A                     |                      |
//! | Subset Inversion | A -> B[0], C -> B[1]       | B[0] -> A, B[1] -> C |
//!
//! Afterwards, many nodes only have a single incoming edge. The driver of that edge is used as an
//! alias for the node's signal. A process that is the only driver of some bits drives them
//! directly. All other driven nodes are lowered to a process.

use std::cmp;
use std::io::{self, Write};
use std::ops::{BitOr, BitOrAssign};

use vogls_ir::{
    BasicBlock, BasicBlockTerminator, Bits, GlobalContext, Instruction, IntrinsicOp, SignalFlags,
    SignalKey, SignalSlice,
};
use vogls_utils::{TableKey, VgHashMap, VgHashSet};

use super::{DelayKind, DriveEdgeKey, DriveNode, DriveNodeKey, DriverResolution, ExprSource};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeFlags(u8);

impl NodeFlags {
    pub(super) const EMPTY: Self = Self(0b0000_0000);
    /// Bits of the signal are written outside of the drive graph.
    pub(super) const DRIVE_EXT: Self = Self(0b0000_0001);
    /// The signal is probed outside of the design.
    const PROBE_EXT: Self = Self(0b0000_0010);
    /// The last update time of the signal is observed.
    pub(super) const LUPDT: Self = Self(0b0000_0100);
    /// The signal is watched by a process.
    pub(super) const WATCH: Self = Self(0b0000_1000);

    #[inline(always)]
    pub(super) fn contains(self, flags: NodeFlags) -> bool {
        self.0 & flags.0 == flags.0
    }

    #[inline(always)]
    fn intersects(self, flags: NodeFlags) -> bool {
        self.0 & flags.0 != 0
    }

    #[inline(always)]
    pub(super) fn remove(&mut self, flags: NodeFlags) {
        self.0 &= !flags.0;
    }
}

impl std::fmt::Display for NodeFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names = [
            (Self::DRIVE_EXT, "drive_ext"),
            (Self::PROBE_EXT, "probe_ext"),
            (Self::LUPDT, "lupdt"),
            (Self::WATCH, "watch"),
        ];
        let mut names = names.iter().filter(|(flag, _)| self.contains(*flag));
        write!(f, "[")?;
        if let Some((_, name)) = names.next() {
            write!(f, "{name}")?;
            for (_, name) in names {
                write!(f, " {name}")?;
            }
        }
        write!(f, "]")
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

/// What an aliased signal is replaced with when it is probed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FuseTarget {
    Signal(SignalKey, Option<SignalSlice>),
    Constant(Bits),
}

pub type ProbeFuseMap = VgHashMap<SignalKey, FuseTarget>;
/// Maps a signal to the bits of the signal that its writes go to instead, which are as many as
/// those of the signal. There are two kinds of entries, which are told apart by the kind of
/// signal:
///
/// - A variable that is inverted into a slice of a net: the slice is the variable from then on,
///   so every write goes there, its processes' drives as well as external writes.
/// - A net that is written externally and connected bidirectionally: its external writes set the
///   value that it drives, i.e. its drive proxy. The drive graph keeps driving the net itself, so
///   the drives in the IR are not redirected.
pub type WriteFuseMap = VgHashMap<SignalKey, (SignalKey, SignalSlice)>;

/// Makes sure that no alias targets an aliased signal, by following chains of aliases.
fn collapse_alias_chains(probe_fuse: &mut ProbeFuseMap) {
    let keys = probe_fuse.keys().copied().collect::<Vec<_>>();
    for key in keys {
        let mut target = probe_fuse.remove(&key).unwrap();
        while let FuseTarget::Signal(to, slice) = &target
            && let Some(next) = probe_fuse.get(to)
        {
            target = match next {
                FuseTarget::Signal(next_to, next_slice) => FuseTarget::Signal(
                    *next_to,
                    match (*slice, *next_slice) {
                        (None, s) | (s, None) => s,
                        (Some(s), Some(next_s)) => Some(next_s.subslice(s).unwrap()),
                    },
                ),
                FuseTarget::Constant(value) => FuseTarget::Constant(match slice {
                    None => value.clone(),
                    Some(s) => value.slicez(s.lsb(), s.width()),
                }),
            };
        }
        probe_fuse.insert(key, target);
    }
}

/// Makes sure that no write redirect targets a redirected signal, by following chains.
fn collapse_write_chains(write_fuse: &mut WriteFuseMap) {
    let keys = write_fuse.keys().copied().collect::<Vec<_>>();
    for key in keys {
        let (mut to, mut slice) = write_fuse.remove(&key).unwrap();
        while let Some(&(next_to, next_slice)) = write_fuse.get(&to) {
            to = next_to;
            slice = next_slice.subslice(slice).unwrap();
        }
        write_fuse.insert(key, (to, slice));
    }
}

#[derive(Default)]
struct AliasScratch {
    ins: Vec<DriveEdgeKey>,
    outs: Vec<DriveEdgeKey>,
    moved: Vec<DriveEdgeKey>,
}

impl<'a> DriverResolution<'a> {
    // ---------------------------------------------------------------------------------------------
    // Edge list manipulation
    // ---------------------------------------------------------------------------------------------

    pub(super) fn src_slice(&self, edge: DriveEdgeKey) -> SignalSlice {
        let edge = &self.edges[edge];
        SignalSlice::from_width(edge.from_offset, edge.bit_length).unwrap()
    }

    pub(super) fn dst_slice(&self, edge: DriveEdgeKey) -> SignalSlice {
        let edge = &self.edges[edge];
        SignalSlice::from_width(edge.to_offset, edge.bit_length).unwrap()
    }

    fn full_slice(&self, node: DriveNodeKey) -> SignalSlice {
        SignalSlice::with_end(self.nodes[node].bit_length())
    }

    fn unlink_in(&mut self, edge: DriveEdgeKey) {
        let e = &self.edges[edge];
        let (to, prev, next) = (e.to, e.prev_in, e.next_in);
        match prev {
            Some(prev) => self.edges[prev].next_in = next,
            None => self.nodes[to].set_fst_in(next),
        }
        if let Some(next) = next {
            self.edges[next].prev_in = prev;
        }
        let e = &mut self.edges[edge];
        e.prev_in = None;
        e.next_in = None;
    }

    pub(super) fn unlink_out(&mut self, edge: DriveEdgeKey) {
        let e = &self.edges[edge];
        let (from, prev, next) = (e.from, e.prev_out, e.next_out);
        match prev {
            Some(prev) => self.edges[prev].next_out = next,
            None => self.nodes[from].set_fst_out(next),
        }
        if let Some(next) = next {
            self.edges[next].prev_out = prev;
        }
        let e = &mut self.edges[edge];
        e.prev_out = None;
        e.next_out = None;
    }

    fn link_in(&mut self, edge: DriveEdgeKey, node: DriveNodeKey) {
        let next = self.nodes[node].fst_in();
        let e = &mut self.edges[edge];
        e.to = node;
        e.prev_in = None;
        e.next_in = next;
        if let Some(next) = next {
            self.edges[next].prev_in = Some(edge);
        }
        self.nodes[node].set_fst_in(Some(edge));
    }

    fn link_out(&mut self, edge: DriveEdgeKey, node: DriveNodeKey) {
        let next = self.nodes[node].fst_out();
        let e = &mut self.edges[edge];
        e.from = node;
        e.prev_out = None;
        e.next_out = next;
        if let Some(next) = next {
            self.edges[next].prev_out = Some(edge);
        }
        self.nodes[node].set_fst_out(Some(edge));
    }

    pub(super) fn remove_edge(&mut self, edge: DriveEdgeKey) {
        self.unlink_in(edge);
        self.unlink_out(edge);
    }

    pub(super) fn collect_in(&self, node: DriveNodeKey, edges: &mut Vec<DriveEdgeKey>) {
        edges.clear();
        let mut next = self.nodes[node].fst_in();
        while let Some(edge) = next {
            edges.push(edge);
            next = self.edges[edge].next_in;
        }
    }

    pub(super) fn collect_out(&self, node: DriveNodeKey, edges: &mut Vec<DriveEdgeKey>) {
        edges.clear();
        let mut next = self.nodes[node].fst_out();
        while let Some(edge) = next {
            edges.push(edge);
            next = self.edges[edge].next_out;
        }
    }

    /// Moves all reads of `from` to `to[offset..]`, as `from` is aliased to that slice of `to`.
    fn move_reads(&mut self, from: DriveNodeKey, to: DriveNodeKey, offset: u32) {
        let Some(head) = self.nodes[from].fst_read_out() else {
            return;
        };
        self.nodes[from].set_fst_read_out(None);

        let mut tail = head;
        loop {
            let edge = &mut self.read_edges[tail];
            edge.from = to;
            edge.from_offset += offset;
            match edge.next_out {
                Some(next) => tail = next,
                None => break,
            }
        }

        self.read_edges[tail].next_out = self.nodes[to].fst_read_out();
        self.nodes[to].set_fst_read_out(Some(head));
    }

    /// Whether the process of `process` reads `node`.
    fn reads(&self, process: DriveNodeKey, node: DriveNodeKey) -> bool {
        let mut next = self.nodes[process].fst_read_in();
        while let Some(edge) = next {
            if self.read_edges[edge].from == node {
                return true;
            }
            next = self.read_edges[edge].next_in;
        }

        // The process of a delay probes its input, which may be aliased to `node` through others.
        let mut next = self.nodes[process].fst_in();
        while let Some(edge) = next {
            if self.alias_end(self.edges[edge].from, 0).0 == node {
                return true;
            }
            next = self.edges[edge].next_in;
        }
        false
    }

    // ---------------------------------------------------------------------------------------------
    // Flags
    // ---------------------------------------------------------------------------------------------

    /// Determines how every net and variable is used outside of the drive graph.
    pub(super) fn collect_flags(&mut self, gl: &GlobalContext) {
        for node in self.nodes.values_mut() {
            let (DriveNode::Net { key, flags, .. } | DriveNode::Variable { key, flags, .. }) = node
            else {
                continue;
            };
            *flags = NodeFlags::EMPTY;
            let signal = &gl.signals[*key];
            if signal.flags.contains(SignalFlags::EXT_DRIVE) {
                *flags |= NodeFlags::DRIVE_EXT;
            }
            if signal.flags.contains(SignalFlags::EXT_PROBE) {
                *flags |= NodeFlags::PROBE_EXT;
            }
        }

        for bb in gl.bbs.values() {
            for i in &bb.instrs {
                use Instruction as I;
                if let I::LastUpdateTime(_, s) = i {
                    self.mark(*s, NodeFlags::LUPDT);
                }
            }
            use BasicBlockTerminator as T;
            match &bb.terminator {
                T::Watch(_, conditions) => conditions
                    .iter()
                    .for_each(|c| self.mark(c.signal, NodeFlags::WATCH)),
                T::Wait(..)
                | T::VariableWait(..)
                | T::WaitRegion(..)
                | T::Jump(..)
                | T::Branch(..)
                | T::Halt => {}
            }
        }

        for (signal, flag) in std::mem::take(&mut self.observed) {
            self.mark(signal, flag);
        }

        if cfg!(debug_assertions) {
            self.assert_nets_not_driven(gl.bbs.values());
        }
    }

    /// Asserts that no instruction of `bbs` drives a net, which only the drive graph does.
    pub(super) fn assert_nets_not_driven<'b>(&self, bbs: impl Iterator<Item = &'b BasicBlock>) {
        for bb in bbs {
            for i in &bb.instrs {
                use Instruction as I;
                match &i {
                    I::Drive(_, s, _, _) | I::DriveSlice(_, s, _, _) => {
                        if let Some(&node) = self.signal_lut.get(s) {
                            assert!(
                                !matches!(self.nodes[node], DriveNode::Net { .. }),
                                "nets are only driven through the drive graph"
                            );
                        }
                    }
                    I::Intrinsic(_, i, _) => match i.as_ref() {
                        IntrinsicOp::Time
                        | IntrinsicOp::Finish
                        | IntrinsicOp::Random(_)
                        | IntrinsicOp::Display(..)
                        | IntrinsicOp::Assert(..)
                        | IntrinsicOp::VcdOpenFile(..)
                        | IntrinsicOp::VcdAppendModule(..)
                        | IntrinsicOp::VcdPause
                        | IntrinsicOp::BlackBox
                        | IntrinsicOp::VcdResume
                        | IntrinsicOp::SetTimeFormat(_) => {}

                        IntrinsicOp::ReadMem(read_mem) => {
                            if let Some(&node) = self.signal_lut.get(&read_mem.signal) {
                                assert!(
                                    !matches!(self.nodes[node], DriveNode::Net { .. }),
                                    "nets are only driven through the drive graph"
                                );
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    fn mark(&mut self, signal: SignalKey, flag: NodeFlags) {
        if let Some(&node) = self.signal_lut.get(&signal)
            && let Some(flags) = self.nodes[node].flags_mut()
        {
            *flags |= flag;
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Aliasing
    // ---------------------------------------------------------------------------------------------

    /// Simplifies the drive graph and decides which signals are aliased, which processes drive
    /// their drivee directly and which nodes are lowered.
    ///
    /// Without `optimize`, the graph is lowered as it is: nothing is aliased, every process drives
    /// a proxy and every driven node resolves its drivers.
    pub(super) fn alias(&mut self, gl: &GlobalContext, max_rounds: usize, optimize: bool) {
        let mut drive_inv_map = VgHashMap::<SignalKey, Vec<(SignalKey, SignalSlice)>>::default();
        let mut seen = VgHashSet::<DriveNodeKey>::default();
        let mut subgraph = Vec::new();
        let mut scratch = AliasScratch::default();

        for n in self.nodes.table_key_iter() {
            if !seen.insert(n) {
                continue;
            }

            // Optimize subgraph-by-subgraph, as opposed to node-by-node. We are assuming that there
            // are many small subgraphs, so this should improve cache coherency significantly.
            let mut at = 0;
            subgraph.clear();
            subgraph.push(n);
            while at != subgraph.len() {
                let node = subgraph[at];
                let mut next = self.nodes[node].fst_in();
                while let Some(edge) = next {
                    let other = self.edges[edge].from;
                    if seen.insert(other) {
                        subgraph.push(other);
                    }
                    next = self.edges[edge].next_in;
                }
                let mut next = self.nodes[node].fst_out();
                while let Some(edge) = next {
                    let other = self.edges[edge].to;
                    if seen.insert(other) {
                        subgraph.push(other);
                    }
                    next = self.edges[edge].next_out;
                }
                at += 1;
            }

            if optimize {
                self.optimize_subgraph(gl, &subgraph, &mut drive_inv_map, &mut scratch, max_rounds);
                self.decide_aliases(gl, &subgraph);
                self.decide_direct(&subgraph, &mut scratch);
            }
            self.decide_lowered(&subgraph, &mut scratch);
        }

        collapse_alias_chains(&mut self.probe_fuse);
        collapse_write_chains(&mut self.write_fuse);
    }

    /// Aliases every node that has a single driver which spans the entire node to that driver.
    fn decide_aliases(&mut self, gl: &GlobalContext, subgraph: &[DriveNodeKey]) {
        for &k in subgraph {
            let Some(signal) = self.nodes[k].drivee_signal() else {
                continue;
            };
            let Some(e) = self.nodes[k].fst_in() else {
                continue;
            };
            let k_flags = self.nodes[k].flags();
            let is_delay = matches!(self.nodes[k], DriveNode::Delay { .. });

            // C1: The signal does not hold a value of its own.
            if k_flags.contains(NodeFlags::DRIVE_EXT) {
                continue;
            }

            // C2: Node has a single driver edge.
            if self.edges[e].next_in.is_some() {
                continue;
            }

            // C3: Edge spans entire signal.
            if self.dst_slice(e) != self.full_slice(k) {
                continue;
            }

            // C4: Either
            // - signal is not observed for LastUpdateTime or watched
            // - or the fused signal is used in its entirety
            //
            // A delay observes the last update time of its input.
            let driver = self.edges[e].from;
            let from_slice = self.src_slice(e);
            let partial = from_slice != self.full_slice(driver);
            let target = match &self.nodes[driver] {
                // Probes are redirected as they are, so only to a signal of the same mode. A probe
                // of a constant is converted to the mode of the probe.
                DriveNode::Net { key, .. } | DriveNode::Variable { key, .. }
                    if gl.signals[*key].mode == gl.signals[signal].mode =>
                {
                    FuseTarget::Signal(*key, partial.then_some(from_slice))
                }
                DriveNode::Net { .. } | DriveNode::Variable { .. } => continue,
                DriveNode::Constant { value, .. } if !is_delay => {
                    FuseTarget::Constant(value.slicez(from_slice.lsb(), from_slice.width()))
                }
                DriveNode::Constant { .. } | DriveNode::Expr { .. } | DriveNode::Delay { .. } => {
                    continue;
                }
            };

            // C5: The aliases do not form a cycle. The aliases are chained (see
            // `collapse_alias_chains`) when the drive graph is not fully simplified, e.g.
            // `assign a = b, b = a` would alias `a` to `b` and `b` to `a`.
            let (end, offset) = self.alias_end(driver, from_slice.lsb());
            if end == k {
                continue;
            }

            let mut observed = NodeFlags::EMPTY;
            for flag in [NodeFlags::LUPDT, NodeFlags::WATCH] {
                if k_flags.contains(flag) {
                    observed |= flag;
                }
            }
            if is_delay {
                observed |= NodeFlags::LUPDT;
            }
            if observed != NodeFlags::EMPTY {
                if partial {
                    continue;
                }
                // The driver must not be aliased to a slice either, as the aliases are chained.
                // Otherwise, the driver is observed through `k` from now on, which keeps it from
                // being aliased to a slice later.
                let Some(end) = self.whole_alias_end(driver) else {
                    continue;
                };
                if let Some(flags) = self.nodes[end].flags_mut() {
                    *flags |= observed;
                }
            }

            self.probe_fuse.insert(signal, target);
            self.move_reads(k, end, offset);
        }
    }

    /// Follows the aliases of the bits of `node` at `offset` to the node that holds them. An
    /// aliased node keeps the edge of its alias, which spans the node.
    fn alias_end(&self, mut node: DriveNodeKey, mut offset: u32) -> (DriveNodeKey, u32) {
        while let Some(signal) = self.nodes[node].drivee_signal()
            && self.probe_fuse.contains_key(&signal)
        {
            let e = self.nodes[node]
                .fst_in()
                .expect("an aliased node keeps the edge of its alias");
            offset += self.edges[e].from_offset;
            node = self.edges[e].from;
        }
        (node, offset)
    }

    /// Follows the aliases of `node` to the node that it is aliased to in its entirety, or returns
    /// `None` if one of the aliases is to a slice.
    fn whole_alias_end(&self, mut node: DriveNodeKey) -> Option<DriveNodeKey> {
        while let Some(key) = self.nodes[node].driver_signal()
            && let Some(target) = self.probe_fuse.get(&key)
        {
            match target {
                FuseTarget::Signal(to, None) => node = self.signal_lut[to],
                FuseTarget::Signal(_, Some(_)) => return None,
                FuseTarget::Constant(_) => break,
            }
        }
        Some(node)
    }

    /// Lets every process that is the only driver of some bits of a node drive them directly,
    /// unless the process reads the node.
    fn decide_direct(&mut self, subgraph: &[DriveNodeKey], scratch: &mut AliasScratch) {
        for &k in subgraph {
            let Some(signal) = self.nodes[k].drivee_signal() else {
                continue;
            };
            if self.probe_fuse.contains_key(&signal) {
                continue;
            }

            self.collect_in(k, &mut scratch.ins);
            for &e in &scratch.ins {
                let p = self.edges[e].from;
                if !self.nodes[p].is_process() {
                    continue;
                }

                let slice = self.dst_slice(e);
                if scratch
                    .ins
                    .iter()
                    .any(|&other| other != e && self.dst_slice(other).overlaps(slice))
                {
                    continue;
                }
                if self.reads(p, k) {
                    continue;
                }

                self.direct.insert(e);
            }
        }
    }

    /// Lets the process of an assignment drive the wiring of the assignment into the nodes it
    /// drives directly, and lowers every node with remaining drivers.
    fn decide_lowered(&mut self, subgraph: &[DriveNodeKey], scratch: &mut AliasScratch) {
        // The signals that the expressions of each assignment drive directly.
        let mut expr_drivees = VgHashMap::<_, Vec<SignalKey>>::default();
        for &k in subgraph {
            let mut next = self.nodes[k].fst_in();
            while let Some(e) = next {
                let edge = &self.edges[e];
                next = edge.next_in;
                if let DriveNode::Expr { source, .. } = self.nodes[edge.from]
                    && let Some(assignment) = source.assignment()
                    && self.direct.contains(&e)
                {
                    let drivee = self.nodes[k].drivee_signal().unwrap();
                    expr_drivees.entry(assignment).or_default().push(drivee);
                }
            }
        }

        for &k in subgraph {
            let Some(signal) = self.nodes[k].drivee_signal() else {
                continue;
            };
            if self.probe_fuse.contains_key(&signal) {
                continue;
            }

            self.collect_in(k, &mut scratch.ins);
            let mut lowered = false;
            for &e in &scratch.ins {
                if self.direct.contains(&e) {
                    continue;
                }

                let edge = &self.edges[e];
                let wired = edge.assignment.is_some_and(|assignment| {
                    let Some(drivees) = expr_drivees.get(&assignment) else {
                        return false;
                    };
                    if !drivees.contains(&signal) {
                        return false;
                    }
                    let source = match &self.nodes[edge.from] {
                        DriveNode::Constant { .. } => None,
                        DriveNode::Net { key, .. } | DriveNode::Variable { key, .. } => Some(*key),
                        DriveNode::Expr { .. } | DriveNode::Delay { .. } => return false,
                    };
                    let slice = self.dst_slice(e);
                    let is_reading_drivee = source.is_some_and(|key| {
                        super::resolve_alias(
                            &self.probe_fuse,
                            key,
                            edge.from_offset,
                            edge.bit_length,
                        )
                        .is_ok_and(|(key, _)| drivees.contains(&key))
                    });
                    !is_reading_drivee
                        && !scratch
                            .ins
                            .iter()
                            .any(|&other| other != e && self.dst_slice(other).overlaps(slice))
                });

                if wired {
                    let assignment = edge.assignment.unwrap();
                    self.expr_wires.entry(assignment).or_default().push(e);
                } else {
                    lowered = true;
                }
            }

            if lowered {
                self.lower.push(k);
            }
        }
    }

    fn optimize_subgraph(
        &mut self,
        gl: &GlobalContext,
        active: &[DriveNodeKey],
        drive_inv_map: &mut VgHashMap<SignalKey, Vec<(SignalKey, SignalSlice)>>,
        scratch: &mut AliasScratch,
        max_rounds: usize,
    ) {
        let mut reached_fixed_point = false;
        let mut round = 0;
        while !reached_fixed_point && round < max_rounds {
            reached_fixed_point = true;
            round += 1;

            for &n in active {
                // Transitive Transform
                if !self.transitive(gl, n, scratch, &mut reached_fixed_point) {
                    continue;
                }
                self.cyclic(n, scratch, &mut reached_fixed_point);
                self.neighbour_merge(n, scratch, &mut reached_fixed_point);
                self.constant_merge(n, scratch, &mut reached_fixed_point);
                if reached_fixed_point {
                    self.inversion(gl, n, drive_inv_map, scratch, &mut reached_fixed_point);
                }
            }

            self.validate(active);
        }
    }

    /// `A -> B -> C` ~> `A -> B, A -> C`
    ///
    /// A value is converted to the mode of each signal it is stored in, so `A -> C` skips the
    /// conversion to the mode of `B`. That is only lossless if `B` is four-valued or `C` converts to
    /// two-valued logic anyway.
    ///
    /// Returns `false` if the incoming edges of `n` overlap, in which case no other transformation
    /// should be applied to `n` this round.
    fn transitive(
        &mut self,
        gl: &GlobalContext,
        n: DriveNodeKey,
        scratch: &mut AliasScratch,
        reached_fixed_point: &mut bool,
    ) -> bool {
        let DriveNode::Net { key, .. } = self.nodes[n] else {
            return true;
        };
        let is_two_value = gl.signals[key].mode.is_two_value();
        if self.nodes[n].flags().contains(NodeFlags::DRIVE_EXT)
            || self.nodes[n].fst_in().is_none()
            || self.nodes[n].fst_out().is_none()
        {
            return true;
        }

        let AliasScratch { ins, outs, .. } = scratch;

        self.collect_in(n, ins);
        ins.sort_unstable_by_key(|&e| {
            let s = self.dst_slice(e);
            (s.lsb(), s.msb())
        });
        if ins
            .windows(2)
            .any(|w| self.dst_slice(w[0]).overlaps(self.dst_slice(w[1])))
        {
            return false;
        }

        self.collect_out(n, outs);
        outs.sort_unstable_by_key(|&e| {
            let s = self.src_slice(e);
            (s.lsb(), s.msb())
        });

        let mut i = 0;
        let mut o = 0;
        while i < ins.len() && o < outs.len() {
            let (ie, oe) = (ins[i], outs[o]);

            // There are four slices involved in a transitive edge. We label them in order.
            let s1 = self.src_slice(ie);
            let s2 = self.dst_slice(ie);
            let s3 = self.src_slice(oe);

            let pred = self.edges[ie].from;
            let suc = self.edges[oe].to;
            let skips_conversion = is_two_value
                && self.nodes[suc]
                    .drivee_signal()
                    .is_some_and(|s| gl.signals[s].mode.is_four_value());

            if suc == n || skips_conversion || s2.lsb() > s3.msb() {
                o += 1;
            } else if s3.lsb() > s2.msb() {
                i += 1;
            } else if let Some(s2_r_s3) = s2.relative_slice(s3) {
                outs.remove(o);
                self.unlink_out(oe);
                self.edges[oe].from_offset = s1.subslice(s2_r_s3).unwrap().lsb();
                self.link_out(oe, pred);
                *reached_fixed_point = false;
            } else {
                o += 1;
            }
        }

        true
    }

    /// `A -> A` ~> ``
    ///
    /// Only for nets: a delay that drives its own input is a loop through the delay.
    fn cyclic(
        &mut self,
        n: DriveNodeKey,
        scratch: &mut AliasScratch,
        reached_fixed_point: &mut bool,
    ) {
        if !matches!(self.nodes[n], DriveNode::Net { .. }) {
            return;
        }
        self.collect_in(n, &mut scratch.ins);
        for &e in &scratch.ins {
            let edge = &self.edges[e];
            if edge.from == edge.to && edge.from_offset == edge.to_offset {
                self.remove_edge(e);
                *reached_fixed_point = false;
            }
        }
    }

    /// `A[0] -> B[0], A[1] -> B[1]` ~> `A[1:0] -> B[1:0]`
    fn neighbour_merge(
        &mut self,
        n: DriveNodeKey,
        scratch: &mut AliasScratch,
        reached_fixed_point: &mut bool,
    ) {
        let outs = &mut scratch.outs;
        self.collect_out(n, outs);
        if outs.len() <= 1 {
            return;
        }

        outs.sort_unstable_by_key(|&e| {
            let s = self.dst_slice(e);
            (self.edges[e].to, s.lsb(), s.msb())
        });

        let mut l = outs[0];
        for &r in &outs[1..] {
            if self.edges[l].to == self.edges[r].to
                && let Some(from) = self.src_slice(l).concat(self.src_slice(r))
                && let Some(to) = self.dst_slice(l).concat(self.dst_slice(r))
            {
                *reached_fixed_point = false;
                let edge = &mut self.edges[l];
                edge.from_offset = from.lsb();
                edge.to_offset = to.lsb();
                edge.bit_length = from.width();
                self.remove_edge(r);
            } else {
                l = r;
            }
        }
    }

    /// Merge adjacent constants driving the same node into one constant.
    fn constant_merge(
        &mut self,
        n: DriveNodeKey,
        scratch: &mut AliasScratch,
        reached_fixed_point: &mut bool,
    ) {
        let AliasScratch { ins, outs, .. } = scratch;
        self.collect_in(n, ins);
        let num_constants = ins
            .iter()
            .filter(|&&e| matches!(self.nodes[self.edges[e].from], DriveNode::Constant { .. }))
            .count();
        if num_constants <= 1 {
            return;
        }

        ins.sort_unstable_by_key(|&e| {
            let s = self.dst_slice(e);
            (s.lsb(), s.width())
        });

        for i in 1..ins.len() {
            let (l, r) = (ins[i - 1], ins[i]);
            let (ln, rn) = (self.edges[l].from, self.edges[r].from);

            // C1: Both edges are constant drivers that do not sign-extend.
            let (DriveNode::Constant { value: lb, .. }, DriveNode::Constant { value: rb, .. }) =
                (&self.nodes[ln], &self.nodes[rn])
            else {
                continue;
            };
            if ln == rn {
                continue;
            }

            // C2: Edges are adjacent.
            if self.dst_slice(l).msb() + 1 != self.dst_slice(r).lsb() {
                continue;
            }

            // C3: Both edges span their entire constant.
            if self.src_slice(l) != SignalSlice::with_end(lb.size())
                || self.src_slice(r) != SignalSlice::with_end(rb.size())
            {
                continue;
            }

            let lsize = lb.size();
            let bits = Bits::concatenate(rb, lb);
            let bit_length = bits.size();
            let to_lsb = self.dst_slice(l).lsb();

            // Shift over all edges R -> ..
            self.collect_out(rn, outs);
            for &e in outs.iter() {
                self.edges[e].from_offset += lsize.get();
            }

            let DriveNode::Constant { value, .. } = &mut self.nodes[rn] else {
                unreachable!();
            };
            *value = bits;

            // Make edge encompass L -> N and R -> N.
            let edge = &mut self.edges[r];
            edge.from_offset = 0;
            edge.to_offset = to_lsb;
            edge.bit_length = bit_length;

            // Remove edge L -> N. The reads of L are those of its bits in R now.
            self.remove_edge(l);
            let mut next = self.nodes[rn].fst_read_out();
            while let Some(e) = next {
                self.read_edges[e].from_offset += lsize.get();
                next = self.read_edges[e].next_out;
            }
            self.move_reads(ln, rn, 0);

            *reached_fixed_point = false;
        }
    }

    /// `A -> B[0], C -> B[1]` ~> `B[0] -> A, B[1] -> C`
    ///
    /// The drivers of `A` and `C` drive `B` directly through the drive map. A variable has no
    /// drivers, so its process writes `B` directly instead.
    fn inversion(
        &mut self,
        gl: &GlobalContext,
        n: DriveNodeKey,
        drive_inv_map: &mut VgHashMap<SignalKey, Vec<(SignalKey, SignalSlice)>>,
        scratch: &mut AliasScratch,
        reached_fixed_point: &mut bool,
    ) {
        let DriveNode::Net { key: s, .. } = self.nodes[n] else {
            return;
        };
        // The drivers would observe the external writes of `B`. The `DRIVE_EXT` flag of the node
        // cannot tell, as it is also set by inverting a variable into a slice of `B`.
        if gl.signals[s].flags.contains(SignalFlags::EXT_DRIVE) {
            return;
        }

        let AliasScratch { ins, moved, .. } = scratch;
        self.collect_in(n, ins);
        let n_full = self.full_slice(n);
        if !ins.iter().any(|&e| {
            self.dst_slice(e) != n_full && self.src_slice(e) == self.full_slice(self.edges[e].from)
        }) {
            return;
        }

        ins.sort_unstable_by_key(|&e| {
            let s = self.dst_slice(e);
            (s.lsb(), s.width())
        });

        // The most significant bit driven by the edges before. Any earlier edge may be the one that
        // overlaps, e.g. `[3:0]` overlaps `[2:2]` across `[1:1]`. Of the later edges, the next one
        // starts first.
        let mut prev_msb = None;
        for i in 0..ins.len() {
            let e = ins[i];
            let drivee_slice = self.dst_slice(e);

            let is_independent_from_prev = prev_msb.is_none_or(|msb| msb < drivee_slice.lsb());
            let is_independent_from_next = ins
                .get(i + 1)
                .is_none_or(|&next| !drivee_slice.overlaps(self.dst_slice(next)));
            prev_msb =
                Some(prev_msb.map_or(drivee_slice.msb(), |msb| cmp::max(msb, drivee_slice.msb())));
            if !is_independent_from_prev || !is_independent_from_next {
                continue;
            }

            let driver = self.edges[e].from;
            let (driver_signal, driver_flags, is_variable) = match self.nodes[driver] {
                DriveNode::Net { key, flags, .. } => (key, flags, false),
                DriveNode::Variable { key, flags, .. } => (key, flags, true),
                DriveNode::Constant { .. } | DriveNode::Expr { .. } | DriveNode::Delay { .. } => {
                    continue;
                }
            };
            if driver_flags.contains(NodeFlags::DRIVE_EXT) {
                continue;
            }

            // The drivers of `A` are resolved in `B` and `A` is probed through `B` (or driven from
            // it) afterwards, so both have to be of the same mode.
            if gl.signals[driver_signal].mode != gl.signals[s].mode {
                continue;
            }

            if self.src_slice(e) != self.full_slice(driver) {
                continue;
            }

            if self.edges[e].next_out.is_some() || self.edges[e].prev_out.is_some() {
                continue;
            }

            if drive_inv_map
                .get(&s)
                .is_some_and(|inv| inv.iter().any(|(_, slice)| drivee_slice.overlaps(*slice)))
            {
                continue;
            }

            let slice = (drivee_slice != n_full).then_some(drivee_slice);
            if is_variable {
                // The variable is probed through a slice of `B` and its initial value becomes that
                // of the slice. Both have to keep their last update time.
                let n_flags = self.nodes[n].flags();
                let poke_t0 = gl.signals[driver_signal]
                    .flags
                    .contains(SignalFlags::POKE_T0);
                if slice.is_some()
                    && (driver_flags.intersects(NodeFlags::LUPDT | NodeFlags::WATCH)
                        || (n_flags.contains(NodeFlags::LUPDT)
                            && poke_t0 != gl.signals[s].mode.is_two_value()))
                {
                    continue;
                }
            }

            *reached_fixed_point = false;

            // 1. Claim the slice of `B`.
            drive_inv_map
                .entry(s)
                .or_default()
                .push((driver_signal, drivee_slice));

            if is_variable {
                // 2. The process of the variable writes B[0] instead, outside of the drive graph.
                //
                // A net has no drive map entry: its drivers are drained into `B` below, and the
                // edge `B[0] -> A` that drives it is lowered to a drive of `A` itself.
                self.write_fuse.insert(driver_signal, (s, drivee_slice));
                self.probe_fuse
                    .insert(driver_signal, FuseTarget::Signal(s, slice));
                self.remove_edge(e);
                self.move_reads(driver, n, drivee_slice.lsb());
                if let Some(flags) = self.nodes[n].flags_mut() {
                    *flags |= NodeFlags::DRIVE_EXT;
                }
                self.inverted_variables
                    .push((driver_signal, s, drivee_slice));
                continue;
            }

            // 2. Drain driver fanin into drivee fanin.
            self.collect_in(driver, moved);
            for &ek in moved.iter() {
                self.unlink_in(ek);
                self.edges[ek].to_offset = drivee_slice.subslice(self.dst_slice(ek)).unwrap().lsb();
                self.link_in(ek, n);
            }

            // 3. A -> B     ~>      B -> A
            self.remove_edge(e);
            let edge = &mut self.edges[e];
            std::mem::swap(&mut edge.from_offset, &mut edge.to_offset);
            self.link_out(e, n);
            self.link_in(e, driver);
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Debugging
    // ---------------------------------------------------------------------------------------------

    /// Checks the invariants of the drive graph within `subgraph`.
    fn validate(&self, subgraph: &[DriveNodeKey]) {
        if !cfg!(debug_assertions) {
            return;
        }

        let mut ins = Vec::new();
        for &n in subgraph {
            let mut next = self.nodes[n].fst_out();
            while let Some(e) = next {
                let edge = &self.edges[e];
                assert_eq!(edge.from, n);
                assert!(self.src_slice(e).msb() < self.nodes[n].bit_length().get());
                assert!(self.dst_slice(e).msb() < self.nodes[edge.to].bit_length().get());
                next = edge.next_out;
            }

            // The incoming edges of an assign or gate delay tile its input: it has exactly one
            // driver, the expression or the gate. The input of a specify delay is an ordinary net,
            // which can have several drivers or be driven partially.
            if let DriveNode::Delay {
                kind: DelayKind::Assign { .. } | DelayKind::Gate { .. },
                bit_length,
                ..
            } = self.nodes[n]
            {
                self.collect_in(n, &mut ins);
                ins.sort_unstable_by_key(|&e| self.edges[e].to_offset);
                let mut offset = 0;
                for &e in &ins {
                    assert_eq!(self.edges[e].to_offset, offset);
                    offset += self.edges[e].bit_length.get();
                }
                assert!(ins.is_empty() || offset == bit_length.get());
            }

            let mut next = self.nodes[n].fst_read_out();
            while let Some(e) = next {
                let edge = &self.read_edges[e];
                assert_eq!(edge.from, n);
                assert!(
                    edge.from_offset + edge.bit_length.get() <= self.nodes[n].bit_length().get()
                );
                next = edge.next_out;
            }
        }
    }

    /// Writes the drive graph in the DOT format.
    pub(super) fn write_dot(&self, gl: &GlobalContext, w: &mut dyn Write) -> io::Result<()> {
        writeln!(w, "digraph {{")?;
        writeln!(w, "  node [shape=box];")?;
        for n in self.nodes.table_key_iter() {
            // Only render nodes that are connected, e.g. a constant that no longer drives or is
            // read by anything is left out.
            let node = &self.nodes[n];
            if node.fst_in().is_none()
                && node.fst_out().is_none()
                && node.fst_read_in().is_none()
                && node.fst_read_out().is_none()
            {
                continue;
            }
            let label = match &self.nodes[n] {
                DriveNode::Net { key, flags, .. } => format!("{} {flags}", gl.signals[*key].name),
                DriveNode::Variable { key, flags, .. } => {
                    format!("var {} {flags}", gl.signals[*key].name)
                }
                DriveNode::Constant { value, .. } => value.to_string(),
                DriveNode::Expr { source, .. } => match source {
                    ExprSource::Assign { .. } => "expr",
                    ExprSource::SignExtend { .. } => "sign-extend",
                    ExprSource::Gate { .. } => "gate",
                    ExprSource::Udp { .. } => "udp",
                    ExprSource::Pull { pull: false, .. } => "pull0",
                    ExprSource::Pull { pull: true, .. } => "pull1",
                }
                .to_string(),
                DriveNode::Delay { kind, .. } => match kind {
                    DelayKind::Assign { .. } => "assign delay".to_string(),
                    DelayKind::Gate { .. } => "gate delay".to_string(),
                    DelayKind::Specify(_) => "specify delay".to_string(),
                },
            };
            writeln!(w, "  n{} [label={label:?}];", n.get())?;
        }
        for n in self.nodes.table_key_iter() {
            let mut next = self.nodes[n].fst_out();
            while let Some(e) = next {
                let (from, to) = (self.src_slice(e), self.dst_slice(e));
                writeln!(
                    w,
                    "  n{} -> n{} [label=\"[{}:{}] -> [{}:{}]\"];",
                    n.get(),
                    self.edges[e].to.get(),
                    from.msb(),
                    from.lsb(),
                    to.msb(),
                    to.lsb(),
                )?;
                next = self.edges[e].next_out;
            }
            let mut next = self.nodes[n].fst_read_out();
            while let Some(e) = next {
                let edge = &self.read_edges[e];
                writeln!(
                    w,
                    "  n{} -> n{} [style=dashed, label=\"{:?}\"];",
                    n.get(),
                    edge.to.get(),
                    edge.kind,
                )?;
                next = edge.next_out;
            }
        }
        writeln!(w, "}}")
    }
}
