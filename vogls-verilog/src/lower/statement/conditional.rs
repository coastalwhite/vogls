use std::collections::HashMap;

use vogls_ir::{
    BasicBlockBuilder, BasicBlockKey, BasicBlockTerminator, GlobalContext, VariableKey,
};

use crate::ast::AstId;
use crate::ast::statement::{ConditionalStatement, StatementOrNull};
use crate::lower::scope::{Scope, SymbolKey};
use crate::lower::{lower_expr, statements_to_process};
use crate::parser::AstArenas;

pub fn lower<'a>(
    mut builder: BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    conditional: AstId<ConditionalStatement>,
    arenas: &'a AstArenas,
) -> BasicBlockBuilder {
    let ConditionalStatement {
        if_branch,
        else_ifs,
        else_branch,
    } = arenas.get(conditional);

    let condition = lower_expr(
        &mut builder,
        gl,
        scope,
        arenas.get(if_branch.condition),
        arenas,
    );

    struct State {
        origins: Vec<BasicBlockKey>,
        symbols: Vec<SymbolKey>,
        symbol_lookup: HashMap<SymbolKey, usize>,
        assigned: HashMap<(SymbolKey, BasicBlockKey), VariableKey>,
    }

    impl State {
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

    let mut state = State {
        origins: Vec::new(),
        symbols: Vec::new(),
        symbol_lookup: HashMap::new(),
        assigned: HashMap::new(),
    };

    let (mut branch_ref, mut if_true_builder) = builder.branch(gl, condition);
    scope.push_scope();
    if_true_builder =
        lower_statement_or_null(if_true_builder, gl, scope, if_branch.statement, arenas);
    state.insert(if_true_builder.key(), scope.scope_assigned_symbols());
    scope.pop_scope();

    let mut builder = if_true_builder.next_terminate_later(gl);
    for else_if_branch in else_ifs.iter() {
        branch_ref.update(gl, builder.key());

        let else_if_branch = arenas.get(else_if_branch);
        let condition = lower_expr(
            &mut builder,
            gl,
            scope,
            arenas.get(else_if_branch.condition),
            arenas,
        );

        (branch_ref, if_true_builder) = builder.branch(gl, condition);
        scope.push_scope();
        if_true_builder =
            lower_statement_or_null(if_true_builder, gl, scope, else_if_branch.statement, arenas);
        state.insert(if_true_builder.key(), scope.scope_assigned_symbols());
        scope.pop_scope();

        builder = if_true_builder.next_terminate_later(gl);
    }

    let mut branch_ref = Some(branch_ref);
    if let Some(statement) = else_branch {
        branch_ref.take().unwrap().update(gl, builder.key());

        scope.push_scope();
        builder = lower_statement_or_null(builder, gl, scope, *statement, arenas);
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

    return builder;

    // match else_branch {
    //     None => {
    //         builder = if_true_builder.jump(gl);
    //         branch_ref.update(gl, builder.key());
    //
    //         for (s, v) in assigned {
    //             let before = scope.scope_variables[s].last().unwrap().1;
    //             let (v, _) = builder.phi(gl, [(if_true_bb, v), (start_bb, before)].into());
    //             scope.assign(s, v);
    //         }
    //     }
    //     Some(statement) => {
    //         let else_builder = if_true_builder.next_builder(gl);
    //         branch_ref.update(gl, else_builder.key());
    //
    //         scope.push_scope();
    //         let else_builder = lower_statement_or_null(else_builder, gl, scope, *statement, arenas);
    //         let else_assigned = scope.scope_assigned_symbols().collect::<Vec<_>>();
    //         scope.pop_scope();
    //
    //         let else_bb = else_builder.key();
    //
    //         builder = else_builder.jump(gl);
    //         if_true_builder.jump_to(gl, builder.key());
    //
    //         let mut unassigned_symbols = HashSet::new();
    //         let mut phis = HashMap::<SymbolKey, (VariableKey, VariableKey)>::new();
    //         phis.extend(assigned.into_iter().map(|(s, v)| {
    //             let else_v = match &scope.scope_variables[s].last() {
    //                 None => {
    //                     unassigned_symbols.insert(s);
    //                     v
    //                 }
    //                 Some((_, v)) => *v,
    //             };
    //             (s, (v, else_v))
    //         }));
    //         for (s, v) in else_assigned {
    //             unassigned_symbols.remove(&s);
    //             // @TODO: This unwrap_or needs to be checked somehow.
    //             let start_v = scope.scope_variables[s].last().copied().unwrap().1;
    //             phis.entry(s)
    //                 .and_modify(|(_, else_v)| *else_v = v)
    //                 .or_insert((start_v, v));
    //         }
    //         // @TODO: better error handling.
    //         assert!(unassigned_symbols.is_empty());
    //
    //         for (s, (if_true, if_false)) in phis {
    //             let (v, _) = builder.phi(gl, [(if_true_bb, if_true), (else_bb, if_false)].into());
    //             scope.assign(s, v);
    //         }
    //     }
    // }
    //
    // builder
}

pub fn lower_statement_or_null<'a>(
    builder: BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    statement: AstId<StatementOrNull>,
    arenas: &'a AstArenas,
) -> BasicBlockBuilder {
    match arenas.get(statement) {
        StatementOrNull::Attribute(_) => builder,
        StatementOrNull::Statement(statement) => statements_to_process(
            builder,
            gl,
            scope,
            std::slice::from_ref(arenas.get(*statement)),
            arenas,
        ),
    }
}
