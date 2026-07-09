use vogls_codegen::lsra::{Slot, StackOffsets, StackTracker};
use vogls_codegen::{HeapAlignment, HeapBuilder, HeapRef, insert_bb_phis};
use vogls_ir::watchers::WatchMap;
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, ContextFormat, DisplayContext,
    GlobalContext, Instruction, LogicMode, ProcessKey, ResizeOp, ShiftImmOp, SignalKey, UnaryOp,
    VSIZE_64, VariableKey, VariableMap, VectorSize,
};
use vogls_runtime::RtSignalKey;
use vogls_utils::{NonMaxU16, VgHashMap, VgHashSet};

use crate::bytecode::{
    Branch, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, InlineAddrOffset, InlineIndex,
    InlineNBitSize, InstructionPtr, IntrinsicOpEqWrap, Jump, Reg, Schedule, SignedImmediate,
    SixBitSize,
};

enum JumpKind {
    Jump,
    Branch(Reg),
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
    lupdt_indexes: &VgHashMap<RtSignalKey, u64>,
    bytecode: &mut BytecodeEncoder,
) {
    const PRINT: bool = false;

    let process = &gl.processes[process];

    let mut bb_stack = Vec::new();
    let mut bb_seen = VgHashSet::<BasicBlockKey>::default();
    // let mut bb_phis = VgHashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::default();

    let mut assignment = VgHashMap::default();

    let mut post_order = Vec::<BasicBlockKey>::new();
    let mut bb_offsets = VgHashMap::<BasicBlockKey, usize>::default();
    let mut jump_targets = Vec::<(usize, BasicBlockKey, JumpKind)>::new();

    schedule.push_active(InstructionPtr(bytecode.data.len() as u64));
    for tr in &process.regions {
        bb_seen.clear();
        assignment.clear();
        post_order.clear();

        vogls_ir::orders::post_order_keys(
            tr.entry(),
            &gl.bbs,
            &mut bb_seen,
            &mut bb_stack,
            &mut post_order,
        );

        vogls_codegen::lsra::linear_scan_register_allocation(
            &post_order,
            &gl.vars,
            &gl.bbs,
            heap,
            &mut assignment,
            stack_tracker,
            10,
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
                    heap,
                    &stack_offsets,
                    watch_map,
                    signals,
                    io_signals,
                    lupdt_indexes,
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
                        bytecode.load_u64(T0_SPC, time);
                        bytecode.wait(T0_SPC);
                    }
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.panic();
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
                        T0_SPC,
                        false,
                    );
                    bytecode.wait(rtime);
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.panic();
                }
                T::WaitRegion(target, region) => {
                    bytecode.wait_region(*region);
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.panic();
                }
                T::Watch(target, _) => {
                    let index = watch_map.get_watch_index(bb_key);
                    bytecode.start_listen(index as u32);
                    bytecode.next_event();
                    listeners.set_ptr(index, bytecode.current_ptr());
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.panic();
                }
                T::Jump(target) => {
                    jump_targets.push((bytecode.data.len(), *target, JumpKind::Jump));
                    bytecode.panic();
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
                        T0_SPC,
                        false,
                    );
                    jump_targets.push((bytecode.data.len(), *truthy, JumpKind::Branch(rcond)));
                    bytecode.panic();
                    jump_targets.push((bytecode.data.len(), *falsy, JumpKind::Jump));
                    bytecode.panic();
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
        let mut imm = imm as i64;
        if target_offset < offset {
            imm = -imm;
        }
        imm -= 1;
        bytecode.data[offset] = match kind {
            JumpKind::Jump => {
                let imm = SignedImmediate::new(imm.into()).unwrap();
                Jump(imm).encode()
            }
            JumpKind::Branch(rcond) => {
                let imm = SignedImmediate::new(imm.into()).unwrap();
                Branch { rcond, imm }.encode()
            }
        };
    }
}

const T0_SPC: Reg = Reg::X10;
const T0_VAL: Reg = Reg::X11;
const T1_SPC: Reg = Reg::X12;
const T1_VAL: Reg = Reg::X13;
const T2_SPC: Reg = Reg::X14;
const T2_VAL: Reg = Reg::X15;

fn lower_instruction(
    gl: &GlobalContext,
    bce: &mut BytecodeEncoder,
    assignment: &VgHashMap<VariableKey, Slot>,
    heap_builder: &mut HeapBuilder,
    stack_offsets: &StackOffsets,
    watch_map: &WatchMap,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    lupdt_indexes: &VgHashMap<RtSignalKey, u64>,
    instr: &Instruction,
) {
    use Instruction as I;
    match instr {
        I::Constant(dst, value) => {
            // @NOTE:
            // Constants with size greater than 64 are stored on the heap and referenced on there.
            if value.size() <= VSIZE_64 {
                let dslot = assignment[dst];
                let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
                bce.load_bits_into_register(rd, dst.mode(), value);
                store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
            }
        }
        I::Unary(dst, op, src) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T1_SPC,
                false,
            );

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
                (O::Neg, M::TwoValue, _, None) => bce.heap_tv_neg(rd, rs, src_size),
                (O::Neg, M::FourValue, _, Some(_)) => bce.fv_not(rd, rs),
                (O::Neg, M::FourValue, _, None) => bce.heap_fv_neg(rd, rs, src_size),
                (O::ReduceOr, M::TwoValue, _, Some(src_size)) => {
                    bce.cnei(rd, rs, SignedImmediate::ZERO, src_size)
                }
                (O::ReduceOr, M::TwoValue, _, None) => bce.heap_tv_reduce_or(rd, rs, src_size),
                (O::ReduceOr, M::FourValue, _, Some(src_size)) => {
                    bce.fv_reduce_or(rd, rs, src_size)
                }
                (O::ReduceOr, M::FourValue, _, None) => bce.heap_fv_reduce_or(rd, rs, src_size),
                (O::ReduceAnd, M::TwoValue, _, Some(src_size)) => {
                    bce.ceqi(rd, rs, SignedImmediate::MINUS_ONE, src_size)
                }
                (O::ReduceAnd, M::TwoValue, _, None) => bce.heap_tv_reduce_and(rd, rs, src_size),
                (O::ReduceAnd, M::FourValue, _, Some(src_size)) => {
                    bce.fv_reduce_and(rd, rs, src_size)
                }
                (O::ReduceAnd, M::FourValue, _, None) => bce.heap_fv_reduce_and(rd, rs, src_size),
                (O::ReduceXor, M::TwoValue, _, Some(_)) => {
                    bce.count_ones(rd, rs);
                    bce.truncate(rd, rd, SixBitSize::SCALAR);
                }
                (O::ReduceXor, M::TwoValue, _, None) => bce.heap_tv_reduce_xor(rd, rs, src_size),
                (O::ReduceXor, M::FourValue, _, Some(src_size)) => {
                    bce.fv_reduce_xor(rd, rs, src_size)
                }
                (O::ReduceXor, M::FourValue, _, None) => bce.heap_fv_reduce_xor(rd, rs, src_size),
                (O::LeadingZeros, M::TwoValue, _, Some(_)) => todo!(),
                (O::LeadingZeros, M::TwoValue, _, None) => todo!(),
                (O::LeadingZeros, M::FourValue, _, Some(_)) => todo!(),
                (O::LeadingZeros, M::FourValue, _, None) => todo!(),
                (O::TvToFv, _, _, Some(src_size)) => {
                    // @Performance: better lowering.
                    let (spc, val) = rd.to_spc_and_val();
                    bce.copy(val, rs);
                    bce.load_u64(spc, src_size.mask(u64::MAX));
                }
                (O::TvToFv, _, _, None) => {
                    // @Performance: better lowering.
                    let num_words = src_size.get().div_ceil(64) as u64;
                    let val = T2_SPC;
                    match SignedImmediate::new_from_u64(num_words * 64) {
                        None => {
                            bce.load_u64(val, num_words);
                            bce.add(val, rd, val, SixBitSize::N64);
                        }
                        Some(imm) => bce.addi(val, rd, imm, SixBitSize::N64),
                    }
                    bce.heap_tv_copy(val, rs, src_size);
                    // ORNOT with self is a fill 1's
                    bce.heap_tv_ornot(rd, rs, rs, src_size);
                }
                (O::FvToTv, _, _, Some(_)) => {
                    let (spc, val) = rs.to_spc_and_val();
                    bce.and(rd, spc, val)
                }
                (O::FvToTv, _, _, None) => {
                    let num_words = src_size.get().div_ceil(64) as u64;
                    let spc = rs;
                    let val = T2_SPC;
                    match SignedImmediate::new_from_u64(num_words * 64) {
                        None => {
                            bce.load_u64(val, num_words);
                            bce.add(val, rs, val, SixBitSize::N64);
                        }
                        Some(imm) => bce.addi(val, rs, imm, SixBitSize::N64),
                    }
                    bce.heap_tv_and(rd, spc, val, src_size);
                }
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::Resize(dst, op, src) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T1_SPC,
                false,
            );

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
                (O::Truncate, M::TwoValue, Some(dst_size), None) => {
                    bce.load_unaligned(rd, rs, InlineAddrOffset::ZERO, dst_size);
                }
                (O::Truncate, M::TwoValue, None, None) => {
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.load_u64(T2_SPC, src_size.get() as u64);
                    bce.heapheap_tv_truncate(rd, rs, dst_size, T2_SPC);
                }
                (O::Truncate, M::FourValue, Some(dst_size), Some(_)) => {
                    // @Performance: One instruction maybe?
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    let (rsspc, rsval) = rs.to_spc_and_val();
                    bce.truncate(rdspc, rsspc, dst_size);
                    bce.truncate(rdval, rsval, dst_size);
                }
                (O::Truncate, M::FourValue, Some(dst_size), None) => {
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    bce.load_unaligned(rdspc, rs, InlineAddrOffset::ZERO, dst_size);
                    let (addr, offset) = InlineAddrOffset::new(
                        src_size.get().next_power_of_two() as i64,
                        bce,
                        rs,
                        T1_VAL,
                    );
                    bce.load_unaligned(rdval, addr, offset, dst_size);
                }
                (O::Truncate, M::FourValue, None, None) => {
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.load_u64(T2_SPC, src_size.get() as u64);
                    bce.heapheap_fv_truncate(rd, rs, dst_size, T2_SPC);
                }
                (O::ZeroExtend, M::TwoValue, Some(_), Some(_)) => {
                    bce.copy(rd, rs);
                }
                (O::ZeroExtend, M::FourValue, Some(dst_size), Some(src_size)) => {
                    // @Performance: One instruction maybe?
                    let (rsspc, rsval) = rs.to_spc_and_val();
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    let mask = dst_size.mask(u64::MAX) ^ src_size.mask(u64::MAX);
                    match SignedImmediate::new_from_u64(mask) {
                        None => {
                            bce.load_u64(T2_SPC, mask);
                            bce.or(rdspc, rsspc, T2_SPC);
                        }
                        Some(imm) => {
                            bce.ori(rdspc, rsspc, imm, dst_size);
                        }
                    }
                    bce.copy(rdval, rsval);
                }
                (O::ZeroExtend, M::TwoValue, None, Some(src_size)) => {
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.heapreg_tv_zero_extend(rd, rs, dst_size, src_size);
                }
                (O::ZeroExtend, M::FourValue, None, Some(src_size)) => {
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.heapreg_fv_zero_extend(rd, rs, dst_size, src_size);
                }

                (O::ZeroExtend, M::TwoValue, None, None) => {
                    let src_size = InlineNBitSize::new(src_size, bce);
                    bce.load_u64(T1_VAL, dst_size.get().into());
                    bce.heapheap_tv_zero_extend(rd, rs, T1_VAL, src_size);
                }
                (O::ZeroExtend, M::FourValue, None, None) => {
                    let src_size = InlineNBitSize::new(src_size, bce);
                    bce.load_u64(T1_VAL, dst_size.get().into());
                    bce.heapheap_fv_zero_extend(rd, rs, T1_VAL, src_size);
                }

                (O::SignExtend, M::TwoValue, Some(dst_size), Some(src_size)) => {
                    bce.sign_extend(rd, rs, dst_size, src_size);
                }
                (O::SignExtend, M::FourValue, Some(dst_size), Some(src_size)) => {
                    // @Performance: One instruction maybe?
                    let (rsspc, rsval) = rs.to_spc_and_val();
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    bce.sign_extend(rdspc, rsspc, dst_size, src_size);
                    bce.sign_extend(rdval, rsval, dst_size, src_size);
                }
                (O::SignExtend, M::TwoValue, None, Some(src_size)) => {
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.heapreg_tv_sign_extend(rd, rs, dst_size, src_size);
                }
                (O::SignExtend, M::FourValue, None, Some(src_size)) => {
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.heapreg_fv_sign_extend(rd, rs, dst_size, src_size);
                }

                (O::SignExtend, M::TwoValue, None, None) => {
                    let src_size = InlineNBitSize::new(src_size, bce);
                    bce.load_u64(T1_VAL, dst_size.get().into());
                    bce.heapheap_tv_sign_extend(rd, rs, T1_VAL, src_size);
                }
                (O::SignExtend, M::FourValue, None, None) => {
                    let src_size = InlineNBitSize::new(src_size, bce);
                    bce.load_u64(T1_VAL, dst_size.get().into());
                    bce.heapheap_fv_sign_extend(rd, rs, T1_VAL, src_size);
                }

                (O::Truncate, _, None, Some(_)) => unreachable!(),
                (O::ZeroExtend, _, Some(_), None) => unreachable!(),
                (O::SignExtend, _, Some(_), None) => unreachable!(),
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::Binary(dst, op, lhs, rhs) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let rs1 = to_reg(
                bce,
                *lhs,
                &gl.vars,
                assignment[lhs],
                stack_offsets,
                T1_SPC,
                false,
            );
            let rs2 = to_reg(
                bce,
                *rhs,
                &gl.vars,
                assignment[rhs],
                stack_offsets,
                T2_SPC,
                false,
            );

            let dst_size = gl.vars.size(*dst);
            let lhs_size = gl.vars.size(*lhs);
            let rhs_size = gl.vars.size(*rhs);

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

                (O::Concat, M::TwoValue, _, _, _)
                    if SixBitSize::from_vector_size(dst_size).is_some() =>
                {
                    bce.lsor(rd, rs1, rs2, SixBitSize::new_masked(rhs_size.get()))
                }
                (O::Concat, M::TwoValue, _, _, _) => {
                    tv_concat_heap(bce, rd, rs1, rs2, lhs_size, rhs_size, T0_VAL);
                }
                (O::Concat, M::FourValue, _, _, _)
                    if SixBitSize::from_vector_size(dst_size).is_some() =>
                {
                    bce.fv_lsor(rd, rs1, rs2, SixBitSize::new_masked(rhs_size.get()))
                }
                (O::Concat, M::FourValue, _, _, _) => {
                    fv_concat_heap(bce, rd, rs1, rs2, lhs_size, rhs_size, T0_VAL);
                }
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

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::BinaryImm(dst, op, src, imm) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T1_SPC,
                false,
            );

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
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.and(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.andi(rd, rs, imm, size),
                    }
                }
                (O::And, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_and(rd, rs, T2_SPC, src_size);
                }
                (O::And, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_and(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.fv_andi(rd, rs, imm, size),
                    }
                }
                (O::And, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_and(rd, rs, T2_SPC, src_size);
                }
                (O::Or, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.or(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.ori(rd, rs, imm, size),
                    }
                }
                (O::Or, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_or(rd, rs, T2_SPC, src_size);
                }
                (O::Or, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_or(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.fv_ori(rd, rs, imm, size),
                    }
                }
                (O::Or, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_or(rd, rs, T2_SPC, src_size);
                }
                (O::Xor, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.xor(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.xori(rd, rs, imm, size),
                    }
                }
                (O::Xor, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_xor(rd, rs, T2_SPC, src_size);
                }
                (O::Xor, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_xor(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.fv_xori(rd, rs, imm, size),
                    }
                }
                (O::Xor, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_xor(rd, rs, T2_SPC, src_size);
                }
                (O::Add, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.add(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.addi(rd, rs, imm, size),
                    }
                }
                (O::Add, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_add(rd, rs, T2_SPC, src_size);
                }
                (O::Add, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_add(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.fv_addi(rd, rs, imm, size),
                    }
                }
                (O::Add, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_add(rd, rs, T2_SPC, src_size);
                }
                (O::Sub, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.sub(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.subi(rd, rs, imm, size),
                    }
                }
                (O::Sub, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_sub(rd, rs, T2_SPC, src_size);
                }
                (O::Sub, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_sub(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.fv_subi(rd, rs, imm, size),
                    }
                }
                (O::Sub, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_sub(rd, rs, T2_SPC, src_size);
                }
                (O::Multiply, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.mul(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.muli(rd, rs, imm, size),
                    }
                }
                (O::Multiply, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_mul(rd, rs, T2_SPC, src_size);
                }
                (O::Multiply, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_mul(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.fv_muli(rd, rs, imm, size),
                    }
                }
                (O::Multiply, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_mul(rd, rs, T2_SPC, src_size);
                }
                (O::Power, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.pow(rd, rs, T2_SPC, size);
                }
                (O::Power, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_pow(rd, rs, T2_SPC, src_size);
                }
                (O::Power, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_pow(rd, rs, T2_SPC, size);
                }
                (O::Power, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_pow(rd, rs, T2_SPC, src_size);
                }
                (O::Divide, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.div0(rd, rs, T2_SPC, size);
                }
                (O::Divide, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_div0(rd, rs, T2_SPC, src_size);
                }
                (O::Divide, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_div0(rd, rs, T2_SPC, size);
                }
                (O::Divide, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_div0(rd, rs, T2_SPC, src_size);
                }
                (O::Modulus, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.mod0(rd, rs, T2_SPC, size);
                }
                (O::Modulus, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_mod0(rd, rs, T2_SPC, src_size);
                }
                (O::Modulus, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_mod0(rd, rs, T2_SPC, size);
                }
                (O::Modulus, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_mod0(rd, rs, T2_SPC, src_size);
                }
                (O::RevSub, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.sub(rd, T2_SPC, rs, size);
                        }
                        Some(imm) => bce.revsubi(rd, rs, imm, size),
                    }
                }
                (O::RevSub, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_sub(rd, T2_SPC, rs, src_size);
                }
                (O::RevSub, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_sub(rd, T2_SPC, rs, size);
                        }
                        Some(imm) => bce.fv_revsubi(rd, rs, imm, size),
                    }
                }
                (O::RevSub, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_sub(rd, T2_SPC, rs, src_size);
                }
                (O::RevPower, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.pow(rd, T2_SPC, rs, size);
                }
                (O::RevPower, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_pow(rd, T2_SPC, rs, src_size);
                }
                (O::RevPower, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_pow(rd, T2_SPC, rs, size);
                }
                (O::RevPower, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_pow(rd, T2_SPC, rs, src_size);
                }
                (O::RevDivideX, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.divx(rd, T2_SPC, rs, size);
                }
                (O::RevDivideX, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_divx(rd, T2_SPC, rs, src_size);
                }
                (O::RevDivideX, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_divx(rd, T2_SPC, rs, size);
                }
                (O::RevDivideX, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_divx(rd, T2_SPC, rs, src_size);
                }
                (O::RevDivide0, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.div0(rd, T2_SPC, rs, size);
                }
                (O::RevDivide0, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_div0(rd, T2_SPC, rs, src_size);
                }
                (O::RevDivide0, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_div0(rd, T2_SPC, rs, size);
                }
                (O::RevDivide0, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_div0(rd, T2_SPC, rs, src_size);
                }
                (O::RevModulusX, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.modx(rd, T2_SPC, rs, size);
                }
                (O::RevModulusX, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_modx(rd, T2_SPC, rs, src_size);
                }
                (O::RevModulusX, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_modx(rd, T2_SPC, rs, size);
                }
                (O::RevModulusX, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_modx(rd, T2_SPC, rs, src_size);
                }
                (O::RevModulus0, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.mod0(rd, T2_SPC, rs, size);
                }
                (O::RevModulus0, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_mod0(rd, T2_SPC, rs, src_size);
                }
                (O::RevModulus0, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_mod0(rd, T2_SPC, rs, size);
                }
                (O::RevModulus0, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_mod0(rd, T2_SPC, rs, src_size);
                }
                (O::UnsignedLessEqual, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.uleq(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.uleqi(rd, rs, imm, size),
                    }
                }
                (O::UnsignedLessEqual, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_unsigned_leq(rd, rs, T2_SPC, src_size);
                }
                (O::UnsignedLessEqual, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_uleq(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.fv_uleqi(rd, rs, imm, size),
                    }
                }
                (O::UnsignedLessEqual, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_unsigned_leq(rd, rs, T2_SPC, src_size);
                }
                (O::UnsignedGreaterEqual, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.ugt(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.ugti(rd, rs, imm, size),
                    }
                }
                (O::UnsignedGreaterEqual, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_unsigned_gt(rd, rs, T2_SPC, src_size);
                }
                (O::UnsignedGreaterEqual, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_ugt(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.fv_ugti(rd, rs, imm, size),
                    }
                }
                (O::UnsignedGreaterEqual, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_unsigned_gt(rd, rs, T2_SPC, src_size);
                }
                (O::ConcatLeft, M::TwoValue, Some(_), _, _, _) => {
                    // @Performance. There is likely space here for a left_shift_or_immediate
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.lsor(
                        rd,
                        rs,
                        T2_SPC,
                        SixBitSize::from_vector_size(imm.size()).unwrap(),
                    );
                }
                (O::ConcatLeft, M::TwoValue, None, _, _, _) => {
                    match SixBitSize::from_vector_size(imm.size()) {
                        None => {
                            let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                            bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                        }
                        Some(_) => bce.load_bits_into_register(T2_SPC, M::TwoValue, imm),
                    }
                    tv_concat_heap(bce, rd, T2_SPC, rs, imm.size(), src_size, T0_VAL);
                }
                (O::ConcatLeft, M::FourValue, Some(_), _, _, _) => {
                    // @Performance. There is likely space here for a fv_left_shift_or_immediate
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_lsor(
                        rd,
                        rs,
                        T2_SPC,
                        SixBitSize::from_vector_size(imm.size()).unwrap(),
                    );
                }
                (O::ConcatLeft, M::FourValue, None, _, _, _) => {
                    match SixBitSize::from_vector_size(imm.size()) {
                        None => {
                            let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                            bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                        }
                        Some(_) => bce.load_bits_into_register(T2_SPC, M::FourValue, imm),
                    }
                    fv_concat_heap(bce, rd, T2_SPC, rs, imm.size(), src_size, T0_VAL);
                }
                (O::ConcatRight, M::TwoValue, Some(_), _, _, _) => {
                    // @Performance. There is likely space here for a left_shift_or_immediate
                    bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                    bce.lsor(
                        rd,
                        T2_SPC,
                        rs,
                        SixBitSize::from_vector_size(src_size).unwrap(),
                    );
                }
                (O::ConcatRight, M::TwoValue, None, _, _, _) => {
                    match SixBitSize::from_vector_size(imm.size()) {
                        None => {
                            let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                            bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                        }
                        Some(_) => bce.load_bits_into_register(T2_SPC, M::TwoValue, imm),
                    }
                    tv_concat_heap(bce, rd, rs, T2_SPC, src_size, imm.size(), T0_VAL);
                }
                (O::ConcatRight, M::FourValue, Some(_), _, _, _) => {
                    // @Performance. There is likely space here for a fv_left_shift_or_immediate
                    bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                    bce.fv_lsor(
                        rd,
                        T2_SPC,
                        rs,
                        SixBitSize::from_vector_size(src_size).unwrap(),
                    );
                }
                (O::ConcatRight, M::FourValue, None, _, _, _) => {
                    match SixBitSize::from_vector_size(imm.size()) {
                        None => {
                            let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                            bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                        }
                        Some(_) => bce.load_bits_into_register(T2_SPC, M::FourValue, imm),
                    }
                    fv_concat_heap(bce, rd, rs, T2_SPC, src_size, imm.size(), T0_VAL);
                }
                (O::Min, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.min(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.mini(rd, rs, imm, size),
                    }
                }
                (O::Min, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_min(rd, rs, T2_SPC, src_size);
                }
                (O::Min, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_min(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.fv_mini(rd, rs, imm, size),
                    }
                }
                (O::Min, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_min(rd, rs, T2_SPC, src_size);
                }
                (O::Max, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.max(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.maxi(rd, rs, imm, size),
                    }
                }
                (O::Max, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_tv_max(rd, rs, T2_SPC, src_size);
                }
                (O::Max, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, imm);
                            bce.fv_max(rd, rs, T2_SPC, size);
                        }
                        Some(imm) => bce.fv_maxi(rd, rs, imm, size),
                    }
                }
                (O::Max, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_fv_max(rd, rs, T2_SPC, src_size);
                }
                (O::CaseEquality, _, _, M::TwoValue, _, Some(src_size)) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::TwoValue, imm);
                            bce.ceq(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.ceqi(rd, rs, imm, src_size),
                    }
                }
                (O::CaseEquality, _, _, M::TwoValue, _, None) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_ceq(rd, rs, T2_SPC, src_size.get().div_ceil(64));
                }
                (O::CaseEquality, _, _, M::FourValue, _, Some(src_size)) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T2_SPC, M::FourValue, &imm);
                            bce.fv_ceq(rd, rs, T2_SPC);
                        }
                        Some(imm) => bce.fv_ceqi(rd, rs, imm, src_size),
                    }
                }
                (O::CaseEquality, _, _, M::FourValue, _, None) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T2_SPC, imm.offset.bit_offset as u64);
                    bce.heap_ceq(rd, rs, T2_SPC, src_size.get().div_ceil(64) * 2);
                }
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::Slice(dst, src, imm) => {
            // @Incorrect: This ignores that out-of-bounds slices should give X.
            let dst_size = gl.vars.size(*dst);
            let src_size = gl.vars.size(*src);

            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T1_SPC,
                false,
            );
            let mut rimm = to_reg(
                bce,
                *imm,
                &gl.vars,
                assignment[imm],
                stack_offsets,
                T2_SPC,
                false,
            );

            let mut jump_offset: Option<usize> = None;
            use LogicMode as M;
            match imm.mode() {
                M::TwoValue => {}
                M::FourValue => {
                    assert_eq!(dst.mode(), LogicMode::FourValue);
                    let imm_size = SixBitSize::from_vector_size(gl.vars.size(*imm)).unwrap();
                    let (rimmspc, rimmval) = rimm.to_spc_and_val();
                    bce.ceqi(T2_SPC, rimmspc, SignedImmediate::MINUS_ONE, imm_size);

                    let branch_offset = bce.data.len();
                    bce.panic();

                    let (rdspc, rdval) = rd.to_spc_and_val();
                    bce.load_u64(rdspc, 0);
                    bce.load_u64(rdval, 0);

                    jump_offset = Some(bce.data.len());
                    bce.panic();

                    bce.data[branch_offset] = Branch {
                        rcond: T2_SPC,
                        imm: SignedImmediate::new((bce.data.len() - branch_offset) as i64 - 1)
                            .unwrap(),
                    }
                    .encode();
                    rimm = rimmval;
                }
            }

            match (
                src.mode(),
                SixBitSize::from_vector_size(dst_size),
                SixBitSize::from_vector_size(src_size),
            ) {
                (M::TwoValue, Some(dst_size), Some(_)) => {
                    bce.slr(rd, rs, rimm);
                    bce.truncate(rd, rd, dst_size);
                }
                (M::FourValue, Some(dst_size), Some(_)) => {
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    let (rsspc, rsval) = rs.to_spc_and_val();
                    bce.slr(rdspc, rsspc, rimm);
                    bce.slr(rdval, rsval, rimm);
                    bce.truncate(rdspc, rdspc, dst_size);
                    bce.truncate(rdval, rdval, dst_size);
                }
                (M::TwoValue, Some(dst_size), None) => {
                    // @Incorrect. This can reach out-of-bounds.
                    bce.add(T1_VAL, rs, rimm, SixBitSize::N64);
                    bce.load_unaligned(rd, T1_VAL, InlineAddrOffset::ZERO, dst_size);
                }
                (M::FourValue, Some(dst_size), None) => {
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    // @Incorrect. This can reach out-of-bounds.
                    let src_alignment = HeapAlignment::new(src_size, LogicMode::FourValue);
                    bce.add(T1_VAL, rs, rimm, SixBitSize::N64);
                    bce.load_unaligned(rdspc, T1_VAL, InlineAddrOffset::ZERO, dst_size);
                    let (addr, offset) = InlineAddrOffset::new(
                        src_alignment.next_aligned(src_size.get() as u64) as i64,
                        bce,
                        T1_VAL,
                        T1_VAL,
                    );
                    bce.load_unaligned(rdval, addr, offset, dst_size);
                }
                (M::TwoValue, None, None) => {
                    // @Incorrect. This can reach out-of-bounds.
                    bce.add(T1_VAL, rs, rimm, SixBitSize::N64);
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.load_heap_unaligned(rd, T1_VAL, InlineAddrOffset::ZERO, dst_size);
                }
                (M::FourValue, None, None) => {
                    // @Incorrect. This can reach out-of-bounds.
                    bce.add(T1_VAL, rs, rimm, SixBitSize::N64);
                    let dst_inline_size = InlineNBitSize::new(dst_size, bce);
                    bce.load_heap_unaligned(rd, T1_VAL, InlineAddrOffset::ZERO, dst_inline_size);
                    let (addr, offset) = InlineAddrOffset::new(
                        HeapAlignment::B64.next_aligned(src_size.get() as u64) as i64,
                        bce,
                        T1_VAL,
                        T1_VAL,
                    );
                    match SignedImmediate::new_from_u64(dst_size.get() as u64) {
                        None => {
                            bce.load_u64(T0_VAL, dst_size.get() as u64);
                            bce.add(T0_VAL, rd, T0_VAL, SixBitSize::N64);
                        }
                        Some(imm) => bce.addi(T0_VAL, rd, imm, SixBitSize::N64),
                    }
                    bce.load_heap_unaligned(T0_VAL, addr, offset, dst_inline_size);
                }
                (_, None, Some(_)) => unreachable!(),
            }

            if let Some(jump_offset) = jump_offset {
                bce.data[jump_offset] =
                    Jump(SignedImmediate::new((bce.data.len() - jump_offset) as i64 - 1).unwrap())
                        .encode();
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::SliceImm(dst, src, offset) => {
            let dst_size = gl.vars.size(*dst);
            let src_size = gl.vars.size(*src);

            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T1_SPC,
                false,
            );

            use LogicMode as M;
            match (
                src.mode(),
                SixBitSize::from_vector_size(dst_size),
                SixBitSize::from_vector_size(src_size),
            ) {
                (M::TwoValue, Some(dst_size), Some(_)) => {
                    match SignedImmediate::new_from_u64(*offset as u64) {
                        None => bce.load_u64(rd, 0),
                        Some(shift) => bce.slri(rd, rs, shift, dst_size),
                    }
                }
                (M::FourValue, Some(dst_size), Some(_)) => {
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    let (rsspc, rsval) = rs.to_spc_and_val();
                    match SignedImmediate::new_from_u64(*offset as u64) {
                        None => {
                            bce.load_u64(rdspc, 0);
                            bce.load_u64(rdval, 0);
                        }
                        Some(shift) => {
                            // @Incorrect. out-of-bounds reads should return one.
                            bce.slri(rdspc, rsspc, shift, dst_size);
                            bce.slri(rdval, rsval, shift, dst_size);
                        }
                    }
                }
                (M::TwoValue, Some(dst_size), None) => {
                    // @Incorrect. Deal with out-of-bounds reads.
                    let (addr, offset) = InlineAddrOffset::new(i64::from(*offset), bce, rs, T1_VAL);
                    bce.load_unaligned(rd, addr, offset, dst_size);
                }
                (M::FourValue, Some(dst_size), None) => {
                    // @Incorrect. Deal with out-of-bounds reads.
                    let (rdspc, rdval) = rd.to_spc_and_val();

                    let (addr, spc_offset) =
                        InlineAddrOffset::new(i64::from(*offset), bce, rs, T1_VAL);
                    bce.load_unaligned(rdspc, addr, spc_offset, dst_size);

                    let num_words = (src_size.get() as u64).next_multiple_of(64);
                    let (addr, val_offset) = InlineAddrOffset::new(
                        (*offset as u64).wrapping_add(num_words) as i64,
                        bce,
                        rs,
                        T1_VAL,
                    );
                    bce.load_unaligned(rdval, addr, val_offset, dst_size);
                }
                (M::TwoValue, None, None) => {
                    // @Incorrect. Deal with out-of-bounds reads.
                    let (addr, offset) = InlineAddrOffset::new(i64::from(*offset), bce, rs, T1_VAL);
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.load_heap_unaligned(rd, addr, offset, dst_size);
                }
                (M::FourValue, None, None) => {
                    // @Incorrect. Deal with out-of-bounds reads.
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    let dst_size = InlineNBitSize::new(dst_size, bce);

                    let (addr, spc_offset) =
                        InlineAddrOffset::new(i64::from(*offset), bce, rs, T1_VAL);
                    bce.load_heap_unaligned(rdspc, addr, spc_offset, dst_size);

                    let num_words = (src_size.get() as u64).next_multiple_of(64);
                    let (addr, val_offset) = InlineAddrOffset::new(
                        (*offset as u64).wrapping_add(num_words) as i64,
                        bce,
                        rs,
                        T1_VAL,
                    );
                    bce.load_heap_unaligned(rdval, addr, val_offset, dst_size);
                }
                (M::TwoValue, None, Some(_)) => unreachable!(),
                (M::FourValue, None, Some(_)) => unreachable!(),
            }
            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::ShiftImm(dst, op, src, imm) => {
            let size = gl.vars.size(*dst);
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T1_SPC,
                false,
            );

            use LogicMode as M;
            use ShiftImmOp as O;
            // @Incorrect: Deal with overflowing shifts
            match (op, SixBitSize::from_vector_size(size), src.mode()) {
                (O::LogicalShiftLeft, Some(size), M::TwoValue) => {
                    let imm = SignedImmediate::new_from_u64(*imm as u64).unwrap();
                    bce.slli(rd, rs, imm, size);
                }
                (O::LogicalShiftLeft, Some(size), M::FourValue) => {
                    let imm = SignedImmediate::new_from_u64(*imm as u64).unwrap();
                    bce.fv_slli(rd, rs, imm, size);
                }
                (O::LogicalShiftLeft, None, M::TwoValue) => {
                    bce.load_u64(T2_SPC, *imm as u64);
                    bce.heap_tv_sll(rd, rs, T2_SPC, size);
                }
                (O::LogicalShiftLeft, None, M::FourValue) => {
                    bce.load_u64(T2_SPC, *imm as u64);
                    bce.heap_fv_sll(rd, rs, T2_SPC, size);
                }

                (O::LogicalShiftRight, Some(size), M::TwoValue) => {
                    let imm = SignedImmediate::new_from_u64(*imm as u64).unwrap();
                    bce.slri(rd, rs, imm, size);
                }
                (O::LogicalShiftRight, Some(size), M::FourValue) => {
                    let imm = SignedImmediate::new_from_u64(*imm as u64).unwrap();
                    bce.fv_slri(rd, rs, imm, size);
                }
                (O::LogicalShiftRight, None, M::TwoValue) => {
                    bce.load_u64(T2_SPC, *imm as u64);
                    bce.heap_tv_slr(rd, rs, T2_SPC, size);
                }
                (O::LogicalShiftRight, None, M::FourValue) => {
                    bce.load_u64(T2_SPC, *imm as u64);
                    bce.heap_fv_slr(rd, rs, T2_SPC, size);
                }

                (O::ArithmeticShiftRight, Some(size), M::TwoValue) => {
                    let imm = SignedImmediate::new_from_u64(*imm as u64).unwrap();
                    bce.sari(rd, rs, imm, size);
                }
                (O::ArithmeticShiftRight, Some(size), M::FourValue) => {
                    let imm = SignedImmediate::new_from_u64(*imm as u64).unwrap();
                    bce.fv_sari(rd, rs, imm, size);
                }
                (O::ArithmeticShiftRight, None, M::TwoValue) => {
                    bce.load_u64(T2_SPC, *imm as u64);
                    bce.heap_tv_sar(rd, rs, T2_SPC, size);
                }
                (O::ArithmeticShiftRight, None, M::FourValue) => {
                    bce.load_u64(T2_SPC, *imm as u64);
                    bce.heap_fv_sar(rd, rs, T2_SPC, size);
                }
            }
        }
        I::Select(dst, cond, truthy, falsy) => {
            let size = gl.vars.size(*dst);
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let mut src_reg = T2_SPC;
            let mut rcond = to_reg(
                bce,
                *cond,
                &gl.vars,
                assignment[cond],
                stack_offsets,
                T1_SPC,
                false,
            );
            match cond.mode() {
                LogicMode::TwoValue => {}
                LogicMode::FourValue => {
                    bce.fv_ceqi(
                        T2_SPC,
                        rcond,
                        SignedImmediate::MINUS_ONE,
                        SixBitSize::SCALAR,
                    );
                    src_reg = T1_SPC;
                    rcond = T2_SPC;
                }
            }

            // @Performance: Better lowering
            let branch_offset = bce.data.len();
            bce.panic();

            let rfalsy = to_reg(
                bce,
                *falsy,
                &gl.vars,
                assignment[falsy],
                stack_offsets,
                src_reg,
                false,
            );
            match (dst.mode(), SixBitSize::from_vector_size(size)) {
                (LogicMode::TwoValue, Some(_)) => bce.copy(rd, rfalsy),
                (LogicMode::FourValue, Some(_)) => bce.fv_copy(rd, rfalsy),
                (LogicMode::TwoValue, None) => bce.heap_tv_copy(rd, rfalsy, size),
                (LogicMode::FourValue, None) => bce.heap_fv_copy(rd, rfalsy, size),
            }

            let jump_offset = bce.data.len();
            bce.panic();

            let offset = bce.data.len() - branch_offset - 1;
            let offset = SignedImmediate::new_from_u64(offset as u64).unwrap();
            bce.data[branch_offset] = Branch { rcond, imm: offset }.encode();

            let rtruthy = to_reg(
                bce,
                *truthy,
                &gl.vars,
                assignment[truthy],
                stack_offsets,
                src_reg,
                false,
            );
            match (dst.mode(), SixBitSize::from_vector_size(size)) {
                (LogicMode::TwoValue, Some(_)) => bce.copy(rd, rtruthy),
                (LogicMode::FourValue, Some(_)) => bce.fv_copy(rd, rtruthy),
                (LogicMode::TwoValue, None) => bce.heap_tv_copy(rd, rtruthy, size),
                (LogicMode::FourValue, None) => bce.heap_fv_copy(rd, rtruthy, size),
            }

            let offset = bce.data.len() - jump_offset - 1;
            let offset = SignedImmediate::new_from_u64(offset as u64).unwrap();
            bce.data[jump_offset] = Jump(offset).encode();

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::Intrinsic(dst, op, items) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);

            for item in items {
                let reg = to_reg(
                    bce,
                    *item,
                    &gl.vars,
                    assignment[item],
                    stack_offsets,
                    T2_SPC,
                    false,
                );
                bce.push_argument(gl.vars.size(*item), item.mode(), reg);
            }
            let intrinsic_id = bce
                .intrinsics
                .insert_index(IntrinsicOpEqWrap(op.as_ref().clone()));
            let intrinsic_id = match intrinsic_id.try_into().ok().and_then(|v| NonMaxU16::new(v)) {
                None => {
                    bce.load_u64(T2_SPC, intrinsic_id as u64);
                    None
                }
                Some(v) => Some(v),
            };
            bce.intrinsic(rd, intrinsic_id);
            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::LastUpdateTime(dst, signal) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let rt_key = io_signals[signal];
            let idx = InlineIndex::new(lupdt_indexes[&rt_key], bce, T2_VAL);
            bce.last_update_time(rd, idx);
            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::Probe(dst, signal, offset) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);

            let dst_size = gl.vars.size(*dst);
            let signal_size = gl.signals[*signal].size;
            assert!(dst_size <= signal_size);

            let rsignal = T1_SPC;
            load_signal_address(bce, rsignal, *signal, signals, io_signals);

            if *offset != 0 || dst_size != signal_size {
                match (dst.mode(), SixBitSize::from_vector_size(dst_size)) {
                    (LogicMode::TwoValue, None) => {
                        let (addr, offset) =
                            InlineAddrOffset::new(i64::from(*offset), bce, rsignal, T1_VAL);
                        let size = InlineNBitSize::new(dst_size, bce);
                        bce.load_heap_unaligned(rd, addr, offset, size);
                    }
                    (LogicMode::TwoValue, Some(size)) => {
                        let (addr, offset) =
                            InlineAddrOffset::new(i64::from(*offset), bce, rsignal, T1_VAL);
                        bce.load_unaligned(rd, addr, offset, size);
                    }
                    (LogicMode::FourValue, None) => {
                        let size = InlineNBitSize::new(dst_size, bce);
                        let (addr, spc_offset) =
                            InlineAddrOffset::new(i64::from(*offset), bce, rsignal, T1_VAL);
                        bce.load_heap_unaligned(rd, addr, spc_offset, size);
                        let (addr, val_offset) = InlineAddrOffset::new(
                            i64::from(*offset) + i64::from(signal_size.get().next_multiple_of(64)),
                            bce,
                            rsignal,
                            T1_VAL,
                        );
                        let rd_val_offset = dst_size.get().next_multiple_of(64).into();
                        match SignedImmediate::new(rd_val_offset) {
                            None => {
                                bce.load_u64(T2_VAL, rd_val_offset as u64);
                                bce.add(T2_VAL, rd, T2_VAL, SixBitSize::N64);
                            }
                            Some(imm) => bce.addi(T2_VAL, rd, imm, SixBitSize::N64),
                        }
                        bce.load_heap_unaligned(T2_VAL, addr, val_offset, size);
                    }
                    (LogicMode::FourValue, Some(size)) => {
                        let (rdspc, rdval) = rd.to_spc_and_val();
                        let (addr, spc_offset) =
                            InlineAddrOffset::new(i64::from(*offset), bce, rsignal, T1_VAL);
                        bce.load_unaligned(rdspc, addr, spc_offset, size);
                        let (addr, val_offset) = InlineAddrOffset::new(
                            i64::from(*offset) + i64::from(signal_size.get().next_multiple_of(64)),
                            bce,
                            rsignal,
                            T1_VAL,
                        );
                        bce.load_unaligned(rdval, addr, val_offset, size);
                    }
                }
            } else {
                match (dst.mode(), SixBitSize::from_vector_size(signal_size)) {
                    (LogicMode::TwoValue, None) => {
                        bce.load_heap_aligned(rd, rsignal, signal_size.get().div_ceil(64) as u16)
                    }
                    (LogicMode::TwoValue, Some(signal_size)) => {
                        bce.tv_load_aligned(rd, rsignal, InlineAddrOffset::ZERO, signal_size)
                    }
                    (LogicMode::FourValue, None) => bce.load_heap_aligned(
                        rd,
                        rsignal,
                        signal_size.get().div_ceil(64) as u16 * 2,
                    ),
                    (LogicMode::FourValue, Some(signal_size)) => {
                        bce.fv_load_aligned(rd, rsignal, InlineAddrOffset::ZERO, signal_size)
                    }
                }
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::ProbeSlice(dst, signal, offset) => {
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0_SPC, true);
            let roffset = to_reg(
                bce,
                *offset,
                &gl.vars,
                assignment[offset],
                stack_offsets,
                T1_SPC,
                false,
            );

            let dst_size = gl.vars.size(*dst);
            let signal_size = gl.signals[*signal].size;
            let roffsize = SixBitSize::from_vector_size(gl.vars.size(*offset)).unwrap();
            assert!(dst_size <= signal_size);

            let rsignal = T2_SPC;
            let rsignal_off = T2_VAL;
            load_signal_address(bce, rsignal, *signal, signals, io_signals);

            let mut jump_offset = None;
            match offset.mode() {
                M::TwoValue => bce.add(rsignal_off, rsignal, roffset, SixBitSize::N64),
                M::FourValue => {
                    let (roffspc, roffval) = roffset.to_spc_and_val();
                    bce.ceqi(T2_VAL, roffspc, SignedImmediate::MINUS_ONE, roffsize);

                    let branch_offset = bce.data.len();
                    bce.panic();

                    use LogicMode as M;
                    assert_eq!(dst.mode(), M::FourValue);
                    match SixBitSize::from_vector_size(dst_size) {
                        None => {
                            bce.heap_tv_or(rd, rd, rd, dst_size);
                            let offset = dst_size.get().next_multiple_of(64).into();
                            match SignedImmediate::new(offset) {
                                None => {
                                    bce.load_u64(T2_VAL, offset as u64);
                                    bce.add(T2_VAL, rd, T2_VAL, SixBitSize::N64);
                                }
                                Some(imm) => bce.addi(T2_VAL, rd, imm, SixBitSize::N64),
                            }
                            bce.heap_tv_or(T2_VAL, rd, rd, dst_size);
                        }
                        Some(_) => {
                            let (spc, val) = rd.to_spc_and_val();
                            bce.load_u64(spc, 0);
                            bce.load_u64(val, 0);
                        }
                    }

                    jump_offset = Some(bce.data.len());
                    bce.panic();

                    bce.data[branch_offset] = Branch {
                        rcond: T2_VAL,
                        imm: SignedImmediate::new((bce.data.len() - branch_offset) as i64 - 1)
                            .unwrap(),
                    }
                    .encode();

                    bce.add(rsignal_off, rsignal, roffval, SixBitSize::N64)
                }
            }

            use LogicMode as M;
            match (dst.mode(), SixBitSize::from_vector_size(dst_size)) {
                (M::TwoValue, None) => {
                    let size = InlineNBitSize::new(dst_size, bce);
                    bce.load_heap_unaligned(rd, rsignal_off, InlineAddrOffset::ZERO, size);
                }
                (M::TwoValue, Some(size)) => {
                    bce.load_unaligned(rd, rsignal_off, InlineAddrOffset::ZERO, size);
                }
                (M::FourValue, None) => {
                    let size = InlineNBitSize::new(dst_size, bce);
                    bce.load_heap_unaligned(rd, rsignal_off, InlineAddrOffset::ZERO, size);
                    let (addr, val_offset) = InlineAddrOffset::new(
                        i64::from(signal_size.get().next_multiple_of(64)),
                        bce,
                        rsignal_off,
                        T1_SPC,
                    );
                    let rd_val_offset = dst_size.get().next_multiple_of(64).into();
                    match SignedImmediate::new(rd_val_offset) {
                        None => {
                            bce.load_u64(T2_VAL, rd_val_offset as u64);
                            bce.add(T2_VAL, rd, T2_VAL, SixBitSize::N64);
                        }
                        Some(imm) => bce.addi(T2_VAL, rd, imm, SixBitSize::N64),
                    }
                    bce.load_heap_unaligned(T2_VAL, addr, val_offset, size);
                }
                (M::FourValue, Some(size)) => {
                    let alignment = HeapAlignment::new(size.into(), LogicMode::FourValue);
                    let (rdspc, rdval) = rd.to_spc_and_val();
                    bce.load_unaligned(rdspc, rsignal_off, InlineAddrOffset::ZERO, size);
                    let (addr, val_offset) = InlineAddrOffset::new(
                        alignment.next_aligned(signal_size.get() as u64) as i64,
                        bce,
                        rsignal_off,
                        T1_SPC,
                    );
                    bce.load_unaligned(rdval, addr, val_offset, size);
                }
            }

            if let Some(jump_offset) = jump_offset {
                bce.data[jump_offset] =
                    Jump(SignedImmediate::new((bce.data.len() - jump_offset) as i64 - 1).unwrap())
                        .encode();
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T1_VAL);
        }
        I::Drive(signal, src, partial) => {
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T0_SPC,
                false,
            );

            let rt_signal = io_signals[signal];
            let signal_size = gl.signals[*signal].size;
            let src_size = gl.vars.size(*src);

            let roff = T1_SPC;
            let rpoke = T1_VAL;
            load_signal_address(bce, roff, *signal, signals, io_signals);

            let mut branch_offset: Option<usize> = None;
            if partial.is_some() || signal_size != src_size {
                if let Some((partial, _)) = partial {
                    let rpartial = to_reg(
                        bce,
                        *partial,
                        &gl.vars,
                        assignment[partial],
                        stack_offsets,
                        T2_SPC,
                        false,
                    );

                    use LogicMode as M;
                    match partial.mode() {
                        M::TwoValue => bce.add(roff, roff, rpartial, SixBitSize::N64),
                        M::FourValue => {
                            let partial_size =
                                SixBitSize::from_vector_size(gl.vars.size(*partial)).unwrap();
                            let (rpartialspc, rpartialval) = rpartial.to_spc_and_val();
                            bce.cnei(
                                T1_VAL,
                                rpartialspc,
                                SignedImmediate::MINUS_ONE,
                                partial_size,
                            );

                            branch_offset = Some(bce.data.len());
                            bce.panic();

                            bce.add(roff, roff, rpartialval, SixBitSize::N64)
                        }
                    }
                }

                let rpoke_t1 = rpoke;
                let rpoke_t2 = T2_VAL;
                match (src.mode(), SixBitSize::from_vector_size(src_size)) {
                    (LogicMode::TwoValue, None) => {
                        let size = InlineNBitSize::new(src_size, bce);
                        bce.set_heap_unaligned(rpoke, rs, roff, size, InlineAddrOffset::ZERO);
                    }
                    (LogicMode::FourValue, None) => {
                        let size = InlineNBitSize::new(src_size, bce);
                        bce.set_heap_unaligned(rpoke_t1, rs, roff, size, InlineAddrOffset::ZERO);

                        let dst_offset = HeapAlignment::B64.next_aligned(signal_size.get() as u64);
                        let src_offset = HeapAlignment::B64.next_aligned(src_size.get() as u64);

                        match SignedImmediate::new_from_u64(dst_offset) {
                            None => {
                                bce.load_u64(T2_VAL, dst_offset);
                                bce.add(roff, roff, T2_VAL, SixBitSize::N64);
                            }
                            Some(imm) => bce.addi(roff, roff, imm, SixBitSize::N64),
                        }
                        match SignedImmediate::new_from_u64(src_offset) {
                            None => {
                                bce.load_u64(T2_VAL, src_offset);
                                bce.add(T0_VAL, rs, T2_VAL, SixBitSize::N64);
                            }
                            Some(imm) => bce.addi(T0_VAL, rs, imm, SixBitSize::N64),
                        }
                        bce.set_heap_unaligned(
                            rpoke_t2,
                            T0_VAL,
                            roff,
                            size,
                            InlineAddrOffset::ZERO,
                        );
                        bce.or(rpoke, rpoke_t1, rpoke_t2);
                    }
                    (LogicMode::TwoValue, Some(src_size)) => {
                        bce.set_unaligned(rpoke, rs, roff, InlineAddrOffset::ZERO, src_size);
                    }
                    (LogicMode::FourValue, Some(src_size)) => {
                        let (rsspc, rsval) = rs.to_spc_and_val();
                        let alignment =
                            HeapAlignment::new(signal_size.into(), LogicMode::FourValue);
                        bce.set_unaligned(rpoke_t1, rsspc, roff, InlineAddrOffset::ZERO, src_size);
                        let (addr, val_offset) = InlineAddrOffset::new(
                            alignment.next_aligned(signal_size.get() as u64) as i64,
                            bce,
                            roff,
                            T2_VAL,
                        );
                        bce.set_unaligned(rpoke_t2, rsval, addr, val_offset, src_size);
                        bce.or(rpoke, rpoke_t1, rpoke_t2);
                    }
                }
            } else {
                match (src.mode(), SixBitSize::from_vector_size(signal_size)) {
                    (LogicMode::TwoValue, None) => {
                        let size = InlineNBitSize::new(signal_size, bce);
                        bce.tv_set_heap_aligned(rpoke, rs, roff, size, InlineAddrOffset::ZERO);
                    }
                    (LogicMode::FourValue, None) => {
                        let size = InlineNBitSize::new(signal_size, bce);
                        bce.fv_set_heap_aligned(rpoke, rs, roff, size, InlineAddrOffset::ZERO);
                    }
                    (LogicMode::TwoValue, Some(signal_size)) => {
                        bce.tv_set_aligned(rpoke, rs, roff, InlineAddrOffset::ZERO, signal_size)
                    }
                    (LogicMode::FourValue, Some(signal_size)) => {
                        bce.fv_set_aligned(rpoke, rs, roff, InlineAddrOffset::ZERO, signal_size)
                    }
                }
            }

            if gl.signals[*signal].mode == LogicMode::TwoValue {
                let index = InlineIndex::new(rt_signal.as_u64(), bce, T2_VAL);
                bce.tv_correct_first(rpoke, index);
            }
            if let Some(lupdt_index) = lupdt_indexes.get(&io_signals[signal]) {
                let index = InlineIndex::new(*lupdt_index, bce, T2_VAL);
                bce.set_lupdt(rpoke, index);
            }
            for index in watch_map.watch_indices(*signal) {
                let index = InlineIndex::new(index as u64, bce, T2_VAL);
                bce.wake(rpoke, index);
            }

            if let Some(branch_offset) = branch_offset {
                bce.data[branch_offset] = Branch {
                    rcond: T1_VAL,
                    imm: SignedImmediate::new((bce.data.len() - branch_offset) as i64 - 1).unwrap(),
                }
                .encode();
            }
        }
        I::Phi(variable_key, items) => todo!(),
    }
}

fn to_reg(
    bytecode: &mut BytecodeEncoder,
    var: VariableKey,
    vars: &VariableMap,
    slot: Slot,
    stack_offsets: &StackOffsets,
    backup: Reg,
    is_dst: bool,
) -> Reg {
    match slot {
        Slot::Heap(offset) => {
            bytecode.load_u64(backup, offset);
            backup
        }
        Slot::Stack(kind, offset) => {
            let size = vars.size(var);
            if !is_dst || size > VSIZE_64 {
                let kind_offset = match kind {
                    HeapAlignment::B1 => 0,
                    HeapAlignment::B2 => stack_offsets.b2,
                    HeapAlignment::B4 => stack_offsets.b4,
                    HeapAlignment::B8 => stack_offsets.b8,
                    HeapAlignment::B16 => stack_offsets.b16,
                    HeapAlignment::B32 => stack_offsets.b32,
                    HeapAlignment::B64 => stack_offsets.b64,
                };
                let offset = kind_offset as u64 + offset as u64;
                match SignedImmediate::new_from_u64(offset) {
                    None => todo!(),
                    Some(offset) => bytecode.stack_offset(backup, kind, offset),
                }
            }

            if !is_dst && let Some(size) = SixBitSize::from_vector_size(size) {
                match var.mode() {
                    LogicMode::TwoValue => {
                        bytecode.tv_load_aligned(backup, backup, InlineAddrOffset::ZERO, size)
                    }
                    LogicMode::FourValue => {
                        bytecode.fv_load_aligned(backup, backup, InlineAddrOffset::ZERO, size)
                    }
                }
            }
            backup
        }
        Slot::Register(reg) => Reg::new_masked(reg),
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
    stack_offsets: &StackOffsets,
    var: VariableKey,
    slot: Slot,
    value: Reg,
    scratch: Reg,
) {
    match slot {
        Slot::Heap(..) => unreachable!(),
        Slot::Stack(kind, offset) => {
            if let Some(size) = SixBitSize::from_vector_size(vars.size(var)) {
                let kind_offset = match kind {
                    HeapAlignment::B1 => 0,
                    HeapAlignment::B2 => stack_offsets.b2,
                    HeapAlignment::B4 => stack_offsets.b4,
                    HeapAlignment::B8 => stack_offsets.b8,
                    HeapAlignment::B16 => stack_offsets.b16,
                    HeapAlignment::B32 => stack_offsets.b32,
                    HeapAlignment::B64 => stack_offsets.b64,
                };
                let offset = kind_offset as u64 + offset as u64;
                match SignedImmediate::new_from_u64(offset) {
                    None => todo!(),
                    Some(offset) => bytecode.stack_offset(scratch, kind, offset),
                }

                match var.mode() {
                    LogicMode::TwoValue => bytecode.tv_set_aligned(
                        scratch,
                        value,
                        scratch,
                        InlineAddrOffset::ZERO,
                        size,
                    ),
                    LogicMode::FourValue => bytecode.fv_set_aligned(
                        scratch,
                        value,
                        scratch,
                        InlineAddrOffset::ZERO,
                        size,
                    ),
                }
            }
        }
        Slot::Register(rd) => {
            let rd = Reg::new_masked(rd);
            bytecode.copy(rd, value);
        }
    }
}

fn tv_concat_heap(
    bce: &mut BytecodeEncoder,
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    src1_size: VectorSize,
    src2_size: VectorSize,
    scratch: Reg,
) {
    // @Incorrect: If regs are aliasing eachother. This is problematic.
    match SixBitSize::from_vector_size(src2_size) {
        None => {
            let size = InlineNBitSize::new(src2_size, bce);
            bce.set_heap_unaligned(scratch, rs2, rd, size, InlineAddrOffset::ZERO);
        }
        Some(size) => {
            bce.set_unaligned(scratch, rs2, rd, InlineAddrOffset::ZERO, size);
        }
    }
    match SixBitSize::from_vector_size(src1_size) {
        None => {
            let size = InlineNBitSize::new(src1_size, bce);
            let (addr, offset) = InlineAddrOffset::new(src2_size.get().into(), bce, rd, scratch);
            bce.set_heap_unaligned(scratch, rs1, addr, size, offset);
        }
        Some(size) => {
            let (addr, offset) = InlineAddrOffset::new(src2_size.get().into(), bce, rd, scratch);
            bce.set_unaligned(scratch, rs1, addr, offset, size);
        }
    }
}

fn fv_concat_heap(
    bce: &mut BytecodeEncoder,
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    src1_size: VectorSize,
    src2_size: VectorSize,
    scratch: Reg,
) {
    // @Incorrect: If regs are aliasing eachother. This is problematic.
    let (rs1_spc, rs1_val) = match SixBitSize::from_vector_size(src1_size) {
        None => (rs1, rs1),
        Some(_) => rs1.to_spc_and_val(),
    };
    let (rs2_spc, rs2_val) = match SixBitSize::from_vector_size(src2_size) {
        None => (rs2, rs2),
        Some(_) => rs2.to_spc_and_val(),
    };

    tv_concat_heap(bce, rd, rs1_spc, rs2_spc, src1_size, src2_size, scratch);
    let fv_offset = (src1_size.get() + src2_size.get()).div_ceil(64) as u64 * 64;
    match SixBitSize::from_vector_size(src2_size) {
        None => {
            let size = InlineNBitSize::new(src2_size, bce);
            let (addr, offset) = InlineAddrOffset::new(fv_offset as i64, bce, rd, scratch);
            let src_offset = src2_size.get().div_ceil(64) as u64 * 64;
            match SignedImmediate::new_from_u64(src_offset) {
                None => {
                    bce.load_u64(scratch, src_offset);
                    bce.add(scratch, rs2, scratch, SixBitSize::N64);
                }
                Some(value) => {
                    bce.addi(scratch, rs2, value, SixBitSize::N64);
                }
            }
            bce.set_heap_unaligned(scratch, scratch, addr, size, offset);
        }
        Some(size) => {
            let (addr, offset) = InlineAddrOffset::new(fv_offset as i64, bce, rd, scratch);
            bce.set_unaligned(scratch, rs2_val, addr, offset, size);
        }
    }
    match SixBitSize::from_vector_size(src1_size) {
        None => {
            let size = InlineNBitSize::new(src1_size, bce);
            let (addr, offset) = InlineAddrOffset::new(
                u64::from(src2_size.get()).wrapping_add(fv_offset) as i64,
                bce,
                rd,
                scratch,
            );
            let src_offset = src1_size.get().div_ceil(64) as u64 * 64;
            match SignedImmediate::new_from_u64(src_offset) {
                None => {
                    bce.load_u64(scratch, src_offset);
                    bce.add(scratch, rs1, scratch, SixBitSize::N64);
                }
                Some(value) => {
                    bce.addi(scratch, rs1, value, SixBitSize::N64);
                }
            }
            bce.set_heap_unaligned(scratch, scratch, addr, size, offset);
        }
        Some(size) => {
            let (addr, offset) = InlineAddrOffset::new(
                u64::from(src2_size.get()).wrapping_add(fv_offset) as i64,
                bce,
                rd,
                scratch,
            );
            bce.set_unaligned(scratch, rs1_val, addr, offset, size);
        }
    }
}
