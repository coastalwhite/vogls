mod levels;
pub mod plan;

use vogls_frontend::ident_table::IdentId;
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::bits::arithmetic::FvLogicValue;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, BasicBlockKey, BasicBlockTerminator, Bits, GlobalContext, LogicMode,
    ProcessBuilder, ProcessKind, SCALAR_VSIZE, Signal, SignalFlags, SignalKey, VariableKey,
    WatchCondition, WatchEdge,
};
use vogls_utils::{BitsetU8, OrderedSet};

use crate::ast::expr::Expr;
use crate::ast::udp::{
    UdpBody, UdpCombinationalEntry, UdpDeclaration, UdpInitVal, UdpInitialStatement,
    UdpLevelSymbol, UdpOutputSymbol, UdpPorts, UdpSequentialEntry,
};
use crate::ast::{AstId, AstIdRange};
use crate::lower::drivers::DriveOutputFn;
use crate::lower::expression::{get_used_signals, lower_expr, truncate_or_extend};
use crate::lower::fuse::{
    RequiredSignalKind, SiteTerminal, SourceSite, SourceSiteContent, expr_to_src_sites,
};

use self::levels::{Level, Transition};
use self::plan::{Body, Table, Wake};
use super::{Diagnostics, LowerContext, MutLowerContext, eval_constant_expr};

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

/// Is `value`, which is never `z`, one of `values` (by the discriminant of their [`Level`])?
fn lower_values(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    value: VariableKey,
    values: BitsetU8,
) -> VariableKey {
    let bits = |i: usize| Bits::from(Level::ALL[i].into_fv_logic_value());
    match values.num_set() {
        0 => builder.constant(gl, Bits::from(false)),
        1 => builder.case_equals_constant(gl, value, bits(values.first().unwrap())),
        2 => {
            let missing = (0..Level::ALL.len()).find(|&i| !values.get(i)).unwrap();
            builder.not_case_equals_constant(gl, value, bits(missing))
        }
        _ => builder.constant(gl, Bits::from(true)),
    }
}

/// Lowers an instance of `id` with `inputs` to a process.
///
/// The process of a combinational UDP drives its output with `drive`. The process of a sequential
/// UDP writes its `state` (see [`sequential_udp_state`]), which drives the output in the drive
/// graph.
pub fn lower_udp<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    id: AstId<'a, UdpDeclaration<'a>>,
    inputs: AstIdRange<'a, Expr<'a>>,
    state: Option<SignalKey>,
    drive: &DriveOutputFn<'a, '_>,
) -> Result<(), ()> {
    match id.body {
        UdpBody::Combinational(entries) => {
            lower_combinational_udp(ctx, mctx, scope, id, entries, inputs, drive)
        }
        UdpBody::Sequential(_, entries) => {
            let state = state.expect("a sequential UDP has a state");
            lower_sequential_udp(
                ctx,
                mctx,
                scope,
                id.identifier.item.0,
                entries,
                inputs,
                state,
            )
        }
    }
}

pub fn lower_combinational_udp<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    id: AstId<'a, UdpDeclaration<'a>>,
    entries: AstIdRange<'a, UdpCombinationalEntry<'a>>,
    inputs: AstIdRange<'a, Expr<'a>>,
    drive: &DriveOutputFn<'a, '_>,
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
    drive(mctx, &mut builder, 0, output_value);
    builder.watch_to(mctx.gl(), ins.items, proc_builder.entry());

    proc_builder.finalize(mctx.gl());
    Ok(())
}

/// Creates the state of an instance of the UDP `id`, if it is sequential: the variable with the
/// initial value of the output that the process of the instance writes. It drives the output of
/// the instance in the drive graph.
pub fn sequential_udp_state<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    id: AstId<'a, UdpDeclaration<'a>>,
) -> Result<Option<SignalKey>, ()> {
    let UdpBody::Sequential(initial, _) = id.body else {
        return Ok(None);
    };
    let UdpDeclaration {
        attribute_instances: _,
        identifier: _,
        body: _,
        ports,
    } = &*id;

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
                initial_value = Some(initial);
            }
            (output_signal, output_name)
        }
    };

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
        initial_value = Some(initial);
    }

    mctx.gl.signals[output].initialize = initial_value;
    Ok(Some(output))
}

/// Lowers an instance of a sequential UDP to a process that writes its state `output`.
///
/// The table is not evaluated at `t=0`: the state starts with the initial value of the output and
/// the process waits for its inputs to change.
///
/// The instance is planned first (see [`plan`]): the process does not watch the inputs that
/// are tied to a constant or that cannot change the state, and does not keep their previous value.
/// If a single input is left, the process only wakes for the transitions of it that can change
/// the state, and the next state is computed directly when it is a simple function.
fn lower_sequential_udp<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    name: IdentId,
    entries: AstIdRange<'a, UdpSequentialEntry<'a>>,
    inputs: AstIdRange<'a, Expr<'a>>,
    output: SignalKey,
) -> Result<(), ()> {
    let origin = mctx.gl.signals[output].origin;

    for entry in entries.iter() {
        let UdpSequentialEntry {
            level_list,
            edge_list,
            current_state: _,
            next_state: _,
        } = &*entry;
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
    }

    let (mut proc_builder, mut builder) = ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, origin);

    let mut input_signals = Vec::with_capacity(inputs.len());
    let mut constants = Vec::with_capacity(inputs.len());
    for input in inputs.iter() {
        let mut signals = OrderedSet::new();
        get_used_signals(ctx, mctx, scope, &mut signals, input)?;
        constants.push(if signals.is_empty() {
            constant_input(ctx, mctx, scope, input)
        } else {
            None
        });
        input_signals.push(signals);
    }

    let entry_tr = proc_builder.entry();

    let mut input_vars = Vec::with_capacity(inputs.len());
    for input in inputs.iter() {
        let (input, input_ty) = lower_expr(ctx, mctx, scope, &mut builder, input, None)?;
        let input = truncate_or_extend(mctx.gl(), &mut builder, input, input_ty, SCALAR_VSIZE);
        // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2005) p. 105
        // > The z values passed to UDP inputs shall be treated the same as x values.
        let input = builder.z_to_x(mctx.gl(), input);
        input_vars.push(input);
    }

    // The bit of a signal that each input is, if it is one, as a scalar net or a bit-select of a
    // vector is. The inputs are already lowered, so the diagnostics of finding their sites are not
    // of interest.
    let input_bits: Vec<Option<(SignalKey, u32)>> = inputs
        .iter()
        .map(|input| {
            let mut sites = Vec::new();
            expr_to_src_sites(
                ctx,
                &mctx.gl,
                &mut sites,
                scope,
                &mut Diagnostics::default(),
                input,
                SiteTerminal::Probe,
                RequiredSignalKind::None,
            )
            .ok()?;
            let [
                SourceSite {
                    content: SourceSiteContent::Signal { signal, slice, .. },
                    ..
                },
            ] = sites[..]
            else {
                return None;
            };
            (slice.width() == SCALAR_VSIZE).then_some((signal, slice.lsb()))
        })
        .collect();

    // An input can only be `x` if it is four-valued. That is up to the input and not the design:
    // a two-valued design can still give an `x` to an input, as with a `1'bx` in its expression.
    // An input that is a bit of a signal has the mode of that signal, even when reading the bit
    // gives a four-valued value, as a select of a two-valued vector does.
    let input_modes: Vec<LogicMode> = (0..inputs.len())
        .map(|i| match input_bits[i] {
            Some((signal, _)) => mctx.gl.signals[signal].mode,
            None => input_vars[i].mode(),
        })
        .collect();

    let plan = plan::compute_or_get_cache_plan(
        &mut mctx.udp_plans,
        name,
        || Table::new(entries, inputs.len()).expect("the entries are checked"),
        &constants,
        |i| input_modes[i],
        mctx.gl.signals[output].mode,
    );
    let table = match &plan {
        Ok(plan) => &plan.table,
        Err(table) => table,
    };

    // The single sensitive input can only be watched for some edges if it is one bit of a signal.
    let single = plan
        .as_ref()
        .ok()
        .and_then(|plan| plan.single)
        .and_then(|single| {
            let (signal, offset) = input_bits[single.input]?;
            Some((single, signal, offset))
        });

    // The body of the single sensitive input, if it is a simple function.
    let body = single.and_then(|(single, signal, _)| {
        let body = single.body?;

        // The process runs once for all the changes since it last ran. A copy of a value that
        // reads the watched signal sees the value after the last change, which need not be a
        // waking transition when only some edges wake: on `0 -> 1 -> 0` with a watch on the
        // positive edge, it would copy `0` where the edge gives `1`.
        if let Body::Copy(source) = body
            && single.wake != Wake::Any
            && input_signals[source].items.contains(&signal)
        {
            return None;
        }

        Some((single, body))
    });

    let watch_bb = builder.new_basic_block(mctx.gl());
    let mut before_inputs = vec![None; inputs.len()];
    match body {
        Some((_, Body::Constant(value))) => {
            let value = builder.constant_scalar(mctx.gl(), value.into_fv_logic_value());
            builder.drive(mctx.gl(), output, value);
            builder.jump_to(mctx.gl(), watch_bb);
        }
        Some((_, Body::Copy(source))) => {
            builder.drive(mctx.gl(), output, input_vars[source]);
            builder.jump_to(mctx.gl(), watch_bb);
        }
        Some((single, Body::CopyOrMerge(source))) => {
            let input = single.input;
            let before_input = new_before_input(mctx, input_modes[input]);
            before_inputs[input] = Some(before_input);
            // It is driven with the input after `z_to_x`, and starts as `x`: only an input that can
            // be `x` needs its previous value here.
            let before = builder.probe(mctx.gl(), before_input);
            let after = input_vars[input];

            // The process wakes on every change to keep the previous value, also those that leave
            // the state as it is.
            let is_waking = match single.wake {
                Wake::Posedge => Some(builder.posedge(mctx.gl(), before, after)),
                Wake::Negedge => Some(builder.negedge(mctx.gl(), before, after)),
                Wake::Any => None,
            };
            if let Some(is_waking) = is_waking {
                let start_bb = builder.key();
                builder = builder.next_terminate_later(mctx.gl());
                mctx.gl.bbs[start_bb].terminator =
                    BasicBlockTerminator::Branch(is_waking, builder.key(), watch_bb);
            }

            // The value of `source` if the transition is between `0` and `1`, otherwise the state
            // if it equals that value, and `x` if it does not.
            let x = Bits::new_unknown(SCALAR_VSIZE);
            let before_known = builder.not_case_equals_constant(mctx.gl(), before, x.clone());
            let after_known = builder.not_case_equals_constant(mctx.gl(), after, x.clone());
            let exact = builder.and(mctx.gl(), before_known, after_known);
            let state = builder.probe(mctx.gl(), output);
            let value = input_vars[source];
            let same = builder.case_equals(mctx.gl(), value, state);
            let keep = builder.or(mctx.gl(), exact, same);
            let x = builder.constant(mctx.gl(), x);
            let next = builder.select(mctx.gl(), keep, value, x);
            builder.drive(mctx.gl(), output, next);
            builder.jump_to(mctx.gl(), watch_bb);
        }
        None => {
            // Edge entries need the previous value of their input.
            for i in table.edge_inputs() {
                before_inputs[i].get_or_insert_with(|| new_before_input(mctx, input_modes[i]));
            }
            lower_rows(
                mctx,
                &mut builder,
                table,
                &constants,
                &input_vars,
                &before_inputs,
                output,
                watch_bb,
            );
        }
    }

    // The process wakes on the changes of the sensitive inputs. A single one that is a scalar
    // signal is only watched for the transitions that can change the state, unless it keeps its
    // previous value: that needs every change.
    let conditions = match single {
        Some((single, signal, offset)) => {
            let edge = match single.wake {
                _ if before_inputs[single.input].is_some() => WatchEdge::Any {
                    bit_length: SCALAR_VSIZE,
                },
                Wake::Posedge => WatchEdge::Posedge,
                Wake::Negedge => WatchEdge::Negedge,
                Wake::Any => WatchEdge::Any {
                    bit_length: SCALAR_VSIZE,
                },
            };
            vec![WatchCondition {
                signal,
                edge,
                offset,
            }]
        }
        None => {
            // The table is evaluated on every wake, and a wake without a change of an input can
            // give a state that no change gives: no edge matches, and without a level that matches
            // the state becomes `x`. An input that is a bit of a signal therefore only watches that
            // bit, and not the rest of the signal, as other inputs or other instances can be bits
            // of the same vector.
            let mut conditions = Vec::new();
            for (i, signals) in input_signals.iter().enumerate() {
                // Without an analysis, every input that is not tied to a constant can change the
                // state.
                let sensitive = match &plan {
                    Ok(plan) => plan.sensitivity.get(i),
                    Err(_) => constants[i].is_none(),
                };
                if !sensitive {
                    continue;
                }
                match input_bits[i] {
                    Some((signal, offset)) => conditions.push(WatchCondition {
                        signal,
                        edge: WatchEdge::Any {
                            bit_length: SCALAR_VSIZE,
                        },
                        offset,
                    }),
                    None => conditions.extend(
                        signals
                            .items
                            .iter()
                            .map(|&signal| WatchCondition::entire_signal(&mctx.gl, signal)),
                    ),
                }
            }
            WatchCondition::canonicalize(&mut conditions);
            conditions
        }
    };

    builder.finished_switch_to(mctx.gl(), watch_bb);
    for (input, before_input) in input_vars.iter().zip(&before_inputs) {
        if let Some(before_input) = before_input {
            builder.drive(mctx.gl(), *before_input, *input);
        }
    }
    builder.watch_conditions_to(mctx.gl(), conditions.clone(), entry_tr);

    // A primitive is sensitive to its inputs from the start: the state must see every change at
    // `t=0`, also those made before the process would first get to run and arm a watch, which
    // depends on the order of the processes. A standing watch is armed right at the start.
    //
    // If a condition is poked at `t=0`, the process is scheduled instead (see `Process::standing`),
    // and its first run must only arm the watch: evaluating the table on the poked values would
    // overwrite the initial state. A watch for an edge is never poked.
    if conditions.iter().any(|c| c.is_poked_t0(&mctx.gl)) {
        let arm_tr = proc_builder.next_temporal_region(mctx.gl());
        builder.finished_switch_to(mctx.gl(), arm_tr.entry());
        builder.watch_conditions_to(mctx.gl(), conditions, entry_tr);
        proc_builder.set_entry(arm_tr);
    } else {
        proc_builder.set_standing(mctx.gl(), conditions.into_boxed_slice());
    }

    proc_builder.finalize(mctx.gl());
    Ok(())
}

/// The value of an input tied to a constant, if it is one.
fn constant_input<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &MutLowerContext<'a>,
    scope: SymbolId,
    input: AstId<'a, Expr<'a>>,
) -> Option<Level> {
    // An input without signals that is not a constant expression is lowered as any other input.
    let mut diagnostics = Diagnostics::default();
    let value = eval_constant_expr(
        &mctx.gl,
        ctx.arenas,
        &ctx.table,
        scope,
        &mut diagnostics,
        input.into_constant(),
        Some(SCALAR_VSIZE),
    )
    .ok()?;
    if !diagnostics.errors.is_empty() {
        return None;
    }
    Some(match value.into_bits().select_value(0) {
        FvLogicValue::L0 => Level::L0,
        FvLogicValue::L1 => Level::L1,
        FvLogicValue::X | FvLogicValue::Z => Level::X,
    })
}

/// A copy of an input that holds its value before the last change.
///
/// It has the `mode` of the input: in a two-valued design an input can still be `x`, and in a
/// four-valued design a two-valued input must not start as `x`.
fn new_before_input(mctx: &mut MutLowerContext<'_>, mode: LogicMode) -> SignalKey {
    let mut flags = SignalFlags::EMPTY;
    flags.set(SignalFlags::POKE_T0, mode.is_two_value());
    mctx.gl.signals.insert(Signal {
        name: String::from("__UDP_INPUT"),
        size: SCALAR_VSIZE,
        initialize: None,
        mode,
        flags,
        origin: TokenRange::default(),
    })
}

/// Is the transition from `before` to `after` one of `edge`?
fn lower_edge(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    before: VariableKey,
    after: VariableKey,
    edge: Transition,
) -> VariableKey {
    match edge {
        Transition::RISING => return builder.andnot(gl, after, before),
        Transition::FALLING => return builder.andnot(gl, before, after),
        Transition::POSEDGE => return builder.posedge(gl, before, after),
        Transition::NEGEDGE => return builder.negedge(gl, before, after),
        Transition::QM_TO_QM => return builder.not_case_equals(gl, before, after),
        _ => {}
    }

    // A change from one set of values to another, as `(0?)`.
    let befores = edge.before_values();
    let afters = edge.after_values();
    let mut product = Transition::EMPTY;
    for b in befores.iter() {
        for a in afters.iter() {
            if b != a {
                product |= Transition::new(Level::ALL[b], Level::ALL[a]);
            }
        }
    }
    if product == edge {
        let mut acc = builder.not_case_equals(gl, before, after);
        for (value, values) in [(before, befores), (after, afters)] {
            if values.num_set() < Level::ALL.len() as u32 {
                let is_value = lower_values(gl, builder, value, values);
                acc = builder.and(gl, acc, is_value);
            }
        }
        return acc;
    }

    // Any other set of transitions, one by one.
    let mut acc = builder.constant(gl, Bits::from(false));
    for b in Level::ALL {
        for a in Level::ALL {
            if !edge.contains(Transition::new(b, a)) {
                continue;
            }

            let is_before =
                builder.case_equals_constant(gl, before, Bits::from(b.into_fv_logic_value()));
            let is_after =
                builder.case_equals_constant(gl, after, Bits::from(a.into_fv_logic_value()));
            let is_transition = builder.and(gl, is_before, is_after);
            acc = builder.or(gl, acc, is_transition);
        }
    }
    acc
}

/// Matches the rows of `table` one by one and drives the next state of the first that matches, or
/// `x` if none does. Every path ends in `watch_bb`.
#[allow(clippy::too_many_arguments)]
fn lower_rows(
    mctx: &mut MutLowerContext<'_>,
    builder: &mut BasicBlockBuilder,
    table: &Table,
    constants: &[Option<Level>],
    input_vars: &[VariableKey],
    before_inputs: &[Option<SignalKey>],
    output: SignalKey,
    watch_bb: BasicBlockKey,
) {
    for row in &table.rows {
        let mut acc = builder.constant(mctx.gl(), Bits::from(true));
        for (i, &transition) in table.transitions(row).iter().enumerate() {
            // The rows that are left match the constants.
            if constants[i].is_some() || transition == Transition::TO_QM {
                continue;
            }
            let condition = if transition.is_level() {
                lower_values(mctx.gl(), builder, input_vars[i], transition.after_values())
            } else {
                let before_input = before_inputs[i].expect("an input with edges keeps its value");
                let before = builder.probe(mctx.gl(), before_input);
                // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2005) p. 105
                // > The z values passed to UDP inputs shall be treated the same as x values.
                let before = builder.z_to_x(mctx.gl(), before);
                lower_edge(mctx.gl(), builder, before, input_vars[i], transition)
            };
            acc = builder.and(mctx.gl(), acc, condition);
        }
        if !matches!(row.current, UdpLevelSymbol::QuestionMark) {
            let prb = builder.probe(mctx.gl(), output);
            let is_level = lower_level(mctx.gl(), builder, prb, row.current);
            acc = builder.and(mctx.gl(), acc, is_level);
        }

        let start_bb = builder.key();
        *builder = builder.next_terminate_later(mctx.gl());
        let drive_bb = builder.key();

        // A `-` next state leaves the output untouched and skips the assignment.
        if let Some(next) = row.next {
            let value = builder.constant_scalar(mctx.gl(), next.into_fv_logic_value());
            builder.drive(mctx.gl(), output, value);
        }

        *builder = builder.next_terminate_later(mctx.gl());

        mctx.gl.bbs[start_bb].terminator =
            BasicBlockTerminator::Branch(acc, drive_bb, builder.key());
        mctx.gl.bbs[drive_bb].terminator = BasicBlockTerminator::Jump(watch_bb);
    }

    // No entry matched.
    let value = builder.constant(mctx.gl(), Bits::new_unknown(SCALAR_VSIZE));
    builder.drive(mctx.gl(), output, value);
    builder.jump_to(mctx.gl(), watch_bb);
}
