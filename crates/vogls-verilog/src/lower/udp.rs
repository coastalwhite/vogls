use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, Bits, GlobalContext, LogicMode, ProcessBuilder,
    ProcessKind, SCALAR_VSIZE, Signal, SignalFlags, VariableKey,
};
use vogls_utils::OrderedSet;

use crate::ast::expr::Expr;
use crate::ast::statement::NetLValue;
use crate::ast::udp::{
    UdpBody, UdpCombinationalEntry, UdpDeclaration, UdpEdgeIndicator, UdpEdgeSymbol, UdpInitVal,
    UdpInitialStatement, UdpLevelSymbol, UdpNextState, UdpOutputSymbol, UdpPorts,
    UdpSequentialEntry,
};
use crate::ast::{AstId, AstIdRange};
use crate::lower::VType;
use crate::lower::assign::assign_net_lvalue;
use crate::lower::expression::{get_used_signals, lower_expr, truncate_or_extend};

use super::{LowerContext, MutLowerContext, eval_constant_expr};

fn lower_level(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    input: VariableKey,
    level: UdpLevelSymbol,
) -> VariableKey {
    match level {
        UdpLevelSymbol::L0 => {
            builder.case_equals_constant(gl, input, Bits::new_zeroed(SCALAR_VSIZE))
        }
        UdpLevelSymbol::L1 => builder.case_equals_constant(gl, input, Bits::new_ones(SCALAR_VSIZE)),
        UdpLevelSymbol::X => {
            let input = builder.z_to_x(gl, input);
            builder.case_equals_constant(gl, input, Bits::new_unknown(SCALAR_VSIZE))
        }
        UdpLevelSymbol::QuestionMark => builder.constant(gl, Bits::new_ones(SCALAR_VSIZE)),
        UdpLevelSymbol::B => {
            let input = builder.z_to_x(gl, input);
            builder.not_case_equals_constant(gl, input, Bits::new_unknown(SCALAR_VSIZE))
        }
    }
}

pub fn lower_udp<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, UdpDeclaration<'a>>,
    output_terminal: AstId<'a, NetLValue<'a>>,
    inputs: AstIdRange<'a, Expr<'a>>,
) -> Result<(), ()> {
    match id.body {
        UdpBody::Combinational(entries) => {
            lower_combinational_udp(ctx, mctx, scope, id, entries, output_terminal, inputs)
        }
        UdpBody::Sequential(initial, entries) => lower_sequential_udp(
            ctx,
            mctx,
            scope,
            id,
            initial,
            entries,
            output_terminal,
            inputs,
        ),
    }
}

pub fn lower_combinational_udp<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, UdpDeclaration<'a>>,
    entries: AstIdRange<'a, UdpCombinationalEntry<'a>>,
    output_terminal: AstId<'a, NetLValue<'a>>,
    inputs: AstIdRange<'a, Expr<'a>>,
) -> Result<(), ()> {
    let (proc_builder, mut builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(id));

    let mut ins = OrderedSet::new();
    let mut input_vars = Vec::with_capacity(inputs.len());
    for input in inputs.iter() {
        get_used_signals(ctx, mctx, scope, &mut ins, input)?;
    }

    let entry_tr = proc_builder.entry();
    let exit_bb = builder.new_basic_block(mctx.gl());

    for input in inputs.iter() {
        let (input, input_ty) = lower_expr(ctx, mctx, scope, &mut builder, input, None)?;
        let input = truncate_or_extend(mctx.gl(), &mut builder, input, input_ty, SCALAR_VSIZE);
        input_vars.push(input);
    }

    let mut phi_sources = Vec::new();
    for entry in entries.iter() {
        let UdpCombinationalEntry {
            level_input_list,
            output_symbol,
        } = &*entry;

        if level_input_list.len() != inputs.len() {
            mctx.diagnostics.not_yet_implemented(
                ctx.arenas.get_range_span(*level_input_list),
                "invalid amount of values",
            );
            return Err(());
        }

        let mut acc = builder.constant(mctx.gl(), Bits::from(true));
        for (input, level) in input_vars.iter().zip(level_input_list.iter()) {
            let is_level = lower_level(mctx.gl(), &mut builder, *input, *level);
            acc = builder.and(mctx.gl(), acc, is_level);
        }

        let start_bb = builder.key();
        builder = builder.next_terminate_later(mctx.gl());
        let drive_bb = builder.key();

        let output_value = match output_symbol.item {
            UdpOutputSymbol::L0 => Bits::new_zeroed(SCALAR_VSIZE),
            UdpOutputSymbol::L1 => Bits::new_ones(SCALAR_VSIZE),
            UdpOutputSymbol::X => Bits::new_unknown(SCALAR_VSIZE),
        };
        let output_value = builder.constant(mctx.gl(), output_value);
        let value = builder.convert_mode(mctx.gl(), output_value, LogicMode::FourValue);
        phi_sources.push((builder.key(), value));
        builder = builder.next_terminate_later(mctx.gl());

        debug_assert_eq!(mctx.gl.bbs[start_bb].region, mctx.gl.bbs[drive_bb].region);
        debug_assert_eq!(mctx.gl.bbs[drive_bb].region, entry_tr);
        mctx.gl.bbs[start_bb].terminator =
            BasicBlockTerminator::Branch(acc, drive_bb, builder.key());
        mctx.gl.bbs[drive_bb].terminator = BasicBlockTerminator::Jump(exit_bb);
    }

    // No entry matched.
    let output_value = builder.constant(mctx.gl(), Bits::new_unknown(SCALAR_VSIZE));
    let value = builder.convert_mode(mctx.gl(), output_value, LogicMode::FourValue);
    phi_sources.push((builder.key(), value));
    builder.jump_to(mctx.gl(), exit_bb);

    builder.finished_switch_to(mctx.gl(), exit_bb);
    let (output_value, _) = builder.phi(mctx.gl(), phi_sources.into_boxed_slice());
    assign_net_lvalue(
        ctx,
        mctx,
        scope,
        &mut builder,
        output_terminal,
        output_value,
        VType::SCALAR_NET,
    )?;
    builder.watch_to(mctx.gl(), ins.items, proc_builder.entry());

    proc_builder.finalize(mctx.gl());
    Ok(())
}

pub fn lower_sequential_udp<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, UdpDeclaration<'a>>,
    initial: Option<AstId<'a, UdpInitialStatement>>,
    entries: AstIdRange<'a, UdpSequentialEntry<'a>>,
    output_terminal: AstId<'a, NetLValue<'a>>,
    inputs: AstIdRange<'a, Expr<'a>>,
) -> Result<(), ()> {
    let UdpDeclaration {
        attribute_instances: _,
        identifier: _,
        body: _,
        ports,
    } = &*id;

    let (mut proc_builder, mut builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(id));

    let mut ins = OrderedSet::new();
    let mut input_vars = Vec::with_capacity(inputs.len());
    for input in inputs.iter() {
        get_used_signals(ctx, mctx, scope, &mut ins, input)?;
    }

    let mut phi_sources = Vec::new();
    let mut initial_value = None;
    let (output, output_name) = match *ports {
        UdpPorts::PortList(mut port_list, _) => {
            // @TODO: verify that the declarations match.
            assert!(port_list.len() >= 2);

            let ast_output = port_list.pop_front().unwrap();
            let output = *ast_output;
            let output_name = output.0;
            let mut flags = SignalFlags::EMPTY;
            flags.set(SignalFlags::POKE_T0, ctx.logic_mode.is_two_value());
            let output_signal = mctx.gl.signals.insert(vogls_ir::Signal {
                name: ctx.arenas.ident_table[output.0].to_string(),
                size: SCALAR_VSIZE,
                initialize: None,
                flags,
                origin: ctx.arenas.get_span(ast_output),
                mode: ctx.logic_mode,
            });
            (output_signal, output_name)
        }
        UdpPorts::DeclarationPortList(decl_list) => {
            let output = *decl_list.output_decl;
            let output_name = output.port_identifier.item.0;
            let mut flags = SignalFlags::EMPTY;
            flags.set(SignalFlags::POKE_T0, ctx.logic_mode.is_two_value());
            let output_signal = mctx.gl.signals.insert(vogls_ir::Signal {
                name: ctx.arenas.ident_table[output_name].to_string(),
                size: SCALAR_VSIZE,
                initialize: None,
                flags,
                origin: ctx.arenas.get_item_span(output.port_identifier),
                mode: ctx.logic_mode,
            });

            if let Some(constant_expr) = output.constant_expr {
                let mut initial = eval_constant_expr(
                    &mctx.gl,
                    ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    constant_expr,
                    Some(SCALAR_VSIZE),
                )?
                .into_bits();
                initial = initial.truncate(SCALAR_VSIZE);
                if mctx.gl.signals[output_signal].mode.is_two_value() {
                    initial = initial.special_to_zero();
                };
                mctx.gl.signals[output_signal].initialize = Some(initial.clone());
                initial_value = Some(initial);
            }
            (output_signal, output_name)
        }
    };

    let mut before_inputs = vec![None; inputs.len()];
    for entry in entries.iter() {
        let UdpSequentialEntry {
            level_list,
            edge_list,
            current_state: _,
            next_state: _,
        } = &*entry;
        if level_list.len() < inputs.len() && edge_list.is_some() {
            let i = level_list.len();
            if before_inputs[i].is_none() {
                let mut flags = SignalFlags::EMPTY;
                flags.set(SignalFlags::POKE_T0, ctx.logic_mode.is_two_value());
                let signal = Signal {
                    name: String::from("__UDP_INPUT"),
                    size: SCALAR_VSIZE,
                    initialize: None,
                    mode: ctx.logic_mode,
                    flags,
                    origin: TokenRange::default(),
                };
                let signal = mctx.gl.signals.insert(signal);
                before_inputs[i] = Some(signal);
            }
        }
    }

    if let Some(initial) = initial {
        let UdpInitialStatement {
            output_port_ident,
            init_val,
        } = &*initial;
        if output_port_ident.item.0 != output_name {
            mctx.diagnostics.not_yet_implemented(
                ctx.arenas.get_item_span(*output_port_ident),
                "cannot set initial for non-output port",
            );
            return Err(());
        }
        let mode = mctx.gl.signals[output].mode;
        let initial = match init_val.item {
            UdpInitVal::L0 => Bits::new_zeroed(SCALAR_VSIZE),
            UdpInitVal::L1 => Bits::new_ones(SCALAR_VSIZE),
            UdpInitVal::X if mode.is_four_value() => Bits::new_unknown(SCALAR_VSIZE),
            UdpInitVal::X => Bits::new_zeroed(SCALAR_VSIZE),
        };
        mctx.gl.signals[output].initialize = Some(initial.clone());
        initial_value = Some(initial);
    }

    let (entry_tr, exit_bb, is_init) = match initial_value {
        None => {
            let entry_tr = proc_builder.next_temporal_region(mctx.gl());
            builder.watch_to(mctx.gl(), ins.items.clone(), entry_tr);
            builder.finished_switch_to(mctx.gl(), entry_tr.entry());
            let exit_bb = builder.new_basic_block(mctx.gl());
            (entry_tr, exit_bb, None)
        }
        Some(initial_value) => {
            let entry_tr = proc_builder.entry();
            let exit_bb = builder.new_basic_block(mctx.gl());

            let init_flag = mctx.gl.signals.insert(Signal {
                name: String::from("__UDP_INIT"),
                size: SCALAR_VSIZE,
                initialize: Some(Bits::new_ones(SCALAR_VSIZE)),
                mode: LogicMode::TwoValue,
                flags: SignalFlags::EMPTY,
                origin: TokenRange::default(),
            });
            let is_init = builder.probe(mctx.gl(), init_flag);
            let init_bb = builder.new_basic_block(mctx.gl());
            let eval_bb = builder.new_basic_block(mctx.gl());
            builder.finalize(
                mctx.gl(),
                BasicBlockTerminator::Branch(is_init, init_bb, eval_bb),
            );

            builder.finished_switch_to(mctx.gl(), init_bb);
            let cleared = builder.constant(mctx.gl(), Bits::new_zeroed(SCALAR_VSIZE));
            builder.drive(mctx.gl(), init_flag, cleared);
            let initial_value = builder.constant(mctx.gl(), initial_value);
            let value = builder.convert_mode(mctx.gl(), initial_value, LogicMode::FourValue);
            phi_sources.push((builder.key(), value));
            builder.jump_to(mctx.gl(), exit_bb);

            builder.finished_switch_to(mctx.gl(), eval_bb);
            (entry_tr, exit_bb, Some(is_init))
        }
    };

    for input in inputs.iter() {
        let (input, input_ty) = lower_expr(ctx, mctx, scope, &mut builder, input, None)?;
        let input = truncate_or_extend(mctx.gl(), &mut builder, input, input_ty, SCALAR_VSIZE);
        input_vars.push(input);
    }

    let eval_bb = builder.key();
    let watch_bb = builder.new_basic_block(mctx.gl());
    builder.switch_to(mctx.gl(), watch_bb);
    for (input, before_input) in input_vars.iter().zip(&before_inputs) {
        if let Some(before_input) = before_input {
            builder.drive(mctx.gl(), *before_input, *input);
        }
    }
    builder.watch_to(mctx.gl(), ins.items.clone(), entry_tr);
    builder.finished_switch_to(mctx.gl(), eval_bb);

    // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 114 - 8.7
    // > When the input changes, the edge-sensitive cases are processed first, followed by
    //   level-sensitive cases. Thus, when level-sensitive and edge-sensitive cases specify
    //   different output values, the result is specified by the level-sensitive case.
    for allow_edge_list in [false, true] {
        for entry in entries.iter() {
            let UdpSequentialEntry {
                level_list,
                edge_list,
                current_state,
                next_state,
            } = &*entry;
            if edge_list.is_some() != allow_edge_list {
                continue;
            }

            if level_list.len() + edge_list.map_or(0, |(_, after_list)| 1 + after_list.len())
                != inputs.len()
            {
                let mut tr = ctx.arenas.get_range_span(*level_list);
                if let Some((edge_indicator, after_list)) = edge_list {
                    tr |= ctx.arenas.get_span(*edge_indicator);
                    if !after_list.is_empty() {
                        tr |= ctx.arenas.get_range_span(*after_list);
                    }
                }
                mctx.diagnostics
                    .not_yet_implemented(tr, "invalid amount of values");
                return Err(());
            }

            let mut acc = builder.constant(mctx.gl(), Bits::from(true));
            for (i, level) in level_list.iter().enumerate() {
                let is_level = lower_level(mctx.gl(), &mut builder, input_vars[i], *level);
                acc = builder.and(mctx.gl(), acc, is_level);
            }
            if let Some((edge_indicator, after_level_list)) = edge_list {
                let prb_before = builder.probe(mctx.gl(), before_inputs[level_list.len()].unwrap());
                let prb_after = input_vars[level_list.len()];
                match &**edge_indicator {
                    UdpEdgeIndicator::Levels(before, after) => {
                        let is_before =
                            lower_level(mctx.gl(), &mut builder, prb_before, before.item);
                        acc = builder.and(mctx.gl(), acc, is_before);
                        let is_after = lower_level(mctx.gl(), &mut builder, prb_after, after.item);
                        acc = builder.and(mctx.gl(), acc, is_after);
                    }
                    UdpEdgeIndicator::Edge(edge) => {
                        let condition = match edge.item {
                            UdpEdgeSymbol::R => builder.andnot(mctx.gl(), prb_after, prb_before),
                            UdpEdgeSymbol::F => builder.andnot(mctx.gl(), prb_before, prb_after),
                            UdpEdgeSymbol::P => builder.posedge(mctx.gl(), prb_before, prb_after),
                            UdpEdgeSymbol::N => builder.negedge(mctx.gl(), prb_before, prb_after),
                            UdpEdgeSymbol::Star => {
                                builder.not_case_equals(mctx.gl(), prb_before, prb_after)
                            }
                        };
                        acc = builder.and(mctx.gl(), acc, condition);
                    }
                }
                for (i, level) in after_level_list.iter().enumerate() {
                    let is_level = lower_level(
                        mctx.gl(),
                        &mut builder,
                        input_vars[level_list.len() + 1 + i],
                        *level,
                    );
                    acc = builder.and(mctx.gl(), acc, is_level);
                }
            }
            let prb = builder.probe(mctx.gl(), output);
            let is_level = lower_level(mctx.gl(), &mut builder, prb, current_state.item);
            acc = builder.and(mctx.gl(), acc, is_level);

            let start_bb = builder.key();
            builder = builder.next_terminate_later(mctx.gl());

            let drive_bb = builder.key();

            // A `-` next state leaves the output untouched and skips the assignment.
            let target_bb = match next_state.item {
                UdpNextState::Output(next_state) => {
                    let output_value = match next_state {
                        UdpOutputSymbol::L0 => Bits::new_zeroed(SCALAR_VSIZE),
                        UdpOutputSymbol::L1 => Bits::new_ones(SCALAR_VSIZE),
                        UdpOutputSymbol::X => Bits::new_unknown(SCALAR_VSIZE),
                    };
                    let output_value = builder.constant(mctx.gl(), output_value);
                    builder.drive(mctx.gl(), output, output_value);
                    let value = builder.convert_mode(mctx.gl(), output_value, LogicMode::FourValue);
                    phi_sources.push((builder.key(), value));
                    exit_bb
                }
                _ => watch_bb,
            };

            builder = builder.next_terminate_later(mctx.gl());

            mctx.gl.bbs[start_bb].terminator =
                BasicBlockTerminator::Branch(acc, drive_bb, builder.key());
            debug_assert_eq!(mctx.gl.bbs[drive_bb].region, entry_tr);
            mctx.gl.bbs[drive_bb].terminator = BasicBlockTerminator::Jump(target_bb);
        }
    }

    // No entry matched.
    let output_value = builder.constant(mctx.gl(), Bits::new_unknown(SCALAR_VSIZE));
    builder.drive(mctx.gl(), output, output_value);
    let value = builder.convert_mode(mctx.gl(), output_value, LogicMode::FourValue);
    phi_sources.push((builder.key(), value));
    builder.jump_to(mctx.gl(), exit_bb);

    let after_assign = match is_init {
        None => BasicBlockTerminator::Jump(watch_bb),
        Some(is_init) => {
            // At `t=0` the inputs have not been evaluated, so there is nothing to
            // remember for edge detection yet.
            let init_watch_bb = builder.new_basic_block(mctx.gl());
            builder.finished_switch_to(mctx.gl(), init_watch_bb);
            builder.watch_to(mctx.gl(), ins.items.clone(), entry_tr);
            BasicBlockTerminator::Branch(is_init, init_watch_bb, watch_bb)
        }
    };
    let after_assign = Some(after_assign);

    builder.finished_switch_to(mctx.gl(), exit_bb);
    let (output_value, _) = builder.phi(mctx.gl(), phi_sources.into_boxed_slice());
    assign_net_lvalue(
        ctx,
        mctx,
        scope,
        &mut builder,
        output_terminal,
        output_value,
        VType::SCALAR_NET,
    )?;
    match after_assign {
        None => builder.watch_to(mctx.gl(), ins.items, proc_builder.entry()),
        Some(after_assign) => builder.finalize(mctx.gl(), after_assign),
    }

    proc_builder.finalize(mctx.gl());
    Ok(())
}
