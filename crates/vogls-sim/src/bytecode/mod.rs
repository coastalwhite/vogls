use std::fmt::{self, Debug};

mod control_flow;
mod extend;
mod heap_ops;
mod interrupt;
mod intrinsics;
mod itype_binary;
mod load;
mod load_imm;
pub mod lower;
mod reg;
mod rtype_binary;
mod rtype_unary;
mod set;
mod stack;
mod temporal;

use reg::{Reg, Regs};
use vogls_bits::BitsDataRef;
use vogls_codegen::{HeapOffset, HeapRef};

use vogls_ir::{Bits, IntrinsicOp, LogicMode, VSIZE_64, VectorSize};
use vogls_runtime::RuntimeState;
use vogls_runtime::plugins::{RuntimePlugin, RuntimePluginState};
use vogls_utils::{IndexSet, NonMaxU32};

pub use control_flow::*;
pub use extend::*;
pub use heap_ops::*;
pub use interrupt::*;
pub use intrinsics::*;
pub use itype_binary::*;
pub use load::*;
pub use load_imm::*;
pub use rtype_binary::*;
pub use rtype_unary::*;
pub use set::*;
pub use stack::*;
pub use temporal::*;

pub struct Design {
    pub bytecode: Vec<Bytecode>,
    pub intrinsics: Vec<IntrinsicOp>,
    pub stack_offset: u64,
    pub itrace: bool,
}
pub struct State {
    pub runtime: RuntimeState,
    pub plugins: Vec<RuntimePluginState>,
    pub schedule: Schedule,
    pub listeners: BytecodeListeners,
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            plugins: self
                .plugins
                .iter()
                .map(|p| RuntimePlugin::clone(p.as_ref()))
                .collect(),
            schedule: self.schedule.clone(),
            listeners: self.listeners.clone(),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bytecode(pub u32);

impl fmt::Display for SixBitSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&if self.0 == 0 { 64 } else { self.0 }, f)
    }
}

pub struct ColdContext<'a> {
    stack: Vec<u64>,
    stack_args: Vec<(VectorSize, LogicMode)>,

    intrinsics: &'a [IntrinsicOp],

    stdout: &'a mut (dyn std::io::Write + Send + Sync),
    stderr: &'a mut (dyn std::io::Write + Send + Sync),

    return_value: u32,
}

impl<'a> ColdContext<'a> {
    pub fn new(
        intrinsics: &'a [IntrinsicOp],
        stdout: &'a mut (dyn std::io::Write + Send + Sync),
        stderr: &'a mut (dyn std::io::Write + Send + Sync),
    ) -> Self {
        Self {
            stack: Vec::new(),
            stack_args: Vec::new(),
            intrinsics,
            stdout,
            stderr,
            return_value: 0,
        }
    }
}

impl Bytecode {
    fn opcode(self) -> u8 {
        (self.0 & 0xFF) as u8
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct SixBitSize(u8);

impl From<SixBitSize> for VectorSize {
    fn from(value: SixBitSize) -> Self {
        VectorSize::new(value.0.into()).unwrap_or(VSIZE_64)
    }
}

impl SixBitSize {
    pub const SCALAR: Self = Self(1);
    pub const N32: Self = Self(32);
    pub const N64: Self = Self(0);

    pub fn from_vector_size(size: VectorSize) -> Option<Self> {
        if size.get() > 64 {
            return None;
        }

        Some(Self((size.get() % 64) as u8))
    }

    pub fn new_masked(v: u32) -> Self {
        Self((v & 0x3F) as u8)
    }

    pub fn mask(self, v: u64) -> u64 {
        let shift = if self.0 == 0 { 64 } else { self.0 as u32 };
        v & 1u64.unbounded_shl(shift).wrapping_sub(1)
    }

    fn get(&self) -> u8 {
        if self.0 == 0 { 64 } else { self.0 }
    }
}

pub trait BytecodeInstruction: Sized {
    fn extract(v: Bytecode) -> Self;
    fn encode(&self) -> Bytecode;
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        regs: &Regs,
        state: &RuntimeState,
    ) -> fmt::Result {
        _ = (f, regs, state);
        Ok(())
    }
    fn post_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        regs: &Regs,
        state: &RuntimeState,
    ) -> fmt::Result {
        _ = (f, regs, state);
        Ok(())
    }
    fn execute(
        self,
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    );
}

fn extract_and_execute<I: BytecodeInstruction>(
    c: Bytecode,
    regs: &mut Regs,
    pc: u64,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    cldctx: &mut ColdContext,
) -> u64 {
    let slf = I::extract(c);
    let mut pc = pc + 1;
    slf.execute(regs, &mut pc, state, schedule, listeners, cldctx);
    pc
}

fn extract_and_pre_exec_itrace<I: BytecodeInstruction>(
    c: Bytecode,
    regs: &Regs,
    pc: u64,
    state: &RuntimeState,
    schedule: &Schedule,
    listeners: &BytecodeListeners,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    _ = (pc, schedule, listeners);
    let slf = I::extract(c);
    slf.pre_exec_itrace(f, regs, state)
}

fn extract_and_post_exec_itrace<I: BytecodeInstruction>(
    c: Bytecode,
    regs: &Regs,
    pc: u64,
    state: &RuntimeState,
    schedule: &Schedule,
    listeners: &BytecodeListeners,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    _ = (pc, schedule, listeners);
    let slf = I::extract(c);
    slf.post_exec_itrace(f, regs, state)
}

macro_rules! opcodes {
    ($($name:ident),+ $(,)?) => {
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum BytecodeOpcode {
            $($name,)+
        }

        const X_NUM_INSTRUCTIONS: usize = {
            0 $(+
            { _ = BytecodeOpcode::$name; 1 }
            )+
        };

        static X_INSTRUCTION_FNS: [
            fn(
                c: Bytecode,
                regs: &mut Regs,
                pc: u64,
                state: &mut RuntimeState,
                schedule: &mut Schedule,
                listeners: &mut BytecodeListeners,
                cldctx: &mut ColdContext,
            ) -> u64;
            X_NUM_INSTRUCTIONS
        ] = [$(extract_and_execute::<$name>),+];
        static X_PRE_EXEC_ITRACE_FNS: [
            fn(
                c: Bytecode,
                regs: &Regs,
                pc: u64,
                state: &RuntimeState,
                schedule: &Schedule,
                listeners: &BytecodeListeners,
                f: &mut fmt::Formatter<'_>,
            ) -> fmt::Result;
            X_NUM_INSTRUCTIONS
        ] = [$(extract_and_pre_exec_itrace::<$name>),+];
        static X_POST_EXEC_ITRACE_FNS: [
            fn(
                c: Bytecode,
                regs: &Regs,
                pc: u64,
                state: &RuntimeState,
                schedule: &Schedule,
                listeners: &BytecodeListeners,
                f: &mut fmt::Formatter<'_>,
            ) -> fmt::Result;
            X_NUM_INSTRUCTIONS
        ] = [$(extract_and_post_exec_itrace::<$name>),+];

        impl TryFrom<u8> for BytecodeOpcode {
            type Error = ();
            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $(_ if value == BytecodeOpcode::$name as u8 => Ok(BytecodeOpcode::$name),)*
                    _ => Err(()),
                }
            }
        }

        impl fmt::Display for Bytecode {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Ok(kind) = BytecodeOpcode::try_from(self.opcode()) else {
                    return f.write_str("unknown opcode");
                };

                match kind {
                    $(BytecodeOpcode::$name => {
                        let v = <$name as BytecodeInstruction>::extract(*self);
                        BytecodeInstruction::fmt(&v, f)?;
                    })*
                }

                Ok(())
            }
        }

    };
}

opcodes![
    Interrupt,
    CMov,
    TvAnd,
    TvOr,
    TvXor,
    TvAndNot,
    TvOrNot,
    TvXnor,
    TvCeq,
    TvAdd,
    TvSub,
    TvMul,
    TvDivX,
    TvDiv0,
    TvModX,
    TvMod0,
    TvPow,
    TvUnsignedLeq,
    TvUnsignedGt,
    TvMin,
    TvMax,
    TvSll,
    TvSlr,
    TvSar,
    TvLeftShiftOr,
    TvCountOnes,
    TvAndi,
    TvOri,
    TvXori,
    TvAddi,
    TvSubi,
    TvMuli,
    TvRevSubi,
    TvMini,
    TvMaxi,
    TvUleqi,
    TvUgti,
    TvCeqi,
    TvCnei,
    TvSlli,
    TvSlri,
    TvSari,
    FvAnd,
    FvOr,
    FvXor,
    FvAndNot,
    FvOrNot,
    FvCeq,
    FvPosedge,
    FvNegedge,
    FvAdd,
    FvSub,
    FvMul,
    FvDivX,
    FvDiv0,
    FvModX,
    FvMod0,
    FvPow,
    FvUnsignedLeq,
    FvUnsignedGt,
    FvMin,
    FvMax,
    FvSll,
    FvSlr,
    FvSar,
    FvLeftShiftOr,
    FvNot,
    FvReduceAnd,
    FvReduceOr,
    FvReduceXor,
    FvAndi,
    FvOri,
    FvXori,
    FvAddi,
    FvSubi,
    FvMuli,
    FvRevSubi,
    FvMini,
    FvMaxi,
    FvUleqi,
    FvUgti,
    FvCeqi,
    FvCnei,
    FvSlli,
    FvSlri,
    FvSari,
    PushArgument,
    Intrinsic,
    SignExtend,
    StackOffset,
    LoadImm,
    Jump,
    RelJump,
    Branch,
    TvSetAligned,
    TvSetHeapAligned,
    FvSetAligned,
    FvSetHeapAligned,
    SetUnaligned,
    SetHeapUnaligned,
    TvLoadAligned,
    FvLoadAligned,
    LoadUnaligned,
    LoadHeapAligned,
    LoadHeapUnaligned,
    Wake,
    Reschedule,
    StartListen,
    LastUpdateTime,
    SetLupdt,
    TvCorrectFirst,
    HeapHeapExtend,
    HeapRegExtend,
    HeapHeapTruncate,
    HeapBinaryBitwise,
    HeapBinaryArithmetic,
    HeapBinaryCmp,
    HeapBinaryMinMax,
    HeapBinaryShift,
    HeapCaseEq,
    HeapUnary,
    LoadSize,
];

#[derive(Clone)]
pub struct BytecodeListeners {
    map: Vec<InstructionPtr>,
    active: Vec<u64>,
}

struct LoadSize(Option<VectorSize>);

impl BytecodeInstruction for LoadSize {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::LoadSize as u8);
        Self(VectorSize::new(v.0 >> 8))
    }

    fn encode(&self) -> Bytecode {
        Bytecode(BytecodeOpcode::LoadSize as u32 | (self.0.map_or(0, |v| v.get()) << 8))
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_padded_mnemonic(f, "load_size")?;
        match self.0 {
            Some(size) => fmt::Display::fmt(&size, f),
            None => todo!(),
        }
    }

    fn execute(
        self,
        regs: &mut Regs,
        _pc: &mut u64,
        _state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        match self.0 {
            None => todo!(),
            Some(size) => regs.size = size,
        }
    }
}

#[derive(Clone, Copy)]
pub struct InlineNBitSize<const N: usize>(Option<VectorSize>);

impl<const N: usize> InlineNBitSize<N> {
    pub fn get(self, regs: &Regs) -> VectorSize {
        match self.0 {
            None => regs.size,
            Some(s) => s,
        }
    }

    pub fn new_masked(v: u32) -> Self {
        Self(VectorSize::new(
            v & 1u32.unbounded_shl(N as u32).wrapping_sub(1),
        ))
    }

    pub fn encode(self) -> u32 {
        self.0.map_or(0, |v| v.get())
    }

    pub fn new(size: VectorSize, bce: &mut BytecodeEncoder) -> Self {
        if size.get() < (1u32 << N) {
            return Self(Some(size));
        }

        if size.get() >= (1u32 << 24) {
            todo!();
        }
        bce.data.push(LoadSize(Some(size)).encode());
        Self(None)
    }
}

impl<const NBITS: usize> fmt::Display for InlineNBitSize<NBITS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            None => f.write_str("|...|"),
            Some(size) => fmt::Display::fmt(&size, f),
        }
    }
}

#[derive(Clone, Copy)]
pub struct InlineAddrOffset<const NBITS: usize>(i16);

impl<const NBITS: usize> InlineAddrOffset<NBITS> {
    const ZERO: Self = Self(0);

    #[inline(always)]
    pub fn get(self, base: u64) -> u64 {
        base.wrapping_add_signed(i64::from(self.0))
    }

    #[inline(always)]
    pub fn new_shifted(v: u32, offset: u32) -> Self {
        Self(((v as i32) >> offset) as i16)
    }

    pub fn encode(self) -> u32 {
        i32::from(self.0) as u32 & 1u32.unbounded_shl(NBITS as u32).wrapping_sub(1)
    }

    pub fn new(offset: i64, bce: &mut BytecodeEncoder, addr: Reg, scratch: Reg) -> (Reg, Self) {
        const { assert!(NBITS >= 1 && NBITS <= 16) };
        let min: i64 = const { -(1 << (NBITS - 1)) };
        let max: i64 = const { (1 << (NBITS - 1)) - 1 };

        if (min..=max).contains(&offset) {
            return (addr, Self(offset as i16));
        }

        // @Performance: There are quite a few tricks that we can pull here to make a more
        // efficient lowering.
        bce.load_u64(scratch, offset as u64);
        bce.add(scratch, addr, scratch, SixBitSize::N64);
        (scratch, Self(0))
    }
}

impl<const NBITS: usize> fmt::Display for InlineAddrOffset<NBITS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Clone, Copy)]
pub struct SignedImmediate<const NBITS: usize>(i32);

impl<const NBITS: usize> SignedImmediate<NBITS> {
    const ZERO: Self = Self(0);
    const MINUS_ONE: Self = Self(-1);

    #[inline(always)]
    pub fn new_shifted(v: u32, offset: u32) -> Self {
        Self((v as i32) >> offset)
    }

    pub fn encode(self) -> u32 {
        (self.0 as u32) & 1u32.unbounded_shl(NBITS as u32).wrapping_sub(1)
    }

    pub fn new(value: i64) -> Option<Self> {
        const { assert!(NBITS >= 1 && NBITS <= 32) };
        let min: i64 = const { -(1 << (NBITS - 1)) };
        let max: i64 = const { (1 << (NBITS - 1)) - 1 };

        if (min..=max).contains(&value) {
            return Some(Self(value as i32));
        }

        None
    }

    pub fn new_from_u64(value: u64) -> Option<Self> {
        Self::new(value as i64)
    }

    pub fn new_from_bits(value: &Bits) -> Option<Self> {
        if value.size() > VSIZE_64 || value.contains_special() {
            return None;
        }

        match value.as_data_ref() {
            BitsDataRef::InlineTv(v) => Self::new_from_u64(v),
            BitsDataRef::InlineFv(_spc, v) => Self::new_from_u64(v),
            _ => None,
        }
    }

    fn get_unsigned(&self) -> u32 {
        self.0 as u32
    }
}

#[derive(Clone, Copy)]
pub struct InlineIndex<const NBITS: usize>(Option<NonMaxU32>);

impl<const NBITS: usize> InlineIndex<NBITS> {
    #[inline(always)]
    pub fn new_shifted(v: u32, offset: u32) -> Self {
        Self(NonMaxU32::new(v >> offset))
    }

    pub fn encode(self) -> u32 {
        self.0
            .map_or(1u32.unbounded_shl(NBITS as u32).wrapping_sub(1), |v| {
                v.get()
            })
    }

    pub fn new(value: u64, bce: &mut BytecodeEncoder, reg: Reg) -> Self {
        const { assert!(NBITS >= 1 && NBITS <= 32) };
        if value < ((1 << NBITS) - 1) {
            Self(Some(NonMaxU32::new(value as u32).unwrap()))
        } else {
            bce.load_u64(reg, value);
            Self(None)
        }
    }

    pub fn get(self, regs: &Regs, reg: Reg) -> u64 {
        self.0.map_or_else(|| regs[reg], |v| v.get() as u64)
    }
}

impl<const NBITS: usize> fmt::Display for InlineIndex<NBITS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            None => f.write_str("|...|"),
            Some(n) => fmt::Display::fmt(&n, f),
        }
    }
}

impl<const NBITS: usize> fmt::Display for SignedImmediate<NBITS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl BytecodeListeners {
    pub fn new(num_watches: usize) -> Self {
        Self {
            map: vec![InstructionPtr(u64::MAX); num_watches],
            active: vec![0u64; num_watches.div_ceil(64)],
        }
    }

    pub fn set_ptr(&mut self, index: usize, ptr: InstructionPtr) {
        self.map[index] = ptr;
    }
}

#[derive(Clone, Copy)]
pub struct InstructionPtr(pub u64);

#[derive(Clone)]
pub struct TimedEvent {
    time: u64,
    pc: InstructionPtr,
}

#[derive(Clone)]
pub struct Schedule {
    active: Vec<InstructionPtr>,
    regions: Box<[Vec<InstructionPtr>]>,
    future: Vec<TimedEvent>,
    next_time: u64,
}

impl Schedule {
    pub fn new(num_regions: u8) -> Self {
        Self {
            active: Vec::new(),
            regions: vec![Vec::new(); num_regions as usize].into_boxed_slice(),
            future: Vec::new(),
            next_time: u64::MAX,
        }
    }

    pub fn push_active(&mut self, ptr: InstructionPtr) {
        self.active.push(ptr);
    }

    pub fn pop(&mut self, time: &mut u64) -> Option<InstructionPtr> {
        if let Some(pc) = self.active.pop() {
            return Some(pc);
        }

        'fill_active: {
            for region in &mut self.regions {
                if region.len() > 0 {
                    std::mem::swap(&mut self.active, region);
                    break 'fill_active;
                }
            }

            *time = self.next_time;
            let mut next_time = u64::MAX;
            self.active.extend(
                self.future
                    .extract_if(.., |te| {
                        let is_next_timestep = te.time == self.next_time;
                        if !is_next_timestep {
                            next_time = te.time.min(next_time);
                        }
                        is_next_timestep
                    })
                    .map(|te| te.pc),
            );

            self.next_time = next_time;
        };

        self.active.pop()
    }
}

#[derive(Default)]
pub struct BytecodeEncoder {
    pub data: Vec<Bytecode>,
    pub intrinsics: IndexSet<IntrinsicOpEqWrap>,
}

#[repr(transparent)]
pub struct IntrinsicOpEqWrap(pub IntrinsicOp);

impl std::hash::Hash for IntrinsicOpEqWrap {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(&self.0);
        match &self.0 {
            IntrinsicOp::Time | IntrinsicOp::Finish | IntrinsicOp::Random => {}
            IntrinsicOp::Display(f) | IntrinsicOp::Assert(f) => f.hash(state),
            IntrinsicOp::VcdOpenFile(_) | IntrinsicOp::VcdAppendModule(_) => {}
            IntrinsicOp::VcdPause | IntrinsicOp::VcdResume | IntrinsicOp::ReadMem(_) => {}
        }
    }
}
impl PartialEq for IntrinsicOpEqWrap {
    fn eq(&self, other: &Self) -> bool {
        if std::mem::discriminant(&self.0) != std::mem::discriminant(&other.0) {
            return false;
        }
        match (&self.0, &other.0) {
            (IntrinsicOp::Time | IntrinsicOp::Finish | IntrinsicOp::Random, _) => true,
            (
                IntrinsicOp::Display(f) | IntrinsicOp::Assert(f),
                IntrinsicOp::Display(fo) | IntrinsicOp::Assert(fo),
            ) => f == fo,
            (IntrinsicOp::VcdOpenFile(_) | IntrinsicOp::VcdAppendModule(_), _) => false,
            (IntrinsicOp::VcdPause | IntrinsicOp::VcdResume | IntrinsicOp::ReadMem(_), _) => false,
            _ => unreachable!(),
        }
    }
}
impl Eq for IntrinsicOpEqWrap {}

impl BytecodeEncoder {
    pub fn current_ptr(&self) -> InstructionPtr {
        InstructionPtr(self.data.len() as u64)
    }

    pub fn not(&mut self, rd: Reg, rs: Reg, size: SixBitSize) {
        self.xori(rd, rs, SignedImmediate::MINUS_ONE, size);
    }
    pub fn copy(&mut self, rd: Reg, rs: Reg) {
        if rd == rs {
            return;
        }
        self.ori(rd, rs, SignedImmediate::ZERO, SixBitSize::N64);
    }
    pub fn fv_copy(&mut self, rd: Reg, rs: Reg) {
        if rd == rs {
            return;
        }
        let (rdspc, rdval) = rd.to_spc_and_val();
        let (rsspc, rsval) = rd.to_spc_and_val();
        self.copy(rdspc, rsspc);
        self.copy(rdval, rsval);
    }
    pub fn truncate(&mut self, rd: Reg, rs: Reg, size: SixBitSize) {
        self.andi(rd, rs, SignedImmediate::MINUS_ONE, size);
    }
    pub fn load_u64(&mut self, rd: Reg, value: u64) {
        if value == 0 {
            self.xor(rd, rd, rd);
            return;
        }

        if value == u64::MAX {
            self.ori(rd, rd, SignedImmediate::MINUS_ONE, SixBitSize::N64);
            return;
        }

        // @Performance: Do something smart with the sign extend here for negative integers.
        let mut clear = true;
        for segment in 0..4 {
            let s = (value >> (segment * 16)) & 0xFFFF;
            if s != 0 {
                self.load_imm16(rd, clear, false, segment, s as u16 as i16);
                clear = false;
            }
        }
    }
    pub fn load_bits_into_register(&mut self, rd: Reg, mode: LogicMode, value: &Bits) {
        let size = SixBitSize::from_vector_size(value.size()).expect("Does not fit in register");
        if value.contains_special() {
            assert_eq!(mode, LogicMode::FourValue);
        }

        match (value.as_data_ref(), mode) {
            (BitsDataRef::InlineTv(v), LogicMode::TwoValue) => self.load_u64(rd, v),
            (BitsDataRef::InlineTv(v), LogicMode::FourValue) => {
                let (rdspc, rdval) = rd.to_spc_and_val();
                self.load_u64(rdspc, size.mask(u64::MAX));
                self.load_u64(rdval, v);
            }

            (BitsDataRef::SeparateTv(_), _) => unreachable!(),
            (BitsDataRef::InlineFv(spc, val), LogicMode::FourValue) => {
                let (rdspc, rdval) = rd.to_spc_and_val();
                self.load_u64(rdspc, spc);
                self.load_u64(rdval, val);
            }
            (BitsDataRef::InlineFv(_, val), LogicMode::TwoValue) => {
                self.load_u64(rd, val);
            }
            (BitsDataRef::SeparateFv(items), LogicMode::FourValue) => {
                let (rdspc, rdval) = rd.to_spc_and_val();
                self.load_u64(rdspc, items[0]);
                self.load_u64(rdval, items[1]);
            }
            (BitsDataRef::SeparateFv(items), LogicMode::TwoValue) => {
                self.load_u64(rd, items[1]);
            }
        }
    }
    pub fn mask(&mut self, rd: Reg, rs: Reg, size: SixBitSize) {
        self.andi(rd, rs, SignedImmediate::MINUS_ONE, size)
    }
}

impl Design {
    pub fn execute(
        &self,
        state: &mut State,
        stdout: &mut (dyn std::io::Write + Send + Sync),
        stderr: &mut (dyn std::io::Write + Send + Sync),
    ) -> Result<(), ()> {
        if self.itrace {
            self.execute_inner::<true>(state, stdout, stderr)
        } else {
            self.execute_inner::<false>(state, stdout, stderr)
        }
    }
    fn execute_inner<const ITRACE: bool>(
        &self,
        state: &mut State,
        stdout: &mut (dyn std::io::Write + Send + Sync),
        stderr: &mut (dyn std::io::Write + Send + Sync),
    ) -> Result<(), ()> {
        let code = &self.bytecode;
        let Some(entry) = state.schedule.pop(&mut state.runtime.time) else {
            return Ok(());
        };

        struct DisplayWith<'a>(
            Bytecode,
            &'a Regs,
            u64,
            &'a RuntimeState,
            &'a Schedule,
            &'a BytecodeListeners,
            fn(
                Bytecode,
                &Regs,
                u64,
                &RuntimeState,
                &Schedule,
                &BytecodeListeners,
                &mut fmt::Formatter<'_>,
            ) -> fmt::Result,
        );
        impl<'a> fmt::Display for DisplayWith<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                (self.6)(self.0, self.1, self.2, self.3, self.4, self.5, f)
            }
        }

        let mut pc = entry.0;
        let mut cldctx = ColdContext::new(&self.intrinsics, stdout, stderr);
        let mut regs = Regs::new(self.stack_offset);
        while let Some(&c) = code.get(pc as usize) {
            let opcode = c.opcode();
            if ITRACE {
                writeln!(&mut cldctx.stderr, "[PC={pc:4}]: {c}").unwrap();
                write!(
                    &mut cldctx.stderr,
                    "{}",
                    DisplayWith(
                        c,
                        &regs,
                        pc,
                        &state.runtime,
                        &state.schedule,
                        &state.listeners,
                        X_PRE_EXEC_ITRACE_FNS[opcode as usize]
                    )
                )
                .unwrap();
            }
            let f = X_INSTRUCTION_FNS[opcode as usize];
            pc = (f)(
                c,
                &mut regs,
                pc,
                &mut state.runtime,
                &mut state.schedule,
                &mut state.listeners,
                &mut cldctx,
            );
            if ITRACE {
                write!(
                    &mut cldctx.stderr,
                    "{}",
                    DisplayWith(
                        c,
                        &regs,
                        pc,
                        &state.runtime,
                        &state.schedule,
                        &state.listeners,
                        X_POST_EXEC_ITRACE_FNS[opcode as usize]
                    )
                )
                .unwrap();
            }
        }

        if cldctx.return_value != 0 {
            return Err(());
        }

        Ok(())
    }
}

fn value_to_heap_ref(value: u64, size: VectorSize, mode: LogicMode) -> HeapRef {
    let alignment = match mode {
        LogicMode::TwoValue => size.get().next_power_of_two().min(64),
        LogicMode::FourValue => (size.get() * 2).next_power_of_two().min(64),
    } as u64;
    assert_eq!(
        value % alignment,
        0,
        "bit_offset ({value}) is not aligned to {alignment}"
    );
    let bit_offset = value as usize;
    HeapOffset { bit_offset }.to_ref(size)
}

const MNEMONIC_ALIGN: usize = 22;
const EXEC_ITRACE_INDENT: &str = "  ";

fn write_padded_mnemonic(f: &mut fmt::Formatter<'_>, mnemonic: &str) -> fmt::Result {
    write!(f, "{mnemonic:<0$}", MNEMONIC_ALIGN)
}

fn write_register(
    f: &mut fmt::Formatter<'_>,
    regs: &Regs,
    name: &str,
    reg: Reg,
    mode: LogicMode,
) -> fmt::Result {
    match mode {
        LogicMode::TwoValue => write!(f, "{name} = 0x{:x}", regs[reg])?,
        LogicMode::FourValue => {
            let (spc, val) = reg.to_spc_and_val();
            let spc = regs[spc];
            let val = regs[val];
            let bits =
                Bits::from_boxed_slice(vogls_ir::Mode::FourValue, VSIZE_64, [spc, val].into());
            write!(f, "{name} = {bits} (0x{spc:x}, 0x{val:x})")?;
        }
    }

    Ok(())
}
