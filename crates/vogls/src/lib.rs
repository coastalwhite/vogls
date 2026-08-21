pub use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
pub use vogls_ir::optimize::{OptFlags, Optimizations};
pub use vogls_ir::{Bits, LogicMode, SignalKey, VectorSize};
pub use vogls_runtime::RtSignalKey;
pub use vogls_verilog::elaborate::{VSymbol, VSymbolTable};
pub use vogls_world::{NeverWorld, World, std::StdWorld, std::StdWorldCaptured};

pub use vogls_bits as bits;
pub use vogls_bytecode as sim;
pub use vogls_codegen as codegen;
pub use vogls_frontend as frontend;
pub use vogls_ir as ir;
pub use vogls_runtime as runtime;
pub use vogls_utils as utils;

pub mod design;
pub mod symbol;
pub mod timing;

mod design_builder;
mod elaborated_design;
mod lowered_design;
mod parsed_design;
mod plugin;
#[cfg(test)]
mod tests;
mod vir_design_builder;

#[cfg(feature = "unstable")]
pub mod sync;

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
