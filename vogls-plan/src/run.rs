use std::sync::Arc;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use vogls::SimulationIo;
use vogls::design::DesignState;
use vogls::utils::{IndexMap, VgHashMap, new_table_key};
use vogls_trace::Trace;

use crate::TraceRef;
use crate::array::{Array, DslLazyArray, LazyArrayKey};
use crate::buffer::Buffer;
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::design::{LazyDesign, LazyDesignKey, PlanDesign, SignalRef, Time};
use crate::dsl::{DslNode, DslPtr};
use crate::output::Output;
use crate::plan::Plan;
use crate::run_vector::{RunOffsets, RunVector};
use crate::value::Value;

new_table_key! { pub struct LazyRunKey; }

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LazyRun {
    pub design: LazyDesignKey,
    pub steps: Vec<LazyStep>,
}

#[derive(Clone)]
pub struct DslLazyRun {
    pub design: Arc<LazyDesign>,
    pub steps: Vec<DslLazyStep>,
}

#[derive(Clone)]
pub enum DslLazyStep {
    TraceStart,
    TraceStop,
    TraceAgg(String, RunAgg),
    SetSignal(String, SignalRef, DslLazyArray),
    Repeat(usize),
    RunFor(Time),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum LazyStep {
    TraceStart,
    TraceStop,
    TraceAgg(String, RunAgg),
    SetSignal(String, SignalRef, LazyArrayKey),
    Repeat(usize),
    RunFor(Time),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum RunAgg {
    HammingWeight,
    HammingDistance,
}

impl LazyRun {
    fn num_traces(&self, inputs: &ComputeInputs) -> Result<usize, ()> {
        let mut current_num_traces = 1;
        for step in self.steps.iter() {
            if let Some(step_num_traces) = step.num_traces(inputs) {
                let step_num_traces = step_num_traces?;

                if current_num_traces == 1 {
                    current_num_traces = step_num_traces;
                    continue;
                }

                if current_num_traces != step_num_traces && step_num_traces != 1 {
                    return Err(());
                }
            }
        }
        Ok(current_num_traces)
    }
}

impl LazyStep {
    fn num_traces(&self, inputs: &ComputeInputs) -> Option<Result<usize, ()>> {
        match self {
            Self::TraceStart => None,
            Self::TraceStop => None,
            Self::TraceAgg(_, _) => None,
            Self::SetSignal(_, _, k) => Some(Ok(inputs.arrays[k].len())),
            Self::Repeat(n) => Some(Ok(*n)),
            Self::RunFor(_) => None,
        }
    }

    fn compute(
        &self,
        design: &PlanDesign,
        state: &mut DesignState,
        inputs: &ComputeInputs,
        trace_ref: &mut Option<TraceRef>,
        outputs: &mut Vec<Output>,
        traces: &mut Vec<Trace>,
        rid: usize,
    ) -> ComputeResult<()> {
        match self {
            Self::TraceStart => {
                if trace_ref.is_some() {
                    panic!("double trace");
                }
                *trace_ref = Some(TraceRef(0));
                Ok(())
            }
            Self::TraceStop => {
                let Some(trace_ref) = trace_ref.take() else {
                    panic!("didn't start trace");
                };
                traces.push(trace_ref.extract(state));
                Ok(())
            }
            Self::TraceAgg(_, agg) => {
                let trace = trace_ref
                    .take()
                    .map(|t| t.extract(state))
                    .or_else(|| traces.pop());
                let Some(trace) = trace else {
                    panic!("didn't start trace");
                };

                match agg {
                    RunAgg::HammingWeight => todo!(),
                    RunAgg::HammingDistance => {
                        let (times, distances) = trace.hamming_distance();
                        outputs.push(Output::Array(Array::UInts(Buffer::from_vec(distances))));
                        outputs.push(Output::Array(Array::UInts(Buffer::from_vec(times))));
                    }
                }

                Ok(())
            }
            Self::Repeat(_) => Ok(()),
            Self::SetSignal(_, signal_ref, lp) => {
                let handle = design.handles[signal_ref];
                let rt = design.design.resolve_handle(handle);
                let size = design.design.resolve_handle_width(handle);
                let value = inputs.arrays[lp].get(rid);
                let value = value.to_bits(size);
                design.design.set_signal(state, rt, &value);
                Ok(())
            }
            Self::RunFor(time) => design
                .design
                .run_from_state(
                    state,
                    &mut SimulationIo {
                        stdout: Box::new(std::io::stdout()),
                        stderr: Box::new(std::io::stderr()),
                    },
                    time.value,
                )
                .map_err(|_| ComputeError::FailedToRun),
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Run(Plan);

impl DslNode for DslLazyRun {
    fn convert_one<'a>(
        &'a self,
        graph: &'a mut ComputeGraph,
        converted: &'a VgHashMap<DslPtr, Key>,
        csp: &'a mut CommonSubPlan,
    ) -> Key {
        let r = LazyRun {
            design: converted[&DslPtr::from(self.design.as_ref() as &dyn DslNode)].as_design(),
            steps: self
                .steps
                .iter()
                .map(|step| match step {
                    DslLazyStep::TraceStart => LazyStep::TraceStart,
                    DslLazyStep::TraceStop => LazyStep::TraceStop,
                    DslLazyStep::TraceAgg(name, agg) => {
                        LazyStep::TraceAgg(name.clone(), agg.clone())
                    }
                    DslLazyStep::SetSignal(name, s, arr) => LazyStep::SetSignal(
                        name.clone(),
                        s.clone(),
                        converted[&DslPtr::from(arr as &dyn DslNode)].as_array(),
                    ),
                    DslLazyStep::Repeat(n) => LazyStep::Repeat(*n),
                    DslLazyStep::RunFor(time) => LazyStep::RunFor(time.clone()),
                })
                .collect(),
        };
        Key::Run(csp.runs.insert(&mut graph.runs, r))
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.push(self.design.as_ref() as &dyn DslNode);
        for step in &self.steps {
            match step {
                DslLazyStep::TraceStart
                | DslLazyStep::TraceStop
                | DslLazyStep::TraceAgg(..)
                | DslLazyStep::RunFor(_)
                | DslLazyStep::Repeat(_) => {}
                DslLazyStep::SetSignal(_, _, arr) => f.push(arr as &dyn DslNode),
            }
        }
    }
}

impl ComputeNode for LazyRun {
    type Output = Plan;

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.designs.push(self.design);
        for step in &self.steps {
            match step {
                LazyStep::RunFor(_)
                | LazyStep::Repeat(_)
                | LazyStep::TraceStart
                | LazyStep::TraceStop
                | LazyStep::TraceAgg(..) => {}
                LazyStep::SetSignal(_, _, k) => deps.arrays.push(*k),
            }
        }
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output> {
        let num_traces = self
            .num_traces(inputs)
            .map_err(|_| ComputeError::NumTracesMismatch)?;
        // @TODO: Insert caching here.
        let design = &inputs.designs[&self.design];
        let mut state = design.design.initial_state().clone();

        // Execute all steps that are shared between traces first on a single trace.
        let mut prelude_steps = 0;
        let mut outputs = Vec::new();
        while let Some(step) = self.steps.get(prelude_steps) {
            match step.num_traces(inputs) {
                Some(Err(_)) => return Err(ComputeError::FailedToResolveNumTraces),
                None => {}
                Some(Ok(n)) => {
                    if n != 1 {
                        break;
                    }
                }
            }

            if matches!(step, LazyStep::TraceStart | LazyStep::TraceStop) {
                break;
            }

            step.compute(
                &design,
                &mut state,
                &inputs,
                &mut None,
                &mut outputs,
                &mut Vec::new(),
                0,
            )?;
            prelude_steps += 1;
        }

        let items = ctx.pool.install(|| {
            (0..num_traces)
                .into_par_iter()
                .map(|rid| {
                    let mut state = state.clone();
                    let mut trace_ref = None;
                    let mut outputs = Vec::<Output>::new();
                    let mut traces = Vec::new();

                    for step in &self.steps[prelude_steps..] {
                        step.compute(
                            &design,
                            &mut state,
                            &inputs,
                            &mut trace_ref,
                            &mut outputs,
                            &mut traces,
                            rid,
                        )?;
                    }

                    if let Some(trace_ref) = trace_ref.take() {
                        traces.push(trace_ref.extract(&mut state));
                    }

                    Ok(outputs)
                })
                .collect::<ComputeResult<Vec<Vec<Output>>>>()
        })?;
        let items = items
            .iter()
            .map(|o| collapse_outputs(o))
            .collect::<Vec<_>>();
        let mut items = items.into_iter();

        let mut components = IndexMap::default();

        for step in &self.steps {
            match step {
                LazyStep::TraceStart
                | LazyStep::TraceStop
                | LazyStep::RunFor(..)
                | LazyStep::Repeat(..) => {}
                LazyStep::TraceAgg(name, ..) => {
                    components.insert(
                        format!("{name}.dist"),
                        Output::RunVector(items.next().unwrap()),
                    );
                    components.insert(
                        format!("{name}.time"),
                        Output::RunVector(items.next().unwrap()),
                    );
                }
                LazyStep::SetSignal(name, ..) => {
                    components.insert(name.clone(), Output::RunVector(items.next().unwrap()));
                }
            }
        }

        Ok(Plan { components })
    }
}

fn collapse_outputs(outputs: &[Output]) -> RunVector {
    match &outputs[0] {
        Output::Value(v) => {
            let data = match v {
                Value::Float(_) => Array::Floats(
                    outputs
                        .iter()
                        .map(|o| match o {
                            Output::Value(Value::Float(v)) => *v,
                            _ => unreachable!(),
                        })
                        .collect::<Buffer<f64>>(),
                ),
                Value::Int(_) => Array::Ints(
                    outputs
                        .iter()
                        .map(|o| match o {
                            Output::Value(Value::Int(v)) => *v,
                            _ => unreachable!(),
                        })
                        .collect::<Buffer<i64>>(),
                ),
                Value::UInt(_) => Array::UInts(
                    outputs
                        .iter()
                        .map(|o| match o {
                            Output::Value(Value::UInt(v)) => *v,
                            _ => unreachable!(),
                        })
                        .collect::<Buffer<u64>>(),
                ),
                Value::Bits(_) => todo!(),
            };
            RunVector {
                offsets: RunOffsets::Scalar(outputs.len()),
                data,
            }
        }
        Output::Array(v) => {
            let mut offset = 0usize;
            let offsets = outputs
                .iter()
                .map(|o| match o {
                    Output::Array(a) => {
                        offset += a.len();
                        offset as u64
                    }
                    _ => unreachable!(),
                })
                .collect();
            let data = match v {
                Array::Floats(_) => {
                    let mut data = Vec::with_capacity(offset);
                    outputs.iter().for_each(|o| match o {
                        Output::Array(Array::Floats(v)) => data.extend_from_slice(v.as_slice()),
                        _ => unreachable!(),
                    });
                    Array::Floats(Buffer::from_vec(data))
                }
                Array::Ints(_) => {
                    let mut data = Vec::with_capacity(offset);
                    outputs.iter().for_each(|o| match o {
                        Output::Array(Array::Ints(v)) => data.extend_from_slice(v.as_slice()),
                        _ => unreachable!(),
                    });
                    Array::Ints(Buffer::from_vec(data))
                }
                Array::UInts(_) => {
                    let mut data = Vec::with_capacity(offset);
                    outputs.iter().for_each(|o| match o {
                        Output::Array(Array::UInts(v)) => data.extend_from_slice(v.as_slice()),
                        _ => unreachable!(),
                    });
                    Array::UInts(Buffer::from_vec(data))
                }
                Array::Bits(..) => todo!(),
            };

            RunVector {
                offsets: RunOffsets::Offsets(offsets),
                data,
            }
        }
        Output::Plan(_plan) => todo!(),
        Output::RunVector(_) => todo!(),
    }
}
