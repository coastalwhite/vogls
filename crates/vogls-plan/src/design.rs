use std::fmt;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use vogls::design::{Arena, Design, Macro};
use vogls::utils::{IndexMap, VgHashMap, VgHashSet, new_table_key};
use vogls::{LogicMode, OptFlags, Optimizations, SignalHandle, VoglsPlugin};
use vogls_trace::TracePlugin;

use crate::CspAble;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::run::DslLazyRun;

new_table_key! { pub struct LazyDesignKey; }

#[derive(Clone)]
pub struct LazyDesign {
    pub sources: Vec<PathBuf>,
    pub top_level_module: Option<String>,
    pub defines: Vec<String>,
    pub trace: bool,
    pub handles: VgHashSet<SignalRef>,
}

impl LazyDesign {
    pub fn run(self: Arc<Self>) -> DslLazyRun {
        DslLazyRun {
            design: self,
            steps: Vec::new(),
            ty: IndexMap::new(),
        }
    }
}

pub struct PlanDesign {
    pub design: Design,
    pub handles: VgHashMap<SignalRef, SignalHandle>,
}

impl CspAble for LazyDesign {
    fn csp_eq(&self, other: &Self) -> bool {
        let Self {
            sources: l_sources,
            top_level_module: l_tlm,
            defines: l_defines,
            trace: _,
            handles: _,
        } = self;
        let Self {
            sources: r_sources,
            top_level_module: r_tlm,
            defines: r_defines,
            trace: _,
            handles: _,
        } = other;
        l_sources == r_sources && l_defines == r_defines && l_tlm == r_tlm
    }
    fn csp_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sources.hash(state);
        self.defines.hash(state);
        self.top_level_module.hash(state);
    }
    fn csp_merge(&mut self, other: Self) {
        self.trace |= other.trace;
        self.handles.extend(other.handles);
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SignalRef {
    pub inner: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Time {
    pub value: u64,
    pub unit: TimeUnit,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum TimeUnit {
    Femptoseconds,
    Picoseconds,
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
}

impl ComputeNode for LazyDesign {
    type Key = LazyDesignKey;
    type Output = PlanDesign;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Design {{ sources: {:?}, num_handles: {} }}",
            &self.sources,
            self.handles.len()
        )
    }

    fn extend_inputs(&self, _deps: &mut ComputeDependencies) {}
    fn compute(
        &self,
        _ctx: &ComputeContext,
        _inputs: &ComputeInputs,
    ) -> ComputeResult<Self::Output> {
        let mut builder = vogls::DesignBuilder::new();
        let mut arena = Arena::default();
        for define in &self.defines {
            builder.define_macro(define, Macro::default());
        }
        for path in &self.sources {
            builder
                .add_source(path)
                .map_err(|_| ComputeError::Tokenization)?;
        }
        let parsed = builder.parse(&arena).map_err(|err| {
            println!("{err}");
            ComputeError::Parsing
        })?;
        let mut design = parsed
            .elaborate(LogicMode::TwoValue, self.top_level_module.as_deref())
            .map_err(|_| ComputeError::Elaboration)?;

        let handles = self
            .handles
            .iter()
            .map(|s| {
                let stable = design.table();
                let mut symbol = stable.roots()[0];
                for i in &s.inner {
                    let Some(ident) = design.ident_table().get(i) else {
                        return Err(ComputeError::UnknownSignal);
                    };
                    let Some(sid) = stable.resolve(symbol, ident) else {
                        return Err(ComputeError::UnknownSignal);
                    };
                    symbol = sid;
                }

                let Some(handle) = design.get_signal_handle(symbol) else {
                    return Err(ComputeError::UnknownSignal);
                };

                Ok((s.clone(), handle))
            })
            .collect::<ComputeResult<VgHashMap<SignalRef, SignalHandle>>>()?;

        let mut plugins = Vec::new();
        if self.trace {
            plugins.push(Box::new(TracePlugin::default()) as Box<dyn VoglsPlugin>);
        }
        let mut design = design.lower(plugins).map_err(|_| ComputeError::Lowering)?;
        design.optimize(Optimizations {
            rounds: 2,
            flags: OptFlags::ALL,
        });
        let design = design.to_bytecode().map_err(|_| ComputeError::Bytecode)?;
        arena.reset();
        Ok(PlanDesign { design, handles })
    }
}

impl DslNode for LazyDesign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Design {{ sources: {:?}, num_handles: {} }}",
            &self.sources,
            self.handles.len()
        )
    }
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        _converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = self.clone();
        Key::PlanDesign(csp.designs.insert(&mut graph.designs, r))
    }
    fn extend_inputs<'a>(&'a self, _f: &mut Vec<&'a dyn DslNode>) {}
}
