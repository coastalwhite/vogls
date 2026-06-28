use vogls_codegen::lsra::{Slot, StackItemKind, StackOffsets, StackTracker};
use vogls_codegen::{HeapOffset, HeapRef, insert_bb_phis, resolve_var_logic_mode_map};
use vogls_ir::watchers::WatchMap;
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, Bits, ContextFormat,
    DisplayContext, GlobalContext, Instruction, LogicMode, ProcessKey, ResizeOp, SignalKey,
    UnaryOp, VSIZE_64, VariableKey, VariableMap, VectorSize,
};
use vogls_runtime::RtSignalKey;
use vogls_utils::{NonMaxU16, VgHashMap, VgHashSet};

use crate::bytecode::{
    BitwiseOp, Bytecode, BytecodeEncoder, BytecodeKind, BytecodeListeners, EncJump, EncRelJump,
    Encoding, InstructionPtr, IntrinsicOpEqWrap, Reg, Schedule, SixBitSize,
};

enum JumpKind {
    Jump,
    Branch,
}

pub fn lower_process_to_bytecode(
    process: ProcessKey,
    gl: &GlobalContext,
    stack_tracker: &mut StackTracker,
    max_stack_words: &mut usize,
    watch_map: &WatchMap,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    bytecode: &mut BytecodeEncoder,
) {
    const PRINT: bool = false;

    let process = &gl.processes[process];

    let mut bb_stack = Vec::new();
    let mut bb_stack2 = Vec::new();
    let mut bb_seen = VgHashSet::<BasicBlockKey>::default();
    let mut bb_phis = VgHashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::default();

    let mut assignment = VgHashMap::default();
    let mut var_mode = VgHashMap::<VariableKey, LogicMode>::default();
    let mut conv_map = VgHashMap::<VariableKey, HeapOffset>::default();

    let mut post_order = Vec::<BasicBlockKey>::new();
    let mut bb_offsets = VgHashMap::<BasicBlockKey, usize>::default();
    let mut jump_targets = Vec::<(usize, BasicBlockKey, JumpKind)>::new();

    schedule.push_active(InstructionPtr(bytecode.data.len() as u64));
    for tr in &process.regions {
        assignment.clear();
        var_mode.clear();
        conv_map.clear();
        post_order.clear();

        resolve_var_logic_mode_map(
            &process.regions,
            gl,
            &mut bb_stack,
            &mut bb_seen,
            &mut var_mode,
            &mut conv_map,
        );
        insert_bb_phis(
            &process.regions,
            gl,
            &mut bb_stack,
            &mut bb_seen,
            &mut bb_phis,
        );

        vogls_ir::orders::post_order_keys(
            tr.entry(),
            &gl.bbs,
            &mut bb_seen,
            &mut bb_stack2,
            &mut post_order,
        );

        vogls_codegen::lsra::linear_scan_register_allocation(
            &post_order,
            &gl.vars,
            &gl.bbs,
            &mut assignment,
            stack_tracker,
            9,
        );

        let stack_offsets = stack_tracker.offsets();
        *max_stack_words = usize::max(*max_stack_words, stack_tracker.num_words());

        let mut ctx = DisplayContext::new(gl);
        ctx.prepare_process(tr.entry());

        post_order.reverse();
        for &bb_key in &post_order {
            bb_offsets.insert(bb_key, bytecode.data.len());

            let bb = &gl.bbs[bb_key];
            for i in &bb.instrs {
                let offset = bytecode.data.len();
                if PRINT {
                    println!("{}", i.display(&ctx));
                }
                lower_instruction(
                    gl,
                    bytecode,
                    &assignment,
                    &stack_offsets,
                    watch_map,
                    signals,
                    io_signals,
                    i,
                );
                if PRINT {
                    for c in &bytecode.data[offset..] {
                        println!("  {c}");
                    }
                }
            }

            use BasicBlockTerminator as T;
            match &bb.terminator {
                T::Wait(target, time) => {
                    let time = time.0;

                    if time != 0 {
                        bytecode.load_u64(T0, time);
                        bytecode.wait(T0);
                    }
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.jump(0);
                }
                T::VariableWait(target, src) => {
                    if src.mode() == LogicMode::FourValue {
                        todo!();
                    }

                    let rtime = to_reg(
                        bytecode,
                        *src,
                        &gl.vars,
                        assignment[src],
                        &stack_offsets,
                        T0,
                    );
                    bytecode.wait(rtime);
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.jump(0);
                }
                T::WaitRegion(target, region) => {
                    bytecode.wait_region(*region);
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.jump(0);
                }
                T::Watch(target, _) => {
                    let index = watch_map.get_watch_index(bb_key);
                    bytecode.start_listen(index as u32);
                    bytecode.next_event();
                    listeners.set_ptr(index, bytecode.current_ptr());
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.jump(0);
                }
                T::Jump(target) => {
                    jump_targets.push((bytecode.data.len(), *target, JumpKind::Jump));
                    bytecode.jump(0);
                }
                T::Branch(cond, truthy, falsy) => {
                    if cond.mode() == LogicMode::FourValue {
                        todo!();
                    }

                    let rcond = to_reg(
                        bytecode,
                        *cond,
                        &gl.vars,
                        assignment[cond],
                        &stack_offsets,
                        T0,
                    );
                    jump_targets.push((bytecode.data.len(), *truthy, JumpKind::Branch));
                    bytecode.branch(rcond, 0);
                    jump_targets.push((bytecode.data.len(), *falsy, JumpKind::Jump));
                    bytecode.jump(0);
                }
                T::Halt => {
                    bytecode.next_event();
                }
            }
        }
    }

    for (offset, target, kind) in jump_targets {
        let target_offset = bb_offsets[&target];
        let imm = target_offset.abs_diff(offset);
        assert!(imm < (1 << 19));
        let mut imm = imm as i32;
        if target_offset < offset {
            imm = -imm;
        }
        bytecode.data[offset] = match kind {
            JumpKind::Jump => Bytecode((BytecodeKind::jump as u32) | EncJump { imm }.encode()),
            JumpKind::Branch => {
                let mut enc = EncRelJump::extract(bytecode.data[offset]);
                enc.imm = imm;
                Bytecode((BytecodeKind::branch as u32) | enc.encode())
            }
        };
    }
}

const T0: Reg = Reg::X9;
const T1: Reg = Reg::X11;
const T2: Reg = Reg::X13;
const SP: Reg = Reg::X15;

fn lower_instruction(
    gl: &GlobalContext,
    bytecode: &mut BytecodeEncoder,
    assignment: &VgHashMap<VariableKey, Slot>,
    stack_offsets: &StackOffsets,
    watch_map: &WatchMap,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    instr: &Instruction,
) {
    use Instruction as I;
    match instr {
        I::Constant(dst, value) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, *dst, &gl.vars, dslot, stack_offsets, T0);

            if value.size() > VSIZE_64 {
                todo!()
            }

            bytecode.load_bits_into_register(rd, dst.mode(), value);
            store_back(bytecode, dslot, rd);
        }
        I::Unary(dst, op, src) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, *dst, &gl.vars, dslot, stack_offsets, T0);
            let rs = to_reg(bytecode, *src, &gl.vars, assignment[src], stack_offsets, T1);

            let src_size = gl.vars.size(*src);

            use LogicMode as M;
            use UnaryOp as O;
            if let Some(src_size) = SixBitSize::from_vector_size(src_size) {
                match (op, dst.mode(), src.mode()) {
                    (O::Neg, M::TwoValue, _) => bytecode.not(rd, rs, src_size),
                    (O::Neg, M::FourValue, _) => bytecode.fv_not(rd, rs),
                    (O::ReduceOr, M::TwoValue, _) => bytecode.cnei(rd, rs, 0, src_size),
                    (O::ReduceOr, M::FourValue, _) => bytecode.fv_reduce_or(rd, rs, src_size),
                    (O::ReduceAnd, M::TwoValue, _) => bytecode.ceqi(rd, rs, -1, src_size),
                    (O::ReduceAnd, M::FourValue, _) => bytecode.fv_reduce_and(rd, rs, src_size),
                    (O::ReduceXor, M::TwoValue, _) => {
                        bytecode.count_ones(rd, rs);
                        bytecode.truncate(rd, rd, SixBitSize::SCALAR);
                    }
                    (O::ReduceXor, M::FourValue, _) => bytecode.fv_reduce_xor(rd, rs, src_size),
                    (O::LeadingZeros, M::TwoValue, _) => todo!(),
                    (O::LeadingZeros, M::FourValue, _) => todo!(),
                    (O::TvToFv, ..) => {
                        // @Performance: better lowering.
                        let (spc, val) = reg_as_fv(rd);
                        bytecode.copy(val, rs);
                        bytecode.load_u64(spc, src_size.mask(u64::MAX));
                    }
                    (O::FvToTv, ..) => {
                        let (spc, val) = reg_as_fv(rs);
                        bytecode.and(rd, spc, val)
                    }
                }
            } else {
                todo!()
            }

            store_back(bytecode, dslot, rd);
        }
        I::Resize(dst, op, src) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, *dst, &gl.vars, dslot, stack_offsets, T0);
            let rs = to_reg(bytecode, *src, &gl.vars, assignment[src], stack_offsets, T1);

            let dst_size = gl.vars.size(*dst);
            let src_size = gl.vars.size(*src);

            use LogicMode as M;
            use ResizeOp as O;
            match (
                op,
                dst.mode(),
                SixBitSize::from_vector_size(dst_size),
                SixBitSize::from_vector_size(src_size),
            ) {
                (O::Truncate, M::TwoValue, Some(dst_size), Some(_)) => {
                    bytecode.truncate(rd, rs, dst_size);
                }
                (O::Truncate, M::FourValue, Some(dst_size), Some(_)) => {
                    // @Performance: One instruction maybe?
                    let (rsspc, rsval) = rd.to_spc_and_val();
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    bytecode.truncate(rdspc, rsspc, dst_size);
                    bytecode.truncate(rdval, rsval, dst_size);
                }
                (O::Truncate, ..) => {
                    todo!()
                }
                (O::ZeroExtend, M::TwoValue, Some(_), Some(_)) => {
                    bytecode.copy(rd, rs);
                }
                (O::ZeroExtend, M::FourValue, Some(dst_size), Some(src_size)) => {
                    // @Performance: One instruction maybe?
                    let (rsspc, rsval) = rd.to_spc_and_val();
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    let mask = dst_size.mask(u64::MAX) ^ src_size.mask(u64::MAX);
                    if let Some(value) = lower_to_n_bits_sign_extend(&Bits::new_u64(mask), 10) {
                        bytecode.ori(rdspc, rsspc, value as i16, dst_size);
                    } else {
                        bytecode.load_u64(T2, mask);
                        bytecode.or(rdspc, rsspc, T2);
                    }
                    bytecode.copy(rdval, rsval);
                }
                (O::ZeroExtend, ..) => todo!(),
                (O::SignExtend, ..) => todo!(),
            }

            store_back(bytecode, dslot, rd);
        }
        I::Binary(dst, op, lhs, rhs) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, *dst, &gl.vars, dslot, stack_offsets, T0);
            let rs1 = to_reg(bytecode, *lhs, &gl.vars, assignment[lhs], stack_offsets, T1);
            let rs2 = to_reg(bytecode, *rhs, &gl.vars, assignment[rhs], stack_offsets, T2);

            let dst_size = gl.vars.size(*dst);

            use BinaryOp as O;
            use LogicMode as M;
            match (
                op,
                dst.mode(),
                SixBitSize::from_vector_size(dst_size),
                lhs.mode(),
                rhs.mode(),
            ) {
                (O::And, M::TwoValue, Some(_), _, _) => bytecode.and(rd, rs1, rs2),
                (O::And, M::TwoValue, None, _, _) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::And, size)
                }
                (O::And, M::FourValue, Some(_), _, _) => bytecode.fv_and(rd, rs1, rs2),
                (O::And, M::FourValue, None, _, _) => todo!(),
                (O::Or, M::TwoValue, Some(_), _, _) => bytecode.or(rd, rs1, rs2),
                (O::Or, M::TwoValue, None, _, _) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Or, size)
                }
                (O::Or, M::FourValue, Some(_), _, _) => bytecode.fv_or(rd, rs1, rs2),
                (O::Or, M::FourValue, None, _, _) => todo!(),
                (O::Xor, M::TwoValue, Some(_), _, _) => bytecode.xor(rd, rs1, rs2),
                (O::Xor, M::TwoValue, None, _, _) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Xor, size)
                }
                (O::Xor, M::FourValue, Some(_), _, _) => bytecode.fv_xor(rd, rs1, rs2),
                (O::Xor, M::FourValue, None, _, _) => todo!(),
                (O::Add, M::TwoValue, Some(size), _, _) => bytecode.add(rd, rs1, rs2, size),
                (O::Add, M::TwoValue, None, _, _) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Add, size)
                }
                (O::Add, M::FourValue, _, _, _) => todo!(),
                (O::Sub, M::TwoValue, Some(size), _, _) => bytecode.sub(rd, rs1, rs2, size),
                (O::Sub, M::TwoValue, None, _, _) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Sub, size)
                }
                (O::Sub, M::FourValue, _, _, _) => todo!(),
                (O::Multiply, M::TwoValue, Some(size), _, _) => bytecode.mul(rd, rs1, rs2, size),
                (O::Multiply, M::TwoValue, None, _, _) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Mul, size)
                }
                (O::Multiply, M::FourValue, _, _, _) => todo!(),
                (O::Power, ..) => todo!(),
                (O::Divide, ..) => todo!(),
                (O::Modulus, ..) => todo!(),
                (O::UnsignedLessEqual, ..) => todo!(),
                (O::LogicalShiftLeft, ..) => todo!(),
                (O::LogicalShiftRight, ..) => todo!(),
                (O::ArithmeticShiftRight, ..) => todo!(),
                (O::Concat, ..) => todo!(),
                (O::CopyX, ..) => todo!(),
                (O::CopyZ, ..) => todo!(),
                (O::Min, ..) => todo!(),
                (O::Max, ..) => todo!(),
                (O::CaseEquality, _, Some(_), M::TwoValue, _) => bytecode.ceq(rd, rs1, rs2),
                (O::CaseEquality, _, None, M::TwoValue, _) => todo!(),
                (O::CaseEquality, _, Some(_), M::FourValue, _) => bytecode.fv_ceq(rd, rs1, rs2),
                (O::CaseEquality, _, None, M::FourValue, _) => todo!(),
                (O::Posedge, ..) => todo!(),
                (O::Negedge, ..) => todo!(),
            }

            store_back(bytecode, dslot, rd);
        }
        I::BinaryImm(dst, op, src, imm) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, *dst, &gl.vars, dslot, stack_offsets, T0);
            let rs = to_reg(bytecode, *src, &gl.vars, assignment[src], stack_offsets, T1);

            let dst_size = gl.vars.size(*dst);
            let src_size = gl.vars.size(*src);

            use BinaryImmOp as O;
            use LogicMode as M;
            match (
                op,
                dst.mode(),
                SixBitSize::from_vector_size(dst_size),
                src.mode(),
                imm.contains_special(),
                SixBitSize::from_vector_size(src_size),
            ) {
                (O::And, M::TwoValue, Some(size), _, _, _) => {
                    match lower_to_n_bits_sign_extend(imm, 10) {
                        None => {
                            bytecode.load_u64(
                                T2,
                                imm.zero_extend(VSIZE_64).extract_exact_u64().unwrap(),
                            );
                            bytecode.and(rd, rs, T2);
                        }
                        Some(imm) => {
                            bytecode.andi(rd, rs, imm as i16, size);
                        }
                    }
                }
                (O::And, M::TwoValue, None, _, _, _) => todo!(),
                (O::And, M::FourValue, Some(_), _, _, _) => todo!(),
                (O::And, M::FourValue, None, _, _, _) => todo!(),
                (O::Or, M::TwoValue, Some(size), _, _, _) => {
                    match lower_to_n_bits_sign_extend(imm, 10) {
                        None => {
                            bytecode.load_u64(
                                T2,
                                imm.zero_extend(VSIZE_64).extract_exact_u64().unwrap(),
                            );
                            bytecode.or(rd, rs, T2);
                        }
                        Some(imm) => {
                            bytecode.ori(rd, rs, imm as i16, size);
                        }
                    }
                }
                (O::Or, M::TwoValue, None, _, _, _) => todo!(),
                (O::Or, M::FourValue, Some(_), _, _, _) => todo!(),
                (O::Or, M::FourValue, None, _, _, _) => todo!(),
                (O::Xor, M::TwoValue, Some(size), _, _, _) => {
                    match lower_to_n_bits_sign_extend(imm, 10) {
                        None => {
                            bytecode.load_u64(
                                T2,
                                imm.zero_extend(VSIZE_64).extract_exact_u64().unwrap(),
                            );
                            bytecode.xor(rd, rs, T2);
                        }
                        Some(imm) => {
                            bytecode.xori(rd, rs, imm as i16, size);
                        }
                    }
                }
                (O::Xor, M::TwoValue, None, _, _, _) => todo!(),
                (O::Xor, M::FourValue, Some(_), _, _, _) => todo!(),
                (O::Xor, M::FourValue, None, _, _, _) => todo!(),
                (O::Add, M::TwoValue, Some(_), _, _, _) => todo!(),
                (O::Add, M::TwoValue, None, _, _, _) => todo!(),
                (O::Add, M::FourValue, _, _, _, _) => todo!(),
                (O::Sub, M::TwoValue, Some(_), _, _, _) => todo!(),
                (O::Sub, M::TwoValue, None, _, _, _) => todo!(),
                (O::Sub, M::FourValue, _, _, _, _) => todo!(),
                (O::Multiply, M::TwoValue, Some(_), _, _, _) => todo!(),
                (O::Multiply, M::TwoValue, None, _, _, _) => todo!(),
                (O::Multiply, M::FourValue, _, _, _, _) => todo!(),
                (O::Power, ..) => todo!(),
                (O::Divide, ..) => todo!(),
                (O::Modulus, ..) => todo!(),
                (O::RevSub, ..) => todo!(),
                (O::RevPower, ..) => todo!(),
                (O::RevDivide, ..) => todo!(),
                (O::RevModulus, ..) => todo!(),
                (O::UnsignedLessEqual, ..) => todo!(),
                (O::UnsignedGreaterEqual, ..) => todo!(),
                (O::ConcatLeft, ..) => todo!(),
                (O::ConcatRight, ..) => todo!(),
                (O::Min, ..) => todo!(),
                (O::Max, ..) => todo!(),
                (O::CaseEquality, _, _, M::TwoValue, _, Some(_)) => {
                    match lower_to_n_bits_sign_extend(imm, 10) {
                        None => {
                            bytecode.load_u64(
                                T2,
                                imm.zero_extend(VSIZE_64).extract_exact_u64().unwrap(),
                            );
                            bytecode.ceq(rd, rs, T2);
                        }
                        Some(imm) => {
                            let size = SixBitSize::from_vector_size(src_size).unwrap();
                            bytecode.ceqi(rd, rs, imm as i16, size);
                        }
                    }
                }
                (O::CaseEquality, _, _, M::TwoValue, _, None) => todo!(),
                (O::CaseEquality, _, _, M::FourValue, _, Some(_)) => {
                    if !imm.contains_special()
                        && let Some(imm) = lower_to_n_bits_sign_extend(imm, 10)
                    {
                        let size = SixBitSize::from_vector_size(src_size).unwrap();
                        bytecode.fv_ceqi(rd, rs, imm as i16, size)
                    } else {
                        bytecode.load_bits_into_register(T2, LogicMode::FourValue, &imm);
                        bytecode.fv_ceq(rd, rs, T2);
                    }
                }
                (O::CaseEquality, _, _, M::FourValue, _, None) => todo!(),
            }

            store_back(bytecode, dslot, rd);
        }
        I::Slice(variable_key, variable_key1, variable_key2) => todo!(),
        I::SliceImm(variable_key, variable_key1, _) => todo!(),
        I::ShiftImm(variable_key, shift_imm_op, variable_key1, _) => todo!(),
        I::Select(variable_key, variable_key1, variable_key2, variable_key3) => todo!(),
        I::Intrinsic(dst, op, items) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, *dst, &gl.vars, dslot, stack_offsets, T0);

            for item in items {
                let reg = to_reg(
                    bytecode,
                    *item,
                    &gl.vars,
                    assignment[item],
                    stack_offsets,
                    T2,
                );
                let size = to_8bit_size(bytecode, gl.vars.size(*item));
                bytecode.push_argument(size, item.mode(), reg);
            }
            let intrinsic_id = bytecode
                .intrinsics
                .insert_index(IntrinsicOpEqWrap(op.as_ref().clone()));
            let intrinsic_id = match intrinsic_id.try_into().ok().and_then(|v| NonMaxU16::new(v)) {
                None => {
                    bytecode.load_u64(T2, intrinsic_id as u64);
                    None
                }
                Some(v) => Some(v),
            };
            bytecode.intrinsic(rd, intrinsic_id);
        }
        I::LastUpdateTime(variable_key, signal_key) => todo!(),
        I::Probe(dst, signal, offset) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, *dst, &gl.vars, dslot, stack_offsets, T0);

            let dst_size = gl.vars.size(*dst);
            let signal_size = gl.signals[*signal].size;
            assert!(dst_size <= signal_size);

            if *offset != 0 {
                todo!()
            }

            let Some(signal_size) = SixBitSize::from_vector_size(signal_size) else {
                todo!()
            };

            let roff = T1;
            load_signal_address(bytecode, roff, *signal, signals, io_signals);
            match dst.mode() {
                LogicMode::TwoValue => bytecode.load_aligned(rd, roff, 0, signal_size),
                LogicMode::FourValue => bytecode.load_fv_aligned(rd, roff, 0, signal_size),
            }

            let dst_size = SixBitSize::from_vector_size(dst_size).unwrap();
            if dst_size != signal_size {
                bytecode.mask(rd, rd, dst_size);
            }
            store_back(bytecode, dslot, rd);
        }
        I::ProbeSlice(variable_key, signal_key, variable_key1) => todo!(),
        I::Drive(signal, src, partial) => {
            let rs = to_reg(bytecode, *src, &gl.vars, assignment[src], stack_offsets, T0);

            let signal_size = gl.signals[*signal].size;
            let src_size = gl.vars.size(*src);

            if partial.is_some() {
                todo!()
            }

            if signal_size != src_size {
                todo!()
            }

            let Some(signal_size) = SixBitSize::from_vector_size(signal_size) else {
                todo!()
            };

            let roff = T1;
            let rpoke = T2;
            load_signal_address(bytecode, roff, *signal, signals, io_signals);
            match src.mode() {
                LogicMode::TwoValue => bytecode.set_aligned(rpoke, rs, roff, 0, signal_size),
                LogicMode::FourValue => bytecode.set_fv_aligned(rpoke, rs, roff, 0, signal_size),
            }

            for index in watch_map.watch_indices(*signal) {
                // @TODO: This should have some register based fallback.
                assert!(index < (1 << 20));
                bytecode.wake(rpoke, index as u32);
            }
        }
        I::Phi(variable_key, items) => todo!(),
    }
}

fn reg_as_fv(reg: Reg) -> (Reg, Reg) {
    assert_ne!(reg, Reg::X15);
    (reg, Reg::new_masked((reg as u32) + 1))
}

fn to_reg(
    bytecode: &mut BytecodeEncoder,
    var: VariableKey,
    vars: &VariableMap,
    slot: Slot,
    stack_offsets: &StackOffsets,
    backup: Reg,
) -> Reg {
    match slot {
        Slot::Stack(kind, offset) => {
            let kind_offset = match kind {
                StackItemKind::B1 => 0,
                StackItemKind::B2 => stack_offsets.b2,
                StackItemKind::B4 => stack_offsets.b4,
                StackItemKind::B8 => stack_offsets.b8,
                StackItemKind::B16 => stack_offsets.b16,
                StackItemKind::B32 => stack_offsets.b32,
                StackItemKind::B64 => stack_offsets.b64,
            };
            let offset = kind_offset as u64 + offset as u64;
            bytecode.stack_offset(backup, kind, offset);
            let size = vars.size(var);
            if let Some(size) = SixBitSize::from_vector_size(size) {
                match var.mode() {
                    LogicMode::TwoValue => bytecode.load_aligned(backup, backup, 0, size),
                    LogicMode::FourValue => bytecode.load_fv_aligned(backup, backup, 0, size),
                }
            }
            backup
        }
        Slot::Register(reg) => Reg::new_masked(reg),
    }
}

fn to_8bit_size(bytecode: &mut BytecodeEncoder, size: VectorSize) -> Option<VectorSize> {
    if size.get() > 256 {
        bytecode.load_u64(T0, size.get().into());
        None
    } else {
        Some(size)
    }
}

fn load_signal_address(
    bytecode: &mut BytecodeEncoder,
    dst: Reg,
    signal: SignalKey,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
) {
    let signal = io_signals[&signal];
    let at = signals[signal.as_usize()];
    bytecode.load_u64(dst, at.offset.bit_offset as u64);
}

fn store_back(bytecode: &mut BytecodeEncoder, slot: Slot, value: Reg) {
    match slot {
        Slot::Stack(..) => todo!(),
        Slot::Register(rd) => {
            let rd = Reg::new_masked(rd);
            bytecode.copy(rd, value);
        }
    }
}

fn lower_to_n_bits_sign_extend(imm: &Bits, n: u32) -> Option<i64> {
    if imm.size() > VSIZE_64 {
        return None;
    }

    let initial_size = VectorSize::new(n).unwrap();
    let adjusted = if imm.size() > initial_size {
        imm.truncate(initial_size).sign_extend(imm.size())
    } else {
        imm.sign_extend(imm.size())
    };
    if imm == &adjusted {
        Some(imm.sign_extend(VSIZE_64).extract_exact_u64().unwrap() as i64)
    } else {
        None
    }
}
