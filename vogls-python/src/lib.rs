#[pyo3::pymodule]
mod vogls {
    use std::io::{stderr, stdout};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use pyo3::exceptions::{PyException, PyValueError};
    use pyo3::{IntoPyObjectExt, PyResult, prelude::*};
    use vogls::design::DesignState;
    use vogls::symbol::{NetValue, Symbol};
    use vogls::utils::TimerStack;
    use vogls::{BitsFormatOptions, ExecutionContext, LogicMode, SimulationIo, VectorSize};

    use vogls_plan::{RunAgg, Step, TimeUnit};
    use vogls_trace::TracePlugin;

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

    #[pyo3::pyclass(frozen)]
    pub struct TraceRef {
        snapshot: Snapshot,
        plugin_idx: usize,
    }

    #[pymethods]
    impl Design {
        #[new]
        #[pyo3(signature = (path, top_level_module = None, defines = None, four_value_logic = false, compile = false, trace = false, debug_symbols = false, opt = false))]
        fn new(
            path: PathBuf,
            top_level_module: Option<String>,
            defines: Option<Vec<String>>,
            four_value_logic: bool,
            compile: bool,
            trace: bool,
            debug_symbols: bool,
            opt: bool,
        ) -> PyResult<Self> {
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
                compile,
                output_source: Some(PathBuf::from("out.c")),
                timings: false,
                print_optimized_fuse_signals: false,
                print_round_fuse_signals: false,
                print_unoptimized_fuse_signals: false,
            };

            let mut plugins = Vec::new();
            if trace {
                plugins.push(Box::new(TracePlugin::default()) as _);
            }

            let inner = vogls::design::Design::new(
                &[path.as_path()],
                &mut TimerStack::new(false),
                top_level_module.as_deref(),
                &mut ectx,
                plugins,
            )
            .map_err(|e| PyException::new_err(e.to_string()))?;
            let snapshot = Snapshot {
                inner: Arc::new(Mutex::new(inner.initial_state.clone())),
            };
            Ok(Self {
                inner,
                state: snapshot,
            })
        }

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

        fn signals_resolve(&self, name: Vec<String>) -> PyResult<SignalRef> {
            let mut sid = self.inner.elab_table.roots()[0];
            for n in &name {
                let Some(ident) = self.inner.ident_table.get(n) else {
                    return Err(PyException::new_err("signal not found"));
                };
                let Some(ssid) = self.inner.elab_table.resolve(sid, ident) else {
                    return Err(PyException::new_err("signal not found"));
                };
                sid = ssid;
            }
            let Symbol::Net(net_symbol) = &self.inner.elab_table[sid].content else {
                return Err(PyException::new_err("not a signal"));
            };
            let (signal, _slice) = match &net_symbol.net {
                NetValue::Signal(s) => s.blocking_drive_signal(),
                NetValue::Constant(_) => todo!(),
            };
            Ok(SignalRef {
                inner: self.inner.get_rt_signal(signal),
            })
        }

        fn signals_set(
            &self,
            py: Python<'_>,
            snapshot: Py<Snapshot>,
            signal: Py<SignalRef>,
            value: Py<Bits>,
        ) -> PyResult<()> {
            let snapshot = snapshot.borrow(py);
            let mut snapshot = snapshot.inner.lock().unwrap();
            self.inner
                .set_signal(&mut snapshot, signal.get().inner, &value.get().inner);
            Ok(())
        }

        fn signals_get(
            &self,
            py: Python<'_>,
            snapshot: Py<Snapshot>,
            signal: Py<SignalRef>,
        ) -> PyResult<Bits> {
            let snapshot = snapshot.borrow(py);
            let snapshot = snapshot.inner.lock().unwrap();
            let bits = self.inner.get_signal(&snapshot, signal.get().inner);
            Ok(Bits { inner: bits })
        }

        pub fn trace(&self, py: Python<'_>, snapshot: Py<Snapshot>) -> TraceRef {
            let snapshot = snapshot.borrow(py);
            let design = &self.inner;
            let mut state = snapshot.inner.lock().unwrap();
            state.plugins_mut()[0] = Box::new(TracePlugin::new(design));
            TraceRef {
                snapshot: Snapshot {
                    inner: snapshot.inner.clone(),
                },
                plugin_idx: 0,
            }
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
    pub struct LazyDesign(Arc<vogls_plan::LazyDesign>);

    #[pymethods]
    impl LazyDesign {
        #[new]
        #[pyo3(signature = (paths, top_level_module = None))]
        fn new(paths: Vec<PathBuf>, top_level_module: Option<String>) -> Self {
            Self(Arc::new(vogls_plan::LazyDesign {
                sources: paths,
                top_level_module,
            }))
        }

        pub fn run(&self) -> LazyRun {
            LazyRun(Arc::new(Mutex::new(vogls_plan::LazyRun {
                design: self.0.clone(),
                steps: Vec::new(),
                aggregation: RunAgg::None,
            })))
        }
    }

    #[pyo3::pyclass(frozen)]
    pub struct LazyRun(Arc<Mutex<vogls_plan::LazyRun>>);

    #[pyo3::pyclass(frozen)]
    pub struct LazyPoints(vogls_plan::LazyPoints);

    #[pymethods]
    impl LazyRun {
        pub fn run_for(&self, time: u64) -> Self {
            self.0
                .lock()
                .unwrap()
                .steps
                .push(Step::RunFor(vogls_plan::Time {
                    value: time,
                    unit: TimeUnit::Femptoseconds,
                }));
            Self(self.0.clone())
        }

        pub fn repeat(&self, n: usize) -> Self {
            self.0.lock().unwrap().steps.push(Step::Repeat(n));
            Self(self.0.clone())
        }

        pub fn trace_start(&self) -> Self {
            self.0.lock().unwrap().steps.push(Step::TraceStart);
            Self(self.0.clone())
        }
        pub fn trace_stop(&self) -> Self {
            self.0.lock().unwrap().steps.push(Step::TraceStop);
            Self(self.0.clone())
        }

        pub fn set_signal(&self, name: Vec<String>, value: &LazyPoints) -> Self {
            self.0.lock().unwrap().steps.push(Step::SetSignal(
                vogls_plan::SignalRef { inner: name },
                Arc::new(value.0.clone()),
            ));
            Self(self.0.clone())
        }

        pub fn hamming_distance(&self) -> LazyPoints {
            let mut run = self.0.lock().unwrap().clone();
            run.aggregation = RunAgg::HammingDistance;
            LazyPoints(vogls_plan::LazyPoints::Run(Arc::new(run)))
        }
    }

    #[pymethods]
    impl LazyPoints {
        #[staticmethod]
        #[pyo3(signature = (length, seed = None))]
        pub fn random(length: usize, seed: Option<u64>) -> Self {
            Self(vogls_plan::LazyPoints::Random(vogls_plan::RandomPoints {
                seed: seed.unwrap_or(0),
                length,
            }))
        }

        #[pyo3(signature = (num_threads = Some(0)))]
        pub fn collect(&self, num_threads: Option<usize>) -> PyResult<Points> {
            self.0
                .collect(&vogls_plan::Context::new(num_threads))
                .map(Points)
                .map_err(|_| PyValueError::new_err("error while collecting"))
        }
    }

    #[pyo3::pyclass(frozen)]
    pub struct Points(vogls_plan::Points);

    #[pymethods]
    impl Points {
        pub fn as_list(&self, py: pyo3::Python<'_>) -> PyResult<pyo3::Py<pyo3::types::PyList>> {
            fn points_as_pyany(
                py: pyo3::Python<'_>,
                ps: &vogls_plan::Points,
                at: usize,
            ) -> PyResult<pyo3::Py<pyo3::PyAny>> {
                use pyo3::types::PyDict;
                match ps {
                    vogls_plan::Points::Floats(items) => items[at].into_py_any(py),
                    vogls_plan::Points::Ints(items) => items[at].into_py_any(py),
                    vogls_plan::Points::UInts(items) => items[at].into_py_any(py),
                    vogls_plan::Points::Bits(..) => todo!(),
                    vogls_plan::Points::Lists(points, items) => {
                        points_as_list(py, points, items[at], items[at + 1])?.into_py_any(py)
                    }
                    vogls_plan::Points::Struct(items, _) => {
                        let dict = PyDict::new(py);
                        for (name, ps) in items.as_ref() {
                            dict.set_item(name, points_as_pyany(py, ps, at)?)?;
                        }
                        dict.into_py_any(py)
                    }
                }
            }

            fn points_as_list(
                py: pyo3::Python<'_>,
                ps: &vogls_plan::Points,
                start: usize,
                end: usize,
            ) -> PyResult<pyo3::Py<pyo3::types::PyList>> {
                use pyo3::types::PyList;
                Ok(match ps {
                    vogls_plan::Points::Floats(items) => {
                        PyList::new(py, &items[start..end])?.into()
                    }
                    vogls_plan::Points::Ints(items) => PyList::new(py, &items[start..end])?.into(),
                    vogls_plan::Points::UInts(items) => PyList::new(py, &items[start..end])?.into(),
                    vogls_plan::Points::Bits(..) => todo!(),
                    vogls_plan::Points::Lists(points, items) => {
                        let mut vs = Vec::with_capacity(ps.len());
                        for w in items[start..=end].windows(2) {
                            vs.push(points_as_list(py, points.as_ref(), w[0], w[1])?);
                        }
                        PyList::new(py, vs)?.into()
                    }
                    vogls_plan::Points::Struct(..) => {
                        let mut vs = Vec::with_capacity(ps.len());
                        for i in start..end {
                            vs.push(points_as_pyany(py, ps, i)?);
                        }
                        PyList::new(py, vs)?.into()
                    }
                })
            }

            points_as_list(py, &self.0, 0, self.0.len())
        }
    }
}
