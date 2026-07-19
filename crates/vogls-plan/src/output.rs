use std::fmt;
use std::hash::{Hash as _, Hasher};
use std::sync::Arc;

use vogls::utils::{VgHashMap, new_table_key};

use crate::CspAble;
use crate::array::{Array, DslArrayExtractOutput, DslLazyArray};
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, GraphItem, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::plan::{DslLazyPlan, DslPlanExtractOutput, LazyPlanKey, Plan};
use crate::run_vector::{DslRunVector, DslRunVectorExtractOutput, RunVector};
use crate::typing::Type;
use crate::value::{DslLazyValue, DslValueExtractOutput, Value};

new_table_key! { pub struct LazyOutputKey; }

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct HammingDistance {
    pub indices: Array,
    pub times: Array,
    pub distances: Array,
}

#[derive(Clone)]
pub struct LazyOutput {
    pub ty: Arc<Type>,
    pub f: Arc<dyn OutputNode>,
}

impl CspAble for LazyOutput {
    fn csp_eq(&self, other: &Self) -> bool {
        self.f.as_ref().csp_eq(other.f.as_ref())
    }
    fn csp_hash<H: Hasher>(&self, state: &mut H) {
        self.f.type_id().hash(state);
        self.f.as_ref().csp_hash(state)
    }
    fn csp_merge(&mut self, _other: Self) {}
}

pub trait OutputNode: std::any::Any {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn csp_eq(&self, other: &dyn OutputNode) -> bool;
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
        DslLazyOutput {
            ty: Arc::new(self.ty()),
            f: Arc::new(self.clone()) as _,
        }
    }

    pub fn ty(&self) -> Type {
        match self {
            Output::Value(v) => Type::Value(v.ty()),
            Output::Array(v) => Type::Array(v.ty()),
            Output::Plan(v) => Type::Plan(v.ty()),
            Output::RunVector(v) => Type::RunVector(v.ty()),
        }
    }
}

#[derive(Clone)]
pub struct DslLazyOutput {
    pub ty: Arc<Type>,
    pub f: Arc<dyn DslOutputNode>,
}

impl DslLazyOutput {
    pub fn ty(&self) -> &Arc<Type> {
        &self.ty
    }

    pub fn extract_value(self) -> DslLazyValue {
        assert!(self.ty.is_value());
        DslLazyValue {
            ty: self.ty.clone(),
            f: Arc::new(DslValueExtractOutput(self)),
        }
    }
    pub fn extract_array(self) -> DslLazyArray {
        assert!(self.ty.is_array());
        DslLazyArray {
            ty: self.ty.clone(),
            f: Arc::new(DslArrayExtractOutput(self)),
        }
    }
    pub fn extract_plan(self) -> DslLazyPlan {
        assert!(self.ty.is_plan());
        DslLazyPlan {
            ty: self.ty.clone(),
            f: Arc::new(DslPlanExtractOutput(self)),
        }
    }
    pub fn extract_run_vector(self) -> DslRunVector {
        assert!(self.ty.is_run_vector());
        DslRunVector {
            ty: self.ty.clone(),
            f: Arc::new(DslRunVectorExtractOutput(self)),
        }
    }
}

pub trait DslOutputNode: Send + Sync + 'static {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn OutputNode>;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}

impl ComputeNode for LazyOutput {
    type Key = LazyOutputKey;
    type Output = Output;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.f.fmt(f)
    }
    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        self.f.extend_inputs(deps)
    }
    fn compute(
        &self,
        ctx: &ComputeContext,
        inputs: &ComputeInputs,
    ) -> ComputeResult<<Self as ComputeNode>::Output> {
        self.f.compute(ctx, inputs)
    }
}

impl DslNode for DslLazyOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.f.fmt(f)
    }
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, crate::compute::Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyOutput {
            ty: self.ty.clone(),
            f: self.f.convert_one(converted),
        };
        Key::Output(csp.outputs.insert(&mut graph.outputs, r))
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        self.f.extend_inputs(f)
    }
}

impl DslOutputNode for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Literal Output")
    }
    fn convert_one<'a>(&'a self, _converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn OutputNode> {
        Arc::new(self.clone())
    }
    fn extend_inputs<'a>(&'a self, _f: &mut Vec<&'a dyn DslNode>) {}
}
impl OutputNode for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Literal Output")
    }
    impl_dyn_eq_hash!(OutputNode);
    fn extend_inputs(&self, _deps: &mut ComputeDependencies) {}
    fn compute(&self, _ctx: &ComputeContext, _inputs: &ComputeInputs) -> ComputeResult<Output> {
        Ok(self.clone())
    }
}

macro_rules! impl_upcast {
    ($dsl:ty, $node:ty, $key:ident, $table:ident) => {
        impl From<$dsl> for DslLazyOutput {
            fn from(value: $dsl) -> Self {
                Self {
                    ty: value.ty.clone(),
                    f: Arc::new(value),
                }
            }
        }
        impl DslOutputNode for $dsl {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!("Upcast ", stringify!($key)))
            }
            fn convert_one<'a>(
                &'a self,
                converted: &'a VgHashMap<DslPtr, Key>,
            ) -> Arc<dyn OutputNode> {
                let key = converted[&DslPtr::from(self as &dyn DslNode)];
                let key = <$node as GraphItem>::from_key(key).unwrap();
                Arc::new(key)
            }
            fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
                f.push(self);
            }
        }
        impl OutputNode for <$node as GraphItem>::Key {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!("Upcast ", stringify!($key)))
            }
            fn csp_eq(&self, other: &dyn OutputNode) -> bool {
                let Some(other) = (other as &dyn std::any::Any).downcast_ref::<Self>() else {
                    return false;
                };
                self == other
            }
            fn csp_hash(&self, mut state: &mut dyn std::hash::Hasher) {
                std::any::TypeId::of::<Self>().hash(&mut state);
                self.hash(&mut state);
            }

            fn extend_inputs(&self, deps: &mut ComputeDependencies) {
                deps.$table.push(*self);
            }
            fn compute(
                &self,
                _ctx: &ComputeContext,
                inputs: &ComputeInputs,
            ) -> ComputeResult<Output> {
                Ok(Output::$key(inputs.$table[self].clone()))
            }
        }
    };
}

impl_upcast!(
    crate::value::DslLazyValue,
    crate::value::LazyValue,
    Value,
    values
);
impl_upcast!(
    crate::array::DslLazyArray,
    crate::array::LazyArray,
    Array,
    arrays
);
impl_upcast!(crate::plan::DslLazyPlan, crate::plan::LazyPlan, Plan, plans);
impl_upcast!(
    crate::run_vector::DslRunVector,
    crate::run_vector::LazyRunVector,
    RunVector,
    run_vectors
);

pub struct DslPlanComponent {
    pub plan: DslLazyPlan,
    pub key: String,
}
#[derive(Clone, PartialEq, Hash)]
pub struct PlanComponent {
    pub plan: LazyPlanKey,
    pub key: String,
}

impl DslOutputNode for DslPlanComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Plan component: '{}'", self.key)
    }
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn OutputNode> {
        Arc::new(PlanComponent {
            plan: converted[&DslPtr::from(&self.plan as &dyn DslNode)].as_plan(),
            key: self.key.clone(),
        })
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.push(&self.plan);
    }
}
impl OutputNode for PlanComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Plan component: '{}'", self.key)
    }
    impl_dyn_eq_hash!(OutputNode);
    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.plans.push(self.plan);
    }

    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Output> {
        let plan = &inputs.plans[&self.plan];
        let component = plan
            .components
            .get(&self.key.to_string())
            .ok_or_else(|| ComputeError::UnknownComponent(self.key.clone()))?;
        Ok(component.clone())
    }
}

impl DslPlanComponent {
    pub fn build(self) -> DslLazyOutput {
        let ty = self
            .plan
            .ty()
            .components
            .get(&self.key.to_string())
            .unwrap()
            .clone();
        DslLazyOutput {
            ty: Arc::new(ty),
            f: Arc::new(self),
        }
    }
}
