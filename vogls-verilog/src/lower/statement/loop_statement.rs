use std::collections::HashMap;

use vogls_ir::{BasicBlockBuilder, GlobalContext, PhiRef, Value};

use crate::ast::statement::{LoopStatement, LoopStatementVariant};
use crate::ast::{AstId, AstIdRange};
use crate::lower::scope::{Scope, SymbolKey, SymbolVariant};
use crate::lower::{
    assign_variable_lvalue, get_intersect_symbols_generated, lower_expr, statements_to_process,
};
use crate::parser::AstArenas;

pub fn lower_loop_statement<'a>(
    mut builder: BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    ls: AstId<LoopStatement>,
    arenas: &'a AstArenas,
) -> BasicBlockBuilder {
    use LoopStatementVariant as V;

    let ls = arenas.get(ls);
    let statement = arenas.get(ls.statement);

    let predecessor = builder.key();
    let mut repeat_vars = None;
    match ls.variant {
        V::Repeat(size) => {
            let size = lower_expr(&mut builder, gl, scope, arenas.get(size), arenas);
            let i = builder.constant(gl, Value::Decimal(0));
            repeat_vars = Some((i, size));
        }
        V::For(initialization, _, _) => {
            let initialization = arenas.get(initialization);
            let initialization_var = lower_expr(
                &mut builder,
                gl,
                scope,
                arenas.get(initialization.expr),
                arenas,
            );
            assign_variable_lvalue(
                gl,
                &mut builder,
                scope,
                arenas.get(initialization.lvalue),
                initialization_var,
                arenas,
            );
        }
        V::Forever | V::While(_) => {}
    }

    builder = builder.jump(gl);

    let mut intersect_vars_generated =
        get_intersect_symbols_generated(gl, &*scope, AstIdRange::single(ls.statement), arenas);
    if let V::For(_, _, step) = ls.variant {
        let step = arenas.get(step);
        let step_lvalue = arenas.get(step.lvalue);
        let step_symbol_key = scope
            .get(arenas.get_ident(step_lvalue.ident.item.0))
            .unwrap();
        intersect_vars_generated.push(step_symbol_key);
    }

    let mut phi_refs = HashMap::<SymbolKey, PhiRef>::new();
    for symkey in &intersect_vars_generated {
        match &mut scope.symbols[*symkey].variant {
            SymbolVariant::Signal(_) => {}
            SymbolVariant::Variable(None) => {}
            SymbolVariant::Variable(Some(current_var)) => {
                // @TODO: predecessor might be wrong here.
                let (phi_value, phi) = builder.phi(
                    gl,
                    [(predecessor, *current_var), (predecessor, *current_var)].into(),
                );
                phi_refs.insert(*symkey, phi);
                *current_var = phi_value;
                scope.assign(*symkey, phi_value);
            }
        }
    }

    let loop_start = builder.key();
    let mut repeat_i_phi = None;
    let condition = match ls.variant {
        V::Forever => None,
        V::Repeat(_) => {
            let (i, size) = repeat_vars.as_mut().unwrap();
            let phi_ref;
            (*i, phi_ref) = builder.phi(gl, [(predecessor, *i), (predecessor, *i)].into());
            repeat_i_phi = Some(phi_ref);
            Some(builder.unsigned_lt(gl, *i, *size))
        }
        V::While(condition) => Some(lower_expr(
            &mut builder,
            gl,
            scope,
            arenas.get(condition),
            arenas,
        )),
        V::For(_, condition, _) => Some(lower_expr(
            &mut builder,
            gl,
            scope,
            arenas.get(condition),
            arenas,
        )),
    };

    let branch_ref = match condition {
        None => None,
        Some(condition) => {
            let branch_ref;
            (branch_ref, builder) = builder.branch(gl, condition);
            Some(branch_ref)
        }
    };

    {
        scope.push_scope();
        builder =
            statements_to_process(builder, gl, scope, std::slice::from_ref(statement), arenas);
        scope.pop_scope();
    }

    match ls.variant {
        V::For(_, _, step) => {
            let step = arenas.get(step);
            let step_var = lower_expr(&mut builder, gl, scope, arenas.get(step.expr), arenas);
            assign_variable_lvalue(
                gl,
                &mut builder,
                scope,
                arenas.get(step.lvalue),
                step_var,
                arenas,
            );
        }
        V::Repeat(_) => {
            let (i, _) = repeat_vars.unwrap();
            let phi_ref = repeat_i_phi.unwrap();

            let one = builder.constant(gl, Value::Decimal(1));
            let i_plus_1 = builder.plus(gl, i, one);
            builder.update_phi_ref(gl, phi_ref, 1, builder.key(), i_plus_1);
        }
        V::Forever | V::While(_) => {}
    }

    let bb_key = builder.key();
    for (symbol_key, phi_ref) in phi_refs {
        let SymbolVariant::Variable(Some(var)) = &scope.symbols[symbol_key].variant else {
            todo!();
        };
        builder.update_phi_ref(gl, phi_ref, 1, bb_key, *var);
    }

    let next_builder = builder.next_builder(gl);
    if let Some(branch_ref) = branch_ref {
        branch_ref.update(gl, next_builder.key());
    }
    builder.jump_to(gl, loop_start);
    builder = next_builder;
    builder
}
