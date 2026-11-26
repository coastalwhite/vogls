use std::collections::HashMap;

use vogls_bits::Bits;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, GlobalContext, Instruction, SelectMerge,
    SignalBounds, SignalKey, SignalSlice, VariableKey,
};

pub fn evaluate(
    gl: &GlobalContext,
    bb: BasicBlockKey,
    signals: &mut HashMap<SignalKey, Bits>,
    variables: &mut HashMap<VariableKey, Bits>,
) {
    evaluate_impl(gl, bb, signals, variables, None);
}

fn evaluate_impl(
    gl: &GlobalContext,
    bb: BasicBlockKey,
    signals: &mut HashMap<SignalKey, Bits>,
    variables: &mut HashMap<VariableKey, Bits>,
    prev_bb: Option<BasicBlockKey>,
) {
    let BasicBlock {
        instrs, terminator, ..
    } = &gl.bbs[bb];

    for instr in instrs {
        use Instruction as I;
        match instr {
            I::Constant(dst, bits) => _ = variables.insert(*dst, bits.clone()),
            I::Unary(dst, op, src) => {
                let src = &variables[src];
                let bits = op.evaluate(src);
                _ = variables.insert(*dst, bits);
            }
            I::Resize(dst, op, src) => {
                let src = &variables[src];
                let dst_size = gl.vars.size(*dst);
                let bits = op.evaluate(src, dst_size);
                _ = variables.insert(*dst, bits);
            }
            I::Binary(dst, op, lhs, rhs) => {
                let lhs = &variables[lhs];
                let rhs = &variables[rhs];
                let bits = op.evaluate(lhs, rhs, gl.vars.size(*dst));
                _ = variables.insert(*dst, bits);
            }
            I::Slice(dst, src, offset) => {
                let src = &variables[src];
                let offset = &variables[offset];
                let bits = match offset.extract_exact_u32() {
                    None => Bits::new_unknown(gl.vars.size(*dst)),
                    Some(offset) => src.slicex(offset, gl.vars.size(*dst)),
                };
                _ = variables.insert(*dst, bits);
            }
            I::BinaryImm(dst, op, src, imm) => {
                let src = &variables[src];
                let bits = op.evaluate(src, imm);
                _ = variables.insert(*dst, bits);
            }
            I::SliceImm(dst, src, offset) => {
                let src = &variables[src];
                let bits = src.slicez(*offset, gl.vars.size(*dst));
                _ = variables.insert(*dst, bits);
            }
            I::ShiftImm(dst, op, src, amount) => {
                let src = &variables[src];
                let bits = op.evaluate(src, *amount);
                _ = variables.insert(*dst, bits);
            }
            I::Select(dst, cond, truthy, falsy, kind) => {
                let cond = &variables[cond];
                let truthy = &variables[truthy];
                let falsy = &variables[falsy];
                let bits = if cond.is_one() {
                    truthy.clone()
                } else if cond.is_equal_to_zero() || matches!(kind, SelectMerge::FalseOnSpecial) {
                    falsy.clone()
                } else {
                    Bits::merge(truthy, falsy)
                };
                _ = variables.insert(*dst, bits);
            }
            I::Intrinsic(_, _, _) => todo!(),
            I::LastUpdateTime(_, _) => todo!(),
            I::Probe(dst, src_signal, offset) => {
                let bits = &signals[src_signal];
                let bits = bits.slicez(*offset, gl.vars.size(*dst));
                _ = variables.insert(*dst, bits);
            }
            I::ProbeSlice(dst, src_signal, offset, bounds) => {
                let dst_size = gl.vars.size(*dst);
                let bits = &signals[src_signal];
                let signal_slice = SignalSlice::with_end(bits.size());

                let Some((target_slice, in_bounds)) = variables[offset]
                    .extract_exact_u32()
                    .and_then(|offset| SignalSlice::from_width(offset, dst_size))
                    .and_then(|tgt_slice| Some((tgt_slice, resolve_bounds(*bounds, variables)?)))
                    .and_then(|(tgt_slice, bounds_slice)| {
                        Some((
                            tgt_slice,
                            tgt_slice
                                .overlapping_slice(bounds_slice)?
                                .overlapping_slice(signal_slice)?,
                        ))
                    })
                else {
                    _ = variables.insert(*dst, Bits::new_unknown(dst_size));
                    continue;
                };

                let mut value = bits.slicex(in_bounds.lsb(), in_bounds.width());
                if target_slice != in_bounds {
                    // Bits outside of the bounds read as `x`.
                    value = Bits::new_unknown(dst_size)
                        .set_slice(in_bounds.lsb() - target_slice.lsb(), &value);
                }
                _ = variables.insert(*dst, value);
            }
            I::Drive(dst, dst_signal, src, offset) => {
                // @Performance. Use the kernels directly to calculate the mask and the result at
                // the same time.
                let old_value = initialized_signal(gl, signals, *dst_signal);
                let src_size = gl.vars.size(*src);
                let src = variables[src].clone();
                let old_value_slice = old_value.slicez(*offset, src_size);
                let update_mask = Bits::bitwise_case_equality(&old_value_slice, &src).bitwise_not();
                let new_value = old_value.set_slice(*offset, &src);
                variables.insert(*dst, update_mask);
                signals.insert(*dst_signal, new_value);
            }
            I::DriveSlice(dst, dst_signal, src, offset, bounds) => {
                let src_size = gl.vars.size(*src);
                let signal_slice = SignalSlice::with_end(signals[dst_signal].size());
                let Some((target_slice, in_bounds)) = variables[offset]
                    .extract_exact_u32()
                    .and_then(|offset| SignalSlice::from_width(offset, src_size))
                    .and_then(|tgt_slice| Some((tgt_slice, resolve_bounds(*bounds, variables)?)))
                    .and_then(|(tgt_slice, bounds_slice)| {
                        Some((
                            tgt_slice,
                            tgt_slice
                                .overlapping_slice(bounds_slice)?
                                .overlapping_slice(signal_slice)?,
                        ))
                    })
                else {
                    _ = variables.insert(*dst, Bits::new_zeroed(src_size));
                    continue;
                };

                // @Performance. Use the kernels directly to calculate the mask and the result at
                // the same time.
                let rel_offset = in_bounds.lsb() - target_slice.lsb();
                let src = variables[src].slicex(rel_offset, in_bounds.width());
                let old_value = initialized_signal(gl, signals, *dst_signal);
                let old_value_slice = old_value.slicex(in_bounds.lsb(), in_bounds.width());
                let part_mask = Bits::bitwise_case_equality(&old_value_slice, &src).bitwise_not();
                let update_mask = Bits::new_zeroed(src_size).set_slice(rel_offset, &part_mask);
                let new_value = old_value.set_slice(in_bounds.lsb(), &src);
                variables.insert(*dst, update_mask);
                signals.insert(*dst_signal, new_value);
            }
            I::Phi(dst, items) => {
                let prev_bb = prev_bb.unwrap();
                let src = items
                    .iter()
                    .find_map(|(i_bb, i_key)| (prev_bb == *i_bb).then_some(*i_key))
                    .unwrap();
                let bits = variables[&src].clone();
                _ = variables.insert(*dst, bits.clone());
            }
        }
    }

    let next_bb = match terminator {
        BasicBlockTerminator::Wait(..)
        | BasicBlockTerminator::VariableWait(..)
        | BasicBlockTerminator::WaitRegion(..)
        | BasicBlockTerminator::Watch(..) => todo!(),
        BasicBlockTerminator::Jump(bb) => *bb,
        BasicBlockTerminator::Branch(condition, truthy, falsy) => {
            if variables[condition].not_eq_zero() {
                *truthy
            } else {
                *falsy
            }
        }
        BasicBlockTerminator::Halt => return,
    };
    evaluate_impl(gl, next_bb, signals, variables, Some(bb))
}

/// The current value of `signal`, which is its initial value if it was not yet driven.
fn initialized_signal<'a>(
    gl: &GlobalContext,
    signals: &'a mut HashMap<SignalKey, Bits>,
    signal: SignalKey,
) -> &'a Bits {
    signals.entry(signal).or_insert_with(|| {
        let signal = &gl.signals[signal];
        match &signal.initialize {
            Some(v) => v.clone(),
            None if signal.mode.is_two_value() => Bits::new_zeroed(signal.size),
            None => Bits::new_unknown(signal.size),
        }
    })
}

/// Resolves `bounds` to the bits of a signal of `signal_size` bits they allow to be accessed.
///
/// Returns `None` if the bounds offset contains `x` or `z`, and `Some(None)` if the bounds lie
/// entirely outside of the signal.
fn resolve_bounds(
    bounds: SignalBounds,
    variables: &HashMap<VariableKey, Bits>,
) -> Option<SignalSlice> {
    match bounds {
        SignalBounds::Constant(bounds) => Some(bounds),
        SignalBounds::Variable { offset, bit_length } => {
            let offset = variables[&offset].extract_exact_u32()?;
            SignalSlice::from_width(offset, bit_length)
        }
    }
}
