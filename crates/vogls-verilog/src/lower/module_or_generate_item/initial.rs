use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{ProcessBuilder, ProcessKind};

use crate::ast::module::InitialConstruct;
use crate::ast::{AstId, AstIdRange};
use crate::lower::statement::lower_stmts;
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

    let (mut proc_builder, bb_builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Initial, ctx.arenas.get_span(id));

    let bb_builder = lower_stmts(
        ctx,
        mctx,
        scope,
        &mut proc_builder,
        bb_builder,
        AstIdRange::single(statement),
    )?;
    bb_builder.halt(mctx.gl());

    proc_builder.finalize(mctx.gl());

    Ok(())
}
