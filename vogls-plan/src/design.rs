use std::path::PathBuf;

use vogls::LogicMode;
use vogls::design::{Arena, Design};
use vogls::runtime::plugins::RuntimePluginState;
use vogls::utils::{VgHashMap, new_table_key};
use vogls_trace::TracePlugin;

use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};

new_table_key! { pub struct LazyDesignKey; }

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LazyDesign {
    pub sources: Vec<PathBuf>,
    pub top_level_module: Option<String>,
    pub trace: bool,
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
    type Output = Design;

    fn extend_inputs(&self, _deps: &mut ComputeDependencies) {}
    fn compute(
        &self,
        _ctx: &ComputeContext,
        _inputs: &ComputeInputs,
    ) -> ComputeResult<Self::Output> {
        let mut builder = vogls::design::DesignBuilder::new();
        let mut arena = Arena::default();
        for path in &self.sources {
            builder.add_source(path).map_err(|_| ComputeError {})?;
        }
        let parsed = builder.parse(&mut arena).map_err(|_| ComputeError {})?;
        let design = parsed
            .elaborate(LogicMode::TwoValue, self.top_level_module.as_deref())
            .map_err(|_| ComputeError {})?;
        let mut plugins = Vec::new();
        if self.trace {
            plugins.push(Box::new(TracePlugin::default()) as RuntimePluginState);
        }
        let mut design = design
            .lower(&parsed, plugins)
            .map_err(|_| ComputeError {})?;
        design.optimize(vogls::ir::optimize::OptFlags {
            opt_rounds: 2,
            constant_propagation: true,
            deadcode_elimination: true,
            common_subexpr_elim: true,
            peephole: true,
        });
        let design = design.to_bytecode(parsed).map_err(|_| ComputeError {})?;
        arena.reset();
        Ok(design)
    }
}

impl DslNode for LazyDesign {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        _converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = self.clone();

        Key::Design(
            *csp.designs
                .entry(r.clone())
                .or_insert_with(|| graph.designs.insert(r)),
        )
    }
    fn extend_inputs<'a>(&'a self, _f: &mut Vec<&'a dyn DslNode>) {}
}
