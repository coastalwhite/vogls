use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, Bits, ProcessBuilder, ProcessKind, SCALAR_VSIZE, Signal, SignalFlags,
    VariableKey, VectorSize, WatchCondition,
};
use vogls_utils::OrderedSet;

use crate::ast::AstId;
use crate::ast::module::{
    CmosSwitchType, EnableGateType, GateInstantiation, MosSwitchType, NInputGateInstance,
    NInputGateInstantiation, NInputGateType, NOutputGateInstance, NOutputGateInstantiation,
    NOutputGateType,
};
use crate::ast::statement::{Delay2, Delay3, DelayValue, MinTypMaxExpression, NetLValue};
use crate::lower::assign::{assign_net_lvalue, net_lvalue_bit_length};
use crate::lower::expression::{get_used_signals, lower_expr, truncate_or_extend};
use crate::lower::statement::procedural_timing_control::{
    evaluate_delay_value_as_ticks, lower_min_typ_max_delay,
};
use crate::lower::{LowerContext, MutLowerContext, VType};

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, GateInstantiation<'a>>,
) -> Result<(), ()> {
    use GateInstantiation as G;
    match &*id {
        G::Enable(enable_switch) => {
            if enable_switch.drive_strength.is_some() {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(*enable_switch),
                    "enable gates with drive strength are not yet supported",
                );
                return Err(());
            }
            for i in enable_switch.instances.iter() {
                if i.name.is_some_and(|n| n.range.is_some()) {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(*enable_switch),
                        "enable gate arrays are not yet supported",
                    );
                    return Err(());
                }

                let mut sensitivity_list = OrderedSet::new();
                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.enable_terminal)?;
                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.input_terminal)?;

                let (proc_builder, mut bb_builder) = ProcessBuilder::new(
                    mctx.gl(),
                    ProcessKind::Udp,
                    ctx.arenas.get_span(*enable_switch),
                );
                let entry_tr = proc_builder.entry();

                let (data, _) =
                    lower_expr(ctx, mctx, scope, &mut bb_builder, i.input_terminal, None)?;
                let (control, _) =
                    lower_expr(ctx, mctx, scope, &mut bb_builder, i.enable_terminal, None)?;

                // @NOTE: We collapse L/H to `x`.
                let data = bb_builder.truncate(mctx.gl(), data, SCALAR_VSIZE);
                let data = bb_builder.z_to_x(mctx.gl(), data);
                let control = bb_builder.truncate(mctx.gl(), control, SCALAR_VSIZE);
                let control = bb_builder.z_to_x(mctx.gl(), control);

                let z = bb_builder.constant(mctx.gl(), Bits::new_high_impedance(SCALAR_VSIZE));
                let out = match enable_switch.gatetype.item {
                    EnableGateType::BufIf0 => bb_builder.select_merge(mctx.gl(), control, z, data),
                    EnableGateType::BufIf1 => bb_builder.select_merge(mctx.gl(), control, data, z),
                    EnableGateType::NotIf0 => {
                        let data = bb_builder.binary_not(mctx.gl(), data);
                        bb_builder.select_merge(mctx.gl(), control, z, data)
                    }
                    EnableGateType::NotIf1 => {
                        let data = bb_builder.binary_not(mctx.gl(), data);
                        bb_builder.select_merge(mctx.gl(), control, data, z)
                    }
                };

                let delay = enable_switch.delay.map(|delay3| {
                    move |mctx: &mut MutLowerContext,
                          builder: &mut BasicBlockBuilder,
                          new_value: VariableKey,
                          _driven_value: VariableKey| {
                        lower_delay3(ctx, mctx, scope, builder, delay3, new_value, SCALAR_VSIZE)
                    }
                });
                lower_gate_drive(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    ctx.arenas.get_span(i),
                    delay.as_ref().map(|delay| delay as &GateDelayFn),
                    i.output_terminal,
                    out,
                    SCALAR_VSIZE,
                )?;

                bb_builder.watch_to(mctx.gl(), sensitivity_list.items, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
        G::Mos(mos_switch) => {
            const CONTEXT_WIDTH: Option<VectorSize> = Some(SCALAR_VSIZE);
            for i in mos_switch.instances.iter() {
                let mut sensitivity_list = OrderedSet::new();
                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.enable_terminal)?;
                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.input_terminal)?;

                let (proc_builder, mut bb_builder) = ProcessBuilder::new(
                    mctx.gl(),
                    ProcessKind::Udp,
                    ctx.arenas.get_span(*mos_switch),
                );
                let entry_tr = proc_builder.entry();

                let (data, _) = lower_expr(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    i.input_terminal,
                    CONTEXT_WIDTH,
                )?;
                let (control, _) = lower_expr(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    i.enable_terminal,
                    CONTEXT_WIDTH,
                )?;

                let data = bb_builder.truncate(mctx.gl(), data, SCALAR_VSIZE);
                let control = bb_builder.truncate(mctx.gl(), control, SCALAR_VSIZE);

                let z = bb_builder.constant(mctx.gl(), Bits::new_high_impedance(SCALAR_VSIZE));
                let out = match mos_switch.gatetype.item {
                    MosSwitchType::NMos | MosSwitchType::RNMos => {
                        let control = bb_builder.binary_not(mctx.gl(), control);
                        bb_builder.select_merge(mctx.gl(), control, z, data)
                    }
                    MosSwitchType::PMos | MosSwitchType::RPMos => {
                        bb_builder.select_merge(mctx.gl(), control, z, data)
                    }
                };
                let data_is_z = bb_builder.case_equals_constant(
                    mctx.gl(),
                    data,
                    Bits::new_high_impedance(SCALAR_VSIZE),
                );
                let out = bb_builder.select(mctx.gl(), data_is_z, z, out);

                let delay = mos_switch.delay.map(|delay3| {
                    move |mctx: &mut MutLowerContext,
                          builder: &mut BasicBlockBuilder,
                          new_value: VariableKey,
                          _driven_value: VariableKey| {
                        lower_delay3(ctx, mctx, scope, builder, delay3, new_value, SCALAR_VSIZE)
                    }
                });
                lower_gate_drive(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    ctx.arenas.get_span(i),
                    delay.as_ref().map(|delay| delay as &GateDelayFn),
                    i.output_terminal,
                    out,
                    SCALAR_VSIZE,
                )?;

                bb_builder.watch_to(mctx.gl(), sensitivity_list.items, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
        G::Cmos(cmos_switch) => {
            const CONTEXT_WIDTH: Option<VectorSize> = Some(SCALAR_VSIZE);
            for i in cmos_switch.instances.iter() {
                let mut sensitivity_list = OrderedSet::new();
                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.ncontrol_terminal)?;
                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.pcontrol_terminal)?;
                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.input_terminal)?;

                let (proc_builder, mut bb_builder) = ProcessBuilder::new(
                    mctx.gl(),
                    ProcessKind::Udp,
                    ctx.arenas.get_span(*cmos_switch),
                );
                let entry_tr = proc_builder.entry();

                let (data, _) = lower_expr(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    i.input_terminal,
                    CONTEXT_WIDTH,
                )?;
                let (ncontrol, _) = lower_expr(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    i.ncontrol_terminal,
                    CONTEXT_WIDTH,
                )?;
                let (pcontrol, _) = lower_expr(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    i.pcontrol_terminal,
                    CONTEXT_WIDTH,
                )?;

                let data = bb_builder.truncate(mctx.gl(), data, SCALAR_VSIZE);
                let ncontrol = bb_builder.truncate(mctx.gl(), ncontrol, SCALAR_VSIZE);
                let pcontrol = bb_builder.truncate(mctx.gl(), pcontrol, SCALAR_VSIZE);

                let z = bb_builder.constant(mctx.gl(), Bits::new_high_impedance(SCALAR_VSIZE));
                let out = match cmos_switch.gatetype.item {
                    CmosSwitchType::Cmos | CmosSwitchType::Rcmos => {
                        let andn_control = bb_builder.andnot(mctx.gl(), pcontrol, ncontrol);

                        // out = z    (if both OFF)
                        // out = data (otherwise)
                        bb_builder.select_merge(mctx.gl(), andn_control, z, data)
                    }
                };
                let data_is_z = bb_builder.case_equals_constant(
                    mctx.gl(),
                    data,
                    Bits::new_high_impedance(SCALAR_VSIZE),
                );
                let out = bb_builder.select(mctx.gl(), data_is_z, z, out);

                let delay = cmos_switch.delay.map(|delay3| {
                    move |mctx: &mut MutLowerContext,
                          builder: &mut BasicBlockBuilder,
                          new_value: VariableKey,
                          _driven_value: VariableKey| {
                        lower_delay3(ctx, mctx, scope, builder, delay3, new_value, SCALAR_VSIZE)
                    }
                });
                lower_gate_drive(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    ctx.arenas.get_span(i),
                    delay.as_ref().map(|delay| delay as &GateDelayFn),
                    i.output_terminal,
                    out,
                    SCALAR_VSIZE,
                )?;

                bb_builder.watch_to(mctx.gl(), sensitivity_list.items, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
        G::PassEn(_) | G::PassSwitch(_) => {
            mctx.diagnostics.not_yet_implemented(
                ctx.arenas.get_span(id),
                "gate instantiation type not yet implemented",
            );
            return Err(());
        }
        g @ (G::Pullup(pull_gate) | G::Pulldown(pull_gate)) => {
            if pull_gate.pullup_strength.is_some() {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(*pull_gate),
                    "pulldown/pullup gates with drive strength are not yet supported",
                );
                return Err(());
            }
            let (proc_builder, mut bb_builder) =
                ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(*pull_gate));

            let value = if matches!(g, G::Pullup(_)) {
                Bits::new_ones(SCALAR_VSIZE)
            } else {
                Bits::new_zeroed(SCALAR_VSIZE)
            };
            let value = bb_builder.constant(mctx.gl(), value);

            for instance in pull_gate.instances.iter() {
                let output_bit_length =
                    net_lvalue_bit_length(ctx, mctx, scope, instance.output_terminal)?;
                let value = bb_builder.sign_extend(mctx.gl(), value, output_bit_length);
                assign_net_lvalue(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    instance.output_terminal,
                    value,
                    VType::UnsignedNet(output_bit_length),
                )?;
            }

            bb_builder.halt(mctx.gl());
            proc_builder.finalize(mctx.gl());
        }
        G::NInput(id) => {
            let NInputGateInstantiation {
                gatetype,
                drive_strength,
                delay2,
                instances,
            } = &**id;

            if let Some(drive_strength) = drive_strength {
                mctx.diagnostics.warn_not_yet_implemented(
                    ctx.arenas.get_item_span(*drive_strength),
                    "Drive strength is ignored.",
                );
            }

            use NInputGateType as G;
            for instance in instances.iter() {
                let NInputGateInstance {
                    name: _,
                    output_terminal,
                    input_terminals,
                } = &*instance;

                assert!(!input_terminals.is_empty());

                let (proc_builder, mut bb_builder) =
                    ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(*id));
                let entry_tr = proc_builder.entry();

                let output_bit_length = net_lvalue_bit_length(ctx, mctx, scope, *output_terminal)?;

                let mut sensitivity_list = OrderedSet::new();
                let input = input_terminals.first().unwrap();

                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, input)?;
                let (value, value_ty) = lower_expr(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    input,
                    Some(output_bit_length),
                )?;
                let mut value = truncate_or_extend(
                    mctx.gl(),
                    &mut bb_builder,
                    value,
                    value_ty,
                    output_bit_length,
                );
                for input in input_terminals.iter().skip(1) {
                    get_used_signals(ctx, mctx, scope, &mut sensitivity_list, input)?;
                    let (input, input_ty) = lower_expr(
                        ctx,
                        mctx,
                        scope,
                        &mut bb_builder,
                        input,
                        Some(output_bit_length),
                    )?;
                    let input = truncate_or_extend(
                        mctx.gl(),
                        &mut bb_builder,
                        input,
                        input_ty,
                        output_bit_length,
                    );

                    match gatetype.item {
                        G::And | G::Nand => value = bb_builder.and(mctx.gl(), value, input),
                        G::Or | G::Nor => value = bb_builder.or(mctx.gl(), value, input),
                        G::Xor | G::Xnor => value = bb_builder.xor(mctx.gl(), value, input),
                    }
                }

                if matches!(id.gatetype.item, G::Nand | G::Nor | G::Xnor) {
                    value = bb_builder.binary_not(mctx.gl(), value);
                }

                let delay = delay2.map(|delay2| {
                    move |mctx: &mut MutLowerContext,
                          builder: &mut BasicBlockBuilder,
                          new_value: VariableKey,
                          _driven_value: VariableKey| {
                        lower_delay2(
                            ctx,
                            mctx,
                            scope,
                            builder,
                            delay2,
                            new_value,
                            output_bit_length,
                        )
                    }
                });
                lower_gate_drive(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    ctx.arenas.get_span(instance),
                    delay.as_ref().map(|delay| delay as &GateDelayFn),
                    *output_terminal,
                    value,
                    output_bit_length,
                )?;

                let sensitivity_list = sensitivity_list.items;
                bb_builder.watch_to(mctx.gl(), sensitivity_list, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
        G::NOutput(id) => {
            let NOutputGateInstantiation {
                gatetype,
                drive_strength,
                delay2,
                instances,
            } = &**id;

            if let Some(drive_strength) = drive_strength {
                mctx.diagnostics.warn_not_yet_implemented(
                    ctx.arenas.get_item_span(*drive_strength),
                    "Drive strength is ignored.",
                );
            }

            use NOutputGateType as G;
            for instance in instances.iter() {
                let NOutputGateInstance {
                    name: _,
                    output_terminals,
                    input_terminal,
                } = &*instance;

                let (proc_builder, mut bb_builder) =
                    ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(*id));
                let entry_tr = proc_builder.entry();

                let mut sensitivity_list = OrderedSet::new();
                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, *input_terminal)?;
                let sensitivity_list = sensitivity_list.items;

                for output_terminal in output_terminals.iter() {
                    let output_bit_length =
                        net_lvalue_bit_length(ctx, mctx, scope, output_terminal)?;
                    let (value, value_ty) = lower_expr(
                        ctx,
                        mctx,
                        scope,
                        &mut bb_builder,
                        *input_terminal,
                        Some(output_bit_length),
                    )?;
                    let mut value = truncate_or_extend(
                        mctx.gl(),
                        &mut bb_builder,
                        value,
                        value_ty,
                        output_bit_length,
                    );
                    match gatetype.item {
                        G::Buf => value = bb_builder.z_to_x(mctx.gl(), value),
                        G::Not => value = bb_builder.binary_not(mctx.gl(), value),
                    }

                    let delay = delay2.map(|delay2| {
                        move |mctx: &mut MutLowerContext,
                              builder: &mut BasicBlockBuilder,
                              new_value: VariableKey,
                              _driven_value: VariableKey| {
                            lower_delay2(
                                ctx,
                                mctx,
                                scope,
                                builder,
                                delay2,
                                new_value,
                                output_bit_length,
                            )
                        }
                    });
                    lower_gate_drive(
                        ctx,
                        mctx,
                        scope,
                        &mut bb_builder,
                        ctx.arenas.get_span(instance),
                        delay.as_ref().map(|delay| delay as &GateDelayFn),
                        output_terminal,
                        value,
                        output_bit_length,
                    )?;
                }

                bb_builder.watch_to(mctx.gl(), sensitivity_list, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
    }

    Ok(())
}

/// Called with the new output value and the output value that is currently driven.
pub(crate) type GateDelayFn<'f> = dyn Fn(
        &mut MutLowerContext,
        &mut BasicBlockBuilder,
        VariableKey,
        VariableKey,
    ) -> Result<VariableKey, ()>
    + 'f;

fn lower_delay2<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    delay2: AstId<'a, Delay2<'a>>,
    new_value: VariableKey,
    output_bit_length: VectorSize,
) -> Result<VariableKey, ()> {
    match &*delay2 {
        Delay2::Value(value) => lower_delay_value(ctx, mctx, scope, builder, *value),
        Delay2::Tuple(delay, None) => lower_min_typ_max_delay(ctx, mctx, scope, builder, *delay),
        Delay2::Tuple(rise, Some(fall)) => {
            let span = ctx.arenas.get_span(delay2);
            let transitions = TransitionDelays {
                rise: *rise,
                fall: *fall,
                turn_off: None,
            };
            transitions.lower(
                ctx,
                mctx,
                scope,
                builder,
                span,
                new_value,
                output_bit_length,
            )
        }
    }
}

pub(crate) fn lower_delay3<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    delay3: AstId<'a, Delay3<'a>>,
    new_value: VariableKey,
    output_bit_length: VectorSize,
) -> Result<VariableKey, ()> {
    let (rise, fall, turn_off) = match &*delay3 {
        Delay3::Value(value) => return lower_delay_value(ctx, mctx, scope, builder, *value),
        Delay3::Single(delay) => {
            return lower_min_typ_max_delay(ctx, mctx, scope, builder, *delay);
        }
        Delay3::Double(rise, fall) => (*rise, *fall, None),
        Delay3::Triple(rise, fall, turn_off) => (*rise, *fall, Some(*turn_off)),
    };
    let span = ctx.arenas.get_span(delay3);
    let transitions = TransitionDelays {
        rise,
        fall,
        turn_off,
    };
    transitions.lower(
        ctx,
        mctx,
        scope,
        builder,
        span,
        new_value,
        output_bit_length,
    )
}

pub(crate) fn lower_delay_value<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    value: AstId<'a, DelayValue>,
) -> Result<VariableKey, ()> {
    let delay = evaluate_delay_value_as_ticks(ctx, mctx, scope, value)?;
    Ok(builder.constant_u64(mctx.gl(), delay))
}

/// Separate delays for transitions to 1, 0 and `z`.
struct TransitionDelays<'a> {
    rise: AstId<'a, MinTypMaxExpression<'a>>,
    fall: AstId<'a, MinTypMaxExpression<'a>>,
    turn_off: Option<AstId<'a, MinTypMaxExpression<'a>>>,
}

impl<'a> TransitionDelays<'a> {
    fn lower(
        self,
        ctx: &LowerContext<'a, '_>,
        mctx: &mut MutLowerContext,
        scope: SymbolId,
        builder: &mut BasicBlockBuilder,
        span: TokenRange,
        new_value: VariableKey,
        output_bit_length: VectorSize,
    ) -> Result<VariableKey, ()> {
        if output_bit_length != SCALAR_VSIZE {
            mctx.diagnostics
                .not_yet_implemented(span, "rise and fall delays on non-scalar gate outputs");
            return Err(());
        }

        let rise = lower_min_typ_max_delay(ctx, mctx, scope, builder, self.rise)?;
        let fall = lower_min_typ_max_delay(ctx, mctx, scope, builder, self.fall)?;
        let turn_off = self
            .turn_off
            .map(|t| lower_min_typ_max_delay(ctx, mctx, scope, builder, t))
            .transpose()?;

        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 101 (Table 7-9)
        //
        // Transitions to 1 take the rise delay and transitions to 0 take the fall delay.
        // Transitions to z take the turn-off delay, or the smaller of the rise and fall delay when
        // there is none. Transitions to x take the smallest of all delays.
        let min_rise_fall = builder.min(mctx.gl(), rise, fall);
        let (to_z, to_x) = match turn_off {
            None => (min_rise_fall, min_rise_fall),
            Some(turn_off) => (turn_off, builder.min(mctx.gl(), min_rise_fall, turn_off)),
        };

        let is_one = builder.case_equals_constant(mctx.gl(), new_value, Bits::from(true));
        let is_zero = builder.case_equals_constant(mctx.gl(), new_value, Bits::from(false));
        let is_z = builder.case_equals_constant(
            mctx.gl(),
            new_value,
            Bits::new_high_impedance(SCALAR_VSIZE),
        );
        let delay = builder.select(mctx.gl(), is_z, to_z, to_x);
        let delay = builder.select(mctx.gl(), is_zero, fall, delay);
        Ok(builder.select(mctx.gl(), is_one, rise, delay))
    }
}

pub(crate) fn lower_gate_drive<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    bb_builder: &mut BasicBlockBuilder,
    span: TokenRange,
    delay: Option<&GateDelayFn<'_>>,
    output_terminal: AstId<'a, NetLValue<'a>>,
    value: VariableKey,
    output_bit_length: VectorSize,
) -> Result<(), ()> {
    let assign = |mctx: &mut MutLowerContext, builder: &mut BasicBlockBuilder, value| {
        assign_net_lvalue(
            ctx,
            mctx,
            scope,
            builder,
            output_terminal,
            value,
            VType::UnsignedNet(output_bit_length),
        )
    };
    lower_delayed_drive(
        mctx,
        bb_builder,
        span,
        delay,
        &assign,
        value,
        output_bit_length,
    )
}

/// Assigns a value to the output of a [`lower_delayed_drive`].
pub(crate) type AssignOutputFn<'f> =
    dyn Fn(&mut MutLowerContext, &mut BasicBlockBuilder, VariableKey) -> Result<(), ()> + 'f;

/// Assign `value` to an output with `assign`, after the delay that `delay` gives.
pub(crate) fn lower_delayed_drive(
    mctx: &mut MutLowerContext,
    bb_builder: &mut BasicBlockBuilder,
    span: TokenRange,
    delay: Option<&GateDelayFn<'_>>,
    assign: &AssignOutputFn<'_>,
    value: VariableKey,
    output_bit_length: VectorSize,
) -> Result<(), ()> {
    let Some(delay) = delay else {
        // Fast path: no delay on the output.
        return assign(mctx, bb_builder, value);
    };

    let proxy = mctx.gl.signals.insert(Signal {
        name: format!("GATE_DELAY_PROXY/{}", mctx.gl.signals.len()),
        size: output_bit_length,
        initialize: None,
        flags: SignalFlags::EMPTY,
        origin: span,
        mode: value.mode(),
    });
    bb_builder.drive(mctx.gl(), proxy, value);

    let (mut delay_proc_builder, mut delay_bb_builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, span);
    let delay_entry_tr = delay_proc_builder.entry();

    let new_value = delay_bb_builder.probe(mctx.gl(), proxy);

    // The value this process last drove onto the output.
    let driven = mctx.gl.signals.insert(Signal {
        name: format!("GATE_DELAY_DRIVEN/{}", mctx.gl.signals.len()),
        size: output_bit_length,
        initialize: None,
        flags: SignalFlags::EMPTY,
        origin: span,
        mode: new_value.mode(),
    });
    let driven_value = delay_bb_builder.probe(mctx.gl(), driven);

    let delay = delay(mctx, &mut delay_bb_builder, new_value, driven_value)?;

    // Only wait for the time that that is remaining.
    let time = delay_bb_builder.time(mctx.gl());
    let last_update = delay_bb_builder.lupdt(mctx.gl(), proxy);
    let elapsed = delay_bb_builder.minus(mctx.gl(), time, last_update);
    let elapsed = delay_bb_builder.min(mctx.gl(), elapsed, delay);
    let wait_time = delay_bb_builder.minus(mctx.gl(), delay, elapsed);

    let scheduled = mctx.gl.signals.insert(Signal {
        name: format!("GATE_DELAY_SCHEDULED/{}", mctx.gl.signals.len()),
        size: output_bit_length,
        initialize: None,
        flags: SignalFlags::EMPTY,
        origin: span,
        mode: new_value.mode(),
    });
    delay_bb_builder.drive(mctx.gl(), scheduled, new_value);

    let after_tr = delay_proc_builder.next_temporal_region(mctx.gl());
    delay_bb_builder.variable_wait_to(mctx.gl(), wait_time, after_tr);
    delay_bb_builder.finished_switch_to(mctx.gl(), after_tr.entry());

    // If the proxy changed, go back to waiting.
    let scheduled = delay_bb_builder.probe(mctx.gl(), scheduled);
    let value = delay_bb_builder.probe(mctx.gl(), proxy);
    let unchanged = delay_bb_builder.case_equals(mctx.gl(), scheduled, value);
    let mut reschedule_builder;
    (delay_bb_builder, reschedule_builder) = delay_bb_builder.double_branch(mctx.gl(), unchanged);
    reschedule_builder.temporal_jump_to(mctx.gl(), delay_entry_tr);

    assign(mctx, &mut delay_bb_builder, value)?;
    delay_bb_builder.drive(mctx.gl(), driven, value);

    delay_bb_builder.watch_to(mctx.gl(), vec![proxy], delay_entry_tr);
    let standing = [WatchCondition::entire_signal(&mctx.gl, proxy)].into();
    delay_proc_builder.set_standing(mctx.gl(), standing);
    delay_proc_builder.finalize(mctx.gl());

    Ok(())
}
