mod anonymous_map;

#[pyo3::pymodule]
mod vogls {
    use std::io::{stderr, stdout};
    use std::num::NonZeroU32;
    use std::ops::Deref;
    use std::path::PathBuf;
    use std::sync::{Arc, LazyLock, Mutex};

    use pyo3::exceptions::{PyException, PyTypeError, PyValueError};
    use pyo3::types::PyDict;
    use pyo3::{FromPyObject, IntoPyObjectExt, PyAny, PyResult, prelude::*};
    use vogls::utils::{IndexMap, VgHashSet};
    use vogls::{BitsFormatOptions, SimulationIo, VectorSize};

    use vogls_plan::agg::{build_array_agg, build_run_vector_agg};
    use vogls_plan::array::{Array, ArrayGet, DslLazyArray, LazyArray};
    use vogls_plan::compute::{ComputeNode, GraphItem, display_dot};
    use vogls_plan::design::TimeUnit;
    use vogls_plan::dsl::DslNode;
    use vogls_plan::entropy::Entropy;
    use vogls_plan::expand::Expand;
    use vogls_plan::map::{build_array_map, build_run_vector_map};
    use vogls_plan::mutual_information::MutualInformation;
    use vogls_plan::output::{DslLazyOutput, DslPlanComponent, Output};
    use vogls_plan::pearson_corr::PearsonCorrelation;
    use vogls_plan::plan::{DslLazyPlan, LazyPlan, Plan};
    use vogls_plan::random::RandomBits;
    use vogls_plan::run::DslLazyStep;
    use vogls_plan::run_vector::{DslRunVector, DslRunVectorRepeatArray, LazyRunVector, RunVector};
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
    impl From<LogicMode> for ::vogls::LogicMode {
        fn from(value: LogicMode) -> vogls::LogicMode {
            match value {
                LogicMode::TwoValue => vogls::LogicMode::TwoValue,
                LogicMode::FourValue => vogls::LogicMode::FourValue,
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

    static CONFIG: LazyLock<Mutex<Config>> = LazyLock::new(|| Mutex::new(Config::load_initial()));

    #[pyo3::pyclass(from_py_object)]
    #[derive(Clone)]
    pub struct Config {
        pub num_threads: Option<usize>,
    }
    #[pyclass]
    #[derive(Default)]
    pub struct ConfigOverrides {
        pub num_threads: Option<Option<usize>>,
    }

    impl Config {
        pub fn load_initial() -> Self {
            Self {
                num_threads: Some(0),
            }
        }

        pub fn with_overrides(&self, overrides: &ConfigOverrides) -> Config {
            Self {
                num_threads: overrides
                    .num_threads
                    .unwrap_or_else(|| self.num_threads.clone()),
            }
        }
    }

    #[pymethods]
    impl Config {
        #[staticmethod]
        pub fn current() -> Self {
            CONFIG.lock().unwrap().deref().clone()
        }

        fn __repr__(&self) -> String {
            let Self { num_threads } = self;
            format!("Config {{ num_threads: {num_threads:?} }}")
        }
    }

    #[pymethods]
    impl ConfigOverrides {
        #[staticmethod]
        pub fn empty() -> Self {
            Self::default()
        }

        fn __repr__(&self) -> String {
            use std::fmt::Write;
            let Self { num_threads } = self;
            let mut s = String::from("PartialConfig { ");
            let mut fst = true;
            if let Some(num_threads) = num_threads {
                if !fst {
                    write!(&mut s, ", ").unwrap();
                }
                write!(&mut s, "num_threads: {num_threads:?}").unwrap();
                #[allow(unused_assignments)]
                {
                    fst = false;
                }
            }
            s.push_str(" }");
            s
        }
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
                    signed: false,
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
        #[pyo3(signature = (paths, top_level_module = None, defines = Vec::new()))]
        fn new(
            paths: Vec<PathBuf>,
            top_level_module: Option<String>,
            defines: Vec<String>,
        ) -> Self {
            Self(Arc::new(vogls_plan::design::LazyDesign {
                sources: paths,
                top_level_module,
                defines,
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
        overrides: &ConfigOverrides,
    ) -> PyResult<Lazy::Output> {
        let (lazy_key, mut graph) = vogls_plan::dsl::convert(dsl)?;
        let gbl_cfg = CONFIG.lock().unwrap();
        let cfg = gbl_cfg.with_overrides(&overrides);
        drop(gbl_cfg);
        let ctx = vogls_plan::compute::ComputeContext::new(cfg.num_threads);
        let result = vogls_plan::compute::compute::<Lazy>(lazy_key, &mut graph, &ctx)?;
        Ok(result)
    }
    fn lazy_dot_string<Dsl: DslNode>(dsl: &Dsl) -> PyResult<String> {
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
    pub struct PyArray(pub(crate) Array);
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
        pub fn compute(&self, overrides: Bound<ConfigOverrides>) -> PyResult<PyPlan> {
            lazy_compute::<_, LazyPlan>(&self.0, overrides.borrow().deref()).map(PyPlan)
        }
        pub fn to_dot_graph(&self) -> PyResult<String> {
            lazy_dot_string::<_>(&self.0)
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
        pub fn compute(&self, overrides: Bound<ConfigOverrides>) -> PyResult<PyArray> {
            lazy_compute::<_, LazyArray>(&self.0, overrides.borrow().deref()).map(PyArray)
        }
        pub fn to_dot_graph(&self) -> PyResult<String> {
            lazy_dot_string::<_>(&self.0)
        }

        pub fn window_sum(
            &self,
            by: Bound<PyLazyArray>,
            width: u64,
            start: u64,
            end: u64,
        ) -> PyResult<PyLazyArray> {
            Ok(PyLazyArray(build_array_map(
                WindowSum { width, start, end },
                [self.0.clone(), by.get().0.clone()],
            )?))
        }

        pub fn expand(&self) -> PyResult<PyLazyArray> {
            Ok(PyLazyArray(build_array_map(Expand, [self.0.clone()])?))
        }

        #[staticmethod]
        pub fn ttest(lhs: Bound<PyLazyRunVector>, rhs: Bound<PyLazyRunVector>) -> PyResult<Self> {
            Ok(PyLazyArray(build_run_vector_agg(
                TTest,
                [lhs.get().0.clone(), rhs.get().0.clone()],
            )?))
        }
        #[staticmethod]
        pub fn mutual_information(
            lhs: Bound<PyLazyRunVector>,
            rhs: Bound<PyLazyRunVector>,
        ) -> PyResult<Self> {
            Ok(PyLazyArray(build_run_vector_agg(
                MutualInformation,
                [lhs.get().0.clone(), rhs.get().0.clone()],
            )?))
        }

        #[staticmethod]
        pub fn pearson_corr(
            lhs: Bound<PyLazyRunVector>,
            rhs: Bound<PyLazyRunVector>,
        ) -> PyResult<Self> {
            Ok(PyLazyArray(build_run_vector_agg(
                PearsonCorrelation,
                [lhs.get().0.clone(), rhs.get().0.clone()],
            )?))
        }

        pub fn entropy(&self) -> PyResult<PyLazyValue> {
            Ok(PyLazyValue(build_array_agg(Entropy, [self.0.clone()])?))
        }

        pub fn map<'py>(&self, f: Bound<'py, PyAny>) -> PyResult<PyLazyArray> {
            Ok(PyLazyArray(build_array_map(
                super::anonymous_map::PyAnonymousMap {
                    f: Arc::new(f.unbind()),
                },
                [self.0.clone()],
            )?))
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

        pub fn get(&self, at: usize) -> PyResult<PyLazyValue> {
            Ok(PyLazyValue(build_array_agg(
                ArrayGet(at),
                [self.0.clone()],
            )?))
        }

        pub fn repeat(&self, n: usize) -> PyResult<PyLazyRunVector> {
            let ty = self.0.ty();
            Ok(PyLazyRunVector(DslRunVector {
                ty: Arc::new(Type::RunVector(vogls_plan::typing::RunVectorType {
                    data: ty.data,
                    length: ty.length,
                    width: vogls_plan::typing::RunWidth::Constant(n as u64),
                })),
                f: Arc::new(DslRunVectorRepeatArray(self.0.clone(), n)) as _,
            }))
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

        #[getter]
        pub fn __array_interface__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
            vogls_plan::numpy::to_array_interface(py, &self.0)
        }

        #[staticmethod]
        pub fn from_array_interface<'py>(
            py: Python<'py>,
            array: Bound<'py, PyAny>,
        ) -> PyResult<Self> {
            vogls_plan::numpy::from_array_interface(py, array).map(Self)
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
        pub fn compute(&self, overrides: Bound<ConfigOverrides>) -> PyResult<PyValue> {
            lazy_compute::<_, LazyValue>(&self.0, overrides.borrow().deref()).map(PyValue)
        }
        pub fn to_dot_graph(&self) -> PyResult<String> {
            lazy_dot_string::<_>(&self.0)
        }

        #[staticmethod]
        pub fn ttest(lhs: Bound<PyLazyArray>, rhs: Bound<PyLazyArray>) -> PyResult<Self> {
            Ok(PyLazyValue(build_array_agg(
                TTest,
                [lhs.get().0.clone(), rhs.get().0.clone()],
            )?))
        }
        #[staticmethod]
        pub fn mutual_information(
            lhs: Bound<PyLazyArray>,
            rhs: Bound<PyLazyArray>,
        ) -> PyResult<Self> {
            Ok(PyLazyValue(build_array_agg(
                MutualInformation,
                [lhs.get().0.clone(), rhs.get().0.clone()],
            )?))
        }

        #[staticmethod]
        pub fn pearson_corr(lhs: Bound<PyLazyArray>, rhs: Bound<PyLazyArray>) -> PyResult<Self> {
            Ok(PyLazyValue(build_array_agg(
                PearsonCorrelation,
                [lhs.get().0.clone(), rhs.get().0.clone()],
            )?))
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

        pub fn extract_int(&self) -> PyResult<i64> {
            match &self.0 {
                Value::Int(v) => Ok(*v),
                Value::UInt(v) => Ok(*v as i64),
                _ => todo!(),
            }
        }
    }

    #[pyo3::pymethods]
    impl PyLazyRunVector {
        pub fn compute(&self, overrides: Bound<ConfigOverrides>) -> PyResult<PyRunVector> {
            lazy_compute::<_, LazyRunVector>(&self.0, overrides.borrow().deref()).map(PyRunVector)
        }
        pub fn to_dot_graph(&self) -> PyResult<String> {
            lazy_dot_string::<_>(&self.0)
        }

        pub fn window_sum(
            &self,
            by: Bound<PyLazyRunVector>,
            width: u64,
            start: u64,
            end: u64,
        ) -> PyResult<PyLazyRunVector> {
            Ok(PyLazyRunVector(build_run_vector_map(
                WindowSum { width, start, end },
                [self.0.clone(), by.get().0.clone()],
            )?))
        }

        pub fn expand(&self) -> PyResult<PyLazyRunVector> {
            Ok(PyLazyRunVector(build_run_vector_map(
                Expand,
                [self.0.clone()],
            )?))
        }

        pub fn entropy(&self) -> PyResult<PyLazyArray> {
            Ok(PyLazyArray(build_run_vector_agg(
                Entropy,
                [self.0.clone()],
            )?))
        }

        pub fn map<'py>(&self, f: Bound<'py, PyAny>) -> PyResult<PyLazyRunVector> {
            Ok(PyLazyRunVector(build_run_vector_map(
                super::anonymous_map::PyAnonymousMap {
                    f: Arc::new(f.unbind()),
                },
                [self.0.clone()],
            )?))
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
