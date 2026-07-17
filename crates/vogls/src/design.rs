use std::sync::Arc;

use vogls_codegen::HeapRef;
use vogls_frontend::ident_table::IdentTable;
use vogls_frontend::symbol_table::{FrozenSymbolTable, SymbolId};
use vogls_ir::vcd::{VcdScope, VcdValue, VcdVariableKey};
use vogls_ir::{Bits, GlobalContext, LogicMode, SignalKey, SignalSlice, VectorSize};
use vogls_runtime::SimulationIo;
use vogls_runtime::plugins::RuntimePluginState;
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_utils::{Table, VgHashMap};
pub use vogls_verilog::arena::Arena;
pub use vogls_verilog::elaborate::{VSymbol, VSymbolTable};
pub use vogls_verilog::tokenizer::Macro;

use crate::elaborated_design::SignalHandle;
use crate::symbol::{NetSignal, NetSymbol, NetValue, Symbol};

pub enum DesignBackend {
    Bytecode {
        design: vogls_bytecode::Design,
    },
    #[cfg(feature = "native")]
    Compiled {
        design: vogls_codegen_c::runtime::CDesign,
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
    pub(crate) initial_state: DesignState,
}

#[derive(Clone)]
pub enum DesignState {
    Bytecode(vogls_bytecode::State),
    #[cfg(feature = "native")]
    Compiled(vogls_codegen_c::runtime::CDesignState),
}

impl DesignState {
    pub fn runtime_mut(&mut self) -> &mut RuntimeState {
        match self {
            DesignState::Bytecode(s) => &mut s.runtime,
            #[cfg(feature = "native")]
            DesignState::Compiled(s) => &mut s.runtime,
        }
    }
    pub fn runtime(&self) -> &RuntimeState {
        match self {
            DesignState::Bytecode(s) => &s.runtime,
            #[cfg(feature = "native")]
            DesignState::Compiled(s) => &s.runtime,
        }
    }
    pub fn plugins_mut(&mut self) -> &mut [RuntimePluginState] {
        match self {
            DesignState::Bytecode(s) => &mut s.plugins,
            #[cfg(feature = "native")]
            DesignState::Compiled(s) => &mut s.plugins,
        }
    }
    pub fn plugins(&self) -> &[RuntimePluginState] {
        match self {
            DesignState::Bytecode(s) => &s.plugins,
            #[cfg(feature = "native")]
            DesignState::Compiled(s) => &s.plugins,
        }
    }
}

impl Design {
    pub fn run(
        &mut self,
        io: &mut SimulationIo,
        time: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match (&mut self.backend, &mut self.initial_state) {
            #[cfg(feature = "tailcall")]
            (DesignBackend::Bytecode { design }, DesignState::Bytecode(state)) => design
                .execute_inner_tailcall(state, &mut io.stdout, &mut io.stderr)
                .map_err(|_| "execution failed.".into()),
            #[cfg(not(feature = "tailcall"))]
            (DesignBackend::Bytecode { design }, DesignState::Bytecode(state)) => {
                state.schedule.set_max_time(time);
                if design.itrace {
                    design.execute_with_tracer(
                        &mut vogls_bytecode::InstructionTracer::new_stderr(),
                        state,
                        &mut io.stdout,
                        &mut io.stderr,
                    )
                } else if design.stats {
                    design.execute_with_tracer(
                        &mut vogls_bytecode::ICountTracer::default(),
                        state,
                        &mut io.stdout,
                        &mut io.stderr,
                    )
                } else {
                    design.execute(state, &mut io.stdout, &mut io.stderr)
                }
                .map_err(|_| "execution failed.".into())
            }

            #[cfg(feature = "native")]
            (DesignBackend::Compiled { design }, DesignState::Compiled(initial_state)) => design
                .run(initial_state, io, time)
                .map_err(|_| "execution failed.".into()),
            _ => panic!(),
        }
    }

    pub fn run_from_state(
        &self,
        state: &mut DesignState,
        io: &mut SimulationIo,
        time: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match (&self.backend, state) {
            #[cfg(feature = "tailcall")]
            (DesignBackend::Bytecode { design }, DesignState::Bytecode(state)) => design
                .execute_inner_tailcall(state, &mut io.stdout, &mut io.stderr)
                .map_err(|_| "execution failed.".into()),
            #[cfg(not(feature = "tailcall"))]
            (DesignBackend::Bytecode { design }, DesignState::Bytecode(state)) => {
                state.schedule.set_max_time(time);
                if design.itrace {
                    design.execute_with_tracer(
                        &mut vogls_bytecode::InstructionTracer::new_stderr(),
                        state,
                        &mut io.stdout,
                        &mut io.stderr,
                    )
                } else if design.stats {
                    design.execute_with_tracer(
                        &mut vogls_bytecode::ICountTracer::default(),
                        state,
                        &mut io.stdout,
                        &mut io.stderr,
                    )
                } else {
                    design.execute(state, &mut io.stdout, &mut io.stderr)
                }
                .map_err(|_| "execution failed.".into())
            }
            #[cfg(feature = "native")]
            (DesignBackend::Compiled { design }, DesignState::Compiled(state)) => design
                .run(state, io, time)
                .map_err(|_| "execution failed.".into()),
            _ => panic!(),
        }
    }

    pub fn initial_state(&self) -> &DesignState {
        &self.initial_state
    }

    pub fn resolve_handle_sym(&self, signal: SignalHandle) -> (&NetSymbol, &NetSignal) {
        let Symbol::Net(net) = &self.elab_table[signal.symbol].content else {
            unreachable!();
        };
        let NetValue::Signal(signal) = &net.net else {
            unreachable!();
        };
        (net, signal)
    }

    pub fn resolve_handle(&self, signal: SignalHandle) -> RtSignal {
        let (_, signal) = self.resolve_handle_sym(signal);
        let (key, slice) = signal.probe_signal();
        let key = self.rt_signal_map[&key];
        RtSignal { key, slice }
    }

    pub fn resolve_handle_width(&self, signal: SignalHandle) -> VectorSize {
        let (net, _) = self.resolve_handle_sym(signal);
        net.ty.force_net_width()
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
            state.runtime_mut().heap.store_bits(heap_ref, mode, bits);

            match (&self.backend, state) {
                #[cfg(feature = "native")]
                (DesignBackend::Bytecode { design }, DesignState::Bytecode(state)) => {
                    design.poke_signal(state, signal.key)
                }
                #[cfg(feature = "native")]
                (DesignBackend::Compiled { design }, DesignState::Compiled(state)) => {
                    design.poke_signal(state, signal.key)
                }
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

pub fn vcd_scope(
    symtable: &FrozenSymbolTable<Symbol>,
    scope: SymbolId,
    ident_table: &IdentTable,
) -> vogls_ir::vcd::VcdOutput {
    let mut key = scope;
    while let Some(parent) = symtable[key].parent() {
        key = parent;
    }

    let mut table = Table::new();
    let mut signal_map = VgHashMap::default();
    let mut scope = VcdScope {
        name: "".to_string(),
        items: Vec::new(),
    };
    extend_symbol_table_to_vcd_scope(
        &mut scope,
        &[key],
        symtable,
        ident_table,
        &mut table,
        &mut signal_map,
    );
    vogls_ir::vcd::VcdOutput {
        table,
        signal_map,
        children: scope.items,
    }
}

fn extend_symbol_table_to_vcd_scope(
    scope: &mut VcdScope,
    symbols: &[SymbolId],
    table: &FrozenSymbolTable<Symbol>,
    ident_table: &IdentTable,
    variable_table: &mut Table<VcdVariableKey, vogls_ir::vcd::VcdVariable>,
    signal_map: &mut VgHashMap<SignalKey, Vec<VcdVariableKey>>,
) {
    for sid in symbols.iter() {
        let name = &ident_table[table[*sid].name()];
        match &table[*sid].content {
            Symbol::Module | Symbol::Block | Symbol::GenerateBlocks => {
                let mut subscope = VcdScope {
                    name: name.to_string(),
                    items: Vec::new(),
                };
                extend_symbol_table_to_vcd_scope(
                    &mut subscope,
                    table[*sid].children(&table),
                    table,
                    ident_table,
                    variable_table,
                    signal_map,
                );
                scope
                    .items
                    .push(vogls_ir::vcd::VcdScopeItem::Scope(subscope));
            }
            Symbol::Net(i) => {
                let net = &i.net;

                // @TODO: Property implement this.
                let lsb = 0;
                let msb = i.ty.force_net_width().get() - 1;
                let msb_lsb = (msb > 0).then_some((msb, lsb));

                let (value, signal) = match net {
                    NetValue::Signal(net_signal) => {
                        let (signal, slice) = net_signal.probe_signal();
                        (VcdValue::Signal(signal, slice), Some(signal))
                    }
                    NetValue::Constant(bits) => (VcdValue::Constant(bits.clone()), None),
                };
                let variable_key = variable_table.insert(vogls_ir::vcd::VcdVariable {
                    name: ident_table[table[*sid].name()].to_string(),
                    value,
                    ty: vogls_ir::vcd::NetType::Wire,
                    msb_lsb,
                });
                scope
                    .items
                    .push(vogls_ir::vcd::VcdScopeItem::Variable(variable_key));
                if let Some(signal) = signal {
                    signal_map.entry(signal).or_default().push(variable_key);
                }
            }
            Symbol::Task | Symbol::Function | Symbol::Parameter(_) => {}
        }
    }
}
