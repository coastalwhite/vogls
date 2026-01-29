use core::fmt;
use std::collections::{HashMap, HashSet};

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction,
    IntrinsicOp, Process, ResizeOp, Signal, Time, UnaryOp, VariableKey,
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
        ctx.bb_name_scratch.clear();

        ctx.bb_name_scratch.insert(self.entry, 0);
        bb_stack.push(self.entry);

        while let Some(bb) = bb_stack.pop() {
            ctx.gl.bbs[bb].for_each_var(|v| {
                let new_idx = ctx.var_map.len() as u32;
                ctx.var_map.entry(v).or_insert(new_idx);
            });
            ctx.gl.bbs[bb].terminator.for_each_bb(|k| {
                let name = ctx.bb_name_scratch.len();
                ctx.bb_name_scratch.entry(k).or_insert_with(|| {
                    bb_stack.push(k);
                    name as u32
                });
            });
        }

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
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Modulus => "rem",

            Self::UnsignedLessEqual => "ule",
            Self::CaseEquality => "ceq",
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
                    src2.display(ctx),
                )?;
            }
            Self::Intrinsic(dst, op, args) => {
                dst.ctx_fmt(f, ctx)?;
                f.write_str(" = ")?;
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
                var.ctx_fmt(f, ctx)?;
                f.write_str(" = probe ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
            }
            Self::Drive(sig, var, partial) => {
                f.write_str("drive")?;
                if let Some((offset, length)) = partial {
                    f.write_str("[")?;
                    offset.ctx_fmt(f, ctx)?;
                    write!(f, ", {length}]")?;
                }
                f.write_str(" ")?;
                ctx.gl.signals.get(*sig).unwrap().ctx_fmt(f, ctx)?;
                f.write_str(", ")?;
                var.ctx_fmt(f, ctx)?;
            }

            Self::Phi(dst, srcs) => {
                dst.ctx_fmt(f, ctx)?;
                f.write_str(" = phi ")?;
                for (i, (bb, var)) in srcs.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "L{}", ctx.bb_name_scratch[bb])?;
                    f.write_str(" ")?;
                    var.ctx_fmt(f, ctx)?;
                }
            }
        }

        Ok(())
    }
}

impl ContextFormat for BasicBlockTerminator {
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, ctx: &DisplayContext<'_>) -> fmt::Result {
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
                write!(f, "L{}, ", ctx.bb_name_scratch[bb])?;
                time.ctx_fmt(f, ctx)?
            }
            Self::WaitRegion(bb, region) => {
                write!(f, "L{}, {region}", ctx.bb_name_scratch[bb])?;
            }
            Self::Watch(bb, signals) => {
                write!(f, "L{}", ctx.bb_name_scratch[bb])?;
                for s in signals {
                    f.write_str(", ")?;
                    ctx.gl.signals.get(*s).unwrap().ctx_fmt(f, ctx)?;
                }
            }
            Self::Jump(bb) => {
                write!(f, "L{}", ctx.bb_name_scratch[bb])?;
            }
            Self::Branch(var, true_bb, false_bb) => {
                var.ctx_fmt(f, ctx)?;
                write!(
                    f,
                    ", L{}, L{}",
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
    fn ctx_fmt(&self, f: &mut fmt::Formatter<'_>, _ctx: &DisplayContext<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}
