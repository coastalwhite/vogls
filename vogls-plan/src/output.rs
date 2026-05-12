use std::sync::Arc;

use vogls::utils::{VgHashMap, new_table_key};

use crate::array::{Array, DslLazyArray, DslLazyValue, LazyArrayKey, LazyValueKey, Value};
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::plan::{DslLazyPlan, LazyPlanKey, Plan};
use crate::run::{DslLazyRun, LazyRunKey, Run};

new_table_key! { pub struct LazyOutputKey; }

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct HammingDistance {
    pub indices: Array,
    pub times: Array,
    pub distances: Array,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum LazyOutput {
    Output(Output),
    Value(LazyValueKey),
    Array(LazyArrayKey),
    Plan(LazyPlanKey),
    PlanComponent(LazyPlanKey, String),
    Run(LazyRunKey),
    HammingDistance(HammingDistance),
}
#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Output {
    Value(Value),
    Array(Array),
    Plan(Plan),
    Run(Run),
    HammingDistance(HammingDistance),
}
impl Output {
    pub fn to_lazy_dsl(&self) -> DslLazyOutput {
        DslLazyOutput::Output(self.clone())
    }
}

impl From<DslLazyArray> for DslLazyOutput {
    fn from(value: DslLazyArray) -> Self {
        Self::Array(Arc::new(value))
    }
}
impl From<DslLazyValue> for DslLazyOutput {
    fn from(value: DslLazyValue) -> Self {
        Self::Value(Arc::new(value))
    }
}
impl From<DslLazyPlan> for DslLazyOutput {
    fn from(value: DslLazyPlan) -> Self {
        Self::Plan(Arc::new(value))
    }
}

#[derive(Clone)]
pub enum DslLazyOutput {
    Output(Output),
    Value(Arc<DslLazyValue>),
    Array(Arc<DslLazyArray>),
    Plan(Arc<DslLazyPlan>),
    PlanComponent(Arc<DslLazyPlan>, String),
    Run(Arc<DslLazyRun>),
    HammingDistance(HammingDistance),
}

impl ComputeNode for LazyOutput {
    type Output = Output;

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        match self {
            Self::Output(_) => {}
            Self::Run(k) => deps.runs.push(*k),
            Self::Value(k) => deps.values.push(*k),
            Self::Array(k) => deps.arrays.push(*k),
            Self::Plan(k) | Self::PlanComponent(k, _) => deps.plans.push(*k),
            Self::HammingDistance(_) => {}
        }
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<<Self as ComputeNode>::Output> {
        use Output as O;
        Ok(match self {
            Self::Output(l) => l.clone(),
            Self::Run(l) => O::Run(inputs.runs[l].clone()),
            Self::Value(l) => O::Value(inputs.values[l].clone()),
            Self::Array(l) => O::Array(inputs.arrays[l].clone()),
            Self::Plan(l) => O::Plan(inputs.plans[l].clone()),
            Self::PlanComponent(l, component) => inputs.plans[l]
                .components
                .get(component)
                .ok_or_else(|| ComputeError {})?
                .clone(),
            Self::HammingDistance(v) => Output::HammingDistance(v.clone()),
        })
    }
}

impl DslNode for DslLazyOutput {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, crate::compute::Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        use LazyOutput as O;
        let r = match self {
            Self::Output(v) => O::Output(v.clone()),
            Self::Run(v) => O::Run(converted[&DslPtr::from(v.as_ref() as &dyn DslNode)].as_run()),
            Self::Value(v) => {
                O::Value(converted[&DslPtr::from(v.as_ref() as &dyn DslNode)].as_value())
            }
            Self::Array(v) => {
                O::Array(converted[&DslPtr::from(v.as_ref() as &dyn DslNode)].as_array())
            }
            Self::Plan(v) => {
                O::Plan(converted[&DslPtr::from(v.as_ref() as &dyn DslNode)].as_plan())
            }
            Self::PlanComponent(v, component) => O::PlanComponent(
                converted[&DslPtr::from(v.as_ref() as &dyn DslNode)].as_plan(),
                component.clone(),
            ),
            Self::HammingDistance(v) => O::HammingDistance(v.clone()),
        };
        Key::Output(
            *csp.outputs
                .entry(r.clone())
                .or_insert_with(|| graph.outputs.insert(r)),
        )
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        match self {
            Self::Run(v) => f.push(v.as_ref()),
            Self::Output(_) => {}
            Self::Value(v) => f.push(v.as_ref()),
            Self::Array(v) => f.push(v.as_ref()),
            Self::Plan(v) => f.push(v.as_ref()),
            Self::PlanComponent(v, _) => f.push(v.as_ref()),
            Self::HammingDistance(_) => {}
        }
    }
}
