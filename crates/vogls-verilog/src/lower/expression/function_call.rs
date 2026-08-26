use std::collections::{HashMap, HashSet};

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, ConnectionDirection, ProcessBuilder, TemporalRegionKey, VariableKey,
};
use vogls_utils::VgHashSet;

use crate::ast::expr::Expr;
use crate::ast::statement::TaskEnable;
use crate::ast::{AstId, HIdent};
use crate::elaborate::VSymbol;
use crate::lower::expression::{self, lower_expr};
use crate::lower::{LowerContext, assign_task_output};
use crate::lower::{MutLowerContext, VType, hident_span, try_resolve_hident};

pub fn lower_function_call<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    expr: AstId<Expr>,
    ident: HIdent,
    arguments: &[Option<(VariableKey, VType)>],
) -> Result<(VariableKey, VType), ()> {
    let fn_symbol =
        try_resolve_hident(scope, &ctx.table, ctx.arenas, ident, &mut mctx.diagnostics)?;
    let VSymbol::Function(fn_symbol) = &ctx.table[fn_symbol].content else {
        mctx.diagnostics
            .not_yet_implemented(hident_span(ctx.arenas, ident), "not calling a function");
        return Err(());
    };

    let lowered = fn_symbol.lowered.as_ref().unwrap();
    if fn_symbol.inputs.len() != arguments.len() {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_span(expr), "invalid number of arguments");
        return Err(());
    }

    let mut map = HashMap::new();
    for i in 0..fn_symbol.inputs.len() {
        let (input_signal, input_ty) = fn_symbol.inputs[i];
        let Some((arg_variable, arg_ty)) = arguments[i] else {
            return Err(());
        };
        let arg_variable = expression::coerce_to(
            mctx.gl(),
            builder,
            arg_variable,
            arg_ty,
            input_ty.resize_net_to(input_ty.bit_length()),
        );
        builder.drive(mctx.gl(), input_signal, arg_variable);
    }

    let mut bb_stack = Vec::new();
    let mut bb_map = HashMap::new();
    let mut bb_seen = HashSet::new();

    let gl = mctx.gl();
    let fn_bb = gl.bbs.insert(gl.bbs[lowered.entry].clone());

    bb_stack.push(fn_bb);
    bb_map.insert(lowered.entry, fn_bb);
    while let Some(bb_key) = bb_stack.pop() {
        let terminator = gl.bbs[bb_key].terminator.clone();
        terminator.for_each_temporal_bb(|bb| {
            _ = bb_map.entry(bb).or_insert_with(|| {
                let new_bb = gl.bbs.insert(gl.bbs[bb].clone());
                bb_stack.push(new_bb);
                new_bb
            })
        });
        gl.bbs[bb_key].map_vars(|v| {
            *map.entry(v).or_insert_with(|| {
                let size = gl.vars.size(v);
                gl.vars.insert(v.mode(), size)
            })
        });
    }

    let fn_entry_tr = TemporalRegionKey::from_entry(lowered.entry);

    bb_stack.push(fn_bb);
    bb_seen.insert(fn_bb);
    while let Some(bb_key) = bb_stack.pop() {
        if gl.bbs[bb_key].region != fn_entry_tr {
            mctx.diagnostics
                .not_yet_implemented(ctx.arenas.get_span(expr), "temporal function");
            return Err(());
        }

        gl.bbs[bb_key].map_temporal_bbs(|bb| bb_map[&bb]);
        gl.bbs[bb_key].region = builder.tr();
        gl.bbs[bb_key].terminator.for_each_temporal_bb(|next_bb| {
            if bb_seen.insert(next_bb) {
                bb_stack.push(next_bb);
            }
        });
    }

    let after = builder.new_basic_block(mctx.gl());

    builder.jump_to(mctx.gl(), fn_bb);
    if let Some(terminate) = bb_map.get(&lowered.terminate) {
        // Procedure might contain infinite loop.
        builder.finished_switch_to(mctx.gl(), *terminate);
        builder.jump_to(mctx.gl(), after);
    }

    builder.finished_switch_to(mctx.gl(), after);
    let output_var = builder.probe(mctx.gl(), fn_symbol.output);
    Ok((output_var, fn_symbol.output_ty))
}

pub fn lower_task_enable<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    proc_builder: &mut ProcessBuilder,
    mut builder: BasicBlockBuilder,
    task_enable: AstId<'a, TaskEnable<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let TaskEnable { ident, exprs } = *task_enable;
    let fn_symbol =
        try_resolve_hident(scope, &ctx.table, ctx.arenas, ident, &mut mctx.diagnostics)?;
    let VSymbol::Task(task_symbol) = &ctx.table[fn_symbol].content else {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_item_span(ident), "not enabling a task");
        return Err(());
    };

    let lowered = task_symbol.lowered.as_ref().unwrap();
    if task_symbol.io.len() != exprs.len() {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_item_span(ident),
            "invalid number of arguments",
        );
        return Err(());
    }

    let mut map = HashMap::new();
    for i in 0..task_symbol.io.len() {
        let (signal, direction, input_ty) = task_symbol.io[i];
        if !matches!(
            direction,
            ConnectionDirection::In | ConnectionDirection::Both
        ) {
            continue;
        }

        let arg = exprs.get(i);

        let (arg_variable, arg_ty) = lower_expr(
            ctx,
            mctx,
            scope,
            &mut builder,
            arg,
            Some(input_ty.bit_length()),
        )?;
        let arg_variable =
            expression::coerce_to(mctx.gl(), &mut builder, arg_variable, arg_ty, input_ty);
        builder.drive(mctx.gl(), signal, arg_variable);
    }

    let mut bb_stack = Vec::new();
    let mut bb_map = HashMap::new();

    let gl = &mut mctx.gl;
    let fn_bb = gl.bbs.insert(gl.bbs[lowered.entry].clone());

    bb_stack.push(fn_bb);
    bb_map.insert(lowered.entry, fn_bb);
    while let Some(bb_key) = bb_stack.pop() {
        let terminator = gl.bbs[bb_key].terminator.clone();
        terminator.for_each_temporal_bb(|bb| {
            _ = bb_map.entry(bb).or_insert_with(|| {
                let new_bb = gl.bbs.insert(gl.bbs[bb].clone());
                bb_stack.push(new_bb);
                new_bb
            })
        });
        gl.bbs[bb_key].map_vars(|v| {
            *map.entry(v).or_insert_with(|| {
                let size = gl.vars.size(v);
                gl.vars.insert(v.mode(), size)
            })
        });
    }

    let mut bb_seen = HashSet::new();
    let mut trs = VgHashSet::default();
    bb_stack.push(fn_bb);
    bb_seen.insert(fn_bb);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = &mut mctx.gl.bbs[bb_key];
        bb.map_temporal_bbs(|tgt| bb_map[&tgt]);
        if trs.insert(bb.region) {
            proc_builder.push_temporal_region(bb.region);
        }
        bb.terminator.for_each_temporal_bb(|next_bb| {
            if bb_seen.insert(next_bb) {
                bb_stack.push(next_bb);
            }
        });
    }

    for i in 0..task_symbol.io.len() {
        let (signal, direction, output_ty) = task_symbol.io[i];
        if !matches!(
            direction,
            ConnectionDirection::Out | ConnectionDirection::Both
        ) {
            continue;
        }

        let arg = exprs.get(i);
        let output_var = builder.probe(mctx.gl(), signal);
        assign_task_output(ctx, mctx, scope, &mut builder, output_var, arg, output_ty)?;
    }

    let fn_tr = mctx.gl.bbs[fn_bb].region;
    let after_tr = proc_builder.next_temporal_region(mctx.gl());

    builder.temporal_jump_to(mctx.gl(), fn_tr);
    if let Some(terminate) = bb_map.get(&lowered.terminate) {
        // Procedure might contain infinite loop.
        builder.finished_switch_to(mctx.gl(), *terminate);
        builder.temporal_jump_to(mctx.gl(), after_tr);
    }
    builder.finished_switch_to(mctx.gl(), after_tr.entry());

    Ok(builder)
}
