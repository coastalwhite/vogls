use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
use vogls_frontend::ident_table::IdentId;
use vogls_ir::token_range::TokenRange;
pub use vogls_ir::{Bits, LogicMode, SignalKey, VectorSize};
pub use vogls_sim::{SimulationIo, SimulationState, VcdOutput};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{
    CaseGenerateConstruct, CaseGenerateItem, GenerateBlock, IfGenerateConstruct,
    LoopGenerateConstruct, ModuleOrGenerateItem, ModuleOrGenerateItemContent,
};
pub use vogls_verilog::elaborate::VSymbol;
use vogls_verilog::parser::AstArenas;
use vogls_verilog::tokenizer::Tokenized;

pub use vogls_utils as utils;
pub use vogls_sim as sim;
pub use vogls_bits as bits;

pub mod design;

pub struct ExecutionContext {
    pub stdout: Box<dyn std::io::Write + Send + Sync>,
    pub stderr: Box<dyn std::io::Write + Send + Sync>,
    pub defines: Vec<String>,
    pub emit_hierarchy: bool,
    pub emit_unoptimized_ir: bool,
    pub emit_ir: bool,
    pub emit_vm: bool,
    pub trace: bool,
    pub itrace: bool,
    pub time: u64,
    pub opt_rounds: u8,
    pub logic_mode: LogicMode,
    pub no_run: bool,
    pub vcd: Option<PathBuf>,
}

pub fn token_range_to_line_range(
    tokenized: &Tokenized,
    tr: TokenRange,
    line_luts: &[Vec<usize>],
) -> Option<vogls_trace::Span> {
    let file = tokenized.file_idxs[tr.start];
    if file == tokenized.file_idxs[tr.end - 1] {
        let span_start = tokenized.spans[tr.start].start();
        let span_end = tokenized.spans[tr.end - 1].end();

        let line_start = line_luts[file as usize]
            .binary_search(&span_start)
            .unwrap_or_else(|e| e - 1) as u64;
        let line_end = line_luts[file as usize]
            .binary_search(&span_end)
            .unwrap_or_else(|e| e) as u64;
        return Some(vogls_trace::Span {
            file: file as u64,
            line_range: line_start..line_end,
        });
    }

    None
}

fn append_referenced_modules_generate_block<'a>(
    arenas: &'a AstArenas,
    generate_block: AstId<GenerateBlock>,
    referenced: &mut HashSet<IdentId>,
) {
    match arenas.get(generate_block) {
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
    generate_block: AstId<Option<GenerateBlock>>,
    referenced: &mut HashSet<IdentId>,
) {
    match arenas.get(generate_block) {
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
    module_or_generate_item: AstId<ModuleOrGenerateItem>,
    referenced: &mut HashSet<IdentId>,
) {
    match arenas.get(module_or_generate_item).content {
        ModuleOrGenerateItemContent::ModuleInstantiation(module_instantiation) => {
            let module_instantiation = arenas.get(module_instantiation);
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
            } = arenas.get(loop_generate_construct);
            append_referenced_modules_generate_block(arenas, *block, referenced);
        }
        ModuleOrGenerateItemContent::IfGenerateConstruct(if_generate_construct) => {
            let IfGenerateConstruct {
                condition: _,
                truthy,
                falsy,
            } = arenas.get(if_generate_construct);
            append_referenced_modules_opt_generate_block(arenas, *truthy, referenced);
            if let Some(falsy) = falsy {
                append_referenced_modules_opt_generate_block(arenas, *falsy, referenced);
            }
        }
        ModuleOrGenerateItemContent::CaseGenerateConstruct(case_generate_construct) => {
            let CaseGenerateConstruct { value: _, items } = arenas.get(case_generate_construct);
            for item in items.iter() {
                let CaseGenerateItem { pattern: _, block } = arenas.get(item);
                append_referenced_modules_opt_generate_block(arenas, *block, referenced);
            }
        }
    }
}

pub fn run(
    path: &[&Path],
    top_level_module: Option<&str>,
    ectx: &mut ExecutionContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut design = design::Design::new(path, top_level_module, ectx)?;

    if ectx.no_run {
        return Ok(());
    }

    let stdout = std::mem::replace(&mut ectx.stdout, Box::new(Vec::new()) as _);
    let stderr = std::mem::replace(&mut ectx.stderr, Box::new(Vec::new()) as _);
    let mut io = SimulationIo::new(stdout, stderr);

    design
        .simulation
        .run(&mut design.initial_state, &mut io, ectx.time)
        .map_err(|_| <Box<dyn std::error::Error>>::from("execution failed."))?;

    ectx.stdout = io.stdout;
    ectx.stderr = io.stderr;

    Ok(())
}
