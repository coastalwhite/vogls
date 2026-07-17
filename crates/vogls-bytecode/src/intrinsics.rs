use std::fmt;

use vogls_bits::arithmetic::FvLogicValue;
use vogls_bits::format::BitsFormatOptions;
use vogls_codegen::HeapOffset;
use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::vcd::VcdVariableKey;
use vogls_ir::{Bits, LogicMode, Mode, SCALAR_VSIZE, SignalSlice, VSIZE_32, VSIZE_64, VectorSize};
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_utils::{NonMaxU16, SecondaryTable};
use vogls_vcd::VcdScopeItem;

use crate::{
    BytecodeOpcode, EXEC_ITRACE_INDENT, MNEMONIC_ALIGN, value_to_heap_ref, write_padded_mnemonic,
};

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, ColdContext, InlineNBitSize,
    Schedule, write_register,
};

pub struct PushArgument {
    rs: Reg,
    mode: LogicMode,
    size: InlineNBitSize<19>,
}

pub struct Intrinsic {
    rd: Reg,
    id: Option<NonMaxU16>,
}

pub struct ReadMem {
    pub offset: u64,
    pub size: VectorSize,
    pub mode: LogicMode,
    pub inner: vogls_ir::ReadMem,
}

pub enum IntrinsicOp {
    Time,
    Finish,
    Random(VectorSize, LogicMode),
    Display(Box<DynFormatString>),
    Assert(Box<DynFormatString>),
    VcdOpenFile(String),
    VcdAppendModule(
        Box<(
            Vec<VcdScopeItem>,
            SecondaryTable<RtSignalKey, Box<[(VcdVariableKey, Option<SignalSlice>)]>>,
        )>,
    ),
    VcdPause,
    VcdResume,
    ReadMem(Box<ReadMem>),
}

impl BytecodeInstruction for PushArgument {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::PushArgument as u8);
        let v = v.0;
        Self {
            rs: Reg::new_masked(v >> 8),
            mode: match (v >> 12) & 1 {
                0 => LogicMode::TwoValue,
                _ => LogicMode::FourValue,
            },
            size: InlineNBitSize::new_masked(v >> 13),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            (BytecodeOpcode::PushArgument as u32)
                | ((self.rs as u32) << 8)
                | ((self.mode as u32) << 12)
                | (self.size.encode() << 13),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { size, mode, rs } = self;
        write_padded_mnemonic(f, "push_argument")?;
        write!(f, "{rs}, {mode:?}, {size}")
    }

    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        let size = self.size.get(regs);
        if size > VSIZE_64 {
            let rs = state
                .heap
                .load_bits(regs.get_as_addr(self.rs).to_ref(size), self.mode);
            write!(f, "rs = {}", rs.display(&BitsFormatOptions::default()))?;
        } else {
            write_register(f, regs, "rs", self.rs, self.mode)?;
        }
        writeln!(f)?;
        Ok(())
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { size, mode, rs } = self;
        let size = size.get(regs);
        cldctx.stack_args.push((size, mode));
        match mode {
            LogicMode::FourValue if size <= VSIZE_64 => {
                let (spc, val) = rs.to_spc_and_val();
                cldctx.stack.push(regs[spc]);
                cldctx.stack.push(regs[val]);
            }
            LogicMode::TwoValue | LogicMode::FourValue => cldctx.stack.push(regs[rs]),
        }
    }
}

impl BytecodeInstruction for Intrinsic {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::Intrinsic as u8);
        let v = v.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            id: NonMaxU16::new((v >> 16) as u16),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            (BytecodeOpcode::Intrinsic as u32)
                | ((self.rd as u32) << 8)
                | (self.id.map_or(0u32, |v| v.get() as u32) << 16),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:<1$}", "intrinsic", MNEMONIC_ALIGN)
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rd, id } = self;
        let id = id.map_or_else(|| regs[Reg::X14], |v| v.get() as u64);
        let intrinsic = &cldctx.intrinsics[id as usize];

        use IntrinsicOp as O;
        match intrinsic {
            O::Time => regs[rd] = state.time,
            O::Finish => {
                cldctx.return_value = 0;
                *pc = u64::MAX;
                cldctx.stdout.write_all(b"[FINISH]\n").unwrap();
            }
            O::Random(..) => todo!(),
            O::Display(f) => {
                let mut stack_offset = 0;
                f.write_to(
                    &mut cldctx.stdout,
                    cldctx.stack_args.iter().map(|&(size, mode)| match mode {
                        LogicMode::TwoValue if size <= VSIZE_64 => {
                            let value = cldctx.stack[stack_offset];
                            stack_offset += 1;
                            Bits::from_u64(size, value)
                        }
                        LogicMode::TwoValue => {
                            let value = cldctx.stack[stack_offset];
                            let value = value_to_heap_ref(value, size, mode);
                            stack_offset += 1;
                            state.heap.load_tv_bits(value)
                        }
                        LogicMode::FourValue if size <= VSIZE_32 => {
                            let spc = cldctx.stack[stack_offset];
                            let val = cldctx.stack[stack_offset + 1];
                            stack_offset += 2;
                            Bits::from_four_value_u64(size, spc as u32, val as u32)
                        }
                        LogicMode::FourValue if size <= VSIZE_64 => {
                            let spc = cldctx.stack[stack_offset];
                            let val = cldctx.stack[stack_offset + 1];
                            stack_offset += 2;
                            Bits::from_boxed_slice(Mode::FourValue, size, [spc, val].into())
                        }
                        LogicMode::FourValue => {
                            let value = cldctx.stack[stack_offset];
                            let value = value_to_heap_ref(value, size, mode);
                            stack_offset += 1;
                            state.heap.load_fv_bits(value)
                        }
                    }),
                )
                .unwrap();
            }
            O::Assert(f) => {
                let mut stack_offset = 0usize;
                let (cond_size, cond_mode) = cldctx.stack_args[0];
                assert_eq!(cond_size, SCALAR_VSIZE);
                let condition = match cond_mode {
                    LogicMode::TwoValue => {
                        stack_offset += 1;
                        cldctx.stack[0] != 0
                    }
                    LogicMode::FourValue => {
                        stack_offset += 2;
                        FvLogicValue::from_spc_and_val(cldctx.stack[0] != 0, cldctx.stack[1] != 0)
                            == FvLogicValue::L1
                    }
                };

                if !condition {
                    f.write_to(
                        &mut cldctx.stderr,
                        cldctx.stack_args[1..]
                            .iter()
                            .map(|&(size, mode)| match mode {
                                LogicMode::TwoValue if size <= VSIZE_64 => {
                                    let value = cldctx.stack[stack_offset];
                                    stack_offset += 1;
                                    Bits::from_u64(size, value)
                                }
                                LogicMode::TwoValue => {
                                    let value = cldctx.stack[stack_offset];
                                    let value = value_to_heap_ref(value, size, mode);
                                    stack_offset += 1;
                                    state.heap.load_tv_bits(value)
                                }
                                LogicMode::FourValue if size <= VSIZE_32 => {
                                    let spc = cldctx.stack[stack_offset];
                                    let val = cldctx.stack[stack_offset + 1];
                                    stack_offset += 2;
                                    Bits::from_four_value_u64(size, spc as u32, val as u32)
                                }
                                LogicMode::FourValue if size <= VSIZE_64 => {
                                    let spc = cldctx.stack[stack_offset];
                                    let val = cldctx.stack[stack_offset + 1];
                                    stack_offset += 2;
                                    Bits::from_boxed_slice(Mode::FourValue, size, [spc, val].into())
                                }
                                LogicMode::FourValue => {
                                    let value = cldctx.stack[stack_offset];
                                    let value = value_to_heap_ref(value, size, mode);
                                    stack_offset += 1;
                                    state.heap.load_fv_bits(value)
                                }
                            }),
                    )
                    .unwrap();
                    cldctx.return_value = 1;
                    *pc = u64::MAX;
                }
            }
            O::VcdOpenFile(path) => {
                let vcd = (cldctx.plugins[0].as_mut() as &mut dyn std::any::Any)
                    .downcast_mut::<vogls_vcd::RtVcdOutput>()
                    .unwrap();
                if !vcd.children.is_empty() {
                    writeln!(&mut cldctx.stderr, "ERR! VCD opened a second file").unwrap();
                    cldctx.return_value = 1;
                    *pc = u64::MAX;
                    return;
                }

                vcd.writer = Box::new(std::fs::File::create(path).unwrap());
            }
            O::VcdAppendModule(v) => {
                let (children, map) = v.as_ref();
                let vcd = (cldctx.plugins[0].as_mut() as &mut dyn std::any::Any)
                    .downcast_mut::<vogls_vcd::RtVcdOutput>()
                    .unwrap();

                if vcd.start_ts != state.time {
                    writeln!(
                        &mut cldctx.stderr,
                        "ERR! Dumping vars over several simulation times"
                    )
                    .unwrap();
                    cldctx.return_value = 1;
                    *pc = u64::MAX;
                    return;
                }

                for child in children {
                    child.extend_into(&mut vcd.tracked, &mut vcd.updated_this_time_step);
                }
                vcd.children = children.clone();
                vcd.map = map.clone();
            }
            O::VcdPause => {
                let vcd = (cldctx.plugins[0].as_mut() as &mut dyn std::any::Any)
                    .downcast_mut::<vogls_vcd::RtVcdOutput>()
                    .unwrap();
                _ = vcd.paused = true;
            }
            O::VcdResume => {
                let vcd = (cldctx.plugins[0].as_mut() as &mut dyn std::any::Any)
                    .downcast_mut::<vogls_vcd::RtVcdOutput>()
                    .unwrap();
                vcd.paused = false;
            }
            O::ReadMem(readmem) => {
                let ReadMem {
                    offset,
                    size,
                    mode,
                    inner: readmem,
                } = readmem.as_ref();
                let dst = HeapOffset {
                    bit_offset: *offset as usize,
                }
                .to_ref(*size);

                vogls_runtime::readmem::read_mem(
                    &readmem.path,
                    state.heap.0.as_mut(),
                    dst,
                    (*mode).into(),
                    readmem.offset,
                    readmem.limit,
                    readmem.stride,
                    readmem.binary,
                )
                .unwrap()
            }
        }
        cldctx.stack_args.clear();
        cldctx.stack.clear();
    }
}

impl BytecodeEncoder {
    pub fn push_argument(&mut self, size: VectorSize, mode: LogicMode, rs: Reg) {
        let size = InlineNBitSize::new(size, self);
        self.data.push(PushArgument { size, mode, rs }.encode());
    }

    pub fn intrinsic(&mut self, rd: Reg, id: Option<NonMaxU16>) {
        self.data.push(Intrinsic { rd, id }.encode());
    }
}
