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
//              - [ ] COND
//                - [ ] conditional_port_expr
//              - [x] CONDELSE
//              - [ ] PORT
//                - [ ] port_instance
//              - [ ] INTERCONNECT
//              - [ ] NETDELAY
//                - [ ] net_spec
//              - [ ] DEVICE
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

pub mod tokenizer;

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
    PathPulse(),
    PathPulseProcent(),
}

pub struct AbsoluteDelayType<'a> {
    defs: Vec<DelayDef<'a>>,
}
pub struct IncrementDelayType<'a> {
    defs: Vec<DelayDef<'a>>,
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

pub struct CondPortExpr<'a>(&'a str);

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
    temporature: Option<SignedRealNumberOrRTriple<'a>>,
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
}

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

impl<'a> TokenWalker<'a> {
    pub fn new(content: &'a str) -> Self {
        Self { offset: 0, content }
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

    pub fn expect_char(&mut self, b: u8) -> Result<(), ()> {
        if self.content.as_bytes().get(self.offset).copied() != Some(b) {
            return Err(());
        }

        self.offset += 1;
        Ok(())
    }
    pub fn expect_char_matches(&mut self, mut f: impl FnMut(u8) -> bool) -> Result<(), ()> {
        if let Some(&b) = self.content.as_bytes().get(self.offset)
            && f(b)
        {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn expect_ident(&mut self, s: &str) -> Result<(), ()> {
        if self.next_ident() != Some(s) {
            return Err(());
        }
        Ok(())
    }

    pub fn next_ident(&mut self) -> Option<&'a str> {
        if !matches!(
            self.content.as_bytes().get(self.offset),
            Some(b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z')
        ) {
            return None;
        }

        let start = self.offset;

        self.offset += 1;
        let bs = self.content.as_bytes();
        while let Some(b) = bs.get(self.offset)
            && matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z')
        {
            // @TODO: Escaped characters
            self.offset += 1;
        }

        Some(&self.content[start..self.offset])
    }

    pub fn skip_while(&mut self, mut f: impl FnMut(u8) -> bool) {
        while let Some(&b) = self.content.as_bytes().get(self.offset)
            && f(b)
        {
            self.offset += 1;
        }
    }

    pub fn skip_whitespace(&mut self) {
        self.skip_while(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'));
    }
}

pub trait Consume<'a>: Sized {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()>;
}

pub struct QString<'a>(Cow<'a, str>);
pub struct RealNumber<'a>(&'a str);
pub struct SignedRealNumber<'a>(&'a str);
pub struct RTriple<'a>(
    Option<SignedRealNumber<'a>>,
    Option<SignedRealNumber<'a>>,
    Option<SignedRealNumber<'a>>,
);

impl<'a> Consume<'a> for QString<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        tkw.expect_char(b'"')?;
        let bs = tkw.content.as_bytes();
        let mut j = tkw.offset;
        let mut is_escaped = false;
        while let Some(&b) = bs.get(j)
            && (is_escaped || b != b'"')
        {
            is_escaped = !is_escaped & (b == b'\\');
            j += 1;
        }
        if j == bs.len() {
            return Err(());
        }
        let s = &tkw.content[tkw.offset..j];
        tkw.offset = j + 1;

        // @TODO: Replace escaped "
        Ok(Self(Cow::Borrowed(s)))
    }
}

impl<'a> Consume<'a> for RealNumber<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        // integer
        let start = tkw.offset;
        tkw.expect_char_matches(|b| b.is_ascii_digit())?;
        tkw.skip_while(|b| b.is_ascii_digit());

        fn skip_exponent<'a>(tkw: &mut TokenWalker<'a>) {
            // e [ sign ] integer
            let checkpoint = tkw.offset;
            let has_e = tkw.next_if_equals(b'e');
            tkw.next_if_matches(|b| matches!(b, b'+' | b'-'));
            let has_integer = tkw.does_next_char_match(|b| b.is_ascii_digit());
            if !has_e || has_integer {
                tkw.offset = checkpoint;
                return;
            }
            tkw.skip_while(|b| b.is_ascii_digit());
        }

        // [ . integer ]
        let checkpoint = tkw.offset;
        if !tkw.next_if_equals(b'.') || !tkw.does_next_char_match(|b| b.is_ascii_digit()) {
            tkw.offset = checkpoint;
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        let start = tkw.offset;
        tkw.next_if_matches(|b| matches!(b, b'+' | b'-'));
        RealNumber::consume(tkw)?;
        Ok(Self(&tkw.content[start..tkw.offset]))
    }
}

impl<'a> Consume<'a> for RTriple<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
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
            return Err(());
        }

        Ok(Self(fst, snd, trd))
    }
}

impl<'a> Consume<'a> for SignedRealNumberOrRTriple<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        let checkpoint = tkw.offset;
        Ok(if let Ok(n) = SignedRealNumber::consume(tkw) {
            tkw.skip_whitespace();
            if tkw.is_next_equal_to(b':') {
                tkw.offset = checkpoint;
                SignedRealNumberOrRTriple::RTriple(RTriple::consume(tkw)?)
            } else {
                SignedRealNumberOrRTriple::SignedRealNumber(n)
            }
        } else {
            tkw.offset = checkpoint;
            SignedRealNumberOrRTriple::RTriple(RTriple::consume(tkw)?)
        })
    }
}

impl<'a> Consume<'a> for SdfHeader<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
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
                _ => Err(()),
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
                if tkw.is_next_equal_to(b'(') {
                    tkw.offset += 1;
                    tkw.skip_whitespace();
                    if tkw.next_ident() == Some($name) {
                        let result = $expr;
                        tkw.skip_whitespace();
                        tkw.expect_char(b')')?;
                        Some(result)
                    } else {
                        None
                    }
                } else {
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
                _ => return Err(()),
            }
        });
        let voltage = opt_param!("VOLTAGE"[rtriple]);
        let process = opt_param!("PROCESS"[qstring]);
        let temporature = opt_param!("TEMPORATURE"[rtriple]);
        let timescale = opt_param!("TIMESCALE", {
            // timescale_number ::= 1 | 10 | 100 | 1.0 | 10.0 | 100.0
            // timescale_unit ::= s | ms | us | ns | ps | fs
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
                    _ => return Err(()),
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
                    _ => return Err(()),
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
            temporature,
            timescale,
        })
    }
}

fn parse_param<'a, T: Consume<'a>>(tkw: &mut TokenWalker<'a>, name: &'static str) -> Result<T, ()> {
    parse_param_with(tkw, name, T::consume)
}
fn parse_param_with<'a, T>(
    tkw: &mut TokenWalker<'a>,
    name: &'static str,
    mut f: impl FnMut(&mut TokenWalker<'a>) -> Result<T, ()>,
) -> Result<T, ()> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        tkw.skip_whitespace();
        tkw.expect_char(b'(')?;

        tkw.skip_whitespace();
        tkw.expect_ident("CELL")?;

        let celltype = parse_param::<QString<'a>>(tkw, "CELLTYPE")?;
        let instance = parse_param::<Instance<'a>>(tkw, "INSTANCE")?;
        let mut timing_specs = Vec::new();

        loop {
            tkw.skip_whitespace();
            if !tkw.is_next_equal_to(b'(') {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        tkw.skip_whitespace();
        tkw.expect_char(b'(')?;

        tkw.skip_whitespace();
        let result = match tkw.next_ident() {
            Some("DELAY") => Self::Delay(DelaySpec::consume(tkw)?),
            Some("TIMINGCHECK") => Self::TimingCheck(TimingCheckSpec::consume(tkw)?),
            Some("TIMINGENV") => Self::TimingEnv(TimingEnvSpec::consume(tkw)?),
            Some("LABEL") => Self::Label(LabelSpec::consume(tkw)?),
            _ => return Err(()),
        };

        tkw.skip_whitespace();
        tkw.expect_char(b')')?;
    }
}

impl<'a> Consume<'a> for Instance<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        if tkw.next_if_equals(b')') {
            Ok(Self::Empty)
        } else if tkw.next_if_equals(b'*') {
            Ok(Self::Star)
        } else {
            Ok(Self::HierarchicalIdent(HierarchicalIdent::consume(tkw)?))
        }
    }
}

impl<'a> Consume<'a> for HierarchicalIdent<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        let fst = tkw.next_ident().ok_or(())?;
        let mut next = Vec::new();
        loop {
            tkw.skip_whitespace();
            let checkpoint = tkw.offset;
            if tkw.next_if_matches(|b| matches!(b, b'.' | b'/'))
                && let _ = tkw.skip_whitespace()
                && let Some(ident) = tkw.next_ident()
            {
                let hchar = if tkw.content.as_bytes()[checkpoint] == b'.' {
                    HierarchyDivider::Dot
                } else {
                    HierarchyDivider::Slash
                };
                next.push((hchar, ident));
            } else {
                tkw.offset = checkpoint;
                break;
            }
        }
        Ok(Self { fst, next })
    }
}

impl<'a> Consume<'a> for DelayFile<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
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
            return Err(());
        }

        Ok(result)
    }
}

impl<'a> Consume<'a> for RValue<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
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

impl<'a> Consume<'a> for DelVal<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        let checkpoint = tkw.offset;
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
            tkw.offset = checkpoint;
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        let fst = DelVal::consume(tkw)?;
        if !tkw.next_if_equals(b'(') {
            return Ok(Self::One(fst));
        }
        let snd = DelVal::consume(tkw)?;
        if !tkw.next_if_equals(b'(') {
            return Ok(Self::Two(fst, snd));
        }
        let trd = DelVal::consume(tkw)?;
        if !tkw.next_if_equals(b'(') {
            return Ok(Self::Three(fst, snd, trd));
        }
        let four = DelVal::consume(tkw)?;
        let five = DelVal::consume(tkw)?;
        let six = DelVal::consume(tkw)?;
        if !tkw.next_if_equals(b'(') {
            return Ok(Self::Six(fst, snd, trd, four, five, six));
        }

        let seven = DelVal::consume(tkw)?;
        let eight = DelVal::consume(tkw)?;
        let nine = DelVal::consume(tkw)?;
        let ten = DelVal::consume(tkw)?;
        let eleven = DelVal::consume(tkw)?;
        let twelve = DelVal::consume(tkw)?;
        Ok(Self::Twelve(
            fst, snd, trd, four, five, six, seven, eight, nine, ten, eleven, twelve,
        ))
    }
}
impl<'a> Consume<'a> for RetValList<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        parse_param::<RetValList>(tkw, "RETAIN").map(Self)
    }
}
impl<'a> Consume<'a> for IoPathDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        parse_param_with(tkw, "IOPATH", |tkw| {
            tkw.skip_whitespace();
            let port_spec = PortSpec::consume(tkw)?;

            tkw.skip_whitespace();
            let port_instance = PortInstance::consume(tkw)?;

            let mut retain_defs = Vec::new();
            loop {
                tkw.skip_whitespace();
                let checkpoint = tkw.offset;
                if !tkw.next_if_equals(b'(') {
                    break;
                }

                tkw.skip_whitespace();
                if tkw.next_ident() != Some("RETAIN") {
                    tkw.offset = checkpoint;
                    break;
                }
                tkw.offset = checkpoint;
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
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
            Ok(Self { qstring, conditional_port_expr, io_path })
        })
    }
}
impl<'a> Consume<'a> for CondElseDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        parse_param::<IoPathDef<'a>>(tkw, "CONDELSE").map(Self)
    }
}
impl<'a> Consume<'a> for PortDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        parse_param_with(tkw, "PORT", |tkw| {
            let port_instance = PortInstance::consume(tkw)?;
            tkw.skip_whitespace();
            let delval_list = DelValList::consume(tkw)?;
            Ok(Self { port_instance, delval_list })
        })
    }
}
impl<'a> Consume<'a> for InterconnectDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        parse_param_with(tkw, "INTERCONNECT", |tkw| {
            let from = PortInstance::consume(tkw)?;
            tkw.skip_whitespace();
            let to = PortInstance::consume(tkw)?;
            tkw.skip_whitespace();
            let delval_list = DelValList::consume(tkw)?;
            Ok(Self { from, to, delval_list })
        })
    }
}
impl<'a> Consume<'a> for NetDelayDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        parse_param_with(tkw, "NETDELAY", |tkw| {
            let net_spec = PortInstance::consume(tkw)?;
            tkw.skip_whitespace();
            let delval_list = DelValList::consume(tkw)?;
            Ok(Self { net_spec, delval_list })
        })
    }
}
impl<'a> Consume<'a> for DeviceDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        parse_param_with(tkw, "DEVICE", |tkw| {
            let port_instance = if tkw.is_next_equal_to(b'(') {
                None
            } else {
                Some(PortInstance::consume(tkw)?)
            };
            tkw.skip_whitespace();
            let delval_list = DelValList::consume(tkw)?;
            Ok(Self { port_instance, delval_list })
        })
    }
}

impl<'a> Consume<'a> for Integer<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        if !tkw.does_next_char_match(|b| b.is_ascii_digit()) {
            return Err(());
        }
        let start = tkw.offset;
        tkw.offset += 1;
        tkw.skip_while(|b| b.is_ascii_digit());
        Ok(Self(&tkw.content[start..tkw.offset]))
    }
}

impl<'a> Consume<'a> for PortInstance<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
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
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        if tkw.is_next_equal_to(b'(') {
            PortEdge::consume(tkw).map(Self::Edge)
        } else {
            PortInstance::consume(tkw).map(Self::Instance)
        }
    }
}
impl<'a> Consume<'a> for PortEdge<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
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
            _ => return Err(()),
        };

        tkw.skip_whitespace();
        let instance = PortInstance::consume(tkw)?;

        tkw.skip_whitespace();
        tkw.expect_char(b')')?;

        Ok(Self { edge, instance })
    }
}

impl<'a> Consume<'a> for DelaySpec<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {}
}

impl<'a> Consume<'a> for DelayDef<'a> {
    fn consume(tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        tkw.skip_whitespace();
        let checkpoint = tkw.offset;
        tkw.expect_char(b'(')?;

        tkw.skip_whitespace();
        let ident = tkw.next_ident();
        tkw.offset = checkpoint;
        Ok(match ident {
            Some("IOPATH") => {
                Self::IoPath(IoPathDef::consume(tkw)?)
            },
            Some("RETAIN") => {
                Self::Retain(RetainDef::consume(tkw)?)
            },
            Some("COND") => {
                Self::Cond(CondDef::consume(tkw)?)
            },
            Some("CONDELSE") => {
                Self::CondElse(CondElseDef::consume(tkw)?)
            },
            Some("PORT") => {
                Self::Port(PortDef::consume(tkw)?)
            },
            Some("INTERCONNECT") => {
                Self::Interconnect(InterconnectDef::consume(tkw)?)
            },
            Some("NETDELAY") => {
                Self::NetDelay(NetDelayDef::consume(tkw)?)
            },
            Some("DEVICE") => {
                Self::Device(DeviceDef::consume(tkw)?)
            },
            _ => return Err(()),
        })
    }
}

impl<'a> Consume<'a> for CondPortExpr<'a> {
    fn consume(_tkw: &mut TokenWalker<'a>) -> Result<Self, ()> {
        todo!()
    }
}
