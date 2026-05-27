use std::sync::Arc;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use vogls::SimulationIo;
use vogls::design::{Design, DesignState};
use vogls::symbol::{NetValue, Symbol};
use vogls::utils::{VgHashMap, new_table_key};
use vogls_trace::{Trace, TracePlugin};

use crate::TraceRef;
use crate::array::{Array, DslLazyArray, LazyArrayKey, Value};
use crate::compute::{
    CommonSubPlan, ComputeContext, ComputeDependencies, ComputeError, ComputeGraph, ComputeInputs,
    ComputeNode, ComputeResult, Key,
};
use crate::design::{LazyDesign, LazyDesignKey, SignalRef, Time};
use crate::dsl::{DslNode, DslPtr};
use crate::output::{HammingDistance, Output};

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
    TraceAgg(RunAgg),
    SetSignal(SignalRef, DslLazyArray),
    Repeat(usize),
    RunFor(Time),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum LazyStep {
    TraceStart,
    TraceStop,
    TraceAgg(RunAgg),
    SetSignal(SignalRef, LazyArrayKey),
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
            Self::TraceAgg(_) => None,
            Self::SetSignal(_, k) => Some(Ok(inputs.arrays[k].len())),
            Self::Repeat(n) => Some(Ok(*n)),
            Self::RunFor(_) => None,
        }
    }

    fn compute(
        &self,
        design: &Design,
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
            Self::TraceAgg(agg) => {
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
                        outputs.push(Output::HammingDistance(HammingDistance {
                            indices: Array::Ints([].into()),
                            times: Array::UInts(times.into()),
                            distances: Array::UInts(distances.into()),
                        }));
                    }
                }

                Ok(())
            }
            Self::Repeat(_) => Ok(()),
            Self::SetSignal(signal_ref, lp) => {
                let mut sid = design.elab_table.roots()[0];
                for n in &signal_ref.inner {
                    let Some(ident) = design.ident_table.get(n) else {
                        return Err(ComputeError {});
                    };
                    let Some(ssid) = design.elab_table.resolve(sid, ident) else {
                        return Err(ComputeError {});
                    };
                    sid = ssid;
                }
                let Symbol::Net(net_symbol) = &design.elab_table[sid].content else {
                    return Err(ComputeError {});
                };
                let (signal, _slice) = match &net_symbol.net {
                    NetValue::Signal(s) => s.blocking_drive_signal(),
                    NetValue::Constant(_) => todo!(),
                };
                let value = inputs.arrays[lp].get(rid);
                let value = value.to_bits(net_symbol.net.size());
                design.set_signal(state, design.get_rt_signal(signal), &value);
                Ok(())
            }
            Self::RunFor(time) => design
                .run_from_state(
                    state,
                    &mut SimulationIo {
                        stdout: Box::new(std::io::stdout()),
                        stderr: Box::new(std::io::stderr()),
                    },
                    time.value,
                )
                .map_err(|_| ComputeError {}),
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Run {
    items: Vec<Output>,
}

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
                    DslLazyStep::TraceAgg(agg) => LazyStep::TraceAgg(agg.clone()),
                    DslLazyStep::SetSignal(s, arr) => LazyStep::SetSignal(
                        s.clone(),
                        converted[&DslPtr::from(arr as &dyn DslNode)].as_array(),
                    ),
                    DslLazyStep::Repeat(n) => LazyStep::Repeat(*n),
                    DslLazyStep::RunFor(time) => LazyStep::RunFor(time.clone()),
                })
                .collect(),
        };

        Key::Run(
            *csp.runs
                .entry(r.clone())
                .or_insert_with(|| graph.runs.insert(r)),
        )
    }
    fn extend_inputs<'a>(&'a self, f: &mut Vec<&'a dyn DslNode>) {
        f.push(self.design.as_ref() as &dyn DslNode);
        for step in &self.steps {
            match step {
                DslLazyStep::TraceStart
                | DslLazyStep::TraceStop
                | DslLazyStep::TraceAgg(_)
                | DslLazyStep::RunFor(_)
                | DslLazyStep::Repeat(_) => {}
                DslLazyStep::SetSignal(_, arr) => f.push(arr as &dyn DslNode),
            }
        }
    }
}

impl ComputeNode for LazyRun {
    type Output = Run;

    fn extend_inputs(&self, deps: &mut ComputeDependencies) {
        deps.designs.push(self.design);
        for step in &self.steps {
            match step {
                LazyStep::RunFor(_)
                | LazyStep::Repeat(_)
                | LazyStep::TraceStart
                | LazyStep::TraceStop
                | LazyStep::TraceAgg(_) => {}
                LazyStep::SetSignal(_, k) => deps.arrays.push(*k),
            }
        }
    }
    fn compute(&self, ctx: &ComputeContext, inputs: &ComputeInputs) -> ComputeResult<Self::Output> {
        let num_traces = self.num_traces(inputs).map_err(|_| ComputeError {})?;
        // @TODO: Insert caching here.
        let design = &inputs.designs[&self.design];
        let mut state = design.initial_state.clone();

        // Execute all steps that are shared between traces first on a single trace.
        let mut prelude_steps = 0;
        let mut outputs = Vec::new();
        while let Some(step) = self.steps.get(prelude_steps) {
            match step.num_traces(inputs) {
                Some(Err(_)) => return Err(ComputeError {}),
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
        let items = items.iter().map(|o| collapse_outputs(o)).collect();
        Ok(Run { items })
    }
}

fn collapse_outputs(outputs: &[Output]) -> Output {
    match &outputs[0] {
        Output::Value(v) => Output::Array(match v {
            Value::Float(_) => Array::Floats(
                outputs
                    .iter()
                    .map(|o| match o {
                        Output::Value(Value::Float(v)) => *v,
                        _ => unreachable!(),
                    })
                    .collect::<Arc<[f64]>>(),
            ),
            Value::Int(_) => Array::Ints(
                outputs
                    .iter()
                    .map(|o| match o {
                        Output::Value(Value::Int(v)) => *v,
                        _ => unreachable!(),
                    })
                    .collect::<Arc<[i64]>>(),
            ),
            Value::UInt(_) => Array::UInts(
                outputs
                    .iter()
                    .map(|o| match o {
                        Output::Value(Value::UInt(v)) => *v,
                        _ => unreachable!(),
                    })
                    .collect::<Arc<[u64]>>(),
            ),
            Value::Bits(_) => todo!(),
        }),
        Output::Array(array) => todo!(),
        Output::Plan(plan) => todo!(),
        Output::HammingDistance(_) => {
            let length = outputs
                .iter()
                .map(|o| match o {
                    Output::HammingDistance(arr) => arr.times.len(),
                    _ => unreachable!(),
                })
                .sum::<usize>();

            let mut indices = Vec::with_capacity(length);
            let mut times = Vec::with_capacity(length);
            let mut distances = Vec::with_capacity(length);

            for (i, output) in outputs.iter().enumerate() {
                match output {
                    Output::HammingDistance(hd) => {
                        indices.extend(std::iter::repeat_n(i as u64, hd.times.len()));
                        times.extend(hd.times.as_u64().unwrap().as_ref());
                        distances.extend(hd.distances.as_u64().unwrap().as_ref());
                    }
                    _ => unreachable!(),
                }
            }

            Output::HammingDistance(HammingDistance {
                indices: Array::UInts(indices.into()),
                times: Array::UInts(times.into()),
                distances: Array::UInts(distances.into()),
            })
        }
        Output::Run(_) => todo!(),
    }
}
