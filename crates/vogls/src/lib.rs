pub use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
use vogls_frontend::ident_table::IdentTable;
use vogls_frontend::symbol_table::SymbolId;
pub use vogls_ir::optimize::{OptFlags, Optimizations};
pub use vogls_ir::{Bits, LogicMode, SignalKey, VectorSize};
pub use vogls_runtime::RtSignalKey;
pub use vogls_verilog::elaborate::{VSymbol, VSymbolTable};
pub use vogls_verilog::parser::ParserOptions;
pub use vogls_world::{NeverWorld, World};
#[cfg(feature = "stdworld")]
pub use vogls_world::{std::StdWorld, std::StdWorldCaptured};

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

/// Resolves the symbol from a `.` separated path.
pub fn find_symbol_from_path(
    table: &VSymbolTable,
    ident_table: &IdentTable,
    scope: &str,
) -> Option<SymbolId> {
    fn take_segment<'a>(s: &'a str) -> (&'a str, &'a str) {
        let s = s.trim_start();

        if s.starts_with('\\') {
            let s = &s[1..];
            return s
                .split_once(|c: char| c.is_ascii_whitespace())
                .unwrap_or((s, ""));
        }

        let bs = s.as_bytes();
        if bs
            .first()
            .is_none_or(|b| !matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'_'))
        {
            return ("", s);
        }
        let end = bs[1..]
            .iter()
            .position(|b| !matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'$'))
            .map_or(bs.len(), |i| i + 1);
        s.split_at(end)
    }

    let mut rest = scope.trim();
    let fst_segment;
    (fst_segment, rest) = take_segment(rest);

    let fst_segment = ident_table.get(fst_segment)?;
    let mut sid = match table.resolve_root(fst_segment) {
        Some(sid) => sid,
        None => match table.roots() {
            &[root] => table.resolve(root, fst_segment)?,
            _ => return None,
        },
    };

    while !rest.is_empty() {
        rest = rest.trim_start().strip_prefix('.')?;

        let segment;
        (segment, rest) = take_segment(rest);
        let segment = ident_table.get(segment)?;
        sid = table.resolve(sid, segment)?;
    }

    Some(sid)
}
