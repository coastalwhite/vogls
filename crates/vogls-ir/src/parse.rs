use std::fmt;
use std::num::NonZeroU32;

use hashbrown::hash_map::Entry;
use vogls_bits::{Bits, VectorSize};
use vogls_utils::{VgHashMap, VgHashSet};

use crate::dyn_format_string::{DynFormatArgument, DynFormatString};
use crate::token_range::TokenRange;
use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, GlobalContext,
    Instruction, IntrinsicOp, ProcessKey, ProcessKind, ResizeOp, SCALAR_VSIZE, ShiftImmOp, Signal,
    SignalFlags, SignalKey, TIME_VSIZE, Time, UnaryOp, VSIZE_32, VariableKey,
};

#[derive(Debug)]
pub struct ParseError {
    at: usize,
    error: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { at, error } = self;
        write!(f, "{at}: {error}")
    }
}
impl std::error::Error for ParseError {}

pub struct Cursor<'a> {
    content: &'a str,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub const fn new(content: &'a str) -> Self {
        Self { content, offset: 0 }
    }

    pub fn trim_cursor(&mut self) {
        let bs = self.content.as_bytes();
        while self.offset < bs.len() {
            while bs
                .get(self.offset)
                .is_some_and(|&b| b.is_ascii_whitespace())
            {
                self.offset += 1;
            }
            if !bs[self.offset..].starts_with(b"//") {
                break;
            }
            self.offset += 2;
            let rem = &bs[self.offset..];
            self.offset += rem.iter().position(|b| *b == b'\n').unwrap_or(rem.len());
        }
    }

    fn starts_with(&self, pat: &str) -> bool {
        self.content[self.offset..].starts_with(pat)
    }

    fn is_empty(&self) -> bool {
        self.offset >= self.content.len()
    }
    fn expect_keyword(&mut self, keyword: &str) -> Result<(), Box<ParseError>> {
        let start_offset = self.offset;
        if take_ident(self.content, &mut self.offset).is_none_or(|ident| ident != keyword) {
            return Err(Box::new(ParseError {
                at: start_offset,
                error: format!(
                    "expected '{keyword}', got: '{:?}'",
                    self.content[self.offset..].chars().next()
                ),
            }));
        }
        Ok(())
    }

    fn take_ident(&mut self) -> Result<&'a str, Box<ParseError>> {
        take_ident(self.content, &mut self.offset).ok_or_else(|| {
            Box::new(ParseError {
                at: self.offset,
                error: format!(
                    "expected ident, got: '{:?}'",
                    self.content[self.offset..].chars().next()
                ),
            })
        })
    }

    fn expect_char(&mut self, c: char) -> Result<(), Box<ParseError>> {
        self.next_if_equals(c).then_some(()).ok_or_else(|| {
            Box::new(ParseError {
                at: self.offset,
                error: format!(
                    "expected '{c}', got: '{:?}'",
                    self.content[self.offset..].chars().next()
                ),
            })
        })
    }

    fn next_if_equals(&mut self, c: char) -> bool {
        if self.content[self.offset..].starts_with(c) {
            self.offset += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn is_next_equal_to(&self, c: char) -> bool {
        self.content[self.offset..].starts_with(c)
    }
}

impl ParseError {
    pub fn expected_keyword(at: usize, keyword: &'static str) -> Self {
        Self {
            at,
            error: format!("expected keyword: {keyword}"),
        }
    }

    fn overflow(at: usize) -> ParseError {
        Self {
            at,
            error: format!("overflow"),
        }
    }
}

fn take_ident<'a>(s: &'a str, offset: &mut usize) -> Option<&'a str> {
    let start = *offset;
    let bs = s.as_bytes();
    if !(bs[*offset].is_ascii_alphabetic() || matches!(bs[*offset], b'_')) {
        return None;
    }
    *offset += 1;
    while bs[*offset].is_ascii_alphanumeric() || matches!(bs[*offset], b'_' | b'/' | b'-' | b'.') {
        *offset += 1;
    }

    Some(&s[start..*offset])
}

pub fn parse(s: &str, gl: &mut GlobalContext) -> Result<(), Box<ParseError>> {
    let c = &mut Cursor::new(s);
    let mut symbols = Symbols {
        variables: Default::default(),
        signals: Default::default(),
        bbs: Default::default(),
        var_sizes: Default::default(),
    };

    c.trim_cursor();
    while c.starts_with("signal") {
        parse_signal_definition(c, &mut symbols, gl)?;
        c.trim_cursor();
    }

    while !c.is_empty() {
        parse_process(c, &mut symbols, gl)?;
        c.trim_cursor();
    }

    Ok(())
}

enum VarSize {
    Unresolved(Instruction),
    Resolved(VectorSize),
}

struct Symbols<'a> {
    variables: VgHashMap<&'a str, (bool, VariableKey)>,
    signals: VgHashMap<&'a str, SignalKey>,
    bbs: VgHashMap<&'a str, (bool, BasicBlockKey)>,
    var_sizes: VgHashMap<u64, VarSize>,
}

fn parse_signal_definition<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
) -> Result<(), Box<ParseError>> {
    c.trim_cursor();
    c.expect_keyword("signal")?;

    c.trim_cursor();
    let name = c.take_ident()?;
    c.trim_cursor();
    c.expect_char(':')?;
    c.trim_cursor();
    let size = parse_nonzero_u32(c)?;

    c.trim_cursor();
    let mut initialize = None;
    if c.next_if_equals('=') {
        c.trim_cursor();
        initialize = Some(parse_imm(c)?);
    }

    if symbols
        .signals
        .insert(
            name,
            gl.signals.insert(Signal {
                name: name.to_string(),
                size,
                flags: SignalFlags::EMPTY,
                initialize,
                origin: TokenRange::default(),
            }),
        )
        .is_some()
    {
        return Err(Box::new(ParseError {
            at: c.offset,
            error: format!("duplicate signal"),
        }));
    }
    Ok(())
}

fn parse_process<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
) -> Result<ProcessKey, Box<ParseError>> {
    c.trim_cursor();
    c.expect_keyword("proc")?;

    c.trim_cursor();
    let kind = match c.take_ident()? {
        "assign" => ProcessKind::Assign,
        "always" => ProcessKind::Always,
        "initial" => ProcessKind::Initial,
        "fuse" => ProcessKind::Fuse,
        "specify" => ProcessKind::Specify,
        _ => ProcessKind::Other,
    };

    c.trim_cursor();
    c.expect_char('{')?;

    c.trim_cursor();

    symbols.bbs.clear();
    symbols.variables.clear();
    symbols.var_sizes.clear();

    let mut entry = None;
    while !c.next_if_equals('}') {
        let ident = c.take_ident()?;
        let bb_key = match symbols.bbs.entry(ident) {
            Entry::Occupied(mut entry) => {
                if entry.get().0 {
                    return Err(Box::new(ParseError {
                        at: c.offset,
                        error: format!("basic block '{ident}' is defined twice"),
                    }));
                }
                entry.get_mut().0 = true;
                entry.get().1
            }
            Entry::Vacant(entry) => {
                let bb_key = gl.bbs.insert(BasicBlock {
                    instrs: Vec::new(),
                    terminator: BasicBlockTerminator::Halt,
                });
                entry.insert((true, bb_key));
                bb_key
            }
        };
        entry.get_or_insert(bb_key);
        c.expect_char(':')?;

        parse_bb(c, symbols, gl, bb_key)?;
        c.trim_cursor();
    }

    let entry = entry.ok_or_else(|| {
        Box::new(ParseError {
            at: c.offset,
            error: format!("missing entry basic block"),
        })
    })?;

    for (k, (is_defined, _)) in &symbols.variables {
        if !*is_defined {
            return Err(Box::new(ParseError {
                at: c.offset,
                error: format!("variable '{k}' is used but never defined"),
            }));
        }
    }
    for (k, (is_defined, _)) in &symbols.bbs {
        if !*is_defined {
            return Err(Box::new(ParseError {
                at: c.offset,
                error: format!("basic block '{k}' is used but never defined"),
            }));
        }
    }

    let mut num_unresolved_vars = symbols
        .var_sizes
        .values()
        .filter(|s| matches!(s, VarSize::Unresolved(..)))
        .count();
    while num_unresolved_vars > 0 {
        // @Performance. This is an abomination.
        let keys = symbols
            .var_sizes
            .iter()
            .filter(|(_, s)| matches!(s, VarSize::Unresolved(..)))
            .map(|(k, _)| k)
            .copied()
            .collect::<Vec<_>>();
        let start_length = num_unresolved_vars;
        for key in keys {
            let VarSize::Unresolved(i) = &symbols.var_sizes[&key] else {
                unreachable!();
            };
            use Instruction as I;
            match i {
                I::Constant(..) => unreachable!(),
                I::Unary(_, op, src) => {
                    if let VarSize::Resolved(src_size) = symbols.var_sizes[&src.identifier()] {
                        symbols
                            .var_sizes
                            .insert(key, VarSize::Resolved(op.output_size(src_size)));
                        num_unresolved_vars -= 1;
                    }
                }
                I::Resize(..) => unreachable!(),
                I::Binary(_, op, lhs, rhs) => {
                    if let VarSize::Resolved(lhs_size) = symbols.var_sizes[&lhs.identifier()]
                        && let VarSize::Resolved(rhs_size) = symbols.var_sizes[&rhs.identifier()]
                    {
                        let Some(dst_size) = op.output_size(lhs_size, rhs_size) else {
                            return Err(Box::new(ParseError {
                                at: c.offset,
                                error: format!("invalid size combination: {lhs_size}, {rhs_size}"),
                            }));
                        };
                        symbols.var_sizes.insert(key, VarSize::Resolved(dst_size));
                        num_unresolved_vars -= 1;
                    }
                }
                I::Select(_, _cond, lhs, rhs) => {
                    if let VarSize::Resolved(lhs_size) = symbols.var_sizes[&lhs.identifier()]
                        && let VarSize::Resolved(rhs_size) = symbols.var_sizes[&rhs.identifier()]
                    {
                        if lhs_size != rhs_size {
                            return Err(Box::new(ParseError {
                                at: c.offset,
                                error: format!("invalid size combination: {lhs_size}, {rhs_size}"),
                            }));
                        };
                        symbols.var_sizes.insert(key, VarSize::Resolved(lhs_size));
                        num_unresolved_vars -= 1;
                    }
                }
                I::BinaryImm(_, op, lhs, rhs) => {
                    if let VarSize::Resolved(lhs_size) = symbols.var_sizes[&lhs.identifier()] {
                        let rhs_size = rhs.size();
                        let Some(dst_size) = op.output_size(lhs_size, rhs_size) else {
                            return Err(Box::new(ParseError {
                                at: c.offset,
                                error: format!("invalid size combination: {lhs_size}, {rhs_size}"),
                            }));
                        };
                        symbols.var_sizes.insert(key, VarSize::Resolved(dst_size));
                        num_unresolved_vars -= 1;
                    }
                }
                I::Slice(..) => unreachable!(),
                I::SliceImm(..) => unreachable!(),
                I::ShiftImm(_, _, src, _) => {
                    if let VarSize::Resolved(src_size) = symbols.var_sizes[&src.identifier()] {
                        symbols.var_sizes.insert(key, VarSize::Resolved(src_size));
                        num_unresolved_vars -= 1;
                    }
                }
                I::Intrinsic(..) => todo!(),
                I::LastUpdateTime(..) => unreachable!(),
                I::Probe(..) => unreachable!(),
                I::ProbeSlice(..) => unreachable!(),
                I::Drive(..) => unreachable!(),
                I::Phi(_, items) => {
                    if let Some(src_size) = items.iter().find_map(|(_, v)| {
                        if let VarSize::Resolved(size) = symbols.var_sizes[&v.identifier()] {
                            return Some(size);
                        }
                        None
                    }) {
                        symbols.var_sizes.insert(key, VarSize::Resolved(src_size));
                        num_unresolved_vars -= 1;
                    }
                }
            }
        }
        assert!(num_unresolved_vars < start_length);
    }

    // @Performance: Scratchpad these.
    let mut bb_stack = Vec::new();
    let mut bb_seen = VgHashSet::default();

    // Map each variable to the correct size.
    bb_stack.push(entry);
    bb_seen.insert(entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = &mut gl.bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_seen.insert(bb) {
                bb_stack.push(bb);
            }
        });

        bb.map_vars(|mut v| {
            let VarSize::Resolved(size) = symbols.var_sizes[&v.identifier()] else {
                unreachable!()
            };
            gl.vars.update(&mut v, size);
            v
        });
    }

    let process = gl.processes.insert(crate::Process {
        kind,
        entry,
        origin: TokenRange::default(),
    });

    Ok(process)
}

fn parse_bb<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
    bb_key: BasicBlockKey,
) -> Result<(), Box<ParseError>> {
    let mut terminator: Option<BasicBlockTerminator> = None;
    let mut instrs: Vec<Instruction> = Vec::new();

    loop {
        c.trim_cursor();
        if c.is_next_equal_to('}') {
            break;
        }

        if c.next_if_equals('%') {
            if terminator.is_some() {
                return Err(Box::new(ParseError {
                    at: c.offset,
                    error: format!("already saw terminator"),
                }));
            }

            let dst = c.take_ident()?;
            let dst = match symbols.variables.entry(dst) {
                Entry::Occupied(mut entry) => {
                    if entry.get().0 {
                        return Err(Box::new(ParseError {
                            at: c.offset,
                            error: format!("variable '{dst}' is defined twice"),
                        }));
                    }
                    entry.get_mut().0 = true;
                    entry.get().1
                }
                Entry::Vacant(entry) => {
                    let var = gl.vars.insert(SCALAR_VSIZE);
                    entry.insert((true, var));
                    var
                }
            };
            symbols.var_sizes.insert(
                dst.identifier(),
                VarSize::Unresolved(Instruction::Constant(dst, Bits::new_zeroed(SCALAR_VSIZE))),
            );

            c.trim_cursor();
            c.expect_char('=')?;

            c.trim_cursor();
            let iname = c.take_ident()?;

            use BinaryImmOp as BI;
            use BinaryOp as B;
            use ResizeOp as R;
            use ShiftImmOp as SI;
            use UnaryOp as UO;
            let i = match iname {
                "const" => {
                    c.trim_cursor();
                    let imm = parse_imm(c)?;
                    symbols
                        .var_sizes
                        .insert(dst.identifier(), VarSize::Resolved(imm.size()));
                    Instruction::Constant(dst, imm)
                }

                // Unary
                "negate" => parse_unary(c, symbols, gl, dst, UO::Neg)?,
                "reduce_or" => parse_unary(c, symbols, gl, dst, UO::ReduceOr)?,
                "reduce_and" => parse_unary(c, symbols, gl, dst, UO::ReduceAnd)?,
                "reduce_xor" => parse_unary(c, symbols, gl, dst, UO::ReduceXor)?,
                "leading_zeros" => parse_unary(c, symbols, gl, dst, UO::LeadingZeros)?,

                // Resize
                "truncate" => parse_resize(c, symbols, gl, dst, R::Truncate)?,
                "sign_extend" => parse_resize(c, symbols, gl, dst, R::SignExtend)?,
                "zero_extend" => parse_resize(c, symbols, gl, dst, R::ZeroExtend)?,

                // Binary
                "and" => parse_binary(c, symbols, gl, dst, B::And)?,
                "or" => parse_binary(c, symbols, gl, dst, B::Or)?,
                "xor" => parse_binary(c, symbols, gl, dst, B::Xor)?,
                "add" => parse_binary(c, symbols, gl, dst, B::Add)?,
                "sub" => parse_binary(c, symbols, gl, dst, B::Sub)?,
                "mul" => parse_binary(c, symbols, gl, dst, B::Multiply)?,
                "pow" => parse_binary(c, symbols, gl, dst, B::Power)?,
                "div" => parse_binary(c, symbols, gl, dst, B::Divide)?,
                "rem" => parse_binary(c, symbols, gl, dst, B::Modulus)?,
                "ule" => parse_binary(c, symbols, gl, dst, B::UnsignedLessEqual)?,
                "lsl" => parse_binary(c, symbols, gl, dst, B::LogicalShiftLeft)?,
                "lsr" => parse_binary(c, symbols, gl, dst, B::LogicalShiftRight)?,
                "asr" => parse_binary(c, symbols, gl, dst, B::ArithmeticShiftRight)?,
                "concat" => parse_binary(c, symbols, gl, dst, B::Concat)?,
                "copyx" => parse_binary(c, symbols, gl, dst, B::CopyX)?,
                "copyz" => parse_binary(c, symbols, gl, dst, B::CopyZ)?,
                "min" => parse_binary(c, symbols, gl, dst, B::Min)?,
                "max" => parse_binary(c, symbols, gl, dst, B::Max)?,
                "ceq" => parse_binary(c, symbols, gl, dst, B::CaseEquality)?,
                "posedge" => parse_binary(c, symbols, gl, dst, B::Posedge)?,
                "negedge" => parse_binary(c, symbols, gl, dst, B::Negedge)?,

                // BinaryImm
                "andi" => parse_binary_imm(c, symbols, gl, dst, BI::And)?,
                "ori" => parse_binary_imm(c, symbols, gl, dst, BI::Or)?,
                "xori" => parse_binary_imm(c, symbols, gl, dst, BI::Xor)?,
                "addi" => parse_binary_imm(c, symbols, gl, dst, BI::Add)?,
                "subi" => parse_binary_imm(c, symbols, gl, dst, BI::Sub)?,
                "muli" => parse_binary_imm(c, symbols, gl, dst, BI::Multiply)?,
                "powi" => parse_binary_imm(c, symbols, gl, dst, BI::Power)?,
                "divi" => parse_binary_imm(c, symbols, gl, dst, BI::Divide)?,
                "remi" => parse_binary_imm(c, symbols, gl, dst, BI::Modulus)?,
                "revsubi" => parse_binary_imm(c, symbols, gl, dst, BI::RevSub)?,
                "revpowi" => parse_binary_imm(c, symbols, gl, dst, BI::RevPower)?,
                "revdivi" => parse_binary_imm(c, symbols, gl, dst, BI::RevDivide)?,
                "revremi" => parse_binary_imm(c, symbols, gl, dst, BI::RevModulus)?,
                "ulei" => parse_binary_imm(c, symbols, gl, dst, BI::UnsignedLessEqual)?,
                "ugei" => parse_binary_imm(c, symbols, gl, dst, BI::UnsignedGreaterEqual)?,
                "concati_left" => parse_binary_imm(c, symbols, gl, dst, BI::ConcatLeft)?,
                "concati_right" => parse_binary_imm(c, symbols, gl, dst, BI::ConcatRight)?,
                "mini" => parse_binary_imm(c, symbols, gl, dst, BI::Min)?,
                "maxi" => parse_binary_imm(c, symbols, gl, dst, BI::Max)?,
                "ceqi" => parse_binary_imm(c, symbols, gl, dst, BI::CaseEquality)?,

                "slice" => {
                    c.trim_cursor();
                    let size = parse_braced_size(c)?;
                    symbols
                        .var_sizes
                        .insert(dst.identifier(), VarSize::Resolved(size));

                    c.trim_cursor();
                    let lhs = parse_var(c, symbols, gl)?;

                    c.trim_cursor();
                    c.expect_char(',')?;

                    c.trim_cursor();
                    let rhs = parse_var(c, symbols, gl)?;
                    Instruction::Slice(dst, lhs, rhs)
                }
                "slicei" => {
                    c.trim_cursor();
                    let size = parse_braced_size(c)?;
                    symbols
                        .var_sizes
                        .insert(dst.identifier(), VarSize::Resolved(size));

                    c.trim_cursor();
                    let lhs = parse_var(c, symbols, gl)?;

                    c.trim_cursor();
                    c.expect_char(',')?;

                    c.trim_cursor();
                    let rhs = parse_u32(c)?;
                    Instruction::SliceImm(dst, lhs, rhs)
                }

                "lsli" => parse_shift_imm(c, symbols, gl, dst, SI::LogicalShiftLeft)?,
                "lsri" => parse_shift_imm(c, symbols, gl, dst, SI::LogicalShiftRight)?,
                "asri" => parse_shift_imm(c, symbols, gl, dst, SI::ArithmeticShiftRight)?,

                "select" => {
                    c.trim_cursor();
                    let cond = parse_var(c, symbols, gl)?;
                    c.trim_cursor();
                    c.expect_char(',')?;

                    c.trim_cursor();
                    let truthy = parse_var(c, symbols, gl)?;
                    c.trim_cursor();
                    c.expect_char(',')?;

                    c.trim_cursor();
                    let falsy = parse_var(c, symbols, gl)?;

                    if let Some(VarSize::Resolved(cond_size)) =
                        symbols.var_sizes.get(&cond.identifier())
                        && *cond_size != SCALAR_VSIZE
                    {
                        return Err(Box::new(ParseError {
                            at: c.offset,
                            error: format!("invalid condition: {}", cond_size),
                        }));
                    }

                    if let Some(VarSize::Resolved(truthy_size)) =
                        symbols.var_sizes.get(&truthy.identifier())
                        && let Some(VarSize::Resolved(falsy_size)) =
                            symbols.var_sizes.get(&falsy.identifier())
                    {
                        if truthy_size != falsy_size {
                            return Err(Box::new(ParseError {
                                at: c.offset,
                                error: format!(
                                    "invalid size combination: {truthy_size}, {falsy_size}"
                                ),
                            }));
                        };
                        symbols
                            .var_sizes
                            .insert(dst.identifier(), VarSize::Resolved(*truthy_size));
                    }

                    Instruction::Select(dst, cond, truthy, falsy)
                }

                "lupdt" => {
                    c.trim_cursor();
                    let signal = parse_signal(c, symbols)?;
                    symbols
                        .var_sizes
                        .insert(dst.identifier(), VarSize::Resolved(TIME_VSIZE));
                    Instruction::LastUpdateTime(dst, signal)
                }

                "prb" => {
                    let mut size = None;
                    if c.is_next_equal_to('[') {
                        size = Some(parse_braced_size(c)?);
                    }
                    c.trim_cursor();
                    let signal = parse_signal(c, symbols)?;
                    let size = size.unwrap_or(gl.signals[signal].size);
                    symbols
                        .var_sizes
                        .insert(dst.identifier(), VarSize::Resolved(size));

                    c.trim_cursor();
                    if c.next_if_equals(',') {
                        c.trim_cursor();
                        if c.is_next_equal_to('%') {
                            let offset = parse_var(c, symbols, gl)?;
                            Instruction::ProbeSlice(dst, signal, offset)
                        } else {
                            let offset = parse_u32(c)?;
                            Instruction::Probe(dst, signal, offset)
                        }
                    } else {
                        Instruction::Probe(dst, signal, 0)
                    }
                }

                "vogls.assert" => {
                    c.trim_cursor();
                    let dyn_format_string = parse_dyn_format_string(c)?;
                    let args = (0..dyn_format_string.arguments().len() + 1)
                        .map(|_| {
                            c.trim_cursor();
                            c.expect_char(',')?;
                            c.trim_cursor();
                            parse_var(c, symbols, gl)
                        })
                        .collect::<Result<Box<[_]>, _>>()?;
                    let op = IntrinsicOp::Assert(Box::new(dyn_format_string));

                    symbols
                        .var_sizes
                        .insert(dst.identifier(), VarSize::Resolved(SCALAR_VSIZE));
                    Instruction::Intrinsic(dst, Box::new(op), args)
                }
                "vogls.display" => {
                    c.trim_cursor();
                    let dyn_format_string = parse_dyn_format_string(c)?;
                    let args = (0..dyn_format_string.arguments().len())
                        .map(|_| {
                            c.trim_cursor();
                            c.expect_char(',')?;
                            c.trim_cursor();
                            parse_var(c, symbols, gl)
                        })
                        .collect::<Result<Box<[_]>, _>>()?;
                    let op = IntrinsicOp::Display(Box::new(dyn_format_string));

                    symbols
                        .var_sizes
                        .insert(dst.identifier(), VarSize::Resolved(SCALAR_VSIZE));
                    Instruction::Intrinsic(dst, Box::new(op), args)
                }
                "vogls.finish" => {
                    let op = IntrinsicOp::Finish;
                    symbols
                        .var_sizes
                        .insert(dst.identifier(), VarSize::Resolved(SCALAR_VSIZE));
                    Instruction::Intrinsic(dst, Box::new(op), [].into())
                }

                "phi" => {
                    c.trim_cursor();
                    c.expect_char('[')?;
                    let mut srcs = Vec::new();
                    c.trim_cursor();
                    while !c.next_if_equals(']') {
                        if !srcs.is_empty() {
                            c.expect_char(',')?;
                            c.trim_cursor();
                        }

                        let var = parse_var(c, symbols, gl)?;
                        c.trim_cursor();
                        let bb = parse_label_ref(c, symbols, gl)?;
                        srcs.push((bb, var));

                        c.trim_cursor();
                    }
                    if let Some(src_size) = srcs.iter().find_map(|(_, v)| {
                        if let Some(VarSize::Resolved(size)) =
                            symbols.var_sizes.get(&v.identifier())
                        {
                            return Some(*size);
                        }
                        None
                    }) {
                        symbols
                            .var_sizes
                            .insert(dst.identifier(), VarSize::Resolved(src_size));
                    }
                    Instruction::Phi(dst, srcs.into())
                }
                _ => {
                    return Err(Box::new(ParseError {
                        at: c.offset,
                        error: format!("unknown instruction: '{iname}'"),
                    }));
                }
            };

            symbols
                .var_sizes
                .entry(dst.identifier())
                .or_insert_with(|| VarSize::Unresolved(i.clone()));
            instrs.push(i);
        } else {
            let start = c.offset;
            let ident = c.take_ident()?;
            if c.is_next_equal_to(':') {
                c.offset = start;
                break;
            }

            if terminator.is_some() {
                return Err(Box::new(ParseError {
                    at: c.offset,
                    error: format!("already saw terminator"),
                }));
            }

            if ident == "drv" {
                c.trim_cursor();
                let signal = parse_signal(c, symbols)?;

                c.trim_cursor();
                c.expect_char(',')?;

                c.trim_cursor();
                let src = parse_var(c, symbols, gl)?;
                instrs.push(Instruction::Drive(signal, src, None));
                continue;
            }

            use BasicBlockTerminator as T;
            terminator = Some(match ident {
                "wait" => {
                    c.trim_cursor();
                    c.expect_char('#')?;
                    let time = parse_u64(c)?;
                    c.trim_cursor();
                    c.expect_char(',')?;
                    c.trim_cursor();
                    let next = parse_label_ref(c, symbols, gl)?;
                    T::Wait(next, Time(time))
                }
                "varwait" => {
                    c.trim_cursor();
                    let time = parse_var(c, symbols, gl)?;
                    c.trim_cursor();
                    c.expect_char(',')?;
                    c.trim_cursor();
                    let next = parse_label_ref(c, symbols, gl)?;
                    T::VariableWait(next, time)
                }
                "waitregion" => {
                    c.trim_cursor();
                    let region = parse_u8(c)?;
                    c.trim_cursor();
                    c.expect_char(',')?;
                    c.trim_cursor();
                    let next = parse_label_ref(c, symbols, gl)?;
                    T::WaitRegion(next, region)
                }
                "watch" => {
                    c.trim_cursor();

                    let mut signals = Vec::new();
                    c.expect_char('[')?;
                    signals.push(parse_signal(c, symbols)?);
                    c.trim_cursor();
                    while c.next_if_equals(',') {
                        signals.push(parse_signal(c, symbols)?);
                        c.trim_cursor();
                    }
                    c.expect_char(']')?;
                    c.trim_cursor();

                    c.expect_char(',')?;
                    c.trim_cursor();
                    let next = parse_label_ref(c, symbols, gl)?;
                    T::Watch(next, signals)
                }
                "jump" => {
                    c.trim_cursor();
                    let next = parse_label_ref(c, symbols, gl)?;
                    T::Jump(next)
                }
                "branch" => {
                    let condition = parse_var(c, symbols, gl)?;
                    c.trim_cursor();
                    c.expect_char(',')?;
                    c.trim_cursor();
                    let truthy = parse_label_ref(c, symbols, gl)?;
                    c.trim_cursor();
                    c.expect_char(',')?;
                    c.trim_cursor();
                    let falsy = parse_label_ref(c, symbols, gl)?;
                    T::Branch(condition, truthy, falsy)
                }
                "halt" => T::Halt,
                _ => {
                    return Err(Box::new(ParseError {
                        at: c.offset,
                        error: format!("unknown instruction: '{ident}'"),
                    }));
                }
            });
        }
    }

    let Some(terminator) = terminator else {
        return Err(Box::new(ParseError {
            at: c.offset,
            error: format!("missing terminator"),
        }));
    };
    gl.bbs[bb_key] = crate::BasicBlock { instrs, terminator };

    Ok(())
}

fn parse_var<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
) -> Result<VariableKey, Box<ParseError>> {
    c.trim_cursor();
    c.expect_char('%')?;
    let ident = c.take_ident()?;
    let var = symbols
        .variables
        .entry(ident)
        .or_insert_with(|| (false, gl.vars.insert(SCALAR_VSIZE)))
        .1;
    Ok(var)
}

fn parse_signal<'a>(
    c: &mut Cursor<'a>,
    symbols: &Symbols<'a>,
) -> Result<SignalKey, Box<ParseError>> {
    c.trim_cursor();
    c.expect_char('$')?;
    let start_offset = c.offset;
    let ident = c.take_ident()?;
    Ok(*symbols.signals.get(ident).ok_or_else(|| {
        Box::new(ParseError {
            at: start_offset,
            error: format!("unknown signal: '{ident}'"),
        })
    })?)
}

fn parse_imm(c: &mut Cursor) -> Result<Bits, Box<ParseError>> {
    let size = parse_nonzero_u32(c)?;
    c.expect_char('\'')?;
    match c.content.as_bytes()[c.offset] {
        b'h' | b'H' | b'x' | b'X' => {
            c.offset += 1;
            let s = &c.content[c.offset..];
            let length = s
                .find(|c| !matches!(c, 'a'..='f' | 'A'..='F' | 'x' | 'z' | '0'..='9' | '_'))
                .unwrap_or(s.len());
            c.offset += length;
            Bits::parse_hexadecimal(&s[..length], size).map_err(|_| {
                Box::new(ParseError {
                    at: c.offset,
                    error: format!("invalid hex"),
                })
            })
        }
        b'b' | b'B' => {
            c.offset += 1;
            let s = &c.content[c.offset..];
            let length = s
                .find(|c| !matches!(c, '0' | '1' | 'x' | 'z' | '_'))
                .unwrap_or(s.len());
            c.offset += length;
            Bits::parse_hexadecimal(&s[..length], size).map_err(|_| {
                Box::new(ParseError {
                    at: c.offset,
                    error: format!("invalid binary"),
                })
            })
        }
        _ => {
            return Err(Box::new(ParseError {
                at: c.offset,
                error: format!("invalid base"),
            }));
        }
    }
}

fn parse_u8<'a>(c: &mut Cursor) -> Result<u8, Box<ParseError>> {
    c.trim_cursor();
    let start_offset = c.offset;
    let bs = c.content.as_bytes();
    if !bs[c.offset].is_ascii_digit() {
        return Err(Box::new(ParseError {
            at: c.offset,
            error: format!("expected digit, got '{}'", char::from(bs[c.offset])),
        }));
    }

    let mut current = bs[c.offset] - b'0';
    c.offset += 1;
    while bs[c.offset].is_ascii_digit() {
        current = current
            .checked_mul(10)
            .ok_or_else(|| Box::new(ParseError::overflow(start_offset)))?;
        current = current
            .checked_add(bs[c.offset] - b'0')
            .ok_or_else(|| Box::new(ParseError::overflow(start_offset)))?;
        c.offset += 1;
    }
    Ok(current)
}

fn parse_u32<'a>(c: &mut Cursor) -> Result<u32, Box<ParseError>> {
    c.trim_cursor();
    let start_offset = c.offset;
    let bs = c.content.as_bytes();
    if !bs[c.offset].is_ascii_digit() {
        return Err(Box::new(ParseError {
            at: c.offset,
            error: format!("expected digit, got '{}'", char::from(bs[c.offset])),
        }));
    }

    let mut current = (bs[c.offset] - b'0') as u32;
    c.offset += 1;
    while bs[c.offset].is_ascii_digit() {
        current = current
            .checked_mul(10)
            .ok_or_else(|| Box::new(ParseError::overflow(start_offset)))?;
        current = current
            .checked_add((bs[c.offset] - b'0') as u32)
            .ok_or_else(|| Box::new(ParseError::overflow(start_offset)))?;
        c.offset += 1;
    }
    Ok(current)
}

fn parse_u64<'a>(c: &mut Cursor) -> Result<u64, Box<ParseError>> {
    c.trim_cursor();
    let start_offset = c.offset;
    let bs = c.content.as_bytes();
    if !bs[c.offset].is_ascii_digit() {
        return Err(Box::new(ParseError {
            at: c.offset,
            error: format!("expected digit, got '{}'", char::from(bs[c.offset])),
        }));
    }

    let mut current = (bs[c.offset] - b'0') as u64;
    c.offset += 1;
    while bs[c.offset].is_ascii_digit() {
        current = current
            .checked_mul(10)
            .ok_or_else(|| Box::new(ParseError::overflow(start_offset)))?;
        current = current
            .checked_add((bs[c.offset] - b'0') as u64)
            .ok_or_else(|| Box::new(ParseError::overflow(start_offset)))?;
        c.offset += 1;
    }
    Ok(current)
}

fn parse_nonzero_u32(c: &mut Cursor) -> Result<NonZeroU32, Box<ParseError>> {
    let start_offset = c.offset;
    let num = parse_u32(c)?;
    NonZeroU32::new(num).ok_or_else(|| {
        Box::new(ParseError {
            at: start_offset,
            error: format!("Expected non-zero, got {num}"),
        })
    })
}

fn parse_braced_size(c: &mut Cursor) -> Result<NonZeroU32, Box<ParseError>> {
    c.trim_cursor();
    c.expect_char('[')?;
    c.trim_cursor();
    let size = parse_nonzero_u32(c)?;
    c.trim_cursor();
    c.expect_char(']')?;
    Ok(size)
}

fn parse_label_ref<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
) -> Result<BasicBlockKey, Box<ParseError>> {
    c.trim_cursor();
    c.expect_char('<')?;
    let name = c.take_ident()?;
    c.expect_char('>')?;
    Ok(symbols
        .bbs
        .entry(name)
        .or_insert_with(|| {
            (
                false,
                gl.bbs.insert(BasicBlock {
                    instrs: Vec::new(),
                    terminator: BasicBlockTerminator::Halt,
                }),
            )
        })
        .1)
}

fn parse_unary<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
    dst: VariableKey,
    op: UnaryOp,
) -> Result<Instruction, Box<ParseError>> {
    c.trim_cursor();
    let src = parse_var(c, symbols, gl)?;
    match op {
        UnaryOp::Neg => {
            if let Some(VarSize::Resolved(src_size)) = symbols.var_sizes.get(&src.identifier()) {
                symbols
                    .var_sizes
                    .insert(dst.identifier(), VarSize::Resolved(*src_size));
            }
        }
        UnaryOp::ReduceOr | UnaryOp::ReduceAnd | UnaryOp::ReduceXor => {
            symbols
                .var_sizes
                .insert(dst.identifier(), VarSize::Resolved(SCALAR_VSIZE));
        }
        UnaryOp::LeadingZeros => {
            symbols
                .var_sizes
                .insert(dst.identifier(), VarSize::Resolved(VSIZE_32));
        }
    }
    Ok(Instruction::Unary(dst, op, src))
}

fn parse_resize<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
    dst: VariableKey,
    op: ResizeOp,
) -> Result<Instruction, Box<ParseError>> {
    let size = parse_braced_size(c)?;
    c.trim_cursor();
    let src = parse_var(c, symbols, gl)?;
    symbols
        .var_sizes
        .insert(dst.identifier(), VarSize::Resolved(size));
    Ok(Instruction::Resize(dst, op, src))
}

fn parse_binary<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
    dst: VariableKey,
    op: BinaryOp,
) -> Result<Instruction, Box<ParseError>> {
    c.trim_cursor();
    let lhs = parse_var(c, symbols, gl)?;

    c.trim_cursor();
    c.expect_char(',')?;

    c.trim_cursor();
    let rhs = parse_var(c, symbols, gl)?;

    if let Some(VarSize::Resolved(lhs_size)) = symbols.var_sizes.get(&lhs.identifier())
        && let Some(VarSize::Resolved(rhs_size)) = symbols.var_sizes.get(&rhs.identifier())
    {
        let Some(dst_size) = op.output_size(*lhs_size, *rhs_size) else {
            return Err(Box::new(ParseError {
                at: c.offset,
                error: format!("invalid size combination: {lhs_size}, {rhs_size}"),
            }));
        };
        symbols
            .var_sizes
            .insert(dst.identifier(), VarSize::Resolved(dst_size));
    }

    Ok(Instruction::Binary(dst, op, lhs, rhs))
}

fn parse_binary_imm<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
    dst: VariableKey,
    op: BinaryImmOp,
) -> Result<Instruction, Box<ParseError>> {
    c.trim_cursor();
    let src = parse_var(c, symbols, gl)?;

    c.trim_cursor();
    c.expect_char(',')?;

    c.trim_cursor();
    let imm = parse_imm(c)?;

    if let Some(VarSize::Resolved(src_size)) = symbols.var_sizes.get(&src.identifier()) {
        let Some(dst_size) = op.output_size(*src_size, imm.size()) else {
            return Err(Box::new(ParseError {
                at: c.offset,
                error: format!("invalid size combination: {src_size}, {}", imm.size()),
            }));
        };
        symbols
            .var_sizes
            .insert(dst.identifier(), VarSize::Resolved(dst_size));
    }

    Ok(Instruction::BinaryImm(dst, op, src, imm))
}

fn parse_shift_imm<'a>(
    c: &mut Cursor<'a>,
    symbols: &mut Symbols<'a>,
    gl: &mut GlobalContext,
    dst: VariableKey,
    op: ShiftImmOp,
) -> Result<Instruction, Box<ParseError>> {
    c.trim_cursor();
    let src = parse_var(c, symbols, gl)?;

    c.trim_cursor();
    c.expect_char(',')?;

    c.trim_cursor();
    let amount = parse_u32(c)?;

    if let Some(VarSize::Resolved(src_size)) = symbols.var_sizes.get(&src.identifier()) {
        symbols
            .var_sizes
            .insert(dst.identifier(), VarSize::Resolved(*src_size));
    }

    Ok(Instruction::ShiftImm(dst, op, src, amount))
}

fn parse_dyn_format_string(c: &mut Cursor) -> Result<DynFormatString, Box<ParseError>> {
    c.trim_cursor();
    let start = c.offset;
    c.expect_char('"')?;

    let mut last_copy = c.offset;
    let mut args = Vec::new();
    let mut content = String::new();
    let mut is_escaped = false;
    while let Some(&b) = c.content.as_bytes().get(c.offset)
        && (is_escaped || b != b'"')
    {
        match b {
            b'{' if c.content.as_bytes().get(c.offset + 1) == Some(&b'{') => {
                content.push_str(&c.content[last_copy..c.offset + 1]);
                last_copy = c.offset + 2;
                c.offset += 1;
            }
            b'{' => {
                let Some(end) = c.content[c.offset + 1..].find('}') else {
                    return Err(Box::new(ParseError {
                        at: start,
                        error: format!("unclosed format arg"),
                    }));
                };

                let options = parse_format_options(&c.content[c.offset + 1..][..end])?;

                content.push_str(&c.content[last_copy..c.offset]);
                args.push((content.len(), options));
                c.offset += 1 + end;
                last_copy = c.offset + 1;
            }
            b'\\' | b'"' if is_escaped => {
                content.push_str(&c.content[last_copy..c.offset - 1]);
                last_copy = c.offset;
            }

            b'n' if is_escaped => {
                content.push_str(&c.content[last_copy..c.offset - 1]);
                content.push('\n');
                last_copy = c.offset + 1;
            }
            b't' if is_escaped => {
                content.push_str(&c.content[last_copy..c.offset - 1]);
                content.push('\t');
                last_copy = c.offset + 1;
            }
            b'r' if is_escaped => {
                content.push_str(&c.content[last_copy..c.offset - 1]);
                content.push('\r');
                last_copy = c.offset + 1;
            }
            _ => {}
        }

        is_escaped = !is_escaped && b == b'\\';
        c.offset += 1;
    }

    if c.is_empty() {
        return Err(Box::new(ParseError {
            at: start,
            error: format!("unclosed string"),
        }));
    }

    content.push_str(&c.content[last_copy..c.offset]);
    c.offset += 1;

    Ok(DynFormatString::new(content.into(), args.into()))
}

fn parse_format_options(s: &str) -> Result<DynFormatArgument, Box<ParseError>> {
    if s.is_empty() {
        return Ok(DynFormatArgument::default());
    }

    todo!()
}
