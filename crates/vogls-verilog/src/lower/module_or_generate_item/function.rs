use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{GlobalContext, ProcessBuilder};
use vogls_utils::{SeenSet, VgHashMap};

use crate::ast::module::{FunctionDeclaration, TaskDeclaration};
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::{LoweredFunction, LoweredTask, VSymbol, determine_module_context};
use crate::lower::observed::Scan;
use crate::lower::{LowerContext, MutLowerContext};
use crate::lower::{unwrap_get_fn_mut, unwrap_get_task_mut};

pub fn observe_all<'a>(ctx: &LowerContext<'a, '_>, mctx: &mut MutLowerContext<'a>) {
    let diagnostics = std::mem::take(&mut mctx.diagnostics);
    let mut scan = Scan::default();
    for sid in ctx.table.symbol_id_iter() {
        if let VSymbol::Function(_) | VSymbol::Task(_) = ctx.table[sid].content {
            scan.subroutine(ctx, mctx, sid);
        }
    }
    mctx.diagnostics = diagnostics;
    scan.observe(mctx);
}

pub fn discard_all(ctx: &mut LowerContext<'_, '_>, gl: &mut GlobalContext) {
    let mut stack = SeenSet::new();
    for sid in ctx.table.symbol_id_iter() {
        let VSymbol::Function(f) = &mut ctx.table[sid].content else {
            continue;
        };
        let Some(LoweredFunction { entry, terminate }) = f.lowered.take() else {
            continue;
        };

        stack.clear();
        stack.insert(entry);
        stack.insert(terminate);
        while let Some(bb) = stack.pop() {
            if let Some(bb) = gl.bbs.remove(bb) {
                bb.terminator
                    .for_each_temporal_bb(|next| _ = stack.insert(next));
            }
        }
    }
}

pub fn lower_all<'a>(
    ctx: &mut LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
) -> Result<(), ()> {
    let mut states = VgHashMap::default();
    let mut error = false;
    for sid in ctx.table.symbol_id_iter() {
        if let VSymbol::Function(_) | VSymbol::Task(_) = ctx.table[sid].content {
            error |= lower_on_demand(ctx, mctx, &mut states, sid).is_err();
        }
    }
    if error { Err(()) } else { Ok(()) }
}

fn lower_on_demand<'a>(
    ctx: &mut LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    states: &mut VgHashMap<SymbolId, Option<Result<(), ()>>>,
    sid: SymbolId,
) -> Result<(), ()> {
    match states.get(&sid) {
        Some(Some(result)) => return *result,
        // A recursive call, which is reported when the call is lowered.
        Some(None) => return Ok(()),
        None => {}
    }
    states.insert(sid, None);

    let diagnostics = std::mem::take(&mut mctx.diagnostics);
    let mut scan = Scan::default();
    scan.subroutine(ctx, mctx, sid);
    mctx.diagnostics = diagnostics;

    // The errors of a callee are reported by the callee itself.
    let mut result = Ok(());
    for &callee in &scan.calls.items {
        result = result.and(lower_on_demand(ctx, mctx, states, callee));
    }

    if result.is_ok() {
        let (_, module) = determine_module_context(sid, &ctx.table);
        ctx.time_scale = module.time_scale;
        result = match &ctx.table[sid].content {
            VSymbol::Function(f) => {
                let id = ctx.table_ast_refs.fns[f.ast_id];
                lower(ctx, mctx, sid, id)
            }
            VSymbol::Task(t) => {
                let id = ctx.table_ast_refs.tasks[t.ast_id];
                lower_task(ctx, mctx, sid, id)
            }
            _ => unreachable!("not a function or task"),
        };
    }

    states.insert(sid, Some(result));
    result
}

pub fn lower<'a>(
    ctx: &mut LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    id: AstId<'a, FunctionDeclaration<'a>>,
) -> Result<(), ()> {
    let FunctionDeclaration {
        automatic: _,
        range_or_type: _,
        ident: _,
        tf_input_decls: _,
        block_item_decls: _,
        statement,
    } = &*id;

    let (mut proc_builder, builder) = ProcessBuilder::new_anonymous(mctx.gl());
    let entry_key = builder.key();

    let builder = crate::lower::statement::lower_stmts(
        ctx,
        mctx,
        scope,
        &mut proc_builder,
        builder,
        AstIdRange::single(*statement),
    )?;

    let terminate_key = builder.key();
    builder.halt(mctx.gl());
    proc_builder.finalize(mctx.gl());

    unwrap_get_fn_mut(&mut ctx.table, scope).lowered = Some(LoweredFunction {
        entry: entry_key,
        terminate: terminate_key,
    });

    Ok(())
}

pub fn lower_task<'a>(
    ctx: &mut LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    id: AstId<'a, TaskDeclaration<'a>>,
) -> Result<(), ()> {
    let TaskDeclaration {
        automatic: _,
        ident: _,
        task_ports: _,
        block_item_decls: _,
        statement_or_null,
    } = &*id;

    let (mut proc_builder, builder) = ProcessBuilder::new_anonymous(mctx.gl());

    let entry_key = builder.key();

    let builder = crate::lower::statement::lower_stmts(
        ctx,
        mctx,
        scope,
        &mut proc_builder,
        builder,
        statement_or_null.as_id_range(),
    )?;

    let terminate_key = builder.key();
    builder.halt(mctx.gl());
    proc_builder.finalize(mctx.gl());

    unwrap_get_task_mut(&mut ctx.table, scope).lowered = Some(LoweredTask {
        entry: entry_key,
        terminate: terminate_key,
    });

    Ok(())
}
