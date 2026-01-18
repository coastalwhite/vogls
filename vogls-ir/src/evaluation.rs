use std::collections::HashMap;

use vogls_bits::Bits;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction,
    ResizeOp, SignalKey, UnaryOp, VariableKey,
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
    let BasicBlock { instrs, terminator } = &gl.bbs[bb];

    for instr in instrs {
        use Instruction as I;
        match instr {
            I::Constant(dst, bits) => _ = variables.insert(*dst, bits.clone()),
            I::Unary(dst, op, src) => {
                use UnaryOp as O;
                let src = &variables[src];
                let bits = match op {
                    O::Copy => src.clone(),
                    O::Neg => src.bitwise_negate(),
                    O::ReduceOr => Bits::from(src.reduce_or()),
                    O::ReduceAnd => Bits::from(src.reduce_and()),
                    O::ReduceXor => Bits::from(src.reduce_xor()),
                };
                _ = variables.insert(*dst, bits.clone());
            }
            I::Resize(dst, op, src) => {
                use ResizeOp as O;
                let src = &variables[src];
                let dst_size = gl.vars[*dst].size;
                let bits = match op {
                    O::Truncate => src.truncate(dst_size),
                    O::ZeroExtend => src.zero_extend(dst_size),
                    O::SignExtend => src.sign_extend(dst_size),
                };
                _ = variables.insert(*dst, bits.clone());
            }
            I::Binary(dst, op, lhs, rhs) => {
                use BinaryOp as O;
                let lhs = &variables[lhs];
                let rhs = &variables[rhs];
                let bits = match op {
                    O::And => Bits::bitwise_and(lhs, rhs),
                    O::Or => Bits::bitwise_or(lhs, rhs),
                    O::Xor => Bits::bitwise_xor(lhs, rhs),
                    O::Add => Bits::add(lhs, rhs),
                    O::Sub => Bits::subtract(lhs, rhs),
                    O::Multiply => Bits::multiply(lhs, rhs),
                    O::Divide => Bits::divide(lhs, rhs),
                    O::Modulus => Bits::modulus(lhs, rhs),
                    O::UnsignedLessEqual => Bits::from(Bits::is_unsigned_leq(lhs, rhs)),
                    O::SelectBit => Bits::from(lhs.select_bit(rhs.extract_exact_u32())),
                    O::LogicalShiftLeft => lhs.logical_shift_left(rhs.extract_exact_u32()),
                    O::LogicalShiftRight => lhs.logical_shift_right(rhs.extract_exact_u32()),
                    O::ArithmeticShiftRight => lhs.arithmetic_shift_right(rhs.extract_exact_u32()),
                    O::Concat => Bits::concatenate(lhs, rhs),
                };
                _ = variables.insert(*dst, bits.clone());
            }
            I::Intrinsic(_, _, _) => todo!(),
            I::Probe(dst, src_signal) => {
                let bits = signals[src_signal].clone();
                _ = variables.insert(*dst, bits);
            }
            I::Drive(dst_signal, src, region, partial) => {
                if *region != 0 || partial.is_some() {
                    todo!()
                }

                let bits = variables[src].clone();
                signals.insert(*dst_signal, bits);
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
