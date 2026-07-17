use std::fmt;
#[cfg(feature = "native")]
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use slotmap::SlotMap;
use vogls_codegen::lsra::StackTracker;
use vogls_codegen::{HeapBuilder, HeapOffset, HeapRef};
use vogls_frontend::ident_table::IdentTable;
use vogls_frontend::symbol_table::FrozenSymbolTable;
#[cfg(feature = "unstable")]
use vogls_ir::ProcessKind;
use vogls_ir::optimize::Optimizations;
use vogls_ir::watchers::WatchMap;
use vogls_ir::{GlobalContext, LogicMode, SCALAR_VSIZE, Signal, SignalKey};
use vogls_runtime::plugins::RuntimePlugin;
#[cfg(feature = "native")]
use vogls_runtime::plugins::RuntimePluginState;
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_bytecode::bytecode::lower::{LowerBytecodeOptions, lower_process_to_bytecode};
use vogls_bytecode::bytecode::{BytecodeEncoder, BytecodeListeners, Schedule};
#[cfg(feature = "native")]
use vogls_utils::TimerStack;
use vogls_utils::{TableKey as _, VgHashMap};
use vogls_verilog::lower::Diagnostics;
use vogls_verilog::tokenizer::Tokenized;

use crate::design::{Design, DesignBackend, DesignState, RtSignal, vcd_scope};
use crate::plugin::VoglsPlugin;
use crate::symbol::{NetValue, Symbol};
use crate::{ElaboratedDesign, SignalHandle};

#[derive(Clone)]
pub struct LoweredDesign {
    pub(crate) table: FrozenSymbolTable<Symbol>,
    pub(crate) gl: GlobalContext,
    pub(crate) plugins: Vec<Box<dyn VoglsPlugin>>,
    pub(crate) vcd: Option<PathBuf>,
    pub(crate) has_vcd: bool,
    // @TODO: This is duplicated in the ParsedDesign. Maybe we can somehow, remove this?
    pub(crate) ident_table: IdentTable,
    #[expect(unused)]
    pub(crate) token_buffer: Tokenized,

    pub itrace: bool,
    pub emit_vm: bool,
    pub stats: bool,
    pub debug_symbols: bool,
    pub output_source: Option<PathBuf>,
    pub print_vm_map: bool,
}

impl Clone for Box<dyn VoglsPlugin> {
    fn clone(&self) -> Self {
        VoglsPlugin::clone(self.as_ref())
    }
}

pub struct LowerError<'a> {
    pub(crate) design: ElaboratedDesign<'a>,
    pub(crate) diagnostics: Diagnostics,
    #[expect(unused)]
    pub(crate) stage: LowerErrorStage,
}

#[derive(Debug)]
pub(crate) enum LowerErrorStage {
    GlobalItems,
    Modules,
}

impl<'a> fmt::Display for LowerError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.diagnostics.report(&self.design.token_buffer).fmt(f)
    }
}

pub struct EmitDesignIr<'a>(&'a LoweredDesign);
struct CodegenPreparation {
    heap_builder: HeapBuilder,
    signal_to_heap: Arc<[HeapRef]>,
    signal_mode: Arc<[LogicMode]>,
    rt_signal_map: VgHashMap<SignalKey, RtSignalKey>,
    lupdt_indexes: VgHashMap<RtSignalKey, u64>,
    plugins: Vec<Box<dyn RuntimePlugin>>,
}

const NUM_REGIONS: u8 = 3;
impl LoweredDesign {
    pub fn optimize(&mut self, opts: Optimizations) -> &mut Self {
        let processes = self.gl.processes.keys().collect::<Vec<_>>();
        vogls_ir::optimize::optimize_processes(&mut self.gl, &processes, opts);
        self
    }

    pub fn emit_ir<'a>(&'a self) -> EmitDesignIr<'a> {
        EmitDesignIr(self)
    }

    fn prepare_codegen(&mut self) -> CodegenPreparation {
        let mut heap_builder = HeapBuilder::new();
        let mut signal_to_heap = Vec::new();
        let mut signal_mode = Vec::new();
        let mut rt_signal_map = VgHashMap::default();
        let mut lupdt_indexes = VgHashMap::<RtSignalKey, u64>::default();

        generate_signals_heap(
            &mut heap_builder,
            &mut rt_signal_map,
            &self.gl.signals,
            &mut signal_to_heap,
            &mut signal_mode,
            self.print_vm_map,
        );
        find_lupdt_signals(&self.gl, &rt_signal_map, &mut lupdt_indexes);
        let signal_to_heap: Arc<[HeapRef]> = signal_to_heap.into();
        let signal_mode: Arc<[LogicMode]> = signal_mode.into();

        let handle_map = VgHashMap::from_iter(self.table.symbol_id_iter().filter_map(|sid| {
            let Symbol::Net(net) = &self.table[sid].content else {
                return None;
            };
            let NetValue::Signal(signal) = &net.net else {
                return None;
            };
            let (k, slice) = signal.probe_signal();
            let key = rt_signal_map[&k];

            Some((SignalHandle { symbol: sid }, RtSignal { key, slice }))
        }));

        let mut plugins: Vec<Box<dyn RuntimePlugin>> = std::mem::take(&mut self.plugins)
            .into_iter()
            .map(|mut x| {
                x.finalize(&handle_map, &signal_to_heap);
                x as Box<dyn RuntimePlugin>
            })
            .collect();

        if self.has_vcd() || self.vcd.is_some() {
            let tlm = self.table.roots()[0];
            let scope = vcd_scope(&self.table, tlm, &self.ident_table);
            let (children, map) = vogls_vcd::VcdScope::lower(&scope, &rt_signal_map);
            let rtvcdoutput = match &self.vcd {
                Some(path) => {
                    vogls_vcd::RtVcdOutput::new_path(path, signal_to_heap.clone(), children, map)
                }
                None => vogls_vcd::RtVcdOutput::new(
                    Box::new(Vec::new()),
                    signal_to_heap.clone(),
                    Vec::new(),
                    map,
                ),
            };
            plugins.push(Box::new(rtvcdoutput));
        }

        CodegenPreparation {
            heap_builder,
            signal_to_heap,
            signal_mode,
            rt_signal_map,
            lupdt_indexes,
            plugins,
        }
    }

    #[cfg(feature = "native")]
    pub fn compile(mut self) -> Result<Design, Box<dyn std::error::Error>> {
        use crate::design::{DesignBackend, DesignState};

        let CodegenPreparation {
            heap_builder,
            signal_to_heap,
            signal_mode,
            rt_signal_map,
            lupdt_indexes,
            plugins,
        } = self.prepare_codegen();

        let (initial_state, design) = lower_to_shared_object(
            &self.gl,
            &rt_signal_map,
            heap_builder,
            &signal_to_heap,
            lupdt_indexes,
            &mut TimerStack::new(false),
            self.itrace,
            self.stats,
            self.debug_symbols,
            self.output_source.as_deref(),
            plugins,
            NUM_REGIONS,
        )?;

        return Ok(Design {
            gl: self.gl,
            ident_table: self.ident_table,
            elab_table: self.table,
            backend: DesignBackend::Compiled { design },
            rt_signal_map,
            signal_to_heap,
            signal_mode,
            initial_state: DesignState::Compiled(initial_state),
        });
    }

    pub fn to_bytecode(mut self) -> Result<Design, Box<dyn std::error::Error>> {
        let CodegenPreparation {
            mut heap_builder,
            signal_to_heap,
            signal_mode,
            mut rt_signal_map,
            lupdt_indexes,
            plugins,
        } = self.prepare_codegen();

        let mut stack_tracker = StackTracker::default();
        let mut bytecode = BytecodeEncoder::default();
        let mut schedule = Schedule::new(NUM_REGIONS);
        let watch_map = WatchMap::new(&self.gl.bbs);
        let mut listeners = BytecodeListeners::new(watch_map.num_watches());
        let mut num_stack_words = 0;
        let options = LowerBytecodeOptions {
            emit: self.emit_vm,
            has_plugins: !plugins.is_empty(),
        };

        for process in self.gl.processes.keys() {
            lower_process_to_bytecode(
                process,
                &self.gl,
                &mut stack_tracker,
                &mut heap_builder,
                &mut num_stack_words,
                &watch_map,
                &mut schedule,
                &mut listeners,
                &signal_to_heap,
                &rt_signal_map,
                &lupdt_indexes,
                &mut bytecode,
                &options,
            );
        }

        let stack_offset = heap_builder.claim_words(num_stack_words) as u64;
        let mut heap = heap_builder.finish();
        let mut lupdt_updated = vec![false; lupdt_indexes.len()];

        for (key, signal) in &self.gl.signals {
            if let Some(initialize) = &signal.initialize {
                let rt_key = rt_signal_map[&key];
                assert_eq!(initialize.size(), signal.size);
                heap.store_bits(signal_to_heap[rt_key.as_usize()], signal.mode, initialize);
                let is_unchanged = match signal.mode {
                    LogicMode::TwoValue => initialize.count_zeros() == initialize.size().get(),
                    LogicMode::FourValue => initialize.count_unknown() == initialize.size().get(),
                };
                if !is_unchanged && let Some(lupdt_idx) = lupdt_indexes.get(&rt_key) {
                    lupdt_updated[*lupdt_idx as usize] = true;
                }
            }
        }
        let runtime = RuntimeState::new(&self.gl, heap, &lupdt_updated);
        let state = vogls_bytecode::bytecode::State {
            runtime,
            plugins,
            schedule,
            listeners,
        };
        let design = vogls_bytecode::bytecode::Design {
            bytecode: bytecode.data,
            intrinsics: bytecode
                .intrinsics
                .take_keys()
                .into_iter()
                .map(|v| v.0)
                .collect(),
            stack_offset,
            itrace: self.itrace,
            stats: self.stats,
        };

        Ok(Design {
            gl: self.gl,
            ident_table: self.ident_table,
            elab_table: self.table,
            backend: DesignBackend::Bytecode { design },
            rt_signal_map,
            signal_to_heap,
            signal_mode,
            initial_state: DesignState::Bytecode(state),
        })
    }

    fn has_vcd(&self) -> bool {
        self.has_vcd
    }

    pub fn trace_vcd(&mut self, vcd: PathBuf) -> &mut Self {
        self.vcd = Some(vcd);
        self
    }

    #[cfg(feature = "unstable")]
    pub fn process_stats(&self) -> LowerStats {
        let mut counts = [0u64; ProcessKind::NUM_KINDS];
        for process in self.gl.processes.values() {
            counts[process.kind as usize] += 1;
        }
        LowerStats(counts)
    }
}

impl<'a> fmt::Display for EmitDesignIr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for signal in self.0.gl.signals.values() {
            signal.display().fmt(f)?;
            writeln!(f)?;
        }
        writeln!(f)?;
        for process in self.0.gl.processes.values() {
            process.display(&self.0.gl).fmt(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

pub fn generate_signals_heap(
    heap_builder: &mut HeapBuilder,
    signal_map: &mut VgHashMap<SignalKey, RtSignalKey>,
    signals: &SlotMap<SignalKey, Signal>,
    heap_refs: &mut Vec<HeapRef>,
    signal_mode: &mut Vec<LogicMode>,
    print_mapping: bool,
) {
    signal_mode.extend(signals.values().map(|s| s.mode));
    signal_map.extend(signals.keys().enumerate().map(|(i, key)| {
        if print_mapping {
            let signal = &signals[key];
            eprintln!("{}: {}", signal.name, i);
        }
        (key, RtSignalKey::from_usize(i).unwrap())
    }));
    heap_refs.resize(
        signals.len(),
        HeapRef {
            offset: HeapOffset { bit_offset: 0 },
            size: SCALAR_VSIZE,
        },
    );

    for (min_bits, max_bits) in [
        // (1, u32::MAX)
        (33, u32::MAX),
        (17, 32),
        (9, 16),
        (5, 8),
        (3, 4),
        (2, 2),
        (1, 1),
    ] {
        for (i, signal) in signals.values().enumerate() {
            let size = signal.size;
            let mut num_bits = size.get();
            if signal.mode == LogicMode::FourValue {
                num_bits = num_bits * 2;
            }

            if (min_bits..=max_bits).contains(&num_bits) {
                heap_refs[i] = heap_builder.claim(signal.mode, size);
            }
        }
    }
}

pub fn find_lupdt_signals(
    gl: &GlobalContext,
    signal_map: &VgHashMap<SignalKey, RtSignalKey>,
    lupdt_indexes: &mut VgHashMap<RtSignalKey, u64>,
) {
    for bb in gl.bbs.values() {
        for i in bb.instrs.iter() {
            if let vogls_ir::Instruction::LastUpdateTime(_, signal) = i {
                let idx = lupdt_indexes.len();
                lupdt_indexes
                    .entry(signal_map[signal])
                    .or_insert(idx as u64);
            }
        }
    }
}

#[cfg(feature = "native")]
pub fn lower_to_shared_object(
    gl: &GlobalContext,
    signal_map: &VgHashMap<SignalKey, RtSignalKey>,
    mut heap_builder: HeapBuilder,
    heap_refs: &[HeapRef],
    lupdt_indexes: VgHashMap<RtSignalKey, u64>,
    timers: &mut TimerStack,

    itrace: bool,
    stats: bool,
    debug_symbols: bool,
    output_source: Option<&Path>,
    plugins: Vec<RuntimePluginState>,
    num_additional_regions: u8,
) -> Result<
    (
        vogls_codegen_c::runtime::CDesignState,
        vogls_codegen_c::runtime::CDesign,
    ),
    Box<dyn std::error::Error>,
> {
    use std::process::Command;

    use vogls_codegen_c::runtime::{CDesign, SharedObjectContainer};
    use vogls_codegen_c::{
        CLowerOptions, ListenerBuilder, StateBuilder, lower_process_array, lower_signal_drive_fn,
        lower_signal_drive_header,
    };
    use vogls_ir::ProcessKind;

    timers.start("lower to C");
    let mut listener_builder = ListenerBuilder::default();
    let mut out = Vec::new();
    let mut state_builder = StateBuilder::default();
    let mut index = 0u64;
    let signal_to_tv_index =
        VgHashMap::from_iter(gl.signals.iter().filter_map(|(key, s)| match s.mode {
            LogicMode::TwoValue => {
                let i = index;
                index += 1;
                Some((signal_map[&key], i))
            }
            LogicMode::FourValue => None,
        }));

    for signal in gl.signals.keys() {
        lower_signal_drive_header(&mut out, signal, &signal_map)?;
    }

    let lower_options = CLowerOptions {
        itrace,
        stats,
        num_plugins: plugins.len(),
    };
    let mut byte_count = [0u64; ProcessKind::NUM_KINDS];
    for (i, process) in gl.processes.keys().enumerate() {
        let start_length = out.len();
        vogls_codegen_c::lower_process(
            &mut out,
            process,
            i,
            gl,
            &mut heap_builder,
            &mut listener_builder,
            &mut state_builder,
            signal_map,
            &lupdt_indexes,
            heap_refs,
            &signal_to_tv_index,
            &lower_options,
        )?;
        byte_count[gl.processes[process].kind as usize] += (out.len() - start_length) as u64;
    }

    if false {
        println!("Process C Bytecount:");
        for (kind, count) in ProcessKind::KINDS.into_iter().zip(byte_count) {
            if count == 0 {
                continue;
            }
            println!("  {}: {}", kind.into_static_str(), count);
        }
    }

    lower_process_array(&mut out, gl)?;

    for signal in gl.signals.keys() {
        lower_signal_drive_fn(
            &mut out,
            gl,
            signal,
            &listener_builder,
            signal_map,
            &lupdt_indexes,
            &mut state_builder,
            &lower_options,
        )?;
    }

    let mut c_file = Vec::new();
    let mut tempdir = tempfile::TempDir::with_prefix("vogls")?;

    vogls_codegen_c::prologue(&mut c_file)?;
    c_file.extend(&out);
    vogls_codegen_c::epilogue(&mut c_file)?;
    if let Some(output_source) = output_source {
        std::fs::write(output_source, &c_file)?;
    }
    if debug_symbols {
        tempdir.disable_cleanup(debug_symbols);
        std::fs::write(tempdir.path().join("design.c"), &c_file)?;
        println!("Output directory: {}", tempdir.path().display());
    }
    timers.stop();

    timers.start("compile C");
    let code_path = tempdir.path().join("code.so");
    let mut command = Command::new("clang");
    command.args(["-x", "c", "-O1", "-fPIC", "-shared"]);
    if debug_symbols {
        command.arg("-g3").arg(tempdir.path().join("design.c"));
    } else {
        command.arg("-");
    }
    command.arg("-o").arg(&code_path);
    let mut command = command.stdin(std::process::Stdio::piped()).spawn()?;
    if !debug_symbols {
        use std::io::Write as _;
        command.stdin.take().unwrap().write_all(&c_file)?;
    }
    if !command.wait()?.success() {
        return Err("compilation failed!".into());
    }
    timers.stop();

    struct SharedObject {
        code_path: PathBuf,

        // Kept around so it isn't dropped.
        #[allow(unused)]
        tempdir: tempfile::TempDir,
    }
    impl SharedObjectContainer for SharedObject {
        fn as_path(&self) -> &Path {
            self.code_path.as_path()
        }
    }

    let mut heap = heap_builder.finish();
    let mut lupdt_updated = vec![false; lupdt_indexes.len()];

    for (key, signal) in &gl.signals {
        if let Some(initialize) = &signal.initialize {
            let rt_key = signal_map[&key];
            assert_eq!(initialize.size(), signal.size);
            heap.store_bits(heap_refs[rt_key.as_usize()], signal.mode, initialize);
            let is_unchanged = match signal.mode {
                LogicMode::TwoValue => initialize.count_zeros() == initialize.size().get(),
                LogicMode::FourValue => initialize.count_unknown() == initialize.size().get(),
            };
            if !is_unchanged && let Some(lupdt_idx) = lupdt_indexes.get(&rt_key) {
                lupdt_updated[*lupdt_idx as usize] = true;
            }
        }
    }

    let design = CDesign::new(
        Box::new(SharedObject { code_path, tempdir }),
        state_builder,
        num_additional_regions,
    );
    let mut initial_state = design.new_state(
        heap,
        listener_builder.top,
        num_additional_regions,
        &lupdt_updated,
        gl,
    );
    initial_state.plugins = plugins;

    Ok((initial_state, design))
}

#[cfg(feature = "unstable")]
pub struct LowerStats([u64; ProcessKind::NUM_KINDS]);

#[cfg(feature = "unstable")]
impl LowerStats {
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, u64)> {
        ProcessKind::KINDS
            .iter()
            .map(|k| k.into_static_str())
            .zip(self.0.iter().copied())
    }
}
