use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Write};

use crate::time::TimeFormat;
use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, GlobalContext,
    Instruction, IntrinsicOp, LogicMode, Process, RandomKind, ResizeOp, ShiftImmOp, Signal, Time,
    UnaryOp, VariableKey,
};

const INDENT: &str = "  ";

pub struct ContextDisplay<'a, T: ?Sized + ContextFormat> {
    item: &'a T,
    ctx: &'a DisplayContext<'a>,
}

pub struct DisplayContext<'a> {
    gl: &'a GlobalContext,

    num_trs: u32,
    bb_stack_scratch: Vec<BasicBlockKey>,
    bb_seen_scratch: HashSet<BasicBlockKey>,
    bb_name_scratch: HashMap<BasicBlockKey, u32>,

    var_map: HashMap<VariableKey, u32>,
}

impl<'a> DisplayContext<'a> {
    pub fn new(gl: &'a GlobalContext) -> Self {
        Self {
            gl,
            num_trs: 0,
            bb_stack_scratch: Vec::new(),
            bb_seen_scratch: HashSet::new(),
            bb_name_scratch: HashMap::new(),
            var_map: HashMap::new(),
        }
    }

    pub fn prepare_process(&mut self, entry: BasicBlockKey) {
        let name = self.bb_name_scratch.len();
        self.bb_name_scratch.entry(entry).or_insert_with(|| {
            self.bb_stack_scratch.push(entry);
            if self.gl.bbs[entry].region.entry() == entry {
                let name = self.num_trs;
                self.num_trs += 1;
                name
            } else {
                name as u32
            }
        });

        while let Some(bb) = self.bb_stack_scratch.pop() {
            self.gl.bbs[bb].for_each_var(|v| {
                let new_idx = self.var_map.len() as u32;
                self.var_map.entry(v).or_insert(new_idx);
            });
            self.gl.bbs[bb].terminator.for_each_temporal_bb(|k| {
                let name = self.bb_name_scratch.len();
                self.bb_name_scratch.entry(k).or_insert_with(|| {
                    self.bb_stack_scratch.push(k);
                    if self.gl.bbs[entry].region.entry() == entry {
                        let name = self.num_trs;
                        self.num_trs += 1;
                        name
                    } else {
                        name as u32
                    }
                });
            });
        }
    }

    pub fn get_bb_idx(&self, bb: BasicBlockKey) -> Option<u32> {
        self.bb_name_scratch.get(&bb).copied()
    }
    pub fn get_var_name(&self, var: VariableKey) -> Option<u32> {
        self.var_map.get(&var).copied()
    }

    fn clear(&mut self) {
        let Self {
            gl: _,
            num_trs,
            bb_stack_scratch,
            bb_seen_scratch,
            bb_name_scratch,
            var_map,
        } = self;
        *num_trs = 0;
        bb_stack_scratch.clear();
        bb_seen_scratch.clear();
        bb_name_scratch.clear();
        var_map.clear();
    }
}

impl<'a, T: ?Sized + ContextFormat> fmt::Display for ContextDisplay<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.item.ctx_fmt(f, self.ctx)
    }
}

pub trait ContextFormat {
    fn display<'a>(&'a self, ctx: &'a DisplayContext<'a>) -> ContextDisplay<'a, Self> {
        ContextDisplay { item: self, ctx }
    }
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &DisplayContext<'_>) -> fmt::Result;
}

impl Signal {
    pub fn display<'a>(&'a self) -> impl fmt::Display + 'a {
        struct D<'a>(&'a Signal);
        impl<'a> fmt::Display for D<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let Signal {
                    name,
                    size,
                    flags: _,
                    initialize,
                    mode,
                    origin: _,
                } = self.0;
                let mode = match mode {
                    LogicMode::TwoValue => "tv",
                    LogicMode::FourValue => "fv",
                };
                write!(f, "signal[{mode}] {name}: {size}")?;
                if let Some(initialize) = initialize {
                    write!(f, " = {initialize}")?;
                }
                Ok(())
            }
        }
        D(self)
    }
}

impl Process {
    pub fn display<'a>(&'a self, gl: &'a GlobalContext) -> impl fmt::Display + 'a {
        struct D<'a>(&'a Process, &'a GlobalContext);
        impl<'a> fmt::Display for D<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut ctx = DisplayContext::new(self.1);
                self.0.process_fmt(f, &mut ctx)
            }
        }
        D(self, gl)
    }

    fn process_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        writeln!(f, "proc {} {{", self.kind.into_static_str())?;

        ctx.clear();
        for tr in &self.regions {
            ctx.prepare_process(tr.entry());
        }
        for tr in &self.regions {
            let entry = tr.entry();

            let mut bb_stack = std::mem::take(&mut ctx.bb_stack_scratch);
            let mut bb_seen = std::mem::take(&mut ctx.bb_seen_scratch);

            bb_seen.clear();
            bb_seen.insert(entry);
            bb_stack.push(entry);
            while let Some(bb) = bb_stack.pop() {
                LabelDisplay {
                    include_prefix: true,
                    angles: false,
                    bb,
                }
                .ctx_fmt(f, ctx)?;
                writeln!(f, ":")?;

                let bb = ctx.gl.bbs.get(bb).unwrap();
                bb.ctx_fmt(f, ctx)?;
                bb.terminator.for_each_non_temporal_bb(|bb_key| {
                    if bb_seen.insert(bb_key) {
                        bb_stack.push(bb_key);
                    }
                });
            }

            ctx.bb_stack_scratch = bb_stack;
            ctx.bb_seen_scratch = bb_seen;
        }

        writeln!(f, "}}")?;

        Ok(())
    }
}

impl ContextFormat for BasicBlock {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &DisplayContext<'_>) -> fmt::Result {
        for i in &self.instrs {
            f.write_str(INDENT)?;
            i.ctx_fmt(f, ctx)?;
            writeln!(f)?;
        }

        f.write_str(INDENT)?;
        self.terminator.ctx_fmt(f, ctx)?;
        writeln!(f)?;
        Ok(())
    }
}

impl BinaryOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::AndNot => "andnot",
            Self::OrNot => "ornot",
            Self::Xnor => "xnor",
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Power => "pow",
            Self::Multiply => "mul",
            Self::DivideX => "divx",
            Self::Divide0 => "div0",
            Self::ModulusX => "remx",
            Self::Modulus0 => "rem0",

            Self::UnsignedLessEqual => "ule",
            Self::CaseEquality => "ceq",
            Self::LogicalShiftLeft => "lsl",
            Self::LogicalShiftRight => "lsr",
            Self::ArithmeticShiftRight => "asr",
            Self::Concat => "concat",

            Self::CopyX => "copyx",
            Self::CopyZ => "copyz",

            Self::Min => "min",
            Self::Max => "max",

            Self::Negedge => "negedge",

            Self::RealAdd => "real.add",
            Self::RealSub => "real.sub",
            Self::RealMul => "real.mul",
            Self::RealDiv => "real.div",
            Self::RealPow => "real.pow",
            Self::RealEq => "real.eq",
            Self::RealNe => "real.ne",
            Self::RealLt => "real.lt",
            Self::RealLeq => "real.leq",
            Self::RealGt => "real.gt",
            Self::RealGeq => "real.geq",
            Self::RealATan2 => "real.atan2",
            Self::RealHypot => "real.hypot",
        }
    }
}

impl BinaryImmOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::And => "andi",
            Self::Or => "ori",
            Self::Xor => "xori",
            Self::Add => "addi",
            Self::Sub => "subi",
            Self::Power => "powi",
            Self::Multiply => "muli",
            Self::Divide => "divi",
            Self::Modulus => "remi",
            Self::RevSub => "revsubi",
            Self::RevPower => "revpowi",
            Self::RevDivideX => "revdivxi",
            Self::RevDivide0 => "revdiv0i",
            Self::RevModulusX => "revremxi",
            Self::RevModulus0 => "revrem0i",

            Self::UnsignedLessEqual => "ulei",
            Self::UnsignedGreaterEqual => "ugei",
            Self::ConcatRight => "concati_right",
            Self::ConcatLeft => "concati_left",

            Self::Min => "mini",
            Self::Max => "maxi",

            Self::CaseEquality => "ceqi",
            Self::BitwiseCaseEquality => "bitwise_ceqi",
        }
    }
}

impl UnaryOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Not => "not",
            Self::ReduceAnd => "reduce_and",
            Self::ReduceOr => "reduce_or",
            Self::ReduceXor => "reduce_xor",
            Self::LeadingZeros => "leading_zeros",
            Self::TvToFv => "tvtofv",
            Self::FvToTv => "fvtotv",

            Self::RealToI64 => "real.to_i64",
            Self::RealToU64 => "real.to_u64",
            Self::RealFromUnsignedDecimal => "real.from_unsigned_decimal",
            Self::RealFromSignedDecimal => "real.from_signed_decimal",

            Self::RealToLogical => "real.to_logical",
            Self::RealNeg => "real.neg",
            Self::RealTruncate => "real.truncate",
            Self::RealLn => "real.ln",
            Self::RealLog10 => "real.log10",
            Self::RealExp => "real.exp",
            Self::RealSqrt => "real.sqrt",
            Self::RealFloor => "real.floor",
            Self::RealCeil => "real.ceil",
            Self::RealSin => "real.sin",
            Self::RealCos => "real.cos",
            Self::RealTan => "real.tan",
            Self::RealASin => "real.asin",
            Self::RealACos => "real.acos",
            Self::RealATan => "real.atan",
            Self::RealSinH => "real.sinh",
            Self::RealCosH => "real.cosh",
            Self::RealTanH => "real.tanh",
            Self::RealASinH => "real.asinh",
            Self::RealACosH => "real.acosh",
            Self::RealATanH => "real.atanh",
        }
    }
}

impl ResizeOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Truncate => "truncate",
            Self::ZeroExtend => "zero_extend",
            Self::SignExtend => "sign_extend",
        }
    }
}
impl IntrinsicOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Finish => "finish",
            Self::Time => "time",
            Self::Random(kind) => match kind {
                RandomKind::Uniform => "dist_uniform",
                RandomKind::Normal => "dist_normal",
                RandomKind::Exponential => "dist_exponential",
                RandomKind::Poisson => "dist_poisson",
                RandomKind::ChiSquare => "dist_chi_square",
                RandomKind::T => "dist_t",
                RandomKind::Erlang => "dist_erlang",
            },
            Self::Display(_) => "vogls.display",
            Self::Assert(_) => "vogls.assert",
            Self::VcdOpenFile(_) => "vcd.open_file",
            Self::VcdAppendModule(_) => "vcd.append_module",
            Self::VcdPause => "vcd.pause",
            Self::VcdResume => "vcd.resume",
            Self::BlackBox => "vogls.black_box",
            Self::ReadMem(_) => "readmem",
            Self::SetTimeFormat(_) => "settimeformat",
        }
    }
}

impl ShiftImmOp {
    pub const fn into_mnemonic(self) -> &'static str {
        match self {
            Self::LogicalShiftLeft => "lsli",
            Self::LogicalShiftRight => "lsri",
            Self::ArithmeticShiftRight => "asri",
        }
    }
}

pub struct LabelDisplay {
    pub include_prefix: bool,
    pub angles: bool,
    pub bb: BasicBlockKey,
}
impl LabelDisplay {
    fn new_angled(bb: BasicBlockKey) -> Self {
        Self {
            include_prefix: false,
            angles: true,
            bb,
        }
    }
}

impl ContextFormat for LabelDisplay {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &DisplayContext<'_>) -> fmt::Result {
        if self.angles {
            f.write_char('<')?;
        }
        if ctx.gl.bbs[self.bb].region.entry() == self.bb {
            if self.include_prefix {
                f.write_char('*')?;
            }
            f.write_str("TR")?;
        } else {
            if self.include_prefix {
                f.write_char('.')?;
            }
            f.write_char('L')?;
        }
        ctx.bb_name_scratch[&self.bb].fmt(f)?;
        if self.angles {
            f.write_char('>')?;
        }
        Ok(())
    }
}

impl ContextFormat for Instruction {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &DisplayContext<'_>) -> fmt::Result {
        match self {
            Self::Constant(var, val) => {
                var.ctx_fmt(f, ctx)?;
                let prefix = match var.mode() {
                    LogicMode::TwoValue => "tv.",
                    LogicMode::FourValue => "fv.",
                };
                write!(f, " = {prefix}const {val}")?;
            }
            Self::Unary(dst, op, src) => {
                write!(
                    f,
                    "{} = {} {}",
                    dst.display(ctx),
                    op.into_mnemonic(),
                    src.display(ctx),
                )?;
            }
            Self::Resize(dst, op, src) => {
                write!(
                    f,
                    "{} = {}[{}] {}",
                    dst.display(ctx),
                    op.into_mnemonic(),
                    ctx.gl.vars.size(*dst),
                    src.display(ctx),
                )?;
            }
            Self::Binary(dst, op, src1, src2) => {
                write!(
                    f,
                    "{} = {} {}, {}",
                    dst.display(ctx),
                    op.into_mnemonic(),
                    src1.display(ctx),
                    src2.display(ctx)
                )?;
            }
            Self::BinaryImm(dst, op, src, imm) => {
                write!(
                    f,
                    "{} = {} {}, {imm}",
                    dst.display(ctx),
                    op.into_mnemonic(),
                    src.display(ctx)
                )?;
            }
            Self::Slice(dst, src, offset) => {
                write!(
                    f,
                    "{} = slice[{}] {}, {}",
                    dst.display(ctx),
                    ctx.gl.vars.size(*dst),
                    src.display(ctx),
                    offset.display(ctx)
                )?;
            }
            Self::SliceImm(dst, src, offset) => {
                write!(
                    f,
                    "{} = slicei[{}] {}, {offset}",
                    dst.display(ctx),
                    ctx.gl.vars.size(*dst),
                    src.display(ctx),
                )?;
            }
            Self::ShiftImm(dst, op, src, amount) => {
                write!(
                    f,
                    "{} = {} {}, {amount}",
                    dst.display(ctx),
                    op.into_mnemonic(),
                    src.display(ctx)
                )?;
            }
            Self::Select(dst, cond, truthy, falsy) => {
                write!(
                    f,
                    "{} = select {}, {}, {}",
                    dst.display(ctx),
                    cond.display(ctx),
                    truthy.display(ctx),
                    falsy.display(ctx),
                )?;
            }
            Self::Intrinsic(dst, op, args) => {
                dst.ctx_fmt(f, ctx)?;
                f.write_str(" = ")?;
                f.write_str(op.into_mnemonic())?;
                f.write_str(" ")?;
                match op.as_ref() {
                    IntrinsicOp::Time => {}
                    IntrinsicOp::Finish => {}
                    IntrinsicOp::Random(_) => {}
                    IntrinsicOp::Display(s) => {
                        s.display_format().fmt(f)?;
                        if !args.is_empty() {
                            f.write_str(", ")?;
                        }
                    }
                    IntrinsicOp::Assert(s) => {
                        s.display_format().fmt(f)?;
                        if !args.is_empty() {
                            f.write_str(", ")?;
                        }
                    }
                    IntrinsicOp::VcdOpenFile(_) => {}
                    IntrinsicOp::VcdAppendModule(_) => {}
                    IntrinsicOp::VcdPause => {}
                    IntrinsicOp::VcdResume => {}
                    IntrinsicOp::BlackBox => {}
                    IntrinsicOp::SetTimeFormat(fmt) => {
                        let TimeFormat {
                            time_unit,
                            precision_number,
                            suffix_string,
                            minimum_field_width,
                        } = fmt;
                        write!(
                            f,
                            "{time_unit:?}, {precision_number}, {suffix_string}, {minimum_field_width}"
                        )?;
                    }
                    IntrinsicOp::ReadMem(_) => {}
                }
                if let Some(arg) = args.first() {
                    arg.ctx_fmt(f, ctx)?;
                    for arg in &args[1..] {
                        f.write_str(", ")?;
                        arg.ctx_fmt(f, ctx)?;
                    }
                }
            }
            Self::LastUpdateTime(var, sig) => {
                var.ctx_fmt(f, ctx)?;
                f.write_str(" = lupdt ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
            }
            Self::Probe(var, sig, offset) => {
                let dst_size = ctx.gl.vars.size(*var);
                let src_size = ctx.gl.signals[*sig].size;
                if *offset > 0 || dst_size != src_size {
                    var.ctx_fmt(f, ctx)?;
                    write!(f, " = prb[{dst_size}] ")?;
                    ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                    write!(f, ", {offset}")?;
                } else {
                    var.ctx_fmt(f, ctx)?;
                    f.write_str(" = prb ")?;
                    ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                }
            }
            Self::ProbeSlice(var, sig, offset) => {
                let dst_size = ctx.gl.vars.size(*var);
                var.ctx_fmt(f, ctx)?;
                write!(f, " = prb[{dst_size}] ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                offset.ctx_fmt(f, ctx)?;
            }
            Self::Drive(dst, sig, var, offset) => {
                dst.ctx_fmt(f, ctx)?;
                f.write_str(" = drv")?;
                f.write_str(" ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                var.ctx_fmt(f, ctx)?;
                if *offset != 0 {
                    f.write_str(", ")?;
                    offset.fmt(f)?;
                }
            }
            Self::DriveSlice(dst, sig, var, offset) => {
                dst.ctx_fmt(f, ctx)?;
                f.write_str(" = drv")?;
                f.write_str(" ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                var.ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                offset.ctx_fmt(f, ctx)?;
            }

            Self::Phi(dst, srcs) => {
                dst.ctx_fmt(f, ctx)?;
                f.write_str(" = phi [")?;
                for (i, (bb, var)) in srcs.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }
                    var.ctx_fmt(f, ctx)?;
                    f.write_str(" ")?;
                    LabelDisplay::new_angled(*bb).ctx_fmt(f, ctx)?;
                }
                f.write_char(']')?;
            }
        }

        Ok(())
    }
}

impl ContextFormat for BasicBlockTerminator {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &DisplayContext<'_>) -> fmt::Result {
        let mnemonic = match self {
            Self::Wait(..) => "wait",
            Self::VariableWait(..) => "varwait",
            Self::WaitRegion(..) => "waitregion",
            Self::Watch(..) => "watch",
            Self::Jump(..) => "jump",
            Self::Branch(..) => "branch",
            Self::Halt => "halt",
        };

        write!(f, "{mnemonic}",)?;

        match self {
            Self::Wait(bb, time) => {
                f.write_char(' ')?;
                time.ctx_fmt(f, ctx)?;
                write!(f, ", ")?;
                LabelDisplay::new_angled(bb.entry()).ctx_fmt(f, ctx)?;
            }
            Self::VariableWait(bb, time) => {
                f.write_char(' ')?;
                time.ctx_fmt(f, ctx)?;
                write!(f, ", ")?;
                LabelDisplay::new_angled(bb.entry()).ctx_fmt(f, ctx)?;
            }
            Self::WaitRegion(bb, region) => {
                f.write_char(' ')?;
                write!(f, "{region}, ")?;
                LabelDisplay::new_angled(bb.entry()).ctx_fmt(f, ctx)?;
            }
            Self::Watch(bb, signals) => {
                f.write_char(' ')?;
                f.write_char('[')?;
                if let Some(fst) = signals.first() {
                    ctx.gl.signals.get(*fst).unwrap().ctx_fmt(f, ctx)?;
                    for s in &signals[1..] {
                        f.write_str(", ")?;
                        ctx.gl.signals.get(*s).unwrap().ctx_fmt(f, ctx)?;
                    }
                }
                f.write_str("], ")?;
                LabelDisplay::new_angled(bb.entry()).ctx_fmt(f, ctx)?;
            }
            Self::Jump(bb) => {
                f.write_char(' ')?;
                LabelDisplay::new_angled(*bb).ctx_fmt(f, ctx)?;
            }
            Self::Branch(var, true_bb, false_bb) => {
                f.write_char(' ')?;
                var.ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                LabelDisplay::new_angled(*true_bb).ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                LabelDisplay::new_angled(*false_bb).ctx_fmt(f, ctx)?;
            }
            Self::Halt => {}
        }

        Ok(())
    }
}

impl ContextFormat for VariableKey {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &DisplayContext<'_>) -> fmt::Result {
        let idx = ctx.var_map[self];
        write!(f, "%t{idx}")
    }
}

impl ContextFormat for Signal {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &DisplayContext<'_>) -> fmt::Result {
        f.write_str("$")?;
        f.write_str(&self.name)?;
        Ok(())
    }
}

impl ContextFormat for Time {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &DisplayContext<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}
