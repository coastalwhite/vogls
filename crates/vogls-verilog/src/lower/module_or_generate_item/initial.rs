use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{ProcessBuilder, ProcessKind};

use crate::ast::{AstId, AstIdRange};
use crate::ast::module::InitialConstruct;
use crate::lower::statement::statements_to_process;
use crate::lower::{LowerContext, MutLowerContext};

/// Lower a Verilog `initial` construct to Vogls IR.
///
/// This construct runs the associated statement once and then stops.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, InitialConstruct<'a>>,
) -> Result<(), ()> {
    let statement = id.0;

    let (process, bb_builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Initial, ctx.arenas.get_span(id));

    let bb_builder =
        statements_to_process(ctx, mctx, scope, bb_builder, AstIdRange::single(statement))?;
    bb_builder.halt(mctx.gl());
    
    process.finalize(mctx.gl());

    Ok(())
}
