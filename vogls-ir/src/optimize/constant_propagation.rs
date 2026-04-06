use std::ops::Range;

use slotmap::SlotMap;
use vogls_bits::Bits;
use vogls_utils::{VgHashMap, VgHashSet};

use crate::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryImmOpSimplification, BinaryOp,
    GlobalContext, Instruction, ProcessKey, ResizeOp, ShiftImmOp, ShiftImmOpSimplification, Signal,
    SignalKey, SliceImmSimplification, Time, Variable, VariableKey, simplify_slice_imm,
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

    // As we might have variables from basic blocks we have not explored yet (primarily due to phi
    // instructions), we need to keep track of all the yet unresolved variables and which variables
    // they depend on being resolved. If there is a cycle in the dependencies for variables, we
    // need to handle the cycle otherwise we will keep looping forever.
    scratch_dep: &mut VgHashMap<VariableKey, (BasicBlockKey, usize, Range<usize>)>,
    scratch_dep_edges: &mut Vec<VariableKey>,
) {
    let entry = gl.processes[process].entry;

    scratch_seen.clear();
    scratch_stack.clear();
    scratch_map.clear();
    scratch_dep.clear();
    scratch_dep_edges.clear();

    scratch_seen.insert(entry);
    scratch_stack.push(entry);
    while let Some(bb_key) = scratch_stack.pop() {
        let bb = &mut gl.bbs[bb_key];
        for (instr_i, i) in bb.instrs.iter_mut().enumerate() {
            if constant_propagate_instruction(i, &gl.vars, &gl.signals, scratch_map).is_err() {
                let dst = i.get_destination_variable().unwrap();
                let offset = scratch_dep_edges.len();
                i.for_each_src(|src| scratch_dep_edges.push(src));
                scratch_dep.insert(dst, (bb_key, instr_i, offset..scratch_dep_edges.len()));
            }
        }
        bb.terminator.for_each_bb(|bb_key| {
            if scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
    }

    // All the variables that couldn't be immediately resolved get resolved by a depth-first
    // search (DFS). We need to take care because there might be dependency loops caused by phi
    // instructions. These we just mark as non-constant and move on.
    if !scratch_dep.is_empty() {
        enum StackItem {
            Previsit(Range<usize>),
            Postvisit,
        }
        let mut var_seen = VgHashSet::default();
        let mut var_stack = Vec::new();

        while let Some(&var) = scratch_dep.keys().next() {
            let (bb_key, instr_i, range) = scratch_dep.remove(&var).unwrap();
            var_stack.push((var, bb_key, instr_i, StackItem::Previsit(range)));

            while let Some((var, bb_key, instr_i, visit)) = var_stack.pop() {
                match visit {
                    StackItem::Previsit(range) => {
                        if constant_propagate_instruction(
                            &mut gl.bbs[bb_key].instrs[instr_i],
                            &gl.vars,
                            &gl.signals,
                            scratch_map,
                        )
                        .is_ok()
                        {
                            continue;
                        }

                        // There is a loop in variable dependencies. Mark all variables in the path
                        // here as non-constant.
                        if var_seen.insert(var) {
                            scratch_map.insert(var, None);
                            var_stack.retain(|(var, _, _, visit)| {
                                if matches!(visit, StackItem::Postvisit) {
                                    scratch_map.insert(*var, None);
                                    return false;
                                }

                                true
                            });
                        }

                        var_stack.push((var, bb_key, instr_i, StackItem::Postvisit));
                        var_stack.extend(scratch_dep_edges[range].iter().filter_map(|src| {
                            let (bb_key, instr_i, range) = scratch_dep.remove(src)?;
                            Some((*src, bb_key, instr_i, StackItem::Previsit(range)))
                        }));
                    }
                    StackItem::Postvisit => {
                        assert!(
                            constant_propagate_instruction(
                                &mut gl.bbs[bb_key].instrs[instr_i],
                                &gl.vars,
                                &gl.signals,
                                scratch_map,
                            )
                            .is_ok()
                        );
                    }
                }
            }
        }
    }

    scratch_seen.clear();
    scratch_stack.clear();
    scratch_mfr.clear();

    scratch_seen.insert(entry);
    scratch_stack.push(entry);
    while let Some(bb_key) = scratch_stack.pop() {
        let bb = &mut gl.bbs[bb_key];
        match &bb.terminator {
            BasicBlockTerminator::VariableWait(target, time) => {
                if let Some(time) = scratch_map[time].as_ref() {
                    let time = time.extract_exact_u64().unwrap_or(0);
                    bb.terminator = BasicBlockTerminator::Wait(*target, Time(time));
                }
            }
            BasicBlockTerminator::Branch(condition, truthy, falsy) => {
                if let Some(condition) = scratch_map[condition].as_ref() {
                    let (taken, untaken) = if condition.eq_one() {
                        (*truthy, *falsy)
                    } else {
                        (*falsy, *truthy)
                    };

                    bb.terminator = BasicBlockTerminator::Jump(taken);
                    scratch_mfr.insert(untaken);
                }
            }
            BasicBlockTerminator::Wait(..)
            | BasicBlockTerminator::WaitRegion(..)
            | BasicBlockTerminator::Watch(..)
            | BasicBlockTerminator::Jump(..)
            | BasicBlockTerminator::Halt => {}
        }

        bb.terminator.for_each_bb(|bb_key| {
            if scratch_seen.insert(bb_key) {
                scratch_stack.push(bb_key);
            }
        });
    }

    scratch_mfr.retain(|bb_key| {
        if scratch_seen.contains(bb_key) {
            return false;
        }
        true
    });

    // If there are any basic-blocks that should be removed, remove them now and clear them up
    // from the phi-instructions of other basic-blocks as those might be waiting on those
    // variables to get resolved.
    if !scratch_mfr.is_empty() {
        scratch_stack.extend(scratch_mfr.iter().copied());
        while let Some(bb_key) = scratch_stack.pop() {
            gl.bbs[bb_key].terminator.for_each_bb(|next| {
                if !scratch_seen.contains(&next) && scratch_mfr.insert(next) {
                    scratch_stack.push(next);
                }
            });
            gl.bbs.remove(bb_key);
        }

        // Remove phi referenced to removed basic-blocks and mark any variables as
        // non-constants, so that they get removed as a dependency in the next stage.
        scratch_seen.clear();
        scratch_seen.insert(entry);
        scratch_stack.push(entry);
        while let Some(bb_key) = scratch_stack.pop() {
            for i in &mut gl.bbs[bb_key].instrs {
                if let Instruction::Phi(dst, srcs) = i {
                    let num_matches = srcs
                        .iter()
                        .filter(|(bb, _)| scratch_mfr.contains(bb))
                        .count();

                    assert!(num_matches < srcs.len());
                    if num_matches == 0 {
                        continue;
                    }

                    if num_matches == srcs.len() - 1 {
                        let src = srcs
                            .iter()
                            .find(|(bb, _)| !scratch_mfr.contains(bb))
                            .unwrap()
                            .1;
                        *i = Instruction::Resize(*dst, ResizeOp::Truncate, src);
                    } else {
                        *srcs = srcs
                            .iter()
                            .filter(|(bb, _)| !scratch_mfr.contains(bb))
                            .copied()
                            .collect();
                    }
                }
            }
            gl.bbs[bb_key].terminator.for_each_bb(|bb_key| {
                if scratch_seen.insert(bb_key) {
                    scratch_stack.push(bb_key);
                }
            });
        }
        scratch_mfr.clear();
    }
}

fn constant_propagate_instruction(
    i: &mut Instruction,
    vars: &SlotMap<VariableKey, Variable>,
    signals: &SlotMap<SignalKey, Signal>,
    scratch_map: &mut VgHashMap<VariableKey, Option<Bits>>,
) -> Result<(), ()> {
    use Instruction as I;

    // Skip the instruction if it is already handled.
    if i.get_destination_variable()
        .is_none_or(|dst| scratch_map.contains_key(&dst))
    {
        return Ok(());
    }

    match i {
        I::Constant(dst, bits) => _ = scratch_map.insert(*dst, Some(bits.clone())),
        I::Unary(dst, op, src) => {
            let src_bits = scratch_map.get(src).ok_or(())?;
            let dst = *dst;
            match src_bits {
                None => _ = scratch_map.insert(dst, None),
                Some(b) => {
                    let value = op.evaluate(b);
                    scratch_map.insert(dst, Some(value.clone()));
                    *i = I::Constant(dst, value);
                }
            }
        }
        I::Resize(dst, op, src) => {
            let src_bits = scratch_map.get(src).ok_or(())?;
            let dst = *dst;
            match src_bits {
                None => _ = scratch_map.insert(dst, None),
                Some(b) => {
                    let value = op.evaluate(b, vars[dst].size);
                    scratch_map.insert(dst, Some(value.clone()));
                    *i = I::Constant(dst, value);
                }
            }
        }
        I::Binary(dst, op, lhs, rhs) => {
            let (dst, op, lhs, rhs) = (*dst, *op, *lhs, *rhs);
            let lhs_bits_entry = scratch_map.get(&lhs);
            let rhs_bits_entry = scratch_map.get(&rhs);
            let operands_are_complete = lhs_bits_entry.is_some() & rhs_bits_entry.is_some();
            let lhs_bits = lhs_bits_entry.map_or(None, |b| b.as_ref());
            let rhs_bits = rhs_bits_entry.map_or(None, |b| b.as_ref());

            use BinaryImmOp as IO;
            use BinaryOp as O;
            use I::BinaryImm as BI;
            use I::ShiftImm as SI;
            use ShiftImmOp as SO;
            match (op, lhs_bits, rhs_bits) {
                (_, Some(l), Some(r)) => {
                    let value = op.evaluate(l, r, vars[dst].size);
                    scratch_map.insert(dst, Some(value.clone()));
                    *i = I::Constant(dst, value);
                    return Ok(());
                }
                (_, None, None) => {}

                (O::And, Some(b), _) => *i = BI(dst, IO::And, rhs, b.clone()),
                (O::And, _, Some(b)) => *i = BI(dst, IO::And, lhs, b.clone()),
                (O::Or, Some(b), _) => *i = BI(dst, IO::Or, rhs, b.clone()),
                (O::Or, _, Some(b)) => *i = BI(dst, IO::Or, lhs, b.clone()),
                (O::Xor, Some(b), _) => *i = BI(dst, IO::Xor, rhs, b.clone()),
                (O::Xor, _, Some(b)) => *i = BI(dst, IO::Xor, lhs, b.clone()),

                (O::Add, Some(b), _) => *i = BI(dst, IO::Add, rhs, b.clone()),
                (O::Add, _, Some(b)) => *i = BI(dst, IO::Add, lhs, b.clone()),
                (O::Sub, Some(b), _) => *i = BI(dst, IO::RevSub, rhs, b.clone()),
                (O::Sub, _, Some(b)) => *i = BI(dst, IO::Sub, lhs, b.clone()),
                (O::Multiply, Some(b), _) => *i = BI(dst, IO::Multiply, rhs, b.clone()),
                (O::Multiply, _, Some(b)) => *i = BI(dst, IO::Multiply, lhs, b.clone()),
                (O::Power, Some(b), _) => *i = BI(dst, IO::RevPower, rhs, b.clone()),
                (O::Power, _, Some(b)) => *i = BI(dst, IO::Power, lhs, b.clone()),
                (O::Divide, Some(b), _) => *i = BI(dst, IO::RevDivide, rhs, b.clone()),
                (O::Divide, _, Some(b)) => *i = BI(dst, IO::Divide, lhs, b.clone()),
                (O::Modulus, Some(b), _) => *i = BI(dst, IO::RevModulus, rhs, b.clone()),
                (O::Modulus, _, Some(b)) => *i = BI(dst, IO::Modulus, lhs, b.clone()),

                (O::UnsignedLessEqual, Some(b), _) => {
                    *i = BI(dst, IO::UnsignedGreaterEqual, rhs, b.clone())
                }
                (O::UnsignedLessEqual, _, Some(b)) => {
                    *i = BI(dst, IO::UnsignedLessEqual, lhs, b.clone())
                }

                (O::LogicalShiftLeft, Some(_), _) => {}
                (O::LogicalShiftLeft, _, Some(b)) => match b.extract_exact_u32() {
                    None => {
                        let value = Bits::new_unknown(vars[dst].size);
                        scratch_map.insert(dst, Some(value.clone()));
                        *i = I::Constant(dst, value);
                        return Ok(());
                    }
                    Some(offset) => *i = SI(dst, SO::LogicalShiftLeft, lhs, offset),
                },
                (O::LogicalShiftRight, Some(_), _) => {}
                (O::LogicalShiftRight, _, Some(b)) => match b.extract_exact_u32() {
                    None => {
                        let value = Bits::new_unknown(vars[dst].size);
                        scratch_map.insert(dst, Some(value.clone()));
                        *i = I::Constant(dst, value);
                        return Ok(());
                    }
                    Some(offset) => *i = SI(dst, SO::LogicalShiftRight, lhs, offset),
                },
                (O::ArithmeticShiftRight, Some(_), _) => {}
                (O::ArithmeticShiftRight, _, Some(b)) => match b.extract_exact_u32() {
                    None => {
                        let value = Bits::new_unknown(vars[dst].size);
                        scratch_map.insert(dst, Some(value.clone()));
                        *i = I::Constant(dst, value);
                        return Ok(());
                    }
                    Some(offset) => *i = SI(dst, SO::ArithmeticShiftRight, lhs, offset),
                },

                (O::Concat, Some(b), _) => *i = BI(dst, IO::ConcatLeft, rhs, b.clone()),
                (O::Concat, _, Some(b)) => *i = BI(dst, IO::ConcatRight, lhs, b.clone()),

                (O::CopyX, Some(_), _) => {}
                (O::CopyX, _, Some(_)) => {}
                (O::CopyZ, Some(_), _) => {}
                (O::CopyZ, _, Some(_)) => {}

                (O::Min, Some(b), _) => *i = BI(dst, IO::Min, rhs, b.clone()),
                (O::Min, _, Some(b)) => *i = BI(dst, IO::Min, lhs, b.clone()),
                (O::Max, Some(b), _) => *i = BI(dst, IO::Max, rhs, b.clone()),
                (O::Max, _, Some(b)) => *i = BI(dst, IO::Max, lhs, b.clone()),

                (O::CaseEquality, Some(b), _) => *i = BI(dst, IO::CaseEquality, rhs, b.clone()),
                (O::CaseEquality, _, Some(b)) => *i = BI(dst, IO::CaseEquality, lhs, b.clone()),

                (O::Posedge, Some(_), _) => {}
                (O::Posedge, _, Some(_)) => {}
                (O::Negedge, Some(_), _) => {}
                (O::Negedge, _, Some(_)) => {}
            };

            // If we managed to convert it to a immediate based operation, we should try to
            // simplify further.
            match i {
                I::BinaryImm(dst, op, src, imm) => {
                    let (dst, src) = (*dst, *src);
                    use BinaryImmOpSimplification as S;
                    match op.simplify(dst, src, imm) {
                        S::Keep => {}
                        S::Source => *i = I::Resize(dst, ResizeOp::Truncate, src),
                        S::Immediate => {
                            let imm = imm.clone();
                            *i = I::Constant(dst, imm.clone());
                            scratch_map.insert(dst, Some(imm));
                            return Ok(());
                        }
                        S::Constant(bits) => {
                            *i = I::Constant(dst, bits.clone());
                            scratch_map.insert(dst, Some(bits));
                            return Ok(());
                        }
                        S::Instruction(instr) => *i = instr,
                    }
                }
                I::ShiftImm(dst, op, src, amount) => {
                    let (dst, src) = (*dst, *src);
                    use ShiftImmOpSimplification as S;
                    match op.simplify(vars[dst].size, *amount) {
                        S::Keep => {}
                        S::Source => *i = I::Resize(dst, ResizeOp::Truncate, src),
                        S::Constant(bits) => {
                            *i = I::Constant(dst, bits.clone());
                            scratch_map.insert(dst, Some(bits));
                            return Ok(());
                        }
                    }
                }
                _ => {}
            }

            if !operands_are_complete {
                return Err(());
            }

            scratch_map.insert(dst, None);
        }
        I::BinaryImm(dst, op, src, imm) => {
            let src_bits = scratch_map.get(src).ok_or(())?;
            let dst = *dst;
            match src_bits.as_ref() {
                None => _ = scratch_map.insert(dst, None),
                Some(b) => {
                    let bits = op.evaluate(b, imm);
                    scratch_map.insert(dst, Some(bits.clone()));
                    *i = I::Constant(dst, bits);
                }
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
                    let dst_size = vars[dst].size;
                    let value = match r.extract_exact_u32() {
                        None => Bits::new_unknown(dst_size),
                        Some(offset) => l.slicex(offset, dst_size),
                    };
                    scratch_map.insert(dst, Some(value.clone()));
                    *i = I::Constant(dst, value);
                    return Ok(());
                }
                (Some(l), None) if l.count_unknown() == l.size().get() => {
                    let dst_size = vars[dst].size;
                    let value = Bits::new_unknown(dst_size);
                    scratch_map.insert(dst, Some(value.clone()));
                    *i = I::Constant(dst, value);
                    return Ok(());
                }
                (None, Some(offset)) => {
                    let dst_size = vars[dst].size;
                    let Some(offset) = offset.extract_exact_u32() else {
                        let value = Bits::new_unknown(dst_size);
                        scratch_map.insert(dst, Some(value.clone()));
                        *i = I::Constant(dst, value);
                        return Ok(());
                    };

                    let src_size = vars[src].size;
                    if offset <= src_size.get() - dst_size.get() {
                        use SliceImmSimplification as S;
                        match simplify_slice_imm(dst, dst_size, src, src_size, offset) {
                            S::Keep => *i = I::SliceImm(dst, src, offset),
                            S::Source => *i = I::Resize(dst, ResizeOp::Truncate, src),
                            S::Constant(bits) => {
                                scratch_map.insert(dst, Some(bits.clone()));
                                *i = I::Constant(dst, bits);
                                return Ok(());
                            }
                            S::Instruction(instruction) => *i = instruction,
                        }
                    }
                }

                (None, None) | (Some(_), None) => {}
            };

            if !operands_are_complete {
                return Err(());
            }
            scratch_map.insert(dst, None);
        }
        I::SliceImm(dst, src, amount) => {
            let src_bits = scratch_map.get(src).ok_or(())?;
            let (dst, amount) = (*dst, *amount);
            match src_bits.as_ref() {
                None => _ = scratch_map.insert(dst, None),
                Some(b) => {
                    let bits = b.slicez(amount, vars[dst].size);
                    scratch_map.insert(dst, Some(bits.clone()));
                    *i = I::Constant(dst, bits);
                }
            }
        }
        I::ShiftImm(dst, op, src, amount) => {
            let src_bits = scratch_map.get(src).ok_or(())?;
            let (dst, amount) = (*dst, *amount);
            match src_bits.as_ref() {
                None => _ = scratch_map.insert(dst, None),
                Some(b) => {
                    let bits = op.evaluate(b, amount);
                    scratch_map.insert(dst, Some(bits.clone()));
                    *i = I::Constant(dst, bits);
                }
            }
        }
        I::Intrinsic(dst, ..) | I::LastUpdateTime(dst, ..) => {
            scratch_map.insert(*dst, None);
        }
        I::Probe(dst, _, _) => {
            scratch_map.insert(*dst, None);
        }
        I::ProbeSlice(dst, signal, offset) => {
            let (dst, signal) = (*dst, *signal);
            let offset_bits = scratch_map.get(offset).ok_or(())?;
            match offset_bits.as_ref() {
                None => {}
                Some(b) => match b.extract_exact_u32() {
                    None => {
                        let bits = Bits::new_unknown(vars[dst].size);
                        scratch_map.insert(dst, Some(bits.clone()));
                        *i = I::Constant(dst, bits);
                        return Ok(());
                    }
                    Some(offset) => {
                        let dst_size = vars[dst].size;
                        let src_size = signals[signal].size;
                        if offset <= src_size.get() - dst_size.get() {
                            *i = I::Probe(dst, signal, offset);
                        }
                    }
                },
            }
            scratch_map.insert(dst, None);
        }
        I::Drive(..) => {}
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
                scratch_map.insert(*dst, None);
                return Ok(());
            }
            if !is_all_complete {
                return Err(());
            }
            let acc = acc.cloned();
            scratch_map.insert(*dst, acc.clone());
            *i = I::Constant(*dst, acc.unwrap())
        }
    }

    Ok(())
}
