use std::fmt;

use rayon::{ThreadPool, ThreadPoolBuilder};
use vogls::design::Design;
use vogls::utils::{Table, VgHashMap};

use crate::array::{Array, LazyArray, LazyArrayKey, LazyValue, LazyValueKey, Value};
use crate::design::{LazyDesign, LazyDesignKey};
use crate::output::{LazyOutput, LazyOutputKey, Output};
use crate::plan::{LazyPlan, LazyPlanKey, Plan};
use crate::run::{LazyRun, LazyRunKey, Run};

#[derive(Debug)]
pub struct ComputeError {}
impl fmt::Display for ComputeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Error")
    }
}
impl std::error::Error for ComputeError {}
pub type ComputeResult<T> = std::result::Result<T, ComputeError>;

#[cfg(feature = "python")]
impl From<ComputeError> for pyo3::PyErr {
    fn from(_value: ComputeError) -> Self {
        pyo3::exceptions::PyValueError::new_err("failed to compute")
    }
}

pub trait GraphItem {
    type Key;
    fn get(graph: &ComputeGraph, key: Self::Key) -> &Self;
    fn from_key(key: Key) -> Option<Self::Key>;
}

macro_rules! impl_graph_key {
    ($(($key:ty, $table:ident, $as:ident, $value:ty, $key_variant:ident)),+ $(,)?) => {
        $(
            impl GraphItem for $value {
                type Key = $key;
                fn get(graph: &ComputeGraph, key: Self::Key) -> &Self {
                    &graph.$table[key]
                }
                fn from_key(key: Key) -> Option<Self::Key> {
                    match key {
                        Key::$key_variant(k) => Some(k),
                        _ => None,
                    }
                }
            }
        )+

        #[derive(Default)]
        pub struct ComputeGraph {
            $(pub $table: Table<$key, $value>,)+
        }

        #[derive(Default)]
        pub struct ComputeDependencies {
            $(pub $table: Vec<$key>,)+
        }

        #[derive(Default)]
        pub struct ComputeInputs {
            $(pub $table: VgHashMap<$key, $key_variant>,)+
        }

        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Key {
            $($key_variant($key),)+
        }

        #[derive(Default)]
        pub struct CommonSubPlan {
            $(pub(crate) $table: VgHashMap<$value, $key>,)+
        }

        impl ComputeDependencies {
            fn drain_keys(&mut self, mut f: impl FnMut(Key)) {
                let Self { $($table,)+ } = self;
                $($table.drain(..).map(Key::$key_variant).for_each(&mut f);)+
            }
        }

        impl Key {
            $(
            pub fn $as(self) -> $key {
                match self {
                    Key::$key_variant(k) => k,
                    _ => unreachable!(),
                }
            }
            )+

            fn has_key_been_computed(self, inputs: &ComputeInputs) -> bool {
                match self {
                    $(Key::$key_variant(k) => inputs.$table.contains_key(&k),)+
                }
            }
            fn extend_key_inputs(self, graph: &ComputeGraph, deps: &mut ComputeDependencies) {
                match self {
                    $(Key::$key_variant(k) => graph.$table[k].extend_inputs(deps),)+
                }
            }
            fn remove_from_inputs(self, inputs: &mut ComputeInputs) {
                match self {
                    $(Key::$key_variant(k) => _ = inputs.$table.remove(&k),)+
                }
            }
            fn compute(self, ctx: &ComputeContext, graph: &ComputeGraph, inputs: &mut ComputeInputs) -> ComputeResult<()> {
                match self {
                    $(Key::$key_variant(k) => _ = inputs.$table.insert(k, graph.$table[k].compute(ctx, &inputs)?),)+
                }
                Ok(())
            }
        }
    };
}

impl_graph_key! {
    (LazyDesignKey, designs, as_design, LazyDesign, Design),
    (LazyPlanKey, plans, as_plan, LazyPlan, Plan),
    (LazyOutputKey, outputs, as_output, LazyOutput, Output),
    (LazyArrayKey, arrays, as_array, LazyArray, Array),
    (LazyValueKey, values, as_value, LazyValue, Value),
    (LazyRunKey, runs, as_run, LazyRun, Run),
}

impl ComputeGraph {
    pub fn get<I: GraphItem>(&self, key: Key) -> &I {
        let k = I::from_key(key).unwrap();
        GraphItem::get(self, k)
    }
}

pub struct ComputeContext {
    pub pool: ThreadPool,
}

impl ComputeContext {
    pub fn new(num_threads: Option<usize>) -> Self {
        let pool = match num_threads {
            None => ThreadPoolBuilder::new()
                .use_current_thread()
                .num_threads(1)
                .build(),
            Some(0) => ThreadPoolBuilder::new().build(),
            Some(n) => ThreadPoolBuilder::new().num_threads(n).build(),
        }
        .expect("failed to build threadpool");
        Self { pool }
    }
}

pub trait ComputeNode {
    type Output;

    fn extend_inputs(&self, deps: &mut ComputeDependencies);
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output>;
}

pub fn compute<T: ComputeNode>(
    node: &T,
    node_key: Key,
    graph: &ComputeGraph,
    ctx: &ComputeContext,
) -> ComputeResult<T::Output> {
    enum Status {
        OnPath,
        Done,
    }
    struct StackItem {
        key: Key,
        dispatched: bool,
    }

    // Determine the fanin count and whether a computation graph has cycles.
    let mut has_cycle = false;
    let mut fanin_count = VgHashMap::<Key, u64>::default();
    let mut stack = Vec::<StackItem>::new();
    let mut statuses = VgHashMap::<Key, Status>::default();
    let mut deps = ComputeDependencies::default();
    stack.push(StackItem {
        key: node_key,
        dispatched: false,
    });
    while let Some(item) = stack.pop() {
        if !item.dispatched {
            statuses.insert(item.key, Status::OnPath);
            stack.push(StackItem {
                key: item.key,
                dispatched: true,
            });

            item.key.extend_key_inputs(graph, &mut deps);

            let start_length = stack.len();
            deps.drain_keys(|key| {
                *fanin_count.entry(key).or_insert_with(|| {
                    stack.push(StackItem {
                        key,
                        dispatched: false,
                    });
                    0
                }) += 1;

                if matches!(statuses.get(&key), Some(Status::OnPath)) {
                    has_cycle = true;
                }
            });

            if has_cycle {
                break;
            }
            if stack.len() != start_length {
                continue;
            }

            stack.pop();
        }

        statuses.insert(item.key, Status::Done);
    }

    if has_cycle {
        return Err(ComputeError {});
    }

    // Start computing inputs in the computation graph. For each node first compute the inputs,
    // then compute the node self. When a input is no longer needed by any node, drop it do reuse
    // memory consumption.
    let mut inputs = ComputeInputs::default();
    node_key.extend_key_inputs(graph, &mut deps);
    deps.drain_keys(|key| {
        if !key.has_key_been_computed(&inputs) {
            stack.push(StackItem {
                key,
                dispatched: false,
            });
        }
    });
    while let Some(item) = stack.pop() {
        if item.key.has_key_been_computed(&inputs) {
            continue;
        }

        if !item.dispatched {
            stack.push(StackItem {
                key: item.key,
                dispatched: true,
            });

            item.key.extend_key_inputs(graph, &mut deps);
            let start_length = stack.len();
            deps.drain_keys(|key| {
                *fanin_count.get_mut(&key).unwrap() -= 1;
                if !key.has_key_been_computed(&inputs) {
                    stack.push(StackItem {
                        key,
                        dispatched: false,
                    });
                }
            });

            // If the stack item has uncomputed inputs, we first need to go and compute those
            // inputs.
            if stack.len() != start_length {
                continue;
            }

            // All the inputs are ready, remove the `dispatched: true` stack item and just compute
            // this item directly.
            stack.pop();
        }

        item.key.compute(ctx, graph, &mut inputs)?;

        // Clean up inputs that are no longer needed.
        item.key.extend_key_inputs(graph, &mut deps);
        deps.drain_keys(|key| {
            if *fanin_count.get(&key).unwrap() == 0 {
                key.remove_from_inputs(&mut inputs);
            }
        });
    }

    node.compute(ctx, &inputs)
}
