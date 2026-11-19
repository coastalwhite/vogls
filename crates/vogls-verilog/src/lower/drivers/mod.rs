//! This module handles instantiation of nets (e.g. `wire`, `wand` and module ports).
//!
//! This is done by constructing a _Directed Acyclic Graph_ (DAG) called the `DriveGraph` that
//! represents every drive of every net in a design. This drive graph can be optimized, for example
//! by aliasing different nets. At the end, the drive graph is converted into VIR processes.
//!
//! # Construction
//!
//! The drive graph is a DAG that consists of `DriveNode`s, `DriveEdge`s and `ReadEdge's`.
//!
//! `DriveNode`s can either represent a net or a value-source for a net. The kinds of nodes there
//! are:
//! - `Net` is an entire signal for a net (e.g. `wire` and module ports)
//! - `Variable` is a value-source given by an entire variable signal (e.g. `reg` and `integer`)
//! - `Constant` is a value-source given by a fixed bit-pattern
//! - `Expr` is a value-source given by an arbitrary AST expression, gate or user-defined primitive
//!   (UDP) instantiation
//! - `Delay` is a value-source that delays the input to output
//!
//! Each `DriveEdge` from `a` to `b` represents a drive relationship of a slice of bits in `a` to a
//! slice of bits in `b`, bit for bit.
//!
//! A `ReadEdge` specifies that a `DriveNode` depends on changes to a input `DriveNode` but does not
//! get driven by the value. For example, a delay might depend on the value of a wire.
//!
//! # Optimization
//!
//! After the `DriveGraph` is constructed, it is optionally transformed into `DriveGraph` that
//! requires less work or less storage. For example, if a `Net` X is fully and exclusively driven by
//! a `Net` or `Variable` Y, it `X` can be aliased as `Y`.
//!
//! # Converting into VIR
//!
//! `DriveGraph` can be converted into VIR processes, by finding a coherent subgraph and turning
//! the nodes into a VIR process.

use std::cmp;
use std::io::Write;
use std::rc::Rc;

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, LogicMode, ProcessBuilder, ProcessKind, SCALAR_VSIZE,
    Signal, SignalFlags, SignalKey, SignalSlice, VariableKey, VectorSize, WatchCondition,
    WatchEdge,
};
use vogls_utils::{Bitset, OrderedSet, SeenSet, Table, VgHashMap, VgHashSet, new_table_key};

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::module::GateInstantiation;
use crate::ast::statement::{Delay2, Delay3};
use crate::ast::udp::{UdpDeclaration, UdpInstance};
use crate::elaborate::{SignalKind, VSymbol};
use crate::lower::expression::{coerce_to, get_used_signals, lower_expr};
use crate::lower::fuse::{DestinationSite, DestinationSiteContent};
use crate::lower::module_or_generate_item::{
    AssignOutputFn, GateDelayFn, lower_assign_delay3, lower_delay_process, lower_delay2,
    lower_delay3, lower_gate_primitive,
};
use crate::lower::specify::{SpecifyDelay, lower_specify_delay};
use crate::lower::udp::lower_udp;
use crate::lower::{LowerContext, MutLowerContext, VType};

mod alias;
mod rewrite;
mod sites;
#[cfg(test)]
mod test;

use alias::NodeFlags;
pub use alias::{FuseTarget, ProbeFuseMap, WriteFuseMap};

/// Default maximum number of rounds to simplify a subgraph of the drive graph.
pub const DEFAULT_MAX_ROUNDS: usize = 1024;

new_table_key! { pub struct DriveNodeKey; }
new_table_key! { pub struct DriveEdgeKey; }
new_table_key! { pub struct ReadEdgeKey; }
new_table_key! { pub struct BidirNodeKey; }
new_table_key! { pub struct BidirEdgeKey; }
new_table_key! { pub struct BidirGroupKey; }

#[derive(Clone)]
pub struct DriveEdge {
    from: DriveNodeKey,
    from_offset: u32,
    to: DriveNodeKey,
    to_offset: u32,
    bit_length: VectorSize,

    prev_out: Option<DriveEdgeKey>,
    next_out: Option<DriveEdgeKey>,
    prev_in: Option<DriveEdgeKey>,
    next_in: Option<DriveEdgeKey>,

    /// The net assignment with expressions that this edge is part of.
    assignment: Option<AssignmentKey>,
}

/// What the reader of a value depends on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReadKind {
    /// Only the value at the time the reader is evaluated.
    Value,
    /// Every change of the value.
    Event,
}

/// A read of `from[from_offset +: bit_length]` by the process of `to`.
///
/// Read edges only ever move all together, when the node they read from is aliased.
#[derive(Clone)]
pub struct ReadEdge {
    kind: ReadKind,

    from: DriveNodeKey,
    from_offset: u32,
    bit_length: VectorSize,
    to: DriveNodeKey,

    next_out: Option<ReadEdgeKey>,
    next_in: Option<ReadEdgeKey>,
}

#[derive(Clone)]
pub struct BidirEdge {
    left: BidirNodeKey,
    left_offset: u32,
    right: BidirNodeKey,
    right_offset: u32,
    bit_length: VectorSize,

    prev_left: Option<BidirEdgeKey>,
    next_left: Option<BidirEdgeKey>,
    prev_right: Option<BidirEdgeKey>,
    next_right: Option<BidirEdgeKey>,
}

#[derive(Clone)]
pub struct BidirNode {
    key: SignalKey,
    fst_bidir: Option<BidirEdgeKey>,
}

#[derive(Clone)]
pub enum DelayKind<'a> {
    /// The delay of a net assignment.
    Assign {
        delay3: AstId<'a, Delay3<'a>>,
        scope: SymbolId,
    },
    /// The delay of an output of a gate.
    Gate {
        delay: GateDelay<'a>,
        scope: SymbolId,
    },
    /// The module path delays of an output with a specify block.
    Specify(Rc<SpecifyDelay<'a>>),
}

#[derive(Clone, Copy)]
pub enum GateDelay<'a> {
    Delay2(AstId<'a, Delay2<'a>>),
    Delay3(AstId<'a, Delay3<'a>>),
}

/// What the process of a [`DriveNode::Expr`] evaluates.
#[derive(Clone, Copy)]
pub enum ExprSource<'a> {
    /// An expression of a net assignment, converted to `ty`. All expressions of one assignment
    /// share one process.
    Assign {
        expr: AstId<'a, Expr<'a>>,
        scope: SymbolId,
        assignment: AssignmentKey,
        ty: VType,
    },
    /// The bit `signal[bit]`, sign-extended to the width of the node. Part of an assignment that
    /// connects a signed signal to wider destinations.
    SignExtend {
        signal: SignalKey,
        bit: u32,
        assignment: AssignmentKey,
    },
    /// An output of a gate instance.
    Gate {
        gate: AstId<'a, GateInstantiation<'a>>,
        scope: SymbolId,
        instance: u32,
        output: u32,
    },
    /// The output of a user-defined primitive instance.
    Udp {
        udp: AstId<'a, UdpDeclaration<'a>>,
        scope: SymbolId,
        instance: AstId<'a, UdpInstance<'a>>,
        /// The state of a sequential UDP, see [`crate::lower::udp::sequential_udp_state`].
        state: Option<SignalKey>,
    },
}

impl ExprSource<'_> {
    /// The net assignment that the process of the node is part of, if any. All nodes of one
    /// assignment share one process.
    pub fn assignment(&self) -> Option<AssignmentKey> {
        match self {
            Self::Assign { assignment, .. } | Self::SignExtend { assignment, .. } => {
                Some(*assignment)
            }
            Self::Gate { .. } | Self::Udp { .. } => None,
        }
    }
}

/// Drives `value` into the bits of the output of a process at `offset`.
pub type DriveOutputFn<'a, 'f> =
    dyn Fn(&mut MutLowerContext<'a>, &mut BasicBlockBuilder, u32, VariableKey) + 'f;

/// Where a process drives the bits of one of its outgoing edges.
struct OutputTarget {
    /// The bits of the output of the process.
    src: SignalSlice,
    signal: SignalKey,
    offset: u32,
}

#[derive(Clone)]
pub enum DriveNode<'a> {
    Net {
        key: SignalKey,
        bit_length: VectorSize,
        flags: NodeFlags,

        fst_in: Option<DriveEdgeKey>,
        fst_out: Option<DriveEdgeKey>,
        fst_read_out: Option<ReadEdgeKey>,
    },
    /// A signal that is only written by processes.
    Variable {
        key: SignalKey,
        bit_length: VectorSize,
        flags: NodeFlags,

        fst_out: Option<DriveEdgeKey>,
        fst_read_out: Option<ReadEdgeKey>,
    },
    Constant {
        value: Bits,

        fst_out: Option<DriveEdgeKey>,
        fst_read_out: Option<ReadEdgeKey>,
    },
    /// An output of a process that evaluates `source`, e.g. an expression or a gate.
    Expr {
        source: ExprSource<'a>,
        bit_length: VectorSize,

        fst_out: Option<DriveEdgeKey>,
        fst_read_in: Option<ReadEdgeKey>,
    },
    /// The value driven onto `input` is driven onto the outgoing edges after a delay.
    Delay {
        kind: DelayKind<'a>,
        input: SignalKey,
        bit_length: VectorSize,

        fst_in: Option<DriveEdgeKey>,
        fst_out: Option<DriveEdgeKey>,
        fst_read_in: Option<ReadEdgeKey>,
    },
}

impl<'a> DriveNode<'a> {
    fn fst_in(&self) -> Option<DriveEdgeKey> {
        match self {
            Self::Net { fst_in, .. } | Self::Delay { fst_in, .. } => *fst_in,
            Self::Variable { .. } | Self::Constant { .. } | Self::Expr { .. } => None,
        }
    }

    fn set_fst_in(&mut self, edge: Option<DriveEdgeKey>) {
        match self {
            Self::Net { fst_in, .. } | Self::Delay { fst_in, .. } => *fst_in = edge,
            Self::Variable { .. } | Self::Constant { .. } | Self::Expr { .. } => {
                assert!(edge.is_none(), "only nets and delays can be driven")
            }
        }
    }

    fn fst_out(&self) -> Option<DriveEdgeKey> {
        match self {
            Self::Net { fst_out, .. }
            | Self::Variable { fst_out, .. }
            | Self::Constant { fst_out, .. }
            | Self::Expr { fst_out, .. }
            | Self::Delay { fst_out, .. } => *fst_out,
        }
    }

    fn set_fst_out(&mut self, edge: Option<DriveEdgeKey>) {
        match self {
            Self::Net { fst_out, .. }
            | Self::Variable { fst_out, .. }
            | Self::Constant { fst_out, .. }
            | Self::Expr { fst_out, .. }
            | Self::Delay { fst_out, .. } => *fst_out = edge,
        }
    }

    fn fst_read_out(&self) -> Option<ReadEdgeKey> {
        match self {
            Self::Net { fst_read_out, .. }
            | Self::Variable { fst_read_out, .. }
            | Self::Constant { fst_read_out, .. } => *fst_read_out,
            Self::Expr { .. } | Self::Delay { .. } => None,
        }
    }

    fn set_fst_read_out(&mut self, edge: Option<ReadEdgeKey>) {
        match self {
            Self::Net { fst_read_out, .. }
            | Self::Variable { fst_read_out, .. }
            | Self::Constant { fst_read_out, .. } => *fst_read_out = edge,
            Self::Expr { .. } | Self::Delay { .. } => {
                unreachable!("only nets, variables and constants can be read")
            }
        }
    }

    fn fst_read_in(&self) -> Option<ReadEdgeKey> {
        match self {
            Self::Expr { fst_read_in, .. } | Self::Delay { fst_read_in, .. } => *fst_read_in,
            Self::Net { .. } | Self::Variable { .. } | Self::Constant { .. } => None,
        }
    }

    fn set_fst_read_in(&mut self, edge: Option<ReadEdgeKey>) {
        match self {
            Self::Expr { fst_read_in, .. } | Self::Delay { fst_read_in, .. } => *fst_read_in = edge,
            Self::Net { .. } | Self::Variable { .. } | Self::Constant { .. } => {
                unreachable!("only expressions, primitives and delays read")
            }
        }
    }

    /// How the node is observed and written outside of the drive graph.
    fn flags(&self) -> NodeFlags {
        match self {
            Self::Net { flags, .. } | Self::Variable { flags, .. } => *flags,
            Self::Constant { .. } | Self::Expr { .. } | Self::Delay { .. } => NodeFlags::EMPTY,
        }
    }

    fn flags_mut(&mut self) -> Option<&mut NodeFlags> {
        match self {
            Self::Net { flags, .. } | Self::Variable { flags, .. } => Some(flags),
            Self::Constant { .. } | Self::Expr { .. } | Self::Delay { .. } => None,
        }
    }

    pub fn bit_length(&self) -> VectorSize {
        match self {
            Self::Net { bit_length, .. }
            | Self::Variable { bit_length, .. }
            | Self::Expr { bit_length, .. }
            | Self::Delay { bit_length, .. } => *bit_length,
            Self::Constant { value, .. } => value.size(),
        }
    }

    /// The signal that holds the value driven into the node.
    fn drivee_signal(&self) -> Option<SignalKey> {
        match self {
            Self::Net { key, .. } => Some(*key),
            Self::Delay { input, .. } => Some(*input),
            Self::Variable { .. } | Self::Constant { .. } | Self::Expr { .. } => None,
        }
    }

    /// The signal that holds the value the node drives, unless it is a constant or a process.
    fn driver_signal(&self) -> Option<SignalKey> {
        match self {
            Self::Net { key, .. } | Self::Variable { key, .. } => Some(*key),
            Self::Constant { .. } | Self::Expr { .. } | Self::Delay { .. } => None,
        }
    }

    /// Whether the node is a process that is part of the drive graph.
    fn is_process(&self) -> bool {
        match self {
            Self::Expr { .. } | Self::Delay { .. } => true,
            Self::Net { .. } | Self::Variable { .. } | Self::Constant { .. } => false,
        }
    }
}

#[derive(Default, Clone)]
pub struct DriverResolution<'a> {
    nodes: Table<DriveNodeKey, DriveNode<'a>>,
    edges: Table<DriveEdgeKey, DriveEdge>,
    read_edges: Table<ReadEdgeKey, ReadEdge>,
    bidir_edges: Table<BidirEdgeKey, BidirEdge>,
    bidir_nodes: Table<BidirNodeKey, BidirNode>,

    signal_lut: VgHashMap<SignalKey, DriveNodeKey>,
    bidir_lut: VgHashMap<SignalKey, BidirNodeKey>,
    /// Expression nodes by their assignment, the address of their expression and their type.
    expr_lut: VgHashMap<(AssignmentKey, usize, VType), DriveNodeKey>,

    num_assignments: u32,
    /// The signals that the processes of primitives and delays read, until the read edges are
    /// built.
    pending_reads: Vec<(DriveNodeKey, Vec<SignalKey>, ReadKind)>,

    /// Signals that are replaced by another signal (slice) or constant when probed.
    probe_fuse: ProbeFuseMap,
    /// Signals that are replaced by another signal (slice) when driven.
    write_fuse: WriteFuseMap,
    /// The drive proxies of the members of bidirectional closures.
    drive_proxies: Vec<SignalKey>,
    /// Nodes whose remaining incoming edges are resolved by a process.
    lower: Vec<DriveNodeKey>,
    /// Edges whose source process drives the drivee directly.
    direct: VgHashSet<DriveEdgeKey>,
    /// Wiring of an assignment that the process of its expressions drives.
    expr_wires: VgHashMap<AssignmentKey, Vec<DriveEdgeKey>>,
    /// The signal that a process drives for an outgoing edge that is not direct.
    edge_proxies: VgHashMap<DriveEdgeKey, SignalKey>,
    /// `(variable, net, slice)` of every variable that drives `net[slice]` directly.
    inverted_variables: Vec<(SignalKey, SignalKey, SignalSlice)>,
}

/// Identifies a net assignment. All expressions of an assignment are evaluated by one process.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct AssignmentKey(u32);

fn signal_proxy(
    gl: &mut GlobalContext,
    name: String,
    bit_length: VectorSize,
    mode: LogicMode,
) -> SignalKey {
    gl.signals.insert(Signal {
        name,
        size: bit_length,
        initialize: match mode {
            LogicMode::TwoValue => None,
            LogicMode::FourValue => Some(Bits::new_high_impedance(bit_length)),
        },
        mode,
        flags: SignalFlags::EMPTY,
        origin: TokenRange::default(),
    })
}

impl<'a> DriverResolution<'a> {
    fn collect_bidir_fast_path(
        &self,
        bidir_key: BidirEdgeKey,
        seen: &mut SeenSet<(BidirNodeKey, u32)>,
        active: &mut Vec<(BidirNodeKey, u32)>,
    ) -> bool {
        seen.clear();
        active.clear();

        let bit_length = self.bidir_edges[bidir_key].bit_length;

        let bidir = &self.bidir_edges[bidir_key];
        if bidir.left == bidir.right {
            return false;
        }

        seen.insert((bidir.left, bidir.left_offset));
        seen.insert((bidir.right, bidir.right_offset));

        active.push((bidir.left, bidir.left_offset));
        active.push((bidir.right, bidir.right_offset));

        while let Some((bidir_key, offset)) = seen.pop() {
            let e1_lo = offset;
            let e1_hi = offset + (bit_length.get() - 1);

            let mut next_bidir_edge_key = self.bidir_nodes[bidir_key].fst_bidir;
            while let Some(bidir_edge_key) = next_bidir_edge_key {
                let bidir_edge = &self.bidir_edges[bidir_edge_key];

                if bidir_edge.left == bidir_edge.right {
                    return false;
                }

                let (slf_offset, other_bidir_key, other_offset, next) =
                    if bidir_edge.left == bidir_key {
                        (
                            bidir_edge.left_offset,
                            bidir_edge.right,
                            bidir_edge.right_offset,
                            bidir_edge.next_left,
                        )
                    } else {
                        (
                            bidir_edge.right_offset,
                            bidir_edge.left,
                            bidir_edge.left_offset,
                            bidir_edge.next_right,
                        )
                    };

                next_bidir_edge_key = next;

                let e2_lo = slf_offset;
                let e2_hi = slf_offset + (bidir_edge.bit_length.get() - 1);
                let overlaps = cmp::max(e1_lo, e2_lo) <= cmp::min(e1_hi, e2_hi);

                if !overlaps {
                    continue;
                }

                if e1_lo != e2_lo || e1_hi != e2_hi {
                    return false;
                }

                if seen.insert((other_bidir_key, other_offset)) {
                    active.push((other_bidir_key, other_offset));
                }
            }
        }

        true
    }

    pub fn push_bidirectional(
        &mut self,
        left: (SignalKey, SignalSlice),
        right: (SignalKey, SignalSlice),
    ) {
        assert_eq!(left.1.width(), right.1.width());
        let lkey = *self.bidir_lut.entry(left.0).or_insert_with(|| {
            self.bidir_nodes.insert(BidirNode {
                key: left.0,
                fst_bidir: None,
            })
        });
        let rkey = *self.bidir_lut.entry(right.0).or_insert_with(|| {
            self.bidir_nodes.insert(BidirNode {
                key: right.0,
                fst_bidir: None,
            })
        });

        // Prepend the new edge to the edge lists of both nodes. A self-loop is only linked
        // through its `left` fields as traversal always follows `next_left` when `left` matches.
        let next_left = self.bidir_nodes[lkey].fst_bidir;
        let next_right = (lkey != rkey)
            .then(|| self.bidir_nodes[rkey].fst_bidir)
            .flatten();
        let edge = self.bidir_edges.insert(BidirEdge {
            left: lkey,
            left_offset: left.1.lsb(),
            right: rkey,
            right_offset: right.1.lsb(),
            bit_length: left.1.width(),
            prev_left: None,
            next_left,
            prev_right: None,
            next_right,
        });

        if let Some(next) = next_left {
            self.set_prev(next, lkey, edge);
        }
        if let Some(next) = next_right {
            self.set_prev(next, rkey, edge);
        }
        self.bidir_nodes[lkey].fst_bidir = Some(edge);
        self.bidir_nodes[rkey].fst_bidir = Some(edge);
    }

    /// Bidirectionally connects two concatenations of signal slices bit for bit.
    ///
    /// Both concatenations are given from least to most significant and must have the same width.
    pub fn push_bidirectional_sites(
        &mut self,
        left: &[(SignalKey, SignalSlice)],
        right: &[(SignalKey, SignalSlice)],
    ) {
        debug_assert_eq!(
            left.iter()
                .map(|(_, s)| u64::from(s.width().get()))
                .sum::<u64>(),
            right
                .iter()
                .map(|(_, s)| u64::from(s.width().get()))
                .sum::<u64>(),
        );

        let mut left_iter = left.iter().copied();
        let mut right_iter = right.iter().copied();
        let mut left_cur = left_iter.next();
        let mut right_cur = right_iter.next();
        while let (Some((l_signal, l_slice)), Some((r_signal, r_slice))) = (left_cur, right_cur) {
            let width = cmp::min(l_slice.width(), r_slice.width());

            let l_piece = SignalSlice::from_width(l_slice.lsb(), width).unwrap();
            let r_piece = SignalSlice::from_width(r_slice.lsb(), width).unwrap();
            self.push_bidirectional((l_signal, l_piece), (r_signal, r_piece));

            let l_next = SignalSlice::from_range(l_slice.lsb() + width.get()..l_slice.msb() + 1);
            let r_next = SignalSlice::from_range(r_slice.lsb() + width.get()..r_slice.msb() + 1);

            left_cur = match l_next {
                Some(rest) => Some((l_signal, rest)),
                None => left_iter.next(),
            };
            right_cur = match r_next {
                Some(rest) => Some((r_signal, rest)),
                None => right_iter.next(),
            };
        }
    }

    fn set_prev(&mut self, edge: BidirEdgeKey, node: BidirNodeKey, prev: BidirEdgeKey) {
        let edge = &mut self.bidir_edges[edge];
        if edge.left == node {
            edge.prev_left = Some(prev);
        } else {
            debug_assert!(edge.right == node);
            edge.prev_right = Some(prev);
        }
    }

    /// Returns the node of `key`, creating it as `kind` if it does not exist yet.
    fn signal_node(
        &mut self,
        gl: &GlobalContext,
        key: SignalKey,
        kind: SignalKind,
    ) -> DriveNodeKey {
        *self.signal_lut.entry(key).or_insert_with(|| {
            let bit_length = gl.signals[key].size;
            self.nodes.insert(match kind {
                SignalKind::Net => DriveNode::Net {
                    key,
                    bit_length,
                    flags: NodeFlags::EMPTY,
                    fst_in: None,
                    fst_out: None,
                    fst_read_out: None,
                },
                SignalKind::Variable => DriveNode::Variable {
                    key,
                    bit_length,
                    flags: NodeFlags::EMPTY,
                    fst_out: None,
                    fst_read_out: None,
                },
            })
        })
    }

    fn push_edge(
        &mut self,
        (from, from_offset): (DriveNodeKey, u32),
        (to, to_offset): (DriveNodeKey, u32),
        bit_length: VectorSize,
    ) -> DriveEdgeKey {
        let next_out = self.nodes[from].fst_out();
        let next_in = self.nodes[to].fst_in();
        let edge = self.edges.insert(DriveEdge {
            from,
            from_offset,
            to,
            to_offset,
            bit_length,
            prev_out: None,
            next_out,
            prev_in: None,
            next_in,
            assignment: None,
        });

        if let Some(next) = next_out {
            self.edges[next].prev_out = Some(edge);
        }
        if let Some(next) = next_in {
            self.edges[next].prev_in = Some(edge);
        }
        self.nodes[from].set_fst_out(Some(edge));
        self.nodes[to].set_fst_in(Some(edge));
        edge
    }

    fn push_read_edge(
        &mut self,
        kind: ReadKind,
        (from, from_offset): (DriveNodeKey, u32),
        to: DriveNodeKey,
        bit_length: VectorSize,
    ) {
        let next_out = self.nodes[from].fst_read_out();
        let next_in = self.nodes[to].fst_read_in();
        let edge = self.read_edges.insert(ReadEdge {
            kind,
            from,
            from_offset,
            bit_length,
            to,
            next_out,
            next_in,
        });
        self.nodes[from].set_fst_read_out(Some(edge));
        self.nodes[to].set_fst_read_in(Some(edge));
    }

    /// Returns a new key for a net assignment.
    fn new_assignment(&mut self) -> AssignmentKey {
        let key = AssignmentKey(self.num_assignments);
        self.num_assignments += 1;
        key
    }

    /// Adds a delay of `delay3` in front of `dsts`, given from least to most significant bit.
    ///
    /// Returns the input signal of the delay, which spans all destinations.
    pub fn push_delay(
        &mut self,
        gl: &mut GlobalContext,
        mode: LogicMode,
        delay3: AstId<'a, Delay3<'a>>,
        scope: SymbolId,
        dsts: &[DestinationSite],
        bit_length: VectorSize,
    ) -> SignalKey {
        let input = signal_proxy(gl, "::DELAY_IN".to_string(), bit_length, mode);
        let delay = self.nodes.insert(DriveNode::Delay {
            kind: DelayKind::Assign { delay3, scope },
            input,
            bit_length,
            fst_in: None,
            fst_out: None,
            fst_read_in: None,
        });
        self.signal_lut.insert(input, delay);
        self.push_outputs(gl, delay, dsts);

        input
    }

    /// Adds an output of a primitive of `bit_length` bits that drives `dsts`, given from least to
    /// most significant bit, after `delay`. The process of the primitive is sensitive to `reads`.
    #[allow(clippy::too_many_arguments)]
    pub fn push_primitive(
        &mut self,
        gl: &mut GlobalContext,
        mode: LogicMode,
        source: ExprSource<'a>,
        dsts: &[DestinationSite],
        bit_length: VectorSize,
        delay: Option<GateDelay<'a>>,
        reads: Vec<SignalKey>,
    ) {
        let (ExprSource::Gate { scope, .. } | ExprSource::Udp { scope, .. }) = source else {
            unreachable!("only gates and user-defined primitives are primitives");
        };
        let node = self.nodes.insert(DriveNode::Expr {
            source,
            bit_length,
            fst_out: None,
            fst_read_in: None,
        });
        self.pending_reads.push((node, reads, ReadKind::Value));

        // The process of a sequential UDP writes its state, a variable that drives the outputs.
        if let ExprSource::Udp {
            state: Some(state), ..
        } = source
        {
            assert!(delay.is_none(), "UDP delays are not supported");
            let state = self.signal_node(gl, state, SignalKind::Variable);
            self.push_outputs(gl, state, dsts);
            return;
        }

        let Some(delay) = delay else {
            self.push_outputs(gl, node, dsts);
            return;
        };

        // The output of a gate is `x` until the gate is first evaluated.
        let mut flags = SignalFlags::EMPTY;
        flags.set(SignalFlags::POKE_T0, mode.is_two_value());
        let input = gl.signals.insert(Signal {
            name: "::GATE_DELAY_IN".to_string(),
            size: bit_length,
            initialize: None,
            mode,
            flags,
            origin: TokenRange::default(),
        });
        let delay = self.nodes.insert(DriveNode::Delay {
            kind: DelayKind::Gate { delay, scope },
            input,
            bit_length,
            fst_in: None,
            fst_out: None,
            fst_read_in: None,
        });
        self.signal_lut.insert(input, delay);
        self.push_edge((node, 0), (delay, 0), bit_length);
        self.push_outputs(gl, delay, dsts);
    }

    /// Adds the module path delays of `specify`, where `input` is the signal that the drivers of
    /// its output drive. The processes of the module paths are sensitive to `reads`.
    pub fn push_specify_delay(
        &mut self,
        gl: &mut GlobalContext,
        input: SignalKey,
        specify: SpecifyDelay<'a>,
        reads: Vec<SignalKey>,
    ) {
        let output = specify.output;
        let bit_length = gl.signals[output].size;
        let delay = self.nodes.insert(DriveNode::Delay {
            kind: DelayKind::Specify(Rc::new(specify)),
            input,
            bit_length,
            fst_in: None,
            fst_out: None,
            fst_read_in: None,
        });
        let prev = self.signal_lut.insert(input, delay);
        assert!(
            prev.is_none(),
            "the input of a specify delay is not driven yet"
        );
        self.pending_reads.push((delay, reads, ReadKind::Event));

        // The delay drives the output, even if the output is declared as a variable.
        let to = self.signal_node(gl, output, SignalKind::Net);
        if let DriveNode::Variable {
            key,
            bit_length,
            flags,
            fst_out,
            fst_read_out,
        } = self.nodes[to]
        {
            self.nodes[to] = DriveNode::Net {
                key,
                bit_length,
                flags,
                fst_in: None,
                fst_out,
                fst_read_out,
            };
        }
        self.push_edge((delay, 0), (to, 0), bit_length);
    }

    /// Connects the bits of the output of `from` to `dsts`, given from least to most significant.
    fn push_outputs(&mut self, gl: &GlobalContext, from: DriveNodeKey, dsts: &[DestinationSite]) {
        let mut offset = 0;
        for dst in dsts {
            if let DestinationSiteContent::Signal {
                signal,
                slice,
                kind,
            } = dst.content
            {
                let to = self.signal_node(gl, signal, kind);
                self.push_edge((from, offset), (to, slice.lsb()), slice.width());
            }
            offset += dst.bit_length().get();
        }
    }

    /// Builds the read edges of all expressions, primitives and delays.
    fn push_reads(
        &mut self,
        ctx: &LowerContext<'a, '_>,
        mctx: &mut MutLowerContext<'a>,
    ) -> Result<(), ()> {
        let mut error = false;
        let mut used_signals = OrderedSet::new();
        for node in self.nodes.table_key_iter() {
            let (expr, scope) = match self.nodes[node] {
                DriveNode::Expr {
                    source: ExprSource::Assign { expr, scope, .. },
                    ..
                } => (expr, scope),
                DriveNode::Expr {
                    source: ExprSource::SignExtend { signal, .. },
                    ..
                } => {
                    self.pending_reads
                        .push((node, vec![signal], ReadKind::Value));
                    continue;
                }
                _ => continue,
            };
            used_signals.clear();
            if get_used_signals(ctx, mctx, scope, &mut used_signals, expr).is_err() {
                error = true;
                continue;
            }
            self.pending_reads
                .push((node, used_signals.items.clone(), ReadKind::Value));
        }

        for (to, reads, kind) in std::mem::take(&mut self.pending_reads) {
            for signal in reads {
                let Some(&from) = self.signal_lut.get(&signal) else {
                    continue;
                };
                if self.nodes[from].is_process() {
                    continue;
                }
                let bit_length = self.nodes[from].bit_length();
                self.push_read_edge(kind, (from, 0), to, bit_length);
            }
        }

        if error {
            return Err(());
        }
        Ok(())
    }

    /// Initializes the signals of every net symbol, including its specify terminal: undriven bits
    /// are `z` and driven bits are `x` in four-value mode, and all bits are poked at `t=0` in
    /// two-value mode. A four-value signal without an initial value is `x`. Variables that drive a
    /// net directly carry their initial value over into the net.
    fn write_initial_values(&self, ctx: &LowerContext<'a, '_>, gl: &mut GlobalContext) {
        let nets = ctx
            .table
            .symbol_id_iter()
            .filter_map(|sid| match &ctx.table[sid].content {
                VSymbol::Signal(symbol) if symbol.kind == SignalKind::Net => {
                    Some(&symbol.terminals)
                }
                _ => None,
            })
            .flat_map(|terminals| {
                std::iter::once(terminals.probe_signal()).chain(terminals.specify)
            });
        for net in nets {
            debug_assert!(
                gl.signals[net].initialize.is_none()
                    && !gl.signals[net].flags.contains(SignalFlags::POKE_T0),
                "nets are initialized by the driver resolution only"
            );
            if self.probe_fuse.contains_key(&net) {
                continue;
            }

            let mut initialize = None;
            if gl.signals[net].mode.is_four_value() {
                let mut value = Bits::new_high_impedance(gl.signals[net].size);
                if let Some(&node) = self.signal_lut.get(&net) {
                    let mut next = self.nodes[node].fst_in();
                    while let Some(edge) = next {
                        let e = &self.edges[edge];
                        value = value.set_slice(e.to_offset, &Bits::new_unknown(e.bit_length));
                        next = e.next_in;
                    }
                }
                initialize = (value.count_unknown() != value.size().get()).then_some(value);
            }

            let signal = &mut gl.signals[net];
            signal.initialize = initialize;
            signal
                .flags
                .set(SignalFlags::POKE_T0, signal.mode.is_two_value());
        }

        for &(variable, net, slice) in &self.inverted_variables {
            let v = &gl.signals[variable];
            let poke_t0 = v.flags.contains(SignalFlags::POKE_T0);
            if v.initialize.is_none() && gl.signals[net].initialize.is_none() {
                if poke_t0 {
                    gl.signals[net].flags.set(SignalFlags::POKE_T0, true);
                }
                continue;
            }
            let value = v.initialize.clone().unwrap_or_else(|| match v.mode {
                LogicMode::TwoValue => Bits::new_zeroed(v.size),
                LogicMode::FourValue => Bits::new_unknown(v.size),
            });

            let n = &mut gl.signals[net];
            let base = n.initialize.take().unwrap_or_else(|| match n.mode {
                LogicMode::TwoValue => Bits::new_zeroed(n.size),
                LogicMode::FourValue => Bits::new_unknown(n.size),
            });
            let value = base.set_slice(slice.lsb(), &value);
            n.initialize = (n.mode.is_two_value() || value.count_unknown() != value.size().get())
                .then_some(value);
            if poke_t0 {
                n.flags.set(SignalFlags::POKE_T0, true);
            }
        }
    }

    /// Computes the bidirectional closures of `edges` bit by bit with a union-find.
    ///
    /// Returns every closure as its members `(node, offset)` and its bit length. Closures of
    /// neighbouring bits that connect the same signals at neighbouring offsets are coalesced into
    /// a single wider closure. Closures with a single member connect nothing and are dropped.
    fn collect_bidir_per_bit(
        &self,
        edges: &[BidirEdgeKey],
    ) -> Vec<(Vec<(BidirNodeKey, u32)>, VectorSize)> {
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }

        let mut bits = VgHashMap::<(BidirNodeKey, u32), usize>::default();
        let mut members = Vec::<(BidirNodeKey, u32)>::new();
        let mut parent = Vec::<usize>::new();
        let mut index = |bit: (BidirNodeKey, u32)| {
            *bits.entry(bit).or_insert_with(|| {
                members.push(bit);
                parent.push(parent.len());
                parent.len() - 1
            })
        };

        let mut unions = Vec::new();
        for &edge in edges {
            let e = &self.bidir_edges[edge];
            for i in 0..e.bit_length.get() {
                let l = index((e.left, e.left_offset + i));
                let r = index((e.right, e.right_offset + i));
                unions.push((l, r));
            }
        }
        for (l, r) in unions {
            let (l, r) = (find(&mut parent, l), find(&mut parent, r));
            if l != r {
                parent[l.max(r)] = l.min(r);
            }
        }

        // Group the bits by their closure.
        let mut closures = VgHashMap::<usize, Vec<(BidirNodeKey, u32)>>::default();
        for i in 0..members.len() {
            let root = find(&mut parent, i);
            closures.entry(root).or_default().push(members[i]);
        }
        let mut closures = closures
            .into_values()
            .filter(|c| c.len() > 1)
            .map(|mut c| {
                c.sort_unstable();
                c
            })
            .collect::<Vec<_>>();
        // A closure sorts before the closure of the next bits, so it is visited first below.
        closures.sort_unstable();

        // Coalesce closures that are each other shifted by one bit.
        let lut = closures
            .iter()
            .enumerate()
            .map(|(i, c)| (c.clone(), i))
            .collect::<VgHashMap<_, _>>();
        let mut consumed = vec![false; closures.len()];
        let mut result = Vec::new();
        for i in 0..closures.len() {
            if consumed[i] {
                continue;
            }
            consumed[i] = true;

            let mut width = 1;
            loop {
                let shifted = closures[i]
                    .iter()
                    .map(|&(node, bit)| (node, bit + width))
                    .collect::<Vec<_>>();
                match lut.get(&shifted) {
                    Some(&j) if !consumed[j] => {
                        consumed[j] = true;
                        width += 1;
                    }
                    _ => break,
                }
            }

            result.push((
                std::mem::take(&mut closures[i]),
                VectorSize::new(width).unwrap(),
            ));
        }
        result
    }

    /// Moves all incoming edges of `from` to `to`, keeping their offsets.
    fn splice_in_edges(&mut self, from: DriveNodeKey, to: DriveNodeKey) {
        let Some(head) = self.nodes[from].fst_in() else {
            return;
        };
        self.nodes[from].set_fst_in(None);

        let mut tail = head;
        loop {
            self.edges[tail].to = to;
            match self.edges[tail].next_in {
                Some(next) => tail = next,
                None => break,
            }
        }

        let old_head = self.nodes[to].fst_in();
        self.edges[tail].next_in = old_head;
        if let Some(old_head) = old_head {
            self.edges[old_head].prev_in = Some(tail);
        }
        self.nodes[to].set_fst_in(Some(head));
    }

    /// Lowers the bidirectional transitive closures to proxy signals.
    ///
    /// ```text
    /// A <-> B --> E                      Ad --+--> P --+--> A
    ///       ^                                 |        |
    ///       |            ===>            Bd --+        +--> B --> E
    ///       v                                 |        |
    /// D --> C                      D --> Cd --+        +--> C
    /// ```
    ///
    /// where `P` is the proxy of a closure and `Ad`, `Bd` and `Cd` are the drive proxies of `A`,
    /// `B` and `C`. All incoming edges of a member move to its drive proxy. The drive proxy drives
    /// the bits of the member that are in a closure through the closure's proxy and the other bits
    /// directly (`Ad --> A`). Only bits that the drive proxy has drivers for are forwarded, so an
    /// undriven bit is never resolved. This matters in two-valued logic, where an undriven bit
    /// reads `0` instead of `z`.
    ///
    /// External writes of a member are redirected to its drive proxy (see [`WriteFuseMap`]): they
    /// write the value that the member drives into its closures, as in Icarus Verilog.
    ///
    /// Lowering happens in three stages:
    /// 1. All closures are collected as groups of `(signal, offset)` entries.
    /// 2. The entries are ordered per signal to find the bits of every member that are in a closure.
    /// 3. The drive proxies of the members and the proxies of the closures are created.
    fn lower_bidirectional(&mut self, gl: &mut GlobalContext) {
        if self.bidir_edges.is_empty() {
            return;
        }

        // 1. Collect the closures.
        let mut groups = Table::<BidirGroupKey, VectorSize>::default();
        let mut entries = Vec::<(BidirGroupKey, SignalKey, u32)>::new();
        {
            let mut seen = SeenSet::new();
            let mut active = Vec::new();
            let mut handled = VgHashSet::default();
            let mut slow = Vec::new();
            for bidir_key in self.bidir_edges.table_key_iter() {
                let bidir = &self.bidir_edges[bidir_key];
                if handled.contains(&(bidir.left, bidir.left_offset)) {
                    // Already part of another closure.
                    continue;
                }

                // Fast path: handle the entire edge in one go.
                if !self.collect_bidir_fast_path(bidir_key, &mut seen, &mut active) {
                    slow.push(bidir_key);
                    continue;
                }
                handled.extend(active.iter().copied());

                let group = groups.insert(self.bidir_edges[bidir_key].bit_length);
                entries.extend(
                    active
                        .iter()
                        .map(|&(node, offset)| (group, self.bidir_nodes[node].key, offset)),
                );
            }

            // Slow path: partially overlapping or self-connected edges are resolved per bit.
            if !slow.is_empty() {
                for (members, bit_length) in self.collect_bidir_per_bit(&slow) {
                    let group = groups.insert(bit_length);
                    entries.extend(
                        members
                            .iter()
                            .map(|&(node, offset)| (group, self.bidir_nodes[node].key, offset)),
                    );
                }
            }
        }

        // 2. The bits of every member that are in a closure. Closures are disjoint per bit, so the
        //    entries of one signal do not overlap.
        let mut by_signal = entries.clone();
        by_signal.sort_unstable_by_key(|&(_, signal, offset)| (signal, offset));
        let mut covered_bits = Bitset::new();
        let mut members = Vec::<(SignalKey, CoveredMask)>::new();
        for chunk in by_signal.chunk_by(|a, b| a.1 == b.1) {
            let signal = chunk[0].1;
            let size = gl.signals[signal].size.get();
            let mask = match *chunk {
                [(group, _, 0)] if groups[group].get() == size => CoveredMask::Entire,
                _ => {
                    let start = covered_bits.len();
                    covered_bits.extend_zeroed(size as usize);
                    for &(group, _, offset) in chunk {
                        for bit in offset..offset + groups[group].get() {
                            covered_bits.set(start + bit as usize, true);
                        }
                    }
                    CoveredMask::Selection(start)
                }
            };
            members.push((signal, mask));
        }

        // 3a. The drive proxies. Each takes over all incoming edges of its member and drives the
        //     driven bits that are in no closure directly. A member that is neither driven nor
        //     written externally needs none.
        //
        //     An external write of a member goes to its drive proxy: it writes the value that the
        //     member itself drives, as in Icarus Verilog. The drivers of the member overwrite a bit
        //     they drive when they update, while a bit that the member does not drive keeps it, as
        //     one more driver of its closures.
        //
        //     TODO: In two-valued logic, a bit of the drive proxy that is neither driven nor written
        //     is `0` instead of `z`, so it drives the closures. The members of a closure and the
        //     nets that drive them should be four-valued, which requires deciding the modes of nets
        //     before lowering.
        let mut drive_proxies = VgHashMap::<SignalKey, (DriveNodeKey, Vec<(u32, u32)>)>::default();
        let mut ins = Vec::new();
        for (signal, mask) in members {
            let net = self.signal_node(gl, signal, SignalKind::Net);
            let is_ext = self.nodes[net].flags().contains(NodeFlags::DRIVE_EXT)
                || gl.signals[signal].flags.contains(SignalFlags::EXT_DRIVE);
            if !is_ext && self.nodes[net].fst_in().is_none() {
                continue;
            }

            let s = &gl.signals[signal];
            let (name, size, mode) = (format!("::DRIVE({})", s.name), s.size, s.mode);
            let proxy = signal_proxy(gl, name, size, mode);
            let drive_proxy = self.signal_node(gl, proxy, SignalKind::Net);
            self.splice_in_edges(net, drive_proxy);
            self.drive_proxies.push(proxy);

            let driven = if is_ext {
                // The drive proxy is written instead of the member, so it keeps a value of its own
                // and every bit may be driven.
                gl.signals[signal].flags.set(SignalFlags::EXT_DRIVE, false);
                gl.signals[proxy].flags.set(SignalFlags::EXT_DRIVE, true);
                self.nodes[net]
                    .flags_mut()
                    .unwrap()
                    .remove(NodeFlags::DRIVE_EXT);
                *self.nodes[drive_proxy].flags_mut().unwrap() |= NodeFlags::DRIVE_EXT;
                self.write_fuse
                    .insert(signal, (proxy, SignalSlice::with_end(size)));
                vec![(0, size.get())]
            } else {
                // Only bits that are driven are forwarded, so an undriven bit is never resolved.
                // In two-valued logic it would read `0` instead of `z`.
                self.collect_in(drive_proxy, &mut ins);
                let mut driven = ins
                    .iter()
                    .map(|&e| self.dst_slice(e))
                    .map(|s| (s.lsb(), s.lsb() + s.width().get()))
                    .collect::<Vec<_>>();
                driven.sort_unstable();
                merge_ranges(driven)
            };

            // The driven bits that are in no closure drive the member directly.
            if let CoveredMask::Selection(start) = mask {
                let is_covered = |bit: u32| covered_bits.get(start + bit as usize);
                let mut bit = 0;
                while bit < size.get() {
                    if is_covered(bit) {
                        bit += 1;
                        continue;
                    }
                    let lo = bit;
                    while bit < size.get() && !is_covered(bit) {
                        bit += 1;
                    }
                    for (a, b) in intersect_ranges(&driven, (lo, bit)) {
                        self.push_edge((drive_proxy, a), (net, a), VectorSize::new(b - a).unwrap());
                    }
                }
            }
            drive_proxies.insert(signal, (drive_proxy, driven));
        }

        // 3b. The proxies of the closures. The entries of a group are contiguous. A closure with a
        //     four-valued member is resolved in four-valued logic.
        for chunk in entries.chunk_by(|a, b| a.0 == b.0) {
            let bit_length = groups[chunk[0].0];
            let mode = if chunk
                .iter()
                .any(|&(_, signal, _)| gl.signals[signal].mode.is_four_value())
            {
                LogicMode::FourValue
            } else {
                LogicMode::TwoValue
            };
            let proxy = signal_proxy(gl, "::TRAN".to_string(), bit_length, mode);
            let proxy = self.signal_node(gl, proxy, SignalKind::Net);

            for &(_, signal, offset) in chunk {
                if let Some((drive_proxy, driven)) = drive_proxies.get(&signal) {
                    for (a, b) in intersect_ranges(driven, (offset, offset + bit_length.get())) {
                        self.push_edge(
                            (*drive_proxy, a),
                            (proxy, a - offset),
                            VectorSize::new(b - a).unwrap(),
                        );
                    }
                }
                let net = self.signal_lut[&signal];
                self.push_edge((proxy, 0), (net, offset), bit_length);
            }
        }
    }

    /// Resolves all drivers.
    ///
    /// 1. Bidirectional connections are lowered to proxies.
    /// 2. The drive graph is simplified, every net with a single, simple driver is aliased to that
    ///    driver and every process that solely drives some bits drives them directly (see
    ///    [`alias`]).
    /// 3. The processes of all expressions, primitives and delays and of all remaining driven nets,
    ///    which resolve their drivers, are lowered.
    /// 4. All probes and drives of aliased signals are rewritten.
    ///
    /// Returns the aliased signals for probing and driving, and where external writes of nets go.
    pub fn finalize(
        mut self,
        ctx: &LowerContext<'a, '_>,
        mctx: &mut MutLowerContext<'a>,
        options: FinalizeOptions<'_>,
    ) -> Result<(ProbeFuseMap, WriteFuseMap), ()> {
        let mut error = self.push_reads(ctx, mctx).is_err();

        self.lower_bidirectional(&mut mctx.gl);

        if let Some(w) = options.unoptimized_dot {
            self.write_dot(&mctx.gl, w).unwrap();
        }
        self.collect_flags(&mctx.gl);
        self.alias(&mctx.gl, options.max_rounds, options.optimize);
        self.write_initial_values(ctx, &mut mctx.gl);
        if let Some(w) = options.optimized_dot {
            self.write_dot(&mctx.gl, w).unwrap();
        }

        error |= self.lower_exprs(ctx, mctx).is_err();
        error |= self.lower_primitives(ctx, mctx).is_err();
        error |= self.lower_delays(ctx, mctx).is_err();

        // Lower all remaining edges
        //
        // A --+
        //     |
        // B --+-> S
        //     |
        // C --+
        //
        // %i0 = lower(A)
        // %i1 = lower(B)
        // %i2 = lower(C)
        // %r1 = resolve_wire for bit 1
        // # ...
        // %rn = resolve_wire for bit n
        // %R = concat { %r1, ..., %rn }
        // %_ = drv S, %R
        let mut scratch = LowerDriversScratch::default();
        for &node in &self.lower {
            self.lower_drivers(mctx, &mut scratch, node);
        }

        // Only the writes of variables are redirected in the IR: the drive graph drives a net
        // itself, also if its external writes are redirected.
        let is_variable = |signal| {
            self.signal_lut
                .get(&signal)
                .is_some_and(|&node| matches!(self.nodes[node], DriveNode::Variable { .. }))
        };
        rewrite::apply_fuse_maps(
            &mut mctx.gl,
            &self.probe_fuse,
            &self.write_fuse,
            is_variable,
        );

        // The input of a delay that is aliased is no longer referenced.
        for node in self.nodes.iter() {
            if let DriveNode::Delay {
                kind: DelayKind::Assign { .. } | DelayKind::Gate { .. },
                input,
                ..
            } = node
                && self.probe_fuse.contains_key(input)
            {
                mctx.gl.signals.remove(*input);
            }
        }
        // As is a drive proxy that is aliased.
        for proxy in &self.drive_proxies {
            if self.probe_fuse.contains_key(proxy) {
                mctx.gl.signals.remove(*proxy);
            }
        }

        if error {
            return Err(());
        }

        Ok((self.probe_fuse, self.write_fuse))
    }

    /// Returns where the process of `node` drives the bits of each of its outgoing edges: the
    /// drivee directly or a proxy signal that the process of the drivee resolves.
    fn output_targets(
        &mut self,
        gl: &mut GlobalContext,
        mode: LogicMode,
        node: DriveNodeKey,
        name: &str,
    ) -> Vec<OutputTarget> {
        let mut targets = Vec::new();
        let mut next = self.nodes[node].fst_out();
        while let Some(e) = next {
            let edge = &self.edges[e];
            next = edge.next_out;

            let (signal, offset) = if self.direct.contains(&e) {
                let to = self.nodes[edge.to].drivee_signal().unwrap();
                (to, edge.to_offset)
            } else {
                let proxy = signal_proxy(gl, name.to_string(), edge.bit_length, mode);
                // The output of a process is `x` until the process first drives it, e.g. that of a
                // delay. Unlike a net proxy, it is not undriven (`z`).
                gl.signals[proxy].initialize = None;
                self.edge_proxies.insert(e, proxy);
                (proxy, 0)
            };
            let edge = &self.edges[e];
            targets.push(OutputTarget {
                src: SignalSlice::from_width(edge.from_offset, edge.bit_length).unwrap(),
                signal,
                offset,
            });
        }
        targets
    }

    /// Lowers the expressions of every net assignment to one process per assignment.
    ///
    /// The process drives the targets of the outgoing edges of the expressions and the wiring of
    /// the assignment that the drivees of the expressions take over.
    fn lower_exprs(
        &mut self,
        ctx: &LowerContext<'a, '_>,
        mctx: &mut MutLowerContext<'a>,
    ) -> Result<(), ()> {
        // Group the expressions and sign-extensions by their assignment.
        let mut assignments = VgHashMap::<AssignmentKey, usize>::default();
        let mut groups = Vec::<Vec<DriveNodeKey>>::new();
        for node in self.nodes.table_key_iter() {
            if let DriveNode::Expr {
                source,
                fst_out: Some(_),
                ..
            } = self.nodes[node]
                && let Some(assignment) = source.assignment()
            {
                let group = *assignments.entry(assignment).or_insert_with(|| {
                    groups.push(Vec::new());
                    groups.len() - 1
                });
                groups[group].push(node);
            }
        }

        let mut error = false;
        let mut used_signals = OrderedSet::new();
        for group in groups {
            let DriveNode::Expr { source, .. } = self.nodes[group[0]] else {
                unreachable!();
            };
            let assignment = source.assignment().unwrap();
            // The span of the first expression of the assignment, else that of a sign-extended
            // signal.
            let span = group
                .iter()
                .find_map(|&node| match self.nodes[node] {
                    DriveNode::Expr {
                        source: ExprSource::Assign { expr, .. },
                        ..
                    } => Some(ctx.arenas.get_span(expr)),
                    _ => None,
                })
                .unwrap_or_else(|| match source {
                    ExprSource::SignExtend { signal, .. } => mctx.gl.signals[signal].origin,
                    _ => unreachable!(),
                });

            let res: Result<(), ()> = (|| {
                let (proc_builder, mut bb_builder) =
                    ProcessBuilder::new(mctx.gl(), ProcessKind::Assign, span);
                let entry = proc_builder.entry();

                used_signals.clear();
                for &node in &group {
                    let DriveNode::Expr {
                        source, bit_length, ..
                    } = self.nodes[node]
                    else {
                        unreachable!();
                    };

                    let value = match source {
                        ExprSource::Assign {
                            expr,
                            scope,
                            ty: target_ty,
                            ..
                        } => {
                            get_used_signals(ctx, mctx, scope, &mut used_signals, expr)?;
                            let context_width =
                                (!target_ty.is_real()).then(|| target_ty.bit_length());
                            let (value, ty) =
                                lower_expr(ctx, mctx, scope, &mut bb_builder, expr, context_width)?;
                            let value = coerce_to(mctx.gl(), &mut bb_builder, value, ty, target_ty);
                            // A signed expression that is wider than its type is sign-extended.
                            if bit_length > target_ty.bit_length() {
                                bb_builder.sign_extend(mctx.gl(), value, bit_length)
                            } else {
                                value
                            }
                        }
                        ExprSource::SignExtend { signal, bit, .. } => {
                            used_signals.insert(signal);
                            let value = bb_builder.probe(mctx.gl(), signal);
                            let value =
                                bb_builder.slice_constant(mctx.gl(), value, bit, SCALAR_VSIZE);
                            if bit_length > SCALAR_VSIZE {
                                bb_builder.sign_extend(mctx.gl(), value, bit_length)
                            } else {
                                value
                            }
                        }
                        ExprSource::Gate { .. } | ExprSource::Udp { .. } => unreachable!(),
                    };

                    let targets = self.output_targets(&mut mctx.gl, ctx.logic_mode, node, "::EXPR");
                    drive_output(mctx, &mut bb_builder, &targets, 0, value);
                }
                for &e in self.expr_wires.get(&assignment).into_iter().flatten() {
                    let edge = &self.edges[e];
                    let value = match &self.nodes[edge.from] {
                        DriveNode::Constant { value, .. } => {
                            bb_builder.constant(mctx.gl(), value.clone())
                        }
                        node => {
                            let key = node.driver_signal().unwrap();
                            used_signals.insert(key);
                            bb_builder.probe(mctx.gl(), key)
                        }
                    };
                    let part = bb_builder.slice_constant(
                        mctx.gl(),
                        value,
                        edge.from_offset,
                        edge.bit_length,
                    );
                    let to = self.nodes[edge.to].drivee_signal().unwrap();
                    bb_builder.drive_partial_constant(mctx.gl(), to, part, edge.to_offset);
                }

                if used_signals.items.is_empty() {
                    bb_builder.halt(mctx.gl());
                } else {
                    bb_builder.watch_to(mctx.gl(), used_signals.items.clone(), entry);
                }
                proc_builder.finalize(mctx.gl());
                Ok(())
            })();
            error |= res.is_err();
        }

        if error {
            return Err(());
        }
        Ok(())
    }

    /// Lowers every output of a gate or user-defined primitive to a process that drives the
    /// targets of its outgoing edges.
    fn lower_primitives(
        &mut self,
        ctx: &LowerContext<'a, '_>,
        mctx: &mut MutLowerContext<'a>,
    ) -> Result<(), ()> {
        let mut error = false;
        for node in self.nodes.table_key_iter() {
            let DriveNode::Expr { source, .. } = self.nodes[node] else {
                continue;
            };
            if source.assignment().is_some() {
                // Lowered per assignment by `lower_exprs`.
                continue;
            }

            let targets = self.output_targets(&mut mctx.gl, ctx.logic_mode, node, "::PRIMITIVE");
            let drive = |mctx: &mut MutLowerContext<'a>,
                         builder: &mut BasicBlockBuilder,
                         offset: u32,
                         value: VariableKey| {
                drive_output(mctx, builder, &targets, offset, value)
            };
            error |= match source {
                ExprSource::Assign { .. } | ExprSource::SignExtend { .. } => unreachable!(),
                ExprSource::Gate {
                    gate,
                    scope,
                    instance,
                    output,
                } => lower_gate_primitive(ctx, mctx, scope, gate, instance, output, &drive),
                ExprSource::Udp {
                    udp,
                    scope,
                    instance,
                    state,
                } => lower_udp(
                    ctx,
                    mctx,
                    scope,
                    udp,
                    instance.input_terminals,
                    state,
                    &drive,
                ),
            }
            .is_err();
        }

        if error {
            return Err(());
        }
        Ok(())
    }

    /// Lowers every delay to a process that drives the targets of its outgoing edges with the
    /// value of its input after the delay.
    fn lower_delays(
        &mut self,
        ctx: &LowerContext<'a, '_>,
        mctx: &mut MutLowerContext<'a>,
    ) -> Result<(), ()> {
        let mut error = false;
        for node in self.nodes.table_key_iter() {
            let DriveNode::Delay {
                kind,
                input,
                bit_length,
                ..
            } = &self.nodes[node]
            else {
                continue;
            };
            let (kind, input, bit_length) = (kind.clone(), *input, *bit_length);

            let targets = self.output_targets(&mut mctx.gl, ctx.logic_mode, node, "::DELAY");
            let drive = |mctx: &mut MutLowerContext<'a>,
                         builder: &mut BasicBlockBuilder,
                         offset: u32,
                         value: VariableKey| {
                drive_output(mctx, builder, &targets, offset, value)
            };
            let assign = |mctx: &mut MutLowerContext<'a>,
                          builder: &mut BasicBlockBuilder,
                          value: VariableKey| {
                drive(mctx, builder, 0, value);
                Ok(())
            };

            let res = match kind {
                DelayKind::Assign { delay3, scope } => {
                    let delay = |mctx: &mut MutLowerContext<'a>,
                                 builder: &mut BasicBlockBuilder,
                                 new_value: VariableKey,
                                 driven_value: VariableKey| {
                        lower_assign_delay3(
                            ctx,
                            mctx,
                            scope,
                            builder,
                            delay3,
                            new_value,
                            driven_value,
                            bit_length,
                        )
                    };
                    lower_delay_process(
                        mctx,
                        ctx.arenas.get_span(delay3),
                        &delay as &GateDelayFn,
                        &assign as &AssignOutputFn,
                        input,
                        bit_length,
                    )
                }
                DelayKind::Gate { delay, scope } => {
                    let span = match delay {
                        GateDelay::Delay2(delay2) => ctx.arenas.get_span(delay2),
                        GateDelay::Delay3(delay3) => ctx.arenas.get_span(delay3),
                    };
                    let delay =
                        |mctx: &mut MutLowerContext<'a>,
                         builder: &mut BasicBlockBuilder,
                         new_value: VariableKey,
                         _driven_value: VariableKey| match delay {
                            GateDelay::Delay2(delay2) => lower_delay2(
                                ctx, mctx, scope, builder, delay2, new_value, bit_length,
                            ),
                            GateDelay::Delay3(delay3) => lower_delay3(
                                ctx, mctx, scope, builder, delay3, new_value, bit_length,
                            ),
                        };
                    lower_delay_process(
                        mctx,
                        span,
                        &delay as &GateDelayFn,
                        &assign as &AssignOutputFn,
                        input,
                        bit_length,
                    )
                }
                DelayKind::Specify(specify) => {
                    lower_specify_delay(ctx, mctx, &specify, input, &drive)
                }
            };
            error |= res.is_err();
        }

        if error {
            return Err(());
        }
        Ok(())
    }

    /// Lowers the incoming edges of a node that are not driven directly into one process that
    /// resolves them and drives the result into the drivee signal of the node.
    ///
    /// The signal is split into segments at the boundaries of every incoming edge, so that each
    /// segment is covered by the same set of drivers for all of its bits. Each segment is resolved
    /// with `resolve_wire` and every contiguous run of driven segments is concatenated back together
    /// and driven. Bits that are not covered by any edge are left untouched, as they may be driven
    /// outside of the drive graph.
    ///
    /// A process does not observe its own drives, so bits of `drivee` that drive other bits of
    /// `drivee`, e.g. `assign w[1] = w[0]`, are read through a proxy that a separate process keeps
    /// up to date.
    fn lower_drivers(
        &self,
        mctx: &mut MutLowerContext<'a>,
        scratch: &mut LowerDriversScratch,
        node: DriveNodeKey,
    ) {
        let drivee = self.nodes[node]
            .drivee_signal()
            .expect("only drivees are lowered");
        let Some(fst_in) = self.nodes[node].fst_in() else {
            return;
        };

        let LowerDriversScratch {
            inputs,
            boundaries,
            self_reads,
        } = scratch;
        inputs.clear();
        boundaries.clear();
        self_reads.clear();

        let (proc_builder, mut bb_builder) =
            ProcessBuilder::new(mctx.gl(), ProcessKind::Fuse, TokenRange::default());
        let entry = proc_builder.entry();
        let mut watchers = Vec::new();

        // %i0 = lower(A), %i1 = lower(B), ...
        let mut next_in = Some(fst_in);
        while let Some(edge_key) = next_in {
            let edge = &self.edges[edge_key];
            next_in = edge.next_in;

            if self.direct.contains(&edge_key) || self.is_expr_wire(edge_key) {
                continue;
            }

            // The proxy of a process holds the bits that the edge drives.
            let source = match &self.nodes[edge.from] {
                DriveNode::Expr { .. } | DriveNode::Delay { .. } => {
                    Ok((self.edge_proxies[&edge_key], 0))
                }
                DriveNode::Constant { value, .. } => {
                    Err(value.slicez(edge.from_offset, edge.bit_length))
                }
                node => Ok((node.driver_signal().unwrap(), edge.from_offset)),
            };
            let width = edge.bit_length;
            let value = match source
                .and_then(|(key, offset)| resolve_alias(&self.probe_fuse, key, offset, width))
            {
                Ok((key, offset)) if key == drivee => {
                    let s = &mctx.gl.signals[drivee];
                    let (name, mode) = (format!("::SELF({})", s.name), s.mode);
                    let proxy = signal_proxy(&mut mctx.gl, name, width, mode);
                    self_reads.push((proxy, offset, width));

                    watchers.push(WatchCondition {
                        signal: proxy,
                        edge: WatchEdge::Any { bit_length: width },
                        offset: 0,
                    });
                    bb_builder.probe(mctx.gl(), proxy)
                }
                Ok((key, offset)) => {
                    watchers.push(WatchCondition {
                        signal: key,
                        edge: WatchEdge::Any { bit_length: width },
                        offset,
                    });
                    bb_builder.probe_slice_constant(mctx.gl(), key, offset, width)
                }
                Err(value) => bb_builder.constant(mctx.gl(), value),
            };

            // Every input is converted to the mode of the drivee before it is resolved, as a value
            // is stored in a signal: e.g. a two-valued net reads `z` from a four-valued input as
            // `0`.
            let drivee_mode = mctx.gl.signals[drivee].mode;
            let value = bb_builder.convert_mode(mctx.gl(), value, drivee_mode);

            boundaries.push(edge.to_offset);
            boundaries.push(edge.to_offset + edge.bit_length.get());
            inputs.push((edge.to_offset, edge.bit_length, value));
        }

        boundaries.sort_unstable();
        boundaries.dedup();

        // %r_i = resolve_wire for segment i
        // %R = concat { %r_n, ..., %r_1 }
        //
        // `run` is the current contiguous run of driven segments with its least significant bit.
        let mut run: Option<(u32, VariableKey)> = None;
        for segment in boundaries.windows(2) {
            let (lo, hi) = (segment[0], segment[1]);
            let width = VectorSize::new(hi - lo).unwrap();

            let mut acc: Option<VariableKey> = None;
            for &(to_offset, input_width, value) in inputs.iter() {
                if to_offset > lo || hi > to_offset + input_width.get() {
                    continue;
                }

                let part = bb_builder.slice_constant(mctx.gl(), value, lo - to_offset, width);
                acc = Some(match acc {
                    None => part,
                    Some(acc) => bb_builder.resolve_wire(mctx.gl(), acc, part),
                });
            }
            run = match (acc, run) {
                (None, None) => None,
                (None, Some((lsb, value))) => {
                    // %_ = drv S[lsb +: width], %R
                    bb_builder.drive_partial_constant(mctx.gl(), drivee, value, lsb);
                    None
                }
                (Some(acc), None) => Some((lo, acc)),
                (Some(acc), Some((lsb, value))) => {
                    Some((lsb, bb_builder.concat(mctx.gl(), acc, value)))
                }
            };
        }
        if let Some((lsb, value)) = run {
            // %_ = drv S[lsb +: width], %R
            bb_builder.drive_partial_constant(mctx.gl(), drivee, value, lsb);
        }

        if watchers.is_empty() {
            // Only constant drivers: drive once.
            bb_builder.halt(mctx.gl());
        } else {
            bb_builder.watch_conditions_to(mctx.gl(), watchers, entry);
        }
        proc_builder.finalize(mctx.gl());

        if self_reads.is_empty() {
            return;
        }

        // %p_i = prb S[offset_i +: width_i]
        // %_ = drv P_i, %p_i
        let (proc_builder, mut bb_builder) =
            ProcessBuilder::new(mctx.gl(), ProcessKind::Fuse, TokenRange::default());
        let entry = proc_builder.entry();
        let mut watchers = Vec::with_capacity(self_reads.len());
        for &(proxy, offset, bit_length) in self_reads.iter() {
            let value = bb_builder.probe_slice_constant(mctx.gl(), drivee, offset, bit_length);
            bb_builder.drive(mctx.gl(), proxy, value);
            watchers.push(WatchCondition {
                signal: drivee,
                edge: WatchEdge::Any { bit_length },
                offset,
            });
        }
        bb_builder.watch_conditions_to(mctx.gl(), watchers, entry);
        proc_builder.finalize(mctx.gl());
    }

    fn is_expr_wire(&self, e: DriveEdgeKey) -> bool {
        self.edges[e].assignment.is_some_and(|assignment| {
            self.expr_wires
                .get(&assignment)
                .is_some_and(|wires| wires.contains(&e))
        })
    }
}

/// Drives `value`, the bits of the output of a process at `offset`, into the targets that overlap
/// them.
fn drive_output(
    mctx: &mut MutLowerContext<'_>,
    builder: &mut BasicBlockBuilder,
    targets: &[OutputTarget],
    offset: u32,
    value: VariableKey,
) {
    let end = offset + mctx.gl.vars.size(value).get();
    for target in targets {
        let lo = cmp::max(offset, target.src.lsb());
        let hi = cmp::min(end, target.src.msb() + 1);
        let Some(width) = hi.checked_sub(lo).and_then(VectorSize::new) else {
            continue;
        };

        let part = builder.slice_constant(mctx.gl(), value, lo - offset, width);
        let offset = target.offset + (lo - target.src.lsb());
        builder.drive_partial_constant(mctx.gl(), target.signal, part, offset);
    }
}

/// Resolves `key[offset +: bit_length]` through the aliases.
///
/// Returns the signal and offset to probe instead or the constant value of the bits.
fn resolve_alias(
    probe_fuse: &ProbeFuseMap,
    mut key: SignalKey,
    mut offset: u32,
    bit_length: VectorSize,
) -> Result<(SignalKey, u32), Bits> {
    while let Some(target) = probe_fuse.get(&key) {
        match target {
            FuseTarget::Signal(to, slice) => {
                key = *to;
                offset += slice.map_or(0, |slice| slice.lsb());
            }
            FuseTarget::Constant(value) => return Err(value.slicez(offset, bit_length)),
        }
    }
    Ok((key, offset))
}

pub struct FinalizeOptions<'w> {
    /// Maximum number of rounds to simplify a subgraph of the drive graph.
    pub max_rounds: usize,
    /// Whether the drive graph is optimized at all, i.e. simplified and aliased. Without it, every
    /// driven net is resolved by its own process.
    pub optimize: bool,
    /// Write the drive graph before simplification in the DOT format.
    pub unoptimized_dot: Option<&'w mut dyn Write>,
    /// Write the drive graph after simplification in the DOT format.
    pub optimized_dot: Option<&'w mut dyn Write>,
}

#[derive(Default)]
struct LowerDriversScratch {
    /// `(to_offset, bit_length, value)` of every lowered incoming edge.
    inputs: Vec<(u32, VectorSize, VariableKey)>,
    /// Segment boundaries within the drivee.
    boundaries: Vec<u32>,
    /// `(proxy, offset, bit_length)` of every read of the drivee by its own drivers.
    self_reads: Vec<(SignalKey, u32, VectorSize)>,
}

/// The bits of a member of a bidirectional closure that are in a closure.
#[derive(Clone, Copy)]
enum CoveredMask {
    /// All bits.
    Entire,
    /// The bits at the given index into the bitset of covered bits, one per bit of the signal.
    Selection(usize),
}

/// Merges sorted, half-open ranges into disjoint ranges. Touching ranges are joined.
fn merge_ranges(ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    let mut merged = Vec::<(u32, u32)>::with_capacity(ranges.len());
    for (lo, hi) in ranges {
        match merged.last_mut() {
            Some((_, last_hi)) if lo <= *last_hi => *last_hi = (*last_hi).max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    merged
}

/// Intersects sorted, disjoint, half-open `ranges` with `(lo, hi)`.
fn intersect_ranges(
    ranges: &[(u32, u32)],
    (lo, hi): (u32, u32),
) -> impl Iterator<Item = (u32, u32)> {
    ranges
        .iter()
        .map(move |&(a, b)| (a.max(lo), b.min(hi)))
        .filter(|(a, b)| a < b)
}
