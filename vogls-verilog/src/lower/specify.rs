use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::bits::arithmetic::FvLogicValue;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    Bits, GlobalContext, LogicMode, PhiRef, ProcessKind, SCALAR_VSIZE, SignalKey, TIME_VSIZE,
    VariableKey, new_process,
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
use crate::lower::{eval_constant_expr, try_resolve_symbol_id, unwrap_get_net_mut};

use super::{LowerContext, MutLowerContext};

pub enum Condition<'a> {
    None,

    InputPosedge,
    InputNegedge,
    NoOtherCondition,

    Expr(AstId<'a, Expr<'a>>),
    InputPosedgeExpr(AstId<'a, Expr<'a>>),
    InputNegedgeExpr(AstId<'a, Expr<'a>>),
}

pub struct SpecifyOutput<'a> {
    pub sid: SymbolId,
    pub inputs: VgHashMap<SignalKey, usize>,
    pub paths: Vec<(SignalKey, Vec<SpecifyPath<'a>>)>,
}

pub struct SpecifyPath<'a> {
    pub condition: Condition<'a>,
    pub delays: Delays,
}

// @Performance.
//
// This structure is huge... 296 bytes at time of writing. Most of the time, this should only be
// around 16 bytes.
//
// We should probably have a vector that contains all the times and a 64-bit DelayPtr that points
// into this vector. We could use 12 bits to indicate whether that delay is `min:typ:max` or `typ`
// and 3 bits to indicate whether it is 1, 2, 3, 6 or 12 delays. That leaves 49 bits to point into
// the delay vector. That is plenty.
//
// Then again. This structure is generally very short lived so it is not that much of a memoryhog
// I guess.
#[derive(Clone)]
pub enum Delays {
    One(Delay),
    Two {
        trise: Delay,
        tfall: Delay,
    },
    Three {
        trise: Delay,
        tfall: Delay,
        tz: Delay,
    },
    Six {
        t01: Delay,
        t10: Delay,
        t0z: Delay,
        tz1: Delay,
        t1z: Delay,
        tz0: Delay,
    },
    Twelve {
        t01: Delay,
        t10: Delay,
        t0z: Delay,
        tz1: Delay,
        t1z: Delay,
        tz0: Delay,

        t0x: Delay,
        tx1: Delay,
        t1x: Delay,
        tx0: Delay,
        txz: Delay,
        tzx: Delay,
    },
}
impl Delays {
    fn calculate(
        &self,
        gl: &mut GlobalContext,
        builder: &mut vogls_ir::BasicBlockBuilder,
        tstart: SignalKey,
        tend: SignalKey,
        bitidx: u32,
    ) -> VariableKey {
        if let Delays::One(delay) = self {
            // @Performance: At the moment, this is still a variable wait. We could propagate this
            // constant up so that the wait can be become a constant wait. This should allow for
            // better optimization / codegen further on.
            return builder.constant_u64(gl, delay.get());
        }

        // Fast path: Two value logic.
        if gl.logic_mode == LogicMode::TwoValue {
            let (Delays::Two {
                trise: t01,
                tfall: t10,
            }
            | Delays::Three {
                trise: t01,
                tfall: t10,
                ..
            }
            | Delays::Six { t01, t10, .. }
            | Delays::Twelve { t01, t10, .. }) = self
            else {
                unreachable!("Delays::One handled before.");
            };

            let (t01, t10) = (t01.get(), t10.get());
            let mut out_delay = builder.constant_u64(gl, t01);
            if t01 != t10 {
                let tstart = builder.probe(gl, tstart);
                let tstart = builder.select_bit_constant(gl, tstart, bitidx);
                let tend = builder.probe(gl, tend);
                let tend = builder.select_bit_constant(gl, tend, bitidx);
                let is_t10 = builder.ornot(gl, tstart, tend);
                let t10_delay = builder.constant_u64(gl, t10);
                out_delay = builder.select(gl, is_t10, t10_delay, out_delay)
            }
            return out_delay;
        }

        if let Delays::Two { trise, tfall } = self {
            // Derived from Chapter 14 of IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001)
            //
            // | f | t | delay             | C |
            // |---|---|-------------------|---|
            // | 0 | 1 | trise             | 0 |
            // | 1 | 0 | tfall             | 1 |
            // | 0 | z | trise             | 0 |
            // | z | 1 | trise             | 0 |
            // | 1 | z | tfall             | 1 |
            // | z | 0 | tfall             | 1 |
            // | 0 | x | trise             | 0 |
            // | x | 1 | trise             | 0 |
            // | 1 | x | tfall             | 1 |
            // | x | 0 | tfall             | 1 |
            // | x | z | max(trise, tfall) | 2 |
            // | z | x | min(trise, tfall) | 3 |
            //
            // C0: (t == 1 || f == 0) -> trise
            // C1: (f == 1 || t == 0) -> tfall
            // C2: ({f,t} == 2'bxz)   -> max(trise, tfall)
            // C3: ({f,t} == 2'bzx)   -> min(trise, tfall)

            let trise = trise.get();
            let tfall = tfall.get();

            let mut out_delay = builder.constant_u64(gl, trise);
            if trise != tfall {
                let tstart = builder.probe(gl, tstart);
                let tstart = builder.select_bit_constant(gl, tstart, bitidx);
                let tend = builder.probe(gl, tend);
                let tend = builder.select_bit_constant(gl, tend, bitidx);
                let x = builder.constant(gl, Bits::from(FvLogicValue::X));
                let z = builder.constant(gl, Bits::from(FvLogicValue::Z));
                let zero = builder.constant(gl, Bits::from(false));
                let one = builder.constant(gl, Bits::from(true));

                // C1
                let start_is_one = builder.case_equals(gl, tstart, one);
                let end_is_zero = builder.case_equals(gl, tend, zero);
                let is_c1 = builder.or(gl, start_is_one, end_is_zero);
                let c1_delay = builder.constant_u64(gl, tfall);
                out_delay = builder.select(gl, is_c1, c1_delay, out_delay);

                // C2
                let start_is_x = builder.case_equals(gl, tstart, x);
                let end_is_z = builder.case_equals(gl, tend, z);
                let is_c2 = builder.and(gl, start_is_x, end_is_z);
                let c2_delay = builder.constant_u64(gl, u64::max(trise, tfall));
                out_delay = builder.select(gl, is_c2, c2_delay, out_delay);

                // C3
                let start_is_z = builder.case_equals(gl, tstart, z);
                let end_is_x = builder.case_equals(gl, tend, x);
                let is_c3 = builder.and(gl, start_is_z, end_is_x);
                let c3_delay = builder.constant_u64(gl, u64::min(trise, tfall));
                out_delay = builder.select(gl, is_c3, c3_delay, out_delay);
            }
            return out_delay;
        }

        if let Delays::Three { trise, tfall, tz } = self {
            // Derived from Chapter 14 of IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001)
            //
            // | f | t | delay             | C |
            // |---|---|-------------------|---|
            // | 0 | 1 | trise             | 0 |
            // | 1 | 0 | tfall             | 1 |
            // | 0 | z | tz                | 2 |
            // | z | 1 | trise             | 0 |
            // | 1 | z | tz                | 2 |
            // | z | 0 | tfall             | 1 |
            // | 0 | x | min(tz, trise)    | 3 |
            // | x | 1 | trise             | 0 |
            // | 1 | x | min(tz, tfall)    | 4 |
            // | x | 0 | tfall             | 1 |
            // | x | z | tz                | 2 |
            // | z | x | min(tfall, trise) | 5 |
            //
            // C0: (t == 1 || f == 0) -> trise
            // C1: (f == 1 || t == 0) -> tfall
            // C2: (t == 1'bz)        -> tz
            // C3: ({f,t} == 2'b0x)   -> min(tz, trise)
            // C4: ({f,t} == 2'b1x)   -> min(tz, tfall)
            // C5: ({f,t} == 2'bzx)   -> min(tfall, trise)

            let trise_delay = trise.get();
            let tfall_delay = tfall.get();
            let tz_delay = tz.get();

            let mut out_delay = builder.constant_u64(gl, trise_delay);
            if trise_delay != tfall_delay || trise_delay != tz_delay {
                let tstart = builder.probe(gl, tstart);
                let tstart = builder.select_bit_constant(gl, tstart, bitidx);
                let tend = builder.probe(gl, tend);
                let tend = builder.select_bit_constant(gl, tend, bitidx);
                let zero = builder.constant(gl, Bits::from(false));
                let one = builder.constant(gl, Bits::from(true));
                let x = builder.constant(gl, Bits::from(FvLogicValue::X));
                let z = builder.constant(gl, Bits::from(FvLogicValue::Z));

                // C1
                if trise_delay != tfall_delay {
                    let start_is_one = builder.case_equals(gl, tstart, one);
                    let end_is_zero = builder.case_equals(gl, tend, zero);
                    let is_c1 = builder.or(gl, start_is_one, end_is_zero);
                    let c1_delay = builder.constant_u64(gl, tfall_delay);
                    out_delay = builder.select(gl, is_c1, c1_delay, out_delay);
                }

                // C2
                if trise_delay != tz_delay {
                    let is_c2 = builder.case_equals(gl, tend, z);
                    let c2_delay = builder.constant_u64(gl, tz_delay);
                    out_delay = builder.select(gl, is_c2, c2_delay, out_delay);
                }

                // C3
                if trise_delay != tz_delay {
                    let start_is_zero = builder.case_equals(gl, tstart, zero);
                    let end_is_x = builder.case_equals(gl, tend, x);
                    let is_c3 = builder.and(gl, start_is_zero, end_is_x);
                    let c3_delay = builder.constant_u64(gl, u64::min(tz_delay, trise_delay));
                    out_delay = builder.select(gl, is_c3, c3_delay, out_delay);
                }

                // C4
                if tfall_delay != tz_delay {
                    let start_is_one = builder.case_equals(gl, tstart, one);
                    let end_is_x = builder.case_equals(gl, tend, x);
                    let is_c4 = builder.and(gl, start_is_one, end_is_x);
                    let c4_delay = builder.constant_u64(gl, u64::min(tz_delay, tfall_delay));
                    out_delay = builder.select(gl, is_c4, c4_delay, out_delay);
                }

                // C5
                if trise_delay != tfall_delay {
                    let start_is_z = builder.case_equals(gl, tstart, z);
                    let end_is_x = builder.case_equals(gl, tend, x);

                    let is_c5 = builder.and(gl, start_is_z, end_is_x);
                    let c5_delay = builder.constant_u64(gl, u64::min(trise_delay, tfall_delay));
                    out_delay = builder.select(gl, is_c5, c5_delay, out_delay);
                }
            }
            return out_delay;
        }

        let (t01, t10, t0z, tz1, t1z, tz0, t0x, tx1, t1x, tx0, txz, tzx) = match self {
            Delays::Six {
                t01,
                t10,
                t0z,
                tz1,
                t1z,
                tz0,
            } => {
                let (t01, t10, t0z, tz1, t1z, tz0) = (
                    t01.get(),
                    t10.get(),
                    t0z.get(),
                    tz1.get(),
                    t1z.get(),
                    tz0.get(),
                );
                (
                    t01,
                    t10,
                    t0z,
                    tz1,
                    t1z,
                    tz0,
                    t0z.min(t01),
                    tz1.max(t01),
                    t1z.min(t10),
                    tz0.max(t10),
                    t1z.max(t0z),
                    tz1.min(tz0),
                )
            }
            Delays::Twelve {
                t01,
                t10,
                t0z,
                tz1,
                t1z,
                tz0,
                t0x,
                tx1,
                t1x,
                tx0,
                txz,
                tzx,
            } => (
                t01.get(),
                t10.get(),
                t0z.get(),
                tz1.get(),
                t1z.get(),
                tz0.get(),
                t0x.get(),
                tx1.get(),
                t1x.get(),
                tx0.get(),
                txz.get(),
                tzx.get(),
            ),
            _ => unreachable!(),
        };

        // @Performance
        // We should be able to do some better delay merging here.
        let mut out_delay = builder.constant_u64(gl, t01);
        let tstart = builder.probe(gl, tstart);
        let tstart = builder.select_bit_constant(gl, tstart, bitidx);
        let tend = builder.probe(gl, tend);
        let tend = builder.select_bit_constant(gl, tend, bitidx);
        let zero = builder.constant(gl, Bits::from(false));
        let one = builder.constant(gl, Bits::from(true));
        let x = builder.constant(gl, Bits::from(FvLogicValue::X));
        let z = builder.constant(gl, Bits::from(FvLogicValue::Z));
        macro_rules! transition {
            ($t:expr, $l:expr, $r:expr) => {
                if t01 != $t {
                    let is_start = builder.case_equals(gl, tstart, $l);
                    let is_end = builder.case_equals(gl, tend, $r);
                    let is_transition = builder.and(gl, is_start, is_end);
                    let transition_time = builder.constant_u64(gl, $t);
                    out_delay = builder.select(gl, is_transition, transition_time, out_delay);
                }
            };
        }
        transition!(t10, one, zero);
        transition!(t0z, zero, z);
        transition!(tz1, z, one);
        transition!(t1z, one, z);
        transition!(tz0, z, zero);
        transition!(t0x, zero, x);
        transition!(tx1, x, one);
        transition!(t1x, one, x);
        transition!(tx0, x, zero);
        transition!(txz, x, z);
        transition!(tzx, z, x);

        out_delay
    }
}

#[derive(Clone)]
pub struct Delay {
    pub min: u64,
    pub typ: u64,
    pub max: u64,
}

impl Delay {
    fn eval<'a>(
        ctx: &LowerContext<'a>,
        mctx: &mut MutLowerContext,
        scope: SymbolId,
        id: AstId<'a, ConstantMinTypMaxExpression<'a>>,
    ) -> Result<Self, ()> {
        match &*id {
            ConstantMinTypMaxExpression::Single(delay) => {
                let delay = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *delay,
                    None,
                )?
                .as_integer()
                .unwrap();

                // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 222
                // "If the path delay expression results in a negative value, it shall be treated as zero."
                let delay = delay.max(0) as u64;
                let delay = delay * ctx.time_scale.time_unit;

                Ok(Self {
                    min: delay,
                    typ: delay,
                    max: delay,
                })
            }
            ConstantMinTypMaxExpression::MinTypMax { min, typ, max } => {
                let min = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *min,
                    None,
                )?
                .as_integer()
                .unwrap();
                let typ = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *typ,
                    None,
                )?
                .as_integer()
                .unwrap();
                let max = eval_constant_expr(
                    &mctx.gl,
                    &ctx.arenas,
                    &ctx.table,
                    scope,
                    &mut mctx.diagnostics,
                    *max,
                    None,
                )?
                .as_integer()
                .unwrap();

                // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 222
                // "If the path delay expression results in a negative value, it shall be treated as zero."
                let min = min.max(0) as u64;
                let typ = typ.max(0) as u64;
                let max = max.max(0) as u64;

                let min = min * ctx.time_scale.time_unit;
                let typ = typ * ctx.time_scale.time_unit;
                let max = max * ctx.time_scale.time_unit;

                Ok(Self { min, typ, max })
            }
        }
    }

    fn is_single(&self) -> bool {
        self.min == self.typ && self.typ == self.max
    }

    fn get(&self) -> u64 {
        if self.is_single() {
            return self.typ;
        }

        // @TODO: Make this configurable
        self.typ
    }
}

pub fn lower_specify<'a>(
    ctx: &mut LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,

    items: AstIdRange<'a, SpecifyBlockItem<'a>>,
    outs_lut: &mut VgHashMap<SignalKey, usize>,
    outs: &mut Vec<(SignalKey, SpecifyOutput<'a>)>,
) -> Result<(), ()> {
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

                        // @NOTE: Should actually not be reachable from normal Verilog.
                        (StateDependentCondition::Ifnone, _) => todo!(),
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

                let input_sid = try_resolve_symbol_id(
                    scope,
                    &ctx.table,
                    &ctx.arenas,
                    input.ident,
                    &mut mctx.diagnostics,
                )?;
                let VSymbol::Net(input_net) = &ctx.table[input_sid].content else {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_item_span(input.ident),
                        "cannot be used as net",
                    );
                    return Err(());
                };
                let input = input_net.net.probe_signal();
                let output_sid = try_resolve_symbol_id(
                    scope,
                    &ctx.table,
                    &ctx.arenas,
                    output.ident,
                    &mut mctx.diagnostics,
                )?;
                let VSymbol::Net(output_net) = &ctx.table[output_sid].content else {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_item_span(output.ident),
                        "cannot be used as net",
                    );
                    return Err(());
                };
                let output = output_net.net.blocking_drive_signal();

                if matches!(variant, PathDeclarationVariant::Full)
                    && input_net.ty.force_net_width() != output_net.ty.force_net_width()
                {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item),
                        "input and output don't have the same net width",
                    );
                    return Err(());
                }
                if matches!(variant, PathDeclarationVariant::Parallel)
                    && input_net.ty.force_net_width().get() != 1
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
                if input_net.ty.force_net_width() != SCALAR_VSIZE {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(item),
                        "specify for non-scalar net",
                    );
                    return Err(());
                }

                let delays = &**path_delay_value;
                let delays = delays.list_of_delay_expressions;
                let delays = match delays.len() {
                    1 => Delays::One(Delay::eval(ctx, mctx, scope, delays.get(0))?),
                    2 => {
                        let trise = Delay::eval(ctx, mctx, scope, delays.get(0))?;
                        let tfall = Delay::eval(ctx, mctx, scope, delays.get(1))?;
                        Delays::Two { trise, tfall }
                    }
                    3 => {
                        let trise = Delay::eval(ctx, mctx, scope, delays.get(0))?;
                        let tfall = Delay::eval(ctx, mctx, scope, delays.get(1))?;
                        let tz = Delay::eval(ctx, mctx, scope, delays.get(2))?;
                        Delays::Three { trise, tfall, tz }
                    }
                    6 => {
                        let t01 = Delay::eval(ctx, mctx, scope, delays.get(0))?;
                        let t10 = Delay::eval(ctx, mctx, scope, delays.get(1))?;
                        let t0z = Delay::eval(ctx, mctx, scope, delays.get(2))?;
                        let tz1 = Delay::eval(ctx, mctx, scope, delays.get(3))?;
                        let t1z = Delay::eval(ctx, mctx, scope, delays.get(4))?;
                        let tz0 = Delay::eval(ctx, mctx, scope, delays.get(5))?;
                        Delays::Six {
                            t01,
                            t10,
                            t0z,
                            tz1,
                            t1z,
                            tz0,
                        }
                    }
                    12 => {
                        let t01 = Delay::eval(ctx, mctx, scope, delays.get(0))?;
                        let t10 = Delay::eval(ctx, mctx, scope, delays.get(1))?;
                        let t0z = Delay::eval(ctx, mctx, scope, delays.get(2))?;
                        let tz1 = Delay::eval(ctx, mctx, scope, delays.get(3))?;
                        let t1z = Delay::eval(ctx, mctx, scope, delays.get(4))?;
                        let tz0 = Delay::eval(ctx, mctx, scope, delays.get(5))?;
                        let t0x = Delay::eval(ctx, mctx, scope, delays.get(6))?;
                        let tx1 = Delay::eval(ctx, mctx, scope, delays.get(7))?;
                        let t1x = Delay::eval(ctx, mctx, scope, delays.get(8))?;
                        let tx0 = Delay::eval(ctx, mctx, scope, delays.get(9))?;
                        let txz = Delay::eval(ctx, mctx, scope, delays.get(10))?;
                        let tzx = Delay::eval(ctx, mctx, scope, delays.get(11))?;
                        Delays::Twelve {
                            t01,
                            t10,
                            t0z,
                            tz1,
                            t1z,
                            tz0,
                            t0x,
                            tx1,
                            t1x,
                            tx0,
                            txz,
                            tzx,
                        }
                    }
                    _ => unreachable!(),
                };

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
                // @TODO: Don't ignore these.
            }
        }
    }

    // @Performance: Scratchpad these.
    let mut input_before_lut = VgHashMap::<SignalKey, usize>::default();
    let mut input_before = Vec::<(SignalKey, VariableKey, Option<PhiRef>)>::new();
    // let mut condition_orsum = VgHashMap::<SignalKey, VariableKey>::default();

    outs_lut.clear();
    for (output, specify) in outs.drain(..) {
        lower_iopath(
            ctx,
            mctx,
            scope,
            output,
            specify,
            &mut input_before_lut,
            &mut input_before,
        )?;
    }

    Ok(())
}

pub fn lower_iopath<'a>(
    ctx: &mut LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    output: SignalKey,
    specify: SpecifyOutput,
    input_before_lut: &mut VgHashMap<SignalKey, usize>,
    input_before: &mut Vec<(SignalKey, VariableKey, Option<PhiRef>)>,
) -> Result<(), ()> {
    let mut proxy = mctx.gl.signals.get(output).unwrap().clone();
    proxy.name = format!("{}::SPECIFY_PROXY", proxy.name);
    let proxy = mctx.gl.signals.insert(proxy);
    let net = unwrap_get_net_mut(&mut ctx.table, specify.sid);
    let prev = net.net.set_specify(proxy);
    assert!(prev.is_none());

    for output_bitidx in 0..mctx.gl.signals[output].size.get() {
        input_before_lut.clear();
        input_before.clear();

        let (_, mut builder) = new_process(mctx.gl(), ProcessKind::Specify, TokenRange::default());
        let entry = builder.key();

        // @Correctness: This might need something like. `Initial Value of Signal X`. I think
        // this is incorrect if the event does not get triggered first.
        for (input, paths) in &specify.paths {
            for path in paths {
                if matches!(
                    path.condition,
                    Condition::InputPosedge
                        | Condition::InputNegedge
                        | Condition::InputPosedgeExpr(_)
                        | Condition::InputNegedgeExpr(_)
                ) {
                    let before = builder.probe(mctx.gl(), *input);
                    let idx = input_before.len();
                    input_before_lut.insert(*input, idx);
                    input_before.push((*input, before, None));
                    break;
                }
            }
        }

        builder = builder.jump(mctx.gl());
        let wait_loop_bb = builder.key();

        for (_, variable, phi_ref) in input_before.iter_mut() {
            let pr;
            (*variable, pr) =
                builder.phi(mctx.gl(), [(entry, *variable), (entry, *variable)].into());
            *phi_ref = Some(pr);
        }

        // active_time = max_{signal} last_update_time(signal)
        // is_active(s) = last_update_time(s) == active_time && active_time != 0xF..F
        let time = builder.time(mctx.gl());
        let mut active_time = builder.constant(mctx.gl(), Bits::from_u64(TIME_VSIZE, 1));
        for (input, _) in &specify.paths {
            let lupdt = builder.lupdt(mctx.gl(), *input);
            let lupdt = builder.plus_constant(mctx.gl(), lupdt, Bits::from_u64(TIME_VSIZE, 1));
            active_time = builder.max(mctx.gl(), active_time, lupdt);
        }
        active_time = builder.minus_constant(mctx.gl(), active_time, Bits::from_u64(TIME_VSIZE, 1));

        let mut wait_time_set = builder.constant(mctx.gl(), Bits::new_zeroed(SCALAR_VSIZE));
        let mut wait_time = builder.constant(mctx.gl(), Bits::new_ones(TIME_VSIZE));

        for (input, paths) in &specify.paths {
            let lupdt = builder.lupdt(mctx.gl(), *input);
            let is_active = builder.case_equals(mctx.gl(), lupdt, active_time);

            let mut new_wait_time_set = Some(wait_time_set);
            let mut new_wait_time = wait_time;

            for path in paths {
                let mut condition = None;
                if matches!(
                    path.condition,
                    Condition::InputPosedge | Condition::InputPosedgeExpr(_)
                ) {
                    let before = input_before[input_before_lut[input]].1;
                    let after = builder.probe(mctx.gl(), *input);
                    condition = Some(builder.posedge(mctx.gl(), before, after));
                }
                if matches!(
                    path.condition,
                    Condition::InputNegedge | Condition::InputNegedgeExpr(_)
                ) {
                    let before = input_before[input_before_lut[input]].1;
                    let after = builder.probe(mctx.gl(), *input);
                    condition = Some(builder.negedge(mctx.gl(), before, after));
                }

                if let Condition::Expr(expr)
                | Condition::InputPosedgeExpr(expr)
                | Condition::InputNegedgeExpr(expr) = path.condition
                {
                    let (expr, _) = lower_expr(ctx, mctx, scope, &mut builder, expr, None)?;
                    let expr = builder.reduce_or(mctx.gl(), expr);
                    condition = Some(match condition {
                        None => expr,
                        Some(condition) => builder.and(mctx.gl(), condition, expr),
                    });
                }

                if matches!(path.condition, Condition::NoOtherCondition) {
                    todo!();
                }

                let condition = condition.map(|c| builder.select_bit_constant(mctx.gl(), c, 0));
                match (condition, &mut new_wait_time_set) {
                    (None, _) | (_, None) => new_wait_time_set = None,
                    (Some(condition), Some(new_wait_time_set)) => {
                        *new_wait_time_set = builder.or(mctx.gl(), *new_wait_time_set, condition);
                    }
                }

                let path_wait_time = builder.minus(mctx.gl(), time, lupdt);
                let delay =
                    path.delays
                        .calculate(mctx.gl(), &mut builder, output, proxy, output_bitidx);
                let path_wait_time = builder.plus(mctx.gl(), path_wait_time, delay);
                let path_wait_time = builder.min(mctx.gl(), new_wait_time, path_wait_time);

                new_wait_time = match condition {
                    None => path_wait_time,
                    Some(condition) => {
                        builder.select(mctx.gl(), condition, path_wait_time, new_wait_time)
                    }
                };
            }

            let new_wait_time_set = new_wait_time_set
                .unwrap_or_else(|| builder.constant(mctx.gl(), Bits::new_ones(SCALAR_VSIZE)));

            wait_time_set = builder.select(mctx.gl(), is_active, new_wait_time_set, wait_time_set);
            wait_time = builder.select(mctx.gl(), is_active, new_wait_time, wait_time);
        }

        // Set the wait time to zero, if no condition matched.
        let zero = builder.constant_u64(mctx.gl(), 0);
        let wait_time = builder.select(mctx.gl(), wait_time_set, wait_time, zero);

        let old_proxy_value = builder.probe(mctx.gl(), proxy);
        let old_proxy_value =
            builder.select_bit_constant(mctx.gl(), old_proxy_value, output_bitidx);
        for (input, variable, _) in input_before.iter_mut() {
            *variable = builder.probe(mctx.gl(), *input);
        }

        builder = builder.variable_wait(mctx.gl(), wait_time);

        for (_, variable, phi_ref) in input_before.iter_mut() {
            builder.update_phi_ref(
                mctx.gl(),
                phi_ref.take().unwrap(),
                1,
                builder.key(),
                *variable,
            );
        }

        // do ... while(...);
        let new_proxy_value = builder.probe(mctx.gl(), proxy);
        let new_proxy_value =
            builder.select_bit_constant(mctx.gl(), new_proxy_value, output_bitidx);
        let do_while_condition = builder.case_equals(mctx.gl(), old_proxy_value, new_proxy_value);
        builder = builder.branch_false_to(mctx.gl(), do_while_condition, wait_loop_bb);

        builder.drive_partial_constant(mctx.gl(), output, new_proxy_value, output_bitidx);
        builder.watch_to(mctx.gl(), vec![proxy], wait_loop_bb);
    }

    Ok(())
}
