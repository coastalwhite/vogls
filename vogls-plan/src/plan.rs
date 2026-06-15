use std::any::Any;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use vogls::utils::{IndexMap, VgHashMap, new_table_key};

use crate::CspAble;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeGraph, ComputeInputs, ComputeNode,
    ComputeResult, Key, PreparationContext,
};
use crate::dsl::{DslNode, DslPtr};
use crate::output::{DslLazyOutput, LazyOutputKey, Output};

new_table_key! { pub struct LazyPlanKey; }

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Plan {
    pub components: IndexMap<String, Output>,
}

impl Plan {
    pub fn to_lazy_dsl(&self) -> DslLazyPlan {
        DslLazyPlan(Arc::new(DslLiteralPlan {
            components: self
                .components
                .iter()
                .map(|(k, v)| (k.clone(), v.to_lazy_dsl()))
                .collect::<IndexMap<String, DslLazyOutput>>(),
        }))
    }
}

#[derive(Clone)]
pub struct DslLazyPlan(pub Arc<dyn DslPlanNode>);
#[derive(Clone)]
pub struct LazyPlan(pub Arc<dyn PlanNode>);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LazyLiteralPlan {
    pub components: IndexMap<String, LazyOutputKey>,
}
#[derive(Clone)]
pub struct DslLiteralPlan {
    pub components: IndexMap<String, DslLazyOutput>,
}

impl DslPlanNode for DslLiteralPlan {
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn PlanNode> {
        Arc::new(LazyLiteralPlan {
            components: self
                .components
                .iter()
                .map(|(k, c)| {
                    (
                        k.clone(),
                        converted[&DslPtr::from(c as &dyn DslNode)].as_output(),
                    )
                })
                .collect(),
        }) as _
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.extend(self.components.iter_values().map(|v| v as &dyn DslNode));
    }
}

impl PlanNode for LazyLiteralPlan {
    fn csp_eq(&self, other: &dyn PlanNode) -> bool {
        let Some(other) = (other as &dyn Any).downcast_ref::<Self>() else {
            return false;
        };
        self == other
    }
    fn csp_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }
    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.outputs.extend(self.components.iter_values());
    }
    fn compute(&self, _ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Plan> {
        let components = self
            .components
            .iter()
            .map(|(k, o)| ComputeResult::Ok((k.clone(), inputs.outputs[o].clone())))
            .collect::<ComputeResult<IndexMap<String, Output>>>()?;
        Ok(Plan { components })
    }
}

pub trait DslPlanNode: Send + Sync + 'static {
    fn convert_one<'a>(&'a self, converted: &'a VgHashMap<DslPtr, Key>) -> Arc<dyn PlanNode>;
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>);
}
pub trait PlanNode: std::any::Any + Send + Sync + 'static {
    fn csp_eq(&self, other: &dyn PlanNode) -> bool;
    fn csp_hash(&self, state: &mut dyn Hasher);
    fn prepare(&self, ctx: &ComputeContext, pctx: &mut PreparationContext) -> ComputeResult<()> {
        _ = ctx;
        _ = pctx;
        Ok(())
    }
    fn extend_inputs(&self, deps: &mut ComputeDependencies);
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Plan>;
}

impl CspAble for LazyPlan {
    fn csp_eq(&self, other: &Self) -> bool {
        self.0.csp_eq(other.0.as_ref())
    }
    fn csp_hash<H: Hasher>(&self, mut state: &mut H) {
        self.0.csp_hash(&mut state)
    }
    fn csp_merge(&mut self, _other: Self) {}
}
impl ComputeNode for LazyPlan {
    type Key = LazyPlanKey;
    type Output = Plan;

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        self.0.extend_inputs(deps);
    }
    fn prepare(
        &self,
        _graph: &ComputeGraph,
        ctx: &ComputeContext,
        pctx: &mut PreparationContext,
    ) -> ComputeResult<()> {
        self.0.prepare(ctx, pctx)
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output> {
        self.0.compute(ctx, inputs)
    }
}
impl DslNode for DslLazyPlan {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyPlan(self.0.convert_one(converted));
        Key::Plan(csp.plans.insert(&mut graph.plans, r))
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        self.0.extend_inputs(f);
    }
}
