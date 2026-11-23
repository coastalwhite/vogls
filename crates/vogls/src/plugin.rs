use std::path::PathBuf;
use std::sync::Arc;

use vogls_codegen::HeapRef;
use vogls_frontend::ident_table::IdentTable;
use vogls_frontend::symbol_table::{FrozenSymbolTable, SymbolId};
use vogls_ir::time::TimeResolution;
use vogls_ir::vcd::{NetType, VcdVariableKey};
use vogls_ir::{LogicMode, SCALAR_VSIZE, SignalSlice};
use vogls_runtime::RtSignalKey;
use vogls_runtime::plugins::RuntimePlugin;
use vogls_utils::{SecondaryTable, TableKey as _, VgHashMap, VgHashSet};
use vogls_vcd::{VcdScope, VcdScopeItem, VcdValue, VcdVariable};

use crate::SignalHandle;
use crate::design::RtSignal;
use crate::elaborated_design::ElaboratedDesign;
use crate::symbol::{SignalValue, Symbol};

/// A plugin that hooks into the design pipeline.
///
/// A plugin is handed to [`ElaboratedDesign::lower`], gets to register the signals it is
/// interested in during [`VoglsPlugin::register_handles`], and is turned into a
/// [`RuntimePlugin`] by [`VoglsPlugin::finalize`] once the runtime layout is known.
pub trait VoglsPlugin: Send + Sync + 'static {
    fn clone(&self) -> Box<dyn VoglsPlugin>;
    fn register_handles(&mut self, design: &mut ElaboratedDesign<'_>);
    fn finalize(self: Box<Self>, ctx: &FinalizeContext<'_>) -> Box<dyn RuntimePlugin>;
}

/// Runtime layout of the design, handed to [`VoglsPlugin::finalize`].
pub struct FinalizeContext<'a> {
    pub handle_map: &'a VgHashMap<SignalHandle, RtSignal>,
    pub signal_to_heap: &'a Arc<[HeapRef]>,
    pub signal_mode: &'a Arc<[LogicMode]>,
    pub time_resolution: TimeResolution,

    pub(crate) table: &'a FrozenSymbolTable<Symbol>,
    pub(crate) ident_table: &'a IdentTable,
}

/// Dumps a VCD trace of the design.
///
/// With a `path`, the whole design is dumped to that file. Without, nothing is dumped until
/// the design itself calls `$dumpfile` / `$dumpvars`.
///
/// The bytecode intrinsics for the `$dump*` system tasks look up the resulting
/// [`vogls_vcd::RtVcdOutput`] amongst the runtime plugins.
#[derive(Clone)]
pub(crate) struct VcdPlugin {
    pub(crate) path: Option<PathBuf>,
    /// The signals that are dumped, see [`VoglsPlugin::register_handles`].
    pub(crate) handles: VgHashSet<SignalHandle>,
}

impl VcdPlugin {
    pub(crate) fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            handles: VgHashSet::default(),
        }
    }
}

impl VoglsPlugin for VcdPlugin {
    fn clone(&self) -> Box<dyn VoglsPlugin> {
        Box::new(Clone::clone(self))
    }

    /// Registers every net and register in the design, as the whole hierarchy is dumped.
    fn register_handles(&mut self, design: &mut ElaboratedDesign<'_>) {
        let symbols = design.table().symbol_id_iter().collect::<Vec<_>>();
        self.handles.extend(
            symbols
                .into_iter()
                .filter_map(|sid| design.get_probe_handle(sid)),
        );
    }

    fn finalize(self: Box<Self>, ctx: &FinalizeContext<'_>) -> Box<dyn RuntimePlugin> {
        let mut builder = VcdScopeBuilder {
            ctx,
            handles: &self.handles,
            num_variables: 0,
            map: VgHashMap::default(),
        };
        let mut children = Vec::new();
        builder.extend(&mut children, &[ctx.table.roots()[0]]);

        let mut map = SecondaryTable::new();
        for (key, variables) in builder.map {
            map.insert(key, variables.into_boxed_slice());
        }

        let output = match &self.path {
            Some(path) => vogls_vcd::RtVcdOutput::new_path(
                path,
                ctx.signal_to_heap.clone(),
                children,
                map,
                ctx.time_resolution,
                ctx.signal_mode.clone(),
            ),
            None => vogls_vcd::RtVcdOutput::new(
                Box::new(Vec::new()),
                ctx.signal_to_heap.clone(),
                Vec::new(),
                map,
                ctx.time_resolution,
                ctx.signal_mode.clone(),
            ),
        };
        Box::new(output)
    }
}

/// Builds the VCD hierarchy from the lowered symbol table, dumping the registered signals.
struct VcdScopeBuilder<'a, 'b> {
    ctx: &'a FinalizeContext<'b>,
    handles: &'a VgHashSet<SignalHandle>,
    num_variables: usize,
    /// The VCD variables, and their slice, that every runtime signal is dumped to.
    map: VgHashMap<RtSignalKey, Vec<(VcdVariableKey, Option<SignalSlice>)>>,
}

impl VcdScopeBuilder<'_, '_> {
    fn extend(&mut self, items: &mut Vec<VcdScopeItem>, symbols: &[SymbolId]) {
        let table = self.ctx.table;
        for &sid in symbols {
            let name = self.ctx.ident_table[table[sid].name()].to_string();
            match &table[sid].content {
                Symbol::Module | Symbol::Block | Symbol::GenerateBlocks | Symbol::Modules => {
                    let mut scope = VcdScope {
                        name,
                        items: Vec::new(),
                    };
                    self.extend(&mut scope.items, table[sid].children(table));
                    items.push(VcdScopeItem::Scope(scope));
                }
                Symbol::Signal(signal) => {
                    let handle = SignalHandle { symbol: sid };
                    if !self.handles.contains(&handle) {
                        continue;
                    }

                    // @TODO: Properly implement the LSB.
                    let msb = signal.ty.bit_length().get() - 1;
                    let msb_lsb = (msb > 0).then_some((msb, 0));
                    let value = match &signal.value {
                        SignalValue::Signal(_) => {
                            let RtSignal { key, slice } = self.ctx.handle_map[&handle];
                            let variable = self.next_variable();
                            self.map.entry(key).or_default().push((variable, slice));
                            (variable, VcdValue::Signal(key))
                        }
                        SignalValue::Constant(bits) => {
                            (self.next_variable(), VcdValue::Constant(bits.clone()))
                        }
                    };
                    items.push(variable(name, value, msb_lsb));
                }
                Symbol::Parameter(bits) => {
                    let msb_lsb =
                        (bits.size() > SCALAR_VSIZE).then_some((bits.size().get() - 1, 0));
                    let value = (self.next_variable(), VcdValue::Constant(bits.clone()));
                    items.push(variable(name, value, msb_lsb));
                }
                Symbol::Task | Symbol::Function => {}
            }
        }
    }

    fn next_variable(&mut self) -> VcdVariableKey {
        let key = VcdVariableKey::from_usize(self.num_variables).unwrap();
        self.num_variables += 1;
        key
    }
}

fn variable(
    name: String,
    (variable, value): (VcdVariableKey, VcdValue),
    msb_lsb: Option<(u32, u32)>,
) -> VcdScopeItem {
    VcdScopeItem::Variable(VcdVariable {
        name,
        variable,
        value,
        ty: NetType::Wire,
        msb_lsb,
    })
}
