#[pyo3::pymodule]
mod vogls {
    use std::io::{stderr, stdout};
    use std::num::NonZeroU32;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use pyo3::exceptions::{PyException, PyTypeError, PyValueError};
    use pyo3::{FromPyObject, IntoPyObjectExt, PyAny, PyResult, prelude::*};
    use vogls::design::DesignState;
    use vogls::utils::{IndexMap, VgHashSet};
    use vogls::{BitsFormatOptions, SimulationIo, VectorSize};

    use vogls_plan::array::{Array, DslLazyArray, LazyArray};
    use vogls_plan::compute::{ComputeNode, GraphItem, display_dot};
    use vogls_plan::design::TimeUnit;
    use vogls_plan::dsl::DslNode;
    use vogls_plan::entropy::Entropy;
    use vogls_plan::mutual_information::MutualInformation;
    use vogls_plan::output::{DslLazyOutput, DslPlanComponent, Output};
    use vogls_plan::plan::{DslLazyPlan, LazyPlan, Plan};
    use vogls_plan::random::RandomBits;
    use vogls_plan::run::DslLazyStep;
    use vogls_plan::run_vector::{DslRunVector, LazyRunVector, RunVector};
    use vogls_plan::ttest::TTest;
    use vogls_plan::typing::{PlanType, Type, TypeKind};
    use vogls_plan::value::{DslLazyValue, LazyValue, Value};
    use vogls_plan::window_sum::WindowSum;

    #[pyo3::pyclass(frozen)]
    #[repr(transparent)]
    pub struct Bits {
        inner: vogls::Bits,
    }

    #[pyo3::pyclass(frozen)]
    #[repr(transparent)]
    pub struct SignalRef {
        inner: vogls::RtSignalKey,
    }

    #[pyo3::pyclass]
    pub struct Design {
        inner: vogls::design::Design,
        state: Snapshot,
    }

    #[pyo3::pyclass]
    #[repr(transparent)]
    pub struct Snapshot {
        inner: Arc<Mutex<vogls::design::DesignState>>,
    }

    #[pyo3::pyclass]
    pub struct DesignBuilder {
        inner: Arc<Mutex<::vogls::DesignBuilder>>,
    }

    #[pyo3::pyclass]
    pub struct ParsedDesign {
        inner: ::vogls::sync::ParsedDesign,
    }

    #[pyo3::pyclass]
    pub struct ElaboratedDesign {
        inner: ::vogls::sync::ElaboratedDesign,
    }

    #[pyo3::pyclass]
    pub struct LoweredDesign {
        inner: ::vogls::LoweredDesign,
    }

    #[pyo3::pyclass(frozen, eq, eq_int, from_py_object)]
    #[derive(PartialEq, Clone)]
    pub enum LogicMode {
        TwoValue,
        FourValue,
    }

    impl From<::vogls::LogicMode> for LogicMode {
        fn from(value: vogls::LogicMode) -> Self {
            match value {
                vogls::LogicMode::TwoValue => Self::TwoValue,
                vogls::LogicMode::FourValue => Self::FourValue,
            }
        }
    }
    impl Into<::vogls::LogicMode> for LogicMode {
        fn into(self) -> vogls::LogicMode {
            match self {
                Self::TwoValue => vogls::LogicMode::TwoValue,
                Self::FourValue => vogls::LogicMode::FourValue,
            }
        }
    }

    #[pymethods]
    impl DesignBuilder {
        #[new]
        pub fn new() -> Self {
            DesignBuilder {
                inner: Arc::new(Mutex::new(::vogls::DesignBuilder::new())),
            }
        }

        pub fn add_source(&mut self, path: PathBuf) -> PyResult<()> {
            self.inner
                .lock()
                .unwrap()
                .add_source(&path)
                .map_err(|_| PyValueError::new_err("failed to tokenize"))?;
            Ok(())
        }
        pub fn add_source_str(&mut self, content: String) -> PyResult<()> {
            self.inner
                .lock()
                .unwrap()
                .add_source_str(content)
                .map_err(|_| PyValueError::new_err("failed to tokenize"))?;
            Ok(())
        }

        pub fn parse(&self) -> PyResult<ParsedDesign> {
            let inner = self.inner.lock().unwrap().clone();
            let design = ::vogls::sync::ParsedDesign::parse(inner)
                .map_err(|_| PyValueError::new_err("failed to parse"))?;
            Ok(ParsedDesign { inner: design })
        }
    }

    #[pymethods]
    impl ParsedDesign {
        #[pyo3(signature = (mode = LogicMode::TwoValue, top_level_module = None))]
        pub fn elaborate(
            &self,
            mode: LogicMode,
            top_level_module: Option<String>,
        ) -> PyResult<ElaboratedDesign> {
            self.inner
                .clone()
                .elaborate(mode.into(), top_level_module)
                .map(|inner| ElaboratedDesign { inner })
                .map_err(|_| PyValueError::new_err("failed to elaborate"))
        }
    }

    #[pymethods]
    impl ElaboratedDesign {
        #[pyo3(signature = ())]
        pub fn lower(&self) -> PyResult<LoweredDesign> {
            self.inner
                .clone()
                .lower(vec![])
                .map(|inner| LoweredDesign { inner })
                .map_err(|_| PyValueError::new_err("failed to elaborate"))
        }
    }

    #[pymethods]
    impl LoweredDesign {
        #[pyo3(signature = ())]
        pub fn to_bytecode(&self) -> PyResult<Design> {
            self.inner
                .clone()
                .to_bytecode()
                .map(|inner| {
                    let state = inner.initial_state().clone();
                    Design {
                        inner,
                        state: Snapshot {
                            inner: Arc::new(Mutex::new(state)),
                        },
                    }
                })
                .map_err(|_| PyValueError::new_err("failed to compile"))
        }

        #[pyo3(signature = ())]
        pub fn compile(&self) -> PyResult<Design> {
            self.inner
                .clone()
                .compile()
                .map(|inner| {
                    let state = inner.initial_state().clone();
                    Design {
                        inner,
                        state: Snapshot {
                            inner: Arc::new(Mutex::new(state)),
                        },
                    }
                })
                .map_err(|_| PyValueError::new_err("failed to compile"))
        }
    }

    #[pyo3::pyclass(frozen)]
    pub struct TraceRef {
        snapshot: Snapshot,
        plugin_idx: usize,
    }

    #[pymethods]
    impl Design {
        // #[new]
        // #[pyo3(signature = (path, top_level_module = None, defines = None, four_value_logic = false, compile = false, trace = false, debug_symbols = false, opt = false))]
        // fn new(
        //     path: PathBuf,
        //     top_level_module: Option<String>,
        //     defines: Option<Vec<String>>,
        //     four_value_logic: bool,
        //     compile: bool,
        //     trace: bool,
        //     debug_symbols: bool,
        //     opt: bool,
        // ) -> PyResult<Self> {
        //     let mut ectx = ExecutionContext {
        //         stdout: Box::new(std::io::stdout()),
        //         stderr: Box::new(std::io::stderr()),
        //         defines: defines.unwrap_or_default(),
        //         emit_hierarchy: false,
        //         emit_unoptimized_ir: false,
        //         emit_ir: false,
        //         emit_vm: false,
        //         emit_process_stats: false,
        //         itrace: false,
        //         stats: false,
        //         debug_symbols,
        //         time: 0,
        //         opt: vogls::ir::optimize::OptFlags {
        //             opt_rounds: if opt { 2 } else { 0 },
        //             constant_propagation: opt,
        //             deadcode_elimination: opt,
        //             common_subexpr_elim: opt,
        //             peephole: opt,
        //         },
        //         logic_mode: if four_value_logic {
        //             LogicMode::FourValue
        //         } else {
        //             LogicMode::TwoValue
        //         },
        //         no_run: false,
        //         vcd: None,
        //         sdf: None,
        //         compile,
        //         output_source: Some(PathBuf::from("out.c")),
        //         timings: false,
        //         print_optimized_fuse_signals: false,
        //         print_round_fuse_signals: false,
        //         print_unoptimized_fuse_signals: false,
        //     };
        //
        //     let mut plugins = Vec::new();
        //     if trace {
        //         plugins.push(Box::new(TracePlugin::default()) as _);
        //     }
        //
        //     let inner = vogls::design::Design::new(
        //         &[path.as_path()],
        //         &mut TimerStack::new(false),
        //         top_level_module.as_deref(),
        //         &mut ectx,
        //         plugins,
        //     )
        //     .map_err(|e| PyException::new_err(e.to_string()))?;
        //     let snapshot = Snapshot {
        //         inner: Arc::new(Mutex::new(inner.initial_state.clone())),
        //     };
        //     Ok(Self {
        //         inner,
        //         state: snapshot,
        //     })
        // }

        fn run(&self, time: u64) -> PyResult<()> {
            self.inner
                .run_from_state(
                    &mut self.state.inner.lock().unwrap(),
                    &mut SimulationIo {
                        stdout: Box::new(stdout()) as _,
                        stderr: Box::new(stderr()) as _,
                    },
                    time,
                )
                .map_err(|e| PyException::new_err(e.to_string()))
        }

        fn run_from(&self, py: Python<'_>, snapshot: Py<Snapshot>, time: u64) -> PyResult<()> {
            let snapshot = snapshot.borrow(py).inner.clone();
            py.detach(|| {
                self.inner
                    .run_from_state(
                        &mut snapshot.lock().unwrap(),
                        &mut SimulationIo {
                            stdout: Box::new(stdout()) as _,
                            stderr: Box::new(stderr()) as _,
                        },
                        time,
                    )
                    .map_err(|e| PyException::new_err(e.to_string()))
            })
        }

        fn snapshot(&self) -> PyResult<Snapshot> {
            Ok(Snapshot {
                inner: self.state.inner.clone(),
            })
        }

        // fn signals_resolve(&self, name: Vec<String>) -> PyResult<SignalRef> {
        //     let mut sid = self.inner.elab_table.roots()[0];
        //     for n in &name {
        //         let Some(ident) = self.inner.ident_table.get(n) else {
        //             return Err(PyException::new_err("signal not found"));
        //         };
        //         let Some(ssid) = self.inner.elab_table.resolve(sid, ident) else {
        //             return Err(PyException::new_err("signal not found"));
        //         };
        //         sid = ssid;
        //     }
        //     let Symbol::Net(net_symbol) = &self.inner.elab_table[sid].content else {
        //         return Err(PyException::new_err("not a signal"));
        //     };
        //     let (signal, _slice) = match &net_symbol.net {
        //         NetValue::Signal(s) => s.blocking_drive_signal(),
        //         NetValue::Constant(_) => todo!(),
        //     };
        //     Ok(SignalRef {
        //         inner: self.inner.get_rt_signal(signal),
        //     })
        // }
        //
        // fn signals_set(
        //     &self,
        //     py: Python<'_>,
        //     snapshot: Py<Snapshot>,
        //     signal: Py<SignalRef>,
        //     value: Py<Bits>,
        // ) -> PyResult<()> {
        //     let snapshot = snapshot.borrow(py);
        //     let mut snapshot = snapshot.inner.lock().unwrap();
        //     self.inner
        //         .set_signal(&mut snapshot, signal.get().inner, &value.get().inner);
        //     Ok(())
        // }
        //
        // fn signals_get(
        //     &self,
        //     py: Python<'_>,
        //     snapshot: Py<Snapshot>,
        //     signal: Py<SignalRef>,
        // ) -> PyResult<Bits> {
        //     let snapshot = snapshot.borrow(py);
        //     let snapshot = snapshot.inner.lock().unwrap();
        //     let bits = self.inner.get_signal(&snapshot, signal.get().inner);
        //     Ok(Bits { inner: bits })
        // }
        //
        // pub fn trace(&self, py: Python<'_>, snapshot: Py<Snapshot>) -> TraceRef {
        //     let snapshot = snapshot.borrow(py);
        //     let design = &self.inner;
        //     let mut state = snapshot.inner.lock().unwrap();
        //     state.plugins_mut()[0] = Box::new(TracePlugin::new(design));
        //     TraceRef {
        //         snapshot: Snapshot {
        //             inner: snapshot.inner.clone(),
        //         },
        //         plugin_idx: 0,
        //     }
        // }
    }

    #[pymethods]
    impl Snapshot {
        pub fn fork(&self) -> Self {
            let state = self.inner.lock().unwrap().clone();
            Self {
                inner: Arc::new(Mutex::new(state)),
            }
        }

        pub fn time(&self) -> u64 {
            self.inner.lock().unwrap().runtime().time
        }
    }

    #[pymethods]
    impl Bits {
        #[staticmethod]
        pub fn from_binary(size: vogls::VectorSize, value: &str) -> PyResult<Self> {
            Ok(Self {
                inner: vogls::Bits::parse_binary(value, size)
                    .map_err(|_| PyValueError::new_err("invalid binary"))?,
            })
        }

        #[staticmethod]
        pub fn from_hex(size: vogls::VectorSize, value: &str) -> PyResult<Self> {
            Ok(Self {
                inner: vogls::Bits::parse_hexadecimal(value, size)
                    .map_err(|_| PyValueError::new_err("invalid binary"))?,
            })
        }

        #[staticmethod]
        #[pyo3(signature = (size, two_value_logic=false))]
        pub fn random(
            py: pyo3::Python<'_>,
            size: vogls::VectorSize,
            two_value_logic: bool,
        ) -> Self {
            py.detach(|| Self {
                inner: vogls::bits::random::rand_bits(
                    size,
                    if two_value_logic {
                        vogls::bits::Mode::TwoValue
                    } else {
                        vogls::bits::Mode::FourValue
                    },
                ),
            })
        }

        pub fn to_hex(&self) -> String {
            self.inner
                .display(&BitsFormatOptions {
                    prefix: false,
                    base: vogls::BitsFormatBase::UpperHex,
                    separator: Some('_'),
                    align: None,
                    fill: '0',
                    width: vogls::BitsFormatWidth::Expand,
                })
                .to_string()
        }

        pub fn slice(&self, offset: u32, size: VectorSize) -> Self {
            Self {
                inner: self.inner.logical_shift_right(offset).truncate(size),
            }
        }

        #[getter]
        pub fn size(&self) -> VectorSize {
            self.inner.size()
        }
    }

    #[pymethods]
    impl TraceRef {
        pub fn print(&self) {
            let state = self.snapshot.inner.lock().unwrap();
            let plugin = match &*state {
                DesignState::Interpretted(s) => &s.plugins[0],
                DesignState::Compiled(s) => &s.plugins[0],
            };
            let trace = (plugin.as_ref() as &dyn std::any::Any)
                .downcast_ref::<vogls_trace::TracePlugin>()
                .unwrap();

            for i in 0..trace.time_offsets.len() - 1 {
                println!(
                    "[T={}] {} events",
                    trace.time_offsets[i].0,
                    trace.time_offsets[i + 1].1 - trace.time_offsets[i].1
                );
            }
        }

        pub fn extract(&self) -> Trace {
            let mut state = self.snapshot.inner.lock().unwrap();
            let plugins = match &mut *state {
                DesignState::Interpretted(s) => &mut s.plugins,
                DesignState::Compiled(s) => &mut s.plugins,
            };
            let trace = plugins.remove(self.plugin_idx);
            let trace = trace as Box<dyn std::any::Any>;
            let trace = trace.downcast::<vogls_trace::TracePlugin>().unwrap();
            Trace(vogls_trace::Trace {
                trace: trace.trace,
                time_offsets: trace.time_offsets,
            })
        }
    }

    #[pyo3::pyclass(frozen)]
    pub struct Trace(vogls_trace::Trace);

    #[pyo3::pymethods]
    impl Trace {
        pub fn hamming_distance(&self, py: pyo3::Python<'_>) -> pyo3::Py<pyo3::types::PyTuple> {
            let (times, hds) = py.detach(|| self.0.hamming_distance());
            let times = pyo3::types::PyList::new(py, times).unwrap();
            let hds = pyo3::types::PyList::new(py, hds).unwrap();
            pyo3::types::PyTuple::new(py, vec![times, hds])
                .unwrap()
                .into()
        }
    }

    #[pyo3::pyclass(frozen)]
    pub struct PyLazyDesign(Arc<vogls_plan::design::LazyDesign>);

    #[pymethods]
    impl PyLazyDesign {
        #[new]
        #[pyo3(signature = (paths, top_level_module = None))]
        fn new(paths: Vec<PathBuf>, top_level_module: Option<String>) -> Self {
            Self(Arc::new(vogls_plan::design::LazyDesign {
                sources: paths,
                top_level_module,
                trace: true,
                handles: VgHashSet::default(),
            }))
        }

        pub fn run(&self) -> PyRun {
            PyRun(Arc::new(Mutex::new(self.0.clone().run())))
        }
    }

    #[pyo3::pyclass(frozen)]
    pub struct PyRun(Arc<Mutex<vogls_plan::run::DslLazyRun>>);

    #[pymethods]
    impl PyRun {
        pub fn run_for(&self, time: u64) -> Self {
            self.0
                .lock()
                .unwrap()
                .steps
                .push(DslLazyStep::RunFor(vogls_plan::design::Time {
                    value: time,
                    unit: TimeUnit::Femptoseconds,
                }));
            Self(self.0.clone())
        }

        pub fn repeat(&self, n: usize) -> Self {
            self.0.lock().unwrap().steps.push(DslLazyStep::Repeat(n));
            Self(self.0.clone())
        }

        pub fn trace_start(&self) -> Self {
            self.0.lock().unwrap().steps.push(DslLazyStep::TraceStart);
            Self(self.0.clone())
        }
        pub fn trace_stop(&self) -> Self {
            self.0.lock().unwrap().steps.push(DslLazyStep::TraceStop);
            Self(self.0.clone())
        }

        pub fn set_signal(&self, name: Vec<String>, value: Py<PyLazyArray>) -> Self {
            self.0.lock().unwrap().steps.push(DslLazyStep::SetSignal(
                vogls_plan::design::SignalRef { inner: name },
                value.get().0.clone(),
            ));
            Self(self.0.clone())
        }

        pub fn hamming_distance(&self, name: String) -> Self {
            let mut inner = self.0.lock().unwrap();
            inner.hamming_distance(name);
            Self(self.0.clone())
        }

        pub fn finish(&self, py: Python<'_>) -> PyResult<Py<PyLazyPlan>> {
            let run = self.0.lock().unwrap().clone();
            Py::new(py, PyLazyPlan(run.finish()))
        }
    }

    fn lazy_compute<Dsl: DslNode, Lazy: ComputeNode + GraphItem>(
        dsl: &Dsl,
    ) -> PyResult<Lazy::Output> {
        let (lazy_key, mut graph) = vogls_plan::dsl::convert(dsl)?;
        let ctx = vogls_plan::compute::ComputeContext::new(None);
        let result = vogls_plan::compute::compute::<Lazy>(lazy_key, &mut graph, &ctx)?;
        Ok(result)
    }
    fn lazy_dot_string<Dsl: DslNode, Lazy: ComputeNode + GraphItem>(dsl: &Dsl) -> PyResult<String> {
        let (lazy_key, graph) = vogls_plan::dsl::convert(dsl)?;
        let result = display_dot(&[lazy_key], &graph).to_string();
        Ok(result)
    }

    #[pyo3::pyclass(frozen)]
    pub struct PyLazyPlan(DslLazyPlan);
    #[pyo3::pyclass(frozen)]
    pub struct PyLazyRunVector(DslRunVector);
    #[pyo3::pyclass(frozen)]
    pub struct PyLazyArray(DslLazyArray);
    #[pyo3::pyclass(frozen)]
    pub struct PyLazyValue(DslLazyValue);
    #[pyo3::pyclass(frozen)]
    pub struct PyPlan(Plan);
    #[pyo3::pyclass(frozen)]
    pub struct PyRunVector(RunVector);
    #[pyo3::pyclass(frozen)]
    pub struct PyArray(Array);
    #[pyo3::pyclass(frozen)]
    pub struct PyValue(Value);

    pub struct PyOutput(Output);
    pub struct PyLazyOutput(DslLazyOutput);

    impl<'py> IntoPyObject<'py> for PyOutput {
        type Target = PyAny;
        type Output = Bound<'py, PyAny>;
        type Error = PyErr;

        fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
            Ok(match self.0 {
                Output::Value(v) => PyValue(v).into_pyobject(py)?.into_any(),
                Output::Array(a) => PyArray(a).into_pyobject(py)?.into_any(),
                Output::Plan(p) => PyPlan(p).into_pyobject(py)?.into_any(),
                Output::RunVector(r) => PyRunVector(r).into_pyobject(py)?.into_any(),
            })
        }
    }
    impl<'py> IntoPyObject<'py> for PyLazyOutput {
        type Target = PyAny;
        type Output = Bound<'py, PyAny>;
        type Error = PyErr;

        fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
            Ok(match self.0.ty().kind() {
                TypeKind::Value => PyLazyValue(self.0.clone().extract_value())
                    .into_pyobject(py)?
                    .into_any(),
                TypeKind::Array => PyLazyArray(self.0.clone().extract_array())
                    .into_pyobject(py)?
                    .into_any(),
                TypeKind::Plan => PyLazyPlan(self.0.clone().extract_plan())
                    .into_pyobject(py)?
                    .into_any(),
                TypeKind::RunVector => PyLazyRunVector(self.0.clone().extract_run_vector())
                    .into_pyobject(py)?
                    .into_any(),
            })
        }
    }

    impl FromPyObject<'_, '_> for PyOutput {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> Result<Self, Self::Error> {
            if let Ok(b) = obj.cast::<PyValue>() {
                return Ok(PyOutput(Output::Value(b.borrow().0.clone())));
            }
            if let Ok(b) = obj.cast::<PyArray>() {
                return Ok(PyOutput(Output::Array(b.borrow().0.clone())));
            }
            if let Ok(b) = obj.cast::<PyPlan>() {
                return Ok(PyOutput(Output::Plan(b.borrow().0.clone())));
            }
            if let Ok(b) = obj.cast::<PyRunVector>() {
                return Ok(PyOutput(Output::RunVector(b.borrow().0.clone())));
            }
            Err(PyTypeError::new_err(
                "expected one of Value, Array, Plan, or RunVector",
            ))
        }
    }
    impl FromPyObject<'_, '_> for PyLazyOutput {
        type Error = PyErr;

        fn extract(obj: Borrowed<'_, '_, PyAny>) -> Result<Self, Self::Error> {
            if let Ok(b) = obj.cast::<PyLazyValue>() {
                return Ok(PyLazyOutput(b.borrow().0.clone().into()));
            }
            if let Ok(b) = obj.cast::<PyLazyArray>() {
                return Ok(PyLazyOutput(b.borrow().0.clone().into()));
            }
            if let Ok(b) = obj.cast::<PyLazyPlan>() {
                return Ok(PyLazyOutput(b.borrow().0.clone().into()));
            }
            if let Ok(b) = obj.cast::<PyLazyRunVector>() {
                return Ok(PyLazyOutput(b.borrow().0.clone().into()));
            }
            Err(PyTypeError::new_err(
                "expected one of Value, Array, Plan, or RunVector",
            ))
        }
    }

    #[pymethods]
    impl PyLazyPlan {
        pub fn compute(&self) -> PyResult<PyPlan> {
            lazy_compute::<_, LazyPlan>(&self.0).map(PyPlan)
        }
        pub fn to_dot_graph(&self) -> PyResult<String> {
            lazy_dot_string::<_, LazyPlan>(&self.0)
        }

        #[staticmethod]
        pub fn from_dict(dict: Bound<pyo3::types::PyDict>) -> PyResult<Self> {
            let mut components = IndexMap::<String, DslLazyOutput>::default();
            let mut ty = IndexMap::<String, Type>::default();
            for (key, value) in dict.iter() {
                let key = key.str()?;
                let value = value.extract::<PyLazyOutput>()?;
                _ = ty.insert(key.to_string(), value.0.ty().as_ref().clone());
                _ = components.insert(key.to_string(), value.0);
            }
            Ok(Self(vogls_plan::plan::DslLazyPlan {
                ty: Arc::new(Type::Plan(PlanType {
                    components: Arc::new(ty),
                })),
                f: Arc::new(vogls_plan::plan::DslLiteralPlan { components }) as _,
            }))
        }

        pub fn get(&self, key: String) -> PyLazyOutput {
            PyLazyOutput(
                DslPlanComponent {
                    plan: self.0.clone(),
                    key,
                }
                .build(),
            )
        }
    }
    #[pymethods]
    impl PyPlan {
        pub fn lazy(&self) -> PyLazyPlan {
            PyLazyPlan(self.0.to_lazy_dsl())
        }

        pub fn get(&self, key: String) -> PyOutput {
            PyOutput(self.0.components[&key].clone())
        }
    }

    #[pymethods]
    impl PyLazyArray {
        pub fn compute(&self) -> PyResult<PyArray> {
            lazy_compute::<_, LazyArray>(&self.0).map(PyArray)
        }
        pub fn to_dot_graph(&self) -> PyResult<String> {
            lazy_dot_string::<_, LazyArray>(&self.0)
        }

        // pub fn min(&self) -> PyLazyValue {
        //     PyLazyValue(DslLazyValue::Aggregation(
        //         Arc::new(self.0.clone()),
        //         ArrayAgg::Min,
        //     ))
        // }

        #[staticmethod]
        pub fn ttest(lhs: Bound<PyLazyRunVector>, rhs: Bound<PyLazyRunVector>) -> Self {
            PyLazyArray(
                TTest {
                    lhs: lhs.get().0.clone().into(),
                    rhs: rhs.get().0.clone().into(),
                }
                .build(),
            )
        }
        #[staticmethod]
        pub fn mutual_information(
            lhs: Bound<PyLazyRunVector>,
            rhs: Bound<PyLazyRunVector>,
        ) -> Self {
            PyLazyArray(
                MutualInformation {
                    lhs: lhs.get().0.clone().into(),
                    rhs: rhs.get().0.clone().into(),
                }
                .build(),
            )
        }

        #[staticmethod]
        #[pyo3(signature = (length, width, seed = None))]
        pub fn random_bits(length: usize, width: NonZeroU32, seed: Option<u64>) -> Self {
            Self(
                RandomBits {
                    length,
                    width,
                    seed: seed.unwrap_or(0),
                }
                .build(),
            )
        }
    }
    #[pymethods]
    impl PyArray {
        pub fn lazy(&self) -> PyLazyArray {
            PyLazyArray(self.0.to_lazy_dsl())
        }

        #[staticmethod]
        pub fn from_f64s(arr: Vec<f64>) -> Self {
            Self(Array::Floats(arr.into()))
        }

        pub fn as_list(&self, py: pyo3::Python<'_>) -> PyResult<pyo3::Py<pyo3::types::PyList>> {
            use pyo3::types::PyList;
            use vogls_plan::array::Array;
            Ok(match &self.0 {
                Array::Floats(items) => PyList::new(py, items.iter())?.into(),
                Array::Ints(items) => PyList::new(py, items.iter())?.into(),
                Array::UInts(items) => PyList::new(py, items.iter())?.into(),
                Array::Bits(..) => todo!(),
            })
        }
    }

    #[pymethods]
    impl PyLazyValue {
        pub fn compute(&self) -> PyResult<PyValue> {
            lazy_compute::<_, LazyValue>(&self.0).map(PyValue)
        }
        pub fn to_dot_graph(&self) -> PyResult<String> {
            lazy_dot_string::<_, LazyValue>(&self.0)
        }
    }
    #[pymethods]
    impl PyValue {
        pub fn lazy(&self) -> PyLazyValue {
            PyLazyValue(self.0.to_lazy_dsl())
        }

        #[staticmethod]
        pub fn from_float(v: f64) -> Self {
            Self(Value::Float(v))
        }
        #[staticmethod]
        pub fn from_unsigned_int(v: u64) -> Self {
            Self(Value::UInt(v))
        }
        #[staticmethod]
        pub fn from_signed_int(v: i64) -> Self {
            Self(Value::Int(v))
        }

        pub fn repeat(&self, n: usize) -> PyArray {
            PyArray(self.0.repeat(n))
        }

        pub fn as_pyany(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
            match &self.0 {
                Value::Float(v) => v.into_py_any(py),
                Value::Int(v) => v.into_py_any(py),
                Value::UInt(v) => v.into_py_any(py),
                Value::Bits(_) => todo!(),
            }
        }

        pub fn extract_float(&self) -> PyResult<f64> {
            match &self.0 {
                Value::Float(v) => Ok(*v),
                _ => todo!(),
            }
        }
    }

    #[pyo3::pymethods]
    impl PyLazyRunVector {
        pub fn compute(&self) -> PyResult<PyRunVector> {
            lazy_compute::<_, LazyRunVector>(&self.0).map(PyRunVector)
        }
        pub fn to_dot_graph(&self) -> PyResult<String> {
            lazy_dot_string::<_, LazyRunVector>(&self.0)
        }

        pub fn window_sum(
            &self,
            by: Bound<PyLazyRunVector>,
            width: u64,
            start: u64,
            end: u64,
        ) -> PyLazyRunVector {
            PyLazyRunVector(
                WindowSum {
                    on: self.0.clone(),
                    by: by.get().0.clone(),
                    width,
                    start,
                    end,
                }
                .build(),
            )
        }

        pub fn entropy(&self) -> PyLazyArray {
            PyLazyArray(
                Entropy {
                    src: self.0.clone(),
                }
                .build(),
            )
        }
    }

    #[pyo3::pymethods]
    impl PyRunVector {
        pub fn as_list(&self, py: pyo3::Python<'_>) -> PyResult<pyo3::Py<pyo3::types::PyList>> {
            let lst = pyo3::types::PyList::new(
                py,
                self.0
                    .array_iter()
                    .map(|arr| PyArray(arr).as_list(py))
                    .collect::<PyResult<Vec<_>>>()?,
            )?;
            Ok(lst.into())
        }
    }
}
