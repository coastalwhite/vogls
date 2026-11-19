use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, Bits, ProcessBuilder, ProcessKind, SCALAR_VSIZE, Signal, SignalFlags,
    SignalKey, SignalSlice, VariableKey, VectorSize, WatchCondition,
};
use vogls_utils::OrderedSet;

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::module::{
    CmosSwitchType, EnableGateType, GateInstantiation, MosSwitchType, NInputGateInstance,
    NInputGateInstantiation, NInputGateType, NOutputGateInstance, NOutputGateInstantiation,
    NOutputGateType,
};
use crate::ast::statement::{Delay2, Delay3, DelayValue, MinTypMaxExpression, NetLValue};
use crate::lower::assign::net_lvalue_bit_length;
use crate::lower::drivers::{DriveOutputFn, ExprSource, GateDelay};
use crate::lower::expression::{get_used_signals, lower_expr, truncate_or_extend};
use crate::lower::fuse::{
    RequiredSignalKind, SiteTerminal, SiteType, SourceSite, SourceSiteContent, expr_to_src_sites,
    net_flat_lvalue_to_dsts,
};
use crate::lower::statement::procedural_timing_control::{
    evaluate_delay_value_as_ticks, lower_min_typ_max_delay,
};
use crate::lower::{LowerContext, MutLowerContext};

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
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
            for (index, i) in enable_switch.instances.iter().enumerate() {
                if i.name.is_some_and(|n| n.range.is_some()) {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(*enable_switch),
                        "enable gate arrays are not yet supported",
                    );
                    return Err(());
                }

                push_gate(
                    ctx,
                    mctx,
                    scope,
                    (id, index, 0),
                    i.output_terminal,
                    enable_switch.delay.map(GateDelay::Delay3),
                    [i.enable_terminal, i.input_terminal],
                )?;
            }
        }
        G::Mos(mos_switch) => {
            for (index, i) in mos_switch.instances.iter().enumerate() {
                push_gate(
                    ctx,
                    mctx,
                    scope,
                    (id, index, 0),
                    i.output_terminal,
                    mos_switch.delay.map(GateDelay::Delay3),
                    [i.enable_terminal, i.input_terminal],
                )?;
            }
        }
        G::Cmos(cmos_switch) => {
            for (index, i) in cmos_switch.instances.iter().enumerate() {
                push_gate(
                    ctx,
                    mctx,
                    scope,
                    (id, index, 0),
                    i.output_terminal,
                    cmos_switch.delay.map(GateDelay::Delay3),
                    [i.ncontrol_terminal, i.pcontrol_terminal, i.input_terminal],
                )?;
            }
        }
        G::PassSwitch(pass_switch) => {
            for i in pass_switch.instances.iter() {
                lower_pass_switch_instance(ctx, mctx, scope, i.fst, i.snd)?;
            }
        }
        G::PassEn(_) => {
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

            for instance in pull_gate.instances.iter() {
                let output_bit_length =
                    net_lvalue_bit_length(ctx, mctx, scope, instance.output_terminal)?;
                let value = if matches!(g, G::Pullup(_)) {
                    Bits::new_ones(output_bit_length)
                } else {
                    Bits::new_zeroed(output_bit_length)
                };
                net_lvalue_to_dsts(ctx, mctx, scope, instance.output_terminal)?;
                mctx.fuse_srcs_scratch.clear();
                mctx.fuse_srcs_scratch.push(SourceSite {
                    content: SourceSiteContent::Constant(value),
                    ty: SiteType::Unsigned,
                });
                mctx.driver_resolution
                    .push_sites(
                        &mctx.gl,
                        &mctx.fuse_srcs_scratch,
                        &mctx.fuse_dsts_scratch,
                        None,
                    )
                    .expect("a constant can always be wired");
            }
        }
        G::NInput(ninput) => {
            let NInputGateInstantiation {
                gatetype: _,
                drive_strength,
                delay2,
                instances,
            } = &**ninput;

            if let Some(drive_strength) = drive_strength {
                mctx.diagnostics.warn_not_yet_implemented(
                    ctx.arenas.get_item_span(*drive_strength),
                    "Drive strength is ignored.",
                );
            }

            for (index, instance) in instances.iter().enumerate() {
                let NInputGateInstance {
                    name: _,
                    output_terminal,
                    input_terminals,
                } = &*instance;

                assert!(!input_terminals.is_empty());
                push_gate(
                    ctx,
                    mctx,
                    scope,
                    (id, index, 0),
                    *output_terminal,
                    delay2.map(GateDelay::Delay2),
                    input_terminals.iter(),
                )?;
            }
        }
        G::NOutput(noutput) => {
            let NOutputGateInstantiation {
                gatetype: _,
                drive_strength,
                delay2,
                instances,
            } = &**noutput;

            if let Some(drive_strength) = drive_strength {
                mctx.diagnostics.warn_not_yet_implemented(
                    ctx.arenas.get_item_span(*drive_strength),
                    "Drive strength is ignored.",
                );
            }

            for (index, instance) in instances.iter().enumerate() {
                let NOutputGateInstance {
                    name: _,
                    output_terminals,
                    input_terminal,
                } = &*instance;

                for (output, output_terminal) in output_terminals.iter().enumerate() {
                    push_gate(
                        ctx,
                        mctx,
                        scope,
                        (id, index, output),
                        output_terminal,
                        delay2.map(GateDelay::Delay2),
                        [*input_terminal],
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// Pushes output `output` of instance `index` of `gate`, which drives `output_terminal` after
/// `delay` and is evaluated from `inputs`.
fn push_gate<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    (gate, index, output): (AstId<'a, GateInstantiation<'a>>, usize, usize),
    output_terminal: AstId<'a, NetLValue<'a>>,
    delay: Option<GateDelay<'a>>,
    inputs: impl IntoIterator<Item = AstId<'a, Expr<'a>>>,
) -> Result<(), ()> {
    let mut reads = OrderedSet::new();
    for input in inputs {
        get_used_signals(ctx, mctx, scope, &mut reads, input)?;
    }

    let bit_length = net_lvalue_bit_length(ctx, mctx, scope, output_terminal)?;
    net_lvalue_to_dsts(ctx, mctx, scope, output_terminal)?;
    let primitive = ExprSource::Gate {
        gate,
        scope,
        instance: u32::try_from(index).unwrap(),
        output: u32::try_from(output).unwrap(),
    };
    mctx.driver_resolution.push_primitive(
        &mut mctx.gl,
        ctx.logic_mode,
        primitive,
        &mctx.fuse_dsts_scratch,
        bit_length,
        delay,
        reads.items,
    );
    Ok(())
}

/// Collects the destinations of `output_terminal` into `mctx.fuse_dsts_scratch`, from least to
/// most significant.
pub(crate) fn net_lvalue_to_dsts<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    output_terminal: AstId<'a, NetLValue<'a>>,
) -> Result<(), ()> {
    mctx.fuse_dsts_scratch.clear();
    mctx.fuse_srcs_scratch.clear();
    for net_flat_lvalue in output_terminal.0.iter().rev() {
        net_flat_lvalue_to_dsts(
            ctx,
            &mctx.gl,
            &mut mctx.fuse_dsts_scratch,
            &mut mctx.fuse_srcs_scratch,
            &mut mctx.diagnostics,
            scope,
            net_flat_lvalue,
        )?;
    }
    Ok(())
}

/// Lowers output `output` of instance `instance` of `gate` to a process that drives its value
/// with `drive`.
pub(crate) fn lower_gate_primitive<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    gate: AstId<'a, GateInstantiation<'a>>,
    instance: u32,
    output: u32,
    drive: &DriveOutputFn<'a, '_>,
) -> Result<(), ()> {
    let instance = instance as usize;
    let mut sensitivity_list = OrderedSet::new();

    use GateInstantiation as G;
    let (proc_builder, mut bb_builder, value) = match &*gate {
        G::Enable(enable_switch) => {
            let i = enable_switch.instances.get(instance);
            get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.enable_terminal)?;
            get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.input_terminal)?;

            let (proc_builder, mut bb_builder) = ProcessBuilder::new(
                mctx.gl(),
                ProcessKind::Udp,
                ctx.arenas.get_span(*enable_switch),
            );

            let (data, _) = lower_expr(ctx, mctx, scope, &mut bb_builder, i.input_terminal, None)?;
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
            (proc_builder, bb_builder, out)
        }
        G::Mos(mos_switch) => {
            const CONTEXT_WIDTH: Option<VectorSize> = Some(SCALAR_VSIZE);
            let i = mos_switch.instances.get(instance);
            get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.enable_terminal)?;
            get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.input_terminal)?;

            let (proc_builder, mut bb_builder) = ProcessBuilder::new(
                mctx.gl(),
                ProcessKind::Udp,
                ctx.arenas.get_span(*mos_switch),
            );

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
            (proc_builder, bb_builder, out)
        }
        G::Cmos(cmos_switch) => {
            const CONTEXT_WIDTH: Option<VectorSize> = Some(SCALAR_VSIZE);
            let i = cmos_switch.instances.get(instance);
            get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.ncontrol_terminal)?;
            get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.pcontrol_terminal)?;
            get_used_signals(ctx, mctx, scope, &mut sensitivity_list, i.input_terminal)?;

            let (proc_builder, mut bb_builder) = ProcessBuilder::new(
                mctx.gl(),
                ProcessKind::Udp,
                ctx.arenas.get_span(*cmos_switch),
            );

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
            (proc_builder, bb_builder, out)
        }
        G::NInput(id) => {
            let NInputGateInstance {
                name: _,
                output_terminal,
                input_terminals,
            } = &*id.instances.get(instance);

            let (proc_builder, mut bb_builder) =
                ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(*id));

            let output_bit_length = net_lvalue_bit_length(ctx, mctx, scope, *output_terminal)?;

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

                use NInputGateType as T;
                match id.gatetype.item {
                    T::And | T::Nand => value = bb_builder.and(mctx.gl(), value, input),
                    T::Or | T::Nor => value = bb_builder.or(mctx.gl(), value, input),
                    T::Xor | T::Xnor => value = bb_builder.xor(mctx.gl(), value, input),
                }
            }

            if matches!(
                id.gatetype.item,
                NInputGateType::Nand | NInputGateType::Nor | NInputGateType::Xnor
            ) {
                value = bb_builder.binary_not(mctx.gl(), value);
            }
            (proc_builder, bb_builder, value)
        }
        G::NOutput(id) => {
            let NOutputGateInstance {
                name: _,
                output_terminals,
                input_terminal,
            } = &*id.instances.get(instance);
            let output_terminal = output_terminals.get(output as usize);

            let (proc_builder, mut bb_builder) =
                ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(*id));

            get_used_signals(ctx, mctx, scope, &mut sensitivity_list, *input_terminal)?;

            let output_bit_length = net_lvalue_bit_length(ctx, mctx, scope, output_terminal)?;
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
            match id.gatetype.item {
                NOutputGateType::Buf => value = bb_builder.z_to_x(mctx.gl(), value),
                NOutputGateType::Not => value = bb_builder.binary_not(mctx.gl(), value),
            }
            (proc_builder, bb_builder, value)
        }
        G::PassSwitch(_) | G::PassEn(_) | G::Pullup(_) | G::Pulldown(_) => {
            unreachable!("only gates with an output process are primitives")
        }
    };

    let entry_tr = proc_builder.entry();
    drive(mctx, &mut bb_builder, 0, value);
    bb_builder.watch_to(mctx.gl(), sensitivity_list.items, entry_tr);
    proc_builder.finalize(mctx.gl());

    Ok(())
}

/// Called with the new output value and the output value that is currently driven.
pub(crate) type GateDelayFn<'a, 'f> = dyn Fn(
        &mut MutLowerContext<'a>,
        &mut BasicBlockBuilder,
        VariableKey,
        VariableKey,
    ) -> Result<VariableKey, ()>
    + 'f;

/// Lowers the terminals of a `tran` / `rtran` switch to bidirectional connections.
///
/// Both terminals must resolve to (slices of) nets of the same width. The terminals are split
/// into pieces wherever either side crosses a signal boundary, e.g. for concatenations.
fn lower_pass_switch_instance<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    fst: AstId<'a, Expr<'a>>,
    snd: AstId<'a, Expr<'a>>,
) -> Result<(), ()> {
    let mut fst_sites = Vec::new();
    let mut snd_sites = Vec::new();
    for (terminal, sites) in [(fst, &mut fst_sites), (snd, &mut snd_sites)] {
        expr_to_src_sites(
            ctx,
            &mctx.gl,
            sites,
            scope,
            &mut mctx.diagnostics,
            terminal,
            SiteTerminal::Probe,
            RequiredSignalKind::Net,
        )?;
    }

    let to_slices = |mctx: &mut MutLowerContext<'a>, terminal, sites: Vec<SourceSite>| {
        sites
            .into_iter()
            .map(|site| match site.content {
                SourceSiteContent::Signal { signal, slice, .. } => Ok((signal, slice)),
                SourceSiteContent::Constant(_)
                | SourceSiteContent::OutOfBounds { .. }
                | SourceSiteContent::Expr { .. } => {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(terminal),
                        "only in-bounds nets can be used as a `tran` terminal",
                    );
                    Err(())
                }
            })
            .collect::<Result<Vec<_>, ()>>()
    };
    let fst_slices = to_slices(mctx, fst, fst_sites)?;
    let snd_slices = to_slices(mctx, snd, snd_sites)?;

    let width = |slices: &[(SignalKey, SignalSlice)]| -> u64 {
        slices.iter().map(|(_, s)| u64::from(s.width().get())).sum()
    };
    if width(&fst_slices) != width(&snd_slices) {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_span(snd),
            "`tran` terminals of different widths are not yet supported",
        );
        return Err(());
    }

    mctx.driver_resolution
        .push_bidirectional_sites(&fst_slices, &snd_slices);

    Ok(())
}

pub(crate) fn lower_delay2<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
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
    mctx: &mut MutLowerContext<'a>,
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
    mctx: &mut MutLowerContext<'a>,
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
        mctx: &mut MutLowerContext<'a>,
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

/// Assigns a value to the output of a [`lower_delay_process`].
pub(crate) type AssignOutputFn<'a, 'f> =
    dyn Fn(&mut MutLowerContext<'a>, &mut BasicBlockBuilder, VariableKey) -> Result<(), ()> + 'f;

/// Builds a process that drives the value of the `input` signal onto an output with `assign`,
/// after the delay that `delay` gives.
pub(crate) fn lower_delay_process<'a>(
    mctx: &mut MutLowerContext<'a>,
    span: TokenRange,
    delay: &GateDelayFn<'a, '_>,
    assign: &AssignOutputFn<'a, '_>,
    input: SignalKey,
    output_bit_length: VectorSize,
) -> Result<(), ()> {
    let (mut delay_proc_builder, mut delay_bb_builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, span);
    let delay_entry_tr = delay_proc_builder.entry();

    let new_value = delay_bb_builder.probe(mctx.gl(), input);

    // The value this process last drove onto the output.
    let mut flags = SignalFlags::EMPTY;
    flags.set(SignalFlags::POKE_T0, new_value.mode().is_two_value());
    let driven = mctx.gl.signals.insert(Signal {
        name: format!("GATE_DELAY_DRIVEN/{}", mctx.gl.signals.len()),
        size: output_bit_length,
        initialize: None,
        flags,
        origin: span,
        mode: new_value.mode(),
    });
    let driven_value = delay_bb_builder.probe(mctx.gl(), driven);

    let delay = delay(mctx, &mut delay_bb_builder, new_value, driven_value)?;

    // Only wait for the time that that is remaining.
    let time = delay_bb_builder.time(mctx.gl());
    let last_update = delay_bb_builder.lupdt(mctx.gl(), input);
    let elapsed = delay_bb_builder.minus(mctx.gl(), time, last_update);
    let elapsed = delay_bb_builder.min(mctx.gl(), elapsed, delay);
    let wait_time = delay_bb_builder.minus(mctx.gl(), delay, elapsed);

    let mut flags = SignalFlags::EMPTY;
    flags.set(SignalFlags::POKE_T0, new_value.mode().is_two_value());
    let scheduled = mctx.gl.signals.insert(Signal {
        name: format!("GATE_DELAY_SCHEDULED/{}", mctx.gl.signals.len()),
        size: output_bit_length,
        initialize: None,
        flags,
        origin: span,
        mode: new_value.mode(),
    });
    delay_bb_builder.drive(mctx.gl(), scheduled, new_value);

    let after_tr = delay_proc_builder.next_temporal_region(mctx.gl());
    delay_bb_builder.variable_wait_to(mctx.gl(), wait_time, after_tr);
    delay_bb_builder.finished_switch_to(mctx.gl(), after_tr.entry());

    // If the input changed, go back to waiting.
    let scheduled = delay_bb_builder.probe(mctx.gl(), scheduled);
    let value = delay_bb_builder.probe(mctx.gl(), input);
    let unchanged = delay_bb_builder.case_equals(mctx.gl(), scheduled, value);
    let mut reschedule_builder;
    (delay_bb_builder, reschedule_builder) = delay_bb_builder.double_branch(mctx.gl(), unchanged);
    reschedule_builder.temporal_jump_to(mctx.gl(), delay_entry_tr);

    assign(mctx, &mut delay_bb_builder, value)?;
    delay_bb_builder.drive(mctx.gl(), driven, value);

    delay_bb_builder.watch_to(mctx.gl(), vec![input], delay_entry_tr);
    let standing = [WatchCondition::entire_signal(&mctx.gl, input)].into();
    delay_proc_builder.set_standing(mctx.gl(), standing);
    delay_proc_builder.finalize(mctx.gl());

    Ok(())
}
