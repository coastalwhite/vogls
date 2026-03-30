use std::fmt::{self, Write};

use super::{VmInstruction, VmProcess};

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
            Self::TvEdge(dst, op, src1, src2) => {
                write!(f, "{dst} = tv.{} {src1}, {src2}", op.into_mnemonic(),)
            }
            Self::TvShift(dst, op, src1, src2) => {
                write!(
                    f,
                    "{} = tv.{} {src1}, {src2}",
                    dst.offset,
                    op.into_mnemonic()
                )
            }
            Self::TvShiftImm(dst, op, src, amount) => {
                write!(
                    f,
                    "{} = tv.{} {src}, {amount}",
                    dst.offset,
                    op.into_mnemonic()
                )
            }
            Self::TvSlice(dst, src1, src2, fill_with_x) => {
                if *fill_with_x {
                    write!(
                        f,
                        "{} = tv.slicex[{}] {}, {src2}",
                        dst.offset, dst.size, src1.offset
                    )
                } else {
                    write!(
                        f,
                        "{} = tv.slicez[{}] {}, {src2}",
                        dst.offset, dst.size, src1.offset
                    )
                }
            }
            Self::TvSliceImm(dst, src, offset) => {
                write!(
                    f,
                    "{} = tv.slicei[{}] {}, {offset}",
                    dst.offset, dst.size, src.offset
                )
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
            Self::FvEdge(dst, op, src1, src2) => {
                write!(f, "{dst} = fv.{} {src1}, {src2}", op.into_mnemonic(),)
            }
            Self::FvShift(dst, op, src1, src2) => {
                write!(
                    f,
                    "{dst} = fv.{} {src1}, {src2}",
                    op.into_mnemonic(),
                    dst = dst.offset
                )
            }
            Self::FvShiftImm(dst, op, src, amount) => {
                write!(
                    f,
                    "{dst} = fv.{} {src}, {amount}",
                    op.into_mnemonic(),
                    dst = dst.offset
                )
            }
            Self::FvSlice(dst, src1, src2, fill_with_x) => {
                if *fill_with_x {
                    write!(
                        f,
                        "{} = fv.slicex[{}] {}, {src2}",
                        dst.offset, dst.size, src1.offset
                    )
                } else {
                    write!(
                        f,
                        "{} = fv.slicez[{}] {}, {src2}",
                        dst.offset, dst.size, src1.offset
                    )
                }
            }
            Self::FvSliceImm(dst, src, offset) => {
                write!(
                    f,
                    "{} = fv.slicei[{}] {}, {offset}",
                    dst.offset, dst.size, src.offset
                )
            }
            Self::FvConcat(dst, src1, src2) => {
                write!(f, "{dst} = fv.concat {}, {}", src1.offset, src2.offset)
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
                    write!(f, " {}", arg.offset)?;
                    for (arg, _) in &args[1..] {
                        write!(f, ", {}", arg.offset)?;
                    }
                }
                Ok(())
            }
            Self::LastUpdateTime(dst, signal) => {
                write!(f, "{dst} = lastupdatetime {}", signal.as_usize())
            }
            Self::Drive(signal, src, partial) => match partial {
                None => write!(f, "drive {}, {}", signal.as_usize(), src.offset),
                Some(offset) => {
                    write!(
                        f,
                        "drive[{offset}, {length}] {}, {}",
                        src.offset,
                        signal.as_usize(),
                        length = src.size,
                    )
                }
            },
            Self::TvVariableWait(time) => write!(f, "tv.wait.var {}", time),
            Self::FvVariableWait(time) => write!(f, "fv.wait.var {}", time),
            Self::Wait(time) => write!(f, "wait #{}", time.0),
            Self::WaitRegion(region) => write!(f, "wait.region {region}"),
            Self::Watch(signals) => {
                f.write_str("watch [")?;
                if let Some(signal) = signals.first() {
                    signal.as_usize().fmt(f)?;
                    for signal in &signals[1..] {
                        write!(f, ", {}", signal.as_usize())?;
                    }
                }
                f.write_char(']')
            }
            Self::Jump(offset) => write!(f, "jump <{offset}>"),
            Self::TvBranch(cond, true_offset, false_offset) => {
                write!(f, "tv.branch {cond}, <{true_offset}>, <{false_offset}>")
            }
            Self::FvBranch(cond, true_offset, false_offset) => {
                write!(f, "fv.branch {cond}, <{true_offset}>, <{false_offset}>")
            }
            Self::Halt => f.write_str("halt"),
        }
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
                | I::TvEdge(..)
                | I::TvShift(..)
                | I::TvShiftImm(..)
                | I::TvSlice(..)
                | I::TvSliceImm(..)
                | I::TvConcat(..)
                | I::FvUnary(..)
                | I::FvResize(..)
                | I::FvBinaryArithmetic(..)
                | I::FvBinaryComparison(..)
                | I::FvEdge(..)
                | I::FvShift(..)
                | I::FvShiftImm(..)
                | I::FvSlice(..)
                | I::FvSliceImm(..)
                | I::FvConcat(..)
                | I::TvToFv(..)
                | I::FvToTv(..)
                | I::Intrinsic(..)
                | I::LastUpdateTime(..)
                | I::Drive(..)
                | I::TvVariableWait(..)
                | I::FvVariableWait(..)
                | I::Wait(..)
                | I::WaitRegion(..)
                | I::Watch(..)
                | I::Halt => {}
                I::Jump(offset) => labels.push(*offset),
                I::TvBranch(_, true_offset, false_offset)
                | I::FvBranch(_, true_offset, false_offset) => {
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
