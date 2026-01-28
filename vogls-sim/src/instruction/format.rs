use std::fmt::{self, Write};

use super::{StackOffset, VmInstruction, VmProcess, VmSignalKey};

impl fmt::Display for StackOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

impl fmt::Display for VmInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constant(dst, value) => write!(f, "{dst} = const {value}"),

            Self::TvUnary(dst, op, src) => {
                write!(f, "{dst} = tv.{} {}", op.into_mnemonic(), src.offset)
            }
            Self::TvResize(dst, op, src) => {
                write!(
                    f,
                    "{} = tv.{}[{}] {}",
                    dst.offset,
                    op.into_mnemonic(),
                    dst.size,
                    src.offset
                )
            }
            Self::TvBinaryArithmetic(dst, op, src1, src2) => {
                write!(
                    f,
                    "{} = tv.{} {src1}, {src2}",
                    dst.offset,
                    op.into_mnemonic()
                )
            }
            Self::TvBinaryComparison(dst, op, src1, src2) => {
                write!(
                    f,
                    "{dst} = tv.{} {}, {src2}",
                    op.into_mnemonic(),
                    src1.offset
                )
            }
            Self::TvShift(dst, op, src1, src2) => {
                write!(
                    f,
                    "{} = tv.{} {src1}, {src2}",
                    dst.offset,
                    op.into_mnemonic()
                )
            }
            Self::TvSelectBit(dst, src1, src2) => {
                write!(f, "{dst} = tv.bselect {}, {src2}", src1.offset)
            }
            Self::TvConcat(dst, src1, src2) => {
                write!(f, "{dst} = tv.concat {}, {}", src1.offset, src2.offset)
            }

            Self::FvUnary(dst, op, src) => {
                write!(
                    f,
                    "{dst} = fv.{} {src}",
                    op.into_mnemonic(),
                    src = src.offset
                )
            }
            Self::FvResize(dst, op, src) => {
                write!(
                    f,
                    "{} = fv.{}[{}] {}",
                    dst.offset,
                    op.into_mnemonic(),
                    dst.size,
                    src.offset
                )
            }
            Self::FvBinaryArithmetic(dst, op, src1, src2) => {
                write!(
                    f,
                    "{dst} = fv.{} {src1}, {src2}",
                    op.into_mnemonic(),
                    dst = dst.offset
                )
            }
            Self::FvBinaryComparison(dst, op, src1, src2) => {
                write!(
                    f,
                    "{dst} = fv.{} {src1}, {src2}",
                    op.into_mnemonic(),
                    src1 = src1.offset
                )
            }
            Self::FvShift(dst, op, _, src1, src2) => {
                write!(f, "{dst} = fv.{} {src1}, {src2}", op.into_mnemonic())
            }
            Self::FvSelectBit(dst, src1, src2) => {
                write!(f, "{dst} = fv.bselect {}, {src2}", src1.offset)
            }
            Self::FvConcat(dst, _, src1, _, src2) => {
                write!(f, "{dst} = fv.concat {src1}, {src2}")
            }

            Self::TvToFv(dst, src) => {
                write!(f, "{} = tv2fv {src}", dst.offset)
            }
            Self::FvToTv(dst, src) => {
                write!(f, "{} = fv2tv {src}", dst.offset)
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
            Self::Drive(signal, src, partial) => match partial {
                None => write!(f, "drive {signal}, {}", src.offset),
                Some(offset) => {
                    write!(
                        f,
                        "drive[{offset}, {length}] {signal}, {}",
                        src.offset,
                        length = src.size,
                    )
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
