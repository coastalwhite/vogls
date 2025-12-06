use core::fmt;
use std::collections::HashSet;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction,
    IntrinsicArg, IntrinsicOp, Module, Process, Signal, Time, Type, UnaryOp, Value, Variable,
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

impl ContextFormat for Module {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        writeln!(f, "module {}:", self.name)?;

        ctx.gl.processes[self.initialize].ctx_fmt(f, ctx)?;
        writeln!(f)?;

        if let Some(s) = self.processes.first() {
            ctx.gl.processes.get(*s).unwrap().ctx_fmt(f, ctx)?;
            for s in &self.processes[1..] {
                writeln!(f)?;
                ctx.gl.processes.get(*s).unwrap().ctx_fmt(f, ctx)?;
            }
        }

        writeln!(f, "endmodule {};", self.name)?;

        Ok(())
    }
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

            use BasicBlockTerminator as T;
            match bb.terminator {
                T::Wait(bb, _) | T::Watch(bb, _) | T::Jump(bb) => {
                    if bb_seen.insert(bb) {
                        bb_stack.push(bb);
                    }
                }
                T::Branch(_, true_bb, false_bb) => {
                    if bb_seen.insert(false_bb) {
                        bb_stack.push(false_bb);
                    }
                    if bb_seen.insert(true_bb) {
                        bb_stack.push(true_bb);
                    }
                }
                T::Halt => {}
            }
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
            Self::BitAnd(_) => "band",
            Self::BitOr(_) => "bor",
            Self::BitXor(_) => "bxor",
            Self::DecimalAnd => "i64and",
            Self::DecimalOr => "i64or",
            Self::DecimalXor => "i64xor",
        }
    }
}

impl UnaryOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::BitNeg(_) => "bneg",
            Self::BitReduceAnd(_) => "bredand",
            Self::BitReduceOr(_) => "bredor",

            Self::DecimalNeg => "i64neg",
            Self::DecimalReduceAnd => "i64redand",
            Self::DecimalReduceOr => "i64redor",
        }
    }
}

impl IntrinsicOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Display => "display",
            Self::Finish => "finish",
            Self::Assert => "assert",
        }
    }
}

impl ContextFormat for Instruction {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        match self {
            Self::ConstantBit(var, val) => {
                ctx.gl.vars.get(*var).unwrap().typed_ctx_fmt(f, ctx)?;
                write!(f, " = bconst {val}")?;
            }
            Self::ConstantDecimal(var, val) => {
                ctx.gl.vars.get(*var).unwrap().typed_ctx_fmt(f, ctx)?;
                write!(f, " = dconst {val}")?;
            }
            Self::Unary(dst, op, src) => {
                ctx.gl.vars.get(*dst).unwrap().typed_ctx_fmt(f, ctx)?;
                f.write_str(" = ")?;
                f.write_str(op.into_mnemonic())?;
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
            Self::Cast(dst, src) => {
                ctx.gl.vars.get(*dst).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(" = cast(")?;
                ctx.gl.vars.get(*src).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                ctx.gl.vars[*dst].ty.ctx_fmt(f, ctx)?;
                f.write_str(")")?;
            }
            Self::Intrinsic(op, args) => {
                f.write_str("vogls.")?;
                f.write_str(op.into_mnemonic())?;
                f.write_str(" ")?;
                if let Some(arg) = args.first() {
                    arg.ctx_fmt(f, ctx)?;
                    for arg in &args[1..] {
                        f.write_str(", ")?;
                        arg.ctx_fmt(f, ctx)?;
                    }
                }
            }
            Self::Probe(var, sig) => {
                ctx.gl.vars.get(*var).unwrap().typed_ctx_fmt(f, ctx)?;
                f.write_str(" = probe ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
            }
            Self::Drive(sig, var) => {
                f.write_str("drive ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                ctx.gl.vars.get(*var).unwrap().typed_ctx_fmt(f, ctx)?;
            }

            Self::Spawn(process, ports) => {
                let process = &ctx.gl.processes[*process];
                write!(f, "spawn @{} (", process.name)?;
                if let Some(sig) = ports.first() {
                    write!(f, "{}", ctx.gl.signals.get(*sig).unwrap().name)?;
                    for sig in ports.iter().skip(1) {
                        write!(f, ", {}", ctx.gl.signals.get(*sig).unwrap().name)?;
                    }
                }
                write!(f, ")")?;
            }

            Self::Instantiate(module, ports) => {
                let module = &ctx.gl.modules[*module];
                write!(f, "instantiate @{} (", module.name)?;
                if let Some(sig) = ports.first() {
                    write!(f, "{}", ctx.gl.signals.get(*sig).unwrap().name)?;
                    for sig in ports.iter().skip(1) {
                        write!(f, ", {}", ctx.gl.signals.get(*sig).unwrap().name)?;
                    }
                }
                write!(f, ")")?;
            }
            Self::Signal(signal) => {
                let signal = ctx.gl.signals.get(*signal).unwrap();
                write!(f, "signal {} {}", signal.ty.display(ctx.gl), signal.name)?;
            }
        }

        Ok(())
    }
}

impl ContextFormat for BasicBlockTerminator {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        let mnemonic = match self {
            Self::Wait(..) => "wait",
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
        self.ty.ctx_fmt(f, ctx)?;
        f.write_str(" ")?;
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
        self.ty.ctx_fmt(f, ctx)?;
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

impl ContextFormat for Type {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &mut DisplayContext<'_>) -> fmt::Result {
        match self {
            Type::Bits(size) => write!(f, "b{size}"),
            Type::Decimal => f.write_str("d"),
        }
    }
}

impl ContextFormat for IntrinsicArg {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        match self {
            Self::StringLiteral(l) => {
                // @TODO: Escape quotes
                f.write_str("\"")?;
                f.write_str(&l)?;
                f.write_str("\"")?;
            }
            Self::Variable(var) => ctx.gl.vars.get(*var).unwrap().ctx_fmt(f, ctx)?,
        }

        Ok(())
    }
}

impl ContextFormat for Value {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &mut DisplayContext<'_>) -> fmt::Result {
        match self {
            Value::Bits(bits) => std::fmt::Display::fmt(&bits, f),
            Value::Decimal(v) => write!(f, "{v}"),
        }
    }
}
