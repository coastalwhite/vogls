use std::fmt;

use rayon::{ThreadPool, ThreadPoolBuilder};
use vogls::utils::{Table, VgHashMap, VgHashSet};

use crate::CspTable;
use crate::array::{Array, LazyArray, LazyArrayKey};
use crate::design::{LazyDesign, LazyDesignKey, PlanDesign, SignalRef};
use crate::output::{LazyOutput, LazyOutputKey, Output};
use crate::plan::{LazyPlan, LazyPlanKey, Plan};
use crate::run_vector::{LazyRunVector, LazyRunVectorKey, RunVector};
use crate::value::{LazyValue, LazyValueKey, Value};

#[derive(Debug)]
pub enum ComputeError {
    CyclicComputationGraph,

    Tokenization,
    Parsing,
    Elaboration,
    Lowering,
    Bytecode,
    Compile,

    UnknownSignal,
    UnknownComponent(String),

    FailedToRun,
    NumTracesMismatch,
    FailedToResolveNumTraces,

    InvalidTypes,
}
impl fmt::Display for ComputeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComputeError::CyclicComputationGraph => f.write_str("Computation graph has a cycle"),

            ComputeError::Tokenization => f.write_str("Tokenization"),
            ComputeError::Parsing => f.write_str("Parsing"),
            ComputeError::Elaboration => f.write_str("Elaboration"),
            ComputeError::Lowering => f.write_str("Lowering"),
            ComputeError::Bytecode => f.write_str("Bytecode"),
            ComputeError::Compile => f.write_str("Compile"),

            ComputeError::UnknownSignal => f.write_str("UnknownSignal"),
            ComputeError::UnknownComponent(name) => write!(f, "unable to find component: '{name}'"),

            ComputeError::FailedToRun => f.write_str("FailedToRun"),
            ComputeError::NumTracesMismatch => f.write_str("NumTracesMismatch"),
            ComputeError::FailedToResolveNumTraces => f.write_str("FailedToResolveNumTraces"),

            ComputeError::InvalidTypes => f.write_str("InvalidTypes"),
        }
    }
}
impl std::error::Error for ComputeError {}
pub type ComputeResult<T> = std::result::Result<T, ComputeError>;

#[cfg(feature = "python")]
impl From<ComputeError> for pyo3::PyErr {
    fn from(err: ComputeError) -> Self {
        pyo3::exceptions::PyValueError::new_err(format!("Failed computation. Reason: {err}"))
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
            $(pub(crate) $table: CspTable<$key>,)+
        }

        impl ComputeDependencies {
            fn drain_keys(&mut self, mut f: impl FnMut(Key)) {
                let Self { $($table,)+ } = self;
                $($table.drain(..).map(Key::$key_variant).for_each(&mut f);)+
            }
            fn try_drain_keys<E>(&mut self, mut f: impl FnMut(Key) -> Result<(), E>) -> Result<(), E> {
                let Self { $($table,)+ } = self;
                $($table.drain(..).map(Key::$key_variant).map(&mut f).collect::<Result<(), E>>()?;)+
                Ok(())
            }
        }

        impl ComputeGraph {
            fn prepare_key(&self, ctx: &ComputeContext, key: Key, pctx: &mut PreparationContext) -> ComputeResult<()> {
                match key {
                    $(Key::$key_variant(k) => self.$table[k].prepare(self, ctx, pctx),)*
                }
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

            fn fmt_node(self, f: &mut fmt::Formatter<'_>, graph: &ComputeGraph) -> fmt::Result {
                match self {
                    $(Key::$key_variant(k) => <_ as ComputeNode>::fmt(&graph.$table[k], f),)+
                }
            }
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
    (LazyDesignKey, designs, as_design, LazyDesign, PlanDesign),
    (LazyPlanKey, plans, as_plan, LazyPlan, Plan),
    (LazyOutputKey, outputs, as_output, LazyOutput, Output),
    (LazyArrayKey, arrays, as_array, LazyArray, Array),
    (LazyValueKey, values, as_value, LazyValue, Value),
    (LazyRunVectorKey, run_vectors, as_run_vector, LazyRunVector, RunVector),
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
            None => ThreadPoolBuilder::new().num_threads(1).build(),
            Some(0) => ThreadPoolBuilder::new().build(),
            Some(n) => ThreadPoolBuilder::new().num_threads(n).build(),
        }
        .expect("failed to build threadpool");
        Self { pool }
    }
}

#[derive(Default)]
pub struct PreparationContext {
    pub signals: VgHashMap<LazyDesignKey, VgHashSet<SignalRef>>,
}

impl PreparationContext {
    fn apply(self, ctx: &ComputeContext, graph: &mut ComputeGraph) -> ComputeResult<()> {
        _ = ctx;
        for (k, signals) in self.signals {
            graph.designs[k].handles.extend(signals);
        }
        Ok(())
    }
}

pub trait ComputeNode {
    type Key;
    type Output;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn extend_inputs(&self, deps: &mut ComputeDependencies);
    fn prepare(
        &self,
        graph: &ComputeGraph,
        ctx: &ComputeContext,
        pctx: &mut PreparationContext,
    ) -> ComputeResult<()> {
        _ = ctx;
        _ = graph;
        _ = pctx;
        Ok(())
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output>;
}

pub fn compute<T: GraphItem + ComputeNode>(
    node_key: Key,
    graph: &mut ComputeGraph,
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
    let mut pctx = PreparationContext::default();
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

        graph.prepare_key(ctx, item.key, &mut pctx)?;
        statuses.insert(item.key, Status::Done);
    }

    if has_cycle {
        return Err(ComputeError::CyclicComputationGraph);
    }

    pctx.apply(ctx, graph)?;

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

    graph.get::<T>(node_key).compute(ctx, &inputs)
}

fn key_to_ident(key: Key) -> impl fmt::Display {
    struct KeyDisplay(Key);
    impl fmt::Display for KeyDisplay {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use vogls::utils::TableKey as _;
            let (ident, key) = match self.0 {
                Key::PlanDesign(k) => ("d", k.get()),
                Key::Plan(k) => ("p", k.get()),
                Key::Output(k) => ("o", k.get()),
                Key::Array(k) => ("a", k.get()),
                Key::Value(k) => ("v", k.get()),
                Key::RunVector(k) => ("r", k.get()),
            };
            f.write_str(ident)?;
            key.fmt(f)
        }
    }
    KeyDisplay(key)
}

pub struct EscapeLabel<'a, W: fmt::Write>(&'a mut W);

pub struct ComputeNodeDisplay<'a>(Key, &'a ComputeGraph);

impl<'a> fmt::Display for ComputeNodeDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt_node(f, self.1)
    }
}

impl<'a, W: fmt::Write> fmt::Write for EscapeLabel<'a, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut prev = 0usize;
        for (i, c) in s.char_indices() {
            let escaped_char = match c {
                '"' => "\\\"",
                '\n' => "\\n",
                '\\' => "\\\\",
                _ => continue,
            };
            self.0.write_str(&s[prev..i])?;
            self.0.write_str(escaped_char)?;
            prev = i + c.len_utf8();
        }
        self.0.write_str(&s[prev..])?;
        Ok(())
    }
}

pub fn display_dot<'a>(roots: &'a [Key], graph: &'a ComputeGraph) -> impl fmt::Display + 'a {
    struct DisplayGraph<'a> {
        roots: &'a [Key],
        graph: &'a ComputeGraph,
    }

    static INDENT: &str = "  ";

    impl<'a> fmt::Display for DisplayGraph<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            use fmt::Write;

            let mut stack = Vec::new();
            let mut seen = VgHashSet::<Key>::default();
            let mut deps = ComputeDependencies::default();

            writeln!(f, "digraph vogls {{")?;
            writeln!(f, "{INDENT}rankdir=\"BT\"")?;
            writeln!(f, "{INDENT}node [fontname=\"Monospace\", shape=\"box\"]")?;

            stack.extend_from_slice(self.roots);
            seen.extend(self.roots);
            while let Some(key) = stack.pop() {
                f.write_str(INDENT)?;
                key_to_ident(key).fmt(f)?;
                f.write_str(" [label=\"")?;
                write!(EscapeLabel(f), "{}", ComputeNodeDisplay(key, self.graph))?;
                f.write_str("\"]")?;
                writeln!(f)?;

                key.extend_key_inputs(self.graph, &mut deps);
                deps.try_drain_keys(|from| {
                    writeln!(f, "{INDENT}{} -> {}", key_to_ident(from), key_to_ident(key))?;
                    if seen.insert(from) {
                        stack.push(from);
                    }
                    fmt::Result::Ok(())
                })?;
            }

            writeln!(f, "}}")?;

            Ok(())
        }
    }

    DisplayGraph { roots, graph }
}
