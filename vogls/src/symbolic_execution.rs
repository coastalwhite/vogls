use std::rc::Rc;

use vogls_bits::arithmetic::FvLogicValue;
use vogls_ir::{LogicMode, ResizeOp, UnaryOp};
use vogls_sim::{BinaryArithmeticOp, HeapOffset, Simulation, SimulationState, VmInstruction};
use vogls_utils::VgHashMap;

pub struct InstructionTracer {
    tracked: VgHashMap<HeapOffset, Box<[Option<Rc<TraceExpr>>]>>,
}

#[derive(Clone)]
pub enum TraceExpr {
    And(Box<[Rc<TraceExpr>]>),
    Or(Box<[Rc<TraceExpr>]>),
    Xor(Box<[Rc<TraceExpr>]>),
    Neg(Rc<TraceExpr>),

    L0,
    L1,

    Secret(u32),
    Mask(u32),
}

impl vogls_sim::InstructionPlugin for InstructionTracer {
    fn instruction(
        &mut self,
        simulation: &Simulation,
        state: &mut SimulationState,
        instruction: &VmInstruction,
    ) {
        use VmInstruction as I;
        match instruction {
            I::Constant(_, _) => {}
            I::TvUnary(dst, op, src) | I::FvUnary(dst, op, src) => {
                let Some(tsrc) = self.tracked.get(&src.offset) else {
                    return;
                };

                let tdst: Box<[Option<Rc<TraceExpr>>]> = match op {
                    UnaryOp::Neg => tsrc
                        .iter()
                        .map(|e| e.as_ref().map(|e| Rc::new(TraceExpr::Neg(e.clone()))))
                        .collect(),
                    UnaryOp::ReduceOr => [Some(Rc::new(TraceExpr::Or(
                        tsrc.iter().filter_map(|e| e.as_ref().cloned()).collect(),
                    )))]
                    .into(),
                    UnaryOp::ReduceAnd => [Some(Rc::new(TraceExpr::And(
                        tsrc.iter().filter_map(|e| e.as_ref().cloned()).collect(),
                    )))]
                    .into(),
                    UnaryOp::ReduceXor => [Some(Rc::new(TraceExpr::Xor(
                        tsrc.iter().filter_map(|e| e.as_ref().cloned()).collect(),
                    )))]
                    .into(),

                    // @TODO: Pessismistic
                    UnaryOp::ContainsX => return,
                };
                self.tracked.insert(*dst, tdst);
            }
            I::TvResize(dst, op, src) | I::FvResize(dst, op, src) => {
                let Some(tsrc) = self.tracked.get(&src.offset) else {
                    return;
                };

                let tdst: Box<[Option<Rc<TraceExpr>>]> = match op {
                    ResizeOp::Truncate => tsrc
                        .iter()
                        .skip((src.size.get() - dst.size.get()) as usize)
                        .cloned()
                        .collect(),
                    ResizeOp::ZeroExtend => {
                        std::iter::repeat_n(None, (dst.size.get() - src.size.get()) as usize)
                            .chain(tsrc.iter().cloned())
                            .collect()
                    }
                    ResizeOp::SignExtend => std::iter::repeat_n(
                        tsrc[0].clone(),
                        (dst.size.get() - src.size.get()) as usize,
                    )
                    .chain(tsrc.iter().cloned())
                    .collect(),
                };
                self.tracked.insert(dst.offset, tdst);
            }
            I::TvBinaryArithmetic(dst, op, lhs, rhs) | I::FvBinaryArithmetic(dst, op, lhs, rhs) => {
                let tlhs = self.tracked.get(lhs);
                let trhs = self.tracked.get(rhs);

                let mode = if matches!(instruction, I::TvBinaryArithmetic(..)) {
                    LogicMode::TwoValue
                } else {
                    LogicMode::FourValue
                };

                use BinaryArithmeticOp as O;
                let tdst: Box<[Option<Rc<TraceExpr>>]> = match (tlhs, trhs, op) {
                    (None, None, _) => return,

                    (Some(tlhs), Some(trhs), O::And) => tlhs
                        .iter()
                        .zip(trhs.iter())
                        .map(|(l, r)| match (l, r) {
                            (Some(l), Some(r)) => {
                                Some(Rc::new(TraceExpr::And([l.clone(), r.clone()].into())))
                            }
                            _ => None,
                        })
                        .collect(),
                    (Some(tsrc), None, O::And) | (None, Some(tsrc), O::And) => {
                        let val = if tlhs.is_some() {
                            state.heap.load_bits(rhs.to_ref(dst.size), mode)
                        } else {
                            state.heap.load_bits(lhs.to_ref(dst.size), mode)
                        };

                        tsrc.iter()
                            .zip(val.value_iter())
                            .map(|(s, v)| {
                                if let Some(s) = s
                                    && v == FvLogicValue::L1
                                {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    }

                    (None, Some(_), O::Or) => todo!(),
                    (None, Some(_), O::Xor) => todo!(),
                    (None, Some(_), O::Add) => todo!(),
                    (None, Some(_), O::Sub) => todo!(),
                    (None, Some(_), O::Power) => todo!(),
                    (None, Some(_), O::Multiply) => todo!(),
                    (None, Some(_), O::Divide) => todo!(),
                    (None, Some(_), O::Modulus) => todo!(),
                    (None, Some(_), O::CopyX) => todo!(),
                    (None, Some(_), O::CopyZ) => todo!(),
                    (None, Some(_), O::Min) => todo!(),
                    (None, Some(_), O::Max) => todo!(),
                    (Some(_), None, O::Or) => todo!(),
                    (Some(_), None, O::Xor) => todo!(),
                    (Some(_), None, O::Add) => todo!(),
                    (Some(_), None, O::Sub) => todo!(),
                    (Some(_), None, O::Power) => todo!(),
                    (Some(_), None, O::Multiply) => todo!(),
                    (Some(_), None, O::Divide) => todo!(),
                    (Some(_), None, O::Modulus) => todo!(),
                    (Some(_), None, O::CopyX) => todo!(),
                    (Some(_), None, O::CopyZ) => todo!(),
                    (Some(_), None, O::Min) => todo!(),
                    (Some(_), None, O::Max) => todo!(),
                    (Some(_), Some(_), O::Or) => todo!(),
                    (Some(_), Some(_), O::Xor) => todo!(),
                    (Some(_), Some(_), O::Add) => todo!(),
                    (Some(_), Some(_), O::Sub) => todo!(),
                    (Some(_), Some(_), O::Power) => todo!(),
                    (Some(_), Some(_), O::Multiply) => todo!(),
                    (Some(_), Some(_), O::Divide) => todo!(),
                    (Some(_), Some(_), O::Modulus) => todo!(),
                    (Some(_), Some(_), O::CopyX) => todo!(),
                    (Some(_), Some(_), O::CopyZ) => todo!(),
                    (Some(_), Some(_), O::Min) => todo!(),
                    (Some(_), Some(_), O::Max) => todo!(),
                };
            }
            I::TvBinaryComparison(dst, op, lhs, rhs) | I::FvBinaryComparison(dst, op, lhs, rhs) => {
            }
            I::TvShift(dst, op, src, amount) | I::FvShift(dst, op, src, amount) => {}
            I::TvSelectBit(dst, src, idx) | I::FvSelectBit(dst, src, idx) => {}
            I::TvConcat(dst, lhs, rhs) | I::FvConcat(dst, lhs, rhs) => {}

            VmInstruction::TvToFv(dst, src) | VmInstruction::FvToTv(dst, src) => {}

            VmInstruction::Intrinsic(..) => {}
            VmInstruction::LastUpdateTime(..) => {}
            VmInstruction::Drive(signal, src, offset) => {}

            // Control flow instruction don't influence the data here.
            VmInstruction::Wait(..)
            | VmInstruction::VariableWait(..)
            | VmInstruction::WaitRegion(..)
            | VmInstruction::Watch(..)
            | VmInstruction::Jump(..)
            | VmInstruction::Branch(..)
            | VmInstruction::Halt => {}
        }
    }
}
