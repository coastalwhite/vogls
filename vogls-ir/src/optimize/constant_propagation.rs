use std::ops::Range;

use vogls_bits::Bits;
use vogls_utils::retain::slice_retain;
use vogls_utils::{VgHashMap, VgHashSet};

use crate::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryImmOpSimplification, BinaryOp,
    GlobalContext, Instruction, ProcessKey, ResizeOp, ShiftImmOp, ShiftImmOpSimplification,
    SliceImmSimplification, Time, VariableKey, simplify_slice_imm,
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
    scratch_dep: &mut VgHashMap<VariableKey, Range<usize>>,
    scratch_dep_edges: &mut Vec<VariableKey>,
) {
    let entry = gl.processes[process].entry;

    scratch_seen.clear();
    scratch_stack.clear();
    scratch_mfr.clear();
    scratch_map.clear();
    scratch_dep.clear();
    scratch_dep_edges.clear();

    let mut cycle_nodes = VgHashSet::default();
    let mut cycle_stack = Vec::new();
    let mut cycle_seen = VgHashSet::default();
    let mut cycle_work = Vec::new();
    let mut cycle_index = VgHashMap::default();
    let mut cycle_lowlink = VgHashMap::default();

    traverse_bbs(
        gl,
        entry,
        scratch_stack,
        scratch_seen,
        scratch_map,
        scratch_mfr,
        scratch_dep,
        scratch_dep_edges,
    );

    // If there are any basic-blocks that should be removed, remove them now and clear them up
    // from the phi-instructions of other basic-blocks as those might be waiting on those
    // variables to get resolved.
    if !scratch_mfr.is_empty() {
        // See what Basic Blocks are still reachable.
        scratch_seen.clear();
        scratch_stack.push(entry);
        scratch_seen.insert(entry);
        while let Some(bb_key) = scratch_stack.pop() {
            gl.bbs[bb_key].terminator.for_each_bb(|bb_key| {
                if scratch_seen.insert(bb_key) {
                    scratch_stack.push(bb_key);
                }
            });
        }
        // Remove all basic blocks that were marked for removal and are no lower reachable.
        scratch_mfr.retain(|bb_key| {
            if scratch_seen.contains(bb_key) {
                return false;
            }
            gl.bbs.remove(*bb_key);
            true
        });

        // Remove phi referenced to removed basic-blocks and mark any variables as
        // non-constants, so that they get removed as a dependency in the next stage.
        scratch_seen.clear();
        scratch_stack.push(entry);
        scratch_seen.insert(entry);
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

    if scratch_dep.is_empty() {
        return;
    }

    // Remove all dependencies that have since been resolved.
    scratch_dep.retain(|_, v| {
        let new_length = slice_retain(&mut scratch_dep_edges[v.clone()], |w| {
            !scratch_map.contains_key(w)
        });
        *v = v.start..v.start + new_length;
        new_length > 0
    });

    if !scratch_dep.is_empty() {
        // Determine all variables which are indirectly dependent on themselves. These, we cannot
        // resolve in this manner and therefore are marked as non-constant.
        find_cycle_nodes(
            &scratch_dep,
            &scratch_dep_edges,
            &mut cycle_nodes,
            &mut cycle_stack,
            &mut cycle_seen,
            &mut cycle_work,
            &mut cycle_index,
            &mut cycle_lowlink,
        );
        for &var in &cycle_nodes {
            scratch_map.insert(var, None);
        }
    }

    scratch_dep.clear();
    scratch_dep_edges.clear();

    traverse_bbs(
        gl,
        entry,
        scratch_stack,
        scratch_seen,
        scratch_map,
        scratch_mfr,
        scratch_dep,
        scratch_dep_edges,
    );
}

pub fn traverse_bbs(
    gl: &mut GlobalContext,
    entry: BasicBlockKey,
    scratch_stack: &mut Vec<BasicBlockKey>,
    scratch_seen: &mut VgHashSet<BasicBlockKey>,
    scratch_map: &mut VgHashMap<VariableKey, Option<Bits>>,
    scratch_mfr: &mut VgHashSet<BasicBlockKey>,
    scratch_dep: &mut VgHashMap<VariableKey, Range<usize>>,
    scratch_dep_edges: &mut Vec<VariableKey>,
) {
    scratch_seen.clear();
    scratch_stack.clear();
    scratch_stack.push(entry);
    while let Some(bb_key) = scratch_stack.pop() {
        let bb = &mut gl.bbs[bb_key];
        let mut all_variables_completed = true;
        let mut bb_made_progress = false;

        for i in &mut bb.instrs {
            use Instruction as I;

            // Skip the instruction if it is already handled.
            if i.get_destination_variable()
                .is_some_and(|dst| scratch_map.contains_key(&dst))
            {
                continue;
            }

            match i {
                I::Constant(dst, bits) => _ = scratch_map.insert(*dst, Some(bits.clone())),
                I::Unary(dst, op, src) => {
                    let Some(src_bits) = scratch_map.get(src) else {
                        all_variables_completed = false;
                        scratch_dep
                            .insert(*dst, scratch_dep_edges.len()..scratch_dep_edges.len() + 1);
                        scratch_dep_edges.push(*src);
                        continue;
                    };

                    let dst = *dst;
                    bb_made_progress = true;
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
                    let Some(src_bits) = scratch_map.get(src) else {
                        all_variables_completed = false;
                        scratch_dep
                            .insert(*dst, scratch_dep_edges.len()..scratch_dep_edges.len() + 1);
                        scratch_dep_edges.push(*src);
                        continue;
                    };
                    let dst = *dst;
                    bb_made_progress = true;
                    match src_bits {
                        None => _ = scratch_map.insert(dst, None),
                        Some(b) => {
                            let value = op.evaluate(b, gl.vars[dst].size);
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
                    bb_made_progress |= operands_are_complete;
                    let lhs_bits = lhs_bits_entry.map_or(None, |b| b.as_ref());
                    let rhs_bits = rhs_bits_entry.map_or(None, |b| b.as_ref());

                    use BinaryImmOp as IO;
                    use BinaryOp as O;
                    use I::BinaryImm as BI;
                    use I::ShiftImm as SI;
                    use ShiftImmOp as SO;
                    match (op, lhs_bits, rhs_bits) {
                        (_, Some(l), Some(r)) => {
                            let value = op.evaluate(l, r, gl.vars[dst].size);
                            scratch_map.insert(dst, Some(value.clone()));
                            *i = I::Constant(dst, value);
                            continue;
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
                                let value = Bits::new_unknown(gl.vars[dst].size);
                                scratch_map.insert(dst, Some(value.clone()));
                                *i = I::Constant(dst, value);
                                continue;
                            }
                            Some(offset) => *i = SI(dst, SO::LogicalShiftLeft, lhs, offset),
                        },
                        (O::LogicalShiftRight, Some(_), _) => {}
                        (O::LogicalShiftRight, _, Some(b)) => match b.extract_exact_u32() {
                            None => {
                                let value = Bits::new_unknown(gl.vars[dst].size);
                                scratch_map.insert(dst, Some(value.clone()));
                                *i = I::Constant(dst, value);
                                continue;
                            }
                            Some(offset) => *i = SI(dst, SO::LogicalShiftRight, lhs, offset),
                        },
                        (O::ArithmeticShiftRight, Some(_), _) => {}
                        (O::ArithmeticShiftRight, _, Some(b)) => match b.extract_exact_u32() {
                            None => {
                                let value = Bits::new_unknown(gl.vars[dst].size);
                                scratch_map.insert(dst, Some(value.clone()));
                                *i = I::Constant(dst, value);
                                continue;
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

                        (O::CaseEquality, Some(b), _) => {
                            *i = BI(dst, IO::CaseEquality, rhs, b.clone())
                        }
                        (O::CaseEquality, _, Some(b)) => {
                            *i = BI(dst, IO::CaseEquality, lhs, b.clone())
                        }

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
                                    bb_made_progress = true;
                                    continue;
                                }
                                S::Constant(bits) => {
                                    *i = I::Constant(dst, bits.clone());
                                    scratch_map.insert(dst, Some(bits));
                                    bb_made_progress = true;
                                    continue;
                                }
                                S::Instruction(instr) => *i = instr,
                            }
                        }
                        I::ShiftImm(dst, op, src, amount) => {
                            let (dst, src) = (*dst, *src);
                            use ShiftImmOpSimplification as S;
                            match op.simplify(gl.vars[dst].size, *amount) {
                                S::Keep => {}
                                S::Source => *i = I::Resize(dst, ResizeOp::Truncate, src),
                                S::Constant(bits) => {
                                    *i = I::Constant(dst, bits.clone());
                                    scratch_map.insert(dst, Some(bits));
                                    bb_made_progress = true;
                                    continue;
                                }
                            }
                        }
                        _ => {}
                    }

                    if operands_are_complete {
                        scratch_map.insert(dst, None);
                    } else {
                        all_variables_completed = false;
                        let start = scratch_dep_edges.len();
                        if lhs_bits_entry.is_none() {
                            scratch_dep_edges.push(lhs);
                        }
                        if rhs_bits_entry.is_none() {
                            scratch_dep_edges.push(rhs);
                        }
                        scratch_dep.insert(dst, start..scratch_dep_edges.len());
                    }
                }
                I::BinaryImm(dst, op, src, imm) => {
                    let Some(src_bits) = scratch_map.get(src) else {
                        all_variables_completed = false;
                        scratch_dep
                            .insert(*dst, scratch_dep_edges.len()..scratch_dep_edges.len() + 1);
                        scratch_dep_edges.push(*src);
                        continue;
                    };

                    let dst = *dst;
                    bb_made_progress = true;
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
                    let operands_are_complete =
                        src_bits_entry.is_some() & offset_bits_entry.is_some();
                    bb_made_progress |= operands_are_complete;
                    let src_bits = src_bits_entry.map_or(None, |b| b.as_ref());
                    let offset_bits = offset_bits_entry.map_or(None, |b| b.as_ref());

                    match (src_bits, offset_bits) {
                        (Some(l), Some(r)) => {
                            let dst_size = gl.vars[dst].size;
                            let value = match r.extract_exact_u32() {
                                None => Bits::new_unknown(dst_size),
                                Some(offset) => l.slicex(offset, dst_size),
                            };
                            scratch_map.insert(dst, Some(value.clone()));
                            *i = I::Constant(dst, value);
                            continue;
                        }
                        (Some(l), None) if l.count_unknown() == l.size().get() => {
                            let dst_size = gl.vars[dst].size;
                            let value = Bits::new_unknown(dst_size);
                            scratch_map.insert(dst, Some(value.clone()));
                            *i = I::Constant(dst, value);
                            bb_made_progress = true;
                            continue;
                        }
                        (None, Some(offset)) => {
                            let dst_size = gl.vars[dst].size;
                            let Some(offset) = offset.extract_exact_u32() else {
                                let value = Bits::new_unknown(dst_size);
                                scratch_map.insert(dst, Some(value.clone()));
                                *i = I::Constant(dst, value);
                                bb_made_progress = true;
                                continue;
                            };

                            let src_size = gl.vars[src].size;
                            if offset <= src_size.get() - dst_size.get() {
                                use SliceImmSimplification as S;
                                match simplify_slice_imm(dst, dst_size, src, src_size, offset) {
                                    S::Keep => *i = I::SliceImm(dst, src, offset),
                                    S::Source => *i = I::Resize(dst, ResizeOp::Truncate, src),
                                    S::Constant(bits) => {
                                        scratch_map.insert(dst, Some(bits.clone()));
                                        *i = I::Constant(dst, bits);
                                        bb_made_progress = true;
                                        continue;
                                    }
                                    S::Instruction(instruction) => *i = instruction,
                                }
                            }
                        }

                        (None, None) | (Some(_), None) => {}
                    };

                    if operands_are_complete {
                        scratch_map.insert(dst, None);
                    } else {
                        all_variables_completed = false;
                        let start = scratch_dep_edges.len();
                        if src_bits_entry.is_none() {
                            scratch_dep_edges.push(src);
                        }
                        if offset_bits_entry.is_none() {
                            scratch_dep_edges.push(offset);
                        }
                        scratch_dep.insert(dst, start..scratch_dep_edges.len());
                    }
                }
                I::SliceImm(dst, src, amount) => {
                    let Some(src_bits) = scratch_map.get(src) else {
                        all_variables_completed = false;
                        scratch_dep
                            .insert(*dst, scratch_dep_edges.len()..scratch_dep_edges.len() + 1);
                        scratch_dep_edges.push(*src);
                        continue;
                    };

                    let (dst, amount) = (*dst, *amount);
                    bb_made_progress = true;
                    match src_bits.as_ref() {
                        None => _ = scratch_map.insert(dst, None),
                        Some(b) => {
                            let bits = b.slicez(amount, gl.vars[dst].size);
                            scratch_map.insert(dst, Some(bits.clone()));
                            *i = I::Constant(dst, bits);
                        }
                    }
                }
                I::ShiftImm(dst, op, src, amount) => {
                    let Some(src_bits) = scratch_map.get(src) else {
                        all_variables_completed = false;
                        scratch_dep
                            .insert(*dst, scratch_dep_edges.len()..scratch_dep_edges.len() + 1);
                        scratch_dep_edges.push(*src);
                        continue;
                    };

                    let (dst, amount) = (*dst, *amount);
                    bb_made_progress = true;
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
                    bb_made_progress = true;
                    scratch_map.insert(*dst, None);
                }
                I::Probe(dst, _) => {
                    bb_made_progress = true;
                    scratch_map.insert(*dst, None);
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
                        bb_made_progress = true;
                        scratch_map.insert(*dst, None);
                        continue;
                    }
                    if !is_all_complete {
                        all_variables_completed = false;
                        let start = scratch_dep_edges.len();
                        scratch_dep_edges.extend(srcs.iter().filter_map(|(_, src)| {
                            if scratch_map.contains_key(src) {
                                None
                            } else {
                                Some(*src)
                            }
                        }));
                        scratch_dep.insert(*dst, start..scratch_dep_edges.len());
                        continue;
                    }
                    bb_made_progress = true;
                    let acc = acc.cloned();
                    scratch_map.insert(*dst, acc.clone());
                    *i = I::Constant(*dst, acc.unwrap())
                }
            }
        }

        match &bb.terminator {
            BasicBlockTerminator::VariableWait(target, time) => {
                if let Some(time) = scratch_map.get(time) {
                    if let Some(time) = time.as_ref() {
                        let time = time.extract_exact_u64().unwrap_or(0);
                        bb.terminator = BasicBlockTerminator::Wait(*target, Time(time));
                    }
                } else {
                    all_variables_completed = false;
                }
            }
            BasicBlockTerminator::Branch(condition, truthy, falsy) => {
                if let Some(condition) = scratch_map.get(condition) {
                    if let Some(condition) = condition.as_ref() {
                        let (taken, untaken) = if condition.eq_one() {
                            (*truthy, *falsy)
                        } else {
                            (*falsy, *truthy)
                        };

                        bb.terminator = BasicBlockTerminator::Jump(taken);
                        scratch_mfr.insert(untaken);
                    }
                } else {
                    all_variables_completed = false;
                }
            }
            BasicBlockTerminator::Wait(..)
            | BasicBlockTerminator::WaitRegion(..)
            | BasicBlockTerminator::Watch(..)
            | BasicBlockTerminator::Jump(..)
            | BasicBlockTerminator::Halt => {}
        }

        if all_variables_completed || !bb_made_progress {
            scratch_seen.insert(bb_key);
        }

        bb.terminator.for_each_bb(|bb_key| {
            if !scratch_seen.contains(&bb_key) {
                scratch_stack.push(bb_key);
            }
        });
    }
}

/// Find the nodes in graph `(nodes, edges)` that appear in a cycle and place them in
/// `cycle_nodes`.
///
/// This is an implementation based on Tarjan's Algorithm for Strongly Connected Components (SCCs)
/// where we filter out the SCCs that have more than 1 node.
///
/// One weird part of this specific implementation is that `edges` might mention nodes that are not
/// in `nodes`. This means they have no edges and thus are by definition not in a cycle.
fn find_cycle_nodes<K: Eq + std::hash::Hash + Copy>(
    nodes: &VgHashMap<K, Range<usize>>,
    edges: &[K],
    cycle_nodes: &mut VgHashSet<K>,

    stack: &mut Vec<K>,
    seen: &mut VgHashSet<K>,
    work: &mut Vec<(K, usize)>,

    index: &mut VgHashMap<K, usize>,
    lowlink: &mut VgHashMap<K, usize>,
) {
    cycle_nodes.clear();
    stack.clear();
    seen.clear();
    work.clear();
    index.clear();
    lowlink.clear();

    let mut index_ctr: usize = 0;

    for (&start, _) in nodes.iter() {
        if index.contains_key(&start) {
            continue;
        }

        index.insert(start, index_ctr);
        lowlink.insert(start, index_ctr);
        index_ctr += 1;
        stack.push(start);
        seen.insert(start);
        work.push((start, 0));

        while let Some((v, mut neighbor_i)) = work.pop() {
            let range = nodes[&v].clone();
            let edge_idx = range.start + neighbor_i;

            if edge_idx < range.end {
                neighbor_i += 1;
                let w = edges[edge_idx];

                if !nodes.contains_key(&w) {
                    continue;
                }

                if !index.contains_key(&w) {
                    index.insert(w, index_ctr);
                    lowlink.insert(w, index_ctr);
                    index_ctr += 1;
                    stack.push(w);
                    seen.insert(w);
                    work.push((v, neighbor_i));
                    work.push((w, 0));
                } else if seen.contains(&w) {
                    let w_idx = index[&w];
                    *lowlink.get_mut(&v).unwrap() = lowlink[&v].min(w_idx);
                }
            } else {
                if let Some(&(parent, _)) = work.last() {
                    let v_ll = lowlink[&v];
                    *lowlink.get_mut(&parent).unwrap() = lowlink[&parent].min(v_ll);
                }

                if lowlink[&v] == index[&v] {
                    let scc_start = stack.iter().rposition(|&x| x == v).unwrap();
                    let scc = &stack[scc_start..];
                    if scc.len() >= 2 {
                        cycle_nodes.extend(scc.iter().copied());
                    }
                    for w in stack.drain(scc_start..) {
                        seen.remove(&w);
                    }
                }
            }
        }
    }
}
