use vogls_utils::TableKey;

use super::*;

macro_rules! signals {
    (
        $($name:literal : $size:literal),* $(,)?
    ) => {{
        let mut signals = ::slotmap::SlotMap::<SignalKey, Signal>::default();
        ([
            $(
            signals.insert(Signal {
                name: $name.into(),
                size: ::vogls_ir::VectorSize::new($size).unwrap(),
                initialize: None,
                origin: ::vogls_ir::token_range::TokenRange::default(),
            })
            ),+
        ], signals)
    }}
}

macro_rules! flag {
    (D) => {{ NodeFlags::DRIVE }};
    (P) => {{ NodeFlags::PROBE }};
    (L) => {{ NodeFlags::LUPDT }};
    (W) => {{ NodeFlags::WATCH }};
}

macro_rules! node {
    ($signals:expr, $signal_to_node:ident, $nodes:expr, $signal:ident, $flags:expr) => {{
        let n = $nodes.insert(Node {
            content: NodeContent::Signal($signal),
            flags: $flags,
            size: $signals[$signal].size,
            fanin: Vec::new(),
            fanout: Vec::new(),
        });
        assert!($signal_to_node.insert($signal, n).is_none());
        n
    }};
    ($signals:expr, $signal_to_node:ident, $nodes:expr, ($bits:expr), $flags:expr) => {{
        let mut flags = $flags;
        flags |= NodeFlags::DRIVE;
        let n = $nodes.insert(Node {
            content: NodeContent::Constant($bits),
            flags,
            size: $bits.size(),
            fanin: Vec::new(),
            fanout: Vec::new(),
        });
        n
    }};
}

macro_rules! graph {
    (
        $signals:expr,
        [
        $($i:literal: $signal:tt $([ $($prop:ident)* ])? ),* $(,)?
        ]
        [
        $($f:literal $( [$(:$f_msb:literal :)?$f_lsb:literal] )? -> $t:literal $( [$(:$t_msb:literal : )?$t_lsb:literal] )?),* $(,)?
        ]
    ) => {{
        #[allow(unused_mut)]
        let mut nodes = Table::new();
        #[allow(unused_mut)]
        let mut edges = Table::new();
        #[allow(unused_mut)]
        let mut signal_to_node = VgHashMap::<SignalKey, NodeKey>::default();

        $(
        #[allow(unused_mut)]
        let mut flags = NodeFlags::EMPTY;
        $($(
        flags |= flag!($prop);
        )?)*

        node!($signals, signal_to_node, nodes, $signal, flags);
        )*

        $(
        let driver = vogls_utils::TableKey::from_usize($f).unwrap();
        let drivee = vogls_utils::TableKey::from_usize($t).unwrap();

        #[allow(unused_mut, unused_assignments)]
        let mut driver_slice = SignalSlice::with_end(nodes[driver].size);
        $(
            let lsb = $f_lsb;
            #[allow(unused_mut, unused_assignments)]
            let mut msb = $f_lsb;
            $(msb = $f_msb;)?
            driver_slice = SignalSlice::new(msb, lsb).unwrap();
        )?
        #[allow(unused_mut, unused_assignments)]
        let mut drivee_slice = SignalSlice::with_end(nodes[drivee].size);
        $(
            let lsb = $t_lsb;
            #[allow(unused_mut, unused_assignments)]
            let mut msb = $t_lsb;
            $(msb = $t_msb;)?
            drivee_slice = SignalSlice::new(msb, lsb).unwrap();
        )?

        assert_eq!(driver_slice.width(), drivee_slice.width());
        let e = edges.insert(Edge {
            driver,
            drivee,
            driver_slice,
            drivee_slice,
        });
        nodes[driver].fanout.push(e);
        nodes[drivee].fanin.push(e);
        )*



        FuseGraph {
            nodes,
            edges,
            signal_to_node,
        }
    }};
}

fn is_equal(g1: &FuseGraph, g2: &FuseGraph) -> bool {
    if g1.nodes.len() != g2.nodes.len() {
        return false;
    }

    let mut n1_scratch = Vec::<Edge>::new();
    let mut n2_scratch = Vec::<Edge>::new();

    g1.nodes.iter().zip(g2.nodes.iter()).all(|(n1, n2)| {
        if n1.flags != n2.flags {
            return false;
        }
        if n1.content != n2.content {
            return false;
        }
        if n1.fanin.len() != n2.fanin.len() {
            return false;
        }
        if n1.fanout.len() != n2.fanout.len() {
            return false;
        }

        let mut is_equal = true;

        n1_scratch.clear();
        n2_scratch.clear();
        n1_scratch.extend(n1.fanin.iter().map(|&e| g1.edges[e].clone()));
        n2_scratch.extend(n2.fanin.iter().map(|&e| g2.edges[e].clone()));
        n1_scratch.sort_unstable_by_key(|e| {
            (
                e.driver,
                e.driver,
                e.driver_slice.msb(),
                e.driver_slice.lsb(),
                e.drivee_slice.msb(),
                e.drivee_slice.lsb(),
            )
        });
        n2_scratch.sort_unstable_by_key(|e| {
            (
                e.driver,
                e.driver,
                e.driver_slice.msb(),
                e.driver_slice.lsb(),
                e.drivee_slice.msb(),
                e.drivee_slice.lsb(),
            )
        });
        for (e1, e2) in n1_scratch.iter().zip(n2_scratch.iter()) {
            is_equal &= e1 == e2;
        }

        n1_scratch.clear();
        n2_scratch.clear();
        n1_scratch.extend(n1.fanout.iter().map(|&e| g1.edges[e].clone()));
        n2_scratch.extend(n2.fanout.iter().map(|&e| g2.edges[e].clone()));
        n1_scratch.sort_unstable_by_key(|e| {
            (
                e.driver,
                e.driver,
                e.driver_slice.msb(),
                e.driver_slice.lsb(),
                e.drivee_slice.msb(),
                e.drivee_slice.lsb(),
            )
        });
        n2_scratch.sort_unstable_by_key(|e| {
            (
                e.driver,
                e.driver,
                e.driver_slice.msb(),
                e.driver_slice.lsb(),
                e.drivee_slice.msb(),
                e.drivee_slice.lsb(),
            )
        });
        for (e1, e2) in n1_scratch.iter().zip(n2_scratch.iter()) {
            is_equal &= e1 == e2;
        }
        is_equal
    })
}

#[track_caller]
fn assert_graph_equal(signals: &SlotMap<SignalKey, Signal>, g1: &FuseGraph, g2: &FuseGraph) {
    if is_equal(g1, g2) {
        return;
    }

    panic!(
        "graphs not equal.\nleft:\n{}\n\nright:\n{}",
        g1.display_dot(&signals),
        g2.display_dot(&signals)
    );
}

#[test]
fn test_transitive_property() {
    let ([s_a, s_b, s_c], signals) = signals!(
        "A" : 1,
        "B" : 1,
        "C" : 1,
    );
    let mut g = graph! {
        signals,
        [ 0: s_a, 1: s_b, 2: s_c ]
        [ 0 -> 1, 1 -> 2 ]
    };
    let out = graph! {
        signals,
        [ 0: s_a, 1: s_b, 2: s_c ]
        [ 0 -> 1, 0 -> 2 ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert!(marshalls.is_empty());
    assert_eq!(
        probe_fuse,
        <VgHashMap<_, _>>::from_iter([
            (s_b, FuseTarget::Signal(s_a, None)),
            (s_c, FuseTarget::Signal(s_a, None))
        ])
    );
    assert!(drive_map.is_empty());
}

#[test]
fn test_transitive_slices_property() {
    let ([s_a, s_b, s_c], signals) = signals!(
        "A" : 4,
        "B" : 2,
        "C" : 1,
    );
    let mut g = graph! {
        signals,
        [ 0: s_a, 1: s_b, 2: s_c ]
        [ 0 [: 3 : 2]-> 1, 1 [1] -> 2 ]
    };
    let out = graph! {
        signals,
        [ 0: s_a, 1: s_b, 2: s_c ]
        [ 0 [: 3 : 2] -> 1, 0 [3] -> 2 ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert!(marshalls.is_empty());
    assert_eq!(
        probe_fuse,
        <VgHashMap<_, _>>::from_iter([
            (
                s_b,
                FuseTarget::Signal(s_a, Some(SignalSlice::new(3, 2).unwrap()))
            ),
            (
                s_c,
                FuseTarget::Signal(s_a, Some(SignalSlice::new(3, 3).unwrap()))
            )
        ])
    );
    assert!(drive_map.is_empty());
}

#[test]
fn test_cyclic_property() {
    let ([s_a], signals) = signals!("A" : 1);
    let mut g = graph! {
        signals,
        [ 0: s_a ]
        [ 0 -> 0 ]
    };
    let out = graph! {
        signals,
        [ 0: s_a ]
        []
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert!(marshalls.is_empty());
    assert!(probe_fuse.is_empty());
    assert!(drive_map.is_empty());
}

#[test]
fn test_neighbour_merge_property() {
    let ([s_a, s_b], signals) = signals!("A": 2, "B": 2);
    let mut g = graph! {
        signals,
        [ 0: s_a, 1: s_b ]
        [
            0 [0] -> 1 [0],
            0 [1] -> 1 [1],
        ]
    };
    let out = graph! {
        signals,
        [ 0: s_a, 1: s_b ]
        [ 0 -> 1 ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert!(marshalls.is_empty());
    assert_eq!(
        probe_fuse,
        <VgHashMap<_, _>>::from_iter([(s_b, FuseTarget::Signal(s_a, None)),])
    );
    assert!(drive_map.is_empty());
}

#[test]
fn test_inversion_property_neg_not_whole_wire() {
    let ([s_a, s_b], signals) = signals!("A": 2, "B": 2);
    let mut g = graph! {
        signals,
        [ 0: s_a [ D ], 1: s_b ]
        [ 0 [0] -> 1 [0] ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_eq!(marshalls, vec![g.signal_to_node[&s_b]]);
    assert!(probe_fuse.is_empty());
    assert!(drive_map.is_empty());
}

#[test]
fn test_inversion_property() {
    let ([s_a, s_b], signals) = signals!("A": 1, "B": 2);
    let mut g = graph! {
        signals,
        [ 0: s_a [ D ], 1: s_b ]
        [ 0 -> 1 [0] ]
    };
    let out = graph! {
        signals,
        [ 0: s_a, 1: s_b [ D ] ]
        [ 1 [0] -> 0 ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert!(marshalls.is_empty());
    assert_eq!(
        probe_fuse,
        <VgHashMap<_, _>>::from_iter([(
            s_a,
            FuseTarget::Signal(s_b, Some(SignalSlice::new(0, 0).unwrap()))
        ),])
    );
    assert_eq!(
        drive_map,
        <VgHashMap<_, _>>::from_iter([(s_a, (s_b, Some(SignalSlice::new(0, 0).unwrap()))),])
    );

    let ([s_a, s_b, s_c], signals) = signals!("A": 1, "B": 1, "C": 2);
    let mut g = graph! {
        signals,
        [ 0: s_a [D], 1: s_b [D], 2: s_c ]
        [ 0 -> 2 [0], 1 -> 2 [1] ]
    };
    let out = graph! {
        signals,
        [ 0: s_a, 1: s_b, 2: s_c [D] ]
        [ 2 [0] -> 0, 2 [1] -> 1 ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert!(marshalls.is_empty());
    assert_eq!(
        probe_fuse,
        <VgHashMap<_, _>>::from_iter([
            (
                s_a,
                FuseTarget::Signal(s_c, Some(SignalSlice::new(0, 0).unwrap()))
            ),
            (
                s_b,
                FuseTarget::Signal(s_c, Some(SignalSlice::new(1, 1).unwrap()))
            ),
        ])
    );
    assert_eq!(
        drive_map,
        <VgHashMap<_, _>>::from_iter([
            (s_a, (s_c, Some(SignalSlice::new(0, 0).unwrap()))),
            (s_b, (s_c, Some(SignalSlice::new(1, 1).unwrap()))),
        ])
    );
}

#[test]
fn test_merge_constants() {
    let ([sa], signals) = signals!("A" : 4);
    let c0 = Bits::from(false);
    let c1 = Bits::from(true);
    let mut g = graph! {
        signals,
        [ 0: (c0.clone()), 1: (c1.clone()), 2: (c0.clone()), 3: (c1.clone()), 4: sa ]
        [
            0 -> 4 [0],
            1 -> 4 [1],
            2 -> 4 [2],
            3 -> 4 [3],
        ]
    };
    let c0 = Bits::from(false);
    let c10 = Bits::from_u64(VectorSize::new(2).unwrap(), 0b10);
    let c010 = Bits::from_u64(VectorSize::new(3).unwrap(), 0b010);
    let c1010 = Bits::from_u64(VectorSize::new(4).unwrap(), 0b1010);
    let out = graph! {
        signals,
        [ 0: (c0.clone()), 1: (c10.clone()), 2: (c010.clone()), 3: (c1010.clone()), 4: sa ]
        [ 3 -> 4 ]
    };

    eprintln!("{}", g.display_dot(&signals));

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert!(marshalls.is_empty());
    assert_eq!(
        probe_fuse,
        <VgHashMap<_, _>>::from_iter([(sa, FuseTarget::Constant(c1010)),])
    );
    assert!(drive_map.is_empty());
}

#[test]
fn test_transitive_subslice() {
    let ([sa, sb, sc], signals) = signals!(
        "A" : 4,
        "B" : 4,
        "C" : 4,
    );
    let mut g = graph! {
        signals,
        [ 0: sa [D], 1: sb [P], 2: sc [P] ]
        [
            0 [2] -> 1 [1],
            1 [1] -> 2 [3],
        ]
    };
    let out = graph! {
        signals,
        [ 0: sa [D], 1: sb [P], 2: sc [P] ]
        [
            0 [2] -> 1 [1],
            0 [2] -> 2 [3],
        ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert_eq!(
        marshalls.as_slice(),
        &[
            NodeKey::from_usize(1).unwrap(),
            NodeKey::from_usize(2).unwrap()
        ],
    );
    assert!(probe_fuse.is_empty());
    assert!(drive_map.is_empty());
}

#[test]
fn test_transitive_subslice2() {
    let ([sa, sb, sc, sd, se, sf], signals) = signals!(
        "A" : 1,
        "B" : 1,
        "C" : 1,
        "D" : 1,
        "E" : 4,
        "F" : 4,
    );
    let mut g = graph! {
        signals,
        [ 0: sa [P], 1: sb [P], 2: sc [P], 3: sd [P], 4: se [P], 5: sf [D] ]
        [
            4 [0] -> 0,
            4 [1] -> 1,
            4 [2] -> 2,
            4 [3] -> 3,
            5 -> 4,
        ]
    };
    let out = graph! {
        signals,
        [ 0: sa [P], 1: sb [P], 2: sc [P], 3: sd [P], 4: se [P], 5: sf [D] ]
        [
            5 [0] -> 0,
            5 [1] -> 1,
            5 [2] -> 2,
            5 [3] -> 3,
            5 -> 4,
        ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::TRANSITIVE,
    );

    assert_graph_equal(&signals, &g, &out);
    assert!(marshalls.is_empty());
    assert_eq!(
        probe_fuse,
        <VgHashMap<_, _>>::from_iter([
            (
                sa,
                FuseTarget::Signal(sf, Some(SignalSlice::new(0, 0).unwrap()))
            ),
            (
                sb,
                FuseTarget::Signal(sf, Some(SignalSlice::new(1, 1).unwrap()))
            ),
            (
                sc,
                FuseTarget::Signal(sf, Some(SignalSlice::new(2, 2).unwrap()))
            ),
            (
                sd,
                FuseTarget::Signal(sf, Some(SignalSlice::new(3, 3).unwrap()))
            ),
            (se, FuseTarget::Signal(sf, None)),
        ])
    );
    assert!(drive_map.is_empty());
}

#[test]
fn test_integration1() {
    let ([s1, s2, s3, s4, w1, w2], signals) = signals!(
        "A" : 1,
        "B" : 1,
        "C" : 1,
        "D" : 1,
        "X" : 4,
        "Z" : 4,
    );
    let c0 = Bits::from(false);
    let c1 = Bits::from(true);
    let mut g = graph! {
        signals,
        [ 0: (c0.clone()), 1: (c1.clone()), 2: (c0.clone()), 3: (c1.clone()), 4: s1, 5: s2, 6: s3, 7: s4, 8: w1, 9: w2 ]
        [
            0 -> 4,
            1 -> 5,
            2 -> 6,
            3 -> 7,
            4 -> 8 [0],
            5 -> 8 [1],
            6 -> 8 [2],
            7 -> 8 [3],
            8 -> 9,
        ]
    };
    let c0 = Bits::from(false);
    let c10 = Bits::from_u64(VectorSize::new(2).unwrap(), 0b10);
    let c010 = Bits::from_u64(VectorSize::new(3).unwrap(), 0b010);
    let c1010 = Bits::from_u64(VectorSize::new(4).unwrap(), 0b1010);
    let out = graph! {
        signals,
        [ 0: (c0.clone()), 1: (c10.clone()), 2: (c010.clone()), 3: (c1010.clone()), 4: s1, 5: s2, 6: s3, 7: s4, 8: w1, 9: w2 ]
        [
            0 -> 4,
            1 [1] -> 5,
            2 [2] -> 6,
            3 [3] -> 7,
            3 -> 8,
            3 -> 9,
        ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert!(marshalls.is_empty());
    assert_eq!(
        probe_fuse,
        <VgHashMap<_, _>>::from_iter([
            (s1, FuseTarget::Constant(c0.clone())),
            (s2, FuseTarget::Constant(c1.clone())),
            (s3, FuseTarget::Constant(c0.clone())),
            (s4, FuseTarget::Constant(c1.clone())),
            (w1, FuseTarget::Constant(c1010.clone())),
            (w2, FuseTarget::Constant(c1010.clone())),
        ])
    );
    assert!(drive_map.is_empty());
}

#[test]
fn test_transitive_subslice3() {
    let ([sa, sb, sc, sd], signals) = signals!(
        "A" : 2,
        "B" : 1,
        "C" : 1,
        "D" : 1,
    );
    let mut g = graph! {
        signals,
        [ 0: sa [D], 1: sb, 2: sc, 3: sd [ P W ] ]
        [
            0 [0] -> 1,
            0 [1] -> 2,
            1 -> 3,
        ]
    };
    let out = graph! {
        signals,
        [ 0: sa [D], 1: sb, 2: sc, 3: sd [ P W ] ]
        [
            0 [0] -> 1,
            0 [1] -> 2,
            0 [0] -> 3,
        ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert_eq!(marshalls.as_slice(), &[TableKey::from_usize(3).unwrap()]);
    assert_eq!(
        probe_fuse,
        <VgHashMap<_, _>>::from_iter([
            (
                sb,
                FuseTarget::Signal(sa, Some(SignalSlice::new(0, 0).unwrap()))
            ),
            (
                sc,
                FuseTarget::Signal(sa, Some(SignalSlice::new(1, 1).unwrap()))
            )
        ])
    );
    assert!(drive_map.is_empty());
}

#[test]
fn test_negative2() {
    let ([sa, sb, sc], signals) = signals!(
        "A" : 1,
        "B" : 1,
        "C" : 2,
    );
    let mut g = graph! {
        signals,
        [ 0: sa [D L], 1: sb [D L], 2: sc ]
        [ 0 -> 2 [0], 1 -> 2 [1] ]
    };
    let out = graph! {
        signals,
        [ 0: sa [L], 1: sb [L], 2: sc [D] ]
        [ 2 [0] -> 0, 2 [1] -> 1 ]
    };

    let mut marshalls = Vec::new();
    let mut probe_fuse = VgHashMap::default();
    let mut drive_map = VgHashMap::default();
    g.optimize_till_fixed_point(
        &mut marshalls,
        &mut probe_fuse,
        &mut drive_map,
        FusePasses::ALL,
    );

    assert_graph_equal(&signals, &g, &out);
    assert_eq!(
        marshalls.as_slice(),
        &[
            TableKey::from_usize(0).unwrap(),
            TableKey::from_usize(1).unwrap()
        ]
    );
    assert!(probe_fuse.is_empty());
    assert_eq!(
        drive_map,
        <VgHashMap<_, _>>::from_iter([
            (sa, (sc, Some(SignalSlice::new(0, 0).unwrap()))),
            (sb, (sc, Some(SignalSlice::new(1, 1).unwrap())))
        ])
    );
}
