use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, Bits, GlobalContext, ProcessBuilder, ProcessKind,
    SCALAR_VSIZE, Signal, SignalFlags, TemporalRegionKey, VariableKey,
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

use super::{LowerContext, MutLowerContext};

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
            let input = builder.x_to_z(gl, input);
            builder.case_equals_constant(gl, input, Bits::new_unknown(SCALAR_VSIZE))
        }
        UdpLevelSymbol::QuestionMark => builder.constant(gl, Bits::new_ones(SCALAR_VSIZE)),
        UdpLevelSymbol::B => {
            let input = builder.x_to_z(gl, input);
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
    let UdpDeclaration {
        attribute_instances: _,
        identifier: _,
        ports,
        body,
    } = &*id;

    let (process, mut builder) =
        ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(id));
    let entry_bb = builder.key();

    let mut ins = OrderedSet::new();
    for input in inputs.iter() {
        get_used_signals(ctx, mctx, scope, &mut ins, input)?;
    }

    let mut before_inputs = vec![None; inputs.len()];
    if let UdpBody::Sequential(_, entries) = body {
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
                    let signal = Signal {
                        name: String::from("__UDP_INPUT"),
                        size: SCALAR_VSIZE,
                        initialize: None,
                        mode: ctx.logic_mode,
                        flags: SignalFlags::EMPTY,
                        origin: TokenRange::default(),
                    };
                    let signal = mctx.gl.signals.insert(signal);
                    before_inputs[i] = Some(signal);
                }
            }
        }
    }

    let mut input_vars = Vec::with_capacity(inputs.len());
    for input in inputs.iter() {
        let (input, input_ty) = lower_expr(ctx, mctx, scope, &mut builder, input, None)?;
        let input = truncate_or_extend(mctx.gl(), &mut builder, input, input_ty, SCALAR_VSIZE);
        input_vars.push(input);
    }

    builder = builder.next_terminate_later(mctx.gl());
    let watch_bb = builder.key();
    for (input, before_input) in input_vars.iter().zip(&before_inputs) {
        if let Some(before_input) = before_input {
            builder.drive(mctx.gl(), *before_input, *input);
        }
    }
    builder.finalize_and_switch_to(
        mctx.gl(),
        BasicBlockTerminator::Watch(TemporalRegionKey::from_entry(entry_bb), ins.items),
        entry_bb,
    );

    match body {
        UdpBody::Combinational(entries) => {
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
                assign_net_lvalue(
                    ctx,
                    mctx,
                    scope,
                    &mut builder,
                    output_terminal,
                    output_value,
                    VType::SCALAR_NET,
                )?;

                let current_entry_bb = builder.key();
                builder = builder.next_terminate_later(mctx.gl());

                mctx.gl.bbs[start_bb].terminator =
                    BasicBlockTerminator::Branch(acc, drive_bb, builder.key());
                mctx.gl.bbs[current_entry_bb].terminator = BasicBlockTerminator::Jump(watch_bb);
            }

            let output_value = builder.constant(mctx.gl(), Bits::new_unknown(SCALAR_VSIZE));
            assign_net_lvalue(
                ctx,
                mctx,
                scope,
                &mut builder,
                output_terminal,
                output_value,
                VType::SCALAR_NET,
            )?;
        }
        UdpBody::Sequential(initial, entries) => {
            let (output, output_name) = match *ports {
                UdpPorts::PortList(mut port_list, _) => {
                    // @TODO: verify that the declarations match.
                    assert!(port_list.len() >= 2);

                    let ast_output = port_list.pop_front().unwrap();
                    let output = *ast_output;
                    let output_name = output.0;
                    let output_signal = mctx.gl.signals.insert(vogls_ir::Signal {
                        name: ctx.arenas.ident_table[output.0].to_string(),
                        size: SCALAR_VSIZE,
                        initialize: None,
                        flags: SignalFlags::EMPTY,
                        origin: ctx.arenas.get_span(ast_output),
                        mode: ctx.logic_mode,
                    });
                    (output_signal, output_name)
                }
                UdpPorts::DeclarationPortList(decl_list) => {
                    let output = *decl_list.output_decl;
                    if let Some(constant_expr) = output.constant_expr {
                        mctx.diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(constant_expr),
                            "constant expr here",
                        );
                    }

                    let output_name = output.port_identifier.item.0;
                    let output_signal = mctx.gl.signals.insert(vogls_ir::Signal {
                        name: ctx.arenas.ident_table[output_name].to_string(),
                        size: SCALAR_VSIZE,
                        initialize: None,
                        flags: SignalFlags::EMPTY,
                        origin: ctx.arenas.get_item_span(output.port_identifier),
                        mode: ctx.logic_mode,
                    });
                    (output_signal, output_name)
                }
            };

            if let Some(initial) = initial {
                let UdpInitialStatement {
                    output_port_ident,
                    init_val,
                } = &**initial;
                if output_port_ident.item.0 != output_name {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_item_span(*output_port_ident),
                        "cannot set initial for non-output port",
                    );
                    return Err(());
                }
                mctx.gl.signals[output].initialize = match init_val.item {
                    UdpInitVal::L0 => Some(Bits::new_zeroed(SCALAR_VSIZE)),
                    UdpInitVal::L1 => Some(Bits::new_ones(SCALAR_VSIZE)),
                    UdpInitVal::X => None,
                };
            }

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

                    if level_list.len()
                        + edge_list.map_or(0, |(_, after_list)| 1 + after_list.len())
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
                        let prb_before =
                            builder.probe(mctx.gl(), before_inputs[level_list.len()].unwrap());
                        let prb_after = input_vars[level_list.len()];
                        match &**edge_indicator {
                            UdpEdgeIndicator::Levels(before, after) => {
                                let is_before =
                                    lower_level(mctx.gl(), &mut builder, prb_before, before.item);
                                acc = builder.and(mctx.gl(), acc, is_before);
                                let is_after =
                                    lower_level(mctx.gl(), &mut builder, prb_after, after.item);
                                acc = builder.and(mctx.gl(), acc, is_after);
                            }
                            UdpEdgeIndicator::Edge(edge) => {
                                let condition = match edge.item {
                                    UdpEdgeSymbol::R => {
                                        builder.andnot(mctx.gl(), prb_after, prb_before)
                                    }
                                    UdpEdgeSymbol::F => {
                                        builder.andnot(mctx.gl(), prb_before, prb_after)
                                    }
                                    UdpEdgeSymbol::P => {
                                        builder.posedge(mctx.gl(), prb_before, prb_after)
                                    }
                                    UdpEdgeSymbol::N => {
                                        builder.negedge(mctx.gl(), prb_before, prb_after)
                                    }
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

                    if let UdpNextState::Output(next_state) = next_state.item {
                        let output_value = match next_state {
                            UdpOutputSymbol::L0 => Bits::new_zeroed(SCALAR_VSIZE),
                            UdpOutputSymbol::L1 => Bits::new_ones(SCALAR_VSIZE),
                            UdpOutputSymbol::X => Bits::new_unknown(SCALAR_VSIZE),
                        };
                        let output_value = builder.constant(mctx.gl(), output_value);
                        builder.drive(mctx.gl(), output, output_value);
                        assign_net_lvalue(
                            ctx,
                            mctx,
                            scope,
                            &mut builder,
                            output_terminal,
                            output_value,
                            VType::SCALAR_NET,
                        )?;
                    }

                    let current_entry_bb = builder.key();
                    builder = builder.next_terminate_later(mctx.gl());

                    mctx.gl.bbs[start_bb].terminator =
                        BasicBlockTerminator::Branch(acc, drive_bb, builder.key());
                    mctx.gl.bbs[current_entry_bb].terminator = BasicBlockTerminator::Jump(watch_bb);
                }
            }

            let output_value = builder.constant(mctx.gl(), Bits::new_unknown(SCALAR_VSIZE));
            builder.drive(mctx.gl(), output, output_value);
            assign_net_lvalue(
                ctx,
                mctx,
                scope,
                &mut builder,
                output_terminal,
                output_value,
                VType::SCALAR_NET,
            )?;
        }
    }
    builder.jump_to(mctx.gl(), watch_bb);
    process.finalize(mctx.gl());
    Ok(())
}
