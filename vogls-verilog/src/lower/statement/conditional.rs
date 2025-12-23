use vogls_ir::{BasicBlockBuilder, BasicBlockTerminator, GlobalContext};

use crate::ast::AstId;
use crate::ast::statement::{
    CaseItemPattern, CaseStatement, CaseStatementVariant, ConditionalStatement, StatementOrNull,
};
use crate::lower::diagnostics::Diagnostics;
use crate::lower::scope::Scope;
use crate::lower::{VTypeTable, lower_expr, statements_to_process};
use crate::parser::AstArenas;

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    conditional: AstId<ConditionalStatement>,
) -> Result<BasicBlockBuilder, ()> {
    let ConditionalStatement {
        if_branch,
        else_ifs,
        else_branch,
    } = arenas.get(conditional);

    let (condition, _) = lower_expr(
        gl,
        arenas,
        types,
        scope,
        diagnostics,
        &mut builder,
        if_branch.condition,
    )?;

    let mut origins = Vec::new();

    let (mut branch_ref, mut if_true_builder) = builder.branch(gl, condition);
    scope.push_scope();
    if_true_builder = lower_statement_or_null(
        gl,
        arenas,
        types,
        scope,
        diagnostics,
        if_true_builder,
        if_branch.statement,
    )?;
    origins.push(if_true_builder.key());
    scope.pop_scope();

    let mut builder = if_true_builder.next_terminate_later(gl);
    for else_if_branch in else_ifs.iter() {
        branch_ref.update(gl, builder.key());

        let else_if_branch = arenas.get(else_if_branch);
        let (condition, _) = lower_expr(
            gl,
            arenas,
            types,
            scope,
            diagnostics,
            &mut builder,
            else_if_branch.condition,
        )?;

        (branch_ref, if_true_builder) = builder.branch(gl, condition);
        scope.push_scope();
        if_true_builder = lower_statement_or_null(
            gl,
            arenas,
            types,
            scope,
            diagnostics,
            if_true_builder,
            else_if_branch.statement,
        )?;
        origins.push(if_true_builder.key());
        scope.pop_scope();

        builder = if_true_builder.next_terminate_later(gl);
    }

    let mut branch_ref = Some(branch_ref);
    if let Some(statement) = else_branch {
        branch_ref.take().unwrap().update(gl, builder.key());

        scope.push_scope();
        builder =
            lower_statement_or_null(gl, arenas, types, scope, diagnostics, builder, *statement)?;
        origins.push(builder.key());
        scope.pop_scope();

        builder = builder.jump(gl);
    }

    for bb in &origins {
        gl.bbs[*bb].terminator = BasicBlockTerminator::Jump(builder.key());
    }
    if let Some(branch_ref) = branch_ref {
        branch_ref.update(gl, builder.key());
    }

    Ok(builder)
}

pub fn lower_case_statement<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    case_statement: AstId<CaseStatement>,
) -> Result<BasicBlockBuilder, ()> {
    let CaseStatement {
        variant,
        expr,
        items,
    } = arenas.get(case_statement);

    match variant {
        CaseStatementVariant::Case => {}
        CaseStatementVariant::CaseZ => todo!(),
        CaseStatementVariant::CaseX => todo!(),
    }

    let (expr_var, _) = lower_expr(gl, arenas, types, scope, diagnostics, &mut builder, *expr)?;

    let mut origins = Vec::new();
    let mut default = None;

    for item in items.iter() {
        let case_item = arenas.get(item);
        let condition = match case_item.pattern.item {
            CaseItemPattern::Default => {
                default = Some(case_item.statement_or_null);
                continue;
            }
            CaseItemPattern::Expressions(exprs) => {
                let fst = exprs.first().expect("spec: 1+ pattern expr in case_item");
                let (v, _) = lower_expr(gl, arenas, types, scope, diagnostics, &mut builder, fst)?;
                let mut acc = builder.equals(gl, expr_var, v);
                for e in exprs.iter().skip(1) {
                    let (v, _) =
                        lower_expr(gl, arenas, types, scope, diagnostics, &mut builder, e)?;
                    let v = builder.equals(gl, expr_var, v);
                    acc = builder.or(gl, acc, v);
                }
                acc
            }
        };

        let (branch_ref, mut if_true_builder) = builder.branch(gl, condition);
        scope.push_scope();
        if_true_builder = lower_statement_or_null(
            gl,
            arenas,
            types,
            scope,
            diagnostics,
            if_true_builder,
            case_item.statement_or_null,
        )?;
        origins.push(if_true_builder.key());
        scope.pop_scope();

        builder = if_true_builder.next_terminate_later(gl);
        branch_ref.update(gl, builder.key());
    }

    if let Some(statement) = default {
        scope.push_scope();
        builder =
            lower_statement_or_null(gl, arenas, types, scope, diagnostics, builder, statement)?;
        origins.push(builder.key());
        scope.pop_scope();
        builder = builder.jump(gl);
    } else {
        origins.push(builder.key());
        builder = builder.jump(gl);
    }

    for bb in &origins {
        gl.bbs[*bb].terminator = BasicBlockTerminator::Jump(builder.key());
    }

    Ok(builder)
}

pub fn lower_statement_or_null<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: BasicBlockBuilder,
    statement: AstId<StatementOrNull>,
) -> Result<BasicBlockBuilder, ()> {
    match arenas.get(statement) {
        StatementOrNull::Attribute(_) => Ok(builder),
        StatementOrNull::Statement(statement) => statements_to_process(
            gl,
            arenas,
            types,
            scope,
            diagnostics,
            builder,
            std::slice::from_ref(arenas.get(*statement)),
        ),
    }
}
