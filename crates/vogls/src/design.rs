use std::io::Write as _;
use std::sync::Arc;

use vogls_codegen::HeapRef;
use vogls_frontend::ident_table::IdentTable;
use vogls_frontend::symbol_table::FrozenSymbolTable;
use vogls_ir::time::TimeResolution;
use vogls_ir::{Bits, GlobalContext, LogicMode, SignalKey, SignalSlice, VectorSize};
use vogls_runtime::plugins::RuntimePluginState;
use vogls_runtime::{RtSignalKey, RuntimeError, RuntimeState};
use vogls_utils::VgHashMap;
pub use vogls_verilog::arena::Arena;
pub use vogls_verilog::elaborate::{VSymbol, VSymbolTable};
pub use vogls_verilog::tokenizer::Macro;
use vogls_world::World;

use crate::elaborated_design::SignalHandle;
use crate::symbol::{SignalRefs, SignalSymbol, SignalValue, Symbol};

#[expect(clippy::large_enum_variant)]
pub enum DesignBackend {
    Bytecode {
        design: vogls_bytecode::Design,
    },
    #[cfg(feature = "native")]
    Cranelift {
        design: vogls_codegen_clif::runtime::ClifDesign,
    },
}

pub struct Design {
    pub(crate) gl: GlobalContext,
    #[expect(unused)]
    pub(crate) ident_table: IdentTable,
    pub(crate) elab_table: FrozenSymbolTable<Symbol>,
    pub(crate) backend: DesignBackend,
    pub(crate) rt_signal_map: VgHashMap<SignalKey, RtSignalKey>,
    pub(crate) signal_mode: Arc<[LogicMode]>,
    pub(crate) signal_to_heap: Arc<[HeapRef]>,
    pub(crate) time_resolution: TimeResolution,
}

#[derive(Clone)]
pub enum DesignState {
    Bytecode(vogls_bytecode::State),
    #[cfg(feature = "native")]
    Cranelift(vogls_codegen_clif::runtime::ClifDesignState),
}

impl DesignState {
    pub fn runtime_mut(&mut self) -> &mut RuntimeState {
        match self {
            DesignState::Bytecode(s) => &mut s.runtime,
            #[cfg(feature = "native")]
            DesignState::Cranelift(s) => &mut s.runtime,
        }
    }
    pub fn runtime(&self) -> &RuntimeState {
        match self {
            DesignState::Bytecode(s) => &s.runtime,
            #[cfg(feature = "native")]
            DesignState::Cranelift(s) => &s.runtime,
        }
    }
    pub fn plugins_mut(&mut self) -> &mut [RuntimePluginState] {
        match self {
            DesignState::Bytecode(s) => &mut s.plugins,
            #[cfg(feature = "native")]
            DesignState::Cranelift(s) => &mut s.plugins,
        }
    }
    pub fn plugins(&self) -> &[RuntimePluginState] {
        match self {
            DesignState::Bytecode(s) => &s.plugins,
            #[cfg(feature = "native")]
            DesignState::Cranelift(s) => &s.plugins,
        }
    }
}

impl Design {
    pub fn run(
        &self,
        state: &mut DesignState,
        world: &mut dyn World,
        time: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = match (&self.backend, state) {
            #[cfg(feature = "tailcall")]
            (DesignBackend::Bytecode { design }, DesignState::Bytecode(state)) => {
                state.schedule.set_max_time(time);
                design.execute_inner_tailcall(state, world)
            }
            #[cfg(not(feature = "tailcall"))]
            (DesignBackend::Bytecode { design }, DesignState::Bytecode(state)) => {
                state.schedule.set_max_time(time);
                #[cfg(feature = "profile")]
                if let Some(profile) = &design.profile {
                    return design
                        .execute_with_tracer(
                            &mut vogls_bytecode::profile::SamplingProfiler::new(
                                design.debug_info.clone(),
                                profile.as_path(),
                            ),
                            state,
                            world,
                        )
                        .map_err(|err| report(world, err));
                }

                if design.itrace {
                    design.execute_with_tracer(
                        &mut vogls_bytecode::InstructionTracer::new_stderr(),
                        state,
                        world,
                    )
                } else if design.stats {
                    design.execute_with_tracer(
                        &mut vogls_bytecode::ICountTracer::default(),
                        state,
                        world,
                    )
                } else {
                    design.execute(state, world)
                }
            }
            #[cfg(feature = "native")]
            (DesignBackend::Cranelift { design }, DesignState::Cranelift(state)) => {
                design.run(state, world, time)
            }

            #[allow(unreachable_patterns)]
            _ => panic!(),
        };
        result.map_err(|err| report(world, err))
    }

    pub fn time_resolution(&self) -> TimeResolution {
        self.time_resolution
    }

    pub fn resolve_handle_sym(&self, signal: SignalHandle) -> (&SignalSymbol, &SignalRefs) {
        let Symbol::Signal(symbol) = &self.elab_table[signal.symbol].content else {
            unreachable!();
        };
        let SignalValue::Signal(refs) = &symbol.value else {
            unreachable!();
        };
        (symbol, refs)
    }

    pub fn resolve_handle(&self, signal: SignalHandle) -> RtSignal {
        let (_, signal) = self.resolve_handle_sym(signal);
        let (key, slice) = signal.probe_signal();
        let key = self.rt_signal_map[&key];
        RtSignal { key, slice }
    }

    /// Writes `bits` to the signal of `handle` from outside of the design.
    ///
    /// The write lasts until a driver of the signal updates it. A net connected by a `tran` is
    /// written through the value it drives into the `tran`, as in Icarus Verilog: the other side
    /// observes the write, and a bit that the net does not drive itself keeps it as one more driver
    /// of the `tran`.
    pub fn write_handle(&self, state: &mut DesignState, handle: SignalHandle, bits: &Bits) {
        let (_, refs) = self.resolve_handle_sym(handle);
        let (key, slice) = refs.blocking_drive_signal();
        let key = self.rt_signal_map[&key];
        self.set_signal(state, RtSignal { key, slice }, bits);
    }

    pub fn resolve_handle_width(&self, signal: SignalHandle) -> VectorSize {
        let (net, _) = self.resolve_handle_sym(signal);
        net.ty.bit_length()
    }

    fn get_heap_ref(&self, signal: RtSignalKey) -> HeapRef {
        self.signal_to_heap[signal.as_usize()]
    }

    pub fn set_signal(&self, state: &mut DesignState, signal: RtSignal, bits: &Bits) {
        let heap_ref = self.get_heap_ref(signal.key);
        let mode = self.signal_mode[signal.key.as_usize()];
        let signal_bits = state.runtime().heap.load_bits(heap_ref, mode);
        let updated = match signal.slice {
            None => &signal_bits != bits,
            Some(slice) => &signal_bits.slicez(slice.lsb(), slice.width()) != bits,
        };

        if updated {
            let value = match signal.slice {
                None => bits.clone(),
                Some(slice) => signal_bits.set_slice(slice.lsb(), bits),
            };
            state.runtime_mut().heap.store_bits(heap_ref, mode, &value);

            match (&self.backend, state) {
                (DesignBackend::Bytecode { design }, DesignState::Bytecode(state)) => {
                    design.poke_signal(state, signal.key)
                }
                #[cfg(feature = "native")]
                (DesignBackend::Cranelift { design }, DesignState::Cranelift(state)) => {
                    design.poke_signal(state, signal.key)
                }

                #[allow(unreachable_patterns)]
                _ => unreachable!(),
            }
        }
    }

    pub fn get_signal(&self, state: &DesignState, signal: RtSignal) -> Bits {
        let heap_ref = self.get_heap_ref(signal.key);
        let mode = self.signal_mode[signal.key.as_usize()];
        state.runtime().heap.load_unaligned_bits(heap_ref, mode)
    }

    pub fn emit_ir(&self) -> String {
        let mut s = String::new();
        for process in self.gl.processes.values() {
            use std::fmt::Write;
            writeln!(&mut s, "{}", process.display(&self.gl)).unwrap();
        }
        s
    }
}

#[derive(Clone, Copy)]
pub struct RtSignal {
    pub(crate) key: RtSignalKey,
    pub(crate) slice: Option<SignalSlice>,
}
impl RtSignal {
    pub fn key(&self) -> RtSignalKey {
        self.key
    }
}

/// Reports the error that ended the simulation on the standard error of `world`.
fn report(world: &mut dyn World, err: RuntimeError) -> Box<dyn std::error::Error> {
    _ = writeln!(world.stderr(), "ERR! {err}");
    err.into()
}
