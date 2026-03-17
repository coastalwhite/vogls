use vogls_ir::{BasicBlockBuilder, BasicBlockTerminator, GlobalContext};

use crate::ast::statement::{LoopStatement, LoopStatementVariant};
use crate::ast::{AstId, AstIdRange};
use crate::lower::Scope;
use crate::lower::diagnostics::Diagnostics;
use crate::lower::{assign, lower_expr};
use crate::parser::AstArenas;

pub fn lower_loop_statement<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    ls: AstId<'a, LoopStatement<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    use LoopStatementVariant as V;

    let ls = &*ls;

    let mut repeat_vars = None;
    match ls.variant {
        V::Repeat(size) => {
            let (size, _) = lower_expr(gl, arenas, scope, diagnostics, &mut builder, size)?;
            let i = builder.constant_u32(gl, 0);
            repeat_vars = Some((i, size));
        }
        V::For(initialization, _, _) => {
            let initialization = &*initialization;
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
                false,
            )?;
        }
        V::Forever | V::While(_) => {}
    }

    let predecessor = builder.key();
    builder = builder.jump(gl);

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
            let condition = builder.reduce_or(gl, condition);
            (branch_ref, builder) = builder.branch(gl, condition);
            Some(branch_ref)
        }
    };

    builder = super::statements_to_process(
        gl,
        arenas,
        scope,
        diagnostics,
        builder,
        AstIdRange::single(ls.statement),
    )?;

    match ls.variant {
        V::For(_, _, step) => {
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
                false,
            )?;
        }
        V::Repeat(_) => {
            let (i, _) = repeat_vars.unwrap();

            let one = builder.constant_u32(gl, 1);
            let i_plus_1 = builder.plus(gl, i, one);

            let phi_ref = repeat_i_phi.unwrap();
            builder.update_phi_ref(gl, phi_ref, 1, builder.key(), i_plus_1);
        }
        V::Forever | V::While(_) => {}
    }

    let builder_key = builder.key();
    let mut builder = builder.next_terminate_later(gl);
    if let Some(branch_ref) = branch_ref {
        builder.update_branch_ref(gl, branch_ref, builder.key());
    }
    gl.bbs[builder_key].terminator = BasicBlockTerminator::Jump(loop_start);

    Ok(builder)
}
