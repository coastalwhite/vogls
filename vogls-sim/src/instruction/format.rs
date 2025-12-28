use std::fmt::{self, Write};

use super::{StackRef, VmInstruction, VmIntrinsicArg, VmProcess, VmSignalKey};

impl fmt::Display for StackRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.offset)
    }
}

impl fmt::Display for VmIntrinsicArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // @TODO: Quote escaping
            VmIntrinsicArg::StringLiteral(s) => write!(f, "\"{s}\""),
            VmIntrinsicArg::Variable(stack_ref, _size) => stack_ref.fmt(f),
        }
    }
}

impl fmt::Display for VmInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(dst, value) => write!(f, "{dst} = const {value}"),
            Self::Unary(dst, op, src) => write!(f, "{dst} = {} {src}", op.into_mnemonic()),
            Self::Binary(dst, op, src1, src2) => {
                write!(f, "{dst} = {} {src1}, {src2}", op.into_mnemonic())
            }
            Self::Move(dst, src, _) => {
                write!(f, "{dst} = {src}")
            }
            Self::Intrinsic(dst, op, args) => {
                write!(f, "{dst} = {}", op.into_mnemonic())?;
                if let Some(arg) = args.first() {
                    write!(f, " {arg}")?;
                    for arg in &args[1..] {
                        write!(f, ", {arg}")?;
                    }
                }
                Ok(())
            }
            Self::Probe(dst, signal) => write!(f, "{dst} = probe {signal}"),
            Self::Drive(signal, src, region, partial) => match partial {
                None => write!(f, "drive[r={region}] {signal}, {src}"),
                Some((offset, length)) => {
                    write!(f, "drive[r={region}][{offset}, {length}] {signal}, {src}")
                }
            },
            Self::Wait(time) => write!(f, "wait #{}", time.0),
            Self::WaitRegion(region) => write!(f, "wait.region {region}"),
            Self::Watch(signals) => {
                f.write_str("watch [")?;
                if let Some(signal) = signals.first() {
                    signal.fmt(f)?;
                    for signal in &signals[1..] {
                        write!(f, ", {}", signal)?;
                    }
                }
                f.write_char(']')
            }
            Self::Jump(offset) => write!(f, "jump <{offset}>"),
            Self::Branch(cond, true_offset, false_offset) => {
                write!(f, "branch {cond}, <{true_offset}>, <{false_offset}>")
            }
            Self::Halt => f.write_str("halt"),
        }
    }
}

impl fmt::Display for VmSignalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${}", self.0)
    }
}

impl fmt::Display for VmProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "vm_process() : {} {{", self.bit_stack_size)?;
        let mut labels = Vec::new();
        for i in &self.instructions {
            use VmInstruction as I;
            match i {
                I::Constant(..)
                | I::Unary(..)
                | I::Binary(..)
                | I::Move(..)
                | I::Intrinsic(..)
                | I::Probe(..)
                | I::Drive(..)
                | I::Wait(..)
                | I::WaitRegion(..)
                | I::Watch(..)
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
