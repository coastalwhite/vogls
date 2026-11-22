//! Property test for the lowering of bidirectional connections and the simplification of the drive
//! graph, see [`DriverResolution::lower_bidirectional`] and [`DriverResolution::alias`].
//!
//! Random drive graphs are evaluated symbolically before and after all graph transformations.
//! Every bit that is written outside of the drive graph (by a process or externally), every output
//! bit of a process and every constant bit is an _atom_. The value of a bit is the set of atoms that
//! reach it through the drive edges, i.e. the inputs of its resolution function. Resolution is
//! idempotent, commutative and associative, so a set describes the value exactly. Bits that are
//! connected bidirectionally (`tran`) are one bit before lowering: they share the resolution of all
//! of their drivers.
//!
//! The graph after the transformations is evaluated the way lowering realizes it: an aliased node is
//! no longer driven, drives go through the drive fuse map and probes go through the probe fuse map,
//! with the single lookup that [`super::rewrite::apply_fuse_maps`] does.
//!
//! Loosely following the observable equivalence of the fuse signals pass, it must hold that:
//!
//! - Probes: every bit of every signal probes the same atoms.
//! - Drives: every atom that is driven reaches the same probed bits.
//! - Watches: a watch, rewritten with [`WatchCondition::fused_into`], wakes on the same atoms.
//! - Last update times: the signal that a `LastUpdateTime` is rewritten to is updated by the same
//!   atoms. This works per signal, so a signal may only be fused into an entire signal.
//!
//! Besides that, the bookkeeping that lowering relies on is checked: every incoming edge of a node
//! that is not aliased is realized, a direct driver does not read what it drives and read edges
//! follow the aliases.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use proptest::prelude::*;
use proptest::sample::Index;
use vogls_frontend::ident_table::IdentTable;
use vogls_frontend::symbol_table::SymbolTable;
use vogls_ir::SCALAR_VSIZE;

use super::*;
use crate::ast::AstIdRange;
use crate::ast::statement::DelayValue;
use crate::lower::Diagnostics;

// -------------------------------------------------------------------------------------------------
// Specification
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum KindSpec {
    Net,
    Variable,
    Constant(u64),
    Expr,
    /// Sign-extends the most significant bit of the selected net or variable.
    SignExtend(Index),
    /// An assign delay, whose input is driven by exactly one expression.
    Delay,
    /// The module path delays of an output, whose input is an ordinary net: it can have several
    /// drivers or be driven partially.
    SpecifyDelay,
}

#[derive(Debug, Clone)]
struct NodeSpec {
    kind: KindSpec,
    width: u32,
    four_value: bool,
    /// Nets: written outside of the drive graph. Variables always are.
    drive_ext: bool,
    /// Nets and variables: the last update time is observed.
    lupdt: bool,
    /// Variables: poked at `t=0`.
    poke_t0: bool,
}

/// An edge from any node into a net. The raw numbers are normalized to a valid edge.
#[derive(Debug, Clone)]
struct EdgeSpec {
    from: Index,
    to: Index,
    /// Spans as many bits as possible.
    full: bool,
    width: u32,
    from_offset: u32,
    to_offset: u32,
    /// The expression of whose assignment a wiring edge is part of.
    assignment: Option<Index>,
}

/// A piece of the edges that tile a net or the input of a delay.
#[derive(Debug, Clone)]
struct PieceSpec {
    from: Index,
    /// Spans the entire source, as far as it fits.
    full: bool,
    width: u32,
    offset: u32,
}

/// A process (expression or delay) that reads an entire net or variable.
#[derive(Debug, Clone)]
struct ReadSpec {
    process: Index,
    signal: Index,
}

/// A bidirectional connection (`tran`) between two nets.
#[derive(Debug, Clone)]
struct BidirSpec {
    left: Index,
    right: Index,
    /// Spans as many bits as possible.
    full: bool,
    width: u32,
    left_offset: u32,
    right_offset: u32,
}

#[derive(Debug, Clone)]
struct WatchSpec {
    signal: Index,
    width: u32,
    offset: u32,
}

#[derive(Debug, Clone)]
struct GraphSpec {
    nodes: Vec<NodeSpec>,
    edges: Vec<EdgeSpec>,
    /// Nets that are driven by a concatenation, e.g. `assign n = {a, b[1], c + d}`.
    concats: Vec<Index>,
    /// The pieces of all concatenations and delay inputs, in a cycle.
    pieces: Vec<PieceSpec>,
    /// Assignments of an expression and wiring to several nets, e.g. `assign {x, y} = {e, x}`. The
    /// first index selects the expression and the others the nets.
    assigns: Vec<(Index, Vec<Index>)>,
    reads: Vec<ReadSpec>,
    watches: Vec<WatchSpec>,
    bidirs: Vec<BidirSpec>,
    /// The maximum number of rounds to simplify a subgraph. Few rounds leave chains of aliases.
    max_rounds: usize,
    /// Whether the drive graph is optimized at all.
    optimize: bool,
}

fn node_spec() -> impl Strategy<Value = NodeSpec> {
    let kind = prop_oneof![
        8 => Just(KindSpec::Net),
        3 => Just(KindSpec::Variable),
        2 => any::<u64>().prop_map(KindSpec::Constant),
        1 => Just(KindSpec::Expr),
        1 => any::<Index>().prop_map(KindSpec::SignExtend),
        1 => Just(KindSpec::Delay),
        1 => Just(KindSpec::SpecifyDelay),
    ];
    (
        kind,
        1..=4u32,
        any::<bool>(),
        prop::bool::weighted(0.1),
        prop::bool::weighted(0.2),
        any::<bool>(),
    )
        .prop_map(
            |(kind, width, four_value, drive_ext, lupdt, poke_t0)| NodeSpec {
                kind,
                width,
                four_value,
                drive_ext,
                lupdt,
                poke_t0,
            },
        )
}

fn edge_spec() -> impl Strategy<Value = EdgeSpec> {
    (
        any::<Index>(),
        any::<Index>(),
        prop::bool::weighted(0.6),
        any::<u32>(),
        any::<u32>(),
        any::<u32>(),
        prop::option::weighted(0.3, any::<Index>()),
    )
        .prop_map(
            |(from, to, full, width, from_offset, to_offset, assignment)| EdgeSpec {
                from,
                to,
                full,
                width,
                from_offset,
                to_offset,
                assignment,
            },
        )
}

fn graph_spec() -> impl Strategy<Value = GraphSpec> {
    let piece = (any::<Index>(), any::<bool>(), any::<u32>(), any::<u32>()).prop_map(
        |(from, full, width, offset)| PieceSpec {
            from,
            full,
            width,
            offset,
        },
    );
    let read =
        (any::<Index>(), any::<Index>()).prop_map(|(process, signal)| ReadSpec { process, signal });
    let watch = (any::<Index>(), any::<u32>(), any::<u32>()).prop_map(|(signal, width, offset)| {
        WatchSpec {
            signal,
            width,
            offset,
        }
    });
    let bidir = (
        any::<Index>(),
        any::<Index>(),
        prop::bool::weighted(0.6),
        any::<u32>(),
        any::<u32>(),
        any::<u32>(),
    )
        .prop_map(
            |(left, right, full, width, left_offset, right_offset)| BidirSpec {
                left,
                right,
                full,
                width,
                left_offset,
                right_offset,
            },
        );
    (
        prop::collection::vec(node_spec(), 1..10),
        prop::collection::vec(edge_spec(), 0..10),
        prop::collection::vec(any::<Index>(), 0..4),
        prop::collection::vec(piece, 1..8),
        prop::collection::vec(read, 0..4),
        prop::collection::vec(watch, 0..4),
        prop::collection::vec(bidir, 0..5),
        prop::collection::vec(
            (any::<Index>(), prop::collection::vec(any::<Index>(), 2..4)),
            0..2,
        ),
        prop_oneof![1 => Just(DEFAULT_MAX_ROUNDS), 1 => 0..=2usize],
        prop::bool::weighted(0.9),
    )
        .prop_map(
            |(
                nodes,
                edges,
                concats,
                pieces,
                reads,
                watches,
                bidirs,
                assigns,
                max_rounds,
                optimize,
            )| {
                GraphSpec {
                    nodes,
                    edges,
                    concats,
                    pieces,
                    assigns,
                    reads,
                    watches,
                    bidirs,
                    max_rounds,
                    optimize,
                }
            },
        )
}

// -------------------------------------------------------------------------------------------------
// Construction
// -------------------------------------------------------------------------------------------------

/// Placeholders for the syntax that processes refer to. The drive graph never looks at them.
struct Ast<'a> {
    expr: AstId<'a, Expr<'a>>,
    delay3: AstId<'a, Delay3<'a>>,
    scope: SymbolId,
}

/// What is observed of the drive graph from the outside.
#[derive(Default)]
struct World {
    /// The nets, variables and delay inputs.
    signals: Vec<SignalKey>,
    /// Signals that are written outside of the drive graph.
    writes: Vec<SignalKey>,
    /// Signals whose last update time is observed. A delay observes that of its input.
    lupdt: Vec<SignalKey>,
    watches: Vec<WatchCondition>,
    /// The names of process nodes.
    processes: VgHashMap<DriveNodeKey, String>,
}

fn build<'a>(spec: &GraphSpec, ast: &Ast<'a>) -> (DriverResolution<'a>, GlobalContext, World) {
    let mut gl = GlobalContext::default();
    let mut dr = DriverResolution::default();
    let mut world = World::default();

    let mut keys = Vec::with_capacity(spec.nodes.len());
    let mut signals = Vec::with_capacity(spec.nodes.len());
    for (i, n) in spec.nodes.iter().enumerate() {
        let width = VectorSize::new(n.width).unwrap();
        let mode = if n.four_value {
            LogicMode::FourValue
        } else {
            LogicMode::TwoValue
        };
        let mut new_signal = |name: String, flags: SignalFlags| {
            gl.signals.insert(Signal {
                name,
                size: width,
                initialize: None,
                mode,
                flags,
                origin: TokenRange::default(),
            })
        };

        let (key, signal) = match n.kind {
            KindSpec::Net | KindSpec::Variable => {
                let is_variable = matches!(n.kind, KindSpec::Variable);
                // Every variable is written outside of the drive graph, by its processes or a
                // `$readmem`. Those writes are redirected with the variable when it is inverted, so
                // only a net that is written externally is flagged.
                let mut signal_flags = SignalFlags::EMPTY;
                signal_flags.set(SignalFlags::POKE_T0, is_variable && n.poke_t0);
                signal_flags.set(SignalFlags::EXT_DRIVE, !is_variable && n.drive_ext);
                let signal = new_signal(format!("s{i}"), signal_flags);
                let kind = if is_variable {
                    SignalKind::Variable
                } else {
                    SignalKind::Net(crate::elaborate::NetResolution::Wire)
                };
                let key = dr.signal_node(&gl, signal, kind);

                let mut flags = NodeFlags::EMPTY;
                if !is_variable && n.drive_ext {
                    flags |= NodeFlags::DRIVE_EXT;
                }
                if n.lupdt {
                    flags |= NodeFlags::LUPDT;
                    world.lupdt.push(signal);
                }
                *dr.nodes[key].flags_mut().unwrap() = flags;

                world.signals.push(signal);
                if is_variable || n.drive_ext {
                    world.writes.push(signal);
                }
                (key, Some(signal))
            }
            KindSpec::Constant(bits) => {
                let mask = u64::MAX >> (64 - n.width);
                let key = dr.nodes.insert(DriveNode::Constant {
                    value: Bits::from_u64(width, bits & mask),
                    fst_out: None,
                    fst_read_out: None,
                });
                (key, None)
            }
            KindSpec::Expr => {
                let assignment = dr.new_assignment();
                let key = dr.nodes.insert(DriveNode::Expr {
                    source: ExprSource::Assign {
                        expr: ast.expr,
                        scope: ast.scope,
                        assignment,
                        ty: VType::UnsignedNet(width),
                    },
                    bit_length: width,
                    fst_out: None,
                    fst_read_in: None,
                });
                world.processes.insert(key, format!("p{i}"));
                (key, None)
            }
            KindSpec::SignExtend(_) => {
                // The signal it reads is selected once all signals exist.
                let assignment = dr.new_assignment();
                let key = dr.nodes.insert(DriveNode::Expr {
                    source: ExprSource::SignExtend {
                        signal: SignalKey::default(),
                        bit: 0,
                        assignment,
                    },
                    bit_length: width,
                    fst_out: None,
                    fst_read_in: None,
                });
                world.processes.insert(key, format!("p{i}"));
                (key, None)
            }
            KindSpec::Delay | KindSpec::SpecifyDelay => {
                let input = new_signal(format!("p{i}.in"), SignalFlags::EMPTY);
                let kind = match n.kind {
                    KindSpec::Delay => DelayKind::Assign {
                        delay3: ast.delay3,
                        scope: ast.scope,
                    },
                    _ => DelayKind::Specify(SpecifyDelay {
                        output: input,
                        inputs: 0..0,
                    }),
                };
                let key = dr.nodes.insert(DriveNode::Delay {
                    kind,
                    input,
                    bit_length: width,
                    fst_in: None,
                    fst_out: None,
                    fst_read_in: None,
                });
                dr.signal_lut.insert(input, key);
                world.processes.insert(key, format!("p{i}"));
                world.signals.push(input);
                world.lupdt.push(input);
                (key, Some(input))
            }
        };
        keys.push(key);
        signals.push(signal);
    }

    let width = |i: usize| spec.nodes[i].width;
    let indices = |f: fn(&KindSpec) -> bool| {
        (0..spec.nodes.len())
            .filter(|&i| f(&spec.nodes[i].kind))
            .collect::<Vec<_>>()
    };
    let nets = indices(|k| matches!(k, KindSpec::Net));
    // Nets and the inputs of specify delays, which drive edges go into.
    let drivees = indices(|k| matches!(k, KindSpec::Net | KindSpec::SpecifyDelay));
    // Nodes with an assignment.
    let exprs = indices(|k| matches!(k, KindSpec::Expr | KindSpec::SignExtend(_)));
    let delays = indices(|k| matches!(k, KindSpec::Delay));
    let readers =
        indices(|k| matches!(k, KindSpec::Expr | KindSpec::Delay | KindSpec::SpecifyDelay));
    let readables = indices(|k| matches!(k, KindSpec::Net | KindSpec::Variable));

    let assignment_of = |dr: &DriverResolution, i: usize| match dr.nodes[keys[i]] {
        DriveNode::Expr { source, .. } => source.assignment().unwrap(),
        _ => unreachable!(),
    };

    // A sign-extension reads the most significant bit of a net or variable, like those that
    // `push_sites` adds for a signed signal that is narrower than its destinations.
    for (i, n) in spec.nodes.iter().enumerate() {
        let KindSpec::SignExtend(source) = &n.kind else {
            continue;
        };
        if readables.is_empty() {
            continue;
        }
        let r = readables[source.index(readables.len())];
        let DriveNode::Expr {
            source: ExprSource::SignExtend { signal, bit, .. },
            ..
        } = &mut dr.nodes[keys[i]]
        else {
            unreachable!();
        };
        *signal = signals[r].unwrap();
        *bit = width(r) - 1;
        let bit_length = VectorSize::new(width(r)).unwrap();
        dr.push_read_edge(ReadKind::Value, (keys[r], 0), keys[i], bit_length);
    }

    // Drive edges into nets and the inputs of specify delays.
    if !drivees.is_empty() {
        for e in &spec.edges {
            let from = e.from.index(spec.nodes.len());
            let to = drivees[e.to.index(drivees.len())];
            let (fw, tw) = (width(from), width(to));

            let max = cmp::min(fw, tw);
            let bit_length = if e.full { max } else { 1 + e.width % max };
            let from_offset = e.from_offset % (fw - bit_length + 1);
            let to_offset = e.to_offset % (tw - bit_length + 1);

            let edge = dr.push_edge(
                (keys[from], from_offset),
                (keys[to], to_offset),
                VectorSize::new(bit_length).unwrap(),
            );
            let assignment = match spec.nodes[from].kind {
                KindSpec::Expr | KindSpec::SignExtend(_) => Some(assignment_of(&dr, from)),
                KindSpec::Net | KindSpec::Variable | KindSpec::Constant(_) => e
                    .assignment
                    .filter(|_| !exprs.is_empty())
                    .map(|a| assignment_of(&dr, exprs[a.index(exprs.len())])),
                KindSpec::Delay | KindSpec::SpecifyDelay => None,
            };
            dr.edges[edge].assignment = assignment;
        }
    }

    // Concatenations and the incoming edges of a delay tile their drivee.
    let mut pieces = spec.pieces.iter().cycle();
    let mut tile = |dr: &mut DriverResolution, to: usize, sources: &[usize]| {
        let mut edges = Vec::new();
        let mut at = 0;
        while !sources.is_empty() && at < width(to) {
            let piece = pieces.next().unwrap();
            let from = sources[piece.from.index(sources.len())];
            let max = cmp::min(width(to) - at, width(from));
            let piece_width = if piece.full {
                max
            } else {
                1 + piece.width % max
            };
            let from_offset = piece.offset % (width(from) - piece_width + 1);
            let edge = dr.push_edge(
                (keys[from], from_offset),
                (keys[to], at),
                VectorSize::new(piece_width).unwrap(),
            );
            edges.push((edge, from));
            at += piece_width;
        }
        edges
    };
    let all_but = |to: usize| {
        (0..spec.nodes.len())
            .filter(|&i| i != to)
            .collect::<Vec<_>>()
    };
    if !nets.is_empty() {
        for c in &spec.concats {
            let to = nets[c.index(nets.len())];
            let edges = tile(&mut dr, to, &all_but(to));
            // The wiring is part of the assignment of the first expression.
            let wiring = edges
                .iter()
                .find(|&&(_, from)| {
                    matches!(
                        spec.nodes[from].kind,
                        KindSpec::Expr | KindSpec::SignExtend(_)
                    )
                })
                .map(|&(_, from)| assignment_of(&dr, from));
            for (edge, from) in edges {
                dr.edges[edge].assignment = match spec.nodes[from].kind {
                    KindSpec::Expr | KindSpec::SignExtend(_) => Some(assignment_of(&dr, from)),
                    KindSpec::Net | KindSpec::Variable | KindSpec::Constant(_) => wiring,
                    KindSpec::Delay | KindSpec::SpecifyDelay => None,
                };
            }
        }
    }
    if !nets.is_empty() && !exprs.is_empty() {
        for (expr, targets) in &spec.assigns {
            let expr = exprs[expr.index(exprs.len())];
            let mut targets = targets
                .iter()
                .map(|t| nets[t.index(nets.len())])
                .collect::<Vec<_>>();
            targets.sort_unstable();
            targets.dedup();
            let assignment = assignment_of(&dr, expr);
            for &to in &targets {
                // The pieces come from the expression and the other targets.
                let sources = std::iter::once(expr)
                    .chain(targets.iter().copied().filter(|&t| t != to))
                    .collect::<Vec<_>>();
                for (edge, _) in tile(&mut dr, to, &sources) {
                    dr.edges[edge].assignment = Some(assignment);
                }
            }
        }
    }
    for &d in &delays {
        tile(&mut dr, d, &all_but(d));
    }

    // Reads of the processes.
    if !readers.is_empty() && !readables.is_empty() {
        for r in &spec.reads {
            let process = readers[r.process.index(readers.len())];
            let signal = readables[r.signal.index(readables.len())];
            let kind = match spec.nodes[process].kind {
                KindSpec::Expr => ReadKind::Value,
                _ => ReadKind::Event,
            };
            let bit_length = VectorSize::new(width(signal)).unwrap();
            dr.push_read_edge(kind, (keys[signal], 0), keys[process], bit_length);
        }
    }

    // Bidirectional connections between nets, including nets that are written externally.
    let tran_nets = nets;
    if !tran_nets.is_empty() {
        for b in &spec.bidirs {
            let left = tran_nets[b.left.index(tran_nets.len())];
            let right = tran_nets[b.right.index(tran_nets.len())];
            let (lw, rw) = (width(left), width(right));
            let max = cmp::min(lw, rw);
            let bit_length = if b.full { max } else { 1 + b.width % max };
            let slice = |offset: u32, w: u32| {
                SignalSlice::from_width(
                    offset % (w - bit_length + 1),
                    VectorSize::new(bit_length).unwrap(),
                )
                .unwrap()
            };
            dr.push_bidirectional(
                (signals[left].unwrap(), slice(b.left_offset, lw)),
                (signals[right].unwrap(), slice(b.right_offset, rw)),
                BidirKind::Tran,
            );
        }
    }

    // Watches of processes outside of the drive graph.
    if !readables.is_empty() {
        for w in &spec.watches {
            let i = readables[w.signal.index(readables.len())];
            let bit_length = 1 + w.width % width(i);
            let offset = w.offset % (width(i) - bit_length + 1);
            world.watches.push(WatchCondition {
                signal: signals[i].unwrap(),
                edge: WatchEdge::Any {
                    bit_length: VectorSize::new(bit_length).unwrap(),
                },
                offset,
            });
            *dr.nodes[keys[i]].flags_mut().unwrap() |= NodeFlags::WATCH;
        }
    }

    (dr, gl, world)
}

// -------------------------------------------------------------------------------------------------
// Symbolic evaluation
// -------------------------------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Atom {
    Const(bool),
    /// A bit of a signal that is written outside of the drive graph.
    Write(String, u32),
    /// A bit of the output of a process.
    Process(String, u32),
    /// The atom, converted to two-valued logic: `x` and `z` became `0`.
    TwoValue(Box<Atom>),
}

impl Atom {
    /// Whether a change of the atom can be observed as an event.
    fn is_event(&self) -> bool {
        match self {
            Self::Const(_) => false,
            Self::Write(..) | Self::Process(..) => true,
            Self::TwoValue(atom) => atom.is_event(),
        }
    }

    /// The atom as it is stored in a signal of `mode`.
    ///
    /// A value is converted when it is stored, so the inputs of a two-valued net are converted
    /// before they are resolved. The transformations never move an atom past a resolution, so
    /// whether an atom was converted on its way describes its value exactly.
    fn stored_in(self, mode: LogicMode) -> Self {
        match (self, mode) {
            (atom @ (Self::Const(_) | Self::TwoValue(_)), _) | (atom, LogicMode::FourValue) => atom,
            (atom, LogicMode::TwoValue) => Self::TwoValue(Box::new(atom)),
        }
    }
}

impl fmt::Debug for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(b) => write!(f, "1'b{}", u8::from(*b)),
            Self::Write(s, bit) => write!(f, "{s}[{bit}]"),
            Self::Process(p, bit) => write!(f, "{p}[{bit}]"),
            Self::TwoValue(atom) => write!(f, "tv({atom:?})"),
        }
    }
}

type Value = BTreeSet<Atom>;
type Values = BTreeMap<(SignalKey, u32), Value>;

fn const_bit(value: &Bits, bit: u32) -> bool {
    value.slicez(bit, SCALAR_VSIZE) == Bits::from(true)
}

/// Where a probe of `signal[bit]` is redirected to, or the constant it is replaced with.
fn probe_location(
    dr: &DriverResolution,
    signal: SignalKey,
    bit: u32,
) -> Result<(SignalKey, u32), bool> {
    match dr.probe_fuse.get(&signal) {
        None => Ok((signal, bit)),
        Some(FuseTarget::Signal(to, slice)) => Ok((*to, bit + slice.map_or(0, |s| s.lsb()))),
        Some(FuseTarget::Constant(value)) => Err(const_bit(value, bit)),
    }
}

/// Where an external write of `signal[bit]` goes to, the way a signal handle resolves it: through
/// every entry of the write fuse map.
fn external_write_location(dr: &DriverResolution, signal: SignalKey, bit: u32) -> (SignalKey, u32) {
    match dr.write_fuse.get(&signal) {
        None => (signal, bit),
        Some(&(to, slice)) => (to, slice.lsb() + bit),
    }
}

/// Where a drive of `signal[bit]` in the IR is redirected to: through the entries of variables in
/// the write fuse map, as `apply_fuse_maps` does.
fn drive_location(dr: &DriverResolution, signal: SignalKey, bit: u32) -> (SignalKey, u32) {
    let is_variable = dr
        .signal_lut
        .get(&signal)
        .is_some_and(|&node| matches!(dr.nodes[node], DriveNode::Variable { .. }));
    match dr.write_fuse.get(&signal) {
        Some(&(to, slice)) if is_variable => (to, slice.lsb() + bit),
        _ => (signal, bit),
    }
}

fn probe(dr: &DriverResolution, values: &Values, signal: SignalKey, bit: u32) -> Value {
    match probe_location(dr, signal, bit) {
        Ok(location) => values.get(&location).cloned().unwrap_or_default(),
        Err(b) => Value::from([Atom::Const(b)]),
    }
}

/// The value of bit `bit` that `node` drives.
fn source(
    dr: &DriverResolution,
    world: &World,
    values: &Values,
    node: DriveNodeKey,
    bit: u32,
) -> Value {
    match &dr.nodes[node] {
        DriveNode::Net { key, .. } | DriveNode::Variable { key, .. } => {
            probe(dr, values, *key, bit)
        }
        DriveNode::Constant { value, .. } => Value::from([Atom::Const(const_bit(value, bit))]),
        DriveNode::Expr { .. } | DriveNode::Delay { .. } => {
            Value::from([Atom::Process(world.processes[&node].clone(), bit)])
        }
    }
}

/// The bits that are connected bidirectionally, by the bits of their closure.
type Closures = BTreeMap<(SignalKey, u32), Rc<Vec<(SignalKey, u32)>>>;

/// Computes the closures of the bidirectional edges, bit by bit.
fn bidir_closures(dr: &DriverResolution) -> Closures {
    fn find(
        parent: &mut BTreeMap<(SignalKey, u32), (SignalKey, u32)>,
        x: (SignalKey, u32),
    ) -> (SignalKey, u32) {
        let p = *parent.entry(x).or_insert(x);
        if p == x {
            return x;
        }
        let root = find(parent, p);
        parent.insert(x, root);
        root
    }

    let mut parent = BTreeMap::new();
    for e in dr.bidir_edges.iter() {
        let left = dr.bidir_nodes[e.left].key;
        let right = dr.bidir_nodes[e.right].key;
        for i in 0..e.bit_length.get() {
            let l = find(&mut parent, (left, e.left_offset + i));
            let r = find(&mut parent, (right, e.right_offset + i));
            parent.insert(l, r);
        }
    }

    let mut members = BTreeMap::<_, Vec<_>>::new();
    for bit in parent.keys().copied().collect::<Vec<_>>() {
        let root = find(&mut parent, bit);
        members.entry(root).or_default().push(bit);
    }
    let mut closures = Closures::new();
    for bits in members.into_values() {
        let bits = Rc::new(bits);
        for &bit in bits.iter() {
            closures.insert(bit, bits.clone());
        }
    }
    closures
}

/// Adds `atoms` to the bit at `location` and every bit connected to it bidirectionally.
///
/// The atoms are stored in the mode of the signal they drive, and then in that of every bit of its
/// closure: the members of a closure resolve their drivers in their own mode.
fn add(
    gl: &GlobalContext,
    values: &mut Values,
    closures: &Closures,
    location: (SignalKey, u32),
    atoms: &Value,
) -> bool {
    let mut changed = false;
    let single = [location];
    let bits = closures.get(&location).map_or(&single[..], |c| &c[..]);
    for &bit in bits {
        let dst = values.entry(bit).or_default();
        for atom in atoms {
            let atom = atom
                .clone()
                .stored_in(gl.signals[location.0].mode)
                .stored_in(gl.signals[bit.0].mode);
            changed |= dst.insert(atom);
        }
    }
    changed
}

/// Computes the atoms that reach every driven bit, as a least fixed point.
///
/// All bits of a bidirectional closure share one value, which resolves all of their drivers.
fn evaluate(
    dr: &DriverResolution,
    gl: &GlobalContext,
    world: &World,
    closures: &Closures,
) -> Values {
    let mut values = Values::new();
    for &signal in &world.writes {
        for bit in 0..gl.signals[signal].size.get() {
            // The value written is one of the signal it is written to.
            let atom = Atom::Write(gl.signals[signal].name.clone(), bit)
                .stored_in(gl.signals[signal].mode);
            let location = external_write_location(dr, signal, bit);
            add(gl, &mut values, closures, location, &Value::from([atom]));
        }
    }

    loop {
        let mut changed = false;
        for k in dr.nodes.table_key_iter() {
            let Some(drivee) = dr.nodes[k].drivee_signal() else {
                continue;
            };
            // The incoming edge of an aliased node is realized by the alias.
            if dr.probe_fuse.contains_key(&drivee) {
                continue;
            }

            let mut next = dr.nodes[k].fst_in();
            while let Some(e) = next {
                let edge = &dr.edges[e];
                next = edge.next_in;
                for j in 0..edge.bit_length.get() {
                    let value = source(dr, world, &values, edge.from, edge.from_offset + j);
                    let location = drive_location(dr, drivee, edge.to_offset + j);
                    changed |= add(gl, &mut values, closures, location, &value);
                }
            }
        }
        if !changed {
            return values;
        }
    }
}

fn events(value: &Value) -> Value {
    value.iter().filter(|a| a.is_event()).cloned().collect()
}

/// The atoms that update any bit of `signal`.
fn signal_events(gl: &GlobalContext, values: &Values, signal: SignalKey) -> Value {
    let mut result = Value::new();
    for bit in 0..gl.signals[signal].size.get() {
        if let Some(value) = values.get(&(signal, bit)) {
            result.extend(events(value));
        }
    }
    result
}

// -------------------------------------------------------------------------------------------------
// Checks
// -------------------------------------------------------------------------------------------------

fn check(
    gl: &GlobalContext,
    world: &World,
    before: &DriverResolution,
    after: &DriverResolution,
) -> Vec<String> {
    assert!(before.probe_fuse.is_empty() && before.write_fuse.is_empty());

    let mut errors = Vec::new();
    // The bidirectional edges are lowered to drive edges afterwards.
    let vb = evaluate(before, gl, world, &bidir_closures(before));
    let va = evaluate(after, gl, world, &Closures::new());
    let name = |s: SignalKey| gl.signals[s].name.as_str();

    // Probes: every bit probes the same atoms.
    let mut reach_before = BTreeMap::<Atom, BTreeSet<String>>::new();
    let mut reach_after = BTreeMap::<Atom, BTreeSet<String>>::new();
    for &s in &world.signals {
        for bit in 0..gl.signals[s].size.get() {
            let b = probe(before, &vb, s, bit);
            let a = probe(after, &va, s, bit);
            if b != a {
                errors.push(format!(
                    "probe {}[{bit}] (-> {:?}): {b:?} before, {a:?} after",
                    name(s),
                    probe_location(after, s, bit).map(|(t, o)| format!("{}[{o}]", name(t))),
                ));
            }
            for (value, reach) in [(b, &mut reach_before), (a, &mut reach_after)] {
                for atom in value.into_iter().filter(Atom::is_event) {
                    reach
                        .entry(atom)
                        .or_default()
                        .insert(format!("{}[{bit}]", name(s)));
                }
            }
        }
    }

    // Drives: every driven bit reaches the same bits.
    let atoms = reach_before
        .keys()
        .chain(reach_after.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for atom in atoms {
        let (b, a) = (reach_before.get(&atom), reach_after.get(&atom));
        if b != a {
            errors.push(format!(
                "drive {atom:?} reaches {:?} before, {:?} after",
                b.cloned().unwrap_or_default(),
                a.cloned().unwrap_or_default(),
            ));
        }
    }

    // Watches: a rewritten watch wakes on the same atoms.
    for &watch in &world.watches {
        let fused = match after.probe_fuse.get(&watch.signal) {
            None => Some(watch),
            Some(FuseTarget::Signal(to, slice)) => Some(watch.fused_into(*to, *slice)),
            Some(FuseTarget::Constant(_)) => None,
        };
        if let Some(fused) = fused
            && fused.offset + fused.bit_length().get() > gl.signals[fused.signal].size.get()
        {
            errors.push(format!(
                "watch {}[{}+:{}] is fused out of bounds into {}[{}+:{}]",
                name(watch.signal),
                watch.offset,
                watch.bit_length(),
                name(fused.signal),
                fused.offset,
                fused.bit_length(),
            ));
            continue;
        }
        for i in 0..watch.bit_length().get() {
            let b = events(
                &vb.get(&(watch.signal, watch.offset + i))
                    .cloned()
                    .unwrap_or_default(),
            );
            let a = fused.map_or_else(Value::new, |f| {
                events(
                    &va.get(&(f.signal, f.offset + i))
                        .cloned()
                        .unwrap_or_default(),
                )
            });
            if b != a {
                errors.push(format!(
                    "watch {}[{}]: wakes on {b:?} before, {a:?} after",
                    name(watch.signal),
                    watch.offset + i,
                ));
            }
        }
    }

    // Last update times: the rewritten signal is updated by the same atoms.
    for &s in &world.lupdt {
        let b = signal_events(gl, &vb, s);
        let a = match after.probe_fuse.get(&s) {
            None => signal_events(gl, &va, s),
            Some(FuseTarget::Signal(to, None)) => signal_events(gl, &va, *to),
            Some(FuseTarget::Signal(to, Some(slice))) => {
                errors.push(format!(
                    "lupdt {} is fused into the slice {}[{}:{}]",
                    name(s),
                    name(*to),
                    slice.msb(),
                    slice.lsb(),
                ));
                continue;
            }
            Some(FuseTarget::Constant(_)) => Value::new(),
        };
        if b != a {
            errors.push(format!(
                "lupdt {}: updated by {b:?} before, {a:?} after",
                name(s)
            ));
        }
    }

    // Probes and drives are redirected as they are, so only to a signal of the same mode.
    for (&s, target) in &after.probe_fuse {
        if let FuseTarget::Signal(to, _) = target
            && gl.signals[s].mode != gl.signals[*to].mode
        {
            errors.push(format!(
                "probes of {} are redirected to {} of another mode",
                name(s),
                name(*to)
            ));
        }
    }
    for (&s, &(to, _)) in &after.write_fuse {
        if gl.signals[s].mode != gl.signals[to].mode {
            errors.push(format!(
                "drives of {} are redirected to {} of another mode",
                name(s),
                name(to)
            ));
        }
    }

    check_reads(gl, before, after, &mut errors);
    check_realized(gl, before, after, &mut errors);
    errors
}

/// Read edges follow the aliases to the bits that are probed instead.
fn check_reads(
    gl: &GlobalContext,
    before: &DriverResolution,
    after: &DriverResolution,
    errors: &mut Vec<String>,
) {
    let name = |s: SignalKey| gl.signals[s].name.as_str();
    for r in before.read_edges.table_key_iter() {
        let (rb, ra) = (&before.read_edges[r], &after.read_edges[r]);
        let signal = before.nodes[rb.from].driver_signal().unwrap();
        for i in 0..rb.bit_length.get() {
            let expected = probe_location(after, signal, rb.from_offset + i);
            let bit = ra.from_offset + i;
            let actual = match &after.nodes[ra.from] {
                DriveNode::Net { key, .. } | DriveNode::Variable { key, .. } => {
                    if after.probe_fuse.contains_key(key) {
                        errors.push(format!(
                            "read of {}[{}] stays on aliased {}[{bit}]",
                            name(signal),
                            rb.from_offset + i,
                            name(*key),
                        ));
                        continue;
                    }
                    Ok((*key, bit))
                }
                DriveNode::Constant { value, .. } => Err(const_bit(value, bit)),
                _ => unreachable!("only nets, variables and constants are read"),
            };
            if expected != actual {
                let fmt = |l: Result<(SignalKey, u32), bool>| match l {
                    Ok((s, o)) => format!("{}[{o}]", name(s)),
                    Err(b) => format!("1'b{}", u8::from(b)),
                };
                errors.push(format!(
                    "read of {}[{}] is probed from {} but the read edge is on {}",
                    name(signal),
                    rb.from_offset + i,
                    fmt(expected),
                    fmt(actual),
                ));
            }
        }
    }
}

/// Every incoming edge of a node that is not aliased is realized: by its process driving the node
/// directly, as the wiring of an assignment or by lowering the node. A process that drives a node
/// directly is its only driver for those bits and does not read the node.
fn check_realized(
    gl: &GlobalContext,
    before: &DriverResolution,
    after: &DriverResolution,
    errors: &mut Vec<String>,
) {
    let name = |s: SignalKey| gl.signals[s].name.as_str();
    let mut ins = Vec::new();
    for k in after.nodes.table_key_iter() {
        let Some(drivee) = after.nodes[k].drivee_signal() else {
            continue;
        };
        after.collect_in(k, &mut ins);
        if after.probe_fuse.contains_key(&drivee) {
            if ins.len() != 1 {
                errors.push(format!(
                    "aliased {} has {} incoming edges",
                    name(drivee),
                    ins.len()
                ));
            }
            continue;
        }

        for &e in &ins {
            let direct = after.direct.contains(&e);
            if !direct && !after.is_expr_wire(e) && !after.lower.contains(&k) {
                errors.push(format!(
                    "an incoming edge of {} is not realized",
                    name(drivee)
                ));
            }
            if !direct {
                continue;
            }

            let p = after.edges[e].from;
            let slice = after.dst_slice(e);
            if ins
                .iter()
                .any(|&other| other != e && after.dst_slice(other).overlaps(slice))
            {
                errors.push(format!(
                    "a direct driver of {}[{}:{}] overlaps another driver",
                    name(drivee),
                    slice.msb(),
                    slice.lsb(),
                ));
            }

            // The signals that the process reads, through the aliases.
            let mut reads = BTreeSet::new();
            let mut probed = |signal: SignalKey, offset: u32, bit_length: VectorSize| {
                for i in 0..bit_length.get() {
                    if let Ok((s, _)) = probe_location(after, signal, offset + i) {
                        reads.insert(s);
                    }
                }
            };
            let mut next = before.nodes[p].fst_read_in();
            while let Some(r) = next {
                let read = &before.read_edges[r];
                let signal = before.nodes[read.from].driver_signal().unwrap();
                probed(signal, read.from_offset, read.bit_length);
                next = read.next_in;
            }
            // A delay probes its input, which is resolved by a process of its own unless aliased.
            if let DriveNode::Delay {
                input, bit_length, ..
            } = after.nodes[p]
            {
                probed(input, 0, bit_length);
            }
            let (target, _) = drive_location(after, drivee, slice.lsb());
            if reads.contains(&target) {
                errors.push(format!(
                    "a direct driver of {} reads {}",
                    name(drivee),
                    name(target)
                ));
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Test
// -------------------------------------------------------------------------------------------------

fn dot(dr: &DriverResolution, gl: &GlobalContext) -> String {
    let mut w = Vec::new();
    dr.write_dot(gl, &mut w).unwrap();
    String::from_utf8(w).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8192))]

    #[test]
    fn alias_preserves_observations(spec in graph_spec()) {
        let delay_value = DelayValue::RealNumber(1.0);
        let delay3 = Delay3::Value(AstId { node: &delay_value, loc: 0 });
        let expr = Expr::Concatenation(AstIdRange { node: &[], loc: 0 });
        let mut table = SymbolTable::<()>::default();
        let scope = table
            .insert_root(IdentTable::EMPTY_IDENT, TokenRange::default(), ())
            .unwrap();
        let ast = Ast {
            expr: AstId { node: &expr, loc: 0 },
            delay3: AstId { node: &delay3, loc: 0 },
            scope,
        };

        let (before, mut gl, world) = build(&spec, &ast);
        let mut after = before.clone();
        after
            .lower_bidirectional(&mut gl, &mut Diagnostics::default())
            .unwrap();
        after.alias(&gl, spec.max_rounds, spec.optimize);

        let errors = check(&gl, &world, &before, &after);
        prop_assert!(
            errors.is_empty(),
            "{}\n\nprobe fuse: {:?}\nwrite fuse: {:?}\n\nbefore:\n{}\nafter:\n{}",
            errors.join("\n"),
            after.probe_fuse.iter().map(|(s, t)| (&gl.signals[*s].name, t)).collect::<Vec<_>>(),
            after
                .write_fuse
                .iter()
                .map(|(s, (t, slice))| (&gl.signals[*s].name, &gl.signals[*t].name, slice))
                .collect::<Vec<_>>(),
            dot(&before, &gl),
            dot(&after, &gl),
        );
    }
}
