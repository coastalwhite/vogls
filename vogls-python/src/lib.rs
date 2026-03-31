pub mod trace;

#[pyo3::pymodule]
mod vogls {
    use std::io::{stderr, stdout};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use pyo3::exceptions::{PyException, PyValueError};
    use pyo3::{PyResult, prelude::*};
    use vogls::design::DesignState;
    use vogls::utils::TimerStack;
    use vogls::{
        BitsFormatOptions, ExecutionContext, LogicMode, SimulationIo, VSymbol, VectorSize,
    };

    use crate::trace::TracePlugin;

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
        #[pyo3(signature = (path, top_level_module = None, defines = None, four_value_logic = false, compile = false, trace = false))]
        fn new(
            path: PathBuf,
            top_level_module: Option<String>,
            defines: Option<Vec<String>>,
            four_value_logic: bool,
            compile: bool,
            trace: bool,
        ) -> PyResult<Self> {
            let mut ectx = ExecutionContext {
                stdout: Box::new(std::io::stdout()),
                stderr: Box::new(std::io::stderr()),
                defines: defines.unwrap_or_default(),
                emit_hierarchy: false,
                emit_unoptimized_ir: false,
                emit_ir: false,
                emit_vm: false,
                itrace: false,
                stats: false,
                debug_symbols: false,
                time: 0,
                opt: vogls::ir::optimize::OptFlags {
                    opt_rounds: 0,
                    ..Default::default()
                },
                logic_mode: if four_value_logic {
                    LogicMode::FourValue
                } else {
                    LogicMode::TwoValue
                },
                no_run: false,
                vcd: None,
                compile,
                output_source: None,
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
            let VSymbol::Net(net_symbol) = &self.inner.elab_table[sid].content else {
                return Err(PyException::new_err("not a signal"));
            };
            let (signal, _slice) = net_symbol.net.blocking_drive_signal();
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
            match &mut *state {
                DesignState::Interpretted(s) => s.plugins[0] = Box::new(TracePlugin::new(design)),
                DesignState::Compiled(s) => s.plugins[0] = Box::new(TracePlugin::new(design)),
            }

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
            Self {
                inner: Arc::new(Mutex::new(self.inner.lock().unwrap().clone())),
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
                .downcast_ref::<super::trace::TracePlugin>()
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
            let trace = trace.downcast::<super::trace::TracePlugin>().unwrap();
            Trace {
                trace: trace.trace,
                time_offsets: trace.time_offsets,
            }
        }
    }

    #[pymodule_export]
    pub use super::trace::Trace;
}
