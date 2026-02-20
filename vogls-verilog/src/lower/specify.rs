use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::bits::arithmetic::FvLogicValue;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockTerminator, Bits, GlobalContext, LogicMode, PhiRef, SCALAR_VSIZE, SignalKey,
    TIME_VSIZE, VariableKey, new_anonymous_builder,
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
    delays: Delays,
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
                let tend = builder.probe(gl, tend);
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
                let tend = builder.probe(gl, tend);
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
                let tend = builder.probe(gl, tend);
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
        let tend = builder.probe(gl, tend);
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

pub struct Delay {
    min: u64,
    typ: u64,
    max: u64,
}

impl Delay {
    fn eval<'a>(
        gl: &mut GlobalContext,
        arenas: &'a AstArenas,
        scope: &mut Scope<'a>,
        id: AstId<ConstantMinTypMaxExpression>,
        diagnostics: &mut Diagnostics,
    ) -> Result<Self, ()> {
        match arenas.get(id) {
            ConstantMinTypMaxExpression::Single(delay) => {
                let delay = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, *delay)?
                    .as_integer()
                    .unwrap();

                // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 222
                // "If the path delay expression results in a negative value, it shall be treated as zero."
                let delay = delay.max(0) as u64;

                Ok(Self {
                    min: delay,
                    typ: delay,
                    max: delay,
                })
            }
            ConstantMinTypMaxExpression::MinTypMax { min, typ, max } => {
                let min = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, *min)?
                    .as_integer()
                    .unwrap();
                let typ = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, *typ)?
                    .as_integer()
                    .unwrap();
                let max = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, *max)?
                    .as_integer()
                    .unwrap();

                // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 222
                // "If the path delay expression results in a negative value, it shall be treated as zero."
                let min = min.max(0) as u64;
                let typ = typ.max(0) as u64;
                let max = max.max(0) as u64;

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
                _ = data_source_expression;

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

                if input_net.ty.force_net_width() != output_net.ty.force_net_width() {
                    diagnostics.not_yet_implemented(
                        arenas.get_span(item),
                        "input and output don't have the same net width",
                    );
                    return Err(());
                }
                if !input_net.dims.is_empty() || !output_net.dims.is_empty() {
                    diagnostics
                        .not_yet_implemented(arenas.get_span(item), "input or output is array");
                    return Err(());
                }
                if input_net.ty.force_net_width() != SCALAR_VSIZE {
                    diagnostics
                        .not_yet_implemented(arenas.get_span(item), "specify for non-scalar net");
                    return Err(());
                }

                let delays = arenas.get(*path_delay_value);
                let delays = delays.list_of_delay_expressions;
                let delays = match delays.len() {
                    1 => Delays::One(Delay::eval(gl, arenas, scope, delays.get(0), diagnostics)?),
                    2 => {
                        let trise = Delay::eval(gl, arenas, scope, delays.get(0), diagnostics)?;
                        let tfall = Delay::eval(gl, arenas, scope, delays.get(1), diagnostics)?;
                        Delays::Two { trise, tfall }
                    }
                    3 => {
                        let trise = Delay::eval(gl, arenas, scope, delays.get(0), diagnostics)?;
                        let tfall = Delay::eval(gl, arenas, scope, delays.get(1), diagnostics)?;
                        let tz = Delay::eval(gl, arenas, scope, delays.get(2), diagnostics)?;
                        Delays::Three { trise, tfall, tz }
                    }
                    6 => {
                        let t01 = Delay::eval(gl, arenas, scope, delays.get(0), diagnostics)?;
                        let t10 = Delay::eval(gl, arenas, scope, delays.get(1), diagnostics)?;
                        let t0z = Delay::eval(gl, arenas, scope, delays.get(2), diagnostics)?;
                        let tz1 = Delay::eval(gl, arenas, scope, delays.get(3), diagnostics)?;
                        let t1z = Delay::eval(gl, arenas, scope, delays.get(4), diagnostics)?;
                        let tz0 = Delay::eval(gl, arenas, scope, delays.get(5), diagnostics)?;
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
                        let t01 = Delay::eval(gl, arenas, scope, delays.get(0), diagnostics)?;
                        let t10 = Delay::eval(gl, arenas, scope, delays.get(1), diagnostics)?;
                        let t0z = Delay::eval(gl, arenas, scope, delays.get(2), diagnostics)?;
                        let tz1 = Delay::eval(gl, arenas, scope, delays.get(3), diagnostics)?;
                        let t1z = Delay::eval(gl, arenas, scope, delays.get(4), diagnostics)?;
                        let tz0 = Delay::eval(gl, arenas, scope, delays.get(5), diagnostics)?;
                        let t0x = Delay::eval(gl, arenas, scope, delays.get(6), diagnostics)?;
                        let tx1 = Delay::eval(gl, arenas, scope, delays.get(7), diagnostics)?;
                        let t1x = Delay::eval(gl, arenas, scope, delays.get(8), diagnostics)?;
                        let tx0 = Delay::eval(gl, arenas, scope, delays.get(9), diagnostics)?;
                        let txz = Delay::eval(gl, arenas, scope, delays.get(10), diagnostics)?;
                        let tzx = Delay::eval(gl, arenas, scope, delays.get(11), diagnostics)?;
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

                if matches!(path.condition, Condition::NoOtherCondition) {
                    todo!();
                }

                let condition = condition.map(|c| builder.extract_constant(gl, c, 0, SCALAR_VSIZE));
                match (condition, &mut new_wait_time_set) {
                    (None, _) | (_, None) => new_wait_time_set = None,
                    (Some(condition), Some(new_wait_time_set)) => {
                        *new_wait_time_set = builder.or(gl, *new_wait_time_set, condition);
                    }
                }

                let path_wait_time = builder.minus(gl, time, lupdt);
                let delay = path.delays.calculate(gl, &mut builder, output, proxy);
                let path_wait_time = builder.plus(gl, path_wait_time, delay);
                let path_wait_time = builder.min(gl, new_wait_time, path_wait_time);

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
        let do_while_condition = builder.case_equals(gl, old_proxy_value, new_proxy_value);
        builder = builder.branch_false_to(gl, do_while_condition, wait_loop_bb);

        builder.drive(gl, output, new_proxy_value);
        builder.jump_to(gl, entry);
    }

    Ok(())
}
