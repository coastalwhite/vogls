use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Write};

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, GlobalContext,
    Instruction, IntrinsicOp, Process, ResizeOp, ShiftImmOp, Signal, Time, UnaryOp, VariableKey,
};

const INDENT: &str = "  ";

pub struct ContextDisplay<'a, T: ?Sized + ContextFormat> {
    item: &'a T,
    ctx: &'a DisplayContext<'a>,
}

pub struct DisplayContext<'a> {
    gl: &'a GlobalContext,

    bb_stack_scratch: Vec<BasicBlockKey>,
    bb_seen_scratch: HashSet<BasicBlockKey>,
    bb_name_scratch: HashMap<BasicBlockKey, u32>,

    var_map: HashMap<VariableKey, u32>,
}

impl<'a> DisplayContext<'a> {
    pub fn new(gl: &'a GlobalContext) -> Self {
        Self {
            gl,
            bb_stack_scratch: Vec::new(),
            bb_seen_scratch: HashSet::new(),
            bb_name_scratch: HashMap::new(),
            var_map: HashMap::new(),
        }
    }

    pub fn prepare_process(&mut self, entry: BasicBlockKey) {
        self.bb_stack_scratch.clear();
        self.bb_name_scratch.clear();

        self.bb_name_scratch.insert(entry, 0);
        self.bb_stack_scratch.push(entry);

        while let Some(bb) = self.bb_stack_scratch.pop() {
            self.gl.bbs[bb].for_each_var(|v| {
                let new_idx = self.var_map.len() as u32;
                self.var_map.entry(v).or_insert(new_idx);
            });
            self.gl.bbs[bb].terminator.for_each_bb(|k| {
                let name = self.bb_name_scratch.len();
                self.bb_name_scratch.entry(k).or_insert_with(|| {
                    self.bb_stack_scratch.push(k);
                    name as u32
                });
            });
        }
    }

    pub fn get_bb_idx(&self, bb: BasicBlockKey) -> Option<u32> {
        self.bb_name_scratch.get(&bb).copied()
    }
}

impl<'a, T: ?Sized + ContextFormat> fmt::Display for ContextDisplay<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.item.ctx_fmt(f, &self.ctx)
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
                    initialize,
                    origin: _,
                } = self.0;
                write!(f, "signal {name}: {size}")?;
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
        ctx.prepare_process(self.entry);

        writeln!(f, "proc {} {{", self.kind.into_static_str())?;

        let mut bb_stack = std::mem::take(&mut ctx.bb_stack_scratch);
        let mut bb_seen = std::mem::take(&mut ctx.bb_seen_scratch);

        bb_seen.clear();
        bb_seen.insert(self.entry);
        bb_stack.push(self.entry);

        while let Some(bb) = bb_stack.pop() {
            writeln!(f, "L{}:", ctx.bb_name_scratch[&bb])?;

            let bb = ctx.gl.bbs.get(bb).unwrap();
            bb.ctx_fmt(f, ctx)?;
            bb.terminator.extend_next_rev(&mut bb_stack, &mut bb_seen);
        }

        ctx.bb_stack_scratch = bb_stack;
        ctx.bb_seen_scratch = bb_seen;

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
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Power => "pow",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Modulus => "rem",

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

            Self::Posedge => "posedge",
            Self::Negedge => "negedge",
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
            Self::RevDivide => "revdivi",
            Self::RevModulus => "revremi",

            Self::UnsignedLessEqual => "ulei",
            Self::UnsignedGreaterEqual => "ugei",
            Self::CaseEquality => "ceqi",
            Self::ConcatRight => "concati_right",
            Self::ConcatLeft => "concati_left",

            Self::Min => "mini",
            Self::Max => "maxi",
        }
    }
}

impl UnaryOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Neg => "negate",
            Self::ReduceAnd => "reduce_and",
            Self::ReduceOr => "reduce_or",
            Self::ReduceXor => "reduce_xor",
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
            Self::Random => "random",
            Self::Display(_) => "vogls.display",
            Self::Assert(_) => "vogls.assert",
            Self::VcdOpenFile(_) => "vcd.open_file",
            Self::VcdAppendModule(_) => "vcd.append_module",
            Self::VcdPause => "vcd.pause",
            Self::VcdResume => "vcd.resume",
            Self::ReadMem(_) => "readmem",
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

impl ContextFormat for Instruction {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &DisplayContext<'_>) -> fmt::Result {
        match self {
            Self::Constant(var, val) => {
                var.ctx_fmt(f, ctx)?;
                write!(f, " = const {val}")?;
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
                    ctx.gl.vars[*dst].size,
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
                    ctx.gl.vars[*dst].size,
                    src.display(ctx),
                    offset.display(ctx)
                )?;
            }
            Self::SliceImm(dst, src, offset) => {
                write!(
                    f,
                    "{} = slicei[{}] {}, {offset}",
                    dst.display(ctx),
                    ctx.gl.vars[*dst].size,
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
                    IntrinsicOp::Random => {}
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
                let dst_size = ctx.gl.vars[*var].size;
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
                let dst_size = ctx.gl.vars[*var].size;
                var.ctx_fmt(f, ctx)?;
                write!(f, " = prb[{dst_size}] ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                offset.ctx_fmt(f, ctx)?;
            }
            Self::Drive(sig, var, offset) => {
                f.write_str("drv")?;
                if let Some((offset, _mask_size)) = offset {
                    f.write_str("[")?;
                    offset.ctx_fmt(f, ctx)?;
                    f.write_str("]")?;
                }
                f.write_str(" ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                var.ctx_fmt(f, ctx)?;
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
                    write!(f, "<L{}>", ctx.bb_name_scratch[bb])?;
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
                write!(f, ", <L{}>", ctx.bb_name_scratch[bb])?;
            }
            Self::VariableWait(bb, time) => {
                f.write_char(' ')?;
                time.ctx_fmt(f, ctx)?;
                write!(f, ", <L{}>", ctx.bb_name_scratch[bb])?;
            }
            Self::WaitRegion(bb, region) => {
                f.write_char(' ')?;
                write!(f, "{region}, <L{}>", ctx.bb_name_scratch[bb])?;
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
                write!(f, "<L{}>", ctx.bb_name_scratch[bb])?;
            }
            Self::Jump(bb) => {
                f.write_char(' ')?;
                write!(f, "<L{}>", ctx.bb_name_scratch[bb])?;
            }
            Self::Branch(var, true_bb, false_bb) => {
                f.write_char(' ')?;
                var.ctx_fmt(f, ctx)?;
                write!(
                    f,
                    ", <L{}>, <L{}>",
                    ctx.bb_name_scratch[true_bb], ctx.bb_name_scratch[false_bb]
                )?;
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
