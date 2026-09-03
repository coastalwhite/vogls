use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, BasicBlockKey, BasicBlockTerminator, ProcessBuilder, SCALAR_VSIZE,
    Time,
};

use crate::ast::AstId;
use crate::ast::statement::{
    CaseItemPattern, CaseStatement, CaseStatementVariant, ConditionalStatement,
};
use crate::lower::expression::{coerce_bin_arithmetic, get_expr_type};
use crate::lower::lower_expr;
use crate::lower::{LowerContext, MutLowerContext};

use super::lower_stmts;

fn join_origins(
    mctx: &mut MutLowerContext,
    origins: &[BasicBlockKey],
    proc_builder: &mut ProcessBuilder,
    builder: &mut BasicBlockBuilder,
) {
    let join = builder.key();
    let join_tr = builder.tr();

    if origins.iter().any(|&bb| mctx.gl.bbs[bb].region != join_tr) {
        let join_tr = builder.mark_as_tr_root(mctx.gl(), join);
        proc_builder.push_temporal_region(join_tr);
    }

    let join_tr = mctx.gl.bbs[join].region;
    for &bb in origins {
        if mctx.gl.bbs[bb].region == join_tr {
            mctx.gl.bbs[bb].terminator = BasicBlockTerminator::Jump(join);
        } else {
            mctx.gl.bbs[bb].terminator = BasicBlockTerminator::Wait(join_tr, Time(0));
        }
    }
}

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    proc_builder: &mut ProcessBuilder,
    mut builder: BasicBlockBuilder,
    conditional: AstId<'a, ConditionalStatement<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let ConditionalStatement {
        if_branch,
        else_ifs,
        else_branch,
    } = &*conditional;

    let (condition, _) = lower_expr(ctx, mctx, scope, &mut builder, if_branch.condition, None)?;
    let condition = builder.reduce_or(mctx.gl(), condition);

    let mut origins = Vec::new();

    let (mut branch_ref, mut if_true_builder) = builder.branch(mctx.gl(), condition);
    let mut falsy_bb = if_true_builder.next_bb_non_temporal(mctx.gl());
    if_true_builder.update_branch_falsy(mctx.gl(), branch_ref, falsy_bb);

    if_true_builder = lower_stmts(
        ctx,
        mctx,
        scope,
        proc_builder,
        if_true_builder,
        if_branch.statement.as_id_range(),
    )?;
    origins.push(if_true_builder.key());

    let mut builder = if_true_builder;
    builder.switch_to(mctx.gl(), falsy_bb);

    for else_if_branch in else_ifs.iter() {
        let else_if_branch = &*else_if_branch;
        let (condition, _) = lower_expr(
            ctx,
            mctx,
            scope,
            &mut builder,
            else_if_branch.condition,
            None,
        )?;
        let condition = builder.reduce_or(mctx.gl(), condition);

        (branch_ref, if_true_builder) = builder.branch(mctx.gl(), condition);
        falsy_bb = if_true_builder.next_bb_non_temporal(mctx.gl());
        if_true_builder.update_branch_falsy(mctx.gl(), branch_ref, falsy_bb);

        if_true_builder = lower_stmts(
            ctx,
            mctx,
            scope,
            proc_builder,
            if_true_builder,
            else_if_branch.statement.as_id_range(),
        )?;
        origins.push(if_true_builder.key());

        builder = if_true_builder;
        builder.switch_to(mctx.gl(), falsy_bb);
    }

    if let Some(statement) = else_branch {
        builder = lower_stmts(
            ctx,
            mctx,
            scope,
            proc_builder,
            builder,
            statement.as_id_range(),
        )?;
    }
    origins.push(builder.key());

    builder = builder.next_terminate_later(mctx.gl());

    join_origins(mctx, &origins, proc_builder, &mut builder);

    Ok(builder)
}

pub fn lower_case_statement<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    proc_builder: &mut ProcessBuilder,
    mut builder: BasicBlockBuilder,
    case_statement: AstId<'a, CaseStatement<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let CaseStatement {
        variant,
        expr,
        items,
    } = &*case_statement;

    let mut context_width = SCALAR_VSIZE;
    // let mut context_is_signed = true;

    let expr_ty = get_expr_type(
        &mctx.gl,
        ctx.arenas,
        &ctx.table,
        scope,
        &mut mctx.diagnostics,
        *expr,
    )?;
    context_width = context_width.max(expr_ty.bit_length());
    // context_is_signed &= expr_ty.is_signed();
    for item in items.iter() {
        let CaseItemPattern::Expressions(iexprs) = item.pattern.item else {
            continue;
        };
        for iexpr in iexprs.iter() {
            let iexpr_ty = get_expr_type(
                &mctx.gl,
                ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                iexpr,
            )?;
            context_width = context_width.max(iexpr_ty.bit_length());
            // context_is_signed &= iexpr_ty.is_signed();
        }
    }

    let (expr_var, expr_var_ty) =
        lower_expr(ctx, mctx, scope, &mut builder, *expr, Some(context_width))?;

    let mut origins = Vec::new();
    let mut default = None;

    for case_item in items.iter() {
        let condition = match case_item.pattern.item {
            CaseItemPattern::Default => {
                default = Some(case_item.statement_or_null);
                continue;
            }
            CaseItemPattern::Expressions(exprs) => {
                let fst = exprs.first().expect("spec: 1+ pattern expr in case_item");
                let (v, v_ty) =
                    lower_expr(ctx, mctx, scope, &mut builder, fst, Some(context_width))?;
                let (expr_var, _, v, _) =
                    coerce_bin_arithmetic(mctx.gl(), &mut builder, expr_var, expr_var_ty, v, v_ty);
                let (expr_var, v) = match variant {
                    CaseStatementVariant::Case => (expr_var, v),
                    CaseStatementVariant::CaseX => {
                        let v = builder.copy_x(mctx.gl(), v, expr_var);
                        let v = builder.copy_z(mctx.gl(), v, expr_var);
                        let expr_var = builder.copy_x(mctx.gl(), expr_var, v);
                        let expr_var = builder.copy_z(mctx.gl(), expr_var, v);
                        (expr_var, v)
                    }
                    CaseStatementVariant::CaseZ => (
                        builder.copy_z(mctx.gl(), expr_var, v),
                        builder.copy_z(mctx.gl(), v, expr_var),
                    ),
                };
                let mut acc = builder.case_equals(mctx.gl(), expr_var, v);
                for e in exprs.iter().skip(1) {
                    let (v, _) =
                        lower_expr(ctx, mctx, scope, &mut builder, e, Some(context_width))?;
                    let (expr_var, v) = match variant {
                        CaseStatementVariant::Case => (expr_var, v),
                        CaseStatementVariant::CaseX => {
                            let v = builder.copy_x(mctx.gl(), v, expr_var);
                            let v = builder.copy_z(mctx.gl(), v, expr_var);
                            let expr_var = builder.copy_x(mctx.gl(), expr_var, v);
                            let expr_var = builder.copy_z(mctx.gl(), expr_var, v);
                            (expr_var, v)
                        }
                        CaseStatementVariant::CaseZ => (
                            builder.copy_z(mctx.gl(), expr_var, v),
                            builder.copy_z(mctx.gl(), v, expr_var),
                        ),
                    };
                    let v = builder.case_equals(mctx.gl(), expr_var, v);
                    acc = builder.or(mctx.gl(), acc, v);
                }
                acc
            }
        };

        let condition = builder.reduce_or(mctx.gl(), condition);
        let (branch_ref, mut if_true_builder) = builder.branch(mctx.gl(), condition);
        let falsy_bb = if_true_builder.next_bb_non_temporal(mctx.gl());
        if_true_builder.update_branch_falsy(mctx.gl(), branch_ref, falsy_bb);

        if_true_builder = lower_stmts(
            ctx,
            mctx,
            scope,
            proc_builder,
            if_true_builder,
            case_item.statement_or_null.as_id_range(),
        )?;
        origins.push(if_true_builder.key());

        builder = if_true_builder;
        builder.switch_to(mctx.gl(), falsy_bb);
    }

    if let Some(statement) = default {
        builder = lower_stmts(
            ctx,
            mctx,
            scope,
            proc_builder,
            builder,
            statement.as_id_range(),
        )?;
    }
    origins.push(builder.key());

    builder = builder.next_terminate_later(mctx.gl());

    join_origins(mctx, &origins, proc_builder, &mut builder);

    Ok(builder)
}
