use std::collections::{HashMap, HashSet};

use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, ConnectionDirection, GlobalContext, VariableKey,
};

use crate::ast::expr::Expr;
use crate::ast::{AstId, AstIdRange, AstItem, HIdent, Identifier};
use crate::elaborate::VSymbol;
use crate::lower::expression::{lower_expr, truncate_or_extend};
use crate::lower::{Diagnostics, VType, hident_span, try_resolve_symbol_id};
use crate::lower::{Scope, assign_task_output};
use crate::parser::AstArenas;

pub fn lower_function_call<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    expr: AstId<Expr>,
    ident: HIdent,
    arguments: &[Option<(VariableKey, VType)>],
) -> Result<(VariableKey, VType), ()> {
    let fn_symbol = try_resolve_symbol_id(scope.key, scope.table, arenas, ident, diagnostics)?;
    let VSymbol::Function(fn_symbol) = &scope.table[fn_symbol].content else {
        diagnostics.not_yet_implemented(hident_span(arenas, ident), "not calling a function");
        return Err(());
    };

    let lowered = fn_symbol.lowered.as_ref().unwrap();
    if fn_symbol.inputs.len() != arguments.len() {
        diagnostics.not_yet_implemented(arenas.get_span(expr), "invalid number of arguments");
        return Err(());
    }

    let mut map = HashMap::new();
    for i in 0..fn_symbol.inputs.len() {
        let (input_signal, input_ty) = fn_symbol.inputs[i] else {
            return Err(());
        };
        let Some((arg_variable, arg_ty)) = arguments[i] else {
            return Err(());
        };
        let arg_variable = truncate_or_extend(
            gl,
            builder,
            arg_variable,
            arg_ty,
            input_ty.force_net_width(),
        );
        builder.drive(gl, input_signal, arg_variable);
    }

    let mut fn_bb = gl.bbs[lowered.entry].clone();
    fn_bb.map_vars(|v| {
        *map.entry(v).or_insert_with(|| {
            let fn_var = gl.vars[v].clone();
            gl.vars.insert(fn_var)
        })
    });

    let origin_bb = builder.key();
    *builder = builder.next_terminate_later(gl);
    fn_bb.terminator = BasicBlockTerminator::Jump(builder.key());
    let fn_bb = gl.bbs.insert(fn_bb);
    gl.bbs[origin_bb].terminator = BasicBlockTerminator::Jump(fn_bb);

    let output_var = builder.probe(gl, fn_symbol.output);

    Ok((output_var, fn_symbol.output_ty.clone()))
}

pub fn lower_task_enable<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    ident: AstItem<Identifier>,
    arguments: AstIdRange<Expr>,
) -> Result<BasicBlockBuilder, ()> {
    let fn_symbol = try_resolve_symbol_id(scope.key, scope.table, arenas, ident, diagnostics)?;
    let VSymbol::Task(task_symbol) = &scope.table[fn_symbol].content else {
        diagnostics.not_yet_implemented(arenas.get_item_span(ident), "not enabling a task");
        return Err(());
    };

    let lowered = task_symbol.lowered.as_ref().unwrap();
    if task_symbol.io.len() != arguments.len() {
        diagnostics.not_yet_implemented(arenas.get_item_span(ident), "invalid number of arguments");
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

        let arg = arguments.get(i);

        let (arg_variable, arg_ty) = lower_expr(gl, arenas, scope, diagnostics, &mut builder, arg)?;
        let arg_variable = truncate_or_extend(
            gl,
            &mut builder,
            arg_variable,
            arg_ty,
            input_ty.force_net_width(),
        );
        builder.drive(gl, signal, arg_variable);
    }

    let mut bb_stack = Vec::new();
    let mut bb_map = HashMap::new();

    let fn_bb = gl.bbs.insert(gl.bbs[lowered.entry].clone());

    bb_stack.push(fn_bb);
    bb_map.insert(lowered.entry, fn_bb);
    while let Some(bb_key) = bb_stack.pop() {
        let terminator = gl.bbs[bb_key].terminator.clone();
        terminator.for_each_bb(|bb| {
            _ = bb_map.entry(bb).or_insert_with(|| {
                let new_bb = gl.bbs.insert(gl.bbs[bb].clone());
                bb_stack.push(new_bb);
                new_bb
            })
        });
        gl.bbs[bb_key].map_vars(|v| {
            *map.entry(v).or_insert_with(|| {
                let fn_var = gl.vars[v].clone();
                gl.vars.insert(fn_var)
            })
        });
    }

    let mut bb_seen = HashSet::new();
    bb_stack.push(fn_bb);
    bb_seen.insert(fn_bb);
    while let Some(bb_key) = bb_stack.pop() {
        gl.bbs[bb_key].map_bbs(|bb| bb_map[&bb]);
        gl.bbs[bb_key]
            .terminator
            .extend_next_rev(&mut bb_stack, &mut bb_seen);
    }

    for i in 0..task_symbol.io.len() {
        let (signal, direction, output_ty) = task_symbol.io[i];
        if !matches!(
            direction,
            ConnectionDirection::Out | ConnectionDirection::Both
        ) {
            continue;
        }

        let arg = arguments.get(i);
        let output_var = builder.probe(gl, signal);
        assign_task_output(
            gl,
            arenas,
            scope,
            diagnostics,
            &mut builder,
            output_var,
            arg,
            output_ty,
        )?;
    }

    let origin_bb = builder.key();
    let builder = builder.next_terminate_later(gl);
    gl.bbs[origin_bb].terminator = BasicBlockTerminator::Jump(fn_bb);
    if let Some(terminate) = bb_map.get(&lowered.terminate) {
        // Procedure might contain infinite loop.
        gl.bbs[*terminate].terminator = BasicBlockTerminator::Jump(builder.key());
    }

    Ok(builder)
}
