use std::collections::HashMap;

use vogls_ir::{
    BasicBlockBuilder, BasicBlockKey, BasicBlockTerminator, GlobalContext, VariableKey,
};

use crate::ast::AstId;
use crate::ast::statement::{
    CaseItemPattern, CaseStatement, CaseStatementVariant, ConditionalStatement, StatementOrNull,
};
use crate::lower::diagnostics::Diagnostics;
use crate::lower::scope::{Scope, SymbolKey};
use crate::lower::{VTypeTable, lower_expr, statements_to_process};
use crate::parser::AstArenas;

struct State {
    origins: Vec<BasicBlockKey>,
    symbols: Vec<SymbolKey>,
    symbol_lookup: HashMap<SymbolKey, usize>,
    assigned: HashMap<(SymbolKey, BasicBlockKey), VariableKey>,
}

impl State {
    fn new() -> Self {
        State {
            origins: Vec::new(),
            symbols: Vec::new(),
            symbol_lookup: HashMap::new(),
            assigned: HashMap::new(),
        }
    }

    fn insert(&mut self, bb: BasicBlockKey, i: impl Iterator<Item = (SymbolKey, VariableKey)>) {
        self.origins.push(bb);
        for (symbol, variable) in i {
            self.symbol_lookup.entry(symbol).or_insert_with(|| {
                let idx = self.symbols.len();
                self.symbols.push(symbol);
                idx
            });
            self.assigned.insert((symbol, bb), variable);
        }
    }
}

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

    let condition = lower_expr(
        gl,
        arenas,
        types,
        scope,
        diagnostics,
        &mut builder,
        if_branch.condition,
    )?;

    let mut state = State::new();

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
    state.insert(if_true_builder.key(), scope.scope_assigned_symbols());
    scope.pop_scope();

    let mut builder = if_true_builder.next_terminate_later(gl);
    for else_if_branch in else_ifs.iter() {
        branch_ref.update(gl, builder.key());

        let else_if_branch = arenas.get(else_if_branch);
        let condition = lower_expr(
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
        state.insert(if_true_builder.key(), scope.scope_assigned_symbols());
        scope.pop_scope();

        builder = if_true_builder.next_terminate_later(gl);
    }

    let mut branch_ref = Some(branch_ref);
    if let Some(statement) = else_branch {
        branch_ref.take().unwrap().update(gl, builder.key());

        scope.push_scope();
        builder =
            lower_statement_or_null(gl, arenas, types, scope, diagnostics, builder, *statement)?;
        state.insert(builder.key(), scope.scope_assigned_symbols());
        scope.pop_scope();

        builder = builder.jump(gl);
    }

    for bb in &state.origins {
        gl.bbs[*bb].terminator = BasicBlockTerminator::Jump(builder.key());
    }
    if let Some(branch_ref) = branch_ref {
        state.origins.push(branch_ref.origin_key());
        branch_ref.update(gl, builder.key());
    }

    for symbol in state.symbols {
        let unassigned_var = scope.scope_variables[symbol]
            .last()
            .copied()
            .map(|(_, v)| v);
        let srcs = state
            .origins
            .iter()
            .map(|bb| {
                let var = match state.assigned.get(&(symbol, *bb)) {
                    None => unassigned_var.unwrap(), // @TODO: better error message.
                    Some(v) => *v,
                };
                (*bb, var)
            })
            .collect();
        let (v, _) = builder.phi(gl, srcs);
        scope.assign(symbol, v);
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

    let expr_var = lower_expr(
        gl,
        arenas,
        types,
        scope,
        diagnostics,
        &mut builder,
        *expr,
    )?;

    let mut state = State::new();
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
                let v = lower_expr(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    &mut builder,
                    fst,
                )?;
                let mut acc = builder.equals(gl, expr_var, v);
                for e in exprs.iter().skip(1) {
                    let v = lower_expr(
                        gl,
                        arenas,
                        types,
                        scope,
                        diagnostics,
                        &mut builder,
                        e,
                    )?;
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
        state.insert(if_true_builder.key(), scope.scope_assigned_symbols());
        scope.pop_scope();

        builder = if_true_builder.next_terminate_later(gl);
        branch_ref.update(gl, builder.key());
    }

    if let Some(statement) = default {
        scope.push_scope();
        builder =
            lower_statement_or_null(gl, arenas, types, scope, diagnostics, builder, statement)?;
        state.insert(builder.key(), scope.scope_assigned_symbols());
        scope.pop_scope();
        builder = builder.jump(gl);
    } else {
        state.origins.push(builder.key());
        builder = builder.jump(gl);
    }

    for bb in &state.origins {
        gl.bbs[*bb].terminator = BasicBlockTerminator::Jump(builder.key());
    }

    for symbol in state.symbols {
        let unassigned_var = scope.scope_variables[symbol]
            .last()
            .copied()
            .map(|(_, v)| v);
        let srcs = state
            .origins
            .iter()
            .map(|bb| {
                let var = match state.assigned.get(&(symbol, *bb)) {
                    None => unassigned_var.unwrap(), // @TODO: better error message.
                    Some(v) => *v,
                };
                (*bb, var)
            })
            .collect();
        let (v, _) = builder.phi(gl, srcs);
        scope.assign(symbol, v);
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
