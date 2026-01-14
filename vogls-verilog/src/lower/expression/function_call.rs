use std::collections::{HashMap, HashSet};

use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, ConnectionDirection, GlobalContext, VariableKey,
};

use crate::ast::expr::Expr;
use crate::ast::{AstId, AstIdRange, AstItem, Identifier};
use crate::hierarchy::HierarchyItem;
use crate::lower::expression::{lower_expr, truncate_or_extend};
use crate::lower::{Diagnostics, VType};
use crate::lower::{Scope, assign_task_output};
use crate::parser::AstArenas;

pub fn lower_function_call<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    expr: AstId<Expr>,
    ident: AstItem<Identifier>,
    arguments: &[Option<(VariableKey, VType)>],
) -> Result<(VariableKey, VType), ()> {
    let fn_name = arenas.get_ident(ident.item.0);
    let Some(fn_symbol) = scope.get(fn_name) else {
        diagnostics.var_not_found(arenas, ident);
        return Err(());
    };
    let HierarchyItem::Function(i) = &scope.hierarchy.symbols[fn_symbol.as_idx()] else {
        diagnostics.not_yet_implemented(arenas.get_item_span(ident), "not calling a function");
        return Err(());
    };
    // @TODO: Error handling
    let fn_symbol = &scope.hierarchy.functions[*i].lower.as_ref().unwrap();

    assert_eq!(fn_symbol.input_vars.len(), fn_symbol.input_types.len());
    if fn_symbol.input_vars.len() != arguments.len() {
        diagnostics.not_yet_implemented(arenas.get_span(expr), "invalid number of arguments");
        return Err(());
    }

    let mut map = HashMap::new();
    for i in 0..fn_symbol.input_vars.len() {
        let Some((arg_variable, arg_ty)) = arguments[i] else {
            return Err(());
        };
        let input_var = fn_symbol.input_vars[i];
        let input_ty = fn_symbol.input_types[i];
        let arg_variable = truncate_or_extend(
            gl,
            builder,
            arg_variable,
            arg_ty,
            input_ty.force_net_width(),
        );
        map.insert(input_var, arg_variable);
    }

    let mut fn_bb = gl.bbs[fn_symbol.entry].clone();
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

    Ok((map[&fn_symbol.output_var], fn_symbol.output_ty))
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
    let fn_name = arenas.get_ident(ident.item.0);
    let Some(fn_symbol) = scope.get(fn_name) else {
        diagnostics.var_not_found(arenas, ident);
        return Err(());
    };
    let HierarchyItem::Task(i) = &scope.hierarchy.symbols[fn_symbol.as_idx()] else {
        diagnostics.not_yet_implemented(arenas.get_item_span(ident), "not enabling a task");
        return Err(());
    };
    // @TODO: Error handling
    let task_symbol = &scope.hierarchy.tasks[*i].lower.as_ref().unwrap();

    assert_eq!(task_symbol.io_vars.len(), task_symbol.io_types.len());
    if task_symbol.io_vars.len() != arguments.len() {
        diagnostics.not_yet_implemented(arenas.get_item_span(ident), "invalid number of arguments");
        return Err(());
    }

    let mut map = HashMap::new();
    for i in 0..task_symbol.io_vars.len() {
        let (direction, input_ty) = task_symbol.io_types[i];
        if !matches!(
            direction,
            ConnectionDirection::In | ConnectionDirection::Both
        ) {
            continue;
        }

        let arg = arguments.get(i);
        let input_var = task_symbol.io_vars[i];

        let (arg_variable, arg_ty) = lower_expr(gl, arenas, scope, diagnostics, &mut builder, arg)?;
        let arg_variable = truncate_or_extend(
            gl,
            &mut builder,
            arg_variable,
            arg_ty,
            input_ty.force_net_width(),
        );
        map.insert(input_var, arg_variable);
    }

    let mut bb_stack = Vec::new();
    let mut bb_map = HashMap::new();

    let fn_bb = gl.bbs.insert(gl.bbs[task_symbol.entry].clone());
    let mut terminator_bb = fn_bb;

    bb_stack.push(fn_bb);
    bb_map.insert(task_symbol.entry, fn_bb);
    while let Some(bb_key) = bb_stack.pop() {
        let terminator = gl.bbs[bb_key].terminator.clone();
        if matches!(terminator, BasicBlockTerminator::Halt) {
            terminator_bb = bb_key;
        }

        terminator.for_each_bb(|bb| {
            _ = bb_map.entry(bb).or_insert_with(|| {
                let new_bb = gl.bbs.insert(gl.bbs[bb].clone());
                bb_stack.push(new_bb);
                new_bb
            })
        });
        gl.bbs[bb_key].terminator.map_bb(|bb| bb_map[&bb]);
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
        gl.bbs[bb_key].terminator.extend_next_rev(&mut bb_stack, &mut bb_seen);
    }

    for i in 0..task_symbol.io_vars.len() {
        let (direction, output_ty) = task_symbol.io_types[i];
        if !matches!(
            direction,
            ConnectionDirection::Out | ConnectionDirection::Both
        ) {
            continue;
        }

        let arg = arguments.get(i);
        let output_var = task_symbol.io_vars[i];
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
    gl.bbs[terminator_bb].terminator = BasicBlockTerminator::Jump(builder.key());

    Ok(builder)
}
