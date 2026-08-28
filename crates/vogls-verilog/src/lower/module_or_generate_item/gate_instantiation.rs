use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::bits::arithmetic::FvLogicValue;
use vogls_ir::{Bits, ProcessBuilder, ProcessKind, SCALAR_VSIZE, VectorSize};
use vogls_utils::OrderedSet;

use crate::ast::AstId;
use crate::ast::module::{
    CmosSwitchType, EnableGateType, GateInstantiation, MosSwitchType, NInputGateInstance,
    NInputGateType, NOutputGateInstance, NOutputGateType,
};
use crate::lower::assign::{assign_net_lvalue, net_lvalue_bit_length};
use crate::lower::expression::{get_used_signals, lower_expr, truncate_or_extend};
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
            if enable_switch.delay.is_some() {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(*enable_switch),
                    "enable gates with delay are not yet supported",
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

                assign_net_lvalue(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    i.output_terminal,
                    out,
                    VType::SCALAR_NET,
                )?;

                bb_builder.watch_to(mctx.gl(), sensitivity_list.items, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
        G::Mos(mos_switch) => {
            if mos_switch.delay.is_some() {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(*mos_switch),
                    "mos switches with delay are not yet supported",
                );
                return Err(());
            }

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

                // @TODO: We resolve L/H to to 0/1 here.
                //
                // In the LRM, it can be collapsed to either 0/1 or z. I feel like this should be a
                // compile toggle.
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

                assign_net_lvalue(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    i.output_terminal,
                    out,
                    VType::SCALAR_NET,
                )?;

                bb_builder.watch_to(mctx.gl(), sensitivity_list.items, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
        G::Cmos(cmos_switch) => {
            if cmos_switch.delay.is_some() {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(*cmos_switch),
                    "cmos switches with delay are not yet supported",
                );
                return Err(());
            }

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

                // @TODO: We resolve L/H to to 0/1 here.
                //
                // In the LRM, it can be collapsed to either 0/1 or z. I feel like this should be a
                // compile toggle.
                let data = bb_builder.truncate(mctx.gl(), data, SCALAR_VSIZE);
                let ncontrol = bb_builder.truncate(mctx.gl(), ncontrol, SCALAR_VSIZE);
                let pcontrol = bb_builder.truncate(mctx.gl(), pcontrol, SCALAR_VSIZE);

                let z = bb_builder.constant(mctx.gl(), Bits::new_high_impedance(SCALAR_VSIZE));
                let out = match cmos_switch.gatetype.item {
                    CmosSwitchType::Cmos | CmosSwitchType::Rcmos => {
                        let nor_control = bb_builder.nor(mctx.gl(), ncontrol, pcontrol);
                        let nor_control = bb_builder.case_equals_constant(
                            mctx.gl(),
                            nor_control,
                            FvLogicValue::L1.into(),
                        );

                        // out = z    (if both OFF)
                        // out = data (otherwise)
                        bb_builder.select_merge(mctx.gl(), nor_control, z, data)
                    }
                };

                assign_net_lvalue(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    i.output_terminal,
                    out,
                    VType::SCALAR_NET,
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
            use NInputGateType as G;
            for instance in id.instances.iter() {
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

                    match id.gatetype.item {
                        G::And | G::Nand => value = bb_builder.and(mctx.gl(), value, input),
                        G::Or | G::Nor => value = bb_builder.or(mctx.gl(), value, input),
                        G::Xor | G::Xnor => value = bb_builder.xor(mctx.gl(), value, input),
                    }
                }

                if matches!(id.gatetype.item, G::Nand | G::Nor | G::Xnor) {
                    value = bb_builder.binary_not(mctx.gl(), value);
                }

                assign_net_lvalue(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    *output_terminal,
                    value,
                    VType::UnsignedNet(output_bit_length),
                )?;

                let sensitivity_list = sensitivity_list.items;
                bb_builder.watch_to(mctx.gl(), sensitivity_list, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
        G::NOutput(id) => {
            use NOutputGateType as G;
            for instance in id.instances.iter() {
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
                    match id.gatetype.item {
                        G::Buf => value = bb_builder.z_to_x(mctx.gl(), value),
                        G::Not => value = bb_builder.binary_not(mctx.gl(), value),
                    }

                    assign_net_lvalue(
                        ctx,
                        mctx,
                        scope,
                        &mut bb_builder,
                        output_terminal,
                        value,
                        VType::UnsignedNet(output_bit_length),
                    )?;
                }

                bb_builder.watch_to(mctx.gl(), sensitivity_list, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
    }

    Ok(())
}
