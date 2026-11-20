use std::sync::Arc;

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
use crate::lower::drivers::DriveOutputFn;
use crate::lower::expression::lower_expr_to_cond_expr;
use crate::lower::cond_expr::{CondExpr, CondExprLowerScratch, CondExprs};
use crate::lower::{eval_constant_expr, try_resolve_hident};

use super::{LowerContext, MutLowerContext, VValue};

#[derive(Clone)]
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

#[derive(Clone)]
pub enum CECondition {
    None,

    InputPosedge,
    InputNegedge,
    NoOtherCondition,

    InputPosedgeNoOtherCondition,
    InputNegedgeNoOtherCondition,

    Expr(CondExpr),
    InputPosedgeExpr(CondExpr),
    InputNegedgeExpr(CondExpr),
}
impl CECondition {

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

    /// The state-dependent condition of the path, if it has one.
    fn expr(&self) -> Option<CondExpr> {
        match self {
            Self::Expr(expr) | Self::InputPosedgeExpr(expr) | Self::InputNegedgeExpr(expr) => {
                Some(*expr)
            }
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct SpecifyOutput {
    pub sid: SymbolId,
    pub inputs: VgHashMap<SignalKey, usize>,
    pub paths: Vec<(SignalKey, Vec<SpecifyPath>)>,
}

#[derive(Clone)]
pub struct SpecifyPath {
    pub condition: CECondition,
    pub delays: DelayPtr,
    pub span: TokenRange,
}

/// Scratch buffers for [`lower_specify_delay`], kept between the bits of an output.
#[derive(Default)]
struct LowerIoPathScratches {
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
    mctx: &mut MutLowerContext<'a>,
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
    mctx: &mut MutLowerContext<'a>,
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

/// Collects the module path delays of the specify block `items` of the module instance `scope`
/// into the [`PathDelays`] of the driver resolution, which puts them in front of their outputs.
pub fn lower_specify<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    items: AstIdRange<'a, SpecifyBlockItem<'a>>,
) -> Result<(), ()> {
    let mut condexpr_scratch = Vec::new();
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
                let output = output_net.terminals.probe_signal();

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
                let mut delays = std::mem::take(&mut mctx.driver_resolution.path_delays.delays);
                let offset = delays.len() as u64;
                let mut triple_mask = 0u16;
                let mut result = Ok(());
                for (i, expr) in exprs.iter().enumerate() {
                    match push_delay(ctx, mctx, scope, expr, &mut delays) {
                        Ok(is_triple) => triple_mask |= u16::from(is_triple) << i,
                        Err(()) => result = Err(()),
                    }
                }
                mctx.driver_resolution.path_delays.delays = delays;
                result?;
                let delays = DelayPtr::new(offset, delay_variant, triple_mask);

                let mut builder = mctx
                    .driver_resolution
                    .path_delays
                    .conds
                    .builder_with_scratch(std::mem::take(&mut condexpr_scratch));
                let mut lower_ce_expr = |e: AstId<'a, Expr<'a>>| {
                    let cond = lower_expr_to_cond_expr(
                        ctx,
                        &mctx.gl,
                        &mut builder,
                        scope,
                        e,
                        Some(SCALAR_VSIZE),
                    );
                    let Some(cond) = cond else {
                        mctx.diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(e),
                            "this state-dependent path condition",
                        );
                        return Err(());
                    };
                    Ok(cond)
                };
                let condition = match condition {
                    Condition::None => CECondition::None,
                    Condition::InputPosedge => CECondition::InputPosedge,
                    Condition::InputNegedge => CECondition::InputNegedge,
                    Condition::NoOtherCondition => CECondition::NoOtherCondition,
                    Condition::InputPosedgeNoOtherCondition => {
                        CECondition::InputPosedgeNoOtherCondition
                    }
                    Condition::InputNegedgeNoOtherCondition => {
                        CECondition::InputNegedgeNoOtherCondition
                    }
                    Condition::Expr(id) => CECondition::Expr(lower_ce_expr(id)?),
                    Condition::InputPosedgeExpr(id) => {
                        CECondition::InputPosedgeExpr(lower_ce_expr(id)?)
                    }
                    Condition::InputNegedgeExpr(id) => {
                        CECondition::InputNegedgeExpr(lower_ce_expr(id)?)
                    }
                };
                condexpr_scratch = builder.take_scratch();
                mctx.driver_resolution.path_delays.push_specify(
                    scope,
                    output_sid,
                    output,
                    input,
                    SpecifyPath {
                        condition,
                        delays,
                        span: ctx.arenas.get_span(item),
                    },
                );
            }
            SpecifyBlockItem::SystemTimingCheck(_) => {
                // @TODO: Don't ignore these.
            }
        }
    }

    Ok(())
}

/// The module path delays of `output`, whose `delays` the paths of `specify` point into.
pub struct SpecifyDelay {
    pub scope: SymbolId,
    pub output: SignalKey,
    pub specify: SpecifyOutput,
    pub delays: Arc<[Time]>,
    /// The conditions that the paths of `specify` are lowered from.
    pub conds: Arc<CondExprs>,
}

/// The condition under which an SDF delay applies.
#[derive(Clone)]
pub enum SdfCondition {
    /// An `IOPATH` or `INTERCONNECT`, which annotates every module path between its ports.
    None,
    /// The `COND` of an `IOPATH`, which annotates the module paths with the same condition. `None`
    /// if the condition cannot be compared, so that it annotates no module path.
    Cond(Option<CondExpr>),
    /// The `CONDELSE` of an `IOPATH`, which annotates the `ifnone` module paths.
    Else,
}

/// An SDF delay from `from` to `to`, which annotates the module paths between them, or is a path
/// of its own if there are none.
#[derive(Clone)]
struct SdfPath {
    from: SignalKey,
    to: SignalKey,
    to_sid: SymbolId,
    condition: SdfCondition,
    delays: DelayPtr,
}

/// All path delays of a design, collected from its specify blocks and SDF files before they are
/// put in front of their outputs in one go (see [`PathDelays::take_merged`]).
#[derive(Default, Clone)]
pub struct PathDelays {
    /// The delay values that every [`DelayPtr`] of the paths points into.
    pub delays: Vec<Time>,
    /// The index of the module paths of each output in `specify`.
    specify_lut: VgHashMap<SignalKey, usize>,
    /// The module paths of specify blocks per output, with the module instance they are in.
    specify: Vec<(SymbolId, SignalKey, SpecifyOutput)>,
    /// The SDF delays, in the order they are read: a later delay overrides an earlier one.
    sdf: Vec<SdfPath>,
    /// The operands of the conditions of the module paths and the SDF delays.
    pub conds: CondExprs,
}

impl<'a> PathDelays {
    pub fn is_empty(&self) -> bool {
        self.specify.is_empty() && self.sdf.is_empty()
    }

    /// The index in `specify` of the module paths of `output`, which are added if there are none.
    fn output_paths(&mut self, scope: SymbolId, output_sid: SymbolId, output: SignalKey) -> usize {
        *self.specify_lut.entry(output).or_insert_with(|| {
            self.specify.push((
                scope,
                output,
                SpecifyOutput {
                    sid: output_sid,
                    inputs: Default::default(),
                    paths: Default::default(),
                },
            ));
            self.specify.len() - 1
        })
    }

    /// Adds the module path `path` from `input` to `output` of a specify block in `scope`.
    pub fn push_specify(
        &mut self,
        scope: SymbolId,
        output_sid: SymbolId,
        output: SignalKey,
        input: SignalKey,
        path: SpecifyPath,
    ) {
        let idx = self.output_paths(scope, output_sid, output);
        let out = &mut self.specify[idx].2;
        let idx = *out.inputs.entry(input).or_insert_with(|| {
            out.paths.push((input, Vec::new()));
            out.paths.len() - 1
        });
        out.paths[idx].1.push(path);
    }

    /// Adds an SDF delay from `from` to `to`, whose values `delays` points into
    /// [`Self::delays`].
    pub fn push_sdf(
        &mut self,
        from: SignalKey,
        to: SignalKey,
        to_sid: SymbolId,
        condition: SdfCondition,
        delays: DelayPtr,
    ) {
        self.sdf.push(SdfPath {
            from,
            to,
            to_sid,
            condition,
            delays,
        });
    }

    /// Takes the path delays of every output, where the SDF delays annotate the module paths of
    /// the specify blocks.
    ///
    /// IEEE Std 1364-2005, 16.2.1: an `IOPATH` without an edge or condition annotates every module
    /// path between its ports, whatever its edge or condition. An SDF delay without such module
    /// paths becomes an unconditional path of its own. A `COND` annotates the module paths with
    /// the same condition and a `CONDELSE` the `ifnone` ones.
    ///
    /// Also returns the number of `COND` and `CONDELSE` delays that annotate no module path, which
    /// are ignored.
    pub fn take_merged(&mut self) -> (Vec<SpecifyDelay>, usize) {
        let mut num_unmatched = 0;
        let mut eq_scratch = Vec::new();
        for SdfPath {
            from,
            to,
            to_sid,
            condition,
            delays,
        } in std::mem::take(&mut self.sdf)
        {
            if !matches!(condition, SdfCondition::None) {
                // A conditional delay can only annotate an existing module path.
                let paths = self
                    .specify_lut
                    .get(&to)
                    .map(|&idx| &mut self.specify[idx].2)
                    .and_then(|out| {
                        let idx = *out.inputs.get(&from)?;
                        Some(&mut out.paths[idx].1)
                    });
                let mut is_matched = false;
                for path in paths.into_iter().flatten() {
                    let matches = match &condition {
                        SdfCondition::None => unreachable!(),
                        SdfCondition::Cond(cond) => {
                            path.condition.expr().zip(*cond).is_some_and(|(a, b)| {
                                self.conds.exprs_cmp(&mut eq_scratch, a, b).is_eq()
                            })
                        }
                        SdfCondition::Else => path.condition.is_ifnone(),
                    };
                    if matches {
                        path.delays = delays.overlay(Some(path.delays), &mut self.delays);
                        is_matched = true;
                    }
                }
                num_unmatched += usize::from(!is_matched);
                continue;
            }

            let idx = self.output_paths(to_sid, to_sid, to);
            let out = &mut self.specify[idx].2;
            let delay_values = &mut self.delays;
            match out.inputs.get(&from) {
                Some(&idx) => out.paths[idx]
                    .1
                    .iter_mut()
                    .for_each(|path| path.delays = delays.overlay(Some(path.delays), delay_values)),
                None => {
                    let delays = delays.overlay(None, delay_values);
                    out.inputs.insert(from, out.paths.len());
                    out.paths.push((
                        from,
                        vec![SpecifyPath {
                            condition: CECondition::None,
                            delays,
                            span: TokenRange::default(),
                        }],
                    ));
                }
            }
        }

        let delays = Arc::<[Time]>::from(std::mem::take(&mut self.delays));
        let conds = Arc::new(std::mem::take(&mut self.conds));
        self.specify_lut.clear();
        let merged = std::mem::take(&mut self.specify)
            .into_iter()
            .map(|(scope, output, specify)| SpecifyDelay {
                scope,
                output,
                specify,
                delays: delays.clone(),
                conds: conds.clone(),
            })
            .collect();
        (merged, num_unmatched)
    }
}

/// Lowers the module path delays of `delay` to one process per bit of its output, which drives the
/// value of that bit of `proxy` with `drive` after the delay of the path that applies.
pub(crate) fn lower_specify_delay<'a>(
    mctx: &mut MutLowerContext<'a>,
    delay: &SpecifyDelay,
    proxy: SignalKey,
    drive: &DriveOutputFn<'a, '_>,
) -> Result<(), ()> {
    let SpecifyDelay {
        scope: _,
        output,
        specify,
        delays,
        conds,
    } = delay;
    let output = *output;
    let LowerIoPathScratches {
        mut input_before_lut,
        mut inputs_before,
        mut conditions,
    } = LowerIoPathScratches::default();
    let mut cond_scratch = CondExprLowerScratch::default();

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

                if let CECondition::Expr(cond)
                | CECondition::InputPosedgeExpr(cond)
                | CECondition::InputNegedgeExpr(cond) = path.condition
                {
                    let expr = conds.lower(mctx.gl(), &mut builder, &mut cond_scratch, cond);

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

        drive(mctx, &mut builder, output_bitidx, new_proxy_value);
        builder.watch_to(mctx.gl(), vec![proxy], wait_loop_tr);
        proc_builder.finalize(mctx.gl());
    }

    Ok(())
}
