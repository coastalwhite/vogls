use vogls_bits::arithmetic::FvLogicValue;
use vogls_codegen::lsra::{Slot, StackOffsets, StackTracker};
use vogls_codegen::{HeapAlignment, HeapBuilder, HeapRef, insert_bb_phis};
use vogls_ir::watchers::WatchMap;
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, ContextFormat, DisplayContext,
    GlobalContext, Instruction, IntrinsicOp, LabelDisplay, LogicMode, ProcessKey, ResizeOp,
    SCALAR_VSIZE, ShiftImmOp, SignalKey, UnaryOp, VSIZE_32, VSIZE_64, VariableKey, VariableMap,
};
use vogls_runtime::RtSignalKey;
use vogls_utils::{NonMaxU16, VgHashMap, VgHashSet};

use crate::bytecode::{
    Branch, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, InlineAddrOffset, InlineIndex,
    InlineNBitSize, InstructionPtr, IntrinsicOpEqWrap, Jump, Reg, Schedule, SignedImmediate,
    SixBitSize,
};

use super::{RescheduleListen, RescheduleRegion, RescheduleWait};

enum JumpKind {
    Jump,
    BranchTrue(Reg),
    BranchFalse(Reg),
    Wait(Reg),
    WaitRegion(u8),
    Listen(u64),
}

pub struct LowerBytecodeOptions {
    pub emit: bool,
    pub has_plugins: bool,
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
    options: &LowerBytecodeOptions,
) {
    let process = &gl.processes[process];

    let mut bb_stack = Vec::new();
    let mut bb_stack2 = Vec::new();
    let mut bb_seen = VgHashSet::<BasicBlockKey>::default();
    let mut bb_phis = VgHashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::default();

    let mut assignment = VgHashMap::default();

    let mut post_order = Vec::<BasicBlockKey>::new();
    let start_offset = bytecode.data.len();
    let mut bb_offsets = VgHashMap::<BasicBlockKey, usize>::default();
    let mut emit_sizes = Vec::<u8>::new();
    let mut jump_targets = Vec::<(usize, BasicBlockKey, JumpKind)>::new();

    insert_bb_phis(
        &process.regions,
        gl,
        &mut bb_stack2,
        &mut bb_seen,
        &mut bb_phis,
    );

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

        post_order.reverse();
        for (order_i, &bb_key) in post_order.iter().enumerate() {
            macro_rules! jump_to_if_not_next {
                ($target:expr) => {
                    let target: BasicBlockKey = $target;
                    if post_order.get(order_i + 1).copied() != Some(target) {
                        jump_targets.push((bytecode.data.len(), target, JumpKind::Jump));
                        bytecode.panic();
                    }
                };
            }

            bb_offsets.insert(bb_key, bytecode.data.len());

            let bb = &gl.bbs[bb_key];
            for i in &bb.instrs {
                let offset = bytecode.data.len();
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
                    options,
                );
                if options.emit {
                    emit_sizes.push((bytecode.data.len() - offset) as u8);
                }
            }

            if let Some(phis) = bb_phis.get(&bb_key) {
                for (dst, src) in phis {
                    let offset = bytecode.data.len();
                    let size = gl.vars.size(*dst);
                    let dslot = assignment[dst];
                    let rd = to_reg(bytecode, *dst, &gl.vars, dslot, &stack_offsets, T0, true);
                    let rs = to_reg(
                        bytecode,
                        *src,
                        &gl.vars,
                        assignment[src],
                        &stack_offsets,
                        T2,
                        false,
                    );
                    use LogicMode as M;
                    match (dst.mode(), SixBitSize::from_vector_size(size)) {
                        (M::TwoValue, None) => bytecode.heap_tv_copy(rd, rs, size),
                        (M::TwoValue, Some(_)) => bytecode.copy(rd, rs),
                        (M::FourValue, None) => bytecode.heap_fv_copy(rd, rs, size),
                        (M::FourValue, Some(_)) => bytecode.fv_copy(rd, rs),
                    }
                    store_back(bytecode, &gl.vars, &stack_offsets, *dst, dslot, rd, T2);
                    if options.emit {
                        emit_sizes.push((bytecode.data.len() - offset) as u8);
                    }
                }
            }

            let offset = bytecode.data.len();
            use BasicBlockTerminator as T;
            match &bb.terminator {
                T::Wait(target, time) => {
                    let time = time.0;

                    if time == 0 {
                        jump_to_if_not_next!(target.entry());
                    } else {
                        bytecode.load_u64(T0, time);
                        jump_targets.push((
                            bytecode.data.len(),
                            target.entry(),
                            JumpKind::Wait(T0),
                        ));
                        bytecode.panic();
                    }
                }
                T::VariableWait(target, src) => {
                    let mut rtime = to_reg(
                        bytecode,
                        *src,
                        &gl.vars,
                        assignment[src],
                        &stack_offsets,
                        T0,
                        false,
                    );

                    match src.mode() {
                        LogicMode::TwoValue => {}
                        LogicMode::FourValue => {
                            let (rtimespc, rtimeval) = rtime.to_spc_and_val();
                            bytecode.contains_no_special(T2, rtimespc, SixBitSize::N64);
                            bytecode.sign_extend(T2, T2, SixBitSize::N64, SixBitSize::N1);
                            bytecode.and(T2, rtimeval, T2);
                            rtime = T2;
                        }
                    }

                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Wait(rtime)));
                    bytecode.panic();
                }
                T::WaitRegion(target, region) => {
                    jump_targets.push((
                        bytecode.data.len(),
                        target.entry(),
                        JumpKind::WaitRegion(*region),
                    ));
                    bytecode.panic();
                }
                T::Watch(target, _) => {
                    let index = watch_map.get_watch_index(bb_key);
                    jump_targets.push((
                        bytecode.data.len(),
                        target.entry(),
                        JumpKind::Listen(index as u64),
                    ));
                    bytecode.panic();
                }
                T::Jump(target) => {
                    jump_to_if_not_next!(*target);
                }
                T::Branch(cond, truthy, falsy) => {
                    let mut rcond = to_reg(
                        bytecode,
                        *cond,
                        &gl.vars,
                        assignment[cond],
                        &stack_offsets,
                        T0,
                        false,
                    );
                    match cond.mode() {
                        LogicMode::TwoValue => {}
                        LogicMode::FourValue => {
                            bytecode.fv_ceqi(T0, rcond, SignedImmediate::MINUS_ONE, SixBitSize::N1);
                            rcond = T0;
                        }
                    }

                    let next_bb = post_order.get(order_i + 1).copied();
                    if next_bb == Some(*truthy) {
                        jump_targets.push((
                            bytecode.data.len(),
                            *falsy,
                            JumpKind::BranchFalse(rcond),
                        ));
                        bytecode.panic();
                    } else if next_bb == Some(*falsy) {
                        jump_targets.push((
                            bytecode.data.len(),
                            *truthy,
                            JumpKind::BranchTrue(rcond),
                        ));
                        bytecode.panic();
                    } else {
                        jump_targets.push((
                            bytecode.data.len(),
                            *truthy,
                            JumpKind::BranchTrue(rcond),
                        ));
                        bytecode.panic();
                        jump_targets.push((bytecode.data.len(), *falsy, JumpKind::Jump));
                        bytecode.panic();
                    }
                }
                T::Halt => {
                    bytecode.next_event();
                }
            }

            if options.emit {
                emit_sizes.push((bytecode.data.len() - offset) as u8);
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
            JumpKind::BranchTrue(rcond) => {
                let imm = SignedImmediate::new(imm.into()).unwrap();
                Branch {
                    rcond,
                    inv: false,
                    imm,
                }
                .encode()
            }
            JumpKind::BranchFalse(rcond) => {
                let imm = SignedImmediate::new(imm.into()).unwrap();
                Branch {
                    rcond,
                    inv: true,
                    imm,
                }
                .encode()
            }
            JumpKind::Wait(rtime) => {
                let offset = SignedImmediate::new(imm.into()).unwrap();
                RescheduleWait { rtime, offset }.encode()
            }
            JumpKind::WaitRegion(region) => {
                let offset = SignedImmediate::new(imm.into()).unwrap();
                RescheduleRegion { region, offset }.encode()
            }
            JumpKind::Listen(index) => {
                listeners.set_ptr(index as usize, InstructionPtr(target_offset as u64));
                RescheduleListen {
                    index: index as u32,
                }
                .encode()
            }
        };
    }

    if options.emit {
        eprintln!("proc {} {{", process.kind.into_static_str());

        let mut ctx = DisplayContext::new(gl);
        for tr in &process.regions {
            ctx.prepare_process(tr.entry());
        }

        let mut offset = start_offset;
        let mut j = 0;

        macro_rules! print_current_bytecode {
            () => {
                let size = emit_sizes[j] as usize;
                let mut k = 0;
                while k < size {
                    let c = bytecode.data[offset + k];
                    eprintln!("  {}", c);
                    k += 1;

                    for _ in 1..c.num_slots() {
                        eprintln!("  <data 0x{:08X}>", bytecode.data[offset + k].0);
                        k += 1;
                    }
                }
                offset += size;
                j += 1;
            };
        }

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

            post_order.reverse();

            for &bb_key in &post_order {
                let bb = &gl.bbs[bb_key];

                let lbl = LabelDisplay {
                    include_prefix: true,
                    angles: false,
                    bb: bb_key,
                };
                let lbl = lbl.display(&ctx);
                eprintln!("{lbl}:");

                for instr in &bb.instrs {
                    eprintln!("{}", instr.display(&ctx));
                    print_current_bytecode!();
                }

                if let Some(phis) = bb_phis.get(&bb_key) {
                    for (dst, src) in phis {
                        eprintln!(
                            "%t{} = phi %t{}",
                            ctx.get_var_name(*dst).unwrap(),
                            ctx.get_var_name(*src).unwrap()
                        );
                        print_current_bytecode!();
                    }
                }

                eprintln!("{}", bb.terminator.display(&ctx));
                print_current_bytecode!();
            }
        }
        eprintln!("}}");
        eprintln!();
    }
}

const T0: Reg = Reg::X10;
const T1: Reg = Reg::X11;
const T2: Reg = Reg::X12;
const T3: Reg = Reg::X13;
const T4: Reg = Reg::X14;
const T5: Reg = Reg::X15;

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
    options: &LowerBytecodeOptions,
) {
    use Instruction as I;
    match instr {
        I::Constant(dst, value) => {
            // Temporary Register Allocation:
            // T0:    RD (VAL / SPC)
            // T1:    RD (VAL)
            // T2:    Scratch for store back.
            // T3-T5: -

            // @NOTE:
            // Constants with size greater than 64 are stored on the heap and referenced on there.
            // Therefore, nothing needs to happen for them; the information is in the slot
            // assignment.
            if value.size() <= VSIZE_64 {
                let dslot = assignment[dst];
                if !matches!(dslot, Slot::Constant(..)) {
                    let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
                    bce.load_bits_into_register(rd, dst.mode(), value);
                    store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T3);
                }
            }
        }
        I::Unary(dst, op, src) => {
            // Temporary Register Allocation:
            // T0:    RD (ADDR / VAL / SPC)
            // T1:    RD (VAL)
            // T2:    RS (ADDR / VAL / SPC)
            // T3:    RS (VAL)
            // T4:    Scratch for addresses and store back.
            // T5:    -
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T2,
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
                    bce.truncate(rd, rd, SixBitSize::N1);
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
                    let offset = HeapAlignment::spc_offset_to_val_offset(src_size, 0);
                    let val = T4;
                    match SignedImmediate::new_from_u64(offset) {
                        None => {
                            bce.load_u64(val, offset);
                            bce.add(val, rd, val, SixBitSize::N64);
                        }
                        Some(imm) => bce.addi(val, rd, imm, SixBitSize::N64),
                    }
                    bce.heap_tv_copy(val, rs, src_size);

                    let size = InlineNBitSize::new(src_size, bce);
                    bce.tv_heap_fill(rd, true, size);
                }
                (O::FvToTv, _, _, Some(_)) => {
                    let (spc, val) = rs.to_spc_and_val();
                    bce.and(rd, spc, val)
                }
                (O::FvToTv, _, _, None) => {
                    let num_words = src_size.get().div_ceil(64) as u64;
                    let spc = rs;
                    let val = T4;
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

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T4);
        }
        I::Resize(dst, op, src) => {
            // Temporary Register Allocation:
            // T0:    RD (ADDR / VAL / SPC)
            // T1:    RD (VAL)
            // T2:    RS (ADDR / VAL / SPC)
            // T3:    RS (VAL)
            // T4:    Scratch for addresses and store back.
            // T5:    -

            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T2,
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
                    bce.load_rel_unaligned(rd, rs, InlineAddrOffset::ZERO, dst_size);
                }
                (O::Truncate, M::TwoValue, None, None) => {
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.load_u64(T4, src_size.get() as u64);
                    bce.heapheap_tv_truncate(rd, rs, dst_size, T4);
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
                    bce.load_rel_unaligned(rdspc, rs, InlineAddrOffset::ZERO, dst_size);
                    let (addr, offset) = InlineAddrOffset::new(
                        src_size.get().next_power_of_two() as i64,
                        bce,
                        rs,
                        T4,
                    );
                    bce.load_rel_unaligned(rdval, addr, offset, dst_size);
                }
                (O::Truncate, M::FourValue, None, None) => {
                    let dst_size = InlineNBitSize::new(dst_size, bce);
                    bce.load_u64(T4, src_size.get() as u64);
                    bce.heapheap_fv_truncate(rd, rs, dst_size, T4);
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
                            bce.load_u64(T4, mask);
                            bce.or(rdspc, rsspc, T4);
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
                    bce.load_u64(T4, dst_size.get().into());
                    bce.heapheap_tv_zero_extend(rd, rs, T4, src_size);
                }
                (O::ZeroExtend, M::FourValue, None, None) => {
                    let src_size = InlineNBitSize::new(src_size, bce);
                    bce.load_u64(T4, dst_size.get().into());
                    bce.heapheap_fv_zero_extend(rd, rs, T4, src_size);
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
                    bce.load_u64(T4, dst_size.get().into());
                    bce.heapheap_tv_sign_extend(rd, rs, T4, src_size);
                }
                (O::SignExtend, M::FourValue, None, None) => {
                    let src_size = InlineNBitSize::new(src_size, bce);
                    bce.load_u64(T4, dst_size.get().into());
                    bce.heapheap_fv_sign_extend(rd, rs, T4, src_size);
                }

                (O::Truncate, _, None, Some(_)) => unreachable!(),
                (O::ZeroExtend, _, Some(_), None) => unreachable!(),
                (O::SignExtend, _, Some(_), None) => unreachable!(),
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T4);
        }
        I::Binary(dst, op, lhs, rhs) => {
            // Temporary Register Allocation:
            // T0:    RD  (ADDR / VAL / SPC)
            // T1:    RD  (VAL)  /  Scratch if DST is address
            // T2:    RS1 (ADDR / VAL / SPC)
            // T3:    RS1 (VAL)
            // T4:    RS2 (ADDR / VAL / SPC)
            // T5:    RS2 (VAL)
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let rs1 = to_reg(
                bce,
                *lhs,
                &gl.vars,
                assignment[lhs],
                stack_offsets,
                T2,
                false,
            );
            let rs2 = to_reg(
                bce,
                *rhs,
                &gl.vars,
                assignment[rhs],
                stack_offsets,
                T4,
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
                    bce.fv_sll(rd, rs1, rs2, size, rhs.mode())
                }
                (O::LogicalShiftLeft, M::FourValue, None, _, _) => {
                    bce.heap_fv_sll(rd, rs1, rs2, dst_size)
                }
                (O::LogicalShiftRight, M::TwoValue, Some(size), _, _) => {
                    bce.slr(rd, rs1, rs2, size)
                }
                (O::LogicalShiftRight, M::TwoValue, None, _, _) => {
                    bce.heap_tv_slr(rd, rs1, rs2, dst_size)
                }
                (O::LogicalShiftRight, M::FourValue, Some(size), _, _) => {
                    bce.fv_slr(rd, rs1, rs2, size, rhs.mode())
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
                    bce.fv_sar(rd, rs1, rs2, size, rhs.mode())
                }
                (O::ArithmeticShiftRight, M::FourValue, None, _, _) => {
                    bce.heap_fv_sar(rd, rs1, rs2, dst_size)
                }

                (O::Concat, M::TwoValue, _, _, _)
                    if SixBitSize::from_vector_size(dst_size).is_some() =>
                {
                    let rhs_size = SixBitSize::from_vector_size(rhs_size).unwrap();
                    bce.lsor(rd, rs1, rs2, rhs_size)
                }
                (O::Concat, M::TwoValue, _, _, _) => {
                    bce.tv_heap_concat(rd, rs1, lhs_size, rs2, rhs_size);
                }
                (O::Concat, M::FourValue, _, _, _)
                    if SixBitSize::from_vector_size(dst_size).is_some() =>
                {
                    let rhs_size = SixBitSize::from_vector_size(rhs_size).unwrap();
                    bce.fv_lsor(rd, rs1, rs2, rhs_size)
                }
                (O::Concat, M::FourValue, _, _, _) => {
                    bce.fv_heap_concat(rd, rs1, lhs_size, rs2, rhs_size);
                }
                (O::CopyX, _, Some(_), M::TwoValue, _) => bce.copy(rd, rs1),
                (O::CopyX, _, None, M::TwoValue, _) => bce.heap_tv_copy(rd, rs1, lhs_size),
                (O::CopyX, _, Some(size), M::FourValue, _) => bce.fv_copyx(rd, rs1, rs2, size),
                (O::CopyX, _, None, M::FourValue, _) => bce.heap_fv_copyx(rd, rs1, rs2, lhs_size),
                (O::CopyZ, _, Some(_), M::TwoValue, _) => bce.copy(rd, rs1),
                (O::CopyZ, _, None, M::TwoValue, _) => bce.heap_tv_copy(rd, rs1, lhs_size),
                (O::CopyZ, _, Some(size), M::FourValue, _) => bce.fv_copyz(rd, rs1, rs2, size),
                (O::CopyZ, _, None, M::FourValue, _) => bce.heap_fv_copyz(rd, rs1, rs2, lhs_size),

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

                (O::Posedge, _, _, M::TwoValue, _) => bce.andnot(rd, rs2, rs1, SixBitSize::N1),
                (O::Posedge, _, _, M::FourValue, _) => bce.fv_posedge(rd, rs1, rs2),
                (O::Negedge, _, _, M::TwoValue, _) => bce.andnot(rd, rs1, rs2, SixBitSize::N1),
                (O::Negedge, _, _, M::FourValue, _) => bce.fv_negedge(rd, rs1, rs2),
            }

            // At this point, T2 - T4 are no longer used. We can use any of them.
            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T2);
        }
        I::BinaryImm(dst, op, src, imm) => {
            // Temporary Register Allocation:
            // T0:    RD  (ADDR / VAL / SPC)
            // T1:    RD  (VAL)
            // T2:    RS  (ADDR / VAL / SPC)
            // T3:    RS  (VAL)
            // T4:    IMM (ADDR / VAL / SPC) / Scratch
            // T5:    IMM (VAL)
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T2,
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
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.and(rd, rs, T4);
                        }
                        Some(imm) => bce.andi(rd, rs, imm, size),
                    }
                }
                (O::And, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_and(rd, rs, T4, src_size);
                }
                (O::And, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_and(rd, rs, T4);
                        }
                        Some(imm) => bce.fv_andi(rd, rs, imm, size),
                    }
                }
                (O::And, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_and(rd, rs, T4, src_size);
                }
                (O::Or, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.or(rd, rs, T4);
                        }
                        Some(imm) => bce.ori(rd, rs, imm, size),
                    }
                }
                (O::Or, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_or(rd, rs, T4, src_size);
                }
                (O::Or, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_or(rd, rs, T4);
                        }
                        Some(imm) => bce.fv_ori(rd, rs, imm, size),
                    }
                }
                (O::Or, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_or(rd, rs, T4, src_size);
                }
                (O::Xor, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.xor(rd, rs, T4);
                        }
                        Some(imm) => bce.xori(rd, rs, imm, size),
                    }
                }
                (O::Xor, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_xor(rd, rs, T4, src_size);
                }
                (O::Xor, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_xor(rd, rs, T4);
                        }
                        Some(imm) => bce.fv_xori(rd, rs, imm, size),
                    }
                }
                (O::Xor, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_xor(rd, rs, T4, src_size);
                }
                (O::Add, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.add(rd, rs, T4, size);
                        }
                        Some(imm) => bce.addi(rd, rs, imm, size),
                    }
                }
                (O::Add, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_add(rd, rs, T4, src_size);
                }
                (O::Add, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_add(rd, rs, T4, size);
                        }
                        Some(imm) => bce.fv_addi(rd, rs, imm, size),
                    }
                }
                (O::Add, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_add(rd, rs, T4, src_size);
                }
                (O::Sub, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.sub(rd, rs, T4, size);
                        }
                        Some(imm) => bce.subi(rd, rs, imm, size),
                    }
                }
                (O::Sub, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_sub(rd, rs, T4, src_size);
                }
                (O::Sub, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_sub(rd, rs, T4, size);
                        }
                        Some(imm) => bce.fv_subi(rd, rs, imm, size),
                    }
                }
                (O::Sub, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_sub(rd, rs, T4, src_size);
                }
                (O::Multiply, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.mul(rd, rs, T4, size);
                        }
                        Some(imm) => bce.muli(rd, rs, imm, size),
                    }
                }
                (O::Multiply, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_mul(rd, rs, T4, src_size);
                }
                (O::Multiply, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_mul(rd, rs, T4, size);
                        }
                        Some(imm) => bce.fv_muli(rd, rs, imm, size),
                    }
                }
                (O::Multiply, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_mul(rd, rs, T4, src_size);
                }
                (O::Power, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.pow(rd, rs, T4, size);
                }
                (O::Power, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_pow(rd, rs, T4, src_size);
                }
                (O::Power, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_pow(rd, rs, T4, size);
                }
                (O::Power, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_pow(rd, rs, T4, src_size);
                }
                (O::Divide, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.div0(rd, rs, T4, size);
                }
                (O::Divide, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_div0(rd, rs, T4, src_size);
                }
                (O::Divide, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_div0(rd, rs, T4, size);
                }
                (O::Divide, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_div0(rd, rs, T4, src_size);
                }
                (O::Modulus, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.mod0(rd, rs, T4, size);
                }
                (O::Modulus, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_mod0(rd, rs, T4, src_size);
                }
                (O::Modulus, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_mod0(rd, rs, T4, size);
                }
                (O::Modulus, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_mod0(rd, rs, T4, src_size);
                }
                (O::RevSub, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.sub(rd, T4, rs, size);
                        }
                        Some(imm) => bce.revsubi(rd, rs, imm, size),
                    }
                }
                (O::RevSub, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_sub(rd, T4, rs, src_size);
                }
                (O::RevSub, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_sub(rd, T4, rs, size);
                        }
                        Some(imm) => bce.fv_revsubi(rd, rs, imm, size),
                    }
                }
                (O::RevSub, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_sub(rd, T4, rs, src_size);
                }
                (O::RevPower, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.pow(rd, T4, rs, size);
                }
                (O::RevPower, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_pow(rd, T4, rs, src_size);
                }
                (O::RevPower, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_pow(rd, T4, rs, size);
                }
                (O::RevPower, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_pow(rd, T4, rs, src_size);
                }
                (O::RevDivideX, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.divx(rd, T4, rs, size);
                }
                (O::RevDivideX, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_divx(rd, T4, rs, src_size);
                }
                (O::RevDivideX, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_divx(rd, T4, rs, size);
                }
                (O::RevDivideX, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_divx(rd, T4, rs, src_size);
                }
                (O::RevDivide0, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.div0(rd, T4, rs, size);
                }
                (O::RevDivide0, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_div0(rd, T4, rs, src_size);
                }
                (O::RevDivide0, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_div0(rd, T4, rs, size);
                }
                (O::RevDivide0, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_div0(rd, T4, rs, src_size);
                }
                (O::RevModulusX, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.modx(rd, T4, rs, size);
                }
                (O::RevModulusX, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_modx(rd, T4, rs, src_size);
                }
                (O::RevModulusX, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_modx(rd, T4, rs, size);
                }
                (O::RevModulusX, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_modx(rd, T4, rs, src_size);
                }
                (O::RevModulus0, M::TwoValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.mod0(rd, T4, rs, size);
                }
                (O::RevModulus0, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_mod0(rd, T4, rs, src_size);
                }
                (O::RevModulus0, M::FourValue, Some(size), _, _, _) => {
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_mod0(rd, T4, rs, size);
                }
                (O::RevModulus0, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_mod0(rd, T4, rs, src_size);
                }
                (O::UnsignedLessEqual, M::TwoValue, _, _, _, Some(size)) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.uleq(rd, rs, T4);
                        }
                        Some(imm) => bce.uleqi(rd, rs, imm, size),
                    }
                }
                (O::UnsignedLessEqual, M::TwoValue, _, _, _, None) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_unsigned_leq(rd, rs, T4, src_size);
                }
                (O::UnsignedLessEqual, M::FourValue, _, _, _, Some(size)) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_uleq(rd, rs, T4, size);
                        }
                        Some(imm) => bce.fv_uleqi(rd, rs, imm, size),
                    }
                }
                (O::UnsignedLessEqual, M::FourValue, _, _, _, None) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_unsigned_leq(rd, rs, T4, src_size);
                }
                (O::UnsignedGreaterEqual, M::TwoValue, _, _, _, Some(size)) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.ugeq(rd, rs, T4);
                        }
                        Some(imm) => bce.ugeqi(rd, rs, imm, size),
                    }
                }
                (O::UnsignedGreaterEqual, M::TwoValue, _, _, _, None) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_unsigned_geq(rd, rs, T4, src_size);
                }
                (O::UnsignedGreaterEqual, M::FourValue, _, _, _, Some(size)) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_ugeq(rd, rs, T4, size);
                        }
                        Some(imm) => bce.fv_ugeqi(rd, rs, imm, size),
                    }
                }
                (O::UnsignedGreaterEqual, M::FourValue, _, _, _, None) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_unsigned_geq(rd, rs, T4, src_size);
                }
                (O::ConcatLeft, M::TwoValue, Some(_), _, _, _) => {
                    // @Performance. There is likely space here for a left_shift_or_immediate
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.lsor(rd, T4, rs, SixBitSize::from_vector_size(src_size).unwrap());
                }
                (O::ConcatLeft, M::TwoValue, None, _, _, _) => {
                    match SixBitSize::from_vector_size(imm.size()) {
                        None => {
                            let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                            bce.load_u64(T4, imm.offset.bit_offset as u64);
                        }
                        Some(_) => bce.load_bits_into_register(T4, M::TwoValue, imm),
                    }
                    bce.tv_heap_concat(rd, T4, imm.size(), rs, src_size);
                }
                (O::ConcatLeft, M::FourValue, Some(_), _, _, _) => {
                    // @Performance. There is likely space here for a fv_left_shift_or_immediate
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_lsor(rd, T4, rs, SixBitSize::from_vector_size(src_size).unwrap());
                }
                (O::ConcatLeft, M::FourValue, None, _, _, _) => {
                    match SixBitSize::from_vector_size(imm.size()) {
                        None => {
                            let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                            bce.load_u64(T4, imm.offset.bit_offset as u64);
                        }
                        Some(_) => bce.load_bits_into_register(T4, M::FourValue, imm),
                    }
                    bce.fv_heap_concat(rd, T4, imm.size(), rs, src_size);
                }
                (O::ConcatRight, M::TwoValue, Some(_), _, _, _) => {
                    // @Performance. There is likely space here for a left_shift_or_immediate
                    bce.load_bits_into_register(T4, M::TwoValue, imm);
                    bce.lsor(
                        rd,
                        rs,
                        T4,
                        SixBitSize::from_vector_size(imm.size()).unwrap(),
                    );
                }
                (O::ConcatRight, M::TwoValue, None, _, _, _) => {
                    match SixBitSize::from_vector_size(imm.size()) {
                        None => {
                            let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                            bce.load_u64(T4, imm.offset.bit_offset as u64);
                        }
                        Some(_) => bce.load_bits_into_register(T4, M::TwoValue, imm),
                    }
                    bce.tv_heap_concat(rd, rs, src_size, T4, imm.size());
                }
                (O::ConcatRight, M::FourValue, Some(_), _, _, _) => {
                    // @Performance. There is likely space here for a fv_left_shift_or_immediate
                    bce.load_bits_into_register(T4, M::FourValue, imm);
                    bce.fv_lsor(
                        rd,
                        rs,
                        T4,
                        SixBitSize::from_vector_size(imm.size()).unwrap(),
                    );
                }
                (O::ConcatRight, M::FourValue, None, _, _, _) => {
                    match SixBitSize::from_vector_size(imm.size()) {
                        None => {
                            let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                            bce.load_u64(T4, imm.offset.bit_offset as u64);
                        }
                        Some(_) => bce.load_bits_into_register(T4, M::FourValue, imm),
                    }
                    bce.fv_heap_concat(rd, rs, src_size, T4, imm.size());
                }
                (O::Min, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.min(rd, rs, T4);
                        }
                        Some(imm) => bce.mini(rd, rs, imm, size),
                    }
                }
                (O::Min, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_min(rd, rs, T4, src_size);
                }
                (O::Min, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_min(rd, rs, T4, size);
                        }
                        Some(imm) => bce.fv_mini(rd, rs, imm, size),
                    }
                }
                (O::Min, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_min(rd, rs, T4, src_size);
                }
                (O::Max, M::TwoValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.max(rd, rs, T4);
                        }
                        Some(imm) => bce.maxi(rd, rs, imm, size),
                    }
                }
                (O::Max, M::TwoValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_max(rd, rs, T4, src_size);
                }
                (O::Max, M::FourValue, Some(size), _, _, _) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, imm);
                            bce.fv_max(rd, rs, T4, size);
                        }
                        Some(imm) => bce.fv_maxi(rd, rs, imm, size),
                    }
                }
                (O::Max, M::FourValue, None, _, _, _) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_max(rd, rs, T4, src_size);
                }
                (O::CaseEquality, _, _, M::TwoValue, _, Some(src_size)) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.ceq(rd, rs, T4);
                        }
                        Some(imm) => bce.ceqi(rd, rs, imm, src_size),
                    }
                }
                (O::CaseEquality, _, _, M::TwoValue, _, None) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_ceq(rd, rs, T4, src_size.get().div_ceil(64));
                }
                (O::CaseEquality, _, _, M::FourValue, _, Some(src_size)) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, &imm);
                            bce.fv_ceq(rd, rs, T4);
                        }
                        Some(imm) => bce.fv_ceqi(rd, rs, imm, src_size),
                    }
                }
                (O::CaseEquality, _, _, M::FourValue, _, None) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_ceq(rd, rs, T4, src_size.get().div_ceil(64) * 2);
                }
                (O::BitwiseCaseEquality, _, _, M::TwoValue, _, Some(src_size)) => {
                    match SignedImmediate::new_from_bits(&imm.bitwise_negate()) {
                        None => {
                            bce.load_bits_into_register(T4, M::TwoValue, imm);
                            bce.xnor(rd, rs, T4, src_size);
                        }
                        Some(imm) => bce.xori(rd, rs, imm, src_size),
                    }
                }
                (O::BitwiseCaseEquality, _, _, M::TwoValue, _, None) => {
                    let imm = heap_builder.claim_constant(M::TwoValue, imm.bitwise_negate());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_tv_xor(rd, rs, T4, src_size);
                }
                (O::BitwiseCaseEquality, _, _, M::FourValue, _, Some(src_size)) => {
                    match SignedImmediate::new_from_bits(imm) {
                        None => {
                            bce.load_bits_into_register(T4, M::FourValue, &imm);
                            bce.fv_bitwise_ceq(rd, rs, T4, src_size);
                        }
                        Some(imm) => bce.fv_bitwise_ceqi(rd, rs, imm, src_size),
                    }
                }
                (O::BitwiseCaseEquality, _, _, M::FourValue, _, None) => {
                    let imm = heap_builder.claim_constant(M::FourValue, imm.clone());
                    bce.load_u64(T4, imm.offset.bit_offset as u64);
                    bce.heap_fv_bitwise_ceq(rd, rs, T4, src_size);
                }
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T5);
        }
        I::Slice(dst, src, imm) => {
            // Temporary Register Allocation:
            // T0:    RD  (ADDR / VAL / SPC)
            // T1:    RD  (VAL)
            // T2:    SRC (ADDR / VAL / SPC)
            // T3:    SRC (VAL) / Scratch if src_size > 64
            // T4:    IMM (ADDR / VAL / SPC)
            // T5:    IMM (VAL)

            // @Incorrect: This ignores that out-of-bounds slices should give X.
            let dst_size = gl.vars.size(*dst);
            let src_size = gl.vars.size(*src);

            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T2,
                false,
            );
            let mut rimm = to_reg(
                bce,
                *imm,
                &gl.vars,
                assignment[imm],
                stack_offsets,
                T4,
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

                    // TempReg: Destructs IMM (SPC). No longer assessible.
                    bce.contains_no_special(T4, rimmspc, imm_size);

                    let branch_offset = bce.data.len();
                    bce.panic();

                    // If IMM contains special values, the output should be all `x`.
                    if dst_size > VSIZE_64 {
                        let size = InlineNBitSize::new(dst_size, bce);
                        bce.fv_heap_fill(rd, FvLogicValue::X, size);
                    } else {
                        let (rdspc, rdval) = rd.to_spc_and_val();
                        bce.load_u64(rdspc, 0);
                        bce.load_u64(rdval, 0);
                    }

                    jump_offset = Some(bce.data.len());
                    bce.panic();

                    bce.data[branch_offset] = Branch {
                        rcond: T4,
                        inv: false,
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
                (M::TwoValue, Some(dst_size), Some(src_size)) => {
                    bce.copy(T1, rs);
                    bce.ori(T0, T0, SignedImmediate::MINUS_ONE, src_size);
                    bce.fv_slrx(rd, T0, rimm, dst_size, LogicMode::TwoValue);
                }
                (M::FourValue, Some(dst_size), Some(_)) => {
                    bce.fv_slrx(rd, rs, rimm, dst_size, LogicMode::TwoValue);
                }
                (M::TwoValue, Some(dst_size), None) => {
                    let src_size = InlineNBitSize::new(src_size, bce);
                    bce.tvtv_heap_slicex(rd, rs, rimm, dst_size, src_size);
                }
                (M::FourValue, Some(dst_size), None) => {
                    let src_size = InlineNBitSize::new(src_size, bce);
                    bce.fvtv_heap_slicex(rd, rs, rimm, dst_size, src_size);
                }
                (M::TwoValue, None, None) => {
                    bce.heap_slice(rd, rs, rimm, dst_size, src_size, false, true, false);
                }
                (M::FourValue, None, None) => {
                    bce.heap_slice(rd, rs, rimm, dst_size, src_size, true, true, false);
                }
                (_, None, Some(_)) => todo!(),
            }

            if let Some(jump_offset) = jump_offset {
                bce.data[jump_offset] =
                    Jump(SignedImmediate::new((bce.data.len() - jump_offset) as i64 - 1).unwrap())
                        .encode();
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T2);
        }
        I::SliceImm(dst, src, offset) => {
            // Temporary Register Allocation:
            // T0:    RD  (ADDR / VAL / SPC)
            // T1:    RD  (VAL)
            // T2:    SRC (ADDR / VAL / SPC)
            // T3:    SRC (VAL)
            // T4:    Scratch
            // T5:    Scratch

            let dst_size = gl.vars.size(*dst);
            let src_size = gl.vars.size(*src);

            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T2,
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
                    let (addr, offset) = InlineAddrOffset::new(i64::from(*offset), bce, rs, T4);
                    bce.load_rel_unaligned(rd, addr, offset, dst_size);
                }
                (M::FourValue, Some(dst_size), None) => {
                    // @Incorrect. Deal with out-of-bounds reads.
                    let (rdspc, rdval) = rd.to_spc_and_val();

                    let (addr, spc_offset) = InlineAddrOffset::new(i64::from(*offset), bce, rs, T4);
                    bce.load_rel_unaligned(rdspc, addr, spc_offset, dst_size);

                    let num_words = HeapAlignment::spc_offset_to_val_offset(src_size, 0);
                    let (addr, val_offset) = InlineAddrOffset::new(
                        (*offset as u64).wrapping_add(num_words) as i64,
                        bce,
                        rs,
                        T4,
                    );
                    bce.load_rel_unaligned(rdval, addr, val_offset, dst_size);
                }
                (M::TwoValue, None, None) => {
                    bce.load_u64(T4, *offset as u64);
                    bce.heap_slice(rd, rs, T4, dst_size, src_size, false, false, false);
                }
                (M::FourValue, None, None) => {
                    bce.load_u64(T4, *offset as u64);
                    bce.heap_slice(rd, rs, T4, dst_size, src_size, true, false, false);
                }
                (M::TwoValue, None, Some(_)) => unreachable!(),
                (M::FourValue, None, Some(_)) => unreachable!(),
            }
            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T2);
        }
        I::ShiftImm(dst, op, src, imm) => {
            // Temporary Register Allocation:
            // T0:    RD  (ADDR / VAL / SPC)
            // T1:    RD  (VAL)
            // T2:    SRC (ADDR / VAL / SPC)
            // T3:    SRC (VAL)
            // T4:    Scratch
            // T5:    x

            let size = gl.vars.size(*dst);
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T2,
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
                    bce.load_u64(T4, *imm as u64);
                    bce.heap_tv_sll(rd, rs, T4, size);
                }
                (O::LogicalShiftLeft, None, M::FourValue) => {
                    bce.load_u64(T4, *imm as u64);
                    bce.heap_fv_sll(rd, rs, T4, size);
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
                    bce.load_u64(T4, *imm as u64);
                    bce.heap_tv_slr(rd, rs, T4, size);
                }
                (O::LogicalShiftRight, None, M::FourValue) => {
                    bce.load_u64(T4, *imm as u64);
                    bce.heap_fv_slr(rd, rs, T4, size);
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
                    bce.load_u64(T4, *imm as u64);
                    bce.heap_tv_sar(rd, rs, T4, size);
                }
                (O::ArithmeticShiftRight, None, M::FourValue) => {
                    bce.load_u64(T4, *imm as u64);
                    bce.heap_fv_sar(rd, rs, T4, size);
                }
            }
            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T2);
        }
        I::Select(dst, cond, truthy, falsy) => {
            // Temporary Register Allocation:
            // T0:    RD  (ADDR / VAL / SPC)
            // T1:    RD  (VAL)
            // T2:    COND (VAL / SPC)
            // T3:    COND (VAL)
            // T4:    TRUTHY / FALSY (ADDR / VAL / SPC)
            // T5:    TRUTHY / FALSY (VAL)

            let size = gl.vars.size(*dst);
            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let mut rcond = to_reg(
                bce,
                *cond,
                &gl.vars,
                assignment[cond],
                stack_offsets,
                T2,
                false,
            );
            match cond.mode() {
                LogicMode::TwoValue => {}
                LogicMode::FourValue => {
                    bce.fv_ceqi(T2, rcond, SignedImmediate::MINUS_ONE, SixBitSize::N1);
                    rcond = T2;
                }
            }
            let src_reg = T4;

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
            bce.data[branch_offset] = Branch {
                rcond,
                inv: false,
                imm: offset,
            }
            .encode();

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

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T2);
        }
        I::Intrinsic(dst, op, items) => {
            // Temporary Register Allocation:
            // T0:    RD  (ADDR / VAL / SPC)
            // T1:    RD  (VAL)
            // T2:    ITEM (VAL / SPC)
            // T3:    ITEM (VAL)
            // T4:    ID
            // T5:    -

            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);

            match op.as_ref() {
                IntrinsicOp::ReadMem(read_mem) => {
                    let signal = read_mem.signal;
                    let signal_addr = signal_address(signal, signals, io_signals);
                    let mode = gl.signals[signal].mode;
                    let size = gl.signals[signal].size;

                    bce.load_u64(T2, signal_addr);
                    bce.push_argument(VSIZE_64, LogicMode::TwoValue, T2);
                    bce.load_u64(T2, mode as u64);
                    bce.push_argument(SCALAR_VSIZE, LogicMode::TwoValue, T2);
                    bce.load_u64(T2, size.get() as u64);
                    bce.push_argument(VSIZE_32, LogicMode::TwoValue, T2);
                }
                _ => {
                    for item in items {
                        let reg = to_reg(
                            bce,
                            *item,
                            &gl.vars,
                            assignment[item],
                            stack_offsets,
                            T2,
                            false,
                        );
                        bce.push_argument(gl.vars.size(*item), item.mode(), reg);
                    }
                }
            }
            let intrinsic_id = bce
                .intrinsics
                .insert_index(IntrinsicOpEqWrap(op.as_ref().clone()));
            let intrinsic_id = match intrinsic_id.try_into().ok().and_then(|v| NonMaxU16::new(v)) {
                None => {
                    bce.load_u64(T4, intrinsic_id as u64);
                    None
                }
                Some(v) => Some(v),
            };
            bce.intrinsic(rd, intrinsic_id);

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T2);
        }
        I::LastUpdateTime(dst, signal) => {
            // Temporary Register Allocation:
            // T0:    RD  (ADDR / VAL / SPC)
            // T1:    RD  (VAL)
            // T2:    -
            // T3:    -
            // T4:    -
            // T5:    IDX

            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let rt_key = io_signals[signal];
            let idx = InlineIndex::new(lupdt_indexes[&rt_key], bce, T5);
            bce.last_update_time(rd, idx);
            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T2);
        }
        I::Probe(dst, signal, offset) => {
            // Temporary Register Allocation:
            // T0:    RD  (ADDR / VAL / SPC)
            // T1:    RD  (VAL)
            // T2:    Signal (ADDR)
            // T3:    Scratch
            // T4:    Scratch
            // T5:    -

            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);

            let dst_size = gl.vars.size(*dst);
            let signal_size = gl.signals[*signal].size;
            assert!(dst_size <= signal_size);

            let rsignal = T2;
            let signal_addr = signal_address(*signal, signals, io_signals);

            if *offset != 0 || dst_size != signal_size {
                match (dst.mode(), SixBitSize::from_vector_size(dst_size)) {
                    (LogicMode::TwoValue, None) => {
                        // @Performance: Better lowering.
                        bce.load_u64(rsignal, signal_addr);
                        bce.load_u64(T4, *offset as u64);
                        bce.heap_slice(rd, rsignal, T4, dst_size, signal_size, false, false, false);
                    }
                    (LogicMode::TwoValue, Some(size)) => {
                        let offset = signal_addr.wrapping_add(*offset as u64);
                        bce.load_unaligned(rd, offset, size);
                    }
                    (LogicMode::FourValue, None) => {
                        // @Performance: Better lowering.
                        bce.load_u64(rsignal, signal_addr);
                        bce.load_u64(T4, *offset as u64);
                        bce.heap_slice(rd, rsignal, T4, dst_size, signal_size, true, false, false);
                    }
                    (LogicMode::FourValue, Some(size)) => {
                        let (rdspc, rdval) = rd.to_spc_and_val();
                        let spc_offset = signal_addr.wrapping_add(*offset as u64);
                        let val_offset =
                            HeapAlignment::spc_offset_to_val_offset(signal_size, spc_offset);
                        bce.load_unaligned(rdspc, spc_offset, size);
                        bce.load_unaligned(rdval, val_offset, size);
                    }
                }
            } else {
                match (dst.mode(), SixBitSize::from_vector_size(signal_size)) {
                    (LogicMode::TwoValue, None) => {
                        bce.load_u64(rsignal, signal_addr);
                        bce.load_heap_aligned(rd, rsignal, signal_size.get().div_ceil(64) as u16);
                    }
                    (LogicMode::TwoValue, Some(signal_size)) => {
                        bce.tv_load_aligned(rd, signal_addr, signal_size)
                    }
                    (LogicMode::FourValue, None) => {
                        bce.load_u64(rsignal, signal_addr);
                        bce.load_heap_aligned(
                            rd,
                            rsignal,
                            signal_size.get().div_ceil(64) as u16 * 2,
                        );
                    }
                    (LogicMode::FourValue, Some(signal_size)) => {
                        bce.load_u64(rsignal, signal_addr);
                        bce.fv_load_aligned(rd, rsignal, InlineAddrOffset::ZERO, signal_size)
                    }
                }
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T2);
        }
        I::ProbeSlice(dst, signal, offset) => {
            // Temporary Register Allocation:
            // T0:    RD      (ADDR / VAL / SPC)
            // T1:    RD      (VAL) / Scratch if dst_size > 64
            // T2:    ROFFSET (ADDR / VAL / SPC)
            // T3:    ROFFSET (VAL)
            // T4:    Signal Address (Offset and Not Offset)
            // T5:    Scratch

            let dslot = assignment[dst];
            let rd = to_reg(bce, *dst, &gl.vars, dslot, stack_offsets, T0, true);
            let mut roffset = to_reg(
                bce,
                *offset,
                &gl.vars,
                assignment[offset],
                stack_offsets,
                T2,
                false,
            );

            let dst_size = gl.vars.size(*dst);
            let signal_size = gl.signals[*signal].size;
            let roffsize = SixBitSize::from_vector_size(gl.vars.size(*offset)).unwrap();
            assert!(dst_size <= signal_size);

            let rsignal = T4;
            let signal_addr = signal_address(*signal, signals, io_signals);
            // @Performance: Better lowering.
            bce.load_u64(rsignal, signal_addr);

            let mut jump_offset = None;
            match offset.mode() {
                M::TwoValue => {}
                M::FourValue => {
                    let (roffspc, roffval) = roffset.to_spc_and_val();

                    // TempReg: Destructs OFFSET (SPC). No longer assessible.
                    bce.contains_no_special(T2, roffspc, roffsize);

                    let branch_offset = bce.data.len();
                    bce.panic();

                    use LogicMode as M;
                    assert_eq!(dst.mode(), M::FourValue);
                    match SixBitSize::from_vector_size(dst_size) {
                        None => {
                            let size = InlineNBitSize::new(dst_size, bce);
                            bce.fv_heap_fill(rd, FvLogicValue::X, size);
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
                        rcond: T2,
                        inv: false,
                        imm: SignedImmediate::new((bce.data.len() - branch_offset) as i64 - 1)
                            .unwrap(),
                    }
                    .encode();

                    roffset = roffval;
                }
            }

            use LogicMode as M;
            match (
                gl.signals[*signal].mode,
                SixBitSize::from_vector_size(dst_size),
            ) {
                (M::TwoValue, None) => {
                    bce.heap_slice(
                        rd,
                        rsignal,
                        roffset,
                        dst_size,
                        signal_size,
                        false,
                        true,
                        offset.mode() == M::FourValue,
                    );
                }
                (M::TwoValue, Some(size)) => {
                    let src_size = InlineNBitSize::new(signal_size, bce);
                    bce.tvtv_heap_slicex(rd, rsignal, roffset, size, src_size);
                }
                (M::FourValue, None) => {
                    // TempReg: dst_size > 64 => T1 is free.

                    bce.heap_slice(
                        rd,
                        rsignal,
                        roffset,
                        dst_size,
                        signal_size,
                        true,
                        true,
                        offset.mode() == M::FourValue,
                    );
                }
                (M::FourValue, Some(size)) => {
                    let src_size = InlineNBitSize::new(signal_size, bce);
                    bce.fvtv_heap_slicex(rd, rsignal, roffset, size, src_size);
                }
            }

            if let Some(jump_offset) = jump_offset {
                bce.data[jump_offset] =
                    Jump(SignedImmediate::new((bce.data.len() - jump_offset) as i64 - 1).unwrap())
                        .encode();
            }

            store_back(bce, &gl.vars, stack_offsets, *dst, dslot, rd, T3);
        }
        I::Drive(signal, src, partial) => {
            // Temporary Register Allocation:
            // T0:    RS       (ADDR / VAL / SPC)
            // T1:    RS       (VAL)
            // T2:    ROFF     (ADDR / VAL / SPC)
            // T3:    RPOKE    (BOOL)
            // T4:    RPARTIAL (ADDR / VAL / SPC)
            // T5:    RPARTIAL (VAL)

            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T0,
                false,
            );

            let signal_size = gl.signals[*signal].size;
            let src_size = gl.vars.size(*src);

            let rpoke = T3;
            let signal_addr = signal_address(*signal, signals, io_signals);

            if *partial != 0 || signal_size != src_size {
                let offset = signal_addr.wrapping_add(u64::from(*partial));

                // TempReg: PARTIAL is no longer used from here.
                let rpoke_t1 = T4;
                let rpoke_t2 = T5;
                match (src.mode(), SixBitSize::from_vector_size(src_size)) {
                    (LogicMode::TwoValue, None) => {
                        let roff = T2;
                        bce.load_u64(roff, offset);
                        let size = InlineNBitSize::new(src_size, bce);
                        bce.set_heap_unaligned(rpoke, rs, roff, size, InlineAddrOffset::ZERO);
                    }
                    (LogicMode::FourValue, None) => {
                        let roff = T2;
                        bce.load_u64(roff, offset);
                        let size = InlineNBitSize::new(src_size, bce);
                        bce.set_heap_unaligned(rpoke_t1, rs, roff, size, InlineAddrOffset::ZERO);

                        let dst_offset = HeapAlignment::B64.next_aligned(signal_size.get() as u64);
                        let src_offset = HeapAlignment::B64.next_aligned(src_size.get() as u64);

                        match SignedImmediate::new_from_u64(dst_offset) {
                            None => {
                                bce.load_u64(T5, dst_offset);
                                bce.add(roff, roff, T5, SixBitSize::N64);
                            }
                            Some(imm) => bce.addi(roff, roff, imm, SixBitSize::N64),
                        }
                        match SignedImmediate::new_from_u64(src_offset) {
                            None => {
                                bce.load_u64(T5, src_offset);
                                bce.add(T1, rs, T5, SixBitSize::N64);
                            }
                            Some(imm) => bce.addi(T1, rs, imm, SixBitSize::N64),
                        }
                        bce.set_heap_unaligned(rpoke_t2, T1, roff, size, InlineAddrOffset::ZERO);
                        bce.or(rpoke, rpoke_t1, rpoke_t2);
                    }
                    (LogicMode::TwoValue, Some(src_size)) => {
                        let roff = T2;
                        bce.load_u64(roff, offset);
                        bce.set_unaligned(rpoke, rs, roff, InlineAddrOffset::ZERO, src_size);
                    }
                    (LogicMode::FourValue, Some(src_size)) => {
                        let roff = T2;
                        bce.load_u64(roff, offset);
                        let (rsspc, rsval) = rs.to_spc_and_val();
                        bce.set_unaligned(rpoke_t1, rsspc, roff, InlineAddrOffset::ZERO, src_size);
                        let val_offset = HeapAlignment::spc_offset_to_val_offset(signal_size, 0);
                        let (addr, val_offset) =
                            InlineAddrOffset::new(val_offset as i64, bce, roff, T5);
                        bce.set_unaligned(rpoke_t2, rsval, addr, val_offset, src_size);
                        bce.or(rpoke, rpoke_t1, rpoke_t2);
                    }
                }
            } else {
                match (src.mode(), SixBitSize::from_vector_size(signal_size)) {
                    (LogicMode::TwoValue, None) => {
                        let roff = T2;
                        bce.load_u64(roff, signal_addr);
                        let size = InlineNBitSize::new(signal_size, bce);
                        bce.tv_set_heap_aligned(rpoke, rs, roff, size, InlineAddrOffset::ZERO);
                    }
                    (LogicMode::FourValue, None) => {
                        let roff = T2;
                        bce.load_u64(roff, signal_addr);
                        let size = InlineNBitSize::new(signal_size, bce);
                        bce.fv_set_heap_aligned(rpoke, rs, roff, size, InlineAddrOffset::ZERO);
                    }
                    (LogicMode::TwoValue, Some(signal_size)) => {
                        bce.tv_set_aligned(rpoke, rs, signal_addr, signal_size)
                    }
                    (LogicMode::FourValue, Some(signal_size)) => {
                        bce.fv_set_aligned(rpoke, rs, signal_addr, signal_size)
                    }
                }
            }

            poke_signal(
                bce,
                gl,
                rpoke,
                *signal,
                io_signals,
                lupdt_indexes,
                watch_map,
                options,
            );
        }
        I::DriveSlice(signal, src, partial) => {
            // Temporary Register Allocation:
            // T0:    RS       (ADDR / VAL / SPC)
            // T1:    RS       (VAL)
            // T2:    ROFF     (ADDR / VAL / SPC)
            // T3:    RPOKE    (BOOL)
            // T4:    RPARTIAL (ADDR / VAL / SPC)
            // T5:    RPARTIAL (VAL)

            let rs = to_reg(
                bce,
                *src,
                &gl.vars,
                assignment[src],
                stack_offsets,
                T0,
                false,
            );

            let signal_size = gl.signals[*signal].size;
            let src_size = gl.vars.size(*src);

            let roff = T2;
            let rpoke = T3;
            let signal_addr = signal_address(*signal, signals, io_signals);

            let mut branch_offset: Option<usize> = None;
            bce.load_u64(roff, signal_addr);
            let mut rpartial = to_reg(
                bce,
                *partial,
                &gl.vars,
                assignment[partial],
                stack_offsets,
                T4,
                false,
            );

            use LogicMode as M;
            match partial.mode() {
                M::TwoValue => {}
                M::FourValue => {
                    let partial_size =
                        SixBitSize::from_vector_size(gl.vars.size(*partial)).unwrap();
                    let (rpartialspc, rpartialval) = rpartial.to_spc_and_val();

                    // TempReg: Destructs PARTIAL (SPC). No longer assessible.
                    bce.contains_special(T4, rpartialspc, partial_size);

                    branch_offset = Some(bce.data.len());
                    bce.panic();

                    rpartial = rpartialval;
                }
            }

            bce.add(roff, roff, rpartial, SixBitSize::N64);

            // TempReg: PARTIAL is no longer used from here.
            let rpoke_t1 = T4;
            let rpoke_t2 = T5;
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
                            bce.load_u64(T5, dst_offset);
                            bce.add(roff, roff, T5, SixBitSize::N64);
                        }
                        Some(imm) => bce.addi(roff, roff, imm, SixBitSize::N64),
                    }
                    match SignedImmediate::new_from_u64(src_offset) {
                        None => {
                            bce.load_u64(T5, src_offset);
                            bce.add(T1, rs, T5, SixBitSize::N64);
                        }
                        Some(imm) => bce.addi(T1, rs, imm, SixBitSize::N64),
                    }
                    bce.set_heap_unaligned(rpoke_t2, T1, roff, size, InlineAddrOffset::ZERO);
                    bce.or(rpoke, rpoke_t1, rpoke_t2);
                }
                (LogicMode::TwoValue, Some(src_size)) => {
                    bce.set_unaligned(rpoke, rs, roff, InlineAddrOffset::ZERO, src_size);
                }
                (LogicMode::FourValue, Some(src_size)) => {
                    let (rsspc, rsval) = rs.to_spc_and_val();
                    bce.set_unaligned(rpoke_t1, rsspc, roff, InlineAddrOffset::ZERO, src_size);
                    let val_offset = HeapAlignment::spc_offset_to_val_offset(signal_size, 0);
                    let (addr, val_offset) =
                        InlineAddrOffset::new(val_offset as i64, bce, roff, T5);
                    bce.set_unaligned(rpoke_t2, rsval, addr, val_offset, src_size);
                    bce.or(rpoke, rpoke_t1, rpoke_t2);
                }
            }

            poke_signal(
                bce,
                gl,
                rpoke,
                *signal,
                io_signals,
                lupdt_indexes,
                watch_map,
                options,
            );

            if let Some(branch_offset) = branch_offset {
                bce.data[branch_offset] = Branch {
                    rcond: T4,
                    inv: false,
                    imm: SignedImmediate::new((bce.data.len() - branch_offset) as i64 - 1).unwrap(),
                }
                .encode();
            }
        }
        I::Phi(..) => {
            // These are handles at the basic block level.
        }
    }
}

fn poke_signal(
    bce: &mut BytecodeEncoder,
    gl: &GlobalContext,
    rpoke: Reg,
    signal: SignalKey,
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    lupdt_indexes: &VgHashMap<RtSignalKey, u64>,
    watch_map: &WatchMap,
    options: &LowerBytecodeOptions,
) {
    let rt_signal = io_signals[&signal];
    let lupdt_index = lupdt_indexes.get(&rt_signal);
    let has_watchers = watch_map.num_watch_indices(signal) > 0;

    if gl.signals[signal].mode == LogicMode::TwoValue
        && (options.has_plugins || lupdt_index.is_some() || has_watchers)
    {
        let index = InlineIndex::new(rt_signal.as_u64(), bce, T5);
        bce.tv_correct_first(rpoke, index);
    }
    if options.has_plugins {
        let index = rt_signal.as_u64();
        bce.plugin_poke(rpoke, index);
    }
    if let Some(lupdt_index) = lupdt_index {
        let index = InlineIndex::new(*lupdt_index, bce, T5);
        bce.set_lupdt(rpoke, index);
    }
    for index in watch_map.watch_indices(signal) {
        let index = InlineIndex::new(index as u64, bce, T5);
        bce.wake(rpoke, index);
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
        Slot::Constant(bits) => {
            let bits = bits.into_bits(vars.size(var));
            bytecode.load_bits_into_register(backup, var.mode(), &bits);
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
                        bytecode.tv_load_rel_aligned(backup, backup, InlineAddrOffset::ZERO, size)
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

fn signal_address(
    signal: SignalKey,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
) -> u64 {
    let signal = io_signals[&signal];
    let at = signals[signal.as_usize()];
    at.offset.bit_offset as u64
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
        Slot::Heap(..) | Slot::Constant(..) => unreachable!(),
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
                    LogicMode::TwoValue => bytecode.tv_rel_set_aligned(
                        scratch,
                        value,
                        scratch,
                        InlineAddrOffset::ZERO,
                        size,
                    ),
                    LogicMode::FourValue => bytecode.fv_rel_set_aligned(
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
