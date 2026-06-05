pub use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
pub use vogls_ir::{Bits, LogicMode, SignalKey, VectorSize};
pub use vogls_runtime::{RtSignalKey, SimulationIo};
pub use vogls_sim::SimulationState;
pub use vogls_verilog::elaborate::{VSymbol, VSymbolTable};

pub use vogls_bits as bits;
pub use vogls_codegen as codegen;
pub use vogls_frontend as frontend;
pub use vogls_ir as ir;
pub use vogls_runtime as runtime;
pub use vogls_sim as sim;
pub use vogls_utils as utils;

pub mod design;
pub mod symbol;
pub mod timing;

mod design_builder;
mod elaborated_design;
mod lowered_design;
mod parsed_design;
mod plugin;
mod vir_design_builder;

pub use design::{Design, DesignBackend, DesignState};
pub use design_builder::{DesignBuilder, DesignBuilderError};
pub use elaborated_design::{ElaboratedDesign, ElaborationError, SignalHandle};
pub use lowered_design::{LowerError, LoweredDesign};
pub use parsed_design::{ParseError, ParseErrorReason, ParsedDesign};
pub use plugin::VoglsPlugin;
pub use vir_design_builder::VirDesignBuilder;

// @TODO: Wrap this in a stable API somehow.
pub use vogls_frontend::diagnostic::Diagnostic;

#[cfg(feature = "unstable")]
pub use lowered_design::LowerStats;

// pub mod symbolic_execution;

// - Diagnostics
//    - Error
//    - Reporting
//
// - Bits
//
// - DesignBuilder
// - ParsedDesign
// - ElaboratedDesign
//   - SignalHandle
// - VIRDesignBuilder
// - LoweredDesign
//   - EmitDesignIr
//   - OptFlags
// - Design
//   - CompiledDesign
//   - BytecodeDesign

// pub fn run(
//     path: &[&Path],
//     timers: &mut TimerStack,
//     top_level_module: Option<&str>,
//     ectx: &mut ExecutionContext,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let mut design = timers.timed("total compilation", |timers| {
//         design::Design::new(path, timers, top_level_module, ectx, Vec::new())
//     })?;
//
//     if ectx.no_run {
//         return Ok(());
//     }
//
//     let stdout = std::mem::replace(&mut ectx.stdout, Box::new(Vec::new()) as _);
//     let stderr = std::mem::replace(&mut ectx.stderr, Box::new(Vec::new()) as _);
//     let mut io = SimulationIo::new(stdout, stderr);
//
//     timers.start("simulation");
//     design
//         .run(&mut io, ectx.time)
//         .map_err(|_| <Box<dyn std::error::Error>>::from("execution failed."))?;
//     timers.stop();
//
//     ectx.stdout = io.stdout;
//     ectx.stderr = io.stderr;
//
//     Ok(())
// }
