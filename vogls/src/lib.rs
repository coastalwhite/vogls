use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use slotmap::SlotMap;
pub use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
use vogls_codegen::{HeapBuilder, HeapOffset, HeapRef};
use vogls_codegen_c::runtime::{CDesign, CDesignState, SharedObjectContainer};
use vogls_codegen_c::{
    lower_process_array, lower_signal_drive_fn, lower_signal_drive_header, CLowerOptions, ListenerBuilder, StateBuilder
};
use vogls_frontend::ident_table::IdentId;
use vogls_ir::optimize::OptFlags;
pub use vogls_ir::{Bits, LogicMode, SignalKey, VectorSize};
use vogls_ir::{GlobalContext, SCALAR_VSIZE, Signal};
use vogls_runtime::plugins::RuntimePluginState;
pub use vogls_runtime::{RtSignalKey, SimulationIo};
pub use vogls_sim::SimulationState;
use vogls_utils::{TableKey, TimerStack, VgHashMap};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{
    CaseGenerateConstruct, CaseGenerateItem, GenerateBlock, IfGenerateConstruct,
    LoopGenerateConstruct, ModuleOrGenerateItem, ModuleOrGenerateItemContent,
};
pub use vogls_verilog::elaborate::VSymbol;
use vogls_verilog::parser::AstArenas;

pub use vogls_bits as bits;
pub use vogls_codegen as codegen;
pub use vogls_frontend as frontend;
pub use vogls_ir as ir;
pub use vogls_runtime as runtime;
pub use vogls_sim as sim;
pub use vogls_utils as utils;

pub mod design;
pub mod fuse_signals;
// pub mod symbolic_execution;

pub struct ExecutionContext {
    pub stdout: Box<dyn std::io::Write + Send + Sync>,
    pub stderr: Box<dyn std::io::Write + Send + Sync>,
    pub defines: Vec<String>,
    pub emit_hierarchy: bool,
    pub emit_unoptimized_ir: bool,
    pub emit_ir: bool,
    pub emit_vm: bool,
    pub itrace: bool,
    pub stats: bool,
    pub debug_symbols: bool,
    pub time: u64,
    pub opt: OptFlags,
    pub logic_mode: LogicMode,
    pub no_run: bool,
    pub vcd: Option<PathBuf>,
    pub compile: bool,
    pub output_source: Option<PathBuf>,
    pub timings: bool,
    pub print_unoptimized_fuse_signals: bool,
    pub print_round_fuse_signals: bool,
    pub print_optimized_fuse_signals: bool,
}

fn append_referenced_modules_generate_block<'a>(
    arenas: &'a AstArenas,
    generate_block: AstId<'a, GenerateBlock<'a>>,
    referenced: &mut HashSet<IdentId>,
) {
    match &*generate_block {
        GenerateBlock::ModuleOrGenerateItem(id) => {
            append_referenced_modules(arenas, *id, referenced)
        }
        GenerateBlock::BeginEnd(_, ids) => {
            for id in ids.iter() {
                append_referenced_modules(arenas, id, referenced);
            }
        }
    }
}

fn append_referenced_modules_opt_generate_block<'a>(
    arenas: &'a AstArenas,
    generate_block: AstId<Option<GenerateBlock<'a>>>,
    referenced: &mut HashSet<IdentId>,
) {
    match &*generate_block {
        None => {}
        Some(GenerateBlock::ModuleOrGenerateItem(id)) => {
            append_referenced_modules(arenas, *id, referenced)
        }
        Some(GenerateBlock::BeginEnd(_, ids)) => {
            for id in ids.iter() {
                append_referenced_modules(arenas, id, referenced);
            }
        }
    }
}

fn append_referenced_modules<'a>(
    arenas: &'a AstArenas,
    module_or_generate_item: AstId<'a, ModuleOrGenerateItem<'a>>,
    referenced: &mut HashSet<IdentId>,
) {
    match module_or_generate_item.content {
        ModuleOrGenerateItemContent::ModuleInstantiation(module_instantiation) => {
            let module_instantiation = &*module_instantiation;
            let module_name = module_instantiation.module_identifier.item.0;
            referenced.insert(module_name);
        }
        ModuleOrGenerateItemContent::ModuleOrGenerateItemDeclaration(_) => {}
        ModuleOrGenerateItemContent::LocalParameterDeclaration(_) => {}
        ModuleOrGenerateItemContent::ParameterOverride => {}
        ModuleOrGenerateItemContent::ContinuousAssign(_) => {}
        ModuleOrGenerateItemContent::GateInstantiation(_) => {}
        ModuleOrGenerateItemContent::UdpInstantiation(_) => {}
        ModuleOrGenerateItemContent::InitialConstruct(_) => {}
        ModuleOrGenerateItemContent::AlwaysConstruct(_) => {}
        ModuleOrGenerateItemContent::LoopGenerateConstruct(loop_generate_construct) => {
            let LoopGenerateConstruct {
                initialization: _,
                condition: _,
                iteration: _,
                block,
            } = &*loop_generate_construct;
            append_referenced_modules_generate_block(arenas, *block, referenced);
        }
        ModuleOrGenerateItemContent::IfGenerateConstruct(if_generate_construct) => {
            let IfGenerateConstruct {
                condition: _,
                truthy,
                falsy,
            } = &*if_generate_construct;
            append_referenced_modules_opt_generate_block(arenas, *truthy, referenced);
            if let Some(falsy) = falsy {
                append_referenced_modules_opt_generate_block(arenas, *falsy, referenced);
            }
        }
        ModuleOrGenerateItemContent::CaseGenerateConstruct(case_generate_construct) => {
            let CaseGenerateConstruct { value: _, items } = &*case_generate_construct;
            for item in items.iter() {
                let CaseGenerateItem { pattern: _, block } = &*item;
                append_referenced_modules_opt_generate_block(arenas, *block, referenced);
            }
        }
    }
}

pub fn run(
    path: &[&Path],
    timers: &mut TimerStack,
    top_level_module: Option<&str>,
    ectx: &mut ExecutionContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut design = timers.timed("total compilation", |timers| {
        design::Design::new(path, timers, top_level_module, ectx, Vec::new())
    })?;

    if ectx.no_run {
        return Ok(());
    }

    let stdout = std::mem::replace(&mut ectx.stdout, Box::new(Vec::new()) as _);
    let stderr = std::mem::replace(&mut ectx.stderr, Box::new(Vec::new()) as _);
    let mut io = SimulationIo::new(stdout, stderr);

    timers.start("simulation");
    design
        .run(&mut io, ectx.time)
        .map_err(|_| <Box<dyn std::error::Error>>::from("execution failed."))?;
    timers.stop();

    ectx.stdout = io.stdout;
    ectx.stderr = io.stderr;

    Ok(())
}

pub fn generate_signals_heap(
    heap_builder: &mut HeapBuilder,
    signal_map: &mut VgHashMap<SignalKey, RtSignalKey>,
    signals: &SlotMap<SignalKey, Signal>,
    heap_refs: &mut Vec<HeapRef>,
    logic_mode: LogicMode,
) {
    signal_map.extend(
        signals
            .keys()
            .enumerate()
            .map(|(i, key)| (key, RtSignalKey::from_usize(i).unwrap())),
    );
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
            if logic_mode == LogicMode::FourValue {
                num_bits = num_bits * 2;
            }

            if (min_bits..=max_bits).contains(&num_bits) {
                heap_refs[i] = heap_builder.claim(logic_mode, size);
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
) -> Result<(CDesignState, CDesign), Box<dyn std::error::Error>> {
    timers.start("lower to C");
    let mut listener_builder = ListenerBuilder::default();
    let mut out = Vec::new();
    let mut state_builder = StateBuilder::default();

    for signal in gl.signals.keys() {
        lower_signal_drive_header(&mut out, signal, &signal_map)?;
    }

    let lower_options = CLowerOptions {
        itrace,
        stats,
        num_plugins: plugins.len(),
    };
    for (i, process) in gl.processes.keys().enumerate() {
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
            &lower_options,
        )?;
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
            heap.store_bits(heap_refs[rt_key.as_usize()], gl.logic_mode, initialize);
            let is_unchanged = match gl.logic_mode {
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
