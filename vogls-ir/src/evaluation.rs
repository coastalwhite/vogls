use std::collections::HashMap;

use vogls_bits::Bits;

use crate::{
    BasicBlock, BasicBlockKey, BasicBlockTerminator, GlobalContext, Instruction, SignalKey,
    VariableKey,
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
                let src = &variables[src];
                let bits = op.evaluate(src);
                _ = variables.insert(*dst, bits.clone());
            }
            I::Resize(dst, op, src) => {
                let src = &variables[src];
                let dst_size = gl.vars[*dst].size;
                let bits = op.evaluate(src, dst_size);
                _ = variables.insert(*dst, bits.clone());
            }
            I::Binary(dst, op, lhs, rhs) => {
                let lhs = &variables[lhs];
                let rhs = &variables[rhs];
                let bits = op.evaluate(lhs, rhs);
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
