use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlock, BasicBlockBuilder, BasicBlockTerminator, Bits, INTEGER_VSIZE, LogicMode,
    ProcessBuilder, Signal, SignalFlags,
};

use crate::ast::statement::{LoopStatement, LoopStatementVariant};
use crate::ast::{AstId, AstIdRange};
use crate::lower::{LowerContext, MutLowerContext};
use crate::lower::{assign, lower_expr};

pub fn lower_loop_statement<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    proc_builder: &mut ProcessBuilder,
    mut builder: BasicBlockBuilder,
    ls: AstId<'a, LoopStatement<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    use LoopStatementVariant as V;

    let ls = &*ls;

    let mut repeat = None;
    match ls.variant {
        V::Repeat(size) => {
            let (size, _) = lower_expr(ctx, mctx, scope, &mut builder, size, None)?;
            let signal = mctx.gl.signals.insert(Signal {
                name: String::from("__REPEAT_VAR"),
                size: INTEGER_VSIZE,
                initialize: Some(Bits::new_u32(0)),
                mode: LogicMode::TwoValue,
                flags: SignalFlags::EMPTY,
                origin: TokenRange::default(),
            });
            repeat = Some((signal, size));
        }
        V::For(initialization, _, _) => {
            let initialization = &*initialization;
            let context_width =
                assign::variable_lvalue_size(ctx, mctx, scope, initialization.lvalue)?;
            let (initialization_var, initialization_var_ty) = lower_expr(
                ctx,
                mctx,
                scope,
                &mut builder,
                initialization.expr,
                Some(context_width),
            )?;
            assign::assign_variable_lvalue(
                ctx,
                mctx,
                scope,
                &mut builder,
                initialization.lvalue,
                initialization_var,
                initialization_var_ty,
                false,
            )?;
        }
        V::Forever | V::While(_) => {}
    }

    let preheader = builder.key();
    builder = builder.jump(mctx.gl());

    let loop_start = builder.key();
    let loop_start_tr = builder.tr();
    let condition = match ls.variant {
        V::Forever => None,
        V::Repeat(_) => {
            let (signal, size) = repeat.unwrap();
            let v = builder.probe(mctx.gl(), signal);
            Some(builder.unsigned_lt(mctx.gl(), v, size))
        }
        V::While(condition) => Some(lower_expr(ctx, mctx, scope, &mut builder, condition, None)?.0),
        V::For(_, condition, _) => {
            Some(lower_expr(ctx, mctx, scope, &mut builder, condition, None)?.0)
        }
    };

    let exit_bb = match condition {
        None => None,
        Some(condition) => {
            let exit_builder;
            let condition = builder.reduce_or(mctx.gl(), condition);
            (builder, exit_builder) = builder.double_branch(mctx.gl(), condition);
            Some(exit_builder.key())
        }
    };

    builder = super::lower_stmts(
        ctx,
        mctx,
        scope,
        proc_builder,
        builder,
        AstIdRange::single(ls.statement),
    )?;

    match ls.variant {
        V::For(_, _, step) => {
            let context_width = assign::variable_lvalue_size(ctx, mctx, scope, step.lvalue)?;
            let (step_var, step_var_ty) = lower_expr(
                ctx,
                mctx,
                scope,
                &mut builder,
                step.expr,
                Some(context_width),
            )?;
            assign::assign_variable_lvalue(
                ctx,
                mctx,
                scope,
                &mut builder,
                step.lvalue,
                step_var,
                step_var_ty,
                false,
            )?;
        }
        V::Repeat(_) => {
            let (signal, _) = repeat.unwrap();
            let v = builder.probe(mctx.gl(), signal);
            let v = builder.plus_constant(mctx.gl(), v, Bits::new_u32(1));
            builder.drive(mctx.gl(), signal, v);
        }
        V::Forever | V::While(_) => {}
    }

    if builder.tr() != loop_start_tr {
        let loop_start_tr = builder.mark_as_tr_root(mctx.gl(), loop_start);
        proc_builder.push_temporal_region(loop_start_tr);
        mctx.gl.bbs[preheader].terminator =
            BasicBlockTerminator::Wait(loop_start_tr, vogls_ir::Time(0));
    }
    builder.jump_to_any(mctx.gl(), loop_start);

    let exit_bb = match exit_bb {
        Some(exit_bb) => exit_bb,
        None => {
            let region = mctx.gl.bbs[loop_start].region;
            mctx.gl.bbs.insert(BasicBlock {
                instrs: Vec::new(),
                region,
                terminator: BasicBlockTerminator::Halt,
            })
        }
    };

    builder.finished_switch_to(mctx.gl(), exit_bb);

    Ok(builder)
}
