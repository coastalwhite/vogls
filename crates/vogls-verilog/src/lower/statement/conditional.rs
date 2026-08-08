use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, BasicBlockTerminator};

use crate::ast::AstId;
use crate::ast::statement::{
    CaseItemPattern, CaseStatement, CaseStatementVariant, ConditionalStatement,
};
use crate::lower::expression::coerce_bin_arithmetic;
use crate::lower::lower_expr;
use crate::lower::{LowerContext, MutLowerContext};

use super::lower_stmts;

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
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
    if_true_builder = lower_stmts(
        ctx,
        mctx,
        scope,
        if_true_builder,
        if_branch.statement.as_id_range(),
    )?;
    origins.push(if_true_builder.key());

    let mut builder = if_true_builder.next_terminate_later(mctx.gl());
    for else_if_branch in else_ifs.iter() {
        builder.update_branch_falsy(mctx.gl(), branch_ref, builder.key());

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
        if_true_builder = lower_stmts(
            ctx,
            mctx,
            scope,
            if_true_builder,
            else_if_branch.statement.as_id_range(),
        )?;
        origins.push(if_true_builder.key());

        builder = if_true_builder.next_terminate_later(mctx.gl());
    }

    let mut branch_ref = Some(branch_ref);
    if let Some(statement) = else_branch {
        builder.update_branch_falsy(mctx.gl(), branch_ref.take().unwrap(), builder.key());

        builder = lower_stmts(ctx, mctx, scope, builder, statement.as_id_range())?;
        origins.push(builder.key());

        builder = builder.jump(mctx.gl());
    }

    for bb in &origins {
        mctx.gl.bbs[*bb].terminator = BasicBlockTerminator::Jump(builder.key());
    }
    if let Some(branch_ref) = branch_ref {
        builder.update_branch_falsy(mctx.gl(), branch_ref, builder.key());
    }

    Ok(builder)
}

pub fn lower_case_statement<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    mut builder: BasicBlockBuilder,
    case_statement: AstId<'a, CaseStatement<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let CaseStatement {
        variant,
        expr,
        items,
    } = &*case_statement;

    let (expr_var, expr_var_ty) = lower_expr(ctx, mctx, scope, &mut builder, *expr, None)?;

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
                let (v, v_ty) = lower_expr(
                    ctx,
                    mctx,
                    scope,
                    &mut builder,
                    fst,
                    Some(expr_var_ty.bit_length()),
                )?;
                let (expr_var, _, v, _) =
                    coerce_bin_arithmetic(mctx.gl(), &mut builder, expr_var, expr_var_ty, v, v_ty);
                let expr_var_adj = match variant {
                    CaseStatementVariant::Case => expr_var,
                    CaseStatementVariant::CaseX => {
                        // @Performance: This should probably be one instruction
                        let x = builder.copy_x(mctx.gl(), expr_var, v);
                        builder.copy_z(mctx.gl(), x, v)
                    }
                    CaseStatementVariant::CaseZ => builder.copy_z(mctx.gl(), expr_var, v),
                };
                let mut acc = builder.case_equals(mctx.gl(), expr_var_adj, v);
                for e in exprs.iter().skip(1) {
                    let (v, _) = lower_expr(
                        ctx,
                        mctx,
                        scope,
                        &mut builder,
                        e,
                        Some(expr_var_ty.bit_length()),
                    )?;
                    let expr_var_adj = match variant {
                        CaseStatementVariant::Case => expr_var,
                        CaseStatementVariant::CaseX => {
                            // @Performance: This should probably be one instruction
                            let x = builder.copy_x(mctx.gl(), expr_var, v);
                            builder.copy_z(mctx.gl(), x, v)
                        }
                        CaseStatementVariant::CaseZ => builder.copy_z(mctx.gl(), expr_var, v),
                    };
                    let v = builder.case_equals(mctx.gl(), expr_var_adj, v);
                    acc = builder.or(mctx.gl(), acc, v);
                }
                acc
            }
        };

        let condition = builder.reduce_or(mctx.gl(), condition);
        let (branch_ref, mut if_true_builder) = builder.branch(mctx.gl(), condition);
        if_true_builder = lower_stmts(
            ctx,
            mctx,
            scope,
            if_true_builder,
            case_item.statement_or_null.as_id_range(),
        )?;
        origins.push(if_true_builder.key());

        builder = if_true_builder.next_terminate_later(mctx.gl());
        builder.update_branch_falsy(mctx.gl(), branch_ref, builder.key());
    }

    if let Some(statement) = default {
        builder = lower_stmts(ctx, mctx, scope, builder, statement.as_id_range())?;
        builder = builder.jump(mctx.gl());
    } else {
        builder = builder.jump(mctx.gl());
    }

    for bb in &origins {
        mctx.gl.bbs[*bb].terminator = BasicBlockTerminator::Jump(builder.key());
    }

    Ok(builder)
}
