use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockTerminator, Bits, GlobalContext, PhiRef, SCALAR_VSIZE, SignalKey, TIME_VSIZE,
    VariableKey, new_anonymous_builder,
};
use vogls_utils::VgHashMap;

use crate::ast::constant_expr::ConstantMinTypMaxExpression;
use crate::ast::expr::Expr;
use crate::ast::specify::{
    EdgeIdentifier, PathDeclaration, PathDeclarationVariant, SpecifyBlockItem,
    StateDependentCondition,
};
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::VSymbol;
use crate::lower::expression::lower_expr;
use crate::lower::{
    eval_constant_expr, hident_span, try_resolve_net, try_resolve_symbol_id, unwrap_get_net_mut,
};
use crate::parser::AstArenas;

use super::{Diagnostics, Scope};

enum Condition {
    None,

    InputPosedge,
    InputNegedge,
    NoOtherCondition,

    Expr(AstId<Expr>),
    InputPosedgeExpr(AstId<Expr>),
    InputNegedgeExpr(AstId<Expr>),
}

pub struct SpecifyOutput {
    sid: SymbolId,
    inputs: VgHashMap<SignalKey, usize>,
    paths: Vec<(SignalKey, Vec<SpecifyPath>)>,
}

pub struct SpecifyPath {
    condition: Condition,
    // @TODO: Incorperate different variants
    delays: u64,
}

pub fn lower_specify<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,

    items: AstIdRange<SpecifyBlockItem>,
    outs_lut: &mut VgHashMap<SignalKey, usize>,
    outs: &mut Vec<(SignalKey, SpecifyOutput)>,

    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    for item in items.iter() {
        match arenas.get(item) {
            SpecifyBlockItem::SpecParamDeclaration => todo!(),
            SpecifyBlockItem::PulseStyleDeclaration => todo!(),
            SpecifyBlockItem::ShowCancelledDeclaration => todo!(),
            SpecifyBlockItem::PathDeclaration(path_declaration) => {
                let PathDeclaration {
                    state_dependent_condition,
                    edge_identifier,
                    input_terminal_descriptors,
                    polarity_operator,
                    variant,
                    data_source_expression,
                    output_terminal_descriptors,
                    path_delay_value,
                } = path_declaration;

                // Don't care for now.
                _ = polarity_operator;

                let condition = match (state_dependent_condition, edge_identifier) {
                    (None, None) => Condition::None,
                    (Some(c), None) => match arenas.get(*c) {
                        StateDependentCondition::If(id) => Condition::Expr(id.into_expr()),
                        StateDependentCondition::Ifnone => Condition::NoOtherCondition,
                    },
                    (None, Some(e)) => match e.item {
                        EdgeIdentifier::Posedge => Condition::InputPosedge,
                        EdgeIdentifier::Negedge => Condition::InputNegedge,
                    },
                    (Some(c), Some(e)) => match (arenas.get(*c), e.item) {
                        (StateDependentCondition::If(id), EdgeIdentifier::Posedge) => {
                            Condition::InputPosedgeExpr(id.into_expr())
                        }
                        (StateDependentCondition::If(id), EdgeIdentifier::Negedge) => {
                            Condition::InputNegedgeExpr(id.into_expr())
                        }

                        // @NOTE: Should actually not be reachable from normal Verilog.
                        (StateDependentCondition::Ifnone, _) => todo!(),
                    },
                };

                if matches!(variant, PathDeclarationVariant::Parallel) {
                    todo!()
                }

                // @TODO: Remove assertions
                assert_eq!(input_terminal_descriptors.len(), 1);
                assert_eq!(output_terminal_descriptors.len(), 1);

                let input = arenas.get(input_terminal_descriptors.get(0));
                let output = arenas.get(output_terminal_descriptors.get(0));

                let (None, None) = (input.constant_range_expr, output.constant_range_expr) else {
                    todo!()
                };

                let input_sid = try_resolve_symbol_id(
                    scope.key,
                    scope.table,
                    arenas,
                    input.ident,
                    diagnostics,
                )?;
                let VSymbol::Net(input_net) = &scope.table[input_sid].content else {
                    diagnostics.not_yet_implemented(
                        arenas.get_item_span(input.ident),
                        "cannot be used as net",
                    );
                    return Err(());
                };
                let input = input_net.signal;
                let output_sid = try_resolve_symbol_id(
                    scope.key,
                    scope.table,
                    arenas,
                    output.ident,
                    diagnostics,
                )?;
                let VSymbol::Net(output_net) = &scope.table[output_sid].content else {
                    diagnostics.not_yet_implemented(
                        arenas.get_item_span(output.ident),
                        "cannot be used as net",
                    );
                    return Err(());
                };
                let output = output_net.signal;

                // @TODO: Verify that these are actually module ports.

                if data_source_expression.is_some() {
                    todo!()
                }

                let delays = arenas.get(*path_delay_value);
                // @TODO: Remove assertion
                assert_eq!(delays.list_of_delay_expressions.len(), 1);

                // @Hack.
                let ConstantMinTypMaxExpression::Single(delays) =
                    arenas.get(delays.list_of_delay_expressions.get(0))
                else {
                    todo!()
                };

                // @Hack. Unwrap.
                let delays = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, *delays)?
                    .as_integer()
                    .unwrap()
                    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 222
                    // "If the path delay expression results in a negative value, it shall be treated as zero."
                    .max(0) as u64;

                let out = *outs_lut.entry(output).or_insert_with(|| {
                    let idx = outs.len();
                    outs.push((
                        output,
                        SpecifyOutput {
                            sid: output_sid,
                            inputs: Default::default(),
                            paths: Default::default(),
                        },
                    ));
                    idx
                });
                let out = &mut outs[out].1;
                let paths_idx = *out.inputs.entry(input).or_insert_with(|| {
                    let idx = out.paths.len();
                    out.paths.push((input, Vec::new()));
                    idx
                });
                out.paths[paths_idx]
                    .1
                    .push(SpecifyPath { condition, delays });
            }
            SpecifyBlockItem::SystemTimingCheck(_) => {
                // @TODO: Don't ignore these for now.
            }
        }
    }

    // @Performance: Scratchpad these.
    let mut input_before_lut = VgHashMap::<SignalKey, usize>::default();
    let mut input_before = Vec::<(SignalKey, VariableKey, Option<PhiRef>)>::new();
    // let mut condition_orsum = VgHashMap::<SignalKey, VariableKey>::default();

    outs_lut.clear();
    for (output, specify) in outs.drain(..) {
        input_before_lut.clear();
        input_before.clear();

        let mut proxy = gl.signals.get(output).unwrap().clone();
        proxy.name = format!("{}::SPECIFY_PROXY", proxy.name);
        let proxy = gl.signals.insert(proxy);

        unwrap_get_net_mut(scope.table, specify.sid).specify_proxy = Some(proxy);

        let mut builder =
            new_anonymous_builder(gl, "specify_proxy".to_string(), TokenRange::default());
        let entry = builder.key();

        for (input, paths) in &specify.paths {
            for path in paths {
                if matches!(
                    path.condition,
                    Condition::InputPosedge
                        | Condition::InputNegedge
                        | Condition::InputPosedgeExpr(_)
                        | Condition::InputNegedgeExpr(_)
                ) {
                    let before = builder.probe(gl, *input);
                    let idx = input_before.len();
                    input_before_lut.insert(*input, idx);
                    input_before.push((*input, before, None));
                    break;
                }
            }
        }

        builder = builder.watch(gl, vec![proxy]);
        let wait_loop_bb = builder.key();

        for (_, variable, phi_ref) in input_before.iter_mut() {
            let pr;
            (*variable, pr) = builder.phi(gl, [(entry, *variable), (entry, *variable)].into());
            *phi_ref = Some(pr);
        }

        let time = builder.time(gl);
        let mut active_time = builder.constant(gl, Bits::new_zeroed(TIME_VSIZE));
        for (input, _) in &specify.paths {
            let lupdt = builder.lupdt(gl, *input);
            active_time = builder.max(gl, active_time, lupdt);
        }

        let mut wait_time_set = builder.constant(gl, Bits::new_zeroed(SCALAR_VSIZE));
        let mut wait_time = builder.constant(gl, Bits::new_ones(TIME_VSIZE));

        for (input, paths) in &specify.paths {
            let lupdt = builder.lupdt(gl, *input);
            let is_active = builder.equals(gl, lupdt, active_time);

            let start_bb = builder.key();
            builder = builder.next_terminate_later(gl);
            let true_bb = builder.key();

            let mut new_wait_time_set = Some(wait_time_set);
            let mut new_wait_time = wait_time;

            for path in paths {
                let mut condition = None;
                if matches!(
                    path.condition,
                    Condition::InputPosedge | Condition::InputPosedgeExpr(_)
                ) {
                    let before = input_before[input_before_lut[input]].1;
                    let after = builder.probe(gl, *input);
                    condition = Some(builder.posedge(gl, before, after));
                }
                if matches!(
                    path.condition,
                    Condition::InputNegedge | Condition::InputNegedgeExpr(_)
                ) {
                    let before = input_before[input_before_lut[input]].1;
                    let after = builder.probe(gl, *input);
                    condition = Some(builder.negedge(gl, before, after));
                }

                if let Condition::Expr(expr)
                | Condition::InputPosedgeExpr(expr)
                | Condition::InputNegedgeExpr(expr) = path.condition
                {
                    let (expr, _) = lower_expr(gl, arenas, scope, diagnostics, &mut builder, expr)?;
                    let expr = builder.reduce_or(gl, expr);
                    condition = Some(match condition {
                        None => expr,
                        Some(condition) => builder.and(gl, condition, expr),
                    });
                }

                let condition = condition.map(|c| builder.extract_constant(gl, c, 0, SCALAR_VSIZE));
                match (condition, &mut new_wait_time_set) {
                    (None, _) | (_, None) => new_wait_time_set = None,
                    (Some(condition), Some(new_wait_time_set)) => {
                        *new_wait_time_set = builder.or(gl, *new_wait_time_set, condition);
                    }
                }

                let path_wait_time = builder.minus(gl, time, lupdt);
                let delay = builder.constant(gl, Bits::new_u64(path.delays));
                let path_wait_time = builder.plus(gl, path_wait_time, delay);
                let path_wait_time = builder.min(gl, new_wait_time, path_wait_time);


                dbg!(condition.is_some());
                new_wait_time = match condition {
                    None => path_wait_time,
                    Some(condition) => builder.select(gl, condition, path_wait_time, new_wait_time),
                };
            }

            let new_wait_time_set = new_wait_time_set
                .unwrap_or_else(|| builder.constant(gl, Bits::new_ones(SCALAR_VSIZE)));

            let end_bb = builder.key();
            builder = builder.jump(gl);

            gl.bbs[start_bb].terminator =
                BasicBlockTerminator::Branch(is_active, true_bb, builder.key());
            (wait_time_set, _) = builder.phi(
                gl,
                [(start_bb, wait_time_set), (end_bb, new_wait_time_set)].into(),
            );
            (wait_time, _) =
                builder.phi(gl, [(start_bb, wait_time), (end_bb, new_wait_time)].into());
        }

        // @TODO: wait_time_set == 0
        let old_proxy_value = builder.probe(gl, proxy);
        for (input, variable, _) in input_before.iter_mut() {
            *variable = builder.probe(gl, *input);
        }

        builder = builder.variable_wait(gl, wait_time);

        for (_, variable, phi_ref) in input_before.iter_mut() {
            builder.update_phi_ref(gl, phi_ref.take().unwrap(), 1, builder.key(), *variable);
        }

        // do ... while(...);
        let new_proxy_value = builder.probe(gl, proxy);
        let do_while_condition = builder.equals(gl, old_proxy_value, new_proxy_value);
        builder = builder.branch_false_to(gl, do_while_condition, wait_loop_bb);

        builder.drive(gl, output, new_proxy_value);
        builder.jump_to(gl, entry);
    }

    Ok(())
}
