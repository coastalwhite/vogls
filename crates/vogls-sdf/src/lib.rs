// - [ ] DelayFile
//   - [x] Header
//   - [ ] Cell
//     - [ ] CellType
//     - [ ] Instance
//     - [ ] Timing Spec
//       - [x] Delay
//          - [x] Absolute
//          - [x] Increment
//          - [x] Path Pulse
//          - [x] Path Pulse Procent
//          - [x] Components
//            - [x] DelValList
//              - [x] DelVal
//            - [x] DelDef
//              - [x] IOPATH
//                - [x] PortSpec
//                - [x] PortInstance
//              - [x] RETAIN
//                - [x] RetValList
//              - [x] COND
//                - [ ] conditional_port_expr
//              - [x] CONDELSE
//              - [x] PORT
//                - [x] port_instance
//              - [x] INTERCONNECT
//              - [x] NETDELAY
//                - [x] net_spec
//              - [x] DEVICE
//       - [x] Timing Check
//         - [x] TimingCheckDef
//           - [x] SETUP
//             - [x] port_tchk
//           - [x] HOLD
//           - [x] SETUPHOLD
//           - [x] RECOVERY
//           - [x] REMOVAL
//           - [x] RECREM
//             - [x] SCond
//             - [x] CCond
//           - [x] SKEW
//           - [x] BIDIRECTSKEW
//           - [x] WIDTH
//           - [x] PERIOD
//           - [x] NOCHANGE
//       - [ ] Timing Env
//          - [ ] TimingEnvDef
//            - [ ] Constraint
//              - [ ] PathConstraint
//              - [ ] PeriodConstraint
//              - [ ] Sum
//              - [ ] Diff
//              - [ ] SkewConstraint
//            - [ ] Timing Environment
//              - [ ] Arrival
//              - [ ] Departure
//              - [ ] Slack
//              - [ ] Waveform
//       - [ ] Label
//         - [ ] LabelDef
//         - [ ] Absolute
//         - [ ] Increment
use std::borrow::Cow;

mod error;
mod timing_check;

pub use error::*;
pub use timing_check::*;

pub trait Consume<'a>: Sized {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self>;
}

pub struct DelayFile<'a> {
    pub header: SdfHeader<'a>,
    pub cells: Vec<Cell<'a>>,
}

pub struct Cell<'a> {
    pub celltype: QString<'a>,
    pub instance: Instance<'a>,
    pub timing_specs: Vec<TimingSpec<'a>>,
}

pub enum Instance<'a> {
    Empty,
    Star,
    HierarchicalIdent(HierarchicalIdent<'a>),
}

pub struct HierarchicalIdent<'a> {
    pub fst: &'a str,
    pub next: Vec<(HierarchyDivider, &'a str)>,
}

pub enum TimingSpec<'a> {
    Delay(DelaySpec<'a>),
    TimingCheck(TimingCheckSpec<'a>),
    TimingEnv(TimingEnvSpec<'a>),
    Label(LabelSpec<'a>),
}

pub struct DelaySpec<'a> {
    pub items: Vec<DelayType<'a>>,
}
pub struct TimingEnvSpec<'a>(pub &'a str);
pub struct LabelSpec<'a>(pub &'a str);

pub enum DelayType<'a> {
    Absolute(AbsoluteDelayType<'a>),
    Increment(IncrementDelayType<'a>),
    PathPulse(PathPulseType<'a>),
    PathPulseProcent(PathPulsePercentType<'a>),
}

pub struct AbsoluteDelayType<'a> {
    pub defs: Vec<DelayDef<'a>>,
}
pub struct IncrementDelayType<'a> {
    pub defs: Vec<DelayDef<'a>>,
}
pub struct PathPulseType<'a> {
    pub input_output_path: Option<(PortInstance<'a>, PortInstance<'a>)>,
    pub values: Vec<Value<'a>>,
}
pub struct PathPulsePercentType<'a> {
    pub input_output_path: Option<(PortInstance<'a>, PortInstance<'a>)>,
    pub values: Vec<Value<'a>>,
}

pub enum DelayDef<'a> {
    IoPath(IoPathDef<'a>),
    Retain(RetainDef<'a>),
    Cond(CondDef<'a>),
    CondElse(CondElseDef<'a>),
    Port(PortDef<'a>),
    Interconnect(InterconnectDef<'a>),
    NetDelay(NetDelayDef<'a>),
    Device(DeviceDef<'a>),
}

pub struct RetainDef<'a>(pub RetValList<'a>);
pub struct IoPathDef<'a> {
    pub port_spec: PortSpec<'a>,
    pub port_instance: PortInstance<'a>,
    pub retain_defs: Vec<RetainDef<'a>>,
    pub delval_list: DelValList<'a>,
}
pub struct CondDef<'a> {
    pub qstring: Option<QString<'a>>,
    pub conditional_port_expr: Expression<'a>,
    pub io_path: IoPathDef<'a>,
}
pub struct CondElseDef<'a>(pub IoPathDef<'a>);
pub struct PortDef<'a> {
    pub port_instance: PortInstance<'a>,
    pub delval_list: DelValList<'a>,
}
pub struct InterconnectDef<'a> {
    pub from: PortInstance<'a>,
    pub to: PortInstance<'a>,
    pub delval_list: DelValList<'a>,
}
pub struct NetDelayDef<'a> {
    pub net_spec: PortInstance<'a>,
    pub delval_list: DelValList<'a>,
}
pub struct DeviceDef<'a> {
    pub port_instance: Option<PortInstance<'a>>,
    pub delval_list: DelValList<'a>,
}

// A merger of `conditional_port_expr` and `simple_expression`.
pub enum Expression<'a> {
    Parenthese(Box<Expression<'a>>),
    Unary(UnaryOp, Box<Expression<'a>>),
    UnaryPort(UnaryOp, Port<'a>),
    UnaryScalar(UnaryOp, ScalarConstant),
    Port(Port<'a>),
    Scalar(ScalarConstant),
    Ternary(
        Box<Expression<'a>>,
        Box<Expression<'a>>,
        Box<Expression<'a>>,
    ),
    Binary(BinaryOp, Box<Expression<'a>>, Box<Expression<'a>>),
    Concat(Vec<Expression<'a>>),
    Replicate(Box<Expression<'a>>, Vec<Expression<'a>>),
}

#[derive(Clone, Copy)]
pub enum ScalarConstant {
    L0,
    L1,
}

#[derive(Clone, Copy)]
pub enum UnaryOp {
    /// Arithmetic identity
    ArithmeticIdentity,
    /// Arithmetic negation
    ArithmeticNegation,
    /// Logical negation
    LogicalNegation,
    /// Bitwise unary negation
    BitwiseUnaryNegation,
    /// Reduction unary AND
    ReductionAnd,
    /// Reduction unary NAND
    ReductionNand,
    /// Reduction unary OR
    ReductionOr,
    /// Reduction unary NOR
    ReductionNor,
    /// Reduction unary XOR
    ReductionXor,
    /// Reduction unary NXOR
    ReductionXnor,
}
impl UnaryOp {
    fn from_2_bytes(bs: [u8; 2]) -> Option<Self> {
        Some(match bs[0] {
            b'+' => Self::ArithmeticIdentity,
            b'-' => Self::ArithmeticNegation,
            b'!' => Self::LogicalNegation,
            b'~' if bs[1] == b'&' => Self::ReductionNand,
            b'~' if bs[1] == b'|' => Self::ReductionNor,
            b'~' if bs[1] == b'^' => Self::ReductionXnor,
            b'~' => Self::BitwiseUnaryNegation,
            b'&' => Self::ReductionAnd,
            b'|' => Self::ReductionOr,
            b'^' if bs[1] == b'~' => Self::ReductionXnor,
            b'^' => Self::ReductionXor,
            _ => return None,
        })
    }

    fn binding_power(self) -> u8 {
        // @TODO: Correct
        match self {
            UnaryOp::ArithmeticIdentity => 1,
            UnaryOp::ArithmeticNegation => 1,
            UnaryOp::LogicalNegation => 1,
            UnaryOp::BitwiseUnaryNegation => 1,
            UnaryOp::ReductionAnd => 1,
            UnaryOp::ReductionNand => 1,
            UnaryOp::ReductionOr => 1,
            UnaryOp::ReductionNor => 1,
            UnaryOp::ReductionXor => 1,
            UnaryOp::ReductionXnor => 1,
        }
    }

    fn num_bytes(self) -> usize {
        match self {
            Self::ArithmeticIdentity
            | Self::ArithmeticNegation
            | Self::LogicalNegation
            | Self::BitwiseUnaryNegation
            | Self::ReductionAnd
            | Self::ReductionOr
            | Self::ReductionXor => 1,
            Self::ReductionNand | Self::ReductionNor | Self::ReductionXnor => 2,
        }
    }
}

#[derive(Clone, Copy)]
pub enum BinaryOp {
    /// Arithmetic sum
    ArithmeticSum,
    /// Arithmetic difference
    ArithmeticDifference,
    /// Arithmetic product
    ArithmeticProduct,
    /// Arithmetic quotient
    ArithmeticQuotient,
    /// Modulus
    Modulus,
    /// Logical equality
    LogicalEquality,
    /// Logical inequality
    LogicalInequality,
    /// Case equality
    CaseEquality,
    /// Case inequality
    CaseInequality,
    /// Logical AND
    LogicalAnd,
    /// Logical OR
    LogicalOr,
    /// Less than
    LessThan,
    /// Less than equal
    LessThanEqual,
    /// Greater than
    GreaterThan,
    /// Greater than equal
    GreaterThanEqual,
    /// Bit-wise binary AND
    BitwiseAnd,
    /// Bit-wise binary inclusive OR
    BitwiseOr,
    /// Bit-wise binary exclusive OR
    BitwiseXor,
    /// Bit-wise binary equivalence
    BitwiseEquiv,
    /// Right shift
    RightShift,
    /// Left shift
    LeftShift,
}
impl BinaryOp {
    fn from_3_bytes(bs: [u8; 3]) -> Option<BinaryOp> {
        Some(match (bs[0], bs[1], bs[2]) {
            (b'+', _, _) => Self::ArithmeticSum,
            (b'-', _, _) => Self::ArithmeticDifference,
            (b'*', _, _) => Self::ArithmeticProduct,
            (b'/', _, _) => Self::ArithmeticQuotient,
            (b'%', _, _) => Self::Modulus,
            (b'=', b'=', b'=') => Self::CaseEquality,
            (b'=', b'=', _) => Self::LogicalEquality,
            (b'!', b'=', _) => Self::LogicalInequality,

            (b'&', b'&', _) => Self::LogicalAnd,
            (b'|', b'|', _) => Self::LogicalOr,

            (b'~', b'^', _) | (b'^', b'~', _) => Self::BitwiseEquiv,
            (b'&', _, _) => Self::BitwiseAnd,
            (b'|', _, _) => Self::BitwiseOr,
            (b'^', _, _) => Self::BitwiseXor,

            (b'<', b'<', _) => Self::LeftShift,
            (b'>', b'>', _) => Self::RightShift,

            (b'<', b'=', _) => Self::LessThanEqual,
            (b'<', _, _) => Self::LessThan,
            (b'>', b'=', _) => Self::GreaterThanEqual,
            (b'>', _, _) => Self::GreaterThan,

            _ => return None,
        })
    }

    fn binding_power(self) -> (u8, u8) {
        // @TODO: fill
        (1, 1)
    }

    fn num_bytes(self) -> usize {
        match self {
            Self::ArithmeticSum
            | Self::ArithmeticDifference
            | Self::ArithmeticProduct
            | Self::ArithmeticQuotient
            | Self::Modulus
            | Self::BitwiseAnd
            | Self::BitwiseOr
            | Self::BitwiseXor
            | Self::LessThan
            | Self::GreaterThan => 1,
            Self::LogicalEquality
            | Self::LogicalInequality
            | Self::LogicalAnd
            | Self::LogicalOr
            | Self::BitwiseEquiv
            | Self::LeftShift
            | Self::RightShift
            | Self::LessThanEqual
            | Self::GreaterThanEqual => 2,
            Self::CaseEquality | Self::CaseInequality => 3,
        }
    }
}

impl<'a> Consume<'a> for Expression<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        enum StackItem<'a> {
            Paren,
            BracketS1,
            Concat(Vec<Expression<'a>>),
            Replication(Box<Expression<'a>>, Vec<Expression<'a>>),
            UnaryOp(UnaryOp),
            Binary(BinaryOp, Box<Expression<'a>>),
            TernaryS1(Box<Expression<'a>>),
            TernaryS2(Box<Expression<'a>>, Box<Expression<'a>>),
        }

        let mut exprs_sp = Vec::new();

        let mut min_bp: u8 = 0;
        let mut current: Expression<'a>;

        let result = 'outer: loop {
            macro_rules! deepen {
                ($item:expr, $bp:expr) => {{
                    exprs_sp.push(($item, min_bp));
                    min_bp = $bp;
                    continue 'outer;
                }};
            }

            current = {
                tkw.skip_whitespace();
                let Some(c) = tkw.peek_char() else {
                    return Err(Box::new(SdfError {
                        line: tkw.line,
                        msg: "missing expression".to_string(),
                    }));
                };
                match c {
                    // ( simple expression )
                    b'(' => {
                        tkw.offset += 1;
                        deepen!(StackItem::Paren, 0);
                    }

                    // unary_operator ( simple expression )
                    // unary_operator port
                    // unary_operator scalar_constant
                    _ if UnaryOp::from_2_bytes(tkw.peek_bytes::<2>()).is_some() => {
                        let op = UnaryOp::from_2_bytes(tkw.peek_bytes::<2>()).unwrap();
                        tkw.offset += op.num_bytes();
                        tkw.skip_whitespace();
                        match tkw.peek_char() {
                            Some(b'(') => {
                                tkw.offset += 1;
                                let bp = op.binding_power();
                                deepen!(StackItem::UnaryOp(op), bp);
                            }
                            Some(b'0' | b'1' | b'\'') => {
                                let sc = ScalarConstant::consume(tkw)?;
                                Self::UnaryScalar(op, sc)
                            }
                            Some(_) => {
                                let port = Port::consume(tkw)?;
                                Self::UnaryPort(op, port)
                            }
                            None => {
                                return Err(Box::new(SdfError {
                                    line: tkw.line,
                                    msg: "missing character".to_string(),
                                }));
                            }
                        }
                    }

                    // scalar_constant
                    b'0' | b'1' | b'\'' => {
                        let sc = ScalarConstant::consume(tkw)?;
                        Self::Scalar(sc)
                    }

                    // { simple_expression [ concat_expression ] }
                    // { simple_expression { simple_expression [ concat_expression ] } }
                    b'{' => {
                        tkw.offset += 1;
                        deepen!(StackItem::BracketS1, 0);
                    }

                    // port
                    _ => {
                        let port = Port::consume(tkw)?;
                        Self::Port(port)
                    }
                }
            };

            loop {
                #[expect(clippy::never_loop)]
                loop {
                    tkw.skip_whitespace();
                    let Some(c) = tkw.peek_char() else {
                        break;
                    };

                    // Ternary operator ( ... ? ... : ... )
                    if c == b'?' {
                        let (l_bp, r_bp) = (2, 1);

                        if l_bp < min_bp {
                            break;
                        }

                        tkw.offset += 1;
                        let condition = Box::new(current);
                        deepen!(StackItem::TernaryS1(condition), r_bp);
                    }

                    let Some(op) = BinaryOp::from_3_bytes(tkw.peek_bytes::<3>()) else {
                        break;
                    };
                    tkw.offset += op.num_bytes();

                    let (l_bp, r_bp) = op.binding_power();

                    if l_bp < min_bp {
                        break;
                    }

                    tkw.offset += 1;
                    let lhs = Box::new(current);
                    deepen!(StackItem::Binary(op, lhs), r_bp);
                }

                let Some((item, bp)) = exprs_sp.pop() else {
                    break 'outer current;
                };

                match item {
                    StackItem::Paren => {
                        tkw.skip_whitespace();
                        tkw.expect_char(b')')?;
                    }
                    StackItem::UnaryOp(op) => {
                        tkw.skip_whitespace();
                        tkw.expect_char(b')')?;
                        current = Expression::Unary(op, Box::new(current));
                    }
                    StackItem::Binary(op, lhs) => {
                        let rhs = Box::new(current);
                        current = Expression::Binary(op, lhs, rhs);
                    }
                    StackItem::BracketS1 => {
                        tkw.skip_whitespace();
                        match tkw.next_expect()? {
                            b'}' => current = Expression::Concat(vec![current]),
                            b',' => deepen!(StackItem::Concat(vec![current]), 0),
                            b'{' => {
                                deepen!(StackItem::Replication(Box::new(current), Vec::new()), 0)
                            }
                            _ => {
                                return Err(Box::new(SdfError {
                                    line: tkw.line,
                                    msg: "unexpected token".to_string(),
                                }));
                            }
                        }
                    }
                    StackItem::Concat(mut exprs) => {
                        exprs.push(current);
                        tkw.skip_whitespace();
                        match tkw.next_expect()? {
                            b'}' => current = Expression::Concat(exprs),
                            b',' => deepen!(StackItem::Concat(exprs), 0),
                            _ => {
                                return Err(Box::new(SdfError {
                                    line: tkw.line,
                                    msg: "unexpected token".to_string(),
                                }));
                            }
                        }
                    }
                    StackItem::Replication(sexpr, mut exprs) => {
                        exprs.push(current);
                        tkw.skip_whitespace();
                        match tkw.next_expect()? {
                            b'}' => current = Expression::Replicate(sexpr, exprs),
                            b',' => deepen!(StackItem::Replication(sexpr, exprs), 0),
                            _ => {
                                return Err(Box::new(SdfError {
                                    line: tkw.line,
                                    msg: "unexpected token".to_string(),
                                }));
                            }
                        }
                    }
                    StackItem::TernaryS1(condition) => {
                        tkw.skip_whitespace();
                        tkw.expect_char(b':')?;
                        let truthy = Box::new(current);
                        deepen!(StackItem::TernaryS2(condition, truthy), bp);
                    }
                    StackItem::TernaryS2(condition, truthy) => {
                        let falsy = Box::new(current);
                        current = Expression::Ternary(condition, truthy, falsy);
                    }
                }

                min_bp = bp;
            }
        };

        Ok(result)
    }
}

impl<'a> Consume<'a> for ScalarConstant {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        tkw.skip_whitespace();
        match tkw.next_expect()? {
            b'0' => return Ok(Self::L0),
            b'1' if tkw.next_if_equals(b'\'') => {}
            b'1' => return Ok(Self::L1),
            b'\'' => {}
            _ => {
                return Err(Box::new(SdfError {
                    line: tkw.line,
                    msg: "expected one of '0', '1' or '''".to_string(),
                }));
            }
        }

        tkw.expect_char_matches(|b| matches!(b, b'b' | b'B'))?;
        Ok(
            if tkw.expect_char_matches(|b| matches!(b, b'1' | b'0'))? == b'1' {
                Self::L1
            } else {
                Self::L0
            },
        )
    }
}

pub enum Version {
    V1_0,
    V2_0,
    V2_1,
    V3_0,
    V4_0,
}
pub enum HierarchyDivider {
    Dot,
    Slash,
}

pub struct SdfHeader<'a> {
    pub version: Version,
    pub design: Option<QString<'a>>,
    pub date: Option<QString<'a>>,
    pub vendor: Option<QString<'a>>,
    pub program_name: Option<QString<'a>>,
    pub program_version: Option<QString<'a>>,
    pub hierarchy_divider: Option<HierarchyDivider>,
    pub voltage: Option<SignedRealNumberOrRTriple<'a>>,
    pub process: Option<QString<'a>>,
    pub temperature: Option<SignedRealNumberOrRTriple<'a>>,
    pub timescale: Option<Timescale>,
}

pub struct Timescale {
    pub number: TimescaleNumber,
    pub unit: TimescaleUnit,
}

pub enum TimescaleNumber {
    N1,
    N10,
    N100,
    N1_0,
    N10_0,
    N100_0,
}
pub enum TimescaleUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
    Picoseconds,
    Femtoseconds,
}

pub struct TokenWalker<'a> {
    offset: usize,
    content: &'a str,
    line: u64,
}

pub struct Value<'a>(pub Option<RealNumberOrTriple<'a>>);
pub struct RValue<'a>(pub Option<SignedRealNumberOrRTriple<'a>>);
pub struct DelVal<'a> {
    pub delay: RValue<'a>,
    pub r_limit: Option<RValue<'a>>,
    pub e_limit: Option<RValue<'a>>,
}
pub enum DelValList<'a> {
    One(DelVal<'a>),
    Two([DelVal<'a>; 2]),
    Three([DelVal<'a>; 3]),
    Six([DelVal<'a>; 6]),
    Twelve([DelVal<'a>; 12]),
}

impl<'a> DelValList<'a> {
    pub fn as_slice(&self) -> &[DelVal<'a>] {
        match self {
            Self::One(dv) => std::slice::from_ref(dv),
            Self::Two(dv) => dv,
            Self::Three(dv) => dv,
            Self::Six(dv) => dv,
            Self::Twelve(dv) => dv,
        }
    }
}

pub enum RetValList<'a> {
    One(DelVal<'a>),
    Two(DelVal<'a>, DelVal<'a>),
    Three(DelVal<'a>, DelVal<'a>, DelVal<'a>),
}

pub enum PortSpec<'a> {
    Instance(PortInstance<'a>),
    Edge(PortEdge<'a>),
}
pub struct PortInstance<'a> {
    pub hident: Option<HierarchicalIdent<'a>>,
    pub port: Port<'a>,
}
pub struct Port<'a> {
    pub hident: HierarchicalIdent<'a>,
    pub b1: Option<Integer<'a>>,
    pub b2: Option<Integer<'a>>,
}

pub struct PortEdge<'a> {
    pub edge: EdgeIdentifier,
    pub instance: PortInstance<'a>,
}

pub enum EdgeIdentifier {
    Posedge,
    Negedge,
    L01,
    L10,
    L0Z,
    LZ1,
    L1Z,
    LZ0,
}

pub struct Integer<'a>(pub &'a str);
pub enum SignedRealNumberOrRTriple<'a> {
    SignedRealNumber(SignedRealNumber<'a>),
    RTriple(RTriple<'a>),
}
pub enum RealNumberOrTriple<'a> {
    RealNumber(RealNumber<'a>),
    Triple(Triple<'a>),
}

pub struct Checkpoint {
    offset: usize,
    line: u64,
}

impl<'a> TokenWalker<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            offset: 0,
            content,
            line: 1,
        }
    }

    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            offset: self.offset,
            line: self.line,
        }
    }
    pub fn restore_checkpoint(&mut self, checkpoint: Checkpoint) {
        self.offset = checkpoint.offset;
        self.line = checkpoint.line;
    }

    pub fn peek_content(&self) -> &str {
        let s = &self.content[self.offset..];
        let n = s.len();
        &s[..n.min(12)]
    }

    pub fn next_if_equals(&mut self, b: u8) -> bool {
        let is_equal = self.is_next_equal_to(b);
        self.offset += usize::from(is_equal);
        is_equal
    }
    pub fn next_if_matches(&mut self, mut f: impl FnMut(u8) -> bool) -> bool {
        if let Some(&b) = self.content.as_bytes().get(self.offset)
            && f(b)
        {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    pub fn is_next_equal_to(&self, b: u8) -> bool {
        self.content.as_bytes().get(self.offset).copied() == Some(b)
    }

    pub fn next_char(&mut self) -> Option<u8> {
        let c = self.content.as_bytes().get(self.offset)?;
        self.offset += 1;
        Some(*c)
    }

    pub fn peek_char(&mut self) -> Option<u8> {
        let c = self.content.as_bytes().get(self.offset)?;
        Some(*c)
    }

    pub fn does_next_char_match(&self, mut f: impl FnMut(u8) -> bool) -> bool {
        self.content
            .as_bytes()
            .get(self.offset)
            .is_some_and(|&b| f(b))
    }

    pub fn expect_char(&mut self, b: u8) -> SdfResult<()> {
        let Some(&found) = self.content.as_bytes().get(self.offset) else {
            return Err(Box::new(SdfError {
                line: self.line,
                msg: format!("expected '{}', but no token found.", char::from(b)),
            }));
        };
        if found != b {
            return Err(Box::new(SdfError {
                line: self.line,
                msg: format!(
                    "expected '{}', but found '{}'",
                    char::from(b),
                    char::from(found)
                ),
            }));
        }

        self.offset += 1;
        Ok(())
    }
    pub fn expect_char_matches(&mut self, mut f: impl FnMut(u8) -> bool) -> SdfResult<u8> {
        let Some(&b) = self.content.as_bytes().get(self.offset) else {
            return Err(Box::new(SdfError {
                line: self.line,
                msg: format!("expected char, found none"),
            }));
        };
        if f(b) {
            Ok(b)
        } else {
            return Err(Box::new(SdfError {
                line: self.line,
                msg: format!("expected char, found '{}'", char::from(b)),
            }));
        }
    }

    pub fn expect_ident(&mut self, s: &str) -> SdfResult<()> {
        let Some(next_ident) = self.next_ident() else {
            return Err(Box::new(SdfError {
                line: self.line,
                msg: format!("expected ident '{}', but found none", s,),
            }));
        };
        if next_ident != s {
            return Err(Box::new(SdfError {
                line: self.line,
                msg: format!("expected ident '{}', but found '{}'", s, next_ident),
            }));
        }

        Ok(())
    }

    pub fn next_ident(&mut self) -> Option<&'a str> {
        if !matches!(
            self.content.as_bytes().get(self.offset),
            Some(b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_')
        ) {
            return None;
        }

        let start = self.offset;

        self.offset += 1;
        // @TODO: Escaped characters
        self.skip_while(|b| matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_'));

        Some(&self.content[start..self.offset])
    }

    pub fn skip_while(&mut self, mut f: impl FnMut(u8) -> bool) {
        while let Some(&b) = self.content.as_bytes().get(self.offset)
            && f(b)
        {
            self.line += u64::from(b == b'\n');
            self.offset += 1;
        }
    }

    pub fn skip_whitespace(&mut self) {
        self.skip_while(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'));
    }

    fn next_expect(&mut self) -> SdfResult<u8> {
        let Some(v) = self.next_char() else {
            return Err(Box::new(SdfError {
                line: self.line,
                msg: "missing character".to_string(),
            }));
        };
        Ok(v)
    }

    fn peek_bytes<const N: usize>(&self) -> [u8; N] {
        let mut i = 0;
        let mut out = [0u8; N];
        while i < N
            && let Some(&b) = self.content.as_bytes().get(self.offset + i)
        {
            out[i] = b;
            i += 1;
        }
        out
    }
}

pub struct QString<'a>(pub Cow<'a, str>);
pub struct RealNumber<'a>(pub &'a str);
pub struct SignedRealNumber<'a>(pub &'a str);
pub struct RTriple<'a>(
    pub Option<SignedRealNumber<'a>>,
    pub Option<SignedRealNumber<'a>>,
    pub Option<SignedRealNumber<'a>>,
);
pub struct Triple<'a>(
    pub Option<RealNumber<'a>>,
    pub Option<RealNumber<'a>>,
    pub Option<RealNumber<'a>>,
);

impl<'a> Consume<'a> for QString<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        tkw.expect_char(b'"')?;
        let mut is_escaped = false;
        let offset = tkw.offset;
        tkw.skip_while(|b| {
            let skip = is_escaped || b != b'"';
            is_escaped = !is_escaped & (b == b'\\');
            skip
        });
        let s = &tkw.content[offset..tkw.offset];
        if tkw.next_char().is_none() {
            return Err(Box::new(SdfError {
                line: tkw.line,
                msg: format!("unclosed string quote"),
            }));
        }

        // @TODO: Replace escaped "
        Ok(Self(Cow::Borrowed(s)))
    }
}

impl<'a> Consume<'a> for RealNumber<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        // integer
        let start = tkw.offset;
        tkw.expect_char_matches(|b| b.is_ascii_digit())?;
        tkw.skip_while(|b| b.is_ascii_digit());

        fn skip_exponent<'a>(tkw: &mut TokenWalker<'a>) {
            // e [ sign ] integer
            let checkpoint = tkw.checkpoint();
            let has_e = tkw.next_if_equals(b'e');
            tkw.next_if_matches(|b| matches!(b, b'+' | b'-'));
            let has_integer = tkw.does_next_char_match(|b| b.is_ascii_digit());
            if !has_e || has_integer {
                tkw.restore_checkpoint(checkpoint);
                return;
            }
            tkw.skip_while(|b| b.is_ascii_digit());
        }

        // [ . integer ]
        let checkpoint = tkw.checkpoint();
        if !tkw.next_if_equals(b'.') || !tkw.does_next_char_match(|b| b.is_ascii_digit()) {
            tkw.restore_checkpoint(checkpoint);
            skip_exponent(tkw);
            return Ok(Self(&tkw.content[start..tkw.offset]));
        }
        tkw.offset += 1;
        tkw.skip_while(|b| b.is_ascii_digit());

        skip_exponent(tkw);

        Ok(Self(&tkw.content[start..tkw.offset]))
    }
}

impl<'a> Consume<'a> for SignedRealNumber<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let start = tkw.offset;
        tkw.next_if_matches(|b| matches!(b, b'+' | b'-'));
        RealNumber::consume(tkw)?;
        Ok(Self(&tkw.content[start..tkw.offset]))
    }
}

impl<'a> Consume<'a> for Triple<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let fst = if tkw.next_if_equals(b':') {
            None
        } else {
            let fst = RealNumber::consume(tkw)?;
            tkw.skip_whitespace();
            tkw.expect_char(b':')?;
            Some(fst)
        };

        tkw.skip_whitespace();
        let snd = if tkw.next_if_equals(b':') {
            None
        } else {
            let snd = RealNumber::consume(tkw)?;
            tkw.skip_whitespace();
            tkw.expect_char(b':')?;
            Some(snd)
        };

        tkw.skip_whitespace();
        let trd = if tkw.does_next_char_match(|b| b.is_ascii_digit()) {
            Some(RealNumber::consume(tkw)?)
        } else {
            None
        };

        if fst.is_none() & snd.is_none() & trd.is_none() {
            return Err(Box::new(SdfError {
                line: tkw.line,
                msg: format!("all three unset in triple"),
            }));
        }

        Ok(Self(fst, snd, trd))
    }
}
impl<'a> Consume<'a> for RTriple<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let fst = if tkw.next_if_equals(b':') {
            None
        } else {
            let fst = SignedRealNumber::consume(tkw)?;
            tkw.skip_whitespace();
            tkw.expect_char(b':')?;
            Some(fst)
        };

        tkw.skip_whitespace();
        let snd = if tkw.next_if_equals(b':') {
            None
        } else {
            let snd = SignedRealNumber::consume(tkw)?;
            tkw.skip_whitespace();
            tkw.expect_char(b':')?;
            Some(snd)
        };

        tkw.skip_whitespace();
        let trd = if tkw.does_next_char_match(|b| b.is_ascii_digit() | matches!(b, b'+' | b'-')) {
            Some(SignedRealNumber::consume(tkw)?)
        } else {
            None
        };

        if fst.is_none() & snd.is_none() & trd.is_none() {
            return Err(Box::new(SdfError {
                line: tkw.line,
                msg: format!("all three unset in rtriple"),
            }));
        }

        Ok(Self(fst, snd, trd))
    }
}

impl<'a> Consume<'a> for SignedRealNumberOrRTriple<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let checkpoint = tkw.checkpoint();
        Ok(if let Ok(n) = SignedRealNumber::consume(tkw) {
            tkw.skip_whitespace();
            if tkw.is_next_equal_to(b':') {
                tkw.restore_checkpoint(checkpoint);
                SignedRealNumberOrRTriple::RTriple(RTriple::consume(tkw)?)
            } else {
                SignedRealNumberOrRTriple::SignedRealNumber(n)
            }
        } else {
            tkw.restore_checkpoint(checkpoint);
            SignedRealNumberOrRTriple::RTriple(RTriple::consume(tkw)?)
        })
    }
}
impl<'a> Consume<'a> for RealNumberOrTriple<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let checkpoint = tkw.checkpoint();
        Ok(if let Ok(n) = RealNumber::consume(tkw) {
            tkw.skip_whitespace();
            if tkw.is_next_equal_to(b':') {
                tkw.restore_checkpoint(checkpoint);
                RealNumberOrTriple::Triple(Triple::consume(tkw)?)
            } else {
                RealNumberOrTriple::RealNumber(n)
            }
        } else {
            tkw.restore_checkpoint(checkpoint);
            RealNumberOrTriple::Triple(Triple::consume(tkw)?)
        })
    }
}

impl<'a> Consume<'a> for SdfHeader<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let version = parse_param_with(tkw, "SDFVERSION", |tkw| {
            // @NOTE: Best code ever.
            let version = QString::consume(tkw)?;
            let version = version.0.as_ref();
            match () {
                _ if version.contains("1.0") => Ok(Version::V1_0),
                _ if version.contains("2.0") => Ok(Version::V2_0),
                _ if version.contains("2.1") => Ok(Version::V2_1),
                _ if version.contains("3.0") => Ok(Version::V3_0),
                _ if version.contains("4.0") => Ok(Version::V4_0),
                _ => Err(Box::new(SdfError {
                    line: tkw.line,
                    msg: format!("unknown version: '{}'", version),
                })),
            }
        })?;

        macro_rules! opt_param {
            ($name:literal [qstring]) => {{
                opt_param!($name, {
                    tkw.skip_whitespace();
                    QString::consume(tkw)?
                })
            }};
            ($name:literal [rtriple]) => {{
                opt_param!($name, {
                    tkw.skip_whitespace();
                    SignedRealNumberOrRTriple::consume(tkw)?
                })
            }};
            ($name:literal, $expr:expr) => {{
                tkw.skip_whitespace();
                let checkpoint = tkw.checkpoint();
                if tkw.next_if_equals(b'(') {
                    tkw.skip_whitespace();
                    if tkw.next_ident() == Some($name) {
                        let result = $expr;
                        tkw.skip_whitespace();
                        tkw.expect_char(b')')?;
                        Some(result)
                    } else {
                        tkw.restore_checkpoint(checkpoint);
                        None
                    }
                } else {
                    tkw.restore_checkpoint(checkpoint);
                    None
                }
            }};
        }

        let design = opt_param!("DESIGN"[qstring]);
        let date = opt_param!("DATE"[qstring]);
        let vendor = opt_param!("VENDOR"[qstring]);
        let program_name = opt_param!("PROGRAM"[qstring]);
        let program_version = opt_param!("VERSION"[qstring]);
        let hierarchy_divider = opt_param!("DIVIDER", {
            tkw.skip_whitespace();
            match tkw.next_char() {
                Some(b'/') => HierarchyDivider::Slash,
                Some(b'.') => HierarchyDivider::Dot,
                _ => {
                    return Err(Box::new(SdfError {
                        line: tkw.line,
                        msg: format!("unknown divider"),
                    }));
                }
            }
        });
        let voltage = opt_param!("VOLTAGE"[rtriple]);
        let process = opt_param!("PROCESS"[qstring]);
        let temperature = opt_param!("TEMPERATURE"[rtriple]);
        let timescale = opt_param!("TIMESCALE", {
            // timescale_number ::= 1 | 10 | 100 | 1.0 | 10.0 | 100.0
            // timescale_unit ::= s | ms | us | ns | ps | fs
            tkw.skip_whitespace();
            let number = {
                let start = tkw.offset;
                tkw.skip_while(|b| b.is_ascii_digit() | (b == b'.'));
                match &tkw.content[start..tkw.offset] {
                    "1" => TimescaleNumber::N1,
                    "10" => TimescaleNumber::N10,
                    "100" => TimescaleNumber::N100,
                    "1.0" => TimescaleNumber::N1_0,
                    "10.0" => TimescaleNumber::N10_0,
                    "100.0" => TimescaleNumber::N100_0,
                    _ => {
                        return Err(Box::new(SdfError {
                            line: tkw.line,
                            msg: format!("unknown timescale number"),
                        }));
                    }
                }
            };
            tkw.skip_whitespace();
            let unit = {
                let start = tkw.offset;
                tkw.skip_while(|b| b.is_ascii_alphabetic());
                match &tkw.content[start..tkw.offset] {
                    "s" => TimescaleUnit::Seconds,
                    "ms" => TimescaleUnit::Milliseconds,
                    "us" => TimescaleUnit::Microseconds,
                    "ns" => TimescaleUnit::Nanoseconds,
                    "ps" => TimescaleUnit::Picoseconds,
                    "fs" => TimescaleUnit::Femtoseconds,
                    _ => {
                        return Err(Box::new(SdfError {
                            line: tkw.line,
                            msg: format!("unknown timescale unit"),
                        }));
                    }
                }
            };

            Timescale { number, unit }
        });

        Ok(Self {
            version,
            design,
            date,
            vendor,
            program_name,
            program_version,
            hierarchy_divider,
            voltage,
            process,
            temperature,
            timescale,
        })
    }
}

fn parse_param<'a, T: Consume<'a>>(tkw: &mut TokenWalker<'a>, name: &'static str) -> SdfResult<T> {
    parse_param_with(tkw, name, T::consume)
}
fn parse_param_with<'a, T>(
    tkw: &mut TokenWalker<'a>,
    name: &'static str,
    mut f: impl FnMut(&mut TokenWalker<'a>) -> SdfResult<T>,
) -> SdfResult<T> {
    tkw.skip_whitespace();
    tkw.expect_char(b'(')?;
    tkw.skip_whitespace();
    tkw.expect_ident(name)?;
    tkw.skip_whitespace();
    let value = f(tkw)?;
    tkw.skip_whitespace();
    tkw.expect_char(b')')?;
    Ok(value)
}
fn peek_param<'a>(tkw: &mut TokenWalker<'a>) -> Option<&'a str> {
    let checkpoint = tkw.checkpoint();
    if tkw.next_char() != Some(b'(') {
        return None;
    }
    tkw.skip_whitespace();
    let ident = tkw.next_ident();
    tkw.restore_checkpoint(checkpoint);
    ident
}
fn expect_peek_param<'a>(tkw: &mut TokenWalker<'a>) -> SdfResult<&'a str> {
    let checkpoint = tkw.checkpoint();
    tkw.expect_char(b'(')?;

    tkw.skip_whitespace();
    let Some(ident) = tkw.next_ident() else {
        return Err(Box::new(SdfError {
            line: tkw.line,
            msg: "expected identifier".to_string(),
        }));
    };
    tkw.restore_checkpoint(checkpoint);
    Ok(ident)
}

impl<'a> Consume<'a> for Cell<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        tkw.skip_whitespace();
        tkw.expect_char(b'(')?;

        tkw.skip_whitespace();
        tkw.expect_ident("CELL")?;

        let celltype = parse_param::<QString<'a>>(tkw, "CELLTYPE")?;
        let instance = parse_param::<Instance<'a>>(tkw, "INSTANCE")?;
        let mut timing_specs = Vec::new();

        loop {
            tkw.skip_whitespace();
            if tkw.is_next_equal_to(b')') {
                break;
            }

            let timing_spec = TimingSpec::consume(tkw)?;
            timing_specs.push(timing_spec);
        }

        tkw.skip_whitespace();
        tkw.expect_char(b')')?;

        Ok(Self {
            celltype,
            instance,
            timing_specs,
        })
    }
}

impl<'a> Consume<'a> for TimingSpec<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let checkpoint = tkw.checkpoint();
        tkw.skip_whitespace();
        tkw.expect_char(b'(')?;

        tkw.skip_whitespace();
        Ok(match tkw.next_ident() {
            Some("DELAY") => {
                tkw.restore_checkpoint(checkpoint);
                Self::Delay(DelaySpec::consume(tkw)?)
            }
            Some("TIMINGCHECK") => {
                tkw.restore_checkpoint(checkpoint);
                Self::TimingCheck(TimingCheckSpec::consume(tkw)?)
            }
            Some("TIMINGENV") => todo!(), //Self::TimingEnv(TimingEnvSpec::consume(tkw)?),
            Some("LABEL") => todo!(),     //Self::Label(LabelSpec::consume(tkw)?),
            Some(name) => {
                return Err(Box::new(SdfError {
                    line: tkw.line,
                    msg: format!("unexpected '{name}'"),
                }));
            }
            None => {
                return Err(Box::new(SdfError {
                    line: tkw.line,
                    msg: format!("expected ident"),
                }));
            }
        })
    }
}

impl<'a> Consume<'a> for Instance<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        if tkw.is_next_equal_to(b')') {
            Ok(Self::Empty)
        } else if tkw.next_if_equals(b'*') {
            Ok(Self::Star)
        } else {
            Ok(Self::HierarchicalIdent(HierarchicalIdent::consume(tkw)?))
        }
    }
}

impl<'a> Consume<'a> for HierarchicalIdent<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let fst = tkw.next_ident().ok_or_else(|| {
            Box::new(SdfError {
                line: tkw.line,
                msg: format!("expected ident"),
            })
        })?;
        let mut next = Vec::new();
        loop {
            tkw.skip_whitespace();
            let checkpoint = tkw.checkpoint();
            if tkw.next_if_matches(|b| matches!(b, b'.' | b'/'))
                && let _ = tkw.skip_whitespace()
                && let Some(ident) = tkw.next_ident()
            {
                let hchar = if tkw.content.as_bytes()[checkpoint.offset] == b'.' {
                    HierarchyDivider::Dot
                } else {
                    HierarchyDivider::Slash
                };
                next.push((hchar, ident));
            } else {
                tkw.restore_checkpoint(checkpoint);
                break;
            }
        }
        Ok(Self { fst, next })
    }
}

impl<'a> Consume<'a> for DelayFile<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let result = parse_param_with(tkw, "DELAYFILE", |tkw| {
            let header = SdfHeader::consume(tkw)?;
            let mut cells = Vec::new();

            loop {
                tkw.skip_whitespace();
                if !tkw.is_next_equal_to(b'(') {
                    break;
                }

                let cell = Cell::consume(tkw)?;
                cells.push(cell);
            }

            Ok(Self { header, cells })
        })?;

        tkw.skip_whitespace();
        if tkw.offset != tkw.content.len() {
            return Err(Box::new(SdfError {
                line: tkw.line,
                msg: format!("remaining token"),
            }));
        }

        Ok(result)
    }
}

impl<'a> Consume<'a> for RValue<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        tkw.expect_char(b'(')?;
        tkw.skip_whitespace();
        if tkw.next_if_equals(b')') {
            Ok(Self(None))
        } else {
            let value = SignedRealNumberOrRTriple::consume(tkw)?;
            tkw.skip_whitespace();
            tkw.expect_char(b')')?;
            Ok(Self(Some(value)))
        }
    }
}

impl<'a> Consume<'a> for Value<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        tkw.expect_char(b'(')?;
        tkw.skip_whitespace();
        if tkw.next_if_equals(b')') {
            Ok(Self(None))
        } else {
            let value = RealNumberOrTriple::consume(tkw)?;
            tkw.skip_whitespace();
            tkw.expect_char(b')')?;
            Ok(Self(Some(value)))
        }
    }
}

impl<'a> Consume<'a> for DelVal<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let checkpoint = tkw.checkpoint();
        tkw.expect_char(b'(')?;

        tkw.skip_whitespace();
        if tkw.is_next_equal_to(b'(') {
            let delay = RValue::consume(tkw)?;
            let r_limit = Some(RValue::consume(tkw)?);
            let mut e_limit = None;
            if tkw.is_next_equal_to(b'(') {
                e_limit = Some(RValue::consume(tkw)?);
            }

            Ok(Self {
                delay,
                r_limit,
                e_limit,
            })
        } else {
            tkw.restore_checkpoint(checkpoint);
            let delay = RValue::consume(tkw)?;
            Ok(Self {
                delay,
                r_limit: None,
                e_limit: None,
            })
        }
    }
}

impl<'a> Consume<'a> for DelValList<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let fst = DelVal::consume(tkw)?;

        tkw.skip_whitespace();
        if !tkw.is_next_equal_to(b'(') {
            return Ok(Self::One(fst));
        }
        let snd = DelVal::consume(tkw)?;

        tkw.skip_whitespace();
        if !tkw.is_next_equal_to(b'(') {
            return Ok(Self::Two([fst, snd]));
        }
        let trd = DelVal::consume(tkw)?;

        tkw.skip_whitespace();
        if !tkw.is_next_equal_to(b'(') {
            return Ok(Self::Three([fst, snd, trd]));
        }

        let four = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        let five = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        let six = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        if !tkw.is_next_equal_to(b'(') {
            return Ok(Self::Six([fst, snd, trd, four, five, six]));
        }

        let seven = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        let eight = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        let nine = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        let ten = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        let eleven = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        let twelve = DelVal::consume(tkw)?;
        Ok(Self::Twelve([
            fst, snd, trd, four, five, six, seven, eight, nine, ten, eleven, twelve,
        ]))
    }
}
impl<'a> Consume<'a> for RetValList<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let fst = DelVal::consume(tkw)?;
        if !tkw.next_if_equals(b'(') {
            return Ok(Self::One(fst));
        }
        let snd = DelVal::consume(tkw)?;
        if !tkw.next_if_equals(b'(') {
            return Ok(Self::Two(fst, snd));
        }
        let trd = DelVal::consume(tkw)?;
        Ok(Self::Three(fst, snd, trd))
    }
}

impl<'a> Consume<'a> for RetainDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param::<RetValList>(tkw, "RETAIN").map(Self)
    }
}
impl<'a> Consume<'a> for IoPathDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "IOPATH", |tkw| {
            tkw.skip_whitespace();
            let port_spec = PortSpec::consume(tkw)?;

            tkw.skip_whitespace();
            let port_instance = PortInstance::consume(tkw)?;

            let mut retain_defs = Vec::new();
            loop {
                tkw.skip_whitespace();
                let checkpoint = tkw.checkpoint();
                if !tkw.next_if_equals(b'(') {
                    break;
                }

                tkw.skip_whitespace();
                if tkw.next_ident() != Some("RETAIN") {
                    tkw.restore_checkpoint(checkpoint);
                    break;
                }
                tkw.restore_checkpoint(checkpoint);
                let def = RetainDef::consume(tkw)?;
                retain_defs.push(def);
            }

            tkw.skip_whitespace();
            let delval_list = DelValList::consume(tkw)?;

            Ok(Self {
                port_spec,
                port_instance,
                retain_defs,
                delval_list,
            })
        })
    }
}
impl<'a> Consume<'a> for CondDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "COND", |tkw| {
            let qstring = if tkw.is_next_equal_to(b'"') {
                Some(QString::consume(tkw)?)
            } else {
                None
            };
            tkw.skip_whitespace();
            let conditional_port_expr = Expression::consume(tkw)?;

            tkw.skip_whitespace();
            let io_path = IoPathDef::consume(tkw)?;
            Ok(Self {
                qstring,
                conditional_port_expr,
                io_path,
            })
        })
    }
}
impl<'a> Consume<'a> for CondElseDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param::<IoPathDef<'a>>(tkw, "CONDELSE").map(Self)
    }
}
impl<'a> Consume<'a> for PortDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "PORT", |tkw| {
            let port_instance = PortInstance::consume(tkw)?;
            tkw.skip_whitespace();
            let delval_list = DelValList::consume(tkw)?;
            Ok(Self {
                port_instance,
                delval_list,
            })
        })
    }
}
impl<'a> Consume<'a> for InterconnectDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "INTERCONNECT", |tkw| {
            tkw.skip_whitespace();
            let from = PortInstance::consume(tkw)?;
            tkw.skip_whitespace();
            let to = PortInstance::consume(tkw)?;
            tkw.skip_whitespace();
            let delval_list = DelValList::consume(tkw)?;
            Ok(Self {
                from,
                to,
                delval_list,
            })
        })
    }
}
impl<'a> Consume<'a> for NetDelayDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "NETDELAY", |tkw| {
            let net_spec = PortInstance::consume(tkw)?;
            tkw.skip_whitespace();
            let delval_list = DelValList::consume(tkw)?;
            Ok(Self {
                net_spec,
                delval_list,
            })
        })
    }
}
impl<'a> Consume<'a> for DeviceDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "DEVICE", |tkw| {
            let port_instance = if tkw.is_next_equal_to(b'(') {
                None
            } else {
                Some(PortInstance::consume(tkw)?)
            };
            tkw.skip_whitespace();
            let delval_list = DelValList::consume(tkw)?;
            Ok(Self {
                port_instance,
                delval_list,
            })
        })
    }
}

impl<'a> Consume<'a> for Integer<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        tkw.expect_char_matches(|b| b.is_ascii_digit())?;
        let start = tkw.offset - 1;
        tkw.skip_while(|b| b.is_ascii_digit());
        Ok(Self(&tkw.content[start..tkw.offset]))
    }
}

impl<'a> Consume<'a> for Port<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let hident = HierarchicalIdent::consume(tkw)?;
        let mut b1 = None;
        let mut b2 = None;
        if tkw.next_if_equals(b'[') {
            tkw.skip_whitespace();
            b1 = Some(Integer::consume(tkw)?);
            tkw.skip_whitespace();
            if tkw.next_if_equals(b':') {
                tkw.skip_whitespace();
                b2 = Some(Integer::consume(tkw)?);
                tkw.skip_whitespace();
            }
            tkw.expect_char(b']')?;
        }
        Ok(Self { hident, b1, b2 })
    }
}

impl<'a> Consume<'a> for PortInstance<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        let mut hident = HierarchicalIdent::consume(tkw)?;
        let mut b1 = None;
        let mut b2 = None;
        if tkw.next_if_equals(b'[') {
            tkw.skip_whitespace();
            b1 = Some(Integer::consume(tkw)?);
            tkw.skip_whitespace();
            if tkw.next_if_equals(b':') {
                tkw.skip_whitespace();
                b2 = Some(Integer::consume(tkw)?);
                tkw.skip_whitespace();
            }
            tkw.expect_char(b']')?;
        }
        Ok(match hident.next.pop() {
            None => Self {
                hident: None,
                port: Port { hident, b1, b2 },
            },
            Some((_, ident)) => Self {
                hident: Some(hident),
                port: Port {
                    hident: HierarchicalIdent {
                        fst: ident,
                        next: Vec::new(),
                    },
                    b1,
                    b2,
                },
            },
        })
    }
}
impl<'a> Consume<'a> for PortSpec<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        if tkw.is_next_equal_to(b'(') {
            PortEdge::consume(tkw).map(Self::Edge)
        } else {
            PortInstance::consume(tkw).map(Self::Instance)
        }
    }
}
impl<'a> Consume<'a> for PortEdge<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        tkw.expect_char(b'(')?;
        tkw.skip_whitespace();
        let edge = match tkw.next_ident() {
            Some("posedge") => EdgeIdentifier::Posedge,
            Some("negedge") => EdgeIdentifier::Negedge,
            Some("01") => EdgeIdentifier::L01,
            Some("10") => EdgeIdentifier::L10,
            Some("0z") => EdgeIdentifier::L0Z,
            Some("z1") => EdgeIdentifier::LZ1,
            Some("1z") => EdgeIdentifier::L1Z,
            Some("z0") => EdgeIdentifier::LZ0,
            _ => {
                return Err(Box::new(SdfError {
                    line: tkw.line,
                    msg: format!("unexpected edge ident"),
                }));
            }
        };

        tkw.skip_whitespace();
        let instance = PortInstance::consume(tkw)?;

        tkw.skip_whitespace();
        tkw.expect_char(b')')?;

        Ok(Self { edge, instance })
    }
}

impl<'a> Consume<'a> for DelaySpec<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "DELAY", |tkw| {
            let mut items = Vec::new();
            let value = DelayType::consume(tkw)?;
            items.push(value);

            loop {
                tkw.skip_whitespace();
                if tkw.is_next_equal_to(b')') {
                    break;
                }
                let item = DelayType::consume(tkw)?;
                items.push(item);
            }

            Ok(Self { items })
        })
    }
}

impl<'a> Consume<'a> for DelayDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        tkw.skip_whitespace();
        let checkpoint = tkw.checkpoint();
        tkw.expect_char(b'(')?;

        tkw.skip_whitespace();
        let ident = tkw.next_ident();
        tkw.restore_checkpoint(checkpoint);
        Ok(match ident {
            Some("IOPATH") => Self::IoPath(IoPathDef::consume(tkw)?),
            Some("RETAIN") => Self::Retain(RetainDef::consume(tkw)?),
            Some("COND") => Self::Cond(CondDef::consume(tkw)?),
            Some("CONDELSE") => Self::CondElse(CondElseDef::consume(tkw)?),
            Some("PORT") => Self::Port(PortDef::consume(tkw)?),
            Some("INTERCONNECT") => Self::Interconnect(InterconnectDef::consume(tkw)?),
            Some("NETDELAY") => Self::NetDelay(NetDelayDef::consume(tkw)?),
            Some("DEVICE") => Self::Device(DeviceDef::consume(tkw)?),
            _ => {
                return Err(Box::new(SdfError {
                    line: tkw.line,
                    msg: format!("unknown delay def"),
                }));
            }
        })
    }
}

impl<'a> Consume<'a> for DelayType<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        tkw.skip_whitespace();
        let checkpoint = tkw.checkpoint();
        tkw.expect_char(b'(')?;

        tkw.skip_whitespace();
        let ident = tkw.next_ident();
        tkw.restore_checkpoint(checkpoint);
        Ok(match ident {
            Some("ABSOLUTE") => Self::Absolute(AbsoluteDelayType::consume(tkw)?),
            Some("INCREMENT") => Self::Increment(IncrementDelayType::consume(tkw)?),
            Some("PATHPULSE") => Self::PathPulse(PathPulseType::consume(tkw)?),
            Some("PATHPULSEPERCENT") => Self::PathPulseProcent(PathPulsePercentType::consume(tkw)?),
            _ => {
                return Err(Box::new(SdfError {
                    line: tkw.line,
                    msg: format!("unknown delay type"),
                }));
            }
        })
    }
}

impl<'a> Consume<'a> for AbsoluteDelayType<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "ABSOLUTE", |tkw| {
            let mut defs = Vec::new();
            let def = DelayDef::consume(tkw)?;
            defs.push(def);

            loop {
                tkw.skip_whitespace();
                if tkw.is_next_equal_to(b')') {
                    break;
                }
                let def = DelayDef::consume(tkw)?;
                defs.push(def);
            }

            Ok(Self { defs })
        })
    }
}
impl<'a> Consume<'a> for IncrementDelayType<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "INCREMENT", |tkw| {
            let mut defs = Vec::new();
            let def = DelayDef::consume(tkw)?;
            defs.push(def);

            loop {
                tkw.skip_whitespace();
                if tkw.is_next_equal_to(b')') {
                    break;
                }
                let def = DelayDef::consume(tkw)?;
                defs.push(def);
            }

            Ok(Self { defs })
        })
    }
}
impl<'a> Consume<'a> for PathPulseType<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "PATHPULSE", |tkw| {
            let input_output_path = if tkw.is_next_equal_to(b'(') {
                None
            } else {
                let from = PortInstance::consume(tkw)?;
                tkw.skip_whitespace();
                let to = PortInstance::consume(tkw)?;
                Some((from, to))
            };

            let mut values = Vec::new();
            let value = Value::consume(tkw)?;
            values.push(value);

            loop {
                tkw.skip_whitespace();
                if tkw.is_next_equal_to(b')') {
                    break;
                }
                let value = Value::consume(tkw)?;
                values.push(value);
            }

            Ok(Self {
                input_output_path,
                values,
            })
        })
    }
}
impl<'a> Consume<'a> for PathPulsePercentType<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> SdfResult<Self> {
        parse_param_with(tkw, "PATHPULSEPERCENT", |tkw| {
            let input_output_path = if tkw.is_next_equal_to(b'(') {
                None
            } else {
                let from = PortInstance::consume(tkw)?;
                tkw.skip_whitespace();
                let to = PortInstance::consume(tkw)?;
                Some((from, to))
            };

            let mut values = Vec::new();
            let value = Value::consume(tkw)?;
            values.push(value);

            loop {
                tkw.skip_whitespace();
                if tkw.is_next_equal_to(b')') {
                    break;
                }
                let value = Value::consume(tkw)?;
                values.push(value);
            }

            Ok(Self {
                input_output_path,
                values,
            })
        })
    }
}
