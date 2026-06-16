use std::ffi::c_void;
use std::{fmt, ptr};

use hashbrown::hash_map::Entry;
use vogls::utils::VgHashMap;

use crate::compute::{CommonSubPlan, ComputeError, ComputeGraph, ComputeResult, Key};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DslPtr(*const c_void);

impl<'a> From<&'a dyn DslNode> for DslPtr {
    fn from(value: &'a dyn DslNode) -> Self {
        Self(ptr::from_ref(value).cast())
    }
}

pub trait DslNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}

pub fn convert(root: &dyn DslNode) -> ComputeResult<(Key, ComputeGraph)> {
    enum Status {
        Unvisited,
        OnPath,
        Done,
    }
    struct StackItem<'a> {
        node: &'a dyn DslNode,
        dispatched: bool,
    }

    // Determine the fanin count and whether a computation graph has cycles.
    let mut stack = Vec::<StackItem>::new();
    let mut statuses = VgHashMap::<DslPtr, Status>::default();
    let mut converted = VgHashMap::<DslPtr, Key>::default();
    let mut inputs = Vec::new();
    let mut graph = ComputeGraph::default();
    let mut csp = CommonSubPlan::default();
    stack.push(StackItem {
        node: root,
        dispatched: false,
    });
    while let Some(item) = stack.pop() {
        if !item.dispatched {
            statuses.insert(item.node.into(), Status::OnPath);
            stack.push(StackItem {
                node: item.node,
                dispatched: true,
            });

            item.node.extend_inputs(&mut inputs);

            let start_length = stack.len();
            for input in inputs.drain(..) {
                match statuses.entry(input.into()) {
                    Entry::Vacant(entry) => {
                        entry.insert(Status::Unvisited);
                        stack.push(StackItem {
                            node: input,
                            dispatched: false,
                        });
                    }
                    Entry::Occupied(entry) if matches!(entry.get(), Status::OnPath) => {
                        return Err(ComputeError::CyclicComputationGraph);
                    }
                    Entry::Occupied(_) => {}
                }
            }

            if stack.len() != start_length {
                continue;
            }

            stack.pop();
        }

        let key = item.node.convert_one(&mut graph, &converted, &mut csp);
        converted.insert(item.node.into(), key);
        statuses.insert(item.node.into(), Status::Done);
    }

    Ok((converted[&DslPtr::from(root)], graph))
}
