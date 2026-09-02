use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{ProcessBuilder, ProcessKind, SignalKey};
use vogls_utils::{IndexSet, OrderedSet};

use crate::ast::expr::Expr;
use crate::ast::module::AlwaysConstruct;
use crate::ast::statement::{
    EventControl, EventExpressionPrimary, ProceduralTimingControl, Statement, StatementContent,
    StatementOrNull,
};
use crate::ast::{AstId, AstIdRange};
use crate::lower::statement::{get_used_signals_stmt_or_null, lower_stmts};
use crate::lower::{LowerContext, MutLowerContext, try_resolve_net};

/// Lower a Verilog `always` construct to Vogls IR.
///
/// This construct keeps rerunning the associated statement.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, AlwaysConstruct<'a>>,
) -> Result<(), ()> {
    let statement = id.0;
    let (mut proc_builder, bb_builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Always, ctx.arenas.get_span(id));
    let entry_tr = proc_builder.entry();

    // Combination `always` get special handling to deal with real designs. You don't want
    // them to sporedically trigger. Instead, you want them to arm immediately at the
    // start. We use the `standing` field on a process for that.
    //
    // We define *combinational* always similar to Icarus Verilog, in that we say if your
    // `always` blocks only includes an event control with a STAR or identifiers that do
    // not include an edge.
    if let Some((signals, ectrl, stmt)) = extract_standing_signals(ctx, mctx, scope, statement)? {
        // Transform the process from
        //   `always @(...) stmt;`
        // to
        //   `always begin stmt; @(...); end`
        // with a standing watch.
        let mut bb_builder = lower_stmts(
            ctx,
            mctx,
            scope,
            &mut proc_builder,
            bb_builder,
            stmt.into_stmt_range(),
        )?;

        if signals.is_empty() {
            mctx.diagnostics.warnings.push((
                ctx.arenas.get_span(ectrl),
                "event control watches no signals and thus never triggers".into(),
            ));
            proc_builder.abort(mctx.gl());
            return Ok(());
        }

        bb_builder.watch_to(mctx.gl(), signals.clone().into(), entry_tr);

        proc_builder.set_standing(mctx.gl(), signals);
        proc_builder.finalize(mctx.gl());

        return Ok(());
    }

    let mut bb_builder = lower_stmts(
        ctx,
        mctx,
        scope,
        &mut proc_builder,
        bb_builder,
        AstIdRange::single(statement),
    )?;
    bb_builder.jump_to_tr(mctx.gl(), entry_tr);

    proc_builder.finalize(mctx.gl());

    Ok(())
}

/// Extract whether the the statement is of the shape `@(watch_list) stmt;` with `watch_list` only
/// containing `AnyEdge` signals.
fn extract_standing_signals<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    statement: AstId<'a, Statement<'a>>,
) -> Result<
    Option<(
        Box<[SignalKey]>,
        AstId<'a, EventControl<'a>>,
        AstId<'a, StatementOrNull<'a>>,
    )>,
    (),
> {
    let StatementContent::ProceduralTimingControlStatement(ptcs) = statement.content else {
        return Ok(None);
    };

    let ProceduralTimingControl::EventControl(ectrl) = &*ptcs.procedural_timing_control else {
        return Ok(None);
    };

    match &**ectrl {
        EventControl::Star => {
            let mut signals = OrderedSet::default();
            get_used_signals_stmt_or_null(ctx, mctx, scope, &mut signals, ptcs.statement_or_null)?;
            let mut signals = signals.items;
            signals.sort_unstable();
            Ok(Some((signals.into(), *ectrl, ptcs.statement_or_null)))
        }
        EventControl::EventExpression(event_expression) => {
            let mut signals = IndexSet::new();
            for event_expression in event_expression.0.iter() {
                let EventExpressionPrimary::Expression(expr) = &*event_expression else {
                    return Ok(None);
                };
                let Expr::Ident(ast_ident, exprs, range_expression) = &**expr else {
                    return Ok(None);
                };
                if !exprs.is_empty() || range_expression.is_some() {
                    return Ok(None);
                }

                let net = try_resolve_net(
                    scope,
                    &ctx.table,
                    ctx.arenas,
                    *ast_ident,
                    &mut mctx.diagnostics,
                )?;
                let signal = net.net.probe_signal();
                signals.insert(signal);
            }
            let mut signals = signals.take_keys();
            signals.sort_unstable();
            Ok(Some((signals.into(), *ectrl, ptcs.statement_or_null)))
        }
    }
}
