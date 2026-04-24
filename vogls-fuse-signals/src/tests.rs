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

macro_rules! graph {
    (
        $signals:expr,
        [
        $($i:literal: $signal:ident $([ $($prop:ident),* ])? ),* $(,)?
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

        let n = nodes.insert(Node {
            content: NodeContent::Signal($signal),
            flags,
            size: $signals[$signal].size,
            fanin: Vec::new(),
            fanout: Vec::new(),
        });
        assert!(signal_to_node.insert($signal, n).is_none());
        )*

        $(
        let driver = vogls_utils::TableKey::from_usize($f).unwrap();
        let drivee = vogls_utils::TableKey::from_usize($t).unwrap();

        #[allow(unused_mut, unused_assignments)]
        let mut driver_slice = SignalSlice::with_end(nodes[driver].size);
        $(
            let lsb = $f_lsb;
            #[allow(unused_mut)]
            let mut msb = $f_lsb;
            $(msb = $f_msb;)?
            driver_slice = SignalSlice::new(msb, lsb).unwrap();
        )?
        #[allow(unused_mut, unused_assignments)]
        let mut drivee_slice = SignalSlice::with_end(nodes[drivee].size);
        $(
            let lsb = $t_lsb;
            #[allow(unused_mut)]
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

    g1.nodes.key_value_iter().zip(g2.nodes.iter()).all(|((k, n1), n2)| {
        if n1.flags != n2.flags {
            return false;
        }
        if n1.content != n2.content {
            return false;
        }
        if n1.fanin.len() != n2.fanin.len() {
            dbg!(k);
            dbg!(&n1.fanin);
            dbg!(&n2.fanin);
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
    g.optimize_till_fixed_point(&mut marshalls, &mut probe_fuse, &mut drive_map);

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
    g.optimize_till_fixed_point(&mut marshalls, &mut probe_fuse, &mut drive_map);

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
    g.optimize_till_fixed_point(&mut marshalls, &mut probe_fuse, &mut drive_map);

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
    g.optimize_till_fixed_point(&mut marshalls, &mut probe_fuse, &mut drive_map);

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
    g.optimize_till_fixed_point(&mut marshalls, &mut probe_fuse, &mut drive_map);

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
    g.optimize_till_fixed_point(&mut marshalls, &mut probe_fuse, &mut drive_map);

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
