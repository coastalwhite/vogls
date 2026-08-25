#![cfg_attr(
    all(nightly, feature = "tailcall"),
    feature(rust_preserve_none_cc, explicit_tail_calls)
)]

use std::fmt::{self, Debug};
use std::path::PathBuf;

mod control_flow;
mod extend;
mod heap_ops;
mod heap_slice;
mod interrupt;
mod intrinsics;
mod itype_binary;
mod load;
mod load_imm;
pub mod lower;
pub mod profile;
mod real;
mod reg;
mod rtype_binary;
mod rtype_unary;
mod set_aligned;
mod set_unaligned;
mod stack;
#[cfg(all(nightly, feature = "tailcall"))]
mod tailcall;
mod temporal;

use reg::{Reg, Regs};
use vogls_bits::BitsDataRef;
use vogls_codegen::{Heap, HeapOffset, HeapRef, SixBitSize};
use vogls_ir::time::TimeResolution;
use vogls_ir::{Bits, LogicMode, VSIZE_64, VectorSize};
use vogls_runtime::plugins::{RuntimePlugin, RuntimePluginState};
use vogls_runtime::{RtSignalKey, RuntimeState};
use vogls_utils::{IndexSet, NonMaxU32};

pub use control_flow::*;
pub use extend::*;
pub use heap_ops::*;
pub use heap_slice::*;
pub use interrupt::*;
pub use intrinsics::*;
pub use itype_binary::*;
pub use load::*;
pub use load_imm::*;
pub use real::*;
pub use rtype_binary::*;
pub use rtype_unary::*;
pub use set_aligned::*;
pub use set_unaligned::*;
pub use stack::*;
pub use temporal::*;
use vogls_world::World;

use std::sync::Arc;

use profile::BytecodeDebugInfo;

use self::reg::{RegInfo, RegStorage};

pub struct Design {
    pub bytecode: Vec<Bytecode>,
    pub intrinsics: Vec<IntrinsicOp>,
    pub stack_offset: u64,
    pub itrace: bool,
    pub stats: bool,
    pub watchers: BytecodeWatchers,
    pub profile: Option<PathBuf>,
    pub debug_info: Option<Arc<BytecodeDebugInfo>>,
    pub time_resolution: TimeResolution,
}
pub struct State {
    pub runtime: RuntimeState,
    pub plugins: Vec<RuntimePluginState>,
    pub schedule: Schedule,
    pub listeners: BytecodeListeners,
}

pub struct BytecodeWatchers {
    pub offsets: Vec<u64>,
    pub watchers: Vec<u64>,
}

impl BytecodeWatchers {
    #[inline(always)]
    pub fn get(&self, index: usize) -> &[u64] {
        let end = index.saturating_add(1);
        assert!(end < self.offsets.len());
        let start = self.offsets[index];
        let end = self.offsets[index.saturating_add(1)];
        &self.watchers[start as usize..end as usize]
    }
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

pub struct ColdContext<'a> {
    stack: Vec<u64>,
    stack_args: Vec<(VectorSize, LogicMode)>,

    intrinsics: &'a [IntrinsicOp],

    world: &'a mut dyn World,

    watchers: &'a BytecodeWatchers,
    plugins: &'a mut [RuntimePluginState],
    heap_scratch: Vec<u64>,

    return_value: u32,
    time_resolution: TimeResolution,
}

impl<'a> ColdContext<'a> {
    pub fn new(
        intrinsics: &'a [IntrinsicOp],
        watchers: &'a BytecodeWatchers,
        plugins: &'a mut [RuntimePluginState],
        world: &'a mut dyn World,
        time_resolution: TimeResolution,
    ) -> Self {
        Self {
            stack: Vec::new(),
            stack_args: Vec::new(),
            intrinsics,
            world,
            watchers,
            heap_scratch: Vec::new(),
            plugins,
            return_value: 0,
            time_resolution,
        }
    }
}

impl Bytecode {
    fn opcode(self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    fn num_additional_slots(self) -> u8 {
        (NUM_ADDITIONAL_SLOTS_FNS[self.opcode() as usize])(self)
    }

    fn unwrap_size(self) -> VectorSize {
        VectorSize::new(self.0).expect("Expected non-zero size")
    }
}

pub trait BytecodeInstruction: Sized {
    /// Extract operands for the encoded bytecode instruction.
    fn extract(v: Bytecode) -> Self;
    /// Encode the operands into a bytecode instruction.
    fn encode(&self) -> Bytecode;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;

    /// Get all the operands that are used as a source in the instruction.
    fn source_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>);
    /// Get all the operands that are used as a destination in the instruction.
    fn dest_operands(&self, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>);

    /// Get the number additional bytecode slots that are used by the instruction.
    ///
    /// Generally, hot instructions only use 1 slot per instruction. However, certain complex
    /// instructions use more than 1 slot to store operand information such as operand sizes,
    /// limiting ranges or offsets.
    fn num_additional_slots(&self) -> u8 {
        0
    }

    /// Perform the instruction on the given state.
    fn execute(
        self,
        code: &[Bytecode],
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
    code: &[Bytecode],
    regs: &mut Regs,
    pc: u64,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    cldctx: &mut ColdContext,
) -> u64 {
    let slf = I::extract(c);
    let mut pc = pc + 1;
    slf.execute(code, regs, &mut pc, state, schedule, listeners, cldctx);
    pc
}

fn extract_and_source_regs<I: BytecodeInstruction>(
    c: Bytecode,
    code: &[Bytecode],
    pc: u64,
    operands: &mut Vec<RegInfo>,
) {
    operands.clear();
    I::extract(c).source_operands(code, pc, operands);
}

fn extract_and_dest_regs<I: BytecodeInstruction>(
    c: Bytecode,
    code: &[Bytecode],
    pc: u64,
    operands: &mut Vec<RegInfo>,
) {
    operands.clear();
    I::extract(c).dest_operands(code, pc, operands);
}

fn extract_and_num_additional_slots<I: BytecodeInstruction>(c: Bytecode) -> u8 {
    I::extract(c).num_additional_slots()
}

type LoopFn = fn(
    c: Bytecode,
    code: &[Bytecode],
    regs: &mut Regs,
    pc: u64,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    cldctx: &mut ColdContext,
) -> u64;
type RegFn = fn(c: Bytecode, code: &[Bytecode], pc: u64, operands: &mut Vec<RegInfo>);

macro_rules! opcodes {
    ($($name:ident),+ $(,)?) => {
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum BytecodeOpcode {
            $($name,)+
        }

        const NUM_INSTRUCTIONS: usize = {
            0 $(+
            { _ = BytecodeOpcode::$name; 1 }
            )+
        };

        static LOOP_INSTR_FNS: [LoopFn; 256] = const {
            // @NOTE: We pad here to 256 such that indexing into it with an u8, does not need a
            // bounds check.
            let mut arr: [LoopFn; 256] = [extract_and_execute::<Interrupt>; 256];
            #[allow(unused_assignments)]
            {
                let mut i = 0;
                $(arr[i] = extract_and_execute::<$name>; i += 1;)+
            }
            arr
        };
        #[cfg(all(nightly, feature = "tailcall"))]
        static TAILCALL_INSTR_FNS: [tailcall::TailcallFn; 256] = const {
            // @NOTE: We pad here to 256 such that indexing into it with an u8, does not need a
            // bounds check.
            let mut arr: [tailcall::TailcallFn; 256] = [tailcall::extract_and_execute_tailcall::<Interrupt>; 256];
            #[allow(unused_assignments)]
            {
                let mut i = 0;
                $(arr[i] = tailcall::extract_and_execute_tailcall::<$name>; i += 1;)+
            }
            arr
        };

        static NUM_ADDITIONAL_SLOTS_FNS: [fn(c: Bytecode) -> u8; NUM_INSTRUCTIONS] = [$(extract_and_num_additional_slots::<$name>),+];
        static SRC_REGS_FNS: [RegFn; NUM_INSTRUCTIONS] = [$(extract_and_source_regs::<$name>),+];
        static DST_REGS_FNS: [RegFn; NUM_INSTRUCTIONS] = [$(extract_and_dest_regs::<$name>),+];

        impl TryFrom<u8> for BytecodeOpcode {
            type Error = ();
            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $(_ if value == BytecodeOpcode::$name as u8 => Ok(BytecodeOpcode::$name),)*
                    _ => Err(()),
                }
            }
        }

        impl BytecodeOpcode {
            pub fn into_static_str(self) -> &'static str {
                match self {
                    $(Self::$name => stringify!($name),)*
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
    TvCne,
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
    TvLeadingZeros,
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
    TvUgeqi,
    TvUlti,
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
    FvXnor,
    FvCeq,
    FvCne,
    FvBitwiseCeq,
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
    FvSlrx,
    FvLeftShiftOr,
    FvCopyX,
    FvCopyZ,
    FvNot,
    FvReduceAnd,
    FvReduceOr,
    FvReduceXor,
    FvLeadingZeros,
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
    FvUgeqi,
    FvUlti,
    FvCeqi,
    FvCnei,
    FvBitwiseCeqi,
    FvSlli,
    FvSlri,
    FvSari,
    PushArgument,
    Intrinsic,
    SignExtend,
    StackOffset,
    StackOffsetReg,
    LoadImm,
    Jump,
    RelJump,
    BranchTrue,
    BranchFalse,
    TvSetAligned,
    TvRelSetAligned,
    TvSetHeapAligned,
    FvSetAligned,
    FvRelSetAligned,
    FvSetHeapAligned,
    SetUnaligned,
    RelSetUnaligned,
    SetHeapUnaligned,
    TvLoadAligned,
    TvLoadRelAligned,
    FvLoadAligned,
    LoadUnaligned,
    LoadRelUnaligned,
    LoadHeapAligned,
    LoadHeapUnaligned,
    Wake,
    WakeMultiple,
    RescheduleWait,
    RescheduleRegion,
    NextEvent,
    RescheduleListen,
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
    HeapFill,
    HeapConcat,
    HeapSlice,
    TvTvHeapSlice0,
    TvTvHeapSliceX,
    TvFvHeapSlice0,
    TvFvHeapSliceX,
    FvTvHeapSlice0,
    FvTvHeapSliceX,
    FvFvHeapSlice0,
    FvFvHeapSliceX,
    PluginPoke,
    RealInstr,
];

#[derive(Clone)]
pub struct BytecodeListeners {
    map: Vec<InstructionPtr>,
    active: Vec<u64>,
}

#[derive(Clone, Copy)]
pub struct InlineNBitSize<const N: usize>(Option<VectorSize>);

impl<const N: usize> InlineNBitSize<N> {
    #[inline(always)]
    pub fn get(self, pc: &mut u64, code: &[Bytecode]) -> VectorSize {
        match self.0 {
            None => {
                let size = code[*pc as usize].0;
                let size = VectorSize::new(size).expect("Expected non-zero size");
                *pc += 1;
                size
            }
            Some(s) => s,
        }
    }

    #[inline(always)]
    pub fn new_masked(v: u32) -> Self {
        Self(VectorSize::new(
            v & 1u32.unbounded_shl(N as u32).wrapping_sub(1),
        ))
    }

    pub fn encode(self) -> u32 {
        self.0.map_or(0, |v| v.get())
    }

    pub fn new(size: VectorSize) -> Self {
        if size.get() < (1u32 << N) {
            return Self(Some(size));
        }
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

        bce.add_immediate(scratch, addr, offset, scratch);
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

    pub fn new(value: u64) -> Self {
        const { assert!(NBITS >= 1 && NBITS <= 32) };
        if value < ((1 << NBITS) - 1) {
            Self(Some(NonMaxU32::new(value as u32).unwrap()))
        } else {
            Self(None)
        }
    }

    #[inline(always)]
    pub fn get(self, pc: &mut u64, code: &[Bytecode]) -> u64 {
        self.0.map_or_else(
            || {
                let items: &[Bytecode; 2] = code[*pc as usize..*pc as usize + 2]
                    .try_into()
                    .expect("Expected more bytecode slots");
                let v = ((items[0].0 as u64) << 32) | (items[1].0 as u64);
                *pc += 2;
                v
            },
            |v| v.get() as u64,
        )
    }

    fn is_inline(&self) -> bool {
        self.0.is_some()
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

    #[inline(always)]
    fn arm(&mut self, index: usize) {
        self.active[index / 64] |= 1u64 << (index % 64);
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
    max_time: u64,
}

impl Schedule {
    pub fn new(num_regions: u8) -> Self {
        Self {
            active: Vec::new(),
            regions: vec![Vec::new(); num_regions as usize].into_boxed_slice(),
            future: Vec::new(),
            next_time: u64::MAX,
            max_time: u64::MAX,
        }
    }

    #[inline(always)]
    pub fn push_active(&mut self, ptr: InstructionPtr) {
        self.active.push(ptr);
    }

    #[inline(always)]
    pub fn pop(
        &mut self,
        state: &mut RuntimeState,
        plugins: &mut [RuntimePluginState],
    ) -> Option<InstructionPtr> {
        if let Some(pc) = self.active.pop() {
            return Some(pc);
        }

        'fill_active: {
            for region in &mut self.regions {
                if !region.is_empty() {
                    std::mem::swap(&mut self.active, region);
                    break 'fill_active;
                }
            }

            for plugin in plugins.iter_mut() {
                plugin.timestep(state);
            }

            // Stop if there are no more events.
            if self.future.is_empty() {
                break 'fill_active;
            }

            // Stop if we reach the maximum time.
            if self.next_time > self.max_time {
                state.time = self.max_time;
                break 'fill_active;
            }

            // Take all the events with the next minimum time and find the new minimum.
            state.time = self.next_time;
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

        let pc = self.active.pop();
        if pc.is_none() {
            for plugin in plugins.iter_mut() {
                plugin.finish(state);
            }
        }
        pc
    }

    pub fn set_max_time(&mut self, time: u64) {
        self.max_time = time;
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
            IntrinsicOp::Time | IntrinsicOp::Finish => {}
            IntrinsicOp::Random(r) => r.hash(state),
            IntrinsicOp::Display(f) | IntrinsicOp::Assert(f) => f.hash(state),
            IntrinsicOp::VcdOpenFile(_) | IntrinsicOp::VcdAppendModule(_) => {}
            IntrinsicOp::VcdPause | IntrinsicOp::VcdResume | IntrinsicOp::ReadMem(_) => {}
            IntrinsicOp::SetTimeFormat(fmt) => fmt.hash(state),
        }
    }
}
impl PartialEq for IntrinsicOpEqWrap {
    fn eq(&self, other: &Self) -> bool {
        if std::mem::discriminant(&self.0) != std::mem::discriminant(&other.0) {
            return false;
        }
        match (&self.0, &other.0) {
            (IntrinsicOp::Time | IntrinsicOp::Finish, _) => true,
            (IntrinsicOp::Random(l), IntrinsicOp::Random(r)) => l == r,
            (IntrinsicOp::SetTimeFormat(l), IntrinsicOp::SetTimeFormat(r)) => l == r,
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
        let (rsspc, rsval) = rs.to_spc_and_val();
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

        if value.count_ones() == value.trailing_ones() {
            self.ori(
                rd,
                rd,
                SignedImmediate::MINUS_ONE,
                SixBitSize::new_masked(value.count_ones() - 1),
            );
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

    fn add_immediate(&mut self, rd: Reg, rs: Reg, offset: i64, scratch: Reg) {
        match SignedImmediate::new(offset) {
            None => {
                self.load_u64(scratch, offset as u64);
                self.add(rd, rs, scratch, SixBitSize::N64);
            }
            Some(imm) => self.addi(rd, rs, imm, SixBitSize::N64),
        }
    }
}

pub trait Tracer {
    fn pre_exec(
        &mut self,
        i: Bytecode,
        code: &[Bytecode],
        regs: &Regs,
        pc: u64,
        state: &RuntimeState,
        schedule: &Schedule,
        listeners: &BytecodeListeners,
    ) {
        _ = (i, code, regs, pc, state, schedule, listeners);
    }
    fn post_exec(
        &mut self,
        i: Bytecode,
        code: &[Bytecode],
        regs: &Regs,
        pc: u64,
        state: &RuntimeState,
        schedule: &Schedule,
        listeners: &BytecodeListeners,
    ) {
        _ = (i, code, regs, pc, state, schedule, listeners);
    }

    fn start(
        &mut self,
        code: &[Bytecode],
        regs: &Regs,
        pc: u64,
        state: &RuntimeState,
        schedule: &Schedule,
        listeners: &BytecodeListeners,
    ) {
        _ = (code, regs, pc, state, schedule, listeners);
    }

    fn finish(
        &mut self,
        code: &[Bytecode],
        regs: &Regs,
        pc: u64,
        state: &RuntimeState,
        schedule: &Schedule,
        listeners: &BytecodeListeners,
    ) {
        _ = (code, regs, pc, state, schedule, listeners);
    }
}

impl Tracer for () {}

struct DisplayWith<'a>(&'a Regs, &'a Heap, &'a [RegInfo]);
impl<'a> fmt::Display for DisplayWith<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_operands(f, self.0, self.1, self.2)
    }
}

pub struct InstructionTracer {
    io: Box<dyn std::io::Write>,
    pc: u64,
    scratch: Vec<RegInfo>,
}

impl InstructionTracer {
    pub fn new_stderr() -> Self {
        Self {
            io: Box::new(std::io::BufWriter::new(std::io::stderr())),
            pc: 0,
            scratch: Vec::new(),
        }
    }
}

impl Tracer for InstructionTracer {
    fn pre_exec(
        &mut self,
        i: Bytecode,
        code: &[Bytecode],
        regs: &Regs,
        pc: u64,
        state: &RuntimeState,
        _schedule: &Schedule,
        _listeners: &BytecodeListeners,
    ) {
        self.pc = pc;
        writeln!(&mut self.io, "[PC={pc:4}]: {i}").unwrap();
        (SRC_REGS_FNS[i.opcode() as usize])(i, code, self.pc, &mut self.scratch);
        write!(
            &mut self.io,
            "{}",
            DisplayWith(regs, &state.heap, &self.scratch)
        )
        .unwrap();
    }

    fn post_exec(
        &mut self,
        i: Bytecode,
        code: &[Bytecode],
        regs: &Regs,
        pc: u64,
        state: &RuntimeState,
        _schedule: &Schedule,
        _listeners: &BytecodeListeners,
    ) {
        _ = pc;
        (DST_REGS_FNS[i.opcode() as usize])(i, code, self.pc, &mut self.scratch);
        write!(
            &mut self.io,
            "{}",
            DisplayWith(regs, &state.heap, &self.scratch)
        )
        .unwrap();
    }
}

pub struct ICountTracer {
    opcode: [u64; 256],
}

impl Default for ICountTracer {
    fn default() -> Self {
        Self {
            opcode: [0u64; 256],
        }
    }
}

impl Tracer for ICountTracer {
    fn pre_exec(
        &mut self,
        i: Bytecode,
        _code: &[Bytecode],
        _regs: &Regs,
        _pc: u64,
        _state: &RuntimeState,
        _schedule: &Schedule,
        _listeners: &BytecodeListeners,
    ) {
        self.opcode[i.opcode() as usize] += 1;
    }

    fn finish(
        &mut self,
        _code: &[Bytecode],
        _regs: &Regs,
        _pc: u64,
        _state: &RuntimeState,
        _schedule: &Schedule,
        _listeners: &BytecodeListeners,
    ) {
        let mut ops = (0..256).collect::<Vec<usize>>();
        ops.sort_by_key(|op| self.opcode[*op]);

        let mut longest_name = 0usize;
        let mut longest_count = 0usize;
        for op in 0u8..=255 {
            let count = self.opcode[op as usize];
            let name = match BytecodeOpcode::try_from(op) {
                Err(_) => "<unknown>",
                Ok(opcode) => opcode.into_static_str(),
            };
            longest_name = longest_name.max(name.len());
            longest_count = if count == 0 {
                longest_count.max(1)
            } else {
                longest_count.max((count.ilog10() + 1) as usize)
            };
        }

        for op in ops.into_iter().rev() {
            let count = self.opcode[op];
            if count == 0 {
                break;
            }
            let name = match BytecodeOpcode::try_from(op as u8) {
                Err(_) => "<unknown>",
                Ok(opcode) => opcode.into_static_str(),
            };
            eprintln!("| {name:<0$} | {count:>1$} |", longest_name, longest_count);
        }
    }
}

impl Design {
    pub fn execute(
        &self,
        state: &mut State,
        world: &mut dyn World,
    ) -> Result<(), ()> {
        self.execute_with_tracer(&mut (), state, world)
    }

    pub fn execute_with_tracer(
        &self,
        tracer: &mut impl Tracer,
        state: &mut State,
        world: &mut dyn World,
    ) -> Result<(), ()> {
        let code = &self.bytecode;
        let Some(entry) = state
            .schedule
            .pop(&mut state.runtime, state.plugins.as_mut())
        else {
            return Ok(());
        };

        let mut pc = entry.0;
        let mut cldctx = ColdContext::new(
            &self.intrinsics,
            &self.watchers,
            &mut state.plugins,
            world,
            self.time_resolution,
        );
        let mut regs = Regs::new(self.stack_offset);

        tracer.start(
            code,
            &regs,
            pc,
            &state.runtime,
            &state.schedule,
            &state.listeners,
        );

        while let Some(&c) = code.get(pc as usize) {
            let opcode = c.opcode();
            tracer.pre_exec(
                c,
                code,
                &regs,
                pc,
                &state.runtime,
                &state.schedule,
                &state.listeners,
            );
            let f = LOOP_INSTR_FNS[opcode as usize];
            pc = (f)(
                c,
                code,
                &mut regs,
                pc,
                &mut state.runtime,
                &mut state.schedule,
                &mut state.listeners,
                &mut cldctx,
            );
            tracer.post_exec(
                c,
                code,
                &regs,
                pc,
                &state.runtime,
                &state.schedule,
                &state.listeners,
            );
        }

        tracer.finish(
            code,
            &regs,
            pc,
            &state.runtime,
            &state.schedule,
            &state.listeners,
        );

        if cldctx.return_value != 0 {
            return Err(());
        }

        Ok(())
    }

    #[cfg(feature = "tailcall")]
    pub fn execute_inner_tailcall(
        &self,
        state: &mut State,
        world: &mut dyn World,
    ) -> Result<(), ()> {
        #[cfg(not(nightly))]
        {
            return self.execute(state, world);
        }

        #[cfg(nightly)]
        {
            let code = &self.bytecode;
            let Some(entry) = state
                .schedule
                .pop(&mut state.runtime, state.plugins.as_mut())
            else {
                return Ok(());
            };

            let pc = entry.0;
            let mut cldctx = ColdContext::new(
                &self.intrinsics,
                &self.watchers,
                state.plugins.as_mut(),
                world,
            );
            let mut regs = Regs::new(self.stack_offset);
            let Some(c) = code.get(pc as usize) else {
                return Ok(());
            };
            let opcode = c.opcode();
            let f = TAILCALL_INSTR_FNS[opcode as usize];
            (f)(
                *c,
                code,
                &mut regs,
                pc,
                &mut state.runtime,
                &mut state.schedule,
                &mut state.listeners,
                &mut cldctx,
            );

            if cldctx.return_value != 0 {
                return Err(());
            }

            Ok(())
        }
    }

    pub fn poke_signal(&self, state: &mut State, key: RtSignalKey) {
        for index in self.watchers.get(key.as_usize()) {
            wake(*index, &mut state.schedule, &mut state.listeners);
        }
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

fn write_operands(
    f: &mut fmt::Formatter<'_>,
    regs: &Regs,
    heap: &Heap,
    operands: &[RegInfo],
) -> fmt::Result {
    if operands.is_empty() {
        return Ok(());
    }

    f.write_str(EXEC_ITRACE_INDENT)?;
    for (i, operand) in operands.iter().enumerate() {
        if i > 0 {
            f.write_str(", ")?;
        }

        let reg = operand.reg();
        match operand.storage() {
            RegStorage::Register => write_register(f, regs, operand.name(), reg, operand.mode())?,
            RegStorage::Heap => {
                write!(f, "{} = heap[0x{:x}]", operand.name(), regs[reg],)?;
                if let Some(size) = operand.size() {
                    let value = heap.load_bits(
                        HeapOffset {
                            bit_offset: regs[reg] as usize,
                        }
                        .to_ref(size),
                        operand.mode(),
                    );
                    write!(
                        f,
                        ": {}",
                        value.display(&vogls_bits::format::BitsFormatOptions {
                            prefix: true,
                            base: vogls_bits::format::BitsFormatBase::UpperHex,
                            separator: Some('_'),
                            signed: false,
                            align: None,
                            fill: '0',
                            width: vogls_bits::format::BitsFormatWidth::Shrink
                        })
                    )?;
                }
            }
        }
    }
    writeln!(f)
}
