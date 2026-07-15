use slotmap::SlotMap;
use vogls_bits::Bits;
use vogls_bits::arithmetic::FvLogicValue;
use vogls_utils::{VgHashMap, VgHashSet};

use crate::optimize::remove_bbs;
use crate::orders::post_order_keys;
use crate::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryImmOpSimplification, BinaryOp,
    GlobalContext, Instruction, LogicMode, ProcessKey, ResizeOp, SCALAR_VSIZE, ShiftImmOp,
    ShiftImmOpSimplification, Signal, SignalKey, Time, UnaryOp, VariableKey, VariableMap,
};

pub fn constant_propagation(
    gl: &mut GlobalContext,
    process: ProcessKey,

    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_mfr: &mut VgHashSet<BasicBlockKey>,

    // Map from Variable to Possibly constant bits.
    //
    // - not in map = "not yet seen"
    // - None = "seen, but not constant"
    // - Some(Bits) = "seen, constant"
    scratch_map: &mut VgHashMap<VariableKey, Option<Bits>>,
) {
    let mut post_order = Vec::new();
    let mut scratch_stack2 = Vec::new();
    let mut additional = Vec::new();
    let mut altered_cfg = false;

    scratch_mfr.clear();

    for tr in &gl.processes[process].regions {
        scratch_map.clear();

        post_order_keys(
            tr.entry(),
            &gl.bbs,
            scratch_seen,
            &mut scratch_stack2,
            &mut post_order,
        );
        post_order.reverse();

        scratch_mfr.extend(post_order.iter().copied());

        for bb_key in post_order.drain(..) {
            let bb = &mut gl.bbs[bb_key];

            // @Performance: Do this in-place where possible.
            let mut output = Vec::with_capacity(bb.instrs.len());
            for i in bb.instrs.drain(..) {
                let result = constant_propagate_instruction(
                    &i,
                    &mut gl.vars,
                    &mut gl.signals,
                    scratch_map,
                    &mut additional,
                );
                if result.replace {
                    output.extend(additional.drain(..));
                } else {
                    output.push(i);
                }
            }
            bb.instrs = output;

            use BasicBlockTerminator as T;
            match &bb.terminator {
                T::VariableWait(tr, var) => {
                    if let Some(Some(value)) = scratch_map.get(var) {
                        let tr = *tr;
                        if value.contains_special() {
                            bb.terminator = T::Wait(tr, Time(0));
                        } else {
                            let value = value.extract_exact_u64().unwrap();
                            bb.terminator = T::Wait(tr, Time(value));
                        }
                    }
                }
                T::Branch(var, truthy, falsy) => {
                    if let Some(Some(value)) = scratch_map.get(var) {
                        let (truthy, falsy) = (*truthy, *falsy);
                        if value.eq_one() {
                            bb.terminator = T::Jump(truthy);
                            altered_cfg = true;
                        } else {
                            bb.terminator = T::Jump(falsy);
                            altered_cfg = true;
                        }
                    }
                }
                T::Wait(..) | T::WaitRegion(..) | T::Watch(..) | T::Jump(..) | T::Halt => {}
            }
        }
    }

    if altered_cfg {
        let tr = gl.processes[process].regions[0];
        scratch_stack.push(tr.entry());
        scratch_mfr.remove(&tr.entry());

        while let Some(bb) = scratch_stack.pop() {
            gl.bbs[bb].terminator.for_each_temporal_bb(|next| {
                if scratch_mfr.remove(&next) {
                    scratch_stack.push(next);
                }
            });
        }

        remove_bbs(
            gl,
            tr,
            &scratch_mfr,
            scratch_stack,
            scratch_seen,
            &mut Vec::new(),
        );
    }
}

struct PropagateResult {
    replace: bool,
}

fn constant_propagate_instruction(
    i: &Instruction,
    vars: &mut VariableMap,
    signals: &mut SlotMap<SignalKey, Signal>,
    scratch_map: &mut VgHashMap<VariableKey, Option<Bits>>,
    additional: &mut Vec<Instruction>,
) -> PropagateResult {
    use Instruction as I;

    // Skip the instruction if it is already handled.
    if i.get_destination_variable()
        .is_some_and(|dst| scratch_map.contains_key(&dst))
    {
        return PropagateResult { replace: false };
    }

    macro_rules! get {
        ($var:expr) => {{
            let Some(value) = scratch_map.get($var) else {
                return PropagateResult { replace: false };
            };
            value
        }};
    }
    macro_rules! assign_constant {
        ($dst:expr, $constant:expr) => {{
            let constant: Bits = $constant;
            if cfg!(debug_assertions) && constant.contains_special() {
                assert_eq!($dst.mode(), LogicMode::FourValue);
            }
            scratch_map.insert($dst, Some(constant.clone()));
            additional.push(I::Constant($dst, constant));
            return PropagateResult { replace: true };
        }};

        ($i:expr, $dst:expr, $constant:expr) => {{
            let constant: Bits = $constant;
            if cfg!(debug_assertions) && constant.contains_special() {
                assert_eq!($dst.mode(), LogicMode::FourValue);
            }
            scratch_map.insert($dst, Some(constant.clone()));
            $i = I::Constant($dst, constant);
            return PropagateResult { replace: true };
        }};
    }

    macro_rules! not_constant {
        ($dst:expr) => {{
            scratch_map.insert($dst, None);
            return PropagateResult { replace: false };
        }};
    }

    match i {
        I::Constant(dst, bits) => {
            if cfg!(debug_assertions) && bits.contains_special() {
                assert_eq!(dst.mode(), LogicMode::FourValue);
            }
            _ = scratch_map.insert(*dst, Some(bits.clone()));
            PropagateResult { replace: false }
        }
        I::Unary(dst, op, src) => {
            let src_bits = get!(src);
            let dst = *dst;
            match src_bits {
                None => not_constant!(dst),
                Some(b) => assign_constant!(dst, op.evaluate(b)),
            }
        }
        I::Resize(dst, op, src) => {
            let src_bits = get!(src);
            let dst = *dst;
            match src_bits {
                None => not_constant!(dst),
                Some(b) => assign_constant!(dst, op.evaluate(b, vars.size(dst))),
            }
        }
        I::Binary(dst, op, lhs, rhs) => {
            let (dst, op, lhs, rhs) = (*dst, *op, *lhs, *rhs);
            let lhs_bits_entry = scratch_map.get(&lhs);
            let rhs_bits_entry = scratch_map.get(&rhs);
            let operands_are_complete = lhs_bits_entry.is_some() & rhs_bits_entry.is_some();
            let lhs_bits = lhs_bits_entry.and_then(|v| v.as_ref());
            let rhs_bits = rhs_bits_entry.and_then(|v| v.as_ref());

            macro_rules! simplify_div_mod_imm {
                ($dst:expr, $src:expr, $imm:expr, $op:ident, $is_rhs:expr, $div_by_zero_equals_x:expr) => {{
                    let b: &Bits = $imm;
                    if b.contains_special()
                        || ($is_rhs && $div_by_zero_equals_x && b.is_equal_to_zero())
                    {
                        assign_constant!(dst, Bits::new_unknown(b.size()));
                    } else if $is_rhs && !$div_by_zero_equals_x && b.is_equal_to_zero() {
                        assign_constant!(dst, Bits::new_zeroed(b.size()));
                    }

                    if lhs.mode() == LogicMode::TwoValue {
                        let tgt = vars.insert(LogicMode::TwoValue, b.size());
                        additional.push(BI(tgt, IO::$op, lhs, b.clone()));
                        additional.push(I::Unary(dst, UnaryOp::TvToFv, tgt));
                    } else {
                        additional.push(BI(dst, IO::$op, lhs, b.clone()));
                    }
                }};
            }

            use BinaryImmOp as IO;
            use BinaryOp as O;
            use I::BinaryImm as BI;
            use I::ShiftImm as SI;
            use ShiftImmOp as SO;
            match (op, lhs_bits, rhs_bits) {
                (_, Some(l), Some(r)) => {
                    let value = op.evaluate(l, r, vars.size(dst));
                    assign_constant!(dst, value);
                }
                (_, None, None) => {}

                (O::And, Some(b), _) => additional.push(BI(dst, IO::And, rhs, b.clone())),
                (O::And, _, Some(b)) => additional.push(BI(dst, IO::And, lhs, b.clone())),
                (O::Or, Some(b), _) => additional.push(BI(dst, IO::Or, rhs, b.clone())),
                (O::Or, _, Some(b)) => additional.push(BI(dst, IO::Or, lhs, b.clone())),
                (O::Xor, Some(b), _) => additional.push(BI(dst, IO::Xor, rhs, b.clone())),
                (O::Xor, _, Some(b)) => additional.push(BI(dst, IO::Xor, lhs, b.clone())),

                (O::Add, Some(b), _) => additional.push(BI(dst, IO::Add, rhs, b.clone())),
                (O::Add, _, Some(b)) => additional.push(BI(dst, IO::Add, lhs, b.clone())),
                (O::Sub, Some(b), _) => additional.push(BI(dst, IO::RevSub, rhs, b.clone())),
                (O::Sub, _, Some(b)) => additional.push(BI(dst, IO::Sub, lhs, b.clone())),
                (O::Multiply, Some(b), _) => additional.push(BI(dst, IO::Multiply, rhs, b.clone())),
                (O::Multiply, _, Some(b)) => additional.push(BI(dst, IO::Multiply, lhs, b.clone())),
                (O::Power, Some(b), _) => additional.push(BI(dst, IO::RevPower, rhs, b.clone())),
                (O::Power, _, Some(b)) => additional.push(BI(dst, IO::Power, lhs, b.clone())),
                (O::DivideX, Some(b), _) => {
                    simplify_div_mod_imm!(dst, lhs, b, RevDivideX, false, false)
                }
                (O::DivideX, _, Some(b)) => simplify_div_mod_imm!(dst, lhs, b, Divide, true, false),
                (O::Divide0, Some(b), _) => {
                    simplify_div_mod_imm!(dst, lhs, b, RevDivide0, false, true)
                }
                (O::Divide0, _, Some(b)) => simplify_div_mod_imm!(dst, lhs, b, Divide, true, true),
                (O::ModulusX, Some(b), _) => {
                    simplify_div_mod_imm!(dst, lhs, b, RevModulusX, false, false)
                }
                (O::ModulusX, _, Some(b)) => {
                    simplify_div_mod_imm!(dst, lhs, b, Modulus, true, false)
                }
                (O::Modulus0, Some(b), _) => {
                    simplify_div_mod_imm!(dst, lhs, b, RevModulus0, false, true)
                }
                (O::Modulus0, _, Some(b)) => {
                    simplify_div_mod_imm!(dst, lhs, b, Modulus, true, true)
                }

                (O::UnsignedLessEqual, Some(b), _) => {
                    additional.push(BI(dst, IO::UnsignedGreaterEqual, rhs, b.clone()))
                }
                (O::UnsignedLessEqual, _, Some(b)) => {
                    additional.push(BI(dst, IO::UnsignedLessEqual, lhs, b.clone()))
                }

                (O::LogicalShiftLeft, Some(_), _) => {}
                (O::LogicalShiftLeft, _, Some(b)) => match b.extract_exact_u32() {
                    None => {
                        let value = Bits::new_unknown(vars.size(dst));
                        assign_constant!(dst, value);
                    }
                    Some(offset) => additional.push(SI(dst, SO::LogicalShiftLeft, lhs, offset)),
                },
                (O::LogicalShiftRight, Some(_), _) => {}
                (O::LogicalShiftRight, _, Some(b)) => match b.extract_exact_u32() {
                    None => {
                        let value = Bits::new_unknown(vars.size(dst));
                        assign_constant!(dst, value);
                    }
                    Some(offset) => additional.push(SI(dst, SO::LogicalShiftRight, lhs, offset)),
                },
                (O::ArithmeticShiftRight, Some(_), _) => {}
                (O::ArithmeticShiftRight, _, Some(b)) => match b.extract_exact_u32() {
                    None => {
                        let value = Bits::new_unknown(vars.size(dst));
                        assign_constant!(dst, value);
                    }
                    Some(offset) => additional.push(SI(dst, SO::ArithmeticShiftRight, lhs, offset)),
                },

                (O::Concat, Some(b), _) => additional.push(BI(dst, IO::ConcatLeft, rhs, b.clone())),
                (O::Concat, _, Some(b)) => {
                    additional.push(BI(dst, IO::ConcatRight, lhs, b.clone()))
                }

                (O::CopyX, Some(_), _) => {}
                (O::CopyX, _, Some(_)) => {}
                (O::CopyZ, Some(_), _) => {}
                (O::CopyZ, _, Some(_)) => {}

                (O::Min, Some(b), _) => additional.push(BI(dst, IO::Min, rhs, b.clone())),
                (O::Min, _, Some(b)) => additional.push(BI(dst, IO::Min, lhs, b.clone())),
                (O::Max, Some(b), _) => additional.push(BI(dst, IO::Max, rhs, b.clone())),
                (O::Max, _, Some(b)) => additional.push(BI(dst, IO::Max, lhs, b.clone())),

                (O::CaseEquality, Some(b), _) => {
                    additional.push(BI(dst, IO::CaseEquality, rhs, b.clone()))
                }
                (O::CaseEquality, _, Some(b)) => {
                    additional.push(BI(dst, IO::CaseEquality, lhs, b.clone()))
                }

                (O::Posedge, Some(_), _) => {}
                (O::Posedge, _, Some(_)) => {}
                (O::Negedge, Some(_), _) => {}
                (O::Negedge, _, Some(_)) => {}
            };

            // If we managed to convert it to a immediate based operation, we should try to
            // simplify further.
            let i = additional.first_mut();
            if let Some(i) = i {
                match i {
                    I::BinaryImm(dst, op, src, imm) => {
                        let (dst, op, src) = (*dst, *op, *src);
                        use BinaryImmOpSimplification as S;
                        match op.simplify(dst, src, imm) {
                            S::Keep => *i = BI(dst, op, src, imm.clone()),
                            S::Source => *i = I::copy(vars, dst, src),
                            S::Immediate => assign_constant!(*i, dst, imm.clone()),
                            S::Constant(value) => assign_constant!(*i, dst, value),
                            S::Instruction(instr) => *i = instr,
                        }
                    }
                    I::ShiftImm(dst, op, src, amount) => {
                        let (dst, src) = (*dst, *src);
                        use ShiftImmOpSimplification as S;
                        match op.simplify(vars.size(dst), *amount) {
                            S::Keep => {}
                            S::Source => *i = I::copy(vars, dst, src),
                            S::Constant(value) => assign_constant!(*i, dst, value),
                        }
                    }
                    _ => {}
                }
            }

            if operands_are_complete && additional.len() == 0 {
                not_constant!(dst);
            }

            if operands_are_complete {
                scratch_map.insert(dst, None);
            }

            PropagateResult {
                replace: additional.len() > 0,
            }
        }
        I::BinaryImm(dst, op, src, imm) => {
            let src_bits = get!(src);
            let dst = *dst;
            match src_bits.as_ref() {
                None => not_constant!(dst),
                Some(b) => assign_constant!(dst, op.evaluate(b, imm)),
            }
        }
        I::Slice(dst, src, offset) => {
            let (dst, src, offset) = (*dst, *src, *offset);
            let src_bits_entry = scratch_map.get(&src);
            let offset_bits_entry = scratch_map.get(&offset);
            let operands_are_complete = src_bits_entry.is_some() & offset_bits_entry.is_some();
            let src_bits = src_bits_entry.map_or(None, |b| b.as_ref());
            let offset_bits = offset_bits_entry.map_or(None, |b| b.as_ref());

            match (src_bits, offset_bits) {
                (Some(l), Some(r)) => {
                    let dst_size = vars.size(dst);
                    let value = match r.extract_exact_u32() {
                        None => Bits::new_unknown(dst_size),
                        Some(offset) => l.slicex(offset, dst_size),
                    };
                    assign_constant!(dst, value);
                }
                (Some(l), None) if l.count_unknown() == l.size().get() => {
                    let dst_size = vars.size(dst);
                    let value = Bits::new_unknown(dst_size);
                    assign_constant!(dst, value);
                }
                (None, Some(offset)) => {
                    let dst_size = vars.size(dst);
                    let Some(offset) = offset.extract_exact_u32() else {
                        let value = Bits::new_unknown(dst_size);
                        assign_constant!(dst, value);
                    };

                    let src_size = vars.size(src);
                    if offset <= src_size.get() - dst_size.get() {
                        fn maybe_cast_mode(
                            dst: VariableKey,
                            src: VariableKey,
                            vars: &mut VariableMap,
                            additional: &mut Vec<Instruction>,
                            mut f: impl FnMut(VariableKey) -> Instruction,
                        ) {
                            if dst.mode() != src.mode() {
                                let dst_size = vars.size(dst);
                                let tgt = vars.insert(src.mode(), dst_size);
                                additional.push(f(tgt));
                                additional.push(I::copy(vars, dst, tgt));
                            } else {
                                additional.push(f(dst));
                            }
                        }

                        if dst_size == src_size {
                            if offset == 0 {
                                additional.push(I::copy(vars, dst, src));
                            } else {
                                maybe_cast_mode(dst, src, vars, additional, |dst| {
                                    I::ShiftImm(dst, ShiftImmOp::LogicalShiftRight, src, offset)
                                });
                            }
                        } else if offset >= src_size.get() {
                            assign_constant!(dst, Bits::new_zeroed(dst_size))
                        } else if offset == 0 {
                            maybe_cast_mode(dst, src, vars, additional, |dst| {
                                I::Resize(dst, ResizeOp::Truncate, src)
                            });
                        } else {
                            maybe_cast_mode(dst, src, vars, additional, |dst| {
                                I::SliceImm(dst, src, offset)
                            });
                        }
                    }
                }

                (None, None) | (Some(_), None) => {}
            };

            if operands_are_complete && additional.len() == 0 {
                not_constant!(dst);
            }

            if operands_are_complete {
                scratch_map.insert(dst, None);
            }

            PropagateResult {
                replace: additional.len() > 0,
            }
        }
        I::SliceImm(dst, src, amount) => {
            let src_bits = get!(src);
            let (dst, amount) = (*dst, *amount);
            match src_bits.as_ref() {
                None => not_constant!(dst),
                Some(b) => {
                    assign_constant!(dst, b.slicez(amount, vars.size(dst)));
                }
            }
        }
        I::ShiftImm(dst, op, src, amount) => {
            let src_bits = get!(src);
            let (dst, amount) = (*dst, *amount);
            match src_bits.as_ref() {
                None => not_constant!(dst),
                Some(b) => assign_constant!(dst, op.evaluate(b, amount)),
            }
        }
        I::Select(dst, cond, truthy, falsy) => {
            let cond_bits = get!(cond);
            let truthy_bits = get!(truthy);
            let falsy_bits = get!(falsy);

            let (dst, truthy, falsy) = (*dst, *truthy, *falsy);

            match (truthy_bits, falsy_bits) {
                (Some(t), Some(f)) if t == f => assign_constant!(dst, t.clone()),
                (Some(t), Some(f)) if t.size() == SCALAR_VSIZE => {
                    use FvLogicValue as L;
                    match (t.select_value(0), f.select_value(0)) {
                        (L::L1, L::L0) => {
                            additional.push(I::copy(vars, dst, *cond));
                            return PropagateResult { replace: true };
                        }
                        (L::L0, L::L1) => {
                            additional.push(I::Unary(dst, UnaryOp::Neg, *cond));
                            return PropagateResult { replace: true };
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            match cond_bits.as_ref() {
                None => not_constant!(dst),
                Some(b) => {
                    let (src, bits) = if b.is_one() {
                        (truthy, truthy_bits)
                    } else {
                        (falsy, falsy_bits)
                    };

                    match bits {
                        None => {
                            additional.push(I::copy(vars, dst, src));
                            return PropagateResult { replace: true };
                        }
                        Some(bits) => assign_constant!(dst, bits.clone()),
                    }
                }
            }
        }

        I::Intrinsic(dst, ..) | I::LastUpdateTime(dst, ..) => not_constant!(*dst),
        I::Probe(dst, _, _) => not_constant!(*dst),

        I::ProbeSlice(dst, signal, offset) => {
            let (dst, signal) = (*dst, *signal);
            let offset_bits = get!(offset);
            match offset_bits.as_ref() {
                None => {}
                Some(b) => match b.extract_exact_u32() {
                    None => assign_constant!(dst, Bits::new_unknown(vars.size(dst))),
                    Some(offset) => {
                        let dst_size = vars.size(dst);
                        let src_size = signals[signal].size;

                        if dst_size
                            .get()
                            .checked_add(offset)
                            .is_some_and(|v| v < src_size.get())
                        {
                            additional.push(I::Probe(dst, signal, offset));
                            return PropagateResult { replace: true };
                        }
                    }
                },
            }
            not_constant!(dst);
        }
        I::Drive(..) => PropagateResult { replace: false },
        I::DriveSlice(signal, src, offset) => {
            let (signal, src) = (*signal, *src);
            let offset_bits = get!(offset);
            match offset_bits.as_ref() {
                None => {}
                Some(b) => match b.extract_exact_u32() {
                    None => {}
                    Some(offset) => {
                        let dst_size = signals[signal].size;
                        let src_size = vars.size(src);

                        if src_size
                            .get()
                            .checked_add(offset)
                            .is_some_and(|v| v < dst_size.get())
                        {
                            additional.push(I::Drive(signal, src, offset));
                            return PropagateResult { replace: true };
                        }
                    }
                },
            }

            PropagateResult { replace: false }
        }
        I::Phi(dst, srcs) => {
            assert!(!srcs.is_empty());
            let mut acc = None;
            let mut is_all_equal_constant = true;
            let mut is_all_complete = true;
            for (_, src) in srcs.iter() {
                let Some(bits) = scratch_map.get(src) else {
                    is_all_complete = false;
                    continue;
                };
                let Some(bits) = bits else {
                    is_all_equal_constant = false;
                    break;
                };
                match acc {
                    None => acc = Some(bits),
                    Some(acc) if acc != bits => {
                        is_all_equal_constant = false;
                        break;
                    }
                    Some(_) => {}
                }
            }

            if !is_all_equal_constant {
                not_constant!(*dst);
            }
            if !is_all_complete {
                return PropagateResult { replace: false };
            }
            assign_constant!(*dst, acc.clone().unwrap().clone());
        }
    }
}
