use std::cmp::Ordering;

use vogls_bits::VectorSize;
use vogls_utils::{VgHashMap, VgHashSet};

use crate::token_range::TokenRange;
use crate::{
    BasicBlockKey, GlobalContext, INTEGER_VSIZE, Instruction, LogicMode, Process, ProcessKind,
    ResizeOp, TIME_VSIZE, TemporalRegionKey, VSIZE_32, VariableKey,
};

struct Fail {
    bb: BasicBlockKey,
    instr: Option<usize>,
    reason: FailReason,
}

enum FailReason {
    SizeCombination {
        fst: VectorSize,
        snd: VectorSize,
    },
    SizeMismatch {
        expected: VectorSize,
        given: VectorSize,
    },
    Mode {
        expected: LogicMode,
        given: LogicMode,
    },
    InvalidInputMode {
        given: LogicMode,
    },
}

impl FailReason {
    pub fn assert_mode(expected: LogicMode, given: LogicMode) -> Result<(), FailReason> {
        if expected != given {
            return Err(FailReason::Mode { expected, given });
        }
        Ok(())
    }
    pub fn assert_size(expected: VectorSize, given: VectorSize) -> Result<(), FailReason> {
        if expected != given {
            return Err(FailReason::SizeMismatch { expected, given });
        }
        Ok(())
    }
}

pub fn check_ir_form(regions: &[TemporalRegionKey], gl: &GlobalContext) {
    let mut var_region = VgHashMap::<VariableKey, TemporalRegionKey>::default();

    let mut stack = Vec::<BasicBlockKey>::new();
    let mut seen = VgHashSet::<BasicBlockKey>::default();

    let mut fails = Vec::new();

    for &tr in regions {
        seen.clear();

        stack.push(tr.entry());
        seen.insert(tr.entry());

        while let Some(bb_key) = stack.pop() {
            let bb = &gl.bbs[bb_key];

            bb.terminator.for_each_non_temporal_bb(|next_bb| {
                if seen.insert(next_bb) {
                    stack.push(next_bb);
                }
            });

            assert_eq!(bb.region, tr);

            for (i, instr) in bb.instrs.iter().enumerate() {
                match check_instruction(gl, tr, instr, &mut var_region) {
                    Ok(()) => {}
                    Err(reason) => fails.push(Fail {
                        bb: bb_key,
                        instr: Some(i),
                        reason,
                    }),
                }
            }
        }
    }

    if !fails.is_empty() {
        let process = Process {
            kind: ProcessKind::Other,
            regions: regions.to_vec(),
            origin: TokenRange::default(),
        };
        eprintln!("{}", process.display(gl));
        for fail in fails {
            let Fail { bb, instr, reason } = fail;
            match instr {
                None => {}
                Some(instr) => {
                    eprint!("[{instr}]: {:?}. Reason: ", &gl.bbs[bb].instrs[instr])
                }
            }
            match reason {
                FailReason::SizeCombination { fst, snd } => {
                    eprintln!("size-combination. {fst}, {snd}.")
                }
                FailReason::SizeMismatch { expected, given } => {
                    eprintln!("size-mismatch. Expected {expected}, given: {given}.")
                }
                FailReason::Mode { expected, given } => {
                    eprintln!("mode-mismatch. Expected {expected:?}, given: {given:?}.")
                }
                FailReason::InvalidInputMode { given } => {
                    eprintln!("invalid-input-mode. {given:?}.")
                }
            }
        }
        panic!("IR shape is incorrect. This is a bug in Vogls, please report this as a bug.");
    }
}

fn check_instruction(
    gl: &GlobalContext,
    tr: TemporalRegionKey,
    instr: &Instruction,
    var_region: &mut VgHashMap<VariableKey, TemporalRegionKey>,
) -> Result<(), FailReason> {
    instr.for_each_var(|v| {
        assert_eq!(*var_region.entry(v).or_insert(tr), tr);
    });

    use Instruction as I;
    match instr {
        I::Constant(dst, bits) => {
            FailReason::assert_size(bits.size(), gl.vars.size(*dst))?;
        }
        I::Unary(dst, op, src) => {
            let expected_size = op.output_size(gl.vars.size(*src));
            let dst_size = gl.vars.size(*dst);
            FailReason::assert_size(expected_size, dst_size)?;
            let Some(expected_mode) = op.output_mode(src.mode()) else {
                return Err(FailReason::InvalidInputMode { given: src.mode() });
            };
            FailReason::assert_mode(expected_mode, dst.mode())?;
        }
        I::Resize(dst, op, src) => {
            FailReason::assert_mode(src.mode(), dst.mode())?;

            let dst_size = gl.vars.size(*dst);
            let src_size = gl.vars.size(*src);
            let fail_cmp = match op {
                ResizeOp::Truncate => Ordering::Greater,
                ResizeOp::ZeroExtend => Ordering::Less,
                ResizeOp::SignExtend => Ordering::Less,
            };

            if dst_size.cmp(&src_size) == fail_cmp {
                return Err(FailReason::SizeCombination {
                    fst: src_size,
                    snd: dst_size,
                });
            }
        }
        I::Binary(dst, op, lhs, rhs) => {
            let output_mode = op.output_mode(lhs.mode(), rhs.mode());
            FailReason::assert_mode(lhs.mode(), output_mode.lhs)?;
            FailReason::assert_mode(rhs.mode(), output_mode.rhs)?;
            FailReason::assert_mode(dst.mode(), output_mode.dst)?;
            assert_eq!(
                gl.vars.size(*dst),
                op.output_size(gl.vars.size(*lhs), gl.vars.size(*rhs))
                    .unwrap()
            );
        }
        I::BinaryImm(dst, op, src, imm) => {
            let imm_mode = if imm.contains_special() {
                LogicMode::FourValue
            } else {
                LogicMode::TwoValue
            };
            let output_mode = op.output_mode(src.mode(), imm_mode);
            assert_eq!(src.mode(), output_mode.src);
            assert_eq!(dst.mode(), output_mode.dst);
            assert_eq!(
                gl.vars.size(*dst),
                op.output_size(gl.vars.size(*src), imm.size()).unwrap()
            );
        }
        I::Slice(dst, _, offset) => {
            assert_eq!(dst.mode(), LogicMode::FourValue);
            assert_eq!(gl.vars.size(*offset), VSIZE_32);
        }
        I::SliceImm(dst, src, _) => {
            assert_eq!(dst.mode(), src.mode());
        }
        I::ShiftImm(dst, _, src, _) => {
            assert_eq!(dst.mode(), src.mode());
        }
        I::Select(dst, _, truthy, falsy) => {
            assert_eq!(dst.mode(), truthy.mode());
            assert_eq!(dst.mode(), falsy.mode());
        }
        I::Intrinsic(..) => {}
        I::LastUpdateTime(dst, _) => {
            assert_eq!(dst.mode(), LogicMode::TwoValue);
            assert_eq!(gl.vars.size(*dst), TIME_VSIZE);
        }
        I::Probe(dst, signal, _) => {
            assert_eq!(dst.mode(), gl.signals[*signal].mode);
        }
        I::ProbeSlice(dst, _, offset) => {
            assert_eq!(dst.mode(), LogicMode::FourValue);
            assert_eq!(gl.vars.size(*offset), VSIZE_32);
        }
        I::Drive(signal, src, _) => {
            FailReason::assert_mode(gl.signals[*signal].mode, src.mode())?;
        }
        I::DriveSlice(signal, src, offset) => {
            FailReason::assert_mode(gl.signals[*signal].mode, src.mode())?;
            FailReason::assert_size(VSIZE_32, gl.vars.size(*offset))?;
        }
        I::Phi(..) => {}
    }

    Ok(())
}
