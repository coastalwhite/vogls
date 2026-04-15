// - [ ] DelayFile
//   - [x] Header
//   - [ ] Cell
//     - [ ] CellType
//     - [ ] Instance
//     - [ ] Timing Spec
//       - [ ] Delay
//          - [ ] Absolute
//          - [ ] Increment
//          - [ ] Path Pulse
//          - [ ] Path Pulse Procent
//          - [ ] Components
//            - [x] DelValList
//              - [x] DelVal
//            - [ ] DelDef
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
//       - [ ] Timing Check
//         - [ ] TimingCheckDef
//           - [ ] SETUP
//             - [ ] port_tchk
//           - [ ] HOLD
//           - [ ] SETUPHOLD
//           - [ ] RECOVERY
//           - [ ] REMOVAL
//           - [ ] RECREM
//             - [ ] SCond
//             - [ ] CCond
//           - [ ] SKEW
//           - [ ] BIDIRECTSKEW
//           - [ ] WIDTH
//           - [ ] PERIOD
//           - [ ] NOCHANGE
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
use std::fmt;

pub mod tokenizer;

#[derive(Debug)]
pub struct Error {
    line: u64,
    msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}
impl std::error::Error for Error {}

pub struct DelayFile<'a> {
    header: SdfHeader<'a>,
    cells: Vec<Cell<'a>>,
}

pub struct Cell<'a> {
    celltype: QString<'a>,
    instance: Instance<'a>,
    timing_specs: Vec<TimingSpec<'a>>,
}

pub enum Instance<'a> {
    Empty,
    Star,
    HierarchicalIdent(HierarchicalIdent<'a>),
}

pub struct HierarchicalIdent<'a> {
    fst: &'a str,
    next: Vec<(HierarchyDivider, &'a str)>,
}

pub enum TimingSpec<'a> {
    Delay(DelaySpec<'a>),
    TimingCheck(TimingCheckSpec<'a>),
    TimingEnv(TimingEnvSpec<'a>),
    Label(LabelSpec<'a>),
}

pub struct DelaySpec<'a> {
    items: Vec<DelayType<'a>>,
}
pub struct TimingCheckSpec<'a>(&'a str);
pub struct TimingEnvSpec<'a>(&'a str);
pub struct LabelSpec<'a>(&'a str);

pub enum DelayType<'a> {
    Absolute(AbsoluteDelayType<'a>),
    Increment(IncrementDelayType<'a>),
    PathPulse(PathPulseType<'a>),
    PathPulseProcent(PathPulsePercentType<'a>),
}

pub struct AbsoluteDelayType<'a> {
    defs: Vec<DelayDef<'a>>,
}
pub struct IncrementDelayType<'a> {
    defs: Vec<DelayDef<'a>>,
}
pub struct PathPulseType<'a> {
    input_output_path: Option<(PortInstance<'a>, PortInstance<'a>)>,
    values: Vec<Value<'a>>,
}
pub struct PathPulsePercentType<'a> {
    input_output_path: Option<(PortInstance<'a>, PortInstance<'a>)>,
    values: Vec<Value<'a>>,
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

pub struct RetainDef<'a>(RetValList<'a>);
pub struct IoPathDef<'a> {
    port_spec: PortSpec<'a>,
    port_instance: PortInstance<'a>,
    retain_defs: Vec<RetainDef<'a>>,
    delval_list: DelValList<'a>,
}
pub struct CondDef<'a> {
    qstring: Option<QString<'a>>,
    conditional_port_expr: CondPortExpr<'a>,
    io_path: IoPathDef<'a>,
}
pub struct CondElseDef<'a>(IoPathDef<'a>);
pub struct PortDef<'a> {
    port_instance: PortInstance<'a>,
    delval_list: DelValList<'a>,
}
pub struct InterconnectDef<'a> {
    from: PortInstance<'a>,
    to: PortInstance<'a>,
    delval_list: DelValList<'a>,
}
pub struct NetDelayDef<'a> {
    net_spec: PortInstance<'a>,
    delval_list: DelValList<'a>,
}
pub struct DeviceDef<'a> {
    port_instance: Option<PortInstance<'a>>,
    delval_list: DelValList<'a>,
}

pub enum CondPortExpr<'a> {
    Parenthese(Box<CondPortExpr<'a>>),
    Simple(Box<SimpleExpression<'a>>),
    Unary(UnaryOp, Box<SimpleExpression<'a>>),
    Binary(BinaryOp, Box<CondPortExpr<'a>>, Box<CondPortExpr<'a>>),
}

pub enum SimpleExpression<'a> {
    Parenthese(Box<SimpleExpression<'a>>),
    Unary(UnaryOp, Box<SimpleExpression<'a>>),
    UnaryPort(UnaryOp, Port<'a>),
    UnaryScalar(UnaryOp, ScalarConstant),
    Port(Port<'a>),
    Scalar(ScalarConstant),
    Ternary(
        Box<SimpleExpression<'a>>,
        Box<SimpleExpression<'a>>,
        Box<SimpleExpression<'a>>,
    ),
    Concat(Box<SimpleExpression<'a>>, Option<Box<SimpleExpression<'a>>>),
    DoubleConcat(
        Box<SimpleExpression<'a>>,
        Box<SimpleExpression<'a>>,
        Option<Box<SimpleExpression<'a>>>,
    ),
}

pub enum ScalarConstant {
    L0,
    L1,
}

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

impl<'a> Consume<'a> for SimpleExpression<'a> {
    fn consume(
        tkw: &mut TokenWalker<'a>,
    ) -> Result<Self, Box<Error>> {
        use Token as T;

        let mut exprs_sp = Vec::new();

        let mut min_bp: u8 = 0;
        let mut current: (Expr, TokenRange);

        let result = 'outer: loop {
            macro_rules! deepen {
                ($item:expr, $bp:expr, $span:expr) => {{
                    sc.exprs_sp.push(($item, min_bp, $span));
                    min_bp = $bp;
                    continue 'outer;
                }};
            }

            let token = tkw.try_get(tkw.offset, diagnostics.as_deref_mut())?;
            let span = TokenRange {
                start: tkw.offset,
                end: tkw.offset + 1,
            };
            current = {
                match token.kind {
                    T::Ident => {
                        let ident = HIdent::consume(
                            tkw,
                            // We cannot reuse the exprs_sp
                            &mut ParserScratches {
                                udps: VgHashSet::default(),
                                exprs_sp: Vec::new(),
                            },
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?;

                        if tkw.next_if_equals(T::LeftBrace) {
                            deepen!(StackItem::Brace(ident, Vec::new(), Vec::new()), 0, span)
                        } else if tkw.next_if_equals(T::LeftParen) {
                            deepen!(StackItem::FnCall(ident, Vec::new(), Vec::new()), 0, span)
                        } else {
                            (Expr::Ident(ident, AstIdRange::default(), None), span)
                        }
                    }
                    T::DollarIdent => {
                        let ident = item_parse::<SystemTaskIdentifier>(
                            tkw,
                            sc,
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?;

                        if tkw.next_if_equals(T::LeftParen) {
                            if tkw.next_if_equals(T::RightParen) {
                                (
                                    Expr::SystemFunctionCall(ident, Some(AstIdRange::default())),
                                    span,
                                )
                            } else {
                                deepen!(
                                    StackItem::SystemFnCall(ident, Vec::new(), Vec::new()),
                                    0,
                                    span
                                )
                            }
                        } else {
                            (Expr::SystemFunctionCall(ident, None), span)
                        }
                    }
                    T::Decimal => (
                        Expr::Decimal(
                            item_parse::<DecimalRef>(
                                tkw,
                                sc,
                                arenas,
                                ast,
                                diagnostics.as_deref_mut(),
                            )?
                            .item,
                        ),
                        span,
                    ),
                    T::Number => (
                        Expr::Sized(item_parse::<SizedNumberRef>(
                            tkw,
                            sc,
                            arenas,
                            ast,
                            diagnostics.as_deref_mut(),
                        )?),
                        span,
                    ),
                    T::String => (
                        Expr::String(
                            item_parse::<StringRef>(
                                tkw,
                                sc,
                                arenas,
                                ast,
                                diagnostics.as_deref_mut(),
                            )?
                            .item,
                        ),
                        span,
                    ),
                    T::LeftBracket => {
                        tkw.offset += 1;
                        deepen!(StackItem::Bracket, 0, span)
                    }
                    T::LeftParen => {
                        tkw.offset += 1;
                        deepen!(StackItem::Paren, 0, span)
                    }
                    t => {
                        let t = *t;
                        tkw.next();
                        let (r_bp, op) = token_to_prefix_op(t).ok_or_else(|| {
                            if let Some(diagnostics) = diagnostics.as_deref_mut() {
                                diagnostics
                                    .errors
                                    .push((span, ParseErrorReason::UnexpectedToken(t)));
                            }
                            ()
                        })?;
                        deepen!(StackItem::Unary(op), r_bp, span);
                    }
                }
            };

            loop {
                loop {
                    let Some(peeked) = tkw.get(tkw.offset) else {
                        break;
                    };

                    // Ternary operator ( ... ? ... : ... )
                    if *peeked.kind == T::QuestionMark {
                        let (l_bp, r_bp) = (2, 1);

                        if l_bp < min_bp {
                            break;
                        }

                        tkw.offset += 1;
                        let span = current.1;
                        let condition = push(arenas, ast, current.0, current.1);
                        deepen!(StackItem::TernaryS1(condition), r_bp, span);
                    }

                    let Some((l_bp, r_bp, op)) = token_to_binary_op(*peeked.kind) else {
                        break;
                    };

                    if l_bp < min_bp {
                        break;
                    }

                    tkw.offset += 1;
                    let span = current.1;
                    let lhs = push(arenas, ast, current.0, current.1);
                    deepen!(StackItem::Binary(op, lhs), r_bp, span);
                }

                let Some((item, bp, loc)) = sc.exprs_sp.pop() else {
                    break 'outer current;
                };

                let location = TokenRange {
                    start: loc.start,
                    end: current.1.end,
                };

                match item {
                    StackItem::Paren => {
                        tkw.next_expect(T::RightParen, diagnostics.as_deref_mut())?;
                    }
                    StackItem::Bracket => match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                        T::RightBracket => {
                            let expr = push(arenas, ast, current.0, current.1);
                            let expr = AstIdRange::single(expr);
                            current = (Expr::Concatenation(expr), location);
                        }
                        T::Comma => {
                            deepen!(
                                StackItem::Concatenation(vec![current.0], vec![current.1]),
                                0,
                                loc
                            );
                        }
                        T::LeftBracket => {
                            let expr = push(arenas, ast, current.0, current.1);
                            let expr = expr.into_constant();
                            deepen!(StackItem::Replication(expr, Vec::new(), Vec::new()), 0, loc);
                        }
                        t => {
                            diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                            return Err(());
                        }
                    },
                    StackItem::Concatenation(mut exprs, mut trs) => {
                        exprs.push(current.0);
                        trs.push(current.1);
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightBracket => {
                                let exprs = push_range(arenas, ast, exprs, trs);
                                current = (Expr::Concatenation(exprs), location);
                            }
                            T::Comma => {
                                deepen!(StackItem::Concatenation(exprs, trs), 0, loc);
                            }
                            t => {
                                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                                return Err(());
                            }
                        }
                    }
                    StackItem::Replication(constant_expr, mut exprs, mut trs) => {
                        exprs.push(current.0);
                        trs.push(current.1);
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightBracket => {
                                tkw.next_expect(T::RightBracket, diagnostics.as_deref_mut())?;
                                let exprs = push_range(arenas, ast, exprs, trs);
                                current = (
                                    Expr::Replication(Replication {
                                        constant_expr,
                                        exprs,
                                    }),
                                    location,
                                );
                            }
                            T::Comma => {
                                deepen!(StackItem::Replication(constant_expr, exprs, trs), 0, loc);
                            }
                            t => {
                                diagnostics.map(|d| d.unexpected_token(tkw.offset, t));
                                return Err(());
                            }
                        }
                    }
                    StackItem::Brace(ident, mut current_braced, mut current_trs) => {
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightBrace => {
                                current_braced.push(current.0);
                                current_trs.push(current.1);
                                if tkw.next_if_equals(T::LeftBrace) {
                                    deepen!(
                                        StackItem::Brace(ident, current_braced, current_trs),
                                        0,
                                        loc
                                    );
                                } else {
                                    let braced =
                                        push_range(arenas, ast, current_braced, current_trs);
                                    current = (Expr::Ident(ident, braced, None), location)
                                }
                            }
                            T::Colon => {
                                let exprs = push_range(arenas, ast, current_braced, current_trs);
                                let braced = push(arenas, ast, current.0, current.1);
                                deepen!(
                                    StackItem::BraceS2(ident, exprs, braced, BraceVariant::MsbLsb),
                                    0,
                                    loc
                                );
                            }
                            T::PlusColon => {
                                let exprs = push_range(arenas, ast, current_braced, current_trs);
                                let braced = push(arenas, ast, current.0, current.1);
                                deepen!(
                                    StackItem::BraceS2(
                                        ident,
                                        exprs,
                                        braced,
                                        BraceVariant::BasePlus
                                    ),
                                    0,
                                    loc
                                );
                            }
                            T::MinusColon => {
                                let exprs = push_range(arenas, ast, current_braced, current_trs);
                                let braced = push(arenas, ast, current.0, current.1);
                                deepen!(
                                    StackItem::BraceS2(
                                        ident,
                                        exprs,
                                        braced,
                                        BraceVariant::BaseMinus
                                    ),
                                    0,
                                    loc
                                );
                            }
                            t => {
                                diagnostics.map(|d| d.unexpected_token(tkw.offset - 1, t));
                                return Err(());
                            }
                        }
                    }
                    StackItem::BraceS2(subject, exprs, lhs, variant) => {
                        let rhs = push(arenas, ast, current.0, current.1);
                        let bit_slice = match variant {
                            BraceVariant::MsbLsb => {
                                BitSlice::MsbLsb(lhs.into_constant(), rhs.into_constant())
                            }
                            BraceVariant::BasePlus => BitSlice::PlusWidth(lhs, rhs.into_constant()),
                            BraceVariant::BaseMinus => {
                                BitSlice::MinusWidth(lhs, rhs.into_constant())
                            }
                        };
                        tkw.next_expect(T::RightBrace, diagnostics.as_deref_mut())?;
                        current = (Expr::Ident(subject, exprs, Some(bit_slice)), location);
                    }
                    StackItem::SystemFnCall(ident, mut params, mut trs) => {
                        params.push(current.0);
                        trs.push(current.1);
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightParen => {
                                let params = push_range(arenas, ast, params, trs);
                                current = (Expr::SystemFunctionCall(ident, Some(params)), location);
                            }
                            T::Comma => {
                                deepen!(StackItem::SystemFnCall(ident, params, trs), 0, location)
                            }
                            t => {
                                diagnostics.map(|d| d.unexpected_token(tkw.offset - 1, t));
                                return Err(());
                            }
                        }
                    }
                    StackItem::FnCall(ident, mut params, mut trs) => {
                        params.push(current.0);
                        trs.push(current.1);
                        match *tkw.try_next(diagnostics.as_deref_mut())?.kind {
                            T::RightParen => {
                                let params = push_range(arenas, ast, params, trs);
                                current = (Expr::FunctionCall(ident, params), location);
                            }
                            T::Comma => {
                                deepen!(StackItem::FnCall(ident, params, trs), 0, location)
                            }
                            t => {
                                diagnostics.map(|d| d.unexpected_token(tkw.offset - 1, t));
                                return Err(());
                            }
                        }
                    }
                    StackItem::Unary(op) => {
                        let subexpr = push(arenas, ast, current.0, current.1);
                        current = (Expr::Unary(op, subexpr), location)
                    }
                    StackItem::Binary(op, lhs) => {
                        let rhs = push(arenas, ast, current.0, current.1);
                        current = (Expr::Binary(op, lhs, rhs), location)
                    }
                    StackItem::TernaryS1(condition) => {
                        tkw.next_expect(T::Colon, diagnostics.as_deref_mut())?;
                        let truthy = push(arenas, ast, current.0, current.1);
                        deepen!(StackItem::TernaryS2(condition, truthy), bp, loc);
                    }
                    StackItem::TernaryS2(condition, truthy) => {
                        let falsy = push(arenas, ast, current.0, current.1);
                        current = (Expr::Ternary(condition, truthy, falsy), location)
                    }
                }

                min_bp = bp;
            }
        };

        Ok(result.0)
    }
}

enum Version {
    V1_0,
    V2_0,
    V2_1,
    V3_0,
    V4_0,
}
enum HierarchyDivider {
    Dot,
    Slash,
}

pub struct SdfHeader<'a> {
    version: Version,
    design: Option<QString<'a>>,
    date: Option<QString<'a>>,
    vendor: Option<QString<'a>>,
    program_name: Option<QString<'a>>,
    program_version: Option<QString<'a>>,
    hierarchy_divider: Option<HierarchyDivider>,
    voltage: Option<SignedRealNumberOrRTriple<'a>>,
    process: Option<QString<'a>>,
    temperature: Option<SignedRealNumberOrRTriple<'a>>,
    timescale: Option<Timescale>,
}

pub struct Timescale {
    number: TimescaleNumber,
    unit: TimescaleUnit,
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

pub struct Value<'a>(Option<RealNumberOrTriple<'a>>);
pub struct RValue<'a>(Option<SignedRealNumberOrRTriple<'a>>);
pub struct DelVal<'a> {
    delay: RValue<'a>,
    r_limit: Option<RValue<'a>>,
    e_limit: Option<RValue<'a>>,
}
pub enum DelValList<'a> {
    One(DelVal<'a>),
    Two(DelVal<'a>, DelVal<'a>),
    Three(DelVal<'a>, DelVal<'a>, DelVal<'a>),
    Six(
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
    ),
    Twelve(
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
        DelVal<'a>,
    ),
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
    hident: Option<HierarchicalIdent<'a>>,
    port: Port<'a>,
}
pub struct Port<'a> {
    hident: HierarchicalIdent<'a>,
    b1: Option<Integer<'a>>,
    b2: Option<Integer<'a>>,
}

pub struct PortEdge<'a> {
    edge: EdgeIdentifier,
    instance: PortInstance<'a>,
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

pub struct Integer<'a>(&'a str);
enum SignedRealNumberOrRTriple<'a> {
    SignedRealNumber(SignedRealNumber<'a>),
    RTriple(RTriple<'a>),
}
enum RealNumberOrTriple<'a> {
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

    pub fn does_next_char_match(&self, mut f: impl FnMut(u8) -> bool) -> bool {
        self.content
            .as_bytes()
            .get(self.offset)
            .is_some_and(|&b| f(b))
    }

    pub fn expect_char(&mut self, b: u8) -> Result<(), Box<Error>> {
        let Some(&found) = self.content.as_bytes().get(self.offset) else {
            return Err(Box::new(Error {
                line: self.line,
                msg: format!("expected '{}', but no token found.", char::from(b)),
            }));
        };
        if found != b {
            return Err(Box::new(Error {
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
    pub fn expect_char_matches(&mut self, mut f: impl FnMut(u8) -> bool) -> Result<(), Box<Error>> {
        let Some(&b) = self.content.as_bytes().get(self.offset) else {
            return Err(Box::new(Error {
                line: self.line,
                msg: format!("expected char, found none"),
            }));
        };
        if f(b) {
            Ok(())
        } else {
            return Err(Box::new(Error {
                line: self.line,
                msg: format!("expected char, found '{}'", char::from(b)),
            }));
        }
    }

    pub fn expect_ident(&mut self, s: &str) -> Result<(), Box<Error>> {
        let Some(next_ident) = self.next_ident() else {
            return Err(Box::new(Error {
                line: self.line,
                msg: format!("expected ident '{}', but found none", s,),
            }));
        };
        if next_ident != s {
            return Err(Box::new(Error {
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
}

pub trait Consume<'a>: Sized {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>>;
}

pub struct QString<'a>(Cow<'a, str>);
pub struct RealNumber<'a>(&'a str);
pub struct SignedRealNumber<'a>(&'a str);
pub struct RTriple<'a>(
    Option<SignedRealNumber<'a>>,
    Option<SignedRealNumber<'a>>,
    Option<SignedRealNumber<'a>>,
);
pub struct Triple<'a>(
    Option<RealNumber<'a>>,
    Option<RealNumber<'a>>,
    Option<RealNumber<'a>>,
);

impl<'a> Consume<'a> for QString<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
            return Err(Box::new(Error {
                line: tkw.line,
                msg: format!("unclosed string quote"),
            }));
        }

        // @TODO: Replace escaped "
        Ok(Self(Cow::Borrowed(s)))
    }
}

impl<'a> Consume<'a> for RealNumber<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        let start = tkw.offset;
        tkw.next_if_matches(|b| matches!(b, b'+' | b'-'));
        RealNumber::consume(tkw)?;
        Ok(Self(&tkw.content[start..tkw.offset]))
    }
}

impl<'a> Consume<'a> for Triple<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
            return Err(Box::new(Error {
                line: tkw.line,
                msg: format!("all three unset in triple"),
            }));
        }

        Ok(Self(fst, snd, trd))
    }
}
impl<'a> Consume<'a> for RTriple<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
            return Err(Box::new(Error {
                line: tkw.line,
                msg: format!("all three unset in rtriple"),
            }));
        }

        Ok(Self(fst, snd, trd))
    }
}

impl<'a> Consume<'a> for SignedRealNumberOrRTriple<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
                _ => Err(Box::new(Error {
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
                    return Err(Box::new(Error {
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
                        return Err(Box::new(Error {
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
                        return Err(Box::new(Error {
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

fn parse_param<'a, T: Consume<'a>>(
    tkw: &mut TokenWalker<'a>,
    name: &'static str,
) -> Result<T, Box<Error>> {
    parse_param_with(tkw, name, T::consume)
}
fn parse_param_with<'a, T>(
    tkw: &mut TokenWalker<'a>,
    name: &'static str,
    mut f: impl FnMut(&mut TokenWalker<'a>) -> Result<T, Box<Error>>,
) -> Result<T, Box<Error>> {
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

impl<'a> Consume<'a> for Cell<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        tkw.skip_whitespace();
        tkw.expect_char(b'(')?;

        dbg!(tkw.peek_content());
        tkw.skip_whitespace();
        tkw.expect_ident("CELL")?;

        dbg!(tkw.peek_content());
        let celltype = parse_param::<QString<'a>>(tkw, "CELLTYPE")?;
        dbg!(tkw.peek_content());
        let instance = parse_param::<Instance<'a>>(tkw, "INSTANCE")?;
        dbg!(tkw.peek_content());
        let mut timing_specs = Vec::new();

        loop {
            tkw.skip_whitespace();
            dbg!(tkw.peek_content());
            if tkw.is_next_equal_to(b')') {
                break;
            }

            let timing_spec = TimingSpec::consume(tkw)?;
            timing_specs.push(timing_spec);
        }
        dbg!(tkw.peek_content());

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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        let checkpoint = tkw.checkpoint();
        tkw.skip_whitespace();
        tkw.expect_char(b'(')?;

        tkw.skip_whitespace();
        Ok(match tkw.next_ident() {
            Some("DELAY") => {
                tkw.restore_checkpoint(checkpoint);
                Self::Delay(DelaySpec::consume(tkw)?)
            }
            Some("TIMINGCHECK") => todo!(), //Self::TimingCheck(TimingCheckSpec::consume(tkw)?),
            Some("TIMINGENV") => todo!(),   //Self::TimingEnv(TimingEnvSpec::consume(tkw)?),
            Some("LABEL") => todo!(),       //Self::Label(LabelSpec::consume(tkw)?),
            Some(name) => {
                return Err(Box::new(Error {
                    line: tkw.line,
                    msg: format!("unexpected '{name}'"),
                }));
            }
            None => {
                return Err(Box::new(Error {
                    line: tkw.line,
                    msg: format!("expected ident"),
                }));
            }
        })
    }
}

impl<'a> Consume<'a> for Instance<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        let fst = tkw.next_ident().ok_or_else(|| {
            Box::new(Error {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        let result = parse_param_with(tkw, "DELAYFILE", |tkw| {
            let header = SdfHeader::consume(tkw)?;
            let mut cells = Vec::new();

            loop {
                tkw.skip_whitespace();
                if !tkw.is_next_equal_to(b'(') {
                    break;
                }

                dbg!(tkw.peek_content());
                let cell = Cell::consume(tkw)?;
                cells.push(cell);
            }

            Ok(Self { header, cells })
        })?;

        tkw.skip_whitespace();
        if tkw.offset != tkw.content.len() {
            return Err(Box::new(Error {
                line: tkw.line,
                msg: format!("remaining token"),
            }));
        }

        Ok(result)
    }
}

impl<'a> Consume<'a> for RValue<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        let fst = DelVal::consume(tkw)?;

        tkw.skip_whitespace();
        if !tkw.is_next_equal_to(b'(') {
            return Ok(Self::One(fst));
        }
        let snd = DelVal::consume(tkw)?;

        tkw.skip_whitespace();
        if !tkw.is_next_equal_to(b'(') {
            return Ok(Self::Two(fst, snd));
        }
        let trd = DelVal::consume(tkw)?;

        tkw.skip_whitespace();
        if !tkw.is_next_equal_to(b'(') {
            return Ok(Self::Three(fst, snd, trd));
        }

        let four = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        let five = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        let six = DelVal::consume(tkw)?;
        tkw.skip_whitespace();
        if !tkw.is_next_equal_to(b'(') {
            return Ok(Self::Six(fst, snd, trd, four, five, six));
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
        Ok(Self::Twelve(
            fst, snd, trd, four, five, six, seven, eight, nine, ten, eleven, twelve,
        ))
    }
}
impl<'a> Consume<'a> for RetValList<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        parse_param::<RetValList>(tkw, "RETAIN").map(Self)
    }
}
impl<'a> Consume<'a> for IoPathDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        parse_param_with(tkw, "COND", |tkw| {
            let qstring = if tkw.is_next_equal_to(b'"') {
                Some(QString::consume(tkw)?)
            } else {
                None
            };
            tkw.skip_whitespace();
            let conditional_port_expr = CondPortExpr::consume(tkw)?;

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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        parse_param::<IoPathDef<'a>>(tkw, "CONDELSE").map(Self)
    }
}
impl<'a> Consume<'a> for PortDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        tkw.expect_char_matches(|b| b.is_ascii_digit())?;
        let start = tkw.offset - 1;
        tkw.skip_while(|b| b.is_ascii_digit());
        Ok(Self(&tkw.content[start..tkw.offset]))
    }
}

impl<'a> Consume<'a> for PortInstance<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        if tkw.is_next_equal_to(b'(') {
            PortEdge::consume(tkw).map(Self::Edge)
        } else {
            PortInstance::consume(tkw).map(Self::Instance)
        }
    }
}
impl<'a> Consume<'a> for PortEdge<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
                return Err(Box::new(Error {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
                return Err(Box::new(Error {
                    line: tkw.line,
                    msg: format!("unknown delay def"),
                }));
            }
        })
    }
}

impl<'a> Consume<'a> for DelayType<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
                return Err(Box::new(Error {
                    line: tkw.line,
                    msg: format!("unknown delay type"),
                }));
            }
        })
    }
}

impl<'a> Consume<'a> for AbsoluteDelayType<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
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

impl<'a> Consume<'a> for CondPortExpr<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, Box<Error>> {
        let start = tkw.offset;
        if tkw.next_if_equals(b'(') {
        } else {
            let mut count = 0;
            tkw.skip_while(|b| {
                if matches!(b, b'(') && count == 0 {
                    return false;
                }
                count += usize::from(matches!(b, b'('));
                count -= usize::from(matches!(b, b')'));
                true
            });
            tkw.expect_char(b')')?;
        }
        let end = tkw.offset;
        Ok(Self(&tkw.content[start..end]))
    }
}
