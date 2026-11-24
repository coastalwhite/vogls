use std::fmt;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use vogls::design::{Arena, Design, Macro};
use vogls::utils::{IndexMap, VgHashMap, VgHashSet, new_table_key};
use vogls::{DesignState, LogicMode, OptFlags, Optimizations, SignalHandle, VoglsPlugin};
use vogls_trace::HammingDistancePlugin;

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
    /// SDF files to annotate, each with the hierarchical scope it applies to (`None` for the top).
    pub sdf: Vec<(Option<String>, PathBuf)>,
    pub mode: LogicMode,
    pub opt_rounds: u8,
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
    pub initial_state: DesignState,
    pub handles: VgHashMap<SignalRef, SignalHandle>,
}

impl CspAble for LazyDesign {
    fn csp_eq(&self, other: &Self) -> bool {
        let Self {
            sources: l_sources,
            top_level_module: l_tlm,
            defines: l_defines,
            sdf: l_sdf,
            mode: l_mode,
            opt_rounds: l_opt_rounds,
            trace: _,
            handles: _,
        } = self;
        let Self {
            sources: r_sources,
            top_level_module: r_tlm,
            defines: r_defines,
            sdf: r_sdf,
            mode: r_mode,
            opt_rounds: r_opt_rounds,
            trace: _,
            handles: _,
        } = other;
        l_sources == r_sources
            && l_defines == r_defines
            && l_tlm == r_tlm
            && l_sdf == r_sdf
            && l_mode == r_mode
            && l_opt_rounds == r_opt_rounds
    }
    fn csp_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sources.hash(state);
        self.defines.hash(state);
        self.top_level_module.hash(state);
        self.sdf.hash(state);
        self.mode.hash(state);
        self.opt_rounds.hash(state);
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
            self.sources,
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
        if self.mode == LogicMode::TwoValue {
            builder.define_macro("__VOGLS__TWO_VALUE_LOGIC", Macro::default());
        }
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
            .elaborate(self.mode, self.top_level_module.as_deref())
            .map_err(|_| ComputeError::Elaboration)?;
        for (scope, path) in &self.sdf {
            if let Err(err) = design.annotate_sdf(path, scope.as_deref()) {
                println!("{err}");
                return Err(ComputeError::SdfAnnotation);
            }
        }

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
            plugins.push(Box::new(HammingDistancePlugin::new()) as Box<dyn VoglsPlugin>);
        }
        let mut design = design.lower(plugins).map_err(|_| ComputeError::Lowering)?;
        design.optimize(Optimizations {
            rounds: self.opt_rounds,
            flags: OptFlags::ALL,
        });
        let (design, initial_state) = design.to_bytecode().map_err(|_| ComputeError::Bytecode)?;
        arena.reset();
        Ok(PlanDesign {
            design,
            initial_state,
            handles,
        })
    }
}

impl DslNode for LazyDesign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Design {{ sources: {:?}, num_handles: {} }}",
            self.sources,
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
