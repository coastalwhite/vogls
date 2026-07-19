#![allow(unused)]

use std::fmt::{self, Write};

use slotmap::SlotMap;
use vogls_bits::format::BitsFormatOptions;
use vogls_ir::{Signal, SignalKey};
use vogls_utils::{TableKey, VgHashSet};

use crate::{Edge, EdgeKey, FuseGraph, Node, NodeContent, NodeFlags, NodeKey};

struct NodeDisplay<'a>(&'a Node, NodeKey, &'a SlotMap<SignalKey, Signal>);
impl<'a> fmt::Display for NodeDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0.content {
            NodeContent::Signal(s) if *s == SignalKey::default() => write!(f, "N{}", self.1.get()),
            NodeContent::Signal(s) => f.write_str(&self.2[*s].name),
            NodeContent::Constant(bits) => bits
                .display(&BitsFormatOptions {
                    prefix: true,
                    ..Default::default()
                })
                .fmt(f),
        }?;

        if self.0.flags != NodeFlags::EMPTY {
            f.write_char('[')?;
            if self.0.flags.contains(NodeFlags::DRIVE) {
                f.write_char('D')?;
            }
            if self.0.flags.contains(NodeFlags::PROBE) {
                f.write_char('P')?;
            }
            if self.0.flags.contains(NodeFlags::LUPDT) {
                f.write_char('L')?;
            }
            if self.0.flags.contains(NodeFlags::WATCH) {
                f.write_char('W')?;
            }
            f.write_char(']')?;
        }

        Ok(())
    }
}

impl Node {
    fn display<'a>(
        &'a self,
        key: NodeKey,
        signals: &'a SlotMap<SignalKey, Signal>,
    ) -> NodeDisplay<'a> {
        NodeDisplay(self, key, signals)
    }
}

pub struct FuseGraphDot<'a> {
    graph: &'a FuseGraph,
    signals: &'a SlotMap<SignalKey, Signal>,
}

pub struct FuseGraphSubgraphDot<'a> {
    graph: &'a FuseGraph,
    signals: &'a SlotMap<SignalKey, Signal>,
    at: NodeKey,
}

impl<'a> fmt::Display for FuseGraphDot<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use vogls_utils::TableKey;

        let mut seen_edges = VgHashSet::default();

        writeln!(f, "digraph {{")?;
        for (key, node) in self.graph.nodes.key_value_iter() {
            writeln!(
                f,
                r#"  n{} [label="{}"];"#,
                key.get(),
                node.display(key, self.signals)
            )?;

            for e in node.fanin.iter().chain(node.fanout.iter()) {
                let edge = &self.graph.edges[*e];
                if seen_edges.insert(*e) {
                    self.graph.fmt_edge(f, edge)?;
                }
            }
        }
        writeln!(f, "}}")?;

        Ok(())
    }
}

impl<'a> fmt::Display for FuseGraphSubgraphDot<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use vogls_utils::TableKey;

        let mut seen = VgHashSet::<NodeKey>::default();
        let mut seen_edges = VgHashSet::<EdgeKey>::default();
        let mut stack = Vec::new();

        stack.push(self.at);
        seen.insert(self.at);

        while let Some(n) = stack.pop() {
            write!(
                f,
                r#"  n{} [label="{}"]"#,
                n.get(),
                self.graph.nodes[n].display(n, self.signals)
            )?;

            for e in self.graph.nodes[n]
                .fanin
                .iter()
                .chain(self.graph.nodes[n].fanout.iter())
            {
                let edge = &self.graph.edges[*e];
                if seen_edges.insert(*e) {
                    self.graph.fmt_edge(f, edge)?;
                }
                if seen.insert(edge.driver) {
                    stack.push(edge.driver);
                }
                if seen.insert(edge.drivee) {
                    stack.push(edge.drivee);
                }
            }
        }

        Ok(())
    }
}

impl FuseGraph {
    pub fn display_dot<'a>(&'a self, signals: &'a SlotMap<SignalKey, Signal>) -> FuseGraphDot<'a> {
        FuseGraphDot {
            graph: self,
            signals,
        }
    }
    pub(crate) fn display_subgraph_dot<'a>(
        &'a self,
        node: NodeKey,
        signals: &'a SlotMap<SignalKey, Signal>,
    ) -> FuseGraphSubgraphDot<'a> {
        FuseGraphSubgraphDot {
            graph: self,
            signals,
            at: node,
        }
    }

    #[allow(unused)]
    fn fmt_edge(&self, f: &mut fmt::Formatter<'_>, edge: &Edge) -> fmt::Result {
        use std::fmt::Write;
        use vogls_utils::TableKey;

        writeln!(
            f,
            r#"  n{} -> n{} [taillabel="{}", headlabel="{}"];"#,
            edge.driver.get(),
            edge.drivee.get(),
            if edge.driver_slice.lsb() > 0
                || edge.driver_slice.width() != self.nodes[edge.driver].size
            {
                format!("[{}:{}]", edge.driver_slice.msb(), edge.driver_slice.lsb())
            } else {
                String::new()
            },
            if edge.drivee_slice.lsb() > 0
                || edge.drivee_slice.width() != self.nodes[edge.drivee].size
            {
                format!("[{}:{}]", edge.drivee_slice.msb(), edge.drivee_slice.lsb())
            } else {
                String::new()
            },
        )
    }
}

#[allow(unused)]
pub fn open_dot(s: &str) -> std::io::Result<()> {
    use std::io::Write;
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
