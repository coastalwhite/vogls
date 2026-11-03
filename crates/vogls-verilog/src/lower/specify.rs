use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    Bits, ProcessBuilder, ProcessKind, SCALAR_VSIZE, Signal, SignalFlags, SignalKey, TIME_VSIZE,
    Time, VariableKey,
};
use vogls_utils::VgHashMap;

use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::expr::Expr;
use crate::ast::specify::{
    EdgeIdentifier, PathDeclaration, PathDeclarationVariant, SpecifyBlockItem,
    StateDependentCondition,
};
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::VSymbol;
use crate::lower::delay_pointer::{DelayPtr, DelayPtrVariant};
use crate::lower::expression::lower_expr;
use crate::lower::{eval_constant_expr, try_resolve_hident, unwrap_get_signal_mut};

use super::{LowerContext, MutLowerContext, VValue};

pub enum Condition<'a> {
    None,

    InputPosedge,
    InputNegedge,
    NoOtherCondition,

    InputPosedgeNoOtherCondition,
    InputNegedgeNoOtherCondition,

    Expr(AstId<'a, Expr<'a>>),
    InputPosedgeExpr(AstId<'a, Expr<'a>>),
    InputNegedgeExpr(AstId<'a, Expr<'a>>),
}

impl Condition<'_> {
    fn is_posedge(&self) -> bool {
        matches!(
            self,
            Self::InputPosedge | Self::InputPosedgeNoOtherCondition | Self::InputPosedgeExpr(_)
        )
    }

    fn is_negedge(&self) -> bool {
        matches!(
            self,
            Self::InputNegedge | Self::InputNegedgeNoOtherCondition | Self::InputNegedgeExpr(_)
        )
    }

    /// Does this path only apply when no other path for the same input and output applies?
    fn is_ifnone(&self) -> bool {
        matches!(
            self,
            Self::NoOtherCondition
                | Self::InputPosedgeNoOtherCondition
                | Self::InputNegedgeNoOtherCondition
        )
    }
}

pub struct SpecifyOutput<'a> {
    pub sid: SymbolId,
    pub inputs: VgHashMap<SignalKey, usize>,
    pub paths: Vec<(SignalKey, Vec<SpecifyPath<'a>>)>,
}

pub struct SpecifyPath<'a> {
    pub condition: Condition<'a>,
    pub delays: DelayPtr,
    pub span: TokenRange,
}

/// Scratch buffers for [`lower_iopath`], kept between calls to avoid reallocating them.
#[derive(Default)]
pub struct LowerIoPathScratches {
    /// The index into `inputs_before` of each input with an edge-sensitive path.
    input_before_lut: VgHashMap<SignalKey, usize>,
    /// Each input with an edge-sensitive path, its value before the last change and the signal
    /// that holds that value.
    inputs_before: Vec<(SignalKey, VariableKey, SignalKey)>,
    /// The condition under which each path of an input applies, where `None` means always.
    conditions: Vec<Option<VariableKey>>,
}

fn eval_delay_to_ticks<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, ConstantExpr<'a>>,
) -> Result<u64, ()> {
    let value = eval_constant_expr(
        &mctx.gl,
        ctx.arenas,
        &ctx.table,
        scope,
        &mut mctx.diagnostics,
        id,
        None,
    )?;

    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 222
    // "If the path delay expression results in a negative value, it shall be treated as zero."
    //
    // Negative reals already become zero in `delay_to_ticks`.
    if let VValue::SignedNet(bits) = &value
        && !bits.contains_special()
        && bits.select_bit(bits.size().get() - 1)
    {
        return Ok(0);
    }

    Ok(value.delay_to_ticks(ctx.time_scale, ctx.time_resolution))
}

/// Evaluate one path delay and append it to the delay store.
fn push_delay<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, ConstantMinTypMaxExpression<'a>>,
    delays: &mut Vec<Time>,
) -> Result<bool, ()> {
    match &*id {
        ConstantMinTypMaxExpression::Single(delay) => {
            let delay = eval_delay_to_ticks(ctx, mctx, scope, *delay)?;
            delays.push(Time(delay));
            Ok(false)
        }
        ConstantMinTypMaxExpression::MinTypMax { min, typ, max } => {
            let min = eval_delay_to_ticks(ctx, mctx, scope, *min)?;
            let typ = eval_delay_to_ticks(ctx, mctx, scope, *typ)?;
            let max = eval_delay_to_ticks(ctx, mctx, scope, *max)?;
            delays.extend([Time(min), Time(typ), Time(max)]);
            Ok(true)
        }
    }
}

pub fn lower_specify<'a>(
    ctx: &mut LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,

    items: AstIdRange<'a, SpecifyBlockItem<'a>>,
    delays: &mut Vec<Time>,
    outs_lut: &mut VgHashMap<SignalKey, usize>,
    outs: &mut Vec<(SignalKey, SpecifyOutput<'a>)>,
) -> Result<(), ()> {
    delays.clear();

    for item in items.iter() {
        match &*item {
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
                _ = data_source_expression;

                let condition = match (state_dependent_condition, edge_identifier) {
                    (None, None) => Condition::None,
                    (Some(c), None) => match &**c {
                        StateDependentCondition::If(id) => Condition::Expr(id.into_expr()),
                        StateDependentCondition::Ifnone => Condition::NoOtherCondition,
                    },
                    (None, Some(e)) => match e.item {
                        EdgeIdentifier::Posedge => Condition::InputPosedge,
                        EdgeIdentifier::Negedge => Condition::InputNegedge,
                    },
                    (Some(c), Some(e)) => match (&**c, e.item) {
                        (StateDependentCondition::If(id), EdgeIdentifier::Posedge) => {
                            Condition::InputPosedgeExpr(id.into_expr())
                        }
                        (StateDependentCondition::If(id), EdgeIdentifier::Negedge) => {
                            Condition::InputNegedgeExpr(id.into_expr())
                        }
                        (StateDependentCondition::Ifnone, EdgeIdentifier::Posedge) => {
                            Condition::InputPosedgeNoOtherCondition
                        }
                        (StateDependentCondition::Ifnone, EdgeIdentifier::Negedge) => {
                            Condition::InputNegedgeNoOtherCondition
                        }
                    },
                };

                // @TODO: Remove assertions
                assert_eq!(input_terminal_descriptors.len(), 1);
                assert_eq!(output_terminal_descriptors.len(), 1);

                let input = input_terminal_descriptors.get(0);
                let output = output_terminal_descriptors.get(0);

                let (None, None) = (input.constant_range_expr, output.constant_range_expr) else {
                    todo!()
                };

                let input_sid = try_resolve_hident(
                    scope,
                    &ctx.table,
                    ctx.arenas,
                    input.ident,
                    &mut mctx.diagnostics,
                )?;
                let VSymbol::Signal(input_net) = &ctx.table[input_sid].content else {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_item_span(input.ident),
                        "cannot be used as net",
                    );
                    return Err(());
                };
                let input = input_net.terminals.probe_signal();
                let output_sid = try_resolve_hident(
                    scope,
                    &ctx.table,
                    ctx.arenas,
                    output.ident,
                    &mut mctx.diagnostics,
                )?;
                let VSymbol::Signal(output_net) = &ctx.table[output_sid].content else {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_item_span(output.ident),
                        "cannot be used as net",
                    );
                    return Err(());
                };
                let output = output_net.terminals.blocking_drive_signal();

                if matches!(variant, PathDeclarationVariant::Full)
                    && input_net.ty.bit_length() != output_net.ty.bit_length()
                {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item),
                        "input and output don't have the same net width",
                    );
                    return Err(());
                }
                if matches!(variant, PathDeclarationVariant::Parallel)
                    && input_net.ty.bit_length().get() != 1
                {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item),
                        "parallel specify with `n` wide input",
                    );
                    return Err(());
                }
                if !input_net.dims.is_empty() || !output_net.dims.is_empty() {
                    mctx.diagnostics
                        .not_yet_implemented(ctx.arenas.get_span(item), "input or output is array");
                    return Err(());
                }
                if input_net.ty.bit_length() != SCALAR_VSIZE {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item),
                        "specify for non-scalar net",
                    );
                    return Err(());
                }

                let exprs = path_delay_value.list_of_delay_expressions;
                use DelayPtrVariant as V;
                let delay_variant = match exprs.len() {
                    1 => V::One,
                    2 => V::Two,
                    3 => V::Three,
                    6 => V::Six,
                    12 => V::Twelve,
                    // The parser only accepts these counts.
                    n => unreachable!("{n} path delays"),
                };
                let offset = delays.len() as u64;
                let mut triple_mask = 0u16;
                for (i, expr) in exprs.iter().enumerate() {
                    let is_triple = push_delay(ctx, mctx, scope, expr, delays)?;
                    triple_mask |= u16::from(is_triple) << i;
                }
                let delays = DelayPtr::new(offset, delay_variant, triple_mask);

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
                out.paths[paths_idx].1.push(SpecifyPath {
                    condition,
                    delays,
                    span: ctx.arenas.get_span(item),
                });
            }
            SpecifyBlockItem::SystemTimingCheck(_) => {
                // @TODO: Don't ignore these.
            }
        }
    }

    let mut scratches = LowerIoPathScratches::default();
    // let mut condition_orsum = VgHashMap::<SignalKey, VariableKey>::default();

    outs_lut.clear();
    for (output, specify) in outs.drain(..) {
        lower_iopath(ctx, mctx, scope, output, specify, delays, &mut scratches)?;
    }

    Ok(())
}

pub fn lower_iopath<'a>(
    ctx: &mut LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    output: SignalKey,
    specify: SpecifyOutput,
    delays: &[Time],
    scratches: &mut LowerIoPathScratches,
) -> Result<(), ()> {
    let LowerIoPathScratches {
        input_before_lut,
        inputs_before,
        conditions,
    } = scratches;

    let mut proxy = mctx.gl.signals.get(output).unwrap().clone();
    proxy.name = format!("{}::SPECIFY_PROXY", proxy.name);
    let proxy = mctx.gl.signals.insert(proxy);
    let symbol = unwrap_get_signal_mut(&mut ctx.table, specify.sid);
    let prev = symbol.terminals.set_specify(proxy);
    assert!(prev.is_none());

    for output_bitidx in 0..mctx.gl.signals[output].size.get() {
        input_before_lut.clear();
        inputs_before.clear();

        let (mut proc_builder, mut builder) =
            ProcessBuilder::new(mctx.gl(), ProcessKind::Specify, TokenRange::default());

        // @Correctness: This might need something like. `Initial Value of Signal X`. I think
        // this is incorrect if the event does not get triggered first.
        for (input, paths) in &specify.paths {
            for path in paths {
                if path.condition.is_posedge() || path.condition.is_negedge() {
                    let Signal { size, mode, .. } = &mctx.gl.signals[*input];
                    let (size, mode) = (*size, *mode);
                    let mut flags = SignalFlags::EMPTY;
                    flags.set(SignalFlags::POKE_T0, mode.is_two_value());
                    let signal = mctx.gl.signals.insert(Signal {
                        name: format!("SPECIFY_BEFORE/{}", mctx.gl.signals.len()),
                        size,
                        initialize: None,
                        flags,
                        origin: TokenRange::default(),
                        mode,
                    });
                    let before = builder.probe(mctx.gl(), *input);
                    builder.drive(mctx.gl(), signal, before);
                    let idx = inputs_before.len();
                    input_before_lut.insert(*input, idx);
                    inputs_before.push((*input, before, signal));
                    break;
                }
            }
        }

        let wait_loop_tr = proc_builder.next_temporal_region(mctx.gl());
        builder.temporal_jump_to(mctx.gl(), wait_loop_tr);
        builder.finished_switch_to(mctx.gl(), wait_loop_tr.entry());

        for (_, variable, signal) in inputs_before.iter_mut() {
            *variable = builder.probe(mctx.gl(), *signal);
        }

        // active_time = max_{signal} last_update_time(signal)
        // is_active(s) = last_update_time(s) == active_time && active_time != 0xF..F
        let mut active_time = builder.constant(mctx.gl(), Bits::from_u64(TIME_VSIZE, 1));
        for (input, _) in &specify.paths {
            let lupdt = builder.lupdt(mctx.gl(), *input);
            let lupdt = builder.plus_constant(mctx.gl(), lupdt, Bits::from_u64(TIME_VSIZE, 1));
            active_time = builder.max(mctx.gl(), active_time, lupdt);
        }
        active_time = builder.minus_constant(mctx.gl(), active_time, Bits::from_u64(TIME_VSIZE, 1));

        let mut endwait_time_set = builder.constant(mctx.gl(), Bits::new_zeroed(SCALAR_VSIZE));
        let mut endwait_time = builder.constant(mctx.gl(), Bits::new_ones(TIME_VSIZE));

        for (input, paths) in &specify.paths {
            let lupdt = builder.lupdt(mctx.gl(), *input);
            let is_active = builder.case_equals(mctx.gl(), lupdt, active_time);

            let mut new_endwait_time_set = Some(endwait_time_set);
            let mut new_wait_time = endwait_time;

            // The condition under which each path applies, where `None` means always. The
            // condition of an `ifnone` path depends on the other paths, so it is filled in after.
            conditions.clear();
            for path in paths {
                let mut condition = None;
                if path.condition.is_posedge() {
                    let before = inputs_before[input_before_lut[input]].1;
                    let after = builder.probe(mctx.gl(), *input);
                    condition = Some(builder.posedge(mctx.gl(), before, after));
                }
                if path.condition.is_negedge() {
                    let before = inputs_before[input_before_lut[input]].1;
                    let after = builder.probe(mctx.gl(), *input);
                    condition = Some(builder.negedge(mctx.gl(), before, after));
                }

                if let Condition::Expr(expr)
                | Condition::InputPosedgeExpr(expr)
                | Condition::InputNegedgeExpr(expr) = path.condition
                {
                    let (expr, _) = lower_expr(ctx, mctx, scope, &mut builder, expr, None)?;

                    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 215
                    //
                    // """
                    // If the conditional expression evaluates to x or z, it shall be treated as
                    // true. If the conditional expression evaluates to multiple bits, the least
                    // significant bit shall represent the result.
                    // """
                    let expr = builder.select_bit_constant(mctx.gl(), expr, 0);
                    let expr = builder.not_case_equals_constant(mctx.gl(), expr, Bits::from(false));
                    condition = Some(match condition {
                        None => expr,
                        Some(condition) => builder.and(mctx.gl(), condition, expr),
                    });
                }

                conditions.push(condition.map(|c| builder.select_bit_constant(mctx.gl(), c, 0)));
            }

            // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 218
            //
            // """
            // The ifnone keyword is used to specify a default state-dependent path delay when all
            // other conditions for the path are false.
            // [..]
            // If there are no corresponding state-dependent module paths to the ifnone module
            // path, then the ifnone module path shall be treated the same as an unconditional
            // simple module path.
            //
            // It is illegal to specify both an ifnone condition for a module path and an
            // unconditional simple module path for the same module path.
            // """
            if paths.iter().any(|path| path.condition.is_ifnone()) {
                let mut any_other_condition = None;
                for (path, condition) in paths.iter().zip(conditions.iter()) {
                    if path.condition.is_ifnone() {
                        continue;
                    }
                    let Some(condition) = *condition else {
                        mctx.diagnostics.ifnone_with_unconditional_path(path.span);
                        return Err(());
                    };
                    let is_true =
                        builder.case_equals_constant(mctx.gl(), condition, Bits::from(true));
                    any_other_condition = Some(match any_other_condition {
                        None => is_true,
                        Some(any) => builder.or(mctx.gl(), any, is_true),
                    });
                }

                let no_other_condition =
                    any_other_condition.map(|any| builder.binary_not(mctx.gl(), any));
                for (path, condition) in paths.iter().zip(conditions.iter_mut()) {
                    if path.condition.is_ifnone() {
                        *condition = match (*condition, no_other_condition) {
                            (None, c) | (c, None) => c,
                            (Some(edge), Some(no_other)) => {
                                Some(builder.and(mctx.gl(), edge, no_other))
                            }
                        };
                    }
                }
            }

            for (path, &condition) in paths.iter().zip(conditions.iter()) {
                match (condition, &mut new_endwait_time_set) {
                    (None, _) | (_, None) => new_endwait_time_set = None,
                    (Some(condition), Some(new_wait_time_set)) => {
                        *new_wait_time_set = builder.or(mctx.gl(), *new_wait_time_set, condition);
                    }
                }

                let delay = path.delays.calculate(
                    delays,
                    mctx.gl(),
                    &mut builder,
                    output,
                    proxy,
                    output_bitidx,
                );
                let path_wait_time = builder.min(mctx.gl(), new_wait_time, delay);

                new_wait_time = match condition {
                    None => path_wait_time,
                    Some(condition) => {
                        builder.select(mctx.gl(), condition, path_wait_time, new_wait_time)
                    }
                };
            }

            let new_endwait_time_set = new_endwait_time_set
                .unwrap_or_else(|| builder.constant(mctx.gl(), Bits::new_ones(SCALAR_VSIZE)));

            endwait_time_set =
                builder.select(mctx.gl(), is_active, new_endwait_time_set, endwait_time_set);
            endwait_time = builder.select(mctx.gl(), is_active, new_wait_time, endwait_time);
        }

        // Set the wait time to zero, if no condition matched.
        let zero = builder.constant_u64(mctx.gl(), 0);
        let time = builder.time(mctx.gl());

        // wait_time = endwait_time.saturating_sub(time - active_time)
        let active_offset = builder.minus(mctx.gl(), time, active_time);
        let active_offset = builder.min(mctx.gl(), active_offset, endwait_time);
        let wait_time = builder.minus(mctx.gl(), endwait_time, active_offset);
        let wait_time = builder.select(mctx.gl(), endwait_time_set, wait_time, zero);

        let old_proxy_value = builder.probe(mctx.gl(), proxy);
        let old_proxy_value =
            builder.select_bit_constant(mctx.gl(), old_proxy_value, output_bitidx);
        let mut flags = SignalFlags::EMPTY;
        flags.set(SignalFlags::POKE_T0, old_proxy_value.mode().is_two_value());
        let old_proxy_signal = mctx.gl.signals.insert(Signal {
            name: format!("SPECIFY_OLD_PROXY/{}", mctx.gl.signals.len()),
            size: mctx.gl.vars.size(old_proxy_value),
            initialize: None,
            flags,
            origin: TokenRange::default(),
            mode: old_proxy_value.mode(),
        });
        builder.drive(mctx.gl(), old_proxy_signal, old_proxy_value);
        for (input, _, signal) in inputs_before.iter_mut() {
            let new_value = builder.probe(mctx.gl(), *input);
            builder.drive(mctx.gl(), *signal, new_value);
        }

        let after_tr = proc_builder.next_temporal_region(mctx.gl());
        builder.variable_wait_to(mctx.gl(), wait_time, after_tr);
        builder.finished_switch_to(mctx.gl(), after_tr.entry());

        // do ... while(...);
        let new_proxy_value = builder.probe(mctx.gl(), proxy);
        let new_proxy_value =
            builder.select_bit_constant(mctx.gl(), new_proxy_value, output_bitidx);
        let old_proxy_value = builder.probe(mctx.gl(), old_proxy_signal);
        let do_while_condition = builder.case_equals(mctx.gl(), old_proxy_value, new_proxy_value);
        let mut false_builder;
        (builder, false_builder) = builder.double_branch(mctx.gl(), do_while_condition);
        false_builder.temporal_jump_to(mctx.gl(), wait_loop_tr);

        builder.drive_partial_constant(mctx.gl(), output, new_proxy_value, output_bitidx);
        builder.watch_to(mctx.gl(), vec![proxy], wait_loop_tr);
        proc_builder.finalize(mctx.gl());
    }

    Ok(())
}
