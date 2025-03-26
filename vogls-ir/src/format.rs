use core::fmt;
use std::collections::HashSet;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction,
    IntrinsicArg, IntrinsicVariant, Module, Section, SectionKey, SectionVariant, Signal, SignalKey,
    Time, Type, UnaryOp, Value, Variable, VariableKey,
};

const INDENT: &str = "  ";
const INSTRUCTION_SPACING: usize = 2;

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
        if let Some(s) = self.sections.first() {
            ctx.gl.sections.get(*s).unwrap().ctx_fmt(f, ctx)?;

            for s in &self.sections[1..] {
                writeln!(f)?;
                ctx.gl.sections.get(*s).unwrap().ctx_fmt(f, ctx)?;
            }
        }

        Ok(())
    }
}

impl ContextFormat for Section {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        let variant_mnemonic = match self.variant {
            SectionVariant::Entity => "entity",
            SectionVariant::Process => "process",
            SectionVariant::Function => "function",
        };

        writeln!(f, "{variant_mnemonic} {}() -> {{", self.name)?;

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

macro_rules! mnemonics {
    ({$($pat:pat => $mnemonic:literal,)+} ($($terminator:literal,)+)) => {
        const MAX_MNEMONIC_LENGTH: usize = {
            let mut x = 0;
            $(
            if $mnemonic.len() > x {
                x = $mnemonic.len();
            }
            )+
            $(
            if $terminator.len() > x {
                x = $terminator.len();
            }
            )+
            x
        };
        const fn mnemonic(&self) -> &'static str {
            match self {
                $($pat => $mnemonic,)+
            }
        }
    };
}

impl Instruction {
    mnemonics! {
        {
            Self::Constant(..) => "const",
            Self::Unary(_, UnaryOp::Neg, _) => "neg",
            Self::Binary(_, BinaryOp::And, _, _) => "and",
            Self::Binary(_, BinaryOp::Or, _, _) => "or",
            Self::Binary(_, BinaryOp::Xor, _, _) => "xor",
            Self::Intrinsic(IntrinsicVariant::Display, _) => "vogls.display",
            Self::Intrinsic(IntrinsicVariant::Finish, _) => "vogls.finish",
            Self::Probe(_, _) => "probe",
            Self::Drive(_, _) => "drive",
        }
        ("wait", "watch", "jump", "branch", "halt",)
    }
}

impl BinaryOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
        }
    }
}

impl UnaryOp {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Neg => "neg",
        }
    }
}

impl IntrinsicVariant {
    pub const fn into_mnemonic(&self) -> &'static str {
        match self {
            Self::Display => "display",
            Self::Finish => "finish",
        }
    }
}

impl ContextFormat for Instruction {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &mut DisplayContext<'_>) -> fmt::Result {
        match self {
            Self::Constant(var, val) => {
                ctx.gl.vars.get(*var).unwrap().typed_ctx_fmt(f, ctx)?;
                f.write_str(" = const ")?;
                val.ctx_fmt(f, ctx)?;
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

impl ContextFormat for Time {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &mut DisplayContext<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl ContextFormat for Type {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &mut DisplayContext<'_>) -> fmt::Result {
        match self {
            Type::Bit => f.write_str("b1"),
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
            Self::Value(val) => val.ctx_fmt(f, ctx)?,
        }

        Ok(())
    }
}

impl ContextFormat for Value {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &mut DisplayContext<'_>) -> fmt::Result {
        match self {
            Value::Bit(true) => f.write_str("1"),
            Value::Bit(false) => f.write_str("0"),
        }
    }
}
