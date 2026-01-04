use core::fmt;
use std::collections::HashSet;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction,
    IntrinsicOp, Process, ResizeOp, Signal, Time, UnaryOp, Variable,
};

const INDENT: &str = "  ";

pub struct ContextDisplay<'a, T: ?Sized + ContextFormat> {
    item: &'a T,
    gl: &'a GlobalContext,
}

pub struct DisplayContext<'a> {
    gl: &'a GlobalContext,

    bb_stack_scratch: Vec<BasicBlockKey>,
    bb_seen_scratch: HashSet<BasicBlockKey>,
}

impl<'a> DisplayContext<'a> {
    pub fn new(gl: &'a GlobalContext) -> Self {
        Self {
            gl,
            bb_stack_scratch: Vec::new(),
            bb_seen_scratch: HashSet::new(),
        }
    }
}

impl<'a, T: ?Sized + ContextFormat> fmt::Display for ContextDisplay<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ctx = DisplayContext::new(self.gl);
        self.item.ctx_fmt(f, &mut ctx)
    }
}

pub trait ContextFormat {
    fn display<'a>(&'a self, gl: &'a GlobalContext) -> ContextDisplay<'a, Self> {
        ContextDisplay { item: self, gl }
    }
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result;
}

impl ContextFormat for Process {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        write!(f, "process {}(", self.name)?;
        if let Some(i) = self.ins.first() {
            ctx.gl.signals.get(*i).unwrap().typed_ctx_fmt(f, ctx)?;
            for i in self.ins.iter().skip(1) {
                f.write_str(", ")?;
                ctx.gl.signals.get(*i).unwrap().typed_ctx_fmt(f, ctx)?;
            }
        }
        write!(f, ")")?;
        if let Some(i) = self.outs.first() {
            f.write_str(" -> ")?;
            if self.outs.len() > 1 {
                f.write_str("(")?;
            }
            ctx.gl.signals.get(*i).unwrap().typed_ctx_fmt(f, ctx)?;
            for i in self.outs.iter().skip(1) {
                f.write_str(", ")?;
                ctx.gl.signals.get(*i).unwrap().typed_ctx_fmt(f, ctx)?;
            }
            if self.outs.len() > 1 {
                f.write_str(")")?;
            }
        }

        writeln!(f, " {{")?;

        let mut bb_stack = std::mem::take(&mut ctx.bb_stack_scratch);
        let mut bb_seen = std::mem::take(&mut ctx.bb_seen_scratch);

        bb_stack.clear();
        bb_seen.clear();

        bb_seen.insert(self.entry);
        bb_stack.push(self.entry);

        while let Some(bb) = bb_stack.pop() {
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
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        writeln!(f, "{}:", self.name)?;
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
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Modulus => "rem",
            Self::UnsignedLessEqual => "ule",
            Self::SelectBit => "bselect",
            Self::LogicalShiftLeft => "lsl",
            Self::LogicalShiftRight => "lsr",
            Self::ArithmeticShiftRight => "asr",
            Self::Concat => "concat",
        }
    }
}

impl UnaryOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Copy => "copy",
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
            Self::Display(_) => "display",
            Self::Assert(_) => "vogls.assert",
            Self::VcdOpenFile(_) => "vcd.open_file",
            Self::VcdAppendModule(_) => "vcd.append_module",
            Self::VcdPause => "vcd.pause",
            Self::VcdResume => "vcd.resume",
        }
    }
}

impl ContextFormat for Instruction {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        match self {
            Self::Constant(var, val) => {
                ctx.gl.vars.get(*var).unwrap().typed_ctx_fmt(f, ctx)?;
                write!(f, " = const {val}")?;
            }
            Self::Unary(dst, op, src) => {
                ctx.gl.vars.get(*dst).unwrap().typed_ctx_fmt(f, ctx)?;
                f.write_str(" = ")?;
                f.write_str(op.into_mnemonic())?;
                match op {
                    UnaryOp::Copy | UnaryOp::Neg | UnaryOp::ReduceOr | UnaryOp::ReduceAnd | UnaryOp::ReduceXor => {}
                }
                f.write_str(" ")?;
                ctx.gl.vars.get(*src).unwrap().ctx_fmt(f, ctx)?;
            }
            Self::Resize(dst, op, src) => {
                ctx.gl.vars.get(*dst).unwrap().typed_ctx_fmt(f, ctx)?;
                f.write_str(" = ")?;
                f.write_str(op.into_mnemonic())?;
                match op {
                    ResizeOp::Truncate | ResizeOp::ZeroExtend | ResizeOp::SignExtend => {
                        write!(f, "[{}]", ctx.gl.vars[*dst].size)?
                    }
                }
                f.write_str(" ")?;
                ctx.gl.vars.get(*src).unwrap().ctx_fmt(f, ctx)?;
            }
            Self::Binary(dst, op, src1, src2) => {
                ctx.gl.vars.get(*dst).unwrap().typed_ctx_fmt(f, ctx)?;
                f.write_str(" = ")?;
                f.write_str(op.into_mnemonic())?;
                f.write_str(" ")?;
                ctx.gl.vars.get(*src1).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                ctx.gl.vars.get(*src2).unwrap().ctx_fmt(f, ctx)?;
            }
            Self::Intrinsic(dst, op, args) => {
                ctx.gl.vars.get(*dst).unwrap().typed_ctx_fmt(f, ctx)?;
                f.write_str(" = ")?;
                f.write_str(op.into_mnemonic())?;
                f.write_str(" ")?;
                if let Some(arg) = args.first() {
                    ctx.gl.vars.get(*arg).unwrap().ctx_fmt(f, ctx)?;
                    for arg in &args[1..] {
                        f.write_str(", ")?;
                        ctx.gl.vars.get(*arg).unwrap().ctx_fmt(f, ctx)?;
                    }
                }
            }
            Self::Probe(var, sig) => {
                ctx.gl.vars.get(*var).unwrap().typed_ctx_fmt(f, ctx)?;
                f.write_str(" = probe ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
            }
            Self::Drive(sig, var, region, partial) => {
                f.write_str("drive")?;
                if *region != 0 {
                    write!(f, "[r={region}] ")?;
                }
                if let Some((offset, length)) = partial {
                    f.write_str("[")?;
                    ctx.gl.vars.get(*offset).unwrap().typed_ctx_fmt(f, ctx)?;
                    write!(f, ", {length}]")?;
                }
                f.write_str(" ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                ctx.gl.vars.get(*var).unwrap().typed_ctx_fmt(f, ctx)?;
            }

            Self::Phi(dst, srcs) => {
                ctx.gl.vars.get(*dst).unwrap().typed_ctx_fmt(f, ctx)?;
                f.write_str(" = phi ")?;
                for (i, (bb, var)) in srcs.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }
                    f.write_str(&ctx.gl.bbs.get(*bb).unwrap().name)?;
                    f.write_str(" ")?;
                    ctx.gl.vars.get(*var).unwrap().typed_ctx_fmt(f, ctx)?;
                }
            }
        }

        Ok(())
    }
}

impl ContextFormat for BasicBlockTerminator {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        let mnemonic = match self {
            Self::Wait(..) => "wait",
            Self::WaitRegion(..) => "wait.region",
            Self::Watch(..) => "watch",
            Self::Jump(..) => "jump",
            Self::Branch(..) => "branch",
            Self::Halt => "halt",
        };

        write!(f, "{mnemonic} ",)?;

        match self {
            Self::Wait(bb, time) => {
                f.write_str(&ctx.gl.bbs.get(*bb).unwrap().name)?;
                f.write_str(", ")?;
                time.ctx_fmt(f, ctx)?
            }
            Self::WaitRegion(bb, region) => {
                f.write_str(&ctx.gl.bbs.get(*bb).unwrap().name)?;
                write!(f, ", {region}")?;
            }
            Self::Watch(bb, signals) => {
                f.write_str(&ctx.gl.bbs.get(*bb).unwrap().name)?;
                for s in signals {
                    f.write_str(", ")?;
                    ctx.gl.signals.get(*s).unwrap().ctx_fmt(f, ctx)?;
                }
            }
            Self::Jump(bb) => {
                f.write_str(&ctx.gl.bbs.get(*bb).unwrap().name)?;
            }
            Self::Branch(var, true_bb, false_bb) => {
                ctx.gl.vars.get(*var).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                let true_bb = &ctx.gl.bbs.get(*true_bb).unwrap().name;
                let false_bb = &ctx.gl.bbs.get(*false_bb).unwrap().name;
                write!(f, "{true_bb}, {false_bb}")?;
            }
            Self::Halt => {}
        }

        Ok(())
    }
}

impl ContextFormat for Variable {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &mut DisplayContext<'_>) -> fmt::Result {
        f.write_str("%")?;
        f.write_str(&self.name)?;
        Ok(())
    }
}

impl Variable {
    fn typed_ctx_fmt(
        &self,

        f: &mut fmt::Formatter<'_>,
        ctx: &mut DisplayContext<'_>,
    ) -> fmt::Result {
        self.ctx_fmt(f, ctx)?;
        Ok(())
    }
}

impl ContextFormat for Signal {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &mut DisplayContext<'_>) -> fmt::Result {
        f.write_str("$")?;
        f.write_str(&self.name)?;
        Ok(())
    }
}

impl Signal {
    fn typed_ctx_fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
        ctx: &mut DisplayContext<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.size)?;
        f.write_str(" ")?;
        self.ctx_fmt(f, ctx)?;
        Ok(())
    }
}

impl ContextFormat for Time {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &mut DisplayContext<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}
