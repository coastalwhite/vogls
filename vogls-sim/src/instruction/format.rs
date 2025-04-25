use std::fmt::{self};

use super::{StackRef, VmInstruction, VmIntrinsicArg, VmProcess};

impl fmt::Display for StackRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b{}@{}", self.size, self.offset)
    }
}

impl fmt::Display for VmIntrinsicArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // @TODO: Quote escaping
            VmIntrinsicArg::StringLiteral(s) => write!(f, "\"{s}\""),
            VmIntrinsicArg::Variable(stack_ref) => stack_ref.fmt(f),
        }
    }
}

impl fmt::Display for VmInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(dst, value) => write!(f, "{dst} = const {value:?}"),
            Self::Unary(dst, op, src) => write!(f, "{dst} = {} {src}", op.into_mnemonic()),
            Self::Binary(dst, op, src1, src2) => {
                write!(f, "{dst} = {} {src1}, {src2}", op.into_mnemonic())
            }
            Self::Intrinsic(op, args) => {
                f.write_str(op.into_mnemonic())?;
                if let Some(arg) = args.first() {
                    write!(f, " {arg}")?;
                    for arg in &args[1..] {
                        write!(f, ", {arg}")?;
                    }
                }
                Ok(())
            }
            Self::Probe(dst, signal) => write!(f, "{dst} = probe {signal:?}"),
            Self::Drive(signal, src) => write!(f, "drive {signal:?}, {src}"),
            Self::Wait(time) => write!(f, "wait #{}", time.0),
            Self::Watch(signals) => write!(f, "watch {:?}", signals.as_slice()),
            Self::Jump(offset) => write!(f, "jump <{offset}>"),
            Self::Branch(cond, true_offset, false_offset) => {
                write!(f, "branch {cond}, <{true_offset}>, <{false_offset}>")
            }
            Self::Halt => f.write_str("halt"),
        }
    }
}

impl fmt::Display for VmProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "vm_process() : {} {{", self.stack_size)?;
        let mut labels = Vec::new();
        for i in &self.instructions {
            use VmInstruction as I;
            match i {
                I::Constant(_, _)
                | I::Unary(_, _, _)
                | I::Binary(_, _, _, _)
                | I::Intrinsic(_, _)
                | I::Probe(_, _)
                | I::Drive(_, _)
                | I::Wait(_)
                | I::Watch(_)
                | I::Halt => {}
                I::Jump(offset) => labels.push(*offset),
                I::Branch(_, true_offset, false_offset) => {
                    labels.extend([*true_offset, *false_offset]);
                }
            }
        }

        labels.sort();
        let mut label_idx = 0;
        for (i, instr) in self.instructions.iter().enumerate() {
            if labels.get(label_idx).copied() == Some(i) {
                writeln!(f, "{i}:")?;
                label_idx += 1;
            }

            writeln!(f, "  {instr}")?;

            while label_idx < labels.len() && labels.get(label_idx).copied() == Some(i) {
                label_idx += 1;
            }
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}
