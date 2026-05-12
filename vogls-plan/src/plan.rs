use std::sync::Arc;

use vogls::utils::{IndexMap, VgHashMap, new_table_key};

use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeGraph, ComputeInputs, ComputeNode,
    ComputeResult, Key,
};
use crate::dsl::{DslNode, DslPtr};
use crate::output::{DslLazyOutput, LazyOutputKey, Output};

new_table_key! { pub struct LazyPlanKey; }

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LazyPlan {
    pub components: IndexMap<String, LazyOutputKey>,
}
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Plan {
    pub components: IndexMap<String, Output>,
}

impl Plan {
    pub fn to_lazy_dsl(&self) -> DslLazyPlan {
        DslLazyPlan {
            components: Arc::new(
                self.components
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_lazy_dsl()))
                    .collect::<IndexMap<String, DslLazyOutput>>(),
            ),
        }
    }
}

#[derive(Clone)]
pub struct DslLazyPlan {
    pub components: Arc<IndexMap<String, DslLazyOutput>>,
}

impl ComputeNode for LazyPlan {
    type Output = Plan;
    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.outputs.extend(self.components.iter_values());
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output> {
        let components = self
            .components
            .iter()
            .map(|(k, o)| ComputeResult::Ok((k.clone(), inputs.outputs[o].clone())))
            .collect::<ComputeResult<IndexMap<String, Output>>>()?;
        Ok(Plan { components })
    }
}

impl DslNode for DslLazyPlan {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyPlan {
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
        };
        Key::Plan(
            *csp.plans
                .entry(r.clone())
                .or_insert_with(|| graph.plans.insert(r)),
        )
    }

    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.extend(self.components.iter_values().map(|v| v as &dyn DslNode));
    }
}
