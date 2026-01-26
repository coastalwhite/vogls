use std::fmt::{self, Write};

use super::{StackRef, VmInstruction, VmProcess, VmSignalKey};

impl fmt::Display for StackRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.offset)
    }
}

impl fmt::Display for VmInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(dst, value) => write!(f, "{dst} = const {value}"),

            Self::TvUnary(dst, op, _, src) => write!(f, "{dst} = tv.{} {src}", op.into_mnemonic()),
            Self::TvResize(dst, op, dst_size, _, src) => {
                write!(f, "{dst} = tv.{}[{dst_size}] {src}", op.into_mnemonic())
            }
            Self::TvBinaryArithmetic(dst, op, _, src1, src2) => {
                write!(f, "{dst} = tv.{} {src1}, {src2}", op.into_mnemonic())
            }
            Self::TvBinaryComparison(dst, op, _, src1, src2) => {
                write!(f, "{dst} = tv.{} {src1}, {src2}", op.into_mnemonic())
            }
            Self::TvShift(dst, op, _, src1, src2) => {
                write!(f, "{dst} = tv.{} {src1}, {src2}", op.into_mnemonic())
            }
            Self::TvSelectBit(dst, _, src1, src2) => {
                write!(f, "{dst} = tv.bselect {src1}, {src2}")
            }
            Self::TvConcat(dst, _, src1, _, src2) => {
                write!(f, "{dst} = tv.concat {src1}, {src2}")
            }

            Self::FvUnary(dst, op, dst_size, src) => {
                write!(f, "{dst} = fv.{}[{dst_size}] {src}", op.into_mnemonic())
            }
            Self::FvResize(dst, op, _, _, src) => {
                write!(f, "{dst} = fv.{} {src}", op.into_mnemonic())
            }
            Self::FvBinaryArithmetic(dst, op, _, src1, src2) => {
                write!(f, "{dst} = fv.{} {src1}, {src2}", op.into_mnemonic())
            }
            Self::FvBinaryComparison(dst, op, _, src1, src2) => {
                write!(f, "{dst} = fv.{} {src1}, {src2}", op.into_mnemonic())
            }
            Self::FvShift(dst, op, _, src1, src2) => {
                write!(f, "{dst} = fv.{} {src1}, {src2}", op.into_mnemonic())
            }
            Self::FvSelectBit(dst, _, src1, src2) => {
                write!(f, "{dst} = fv.bselect {src1}, {src2}")
            }
            Self::FvConcat(dst, _, src1, _, src2) => {
                write!(f, "{dst} = fv.concat {src1}, {src2}")
            }

            Self::TvToFv(dst, src, _) => {
                write!(f, "{dst} = tv2fv {src}")
            }
            Self::FvToTv(dst, src, _) => {
                write!(f, "{dst} = fv2tv {src}")
            }

            Self::Intrinsic(dst, op, args) => {
                write!(f, "{dst} = {}", op.into_mnemonic())?;
                if let Some((arg, _)) = args.first() {
                    write!(f, " {arg}")?;
                    for (arg, _) in &args[1..] {
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
        writeln!(f, "vm_process() {{")?;
        let mut labels = Vec::new();
        for i in &self.instructions {
            use VmInstruction as I;
            match i {
                I::Constant(..)
                | I::TvUnary(..)
                | I::TvResize(..)
                | I::TvBinaryArithmetic(..)
                | I::TvBinaryComparison(..)
                | I::TvShift(..)
                | I::TvSelectBit(..)
                | I::TvConcat(..)
                | I::FvUnary(..)
                | I::FvResize(..)
                | I::FvBinaryArithmetic(..)
                | I::FvBinaryComparison(..)
                | I::FvShift(..)
                | I::FvSelectBit(..)
                | I::FvConcat(..)
                | I::TvToFv(..)
                | I::FvToTv(..)
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
