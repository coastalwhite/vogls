use vogls_codegen::lsra::{Slot, StackItemKind, StackOffsets, StackTracker};
use vogls_codegen::{HeapBuilder, HeapOffset, HeapRef, insert_bb_phis, resolve_var_logic_mode_map};
use vogls_ir::watchers::WatchMap;
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, Bits, ContextFormat,
    DisplayContext, GlobalContext, Instruction, LogicMode, ProcessKey, ResizeOp, SignalKey,
    UnaryOp, VSIZE_64, VariableKey, VariableMap, VectorSize,
};
use vogls_runtime::RtSignalKey;
use vogls_utils::{NonMaxU16, VgHashMap, VgHashSet};

use crate::bytecode::{
    Branch, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, InstructionPtr,
    IntrinsicOpEqWrap, Jump, Reg, Schedule, SixBitSize,
};

enum JumpKind {
    Jump,
    Branch,
}

pub fn lower_process_to_bytecode(
    process: ProcessKey,
    gl: &GlobalContext,
    stack_tracker: &mut StackTracker,
    heap: &mut HeapBuilder,
    max_stack_words: &mut usize,
    watch_map: &WatchMap,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    bytecode: &mut BytecodeEncoder,
) {
    const PRINT: bool = true;

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
            heap,
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
            JumpKind::Jump => Jump(imm).encode(),
            JumpKind::Branch => {
                let mut enc = Branch::extract(bytecode.data[offset]);
                enc.imm = imm;
                enc.encode()
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
    bce: &mut BytecodeEncoder,
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
            // @NOTE:
            // Constants with size greater than 64 are stored on the heap and referenced on there.
            if value.size() <= VSIZE_64 {
                let dslot = assignment[dst];
                let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0);
                bce.load_bits_into_register(rd, dst.mode(), value);
                store_back(bce, &gl.vars, *dst, dslot, rd);
            }
        }
        I::Unary(dst, op, src) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0);
            let rs = to_reg(bce, *src, &gl.vars, assignment[src], stack_offsets, T1);

            let src_size = gl.vars.size(*src);

            use LogicMode as M;
            use UnaryOp as O;
            match (
                op,
                dst.mode(),
                src.mode(),
                SixBitSize::from_vector_size(src_size),
            ) {
                (O::Neg, M::TwoValue, _, Some(src_size)) => bce.not(rd, rs, src_size),
                (O::Neg, M::FourValue, _, Some(_)) => bce.fv_not(rd, rs),
                (O::ReduceOr, M::TwoValue, _, Some(src_size)) => bce.cnei(rd, rs, 0, src_size),
                (O::ReduceOr, M::FourValue, _, Some(src_size)) => {
                    bce.fv_reduce_or(rd, rs, src_size)
                }
                (O::ReduceAnd, M::TwoValue, _, Some(src_size)) => bce.ceqi(rd, rs, -1, src_size),
                (O::ReduceAnd, M::FourValue, _, Some(src_size)) => {
                    bce.fv_reduce_and(rd, rs, src_size)
                }
                (O::ReduceXor, M::TwoValue, _, Some(_)) => {
                    bce.count_ones(rd, rs);
                    bce.truncate(rd, rd, SixBitSize::SCALAR);
                }
                (O::ReduceXor, M::FourValue, _, Some(src_size)) => {
                    bce.fv_reduce_xor(rd, rs, src_size)
                }
                (O::LeadingZeros, M::TwoValue, _, Some(_)) => todo!(),
                (O::LeadingZeros, M::FourValue, _, Some(_)) => todo!(),
                (O::TvToFv, _, _, Some(src_size)) => {
                    // @Performance: better lowering.
                    let (spc, val) = reg_as_fv(rd);
                    bce.copy(val, rs);
                    bce.load_u64(spc, src_size.mask(u64::MAX));
                }
                (O::TvToFv, _, _, None) => {
                    // @Performance: better lowering.
                    let num_words = src_size.get().div_ceil(64) as u64;
                    // ORNOT with self is a fill 1's
                    bce.heap_tv_ornot(rd, rs, rs, src_size);
                    let val = T2;
                    match lower_to_n_bits_sign_extend(&Bits::new_u64(num_words * 64), 10) {
                        None => {
                            bce.load_u64(val, num_words);
                            bce.add(val, rd, val, SixBitSize::N64);
                        }
                        Some(imm10) => bce.addi(val, rd, imm10 as i16, SixBitSize::N64),
                    }
                    // OR with self is a copy
                    bce.heap_tv_or(val, rs, rs, src_size);
                }
                (O::FvToTv, _, _, Some(_)) => {
                    let (spc, val) = reg_as_fv(rs);
                    bce.and(rd, spc, val)
                }
                (O::FvToTv, _, _, None) => {
                    let num_words = src_size.get().div_ceil(64) as u64;
                    let spc = rs;
                    let val = T2;
                    match lower_to_n_bits_sign_extend(&Bits::new_u64(num_words), 10) {
                        None => {
                            bce.load_u64(val, num_words);
                            bce.add(val, rs, val, SixBitSize::N64);
                        }
                        Some(imm10) => bce.addi(val, rs, imm10 as i16, SixBitSize::N64),
                    }
                    bce.heap_tv_and(rd, spc, val, src_size);
                }
                (_, _, _, None) => todo!(),
            }

            store_back(bce, &gl.vars, *dst, dslot, rd);
        }
        I::Resize(dst, op, src) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0);
            let rs = to_reg(bce, *src, &gl.vars, assignment[src], stack_offsets, T1);

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
                    bce.truncate(rd, rs, dst_size);
                }
                (O::Truncate, M::FourValue, Some(dst_size), Some(_)) => {
                    // @Performance: One instruction maybe?
                    let (rsspc, rsval) = rd.to_spc_and_val();
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    bce.truncate(rdspc, rsspc, dst_size);
                    bce.truncate(rdval, rsval, dst_size);
                }
                (O::Truncate, ..) => {
                    todo!()
                }
                (O::ZeroExtend, M::TwoValue, Some(_), Some(_)) => {
                    bce.copy(rd, rs);
                }
                (O::ZeroExtend, M::FourValue, Some(dst_size), Some(src_size)) => {
                    // @Performance: One instruction maybe?
                    let (rsspc, rsval) = rs.to_spc_and_val();
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    let mask = dst_size.mask(u64::MAX) ^ src_size.mask(u64::MAX);
                    if let Some(value) = lower_to_n_bits_sign_extend(&Bits::new_u64(mask), 10) {
                        bce.ori(rdspc, rsspc, value as i16, dst_size);
                    } else {
                        bce.load_u64(T2, mask);
                        bce.or(rdspc, rsspc, T2);
                    }
                    bce.copy(rdval, rsval);
                }
                (O::ZeroExtend, M::TwoValue, None, _) => {
                    todo!()
                }
                (O::ZeroExtend, M::FourValue, ..) => todo!(),

                (O::ZeroExtend, _, Some(_), None) | (O::SignExtend, _, Some(_), None) => {
                    unreachable!()
                }
                (O::SignExtend, ..) => todo!(),
            }

            store_back(bce, &gl.vars, *dst, dslot, rd);
        }
        I::Binary(dst, op, lhs, rhs) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0);
            let rs1 = to_reg(bce, *lhs, &gl.vars, assignment[lhs], stack_offsets, T1);
            let rs2 = to_reg(bce, *rhs, &gl.vars, assignment[rhs], stack_offsets, T2);

            let dst_size = gl.vars.size(*dst);
            let lhs_size = gl.vars.size(*lhs);

            use BinaryOp as O;
            use LogicMode as M;
            match (
                op,
                dst.mode(),
                SixBitSize::from_vector_size(lhs_size),
                lhs.mode(),
                rhs.mode(),
            ) {
                (O::And, M::TwoValue, Some(_), _, _) => bce.and(rd, rs1, rs2),
                (O::And, M::TwoValue, None, _, _) => bce.heap_tv_and(rd, rs1, rs2, dst_size),
                (O::And, M::FourValue, Some(_), _, _) => bce.fv_and(rd, rs1, rs2),
                (O::And, M::FourValue, None, _, _) => bce.heap_fv_and(rd, rs1, rs2, dst_size),
                (O::Or, M::TwoValue, Some(_), _, _) => bce.or(rd, rs1, rs2),
                (O::Or, M::TwoValue, None, _, _) => bce.heap_tv_or(rd, rs1, rs2, dst_size),
                (O::Or, M::FourValue, Some(_), _, _) => bce.fv_or(rd, rs1, rs2),
                (O::Or, M::FourValue, None, _, _) => bce.heap_fv_or(rd, rs1, rs2, dst_size),
                (O::Xor, M::TwoValue, Some(_), _, _) => bce.xor(rd, rs1, rs2),
                (O::Xor, M::TwoValue, None, _, _) => bce.heap_tv_xor(rd, rs1, rs2, dst_size),
                (O::Xor, M::FourValue, Some(_), _, _) => bce.fv_xor(rd, rs1, rs2),
                (O::Xor, M::FourValue, None, _, _) => bce.heap_fv_xor(rd, rs1, rs2, dst_size),

                (O::Add, M::TwoValue, Some(size), _, _) => bce.add(rd, rs1, rs2, size),
                (O::Add, M::TwoValue, None, _, _) => bce.heap_tv_add(rd, rs1, rs2, dst_size),
                (O::Add, M::FourValue, Some(size), _, _) => bce.fv_add(rd, rs1, rs2, size),
                (O::Add, M::FourValue, None, _, _) => bce.heap_fv_add(rd, rs1, rs2, dst_size),
                (O::Sub, M::TwoValue, Some(size), _, _) => bce.sub(rd, rs1, rs2, size),
                (O::Sub, M::TwoValue, None, _, _) => bce.heap_tv_sub(rd, rs1, rs2, dst_size),
                (O::Sub, M::FourValue, Some(size), _, _) => bce.fv_sub(rd, rs1, rs2, size),
                (O::Sub, M::FourValue, None, _, _) => bce.heap_fv_sub(rd, rs1, rs2, dst_size),
                (O::Multiply, M::TwoValue, Some(size), _, _) => bce.mul(rd, rs1, rs2, size),
                (O::Multiply, M::TwoValue, None, _, _) => bce.heap_tv_mul(rd, rs1, rs2, dst_size),
                (O::Multiply, M::FourValue, Some(size), _, _) => bce.fv_mul(rd, rs1, rs2, size),
                (O::Multiply, M::FourValue, None, _, _) => bce.heap_fv_mul(rd, rs1, rs2, dst_size),
                (O::DivideX, _, Some(size), M::TwoValue, _) => bce.divx(rd, rs1, rs2, size),
                (O::DivideX, _, None, M::TwoValue, _) => bce.heap_tv_divx(rd, rs1, rs2, dst_size),
                (O::DivideX, _, Some(size), M::FourValue, _) => bce.fv_divx(rd, rs1, rs2, size),
                (O::DivideX, _, None, M::FourValue, _) => bce.heap_fv_divx(rd, rs1, rs2, dst_size),
                (O::Divide0, M::TwoValue, Some(size), _, _) => bce.div0(rd, rs1, rs2, size),
                (O::Divide0, M::TwoValue, None, _, _) => bce.heap_tv_div0(rd, rs1, rs2, dst_size),
                (O::Divide0, M::FourValue, Some(size), _, _) => bce.fv_div0(rd, rs1, rs2, size),
                (O::Divide0, M::FourValue, None, _, _) => bce.heap_fv_div0(rd, rs1, rs2, dst_size),
                (O::ModulusX, _, Some(size), M::TwoValue, _) => bce.modx(rd, rs1, rs2, size),
                (O::ModulusX, _, None, M::TwoValue, _) => bce.heap_tv_modx(rd, rs1, rs2, dst_size),
                (O::ModulusX, _, Some(size), M::FourValue, _) => bce.fv_modx(rd, rs1, rs2, size),
                (O::ModulusX, _, None, M::FourValue, _) => bce.heap_fv_modx(rd, rs1, rs2, dst_size),
                (O::Modulus0, M::TwoValue, Some(size), _, _) => bce.mod0(rd, rs1, rs2, size),
                (O::Modulus0, M::TwoValue, None, _, _) => bce.heap_tv_mod0(rd, rs1, rs2, dst_size),
                (O::Modulus0, M::FourValue, Some(size), _, _) => bce.fv_mod0(rd, rs1, rs2, size),
                (O::Modulus0, M::FourValue, None, _, _) => bce.heap_fv_mod0(rd, rs1, rs2, dst_size),
                (O::Power, M::TwoValue, Some(size), _, _) => bce.pow(rd, rs1, rs2, size),
                (O::Power, M::TwoValue, None, _, _) => bce.heap_tv_pow(rd, rs1, rs2, dst_size),
                (O::Power, M::FourValue, Some(size), _, _) => bce.fv_pow(rd, rs1, rs2, size),
                (O::Power, M::FourValue, None, _, _) => bce.heap_fv_pow(rd, rs1, rs2, dst_size),

                (O::UnsignedLessEqual, _, Some(_), M::TwoValue, _) => bce.uleq(rd, rs1, rs2),
                (O::UnsignedLessEqual, _, None, M::TwoValue, _) => {
                    bce.heap_tv_unsigned_leq(rd, rs1, rs2, lhs_size)
                }
                (O::UnsignedLessEqual, _, Some(size), M::FourValue, _) => {
                    bce.fv_uleq(rd, rs1, rs2, size)
                }
                (O::UnsignedLessEqual, _, None, M::FourValue, _) => {
                    bce.heap_fv_unsigned_leq(rd, rs1, rs2, lhs_size)
                }
                (O::LogicalShiftLeft, M::TwoValue, Some(size), _, _) => bce.sll(rd, rs1, rs2, size),
                (O::LogicalShiftLeft, M::TwoValue, None, _, _) => {
                    bce.heap_tv_sll(rd, rs1, rs2, dst_size)
                }
                (O::LogicalShiftLeft, M::FourValue, Some(size), _, _) => {
                    bce.fv_sll(rd, rs1, rs2, size)
                }
                (O::LogicalShiftLeft, M::FourValue, None, _, _) => {
                    bce.heap_fv_sll(rd, rs1, rs2, dst_size)
                }
                (O::LogicalShiftRight, M::TwoValue, Some(_), _, _) => bce.slr(rd, rs1, rs2),
                (O::LogicalShiftRight, M::TwoValue, None, _, _) => {
                    bce.heap_tv_slr(rd, rs1, rs2, dst_size)
                }
                (O::LogicalShiftRight, M::FourValue, Some(size), _, _) => {
                    bce.fv_slr(rd, rs1, rs2, size)
                }
                (O::LogicalShiftRight, M::FourValue, None, _, _) => {
                    bce.heap_fv_slr(rd, rs1, rs2, dst_size)
                }
                (O::ArithmeticShiftRight, M::TwoValue, Some(size), _, _) => {
                    bce.sar(rd, rs1, rs2, size)
                }
                (O::ArithmeticShiftRight, M::TwoValue, None, _, _) => {
                    bce.heap_tv_sar(rd, rs1, rs2, dst_size)
                }
                (O::ArithmeticShiftRight, M::FourValue, Some(size), _, _) => {
                    bce.fv_sar(rd, rs1, rs2, size)
                }
                (O::ArithmeticShiftRight, M::FourValue, None, _, _) => {
                    bce.heap_fv_sar(rd, rs1, rs2, dst_size)
                }

                (O::Concat, ..) => todo!(),
                (O::CopyX, ..) => todo!(),
                (O::CopyZ, ..) => todo!(),
                (O::Min, _, Some(_), M::TwoValue, _) => bce.min(rd, rs1, rs2),
                (O::Min, _, None, M::TwoValue, _) => bce.heap_tv_min(rd, rs1, rs2, lhs_size),
                (O::Min, _, Some(size), M::FourValue, _) => bce.fv_min(rd, rs1, rs2, size),
                (O::Min, _, None, M::FourValue, _) => bce.heap_fv_min(rd, rs1, rs2, lhs_size),
                (O::Max, _, Some(_), M::TwoValue, _) => bce.max(rd, rs1, rs2),
                (O::Max, _, None, M::TwoValue, _) => bce.heap_tv_max(rd, rs1, rs2, lhs_size),
                (O::Max, _, Some(size), M::FourValue, _) => bce.fv_max(rd, rs1, rs2, size),
                (O::Max, _, None, M::FourValue, _) => bce.heap_fv_max(rd, rs1, rs2, lhs_size),

                (O::CaseEquality, _, Some(_), M::TwoValue, _) => bce.ceq(rd, rs1, rs2),
                (O::CaseEquality, _, None, M::TwoValue, _) => {
                    bce.heap_ceq(rd, rs1, rs2, lhs_size.get().div_ceil(64))
                }
                (O::CaseEquality, _, Some(_), M::FourValue, _) => bce.fv_ceq(rd, rs1, rs2),
                (O::CaseEquality, _, None, M::FourValue, _) => {
                    bce.heap_ceq(rd, rs1, rs2, lhs_size.get().div_ceil(64) * 2)
                }

                (O::Posedge, _, _, M::TwoValue, _) => bce.andnot(rd, rs2, rs1, SixBitSize::SCALAR),
                (O::Posedge, _, _, M::FourValue, _) => bce.fv_posedge(rd, rs1, rs2),
                (O::Negedge, _, _, M::TwoValue, _) => bce.andnot(rd, rs1, rs2, SixBitSize::SCALAR),
                (O::Negedge, _, _, M::FourValue, _) => bce.fv_negedge(rd, rs1, rs2),
            }

            store_back(bce, &gl.vars, *dst, dslot, rd);
        }
        I::BinaryImm(dst, op, src, imm) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0);
            let rs = to_reg(bce, *src, &gl.vars, assignment[src], stack_offsets, T1);

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
                            bce.load_u64(
                                T2,
                                imm.zero_extend(VSIZE_64).extract_exact_u64().unwrap(),
                            );
                            bce.and(rd, rs, T2);
                        }
                        Some(imm) => {
                            bce.andi(rd, rs, imm as i16, size);
                        }
                    }
                }
                (O::And, M::TwoValue, None, _, _, _) => todo!(),
                (O::And, M::FourValue, Some(_), _, _, _) => todo!(),
                (O::And, M::FourValue, None, _, _, _) => todo!(),
                (O::Or, M::TwoValue, Some(size), _, _, _) => {
                    match lower_to_n_bits_sign_extend(imm, 10) {
                        None => {
                            bce.load_u64(
                                T2,
                                imm.zero_extend(VSIZE_64).extract_exact_u64().unwrap(),
                            );
                            bce.or(rd, rs, T2);
                        }
                        Some(imm) => {
                            bce.ori(rd, rs, imm as i16, size);
                        }
                    }
                }
                (O::Or, M::TwoValue, None, _, _, _) => todo!(),
                (O::Or, M::FourValue, Some(_), _, _, _) => todo!(),
                (O::Or, M::FourValue, None, _, _, _) => todo!(),
                (O::Xor, M::TwoValue, Some(size), _, _, _) => {
                    match lower_to_n_bits_sign_extend(imm, 10) {
                        None => {
                            bce.load_u64(
                                T2,
                                imm.zero_extend(VSIZE_64).extract_exact_u64().unwrap(),
                            );
                            bce.xor(rd, rs, T2);
                        }
                        Some(imm) => {
                            bce.xori(rd, rs, imm as i16, size);
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
                (O::RevDivideX, ..) => todo!(),
                (O::RevDivide0, ..) => todo!(),
                (O::RevModulusX, ..) => todo!(),
                (O::RevModulus0, ..) => todo!(),
                (O::UnsignedLessEqual, ..) => todo!(),
                (O::UnsignedGreaterEqual, ..) => todo!(),
                (O::ConcatLeft, ..) => todo!(),
                (O::ConcatRight, ..) => todo!(),
                (O::Min, ..) => todo!(),
                (O::Max, ..) => todo!(),
                (O::CaseEquality, _, _, M::TwoValue, _, Some(_)) => {
                    match lower_to_n_bits_sign_extend(imm, 10) {
                        None => {
                            bce.load_u64(
                                T2,
                                imm.zero_extend(VSIZE_64).extract_exact_u64().unwrap(),
                            );
                            bce.ceq(rd, rs, T2);
                        }
                        Some(imm) => {
                            let size = SixBitSize::from_vector_size(src_size).unwrap();
                            bce.ceqi(rd, rs, imm as i16, size);
                        }
                    }
                }
                (O::CaseEquality, _, _, M::TwoValue, _, None) => todo!(),
                (O::CaseEquality, _, _, M::FourValue, _, Some(_)) => {
                    if !imm.contains_special()
                        && let Some(imm) = lower_to_n_bits_sign_extend(imm, 10)
                    {
                        let size = SixBitSize::from_vector_size(src_size).unwrap();
                        bce.fv_ceqi(rd, rs, imm as i16, size)
                    } else {
                        bce.load_bits_into_register(T2, LogicMode::FourValue, &imm);
                        bce.fv_ceq(rd, rs, T2);
                    }
                }
                (O::CaseEquality, _, _, M::FourValue, _, None) => todo!(),
            }

            store_back(bce, &gl.vars, *dst, dslot, rd);
        }
        I::Slice(variable_key, variable_key1, variable_key2) => todo!(),
        I::SliceImm(variable_key, variable_key1, _) => todo!(),
        I::ShiftImm(variable_key, shift_imm_op, variable_key1, _) => todo!(),
        I::Select(dst, cond, truthy, falsy) => {
            let size = gl.vars.size(*dst);
            // @Performance: Better lowering
            match (cond.mode(), SixBitSize::from_vector_size(size)) {
                (LogicMode::TwoValue, None) => todo!(),
                (LogicMode::TwoValue, Some(size)) => todo!(),
                (LogicMode::FourValue, None) => todo!(),
                (LogicMode::FourValue, Some(size)) => todo!(),
            }
        }
        I::Intrinsic(dst, op, items) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0);

            for item in items {
                let reg = to_reg(bce, *item, &gl.vars, assignment[item], stack_offsets, T2);
                bce.push_argument(gl.vars.size(*item), item.mode(), reg);
            }
            let intrinsic_id = bce
                .intrinsics
                .insert_index(IntrinsicOpEqWrap(op.as_ref().clone()));
            let intrinsic_id = match intrinsic_id.try_into().ok().and_then(|v| NonMaxU16::new(v)) {
                None => {
                    bce.load_u64(T2, intrinsic_id as u64);
                    None
                }
                Some(v) => Some(v),
            };
            bce.intrinsic(rd, intrinsic_id);
            store_back(bce, &gl.vars, *dst, dslot, rd);
        }
        I::LastUpdateTime(variable_key, signal_key) => todo!(),
        I::Probe(dst, signal, offset) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0);

            let dst_size = gl.vars.size(*dst);
            let signal_size = gl.signals[*signal].size;
            assert!(dst_size <= signal_size);

            if *offset != 0 {
                todo!()
            }

            if dst_size != signal_size {
                todo!()
            }

            // @Performance. Alias for large probes without an offset.
            let roff = T1;
            load_signal_address(bce, roff, *signal, signals, io_signals);
            match (dst.mode(), SixBitSize::from_vector_size(signal_size)) {
                (LogicMode::TwoValue, None) => {
                    bce.load_heap_aligned(rd, roff, signal_size.get().div_ceil(64) as u16)
                }
                (LogicMode::TwoValue, Some(signal_size)) => {
                    bce.tv_load_aligned(rd, roff, 0, signal_size)
                }
                (LogicMode::FourValue, None) => {
                    bce.load_heap_aligned(rd, roff, signal_size.get().div_ceil(64) as u16 * 2)
                }
                (LogicMode::FourValue, Some(signal_size)) => {
                    bce.fv_load_aligned(rd, roff, 0, signal_size)
                }
            }

            store_back(bce, &gl.vars, *dst, dslot, rd);
        }
        I::ProbeSlice(variable_key, signal_key, variable_key1) => todo!(),
        I::Drive(signal, src, partial) => {
            let rs = to_reg(bce, *src, &gl.vars, assignment[src], stack_offsets, T0);

            let signal_size = gl.signals[*signal].size;
            let src_size = gl.vars.size(*src);

            if partial.is_some() {
                todo!()
            }

            if signal_size != src_size {
                todo!()
            }

            let roff = T1;
            let rpoke = T2;
            load_signal_address(bce, roff, *signal, signals, io_signals);
            match (src.mode(), SixBitSize::from_vector_size(signal_size)) {
                (LogicMode::TwoValue, None) => {
                    if signal_size.get() >= (1u32 << 16) {
                        todo!();
                    }
                    bce.tv_set_heap_aligned(rpoke, rs, roff, Some(signal_size));
                }
                (LogicMode::FourValue, None) => {
                    if signal_size.get() >= (1u32 << 16) {
                        todo!();
                    }
                    bce.fv_set_heap_aligned(rpoke, rs, roff, Some(signal_size));
                }
                (LogicMode::TwoValue, Some(signal_size)) => {
                    bce.tv_set_aligned(rpoke, rs, roff, 0, signal_size)
                }
                (LogicMode::FourValue, Some(signal_size)) => {
                    bce.fv_set_aligned(rpoke, rs, roff, 0, signal_size)
                }
            }

            for index in watch_map.watch_indices(*signal) {
                // @TODO: This should have some register based fallback.
                assert!(index < (1 << 20));
                bce.wake(rpoke, index as u32);
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
        Slot::Heap(offset) => {
            bytecode.load_u64(backup, offset);
            backup
        }
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
            let offset = offset << (kind as u32);

            match lower_to_n_bits_sign_extend(&Bits::new_u64(offset), 10) {
                None => {
                    bytecode.load_u64(backup, offset);
                    bytecode.add(backup, SP, backup, SixBitSize::N64);
                }
                Some(imm10) => {
                    bytecode.addi(backup, SP, imm10 as i16, SixBitSize::N64);
                }
            }
            let size = vars.size(var);
            if let Some(size) = SixBitSize::from_vector_size(size) {
                match var.mode() {
                    LogicMode::TwoValue => bytecode.tv_load_aligned(backup, backup, 0, size),
                    LogicMode::FourValue => bytecode.fv_load_aligned(backup, backup, 0, size),
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

fn store_back(
    bytecode: &mut BytecodeEncoder,
    vars: &VariableMap,
    var: VariableKey,
    slot: Slot,
    value: Reg,
) {
    match slot {
        Slot::Heap(..) => unreachable!(),
        Slot::Stack(..) => {
            if vars.size(var) <= VSIZE_64 {
                todo!()
            }
        }
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
