use crate::{
    Consume, HierarchicalIdent, Integer, PortSpec, QString, RValue, ScalarConstant, SdfError,
    SdfResult, TokenWalker, Value, expect_peek_param, parse_param_with, peek_param,
};

pub struct TimingCheckSpec<'a> {
    pub items: Vec<TimingCheckDef<'a>>,
}

pub enum TimingCheckDef<'a> {
    Setup(SetupTimingCheck<'a>),
    Hold(HoldTimingCheck<'a>),
    SetupHold(SetupHoldTimingCheck<'a>),
    Recovery(RecoveryTimingCheck<'a>),
    Removal(RemovalTimingCheck<'a>),
    Recrem(RecremTimingCheck<'a>),
    Skew(SkewTimingCheck<'a>),
    BidirectSkew(BidirectSkewTimingCheck<'a>),
    Width(WidthTimingCheck<'a>),
    Period(PeriodTimingCheck<'a>),
    NoChange(NoChangeTimingCheck<'a>),
}

pub enum PortTimingCheck<'a> {
    PortSpec(PortSpec<'a>),
    Cond(Option<QString<'a>>, TimingCheckCondition<'a>, PortSpec<'a>),
}

pub struct TimingCheckCondition<'a> {
    pub scalar_node: ScalarNode<'a>,
    pub variant: TimingCheckConditionVariant,
}

pub enum TimingCheckConditionVariant {
    Plain,
    Inversion,
    Equality(EqualityOperator, ScalarConstant),
}

pub enum EqualityOperator {
    LogicalEquality,
    LogicalInequality,
    CaseEquality,
    CaseInequality,
}

pub struct ScalarNode<'a> {
    pub hident: HierarchicalIdent<'a>,
    pub offset: Option<Integer<'a>>,
}

pub enum SetupHoldRecremArgs<'a> {
    Base(
        PortTimingCheck<'a>,
        PortTimingCheck<'a>,
        RValue<'a>,
        RValue<'a>,
    ),
    Alternative(
        PortSpec<'a>,
        PortSpec<'a>,
        RValue<'a>,
        RValue<'a>,
        Option<SCond<'a>>,
        Option<CCond<'a>>,
    ),
}

pub struct SCond<'a> {
    pub qstring: Option<QString<'a>>,
    pub timing_check_condition: TimingCheckCondition<'a>,
}
pub struct CCond<'a> {
    pub qstring: Option<QString<'a>>,
    pub timing_check_condition: TimingCheckCondition<'a>,
}

pub struct SetupTimingCheck<'a> {
    pub fst: PortTimingCheck<'a>,
    pub snd: PortTimingCheck<'a>,
    pub value: Value<'a>,
}
pub struct HoldTimingCheck<'a> {
    pub fst: PortTimingCheck<'a>,
    pub snd: PortTimingCheck<'a>,
    pub value: Value<'a>,
}
pub struct SetupHoldTimingCheck<'a>(pub SetupHoldRecremArgs<'a>);

pub struct RecoveryTimingCheck<'a> {
    pub fst: PortTimingCheck<'a>,
    pub snd: PortTimingCheck<'a>,
    pub value: Value<'a>,
}
pub struct RemovalTimingCheck<'a> {
    pub fst: PortTimingCheck<'a>,
    pub snd: PortTimingCheck<'a>,
    pub value: Value<'a>,
}
pub struct RecremTimingCheck<'a>(pub SetupHoldRecremArgs<'a>);
pub struct SkewTimingCheck<'a> {
    pub fst: PortTimingCheck<'a>,
    pub snd: PortTimingCheck<'a>,
    pub value: RValue<'a>,
}
pub struct BidirectSkewTimingCheck<'a> {
    pub fst: PortTimingCheck<'a>,
    pub snd: PortTimingCheck<'a>,
    pub fst_value: Value<'a>,
    pub snd_value: Value<'a>,
}
pub struct WidthTimingCheck<'a> {
    pub port_tchk: PortTimingCheck<'a>,
    pub value: Value<'a>,
}
pub struct PeriodTimingCheck<'a> {
    pub port_tchk: PortTimingCheck<'a>,
    pub value: Value<'a>,
}
pub struct NoChangeTimingCheck<'a> {
    pub fst: PortTimingCheck<'a>,
    pub snd: PortTimingCheck<'a>,
    pub fst_rvalue: RValue<'a>,
    pub snd_rvalue: RValue<'a>,
}

impl<'a> Consume<'a> for TimingCheckSpec<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 68)
        // tc_spec ::= ( TIMINGCHECK tchk_def { tchk_def } )

        parse_param_with(tkw, "TIMINGCHECK", |tkw| {
            let mut items = Vec::new();
            let value = TimingCheckDef::consume(tkw)?;
            items.push(value);

            loop {
                tkw.skip_whitespace();
                if tkw.is_next_equal_to(b')') {
                    break;
                }
                let item = TimingCheckDef::consume(tkw)?;
                items.push(item);
            }

            Ok(Self { items })
        })
    }
}

impl<'a> Consume<'a> for PortTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        // port_tchk ::=
        //   port_spec
        // | ( COND [ qstring ] timing_check_condition port_spec )

        if peek_param(tkw) == Some("COND") {
            parse_param_with(tkw, "COND", |tkw| {
                let qstring = if tkw.is_next_equal_to(b'"') {
                    Some(QString::consume(tkw)?)
                } else {
                    None
                };
                tkw.skip_whitespace();
                let timing_check_condition = TimingCheckCondition::consume(tkw)?;

                tkw.skip_whitespace();
                let port_spec = PortSpec::consume(tkw)?;
                Ok(Self::Cond(qstring, timing_check_condition, port_spec))
            })
        } else {
            Ok(Self::PortSpec(PortSpec::consume(tkw)?))
        }
    }
}

impl<'a> Consume<'a> for TimingCheckCondition<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 73)
        // timing_check_condition ::=
        //   scalar_node
        // | inversion_operator scalar_node
        // | scalar_node equality_operator scalar_constant

        // Inversion Operator
        if tkw.next_if_matches(|b| matches!(b, b'!' | b'~')) {
            tkw.skip_whitespace();
            let scalar_node = ScalarNode::consume(tkw)?;
            return Ok(Self {
                scalar_node,
                variant: TimingCheckConditionVariant::Inversion,
            });
        }

        let scalar_node = ScalarNode::consume(tkw)?;
        tkw.skip_whitespace();
        let operator = match tkw.peek_bytes::<3>() {
            [b'=', b'=', b'='] => {
                tkw.offset += 3;
                EqualityOperator::CaseEquality
            }
            [b'!', b'=', b'='] => {
                tkw.offset += 3;
                EqualityOperator::CaseInequality
            }
            [b'=', b'=', _] => {
                tkw.offset += 2;
                EqualityOperator::LogicalEquality
            }
            [b'!', b'=', _] => {
                tkw.offset += 2;
                EqualityOperator::LogicalInequality
            }
            _ => {
                return Ok(Self {
                    scalar_node,
                    variant: TimingCheckConditionVariant::Plain,
                });
            }
        };

        tkw.skip_whitespace();
        let constant = ScalarConstant::consume(tkw)?;
        Ok(Self {
            scalar_node,
            variant: TimingCheckConditionVariant::Equality(operator, constant),
        })
    }
}

impl<'a> Consume<'a> for ScalarNode<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 73)
        // scalar_node ::= scalar_port | scalar_net

        let hident = HierarchicalIdent::consume(tkw)?;
        tkw.skip_whitespace();
        let mut offset = None;
        if tkw.next_if_equals(b'[') {
            offset = Some(Integer::consume(tkw)?);
            tkw.expect_char(b']')?;
        }
        Ok(Self { hident, offset })
    }
}

impl<'a> Consume<'a> for TimingCheckDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 69)
        // tchk_def ::=
        //   setup_timing_check
        // | hold_timing_check
        // | setuphold_timing_check
        // | recovery_timing_check
        // | removal_timing_check
        // | recrem_timing_check
        // | skew_timing_check
        // | bidirectskew_timing_check
        // | width_timing_check
        // | period_timing_check
        // | nochange_timing_check

        tkw.skip_whitespace();
        let param = expect_peek_param(tkw)?;
        Ok(match param {
            "SETUP" => Self::Setup(SetupTimingCheck::consume(tkw)?),
            "HOLD" => Self::Hold(HoldTimingCheck::consume(tkw)?),
            "SETUPHOLD" => Self::SetupHold(SetupHoldTimingCheck::consume(tkw)?),
            "RECOVERY" => Self::Recovery(RecoveryTimingCheck::consume(tkw)?),
            "REMOVAL" => Self::Removal(RemovalTimingCheck::consume(tkw)?),
            "RECREM" => Self::Recrem(RecremTimingCheck::consume(tkw)?),
            "SKEW" => Self::Skew(SkewTimingCheck::consume(tkw)?),
            "BIDIRECTSKEW" => Self::BidirectSkew(BidirectSkewTimingCheck::consume(tkw)?),
            "WIDTH" => Self::Width(WidthTimingCheck::consume(tkw)?),
            "PERIOD" => Self::Period(PeriodTimingCheck::consume(tkw)?),
            "NOCHANGE" => Self::NoChange(NoChangeTimingCheck::consume(tkw)?),
            _ => {
                return Err(Box::new(SdfError {
                    line: tkw.line,
                    msg: format!("unknown timing check definition: {param}"),
                }));
            }
        })
    }
}

impl<'a> Consume<'a> for SetupTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 69)
        // setup_timing_check ::= ( SETUP port_tchk port_tchk value )

        parse_param_with(tkw, "SETUP", |tkw| {
            let fst = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let snd = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let value = Value::consume(tkw)?;
            Ok(Self { fst, snd, value })
        })
    }
}
impl<'a> Consume<'a> for HoldTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 69)
        // hold_timing_check ::= ( HOLD port_tchk port_tchk value )

        parse_param_with(tkw, "HOLD", |tkw| {
            let fst = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let snd = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let value = Value::consume(tkw)?;
            Ok(Self { fst, snd, value })
        })
    }
}
impl<'a> Consume<'a> for SetupHoldTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 69)
        // setuphold_timing_check ::=
        //   ( SETUPHOLD port_tchk port_tchk rvalue rvalue )
        // | ( SETUPHOLD port_spec port_spec rvalue rvalue [ scond ] [ ccond ] )
        parse_param_with(tkw, "SETUPHOLD", |tkw| {
            Ok(Self(SetupHoldRecremArgs::consume(tkw)?))
        })
    }
}
impl<'a> Consume<'a> for RecoveryTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 69)
        // recovery_timing_check ::= ( RECOVERY port_tchk port_tchk value )
        parse_param_with(tkw, "RECOVERY", |tkw| {
            let fst = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let snd = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let value = Value::consume(tkw)?;
            Ok(Self { fst, snd, value })
        })
    }
}
impl<'a> Consume<'a> for RemovalTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 69)
        // removal_timing_check ::= ( REMOVAL port_tchk port_tchk value )
        parse_param_with(tkw, "REMOVAL", |tkw| {
            let fst = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let snd = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let value = Value::consume(tkw)?;
            Ok(Self { fst, snd, value })
        })
    }
}
impl<'a> Consume<'a> for RecremTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        // recrem_timing_check ::=
        //   ( RECREM port_tchk port_tchk rvalue rvalue )
        // | ( RECREM port_spec port_spec rvalue rvalue [ scond ] [ ccond ] )
        parse_param_with(tkw, "RECREM", |tkw| {
            Ok(Self(SetupHoldRecremArgs::consume(tkw)?))
        })
    }
}
impl<'a> Consume<'a> for SkewTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        // skew_timing_check ::= ( SKEW port_tchk port_tchk rvalue )
        parse_param_with(tkw, "SKEW", |tkw| {
            let fst = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let snd = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let value = RValue::consume(tkw)?;
            Ok(Self { fst, snd, value })
        })
    }
}
impl<'a> Consume<'a> for BidirectSkewTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        // bidirectskew_timing_check ::= ( BIDIRECTSKEW port_tchk port_tchk value value )
        parse_param_with(tkw, "BIDIRECTSKEW", |tkw| {
            let fst = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let snd = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let fst_value = Value::consume(tkw)?;
            tkw.skip_whitespace();
            let snd_value = Value::consume(tkw)?;
            Ok(Self {
                fst,
                snd,
                fst_value,
                snd_value,
            })
        })
    }
}
impl<'a> Consume<'a> for WidthTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        // width_timing_check ::= ( WIDTH port_tchk value )
        parse_param_with(tkw, "WIDTH", |tkw| {
            let port_tchk = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let value = Value::consume(tkw)?;
            Ok(Self { port_tchk, value })
        })
    }
}
impl<'a> Consume<'a> for PeriodTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        // period_timing_check ::= ( PERIOD port_tchk value )
        parse_param_with(tkw, "PERIOD", |tkw| {
            let port_tchk = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let value = Value::consume(tkw)?;
            Ok(Self { port_tchk, value })
        })
    }
}
impl<'a> Consume<'a> for NoChangeTimingCheck<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        // nochange_timing_check ::= ( NOCHANGE port_tchk port_tchk rvalue rvalue )
        parse_param_with(tkw, "NOCHANGE", |tkw| {
            let fst = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let snd = PortTimingCheck::consume(tkw)?;
            tkw.skip_whitespace();
            let fst_rvalue = RValue::consume(tkw)?;
            tkw.skip_whitespace();
            let snd_rvalue = RValue::consume(tkw)?;
            Ok(Self {
                fst,
                snd,
                fst_rvalue,
                snd_rvalue,
            })
        })
    }
}

impl<'a> Consume<'a> for SCond<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        // scond ::= ( SCOND [ qstring ] timing_check_condition )
        parse_param_with(tkw, "SCOND", |tkw| {
            let qstring = if tkw.is_next_equal_to(b'"') {
                Some(QString::consume(tkw)?)
            } else {
                None
            };
            tkw.skip_whitespace();
            let timing_check_condition = TimingCheckCondition::consume(tkw)?;
            Ok(Self {
                qstring,
                timing_check_condition,
            })
        })
    }
}
impl<'a> Consume<'a> for CCond<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        // ccond ::= ( CCOND [ qstring ] timing_check_condition )
        parse_param_with(tkw, "CCOND", |tkw| {
            let qstring = if tkw.is_next_equal_to(b'"') {
                Some(QString::consume(tkw)?)
            } else {
                None
            };
            tkw.skip_whitespace();
            let timing_check_condition = TimingCheckCondition::consume(tkw)?;
            Ok(Self {
                qstring,
                timing_check_condition,
            })
        })
    }
}

impl<'a> Consume<'a> for SetupHoldRecremArgs<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // IEEE Std 1497-2001 (p. 70)
        //   ( RECREM|SETUPHOLD port_tchk port_tchk rvalue rvalue )
        // | ( RECREM|SETUPHOLD port_spec port_spec rvalue rvalue [ scond ] [ ccond ] )

        let fst = PortTimingCheck::consume(tkw)?;
        tkw.skip_whitespace();
        let snd = PortTimingCheck::consume(tkw)?;
        tkw.skip_whitespace();
        let fst_rvalue = RValue::consume(tkw)?;
        tkw.skip_whitespace();
        let snd_rvalue = RValue::consume(tkw)?;
        tkw.skip_whitespace();
        if tkw.is_next_equal_to(b'(') {
            let (PortTimingCheck::PortSpec(fst), PortTimingCheck::PortSpec(snd)) = (fst, snd)
            else {
                return Err(Box::new(SdfError {
                    line: tkw.line,
                    msg: "expected port spec here".to_string(),
                }));
            };

            let mut scond = None;
            let mut ccond = None;
            if peek_param(tkw) == Some("SCOND") {
                scond = Some(SCond::consume(tkw)?);
            }
            if peek_param(tkw) == Some("CCOND") {
                ccond = Some(CCond::consume(tkw)?);
            }

            return Ok(Self::Alternative(
                fst, snd, fst_rvalue, snd_rvalue, scond, ccond,
            ));
        }

        Ok(Self::Base(fst, snd, fst_rvalue, snd_rvalue))
    }
}
