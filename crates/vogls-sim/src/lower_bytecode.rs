use vogls_codegen::lsra::{Slot, StackTracker};
use vogls_codegen::{HeapOffset, HeapRef, insert_bb_phis, resolve_var_logic_mode_map};
use vogls_ir::watchers::WatchMap;
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction, LogicMode,
    ProcessKey, ResizeOp, SignalKey, UnaryOp, VariableKey, VectorSize,
};
use vogls_runtime::RtSignalKey;
use vogls_utils::{VgHashMap, VgHashSet};

use crate::bytecode::{
    BitwiseOp, Bytecode, BytecodeEncoder, BytecodeKind, BytecodeListeners, EncJump, EncRelJump,
    Encoding, InstructionPtr, Reg, Schedule, SixBitSize,
};

enum JumpKind {
    Jump,
    Branch,
}

pub fn lower_process_to_bytecode(
    process: ProcessKey,
    gl: &GlobalContext,
    stack_tracker: &mut StackTracker,
    watch_map: &WatchMap,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    bytecode: &mut BytecodeEncoder,
) {
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
            12,
        );

        post_order.reverse();
        for &bb_key in &post_order {
            bb_offsets.insert(bb_key, bytecode.data.len());

            let bb = &gl.bbs[bb_key];
            for i in &bb.instrs {
                lower_instruction(gl, bytecode, &assignment, watch_map, signals, io_signals, i);
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
                    let rtime = to_reg(bytecode, assignment[src], T0);
                    bytecode.wait(rtime);
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.jump(0);
                }
                T::WaitRegion(target, region) => {
                    bytecode.wait_region(*region);
                    jump_targets.push((bytecode.data.len(), target.entry(), JumpKind::Jump));
                    bytecode.jump(0);
                }
                T::Watch(_, _) => {
                    let index = watch_map.get_watch_index(bb_key);
                    bytecode.start_listen(index as u32);
                    listeners.set_ptr(index, bytecode.current_ptr());
                    bytecode.next_event();
                }
                T::Jump(target) => {
                    jump_targets.push((bytecode.data.len(), *target, JumpKind::Jump));
                    bytecode.jump(0);
                }
                T::Branch(cond, truthy, falsy) => {
                    let rcond = to_reg(bytecode, assignment[cond], T0);
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

const T0: Reg = Reg::X12;
const T1: Reg = Reg::X13;
const T2: Reg = Reg::X14;
const SP: Reg = Reg::X15;

fn lower_instruction(
    gl: &GlobalContext,
    bytecode: &mut BytecodeEncoder,
    assignment: &VgHashMap<VariableKey, Slot>,
    watch_map: &WatchMap,
    signals: &[HeapRef],
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    instr: &Instruction,
) {
    use Instruction as I;
    match instr {
        I::Constant(dst, value) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, dslot, T0);

            let Some(value) = value.as_u64() else {
                todo!();
            };

            bytecode.load_u64(rd, value);
            store_back(bytecode, dslot, rd);
        }
        I::Unary(dst, op, src) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, dslot, T0);
            let rs = to_reg(bytecode, assignment[src], T1);

            let src_size = gl.vars.size(*src);

            use UnaryOp as O;
            if let Some(src_size) = SixBitSize::from_vector_size(src_size) {
                match op {
                    O::Neg => bytecode.not(rd, rs, src_size),
                    O::ReduceOr => bytecode.cnei(rd, rs, 0, src_size),
                    O::ReduceAnd => bytecode.ceqi(rd, rs, -1, src_size),
                    O::ReduceXor => {
                        bytecode.count_ones(rd, rs);
                        bytecode.truncate(rd, rd, SixBitSize::SCALAR);
                    }
                    O::LeadingZeros | O::TvToFv | O::FvToTv => todo!(),
                }
            } else {
                todo!()
            }

            store_back(bytecode, dslot, rd);
        }
        I::Resize(dst, op, src) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, dslot, T0);
            let rs = to_reg(bytecode, assignment[src], T1);

            let dst_size = gl.vars.size(*dst);
            let src_size = gl.vars.size(*src);

            use ResizeOp as O;
            match (
                op,
                SixBitSize::from_vector_size(dst_size),
                SixBitSize::from_vector_size(src_size),
            ) {
                (O::Truncate, Some(dst_size), Some(_)) => {
                    bytecode.truncate(rd, rs, dst_size);
                }
                (O::Truncate, ..) => {
                    todo!()
                }
                (O::ZeroExtend, Some(_), Some(_)) => {
                    bytecode.copy(rd, rs);
                }
                (O::ZeroExtend, ..) => todo!(),
                (O::SignExtend, ..) => todo!(),
            }

            store_back(bytecode, dslot, rd);
        }
        I::Binary(dst, op, lhs, rhs) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, dslot, T0);
            let rs1 = to_reg(bytecode, assignment[lhs], T1);
            let rs2 = to_reg(bytecode, assignment[rhs], T2);

            let dst_size = gl.vars.size(*dst);

            use BinaryOp as O;
            match (op, SixBitSize::from_vector_size(dst_size)) {
                (O::And, Some(_)) => bytecode.and(rd, rs1, rs2),
                (O::And, None) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::And, size)
                }
                (O::Or, Some(_)) => bytecode.or(rd, rs1, rs2),
                (O::Or, None) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Or, size)
                },
                (O::Xor, Some(_)) => bytecode.xor(rd, rs1, rs2),
                (O::Xor, None) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Xor, size)
                },
                (O::Add, Some(size)) => bytecode.add(rd, rs1, rs2, size),
                (O::Add, None) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Add, size)
                },
                (O::Sub, Some(size)) => bytecode.sub(rd, rs1, rs2, size),
                (O::Sub, None) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Sub, size)
                },
                (O::Multiply, Some(size)) => bytecode.mul(rd, rs1, rs2, size),
                (O::Multiply, None) => {
                    let size = to_8bit_size(bytecode, dst_size);
                    bytecode.tv_heap_bitwise(rd, rs1, rs2, BitwiseOp::Mul, size)
                },
                (O::Power, _) => todo!(),
                (O::Divide, _) => todo!(),
                (O::Modulus, _) => todo!(),
                (O::UnsignedLessEqual, _) => todo!(),
                (O::LogicalShiftLeft, _) => todo!(),
                (O::LogicalShiftRight, _) => todo!(),
                (O::ArithmeticShiftRight, _) => todo!(),
                (O::Concat, _) => todo!(),
                (O::CopyX, _) => todo!(),
                (O::CopyZ, _) => todo!(),
                (O::Min, _) => todo!(),
                (O::Max, _) => todo!(),
                (O::CaseEquality, _) => todo!(),
                (O::Posedge, _) => todo!(),
                (O::Negedge, _) => todo!(),
            }
        }
        I::BinaryImm(variable_key, binary_imm_op, variable_key1, bits) => todo!(),
        I::Slice(variable_key, variable_key1, variable_key2) => todo!(),
        I::SliceImm(variable_key, variable_key1, _) => todo!(),
        I::ShiftImm(variable_key, shift_imm_op, variable_key1, _) => todo!(),
        I::Select(variable_key, variable_key1, variable_key2, variable_key3) => todo!(),
        I::Intrinsic(variable_key, intrinsic_op, items) => todo!(),
        I::LastUpdateTime(variable_key, signal_key) => todo!(),
        I::Probe(dst, signal, offset) => {
            let dslot = assignment[dst];
            let rd = to_reg(bytecode, dslot, T0);

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
            bytecode.load_aligned(rd, roff, 0, signal_size);

            let dst_size = SixBitSize::from_vector_size(dst_size).unwrap();
            if dst_size != signal_size {
                bytecode.mask(rd, rd, dst_size);
            }
            store_back(bytecode, dslot, rd);
        }
        I::ProbeSlice(variable_key, signal_key, variable_key1) => todo!(),
        I::Drive(signal, src, partial) => {
            let rs = to_reg(bytecode, assignment[src], T0);

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
            bytecode.set_aligned(rpoke, rs, roff, 0, signal_size);

            for index in watch_map.watch_indices(*signal) {
                // @TODO: This should have some register based fallback.
                assert!(index < (1 << 20));
                bytecode.wake(rpoke, index as u32);
            }
        }
        I::Phi(variable_key, items) => todo!(),
    }
}

fn to_reg(bytecode: &mut BytecodeEncoder, slot: Slot, backup: Reg) -> Reg {
    match slot {
        Slot::Stack(..) => todo!(),
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
