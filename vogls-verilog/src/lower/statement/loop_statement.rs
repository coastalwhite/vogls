use vogls_ir::{BasicBlockBuilder, GlobalContext};

use crate::ast::statement::{LoopStatement, LoopStatementVariant};
use crate::ast::{AstId, AstIdRange};
use crate::lower::diagnostics::Diagnostics;
use crate::lower::scope::{Scope, SymbolVariant};
use crate::lower::{Region, assign, get_intersect_symbols_generated, lower_expr};
use crate::parser::AstArenas;

pub fn lower_loop_statement<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    ls: AstId<LoopStatement>,
) -> Result<BasicBlockBuilder, ()> {
    use LoopStatementVariant as V;

    let ls = arenas.get(ls);

    let predecessor = builder.key();
    let mut repeat_vars = None;
    match ls.variant {
        V::Repeat(size) => {
            let (size, _) = lower_expr(gl, arenas, scope, diagnostics, &mut builder, size)?;
            let i = builder.constant_u32(gl, 0);
            repeat_vars = Some((i, size));
        }
        V::For(initialization, _, _) => {
            let initialization = arenas.get(initialization);
            let (initialization_var, initialization_var_ty) = lower_expr(
                gl,
                arenas,
                scope,
                diagnostics,
                &mut builder,
                initialization.expr,
            )?;
            assign::assign_variable_lvalue(
                gl,
                arenas,
                scope,
                diagnostics,
                &mut builder,
                initialization.lvalue,
                initialization_var,
                initialization_var_ty,
                Region::Active,
            )?;
        }
        V::Forever | V::While(_) => {}
    }

    builder = builder.jump(gl);

    let mut intersect_vars_generated =
        get_intersect_symbols_generated(gl, &*scope, AstIdRange::single(ls.statement), arenas);
    if let V::For(_, _, step) = ls.variant {
        let step = arenas.get(step);
        let step_lvalue = arenas.get(step.lvalue);
        if step_lvalue.0.len() != 1 {
            diagnostics.not_yet_implemented(arenas.get_span(step.lvalue), "step concat lvalue");
            return Err(());
        }
        let step_lvalue = arenas.get(step_lvalue.0.get(0));
        if !step_lvalue.exprs.is_empty() || step_lvalue.range_expression.is_some() {
            diagnostics.not_yet_implemented(
                arenas.get_span(step.lvalue),
                "step with exprs or range expressions",
            );
            return Err(());
        }

        let step_symbol_key = scope
            .get(arenas.get_ident(step_lvalue.ident.item.0))
            .unwrap();
        intersect_vars_generated.push(step_symbol_key);
    }

    for symkey in &intersect_vars_generated {
        match &mut scope.symbols[*symkey].variant {
            SymbolVariant::Constant(_) => todo!(),
            SymbolVariant::Genvar(_) => todo!(),
            SymbolVariant::Task(_) => todo!(),
            SymbolVariant::Signal(_dims, _ty, _) => {}
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
        V::While(condition) => {
            Some(lower_expr(gl, arenas, scope, diagnostics, &mut builder, condition)?.0)
        }
        V::For(_, condition, _) => {
            Some(lower_expr(gl, arenas, scope, diagnostics, &mut builder, condition)?.0)
        }
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
        builder = super::statements_to_process(
            gl,
            arenas,
            scope,
            diagnostics,
            builder,
            AstIdRange::single(ls.statement),
        )?;
        scope.pop_scope();
    }

    match ls.variant {
        V::For(_, _, step) => {
            let step = arenas.get(step);
            let (step_var, step_var_ty) =
                lower_expr(gl, arenas, scope, diagnostics, &mut builder, step.expr)?;
            assign::assign_variable_lvalue(
                gl,
                arenas,
                scope,
                diagnostics,
                &mut builder,
                step.lvalue,
                step_var,
                step_var_ty,
                Region::Active,
            )?;
        }
        V::Repeat(_) => {
            let (i, _) = repeat_vars.unwrap();
            let phi_ref = repeat_i_phi.unwrap();

            let one = builder.constant_u32(gl, 1);
            let i_plus_1 = builder.plus(gl, i, one);
            builder.update_phi_ref(gl, phi_ref, 1, builder.key(), i_plus_1);
        }
        V::Forever | V::While(_) => {}
    }

    let next_builder = builder.next_builder(gl);
    if let Some(branch_ref) = branch_ref {
        branch_ref.update(gl, next_builder.key());
    }
    builder.jump_to(gl, loop_start);
    builder = next_builder;
    Ok(builder)
}
