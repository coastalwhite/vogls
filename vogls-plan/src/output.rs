use std::hash::Hasher;
use std::sync::Arc;

use vogls::utils::{VgHashMap, new_table_key};

use crate::CspAble;
use crate::array::{Array, DslLazyArray, LazyArrayKey};
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::plan::{DslLazyPlan, LazyPlanKey, Plan};
use crate::run_vector::{DslRunVector, LazyRunVectorKey, RunVector};
use crate::value::{DslLazyValue, LazyValueKey, Value};

new_table_key! { pub struct LazyOutputKey; }

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct HammingDistance {
    pub indices: Array,
    pub times: Array,
    pub distances: Array,
}

#[derive(Clone)]
pub enum LazyOutput {
    Output(Output),
    Value(LazyValueKey),
    Array(LazyArrayKey),
    Plan(LazyPlanKey),
    PlanComponent(LazyPlanKey, String),
    RunVector(LazyRunVectorKey),
    Function(Arc<dyn OutputFunction>),
}

impl CspAble for LazyOutput {
    fn csp_eq(&self, other: &Self) -> bool {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            return false;
        }

        match (self, other) {
            (Self::Output(lhs), Self::Output(rhs)) => lhs == rhs,
            (Self::Value(lhs), Self::Value(rhs)) => lhs == rhs,
            (Self::Array(lhs), Self::Array(rhs)) => lhs == rhs,
            (Self::Plan(lhs), Self::Plan(rhs)) => lhs == rhs,
            (Self::PlanComponent(lhs, lhs_s), Self::PlanComponent(rhs, rhs_s)) => {
                lhs == rhs && lhs_s == rhs_s
            }
            (Self::RunVector(lhs), Self::RunVector(rhs)) => lhs == rhs,
            (Self::Function(lhs), Self::Function(rhs)) => {
                OutputFunction::csp_eq(lhs.as_ref(), rhs.as_ref())
            }
            _ => unreachable!(),
        }
    }
    fn csp_hash<H: Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Output(e) => e.hash(state),
            Self::Value(e) => e.hash(state),
            Self::Array(e) => e.hash(state),
            Self::Plan(e) => e.hash(state),
            Self::PlanComponent(e, s) => {
                e.hash(state);
                s.hash(state);
            }
            Self::RunVector(e) => e.hash(state),
            Self::Function(e) => {
                OutputFunction::csp_hash(e.as_ref(), state as &mut dyn Hasher).hash(state)
            }
        }
    }
    fn csp_merge(&mut self, _other: Self) {}
}

pub trait OutputFunction: std::any::Any {
    fn csp_eq(&self, other: &dyn OutputFunction) -> bool;
    fn csp_hash(&self, state: &mut dyn Hasher);
    fn extend_inputs(&self, deps: &mut ComputeDependencies);
    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Output>;
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Output {
    Value(Value),
    Array(Array),
    Plan(Plan),
    RunVector(RunVector),
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
    RunVector(Arc<DslRunVector>),
    Function(Arc<dyn DslOutputFunction>),
}

pub trait DslOutputFunction: Send + Sync + 'static {
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn OutputFunction>;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}

impl ComputeNode for LazyOutput {
    type Key = LazyOutputKey;
    type Output = Output;

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        match self {
            Self::Output(_) => {}
            Self::RunVector(k) => deps.run_vectors.push(*k),
            Self::Value(k) => deps.values.push(*k),
            Self::Array(k) => deps.arrays.push(*k),
            Self::Plan(k) | Self::PlanComponent(k, _) => deps.plans.push(*k),
            Self::Function(f) => f.extend_inputs(deps),
        }
    }
    fn compute(
        &self,
        ctx: &ComputeContext,
        inputs: &ComputeInputs,
    ) -> ComputeResult<<Self as ComputeNode>::Output> {
        use Output as O;
        Ok(match self {
            Self::Output(l) => l.clone(),
            Self::RunVector(l) => O::RunVector(inputs.run_vectors[l].clone()),
            Self::Value(l) => O::Value(inputs.values[l].clone()),
            Self::Array(l) => O::Array(inputs.arrays[l].clone()),
            Self::Plan(l) => O::Plan(inputs.plans[l].clone()),
            Self::PlanComponent(l, component) => inputs.plans[l]
                .components
                .get(component)
                .ok_or_else(|| ComputeError::UnknownComponent)?
                .clone(),
            Self::Function(f) => f.compute(ctx, inputs)?,
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
            Self::RunVector(v) => {
                O::RunVector(converted[&DslPtr::from(v.as_ref() as &dyn DslNode)].as_run_vector())
            }
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
            Self::Function(v) => O::Function(v.convert_one(converted)),
        };
        Key::Output(csp.outputs.insert(&mut graph.outputs, r))
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        match self {
            Self::RunVector(v) => f.push(v.as_ref()),
            Self::Output(_) => {}
            Self::Value(v) => f.push(v.as_ref()),
            Self::Array(v) => f.push(v.as_ref()),
            Self::Plan(v) => f.push(v.as_ref()),
            Self::PlanComponent(v, _) => f.push(v.as_ref()),
            Self::Function(v) => v.extend_inputs(f),
        }
    }
}
