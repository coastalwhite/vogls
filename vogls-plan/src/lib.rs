use std::num::NonZeroU32;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::sync::Arc;

use rand::RngExt;
use rand::rngs::SmallRng;
use rayon::{ThreadPool, ThreadPoolBuilder, prelude::*};
use vogls::design::{Design, DesignState};
use vogls::symbol::{NetValue, Symbol};
use vogls::utils::{TimerStack, VgHashMap};
use vogls::{Bits, ExecutionContext, LogicMode, SimulationIo, VectorSize};
use vogls_trace::{Trace, TracePlugin};

#[derive(Clone)]
pub enum Points {
    Floats(Arc<[f64]>),
    Ints(Arc<[i64]>),
    UInts(Arc<[u64]>),
    Bits(Bits, NonZeroU32),
    Lists(Arc<Points>, Arc<[usize]>),
    Struct(Arc<[(String, Points)]>, usize),
}

#[derive(Clone)]
pub enum Value {
    Float(f64),
    Int(i64),
    UInt(u64),
    Bits(Bits),
    Lists(Points),
    Struct(Vec<Value>),
}
impl Value {
    fn to_bits(&self, size: VectorSize) -> Bits {
        match self {
            Value::Float(_) => todo!(),
            Value::Int(_) => todo!(),
            Value::UInt(value) => Bits::from_u64(size, *value),
            Value::Bits(bits) => bits.clone(),
            Value::Lists(_) => todo!(),
            Value::Struct(_) => todo!(),
        }
    }
}

impl Points {
    pub fn len(&self) -> usize {
        match self {
            Self::Floats(i) => i.len(),
            Self::Ints(i) => i.len(),
            Self::UInts(i) => i.len(),
            Self::Bits(b, stride) => (b.size().get() / stride.get()) as usize,
            Self::Lists(_, offsets) => offsets.len() - 1,
            Self::Struct(_, length) => *length,
        }
    }

    fn get(&self, idx: usize) -> Value {
        match self {
            Points::Floats(i) => Value::Float(i[idx]),
            Points::Ints(i) => Value::Int(i[idx]),
            Points::UInts(i) => Value::UInt(i[idx]),
            Points::Bits(..) => todo!(),
            Points::Lists(..) => todo!(),
            Points::Struct(..) => todo!(),
        }
    }
}

#[derive(Clone)]
pub struct RandomPoints {
    pub seed: u64,
    pub length: usize,
}

impl RandomPoints {
    fn len(&self) -> usize {
        self.length
    }

    fn collect(&self) -> Points {
        let rng = <SmallRng as rand::SeedableRng>::seed_from_u64(self.seed);
        Points::UInts(rng.random_iter().take(self.length).collect())
    }
}

#[derive(Clone)]
pub enum LazyPoints {
    Constant(Points),
    Random(RandomPoints),
    Run(Arc<LazyRun>),
}

#[derive(Clone)]
pub struct LazyRun {
    pub design: Arc<LazyDesign>,
    pub steps: Vec<Step>,
    pub aggregation: RunAgg,
}
impl LazyRun {
    fn num_traces(&self) -> Result<usize, ()> {
        let mut current_num_traces = 1;
        for step in self.steps.iter() {
            if let Some(step_num_traces) = step.num_traces() {
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

pub struct LazyDesign {
    pub sources: Vec<PathBuf>,
    pub top_level_module: Option<String>,
}

#[derive(Clone)]
pub struct SignalRef {
    pub inner: Vec<String>,
}

#[derive(Clone)]
pub struct Time {
    pub value: u64,
    pub unit: TimeUnit,
}

#[derive(Clone)]
pub enum TimeUnit {
    Femptoseconds,
    Picoseconds,
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
}

#[derive(Clone)]
pub enum Step {
    TraceStart,
    TraceStop,
    SetSignal(SignalRef, Arc<LazyPoints>),
    Repeat(usize),
    RunFor(Time),
}
impl Step {
    fn num_traces(&self) -> Option<Result<usize, ()>> {
        match self {
            Self::TraceStart => None,
            Self::TraceStop => None,
            Self::SetSignal(_, expr) => Some(expr.len()),
            Self::Repeat(n) => Some(Ok(*n)),
            Self::RunFor(_) => None,
        }
    }

    fn map_lazy_points<E>(&self, f: &mut dyn FnMut(&LazyPoints) -> Result<(), E>) -> Result<(), E> {
        match self {
            Step::TraceStart | Step::TraceStop | Step::Repeat(_) | Step::RunFor(_) => Ok(()),
            Step::SetSignal(_, lp) => f(lp.as_ref()),
        }
    }
}

#[derive(Clone)]
pub enum RunAgg {
    None,
    HammingWeight,
    HammingDistance,
}

pub struct TraceRef(usize);

impl TraceRef {
    pub fn extract(&self, state: &mut DesignState) -> Trace {
        let plugins = match &mut *state {
            DesignState::Interpretted(s) => &mut s.plugins,
            DesignState::Compiled(s) => &mut s.plugins,
        };
        let trace = plugins.remove(self.0);
        let trace = trace as Box<dyn std::any::Any>;
        let trace = trace.downcast::<vogls_trace::TracePlugin>().unwrap();
        Trace {
            trace: trace.trace,
            time_offsets: trace.time_offsets,
        }
    }
}

impl Step {
    pub fn execute(
        &self,
        design: &Design,
        state: &mut DesignState,
        inputs: &VgHashMap<LazyPointsKey, Points>,
        trace_ref: &mut Option<TraceRef>,
        traces: &mut Vec<Trace>,
        rid: usize,
    ) -> Result<(), ()> {
        match self {
            Self::TraceStart => {
                if trace_ref.is_some() {
                    panic!("double trace");
                }
                state.plugins_mut()[0] = Box::new(TracePlugin::new(design));
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
            Self::Repeat(_) => Ok(()),
            Self::SetSignal(signal_ref, lp) => {
                let mut sid = design.elab_table.roots()[0];
                for n in &signal_ref.inner {
                    let Some(ident) = design.ident_table.get(n) else {
                        return Err(());
                    };
                    let Some(ssid) = design.elab_table.resolve(sid, ident) else {
                        return Err(());
                    };
                    sid = ssid;
                }
                let Symbol::Net(net_symbol) = &design.elab_table[sid].content else {
                    return Err(());
                };
                let (signal, _slice) = match &net_symbol.net {
                    NetValue::Signal(s) => s.blocking_drive_signal(),
                    NetValue::Constant(_) => todo!(),
                };
                let value = inputs[&LazyPointsKey(NonNull::from_ref(lp.as_ref()))].get(rid);
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
                .map_err(|_| ()),
        }
    }
}

impl LazyRun {
    pub fn compile_design(&self) -> Result<Design, ()> {
        let mut plugins = Vec::new();
        if matches!(
            self.aggregation,
            RunAgg::HammingWeight | RunAgg::HammingDistance
        ) {
            plugins.push(Box::new(TracePlugin::default()) as _);
        }
        let defines = None;
        let debug_symbols = false;
        let opt = false;
        let four_value_logic = false;
        let compile = false;
        let mut ectx = ExecutionContext {
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            defines: defines.unwrap_or_default(),
            emit_hierarchy: false,
            emit_unoptimized_ir: false,
            emit_ir: false,
            emit_vm: false,
            emit_process_stats: false,
            itrace: false,
            stats: false,
            debug_symbols,
            time: 0,
            opt: vogls::ir::optimize::OptFlags {
                opt_rounds: if opt { 2 } else { 0 },
                constant_propagation: opt,
                deadcode_elimination: opt,
                common_subexpr_elim: opt,
                peephole: opt,
            },
            logic_mode: if four_value_logic {
                LogicMode::FourValue
            } else {
                LogicMode::TwoValue
            },
            no_run: false,
            vcd: None,
            sdf: None,
            compile,
            output_source: None,
            timings: false,
            print_optimized_fuse_signals: false,
            print_round_fuse_signals: false,
            print_unoptimized_fuse_signals: false,
        };

        let paths = self
            .design
            .sources
            .iter()
            .map(|p| p.as_path())
            .collect::<Vec<_>>();
        Design::new(
            &paths,
            &mut TimerStack::new(false),
            self.design.top_level_module.as_deref(),
            &mut ectx,
            plugins,
        )
        .map_err(|_| ())
    }
}

impl RunAgg {
    pub fn collect(&self, _design: &Design, _state: &mut DesignState, traces: &[Trace]) -> Value {
        match self {
            Self::None => todo!(),
            Self::HammingWeight => todo!(),
            Self::HammingDistance => {
                let Some(trace) = traces.first() else {
                    panic!("no trace yet");
                };
                let (times, hds) = trace.hamming_distance();
                let length = times.len();
                Value::Lists(Points::Struct(
                    [
                        ("times".to_string(), Points::UInts(times.into())),
                        ("hds".to_string(), Points::UInts(hds.into())),
                    ]
                    .into(),
                    length,
                ))
            }
        }
    }
}

pub struct Context {
    pool: ThreadPool,
}

impl Context {
    pub fn new(num_threads: Option<usize>) -> Self {
        let pool = match num_threads {
            None => ThreadPoolBuilder::new()
                .use_current_thread()
                .num_threads(1)
                .build(),
            Some(0) => ThreadPoolBuilder::new().build(),
            Some(n) => ThreadPoolBuilder::new().num_threads(n).build(),
        }
        .expect("failed to build threadpool");
        Self { pool }
    }
}

impl LazyPoints {
    pub fn len(&self) -> Result<usize, ()> {
        match self {
            Self::Constant(p) => Ok(p.len()),
            Self::Random(p) => Ok(p.len()),
            Self::Run(k) => k.num_traces(),
        }
    }

    pub fn collect(&self, ctx: &Context) -> Result<Points, ()> {
        match self {
            Self::Constant(points) => Ok(points.clone()),
            Self::Random(points) => Ok(points.collect()),
            Self::Run(k) => {
                let num_traces = k.num_traces()?;
                // @TODO: Insert caching here.
                let design = k.compile_design()?;
                let mut state = design.initial_state.clone();

                let mut inputs = VgHashMap::default();
                for step in &k.steps {
                    step.map_lazy_points(&mut |lp| {
                        inputs.insert(LazyPointsKey(NonNull::from_ref(lp)), lp.collect(ctx)?);
                        Ok(())
                    })?;
                }

                // Execute all steps that are shared between traces first on a single trace.
                let mut prelude_steps = 0;
                while let Some(step) = k.steps.get(prelude_steps) {
                    match step.num_traces() {
                        Some(Err(_)) => return Err(()),
                        None => {}
                        Some(Ok(n)) => {
                            if n != 1 {
                                break;
                            }
                        }
                    }

                    if matches!(step, Step::TraceStart | Step::TraceStop) {
                        break;
                    }

                    step.execute(&design, &mut state, &inputs, &mut None, &mut Vec::new(), 0)?;
                    prelude_steps += 1;
                }

                let results = ctx.pool.install(|| {
                    (0..num_traces)
                        .into_par_iter()
                        .map(|rid| {
                            let mut state = state.clone();
                            let mut trace_ref = None;
                            let mut traces = Vec::new();

                            for step in &k.steps[prelude_steps..] {
                                step.execute(
                                    &design,
                                    &mut state,
                                    &inputs,
                                    &mut trace_ref,
                                    &mut traces,
                                    rid,
                                )?;
                            }

                            if let Some(trace_ref) = trace_ref.take() {
                                traces.push(trace_ref.extract(&mut state));
                            }

                            Ok(k.aggregation.collect(&design, &mut state, &traces))
                        })
                        .collect::<Result<Vec<Value>, ()>>()
                })?;

                // @TODO: Deal with zero-width points
                let mut times = Vec::new();
                let mut hds = Vec::new();
                let mut offsets = Vec::new();

                let mut offset = 0;
                for result in results {
                    let Value::Lists(values) = result else {
                        todo!();
                    };
                    let Points::Struct(values, _) = values else {
                        todo!();
                    };
                    assert_eq!(values.len(), 2);
                    let mut values = values.iter();

                    let t = values.next().unwrap();
                    let h = values.next().unwrap();

                    let (_, Points::UInts(t)) = &t else {
                        todo!();
                    };
                    let (_, Points::UInts(h)) = &h else {
                        todo!();
                    };

                    times.extend_from_slice(t.as_ref());
                    hds.extend_from_slice(h.as_ref());
                    offsets.push(offset);
                    offset += t.len();
                }
                offsets.push(offset);

                Ok(Points::Lists(
                    Arc::new(Points::Struct(
                        [
                            ("times".into(), Points::UInts(times.into())),
                            ("hds".into(), Points::UInts(hds.into())),
                        ]
                        .into(),
                        offsets.len() - 1,
                    )),
                    offsets.into(),
                ))
            }
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct LazyPointsKey(NonNull<LazyPoints>);
unsafe impl Sync for LazyPointsKey {}
unsafe impl Send for LazyPointsKey {}
