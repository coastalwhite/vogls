use std::fmt::{self, Debug};
use std::ops::{Index, IndexMut};

use vogls_bits::BitsDataRef;
use vogls_bits::arithmetic::{
    FvLogicValue, fv_bin_u64_cell_bitwise_op, fv_bitwise_and_elem, fv_bitwise_andnot_elem,
    fv_bitwise_inv_elem, fv_bitwise_or_elem, fv_bitwise_ornot_elem, fv_bitwise_xor_elem,
    fv_reduce_and_elem, fv_reduce_or_elem, fv_reduce_xor_elem,
    tv_bin_u64_cell_bitwise_mask_last_op, tv_bin_u64_cell_bitwise_op,
};
use vogls_bits::extend::tv_l_zero_extend;
use vogls_bits::format::{BitsFormatBase, BitsFormatWidth};
use vogls_bits::truncate::tv_l_truncate;
use vogls_codegen::lsra::StackItemKind;
use vogls_codegen::{HeapOffset, HeapRef};

use vogls_ir::{Bits, IntrinsicOp, LogicMode, Mode, SCALAR_VSIZE, VSIZE_32, VSIZE_64, VectorSize};
use vogls_runtime::RuntimeState;
use vogls_runtime::plugins::{RuntimePlugin, RuntimePluginState};
use vogls_utils::{IndexSet, NonMaxU16};

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reg {
    #[default]
    X0,
    X1,
    X2,
    X3,
    X4,
    X5,
    X6,
    X7,
    X8,
    X9,
    X10,
    X11,
    X12,
    X13,
    X14,
    X15,
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "x{}", *self as u32)
    }
}

pub struct Regs([u64; 16]);

impl Index<Reg> for Regs {
    type Output = u64;

    fn index(&self, index: Reg) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl IndexMut<Reg> for Regs {
    fn index_mut(&mut self, index: Reg) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

impl fmt::Display for SixBitSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&if self.0 == 0 { 64 } else { self.0 }, f)
    }
}

impl Reg {
    #[inline(always)]
    pub fn new_masked(v: u32) -> Self {
        match v & 0xF {
            0 => Self::X0,
            1 => Self::X1,
            2 => Self::X2,
            3 => Self::X3,
            4 => Self::X4,
            5 => Self::X5,
            6 => Self::X6,
            7 => Self::X7,
            8 => Self::X8,
            9 => Self::X9,
            10 => Self::X10,
            11 => Self::X11,
            12 => Self::X12,
            13 => Self::X13,
            14 => Self::X14,
            15 => Self::X15,
            _ => unreachable!(),
        }
    }

    /// Get the two registers used to store Four-Value Logic.
    ///
    /// This splits the value into the _Special_ (`spc`) and the _Value_ (`val`).
    ///
    /// |           | special=0 | special=1 |
    /// | value = 0 |         x |         0 |
    /// | value = 1 |         z |         1 |
    pub fn to_spc_and_val(self) -> (Self, Self) {
        debug_assert_ne!(self, Self::X15);
        (self, Self::new_masked(self as u32 + 1))
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
        if self.0 == 0 {
            v
        } else {
            v & ((1u64 << self.0) - 1)
        }
    }
}

#[derive(Default)]
struct EncUnary {
    rd: Reg,
    rs: Reg,
    size: SixBitSize,
}
#[derive(Default)]
struct EncBinaryImm {
    rd: Reg,
    rs: Reg,
    size: SixBitSize,

    imm10: i16,
}
#[derive(Default)]
struct EncBinaryUImm {
    rd: Reg,
    rs: Reg,
    size: SixBitSize,

    imm10: u16,
}
#[derive(Default)]
struct EncUnaryUImm {
    rd: Reg,
    rs: Reg,
    imm: u16,
}
#[derive(Default)]
struct EncSet {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    size: SixBitSize,

    // 6 bits
    imm: i8,
}
#[derive(Default)]
struct EncHeapSet {
    rd: Reg,
    rs: Reg,
    roff: Reg,
    size: Option<VectorSize>,
}
#[derive(Default)]
struct EncBinaryReg {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    size: SixBitSize,
}

#[derive(Default)]
struct EncHeapBitwiseReg {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,

    op: BitwiseOp,

    // None: Size is in X12.
    size: Option<VectorSize>,
}

#[derive(Default)]
struct EncHeapCaseEq {
    rd: Reg,
    rs1: Reg,
    rs2: Reg,

    fv: bool,
    ne: bool,

    // None: Size is in X12.
    size: Option<VectorSize>,
}

#[derive(Default)]
struct EncHeapUnaryReg {
    rd: Reg,
    rs: Reg,

    imm16: u16,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum BitwiseOp {
    #[default]
    TvAnd,
    TvOr,
    TvXor,
    TvAndNot,
    TvOrNot,
    TvAdd,
    TvSub,
    TvMul,

    FvAnd,
    FvOr,
    FvXor,
    FvAndNot,
    FvOrNot,
    FvAdd,
    FvSub,
    FvMul,
}
impl BitwiseOp {
    fn new_masked(v: u32) -> Self {
        match v & 0xF {
            0 => Self::TvAnd,
            1 => Self::TvOr,
            2 => Self::TvXor,
            3 => Self::TvAndNot,
            4 => Self::TvOrNot,
            5 => Self::TvAdd,
            6 => Self::TvSub,
            7 => Self::TvMul,

            8 => Self::FvAnd,
            9 => Self::FvOr,
            10 => Self::FvXor,
            11 => Self::FvAndNot,
            12 => Self::FvOrNot,
            13 => Self::FvAdd,
            14 => Self::FvSub,
            _ => Self::FvMul,
        }
    }

    fn is_four_value(self) -> bool {
        (self as u32) >= 8
    }
}

#[derive(Default)]
struct EncLoadImm {
    rd: Reg,
    clear: bool,
    sign_extend: bool,
    segment: u8,
    imm: i16,
}

#[derive(Default)]
struct EncWake {
    rcond: Reg,
    index: u32,
}

#[derive(Default)]
pub struct EncJump {
    pub imm: i32,
}

#[derive(Default)]
pub struct EncRelJump {
    rs: Reg,
    pub imm: i32,
}

#[derive(Default)]
pub struct EncUImm {
    pub imm: u32,
}

#[derive(Default)]
pub struct EncReschedule {
    rtime: Reg,
    schedule_self: bool,
    region: u8,
}

#[derive(Default)]
struct EncEmpty {}

#[derive(Default)]
struct EncPushArgument {
    size: Option<VectorSize>,
    mode: LogicMode,
    rs: Reg,
}

#[derive(Default)]
struct EncIntrinsic {
    rd: Reg,
    id: Option<NonMaxU16>,
}

pub trait Encoding: Sized {
    fn extract(bytecode: Bytecode) -> Self;
    fn encode(self) -> u32;
}

impl Encoding for EncUnary {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8) | ((self.rs as u32) << 12) | ((self.size.0 as u32) << 16)
    }
}
impl Encoding for EncUnaryUImm {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            imm: (v >> 16) as u16,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8) | ((self.rs as u32) << 12) | ((self.imm as u32) << 16)
    }
}
impl Encoding for EncBinaryImm {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
            imm10: ((v as i32) >> 22) as i16,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.rs as u32) << 12)
            | ((self.size.0 as u32) << 16)
            | ((self.imm10 as u16 as u32) << 22)
    }
}
impl Encoding for EncBinaryUImm {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            size: SixBitSize::new_masked(v >> 16),
            imm10: (v >> 22) as u16,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.rs as u32) << 12)
            | ((self.size.0 as u32) << 16)
            | ((self.imm10 as u32) << 22)
    }
}
impl Encoding for EncHeapUnaryReg {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            imm16: (v >> 16) as u16,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8) | ((self.rs as u32) << 12) | ((self.imm16 as u32) << 22)
    }
}

impl Encoding for EncSet {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            size: SixBitSize::new_masked(v >> 20),
            imm: (v >> 26) as i32 as i8,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.rs as u32) << 12)
            | ((self.roff as u32) << 16)
            | ((self.size.0 as u32) << 20)
            | ((self.imm as u16 as u32) << 26)
    }
}

impl Encoding for EncHeapSet {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            roff: Reg::new_masked(v >> 16),
            size: VectorSize::new(v >> 20),
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.rs as u32) << 12)
            | ((self.roff as u32) << 16)
            | (self.size.map_or(0, |v| v.get()) << 20)
    }
}

impl Encoding for EncBinaryReg {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            size: SixBitSize::new_masked(v >> 20),
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.rs1 as u32) << 12)
            | ((self.rs2 as u32) << 16)
            | ((self.size.0 as u32) << 20)
    }
}

impl Encoding for EncHeapBitwiseReg {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            op: BitwiseOp::new_masked(v >> 20),
            size: VectorSize::new(v >> 24),
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.rs1 as u32) << 12)
            | ((self.rs2 as u32) << 16)
            | ((self.op as u32) << 20)
            | (self.size.map_or(0, |v| v.get()) << 24)
    }
}

impl Encoding for EncHeapCaseEq {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs1: Reg::new_masked(v >> 12),
            rs2: Reg::new_masked(v >> 16),
            fv: (v >> 20) & 1 != 0,
            ne: (v >> 21) & 1 != 0,
            size: VectorSize::new(v >> 22),
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.rs1 as u32) << 12)
            | ((self.rs2 as u32) << 16)
            | ((self.fv as u32) << 20)
            | ((self.ne as u32) << 21)
            | (self.size.map_or(0, |v| v.get()) << 22)
    }
}

impl Encoding for EncLoadImm {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            clear: (v >> 12) & 1 != 0,
            sign_extend: (v >> 13) & 1 != 0,
            segment: ((v >> 14) & 0x3) as u8,
            imm: ((v as i32) >> 16) as i16,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.clear as u32) << 12)
            | ((self.sign_extend as u32) << 13)
            | ((self.segment as u32) << 14)
            | ((self.imm as u16 as u32) << 16)
    }
}

impl Encoding for EncWake {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rcond: Reg::new_masked(v >> 8),
            index: v >> 12,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rcond as u32) << 8) | ((self.index as u32) << 12)
    }
}

impl Encoding for EncJump {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            imm: (v as i32) >> 8,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        (self.imm as u32) << 8
    }
}

impl Encoding for EncRelJump {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rs: Reg::new_masked(v >> 8),
            imm: (v as i32) >> 12,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rs as u32) << 8) | (self.imm as u32) << 12
    }
}

impl Encoding for EncEmpty {
    #[inline(always)]
    fn extract(_bytecode: Bytecode) -> Self {
        Self {}
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        0
    }
}

impl Encoding for EncReschedule {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rtime: Reg::new_masked(v >> 8),
            region: ((v >> 12) & 0xFF) as u8,
            schedule_self: (v >> 20) & 1 != 0,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rtime as u32) << 8)
            | ((self.region as u32) << 12)
            | (self.schedule_self as u32) << 20
    }
}

impl Encoding for EncUImm {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self { imm: v >> 8 }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        (self.imm as u32) << 8
    }
}

impl Encoding for EncPushArgument {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rs: Reg::new_masked(v >> 8),
            mode: match (v >> 12) & 1 {
                0 => LogicMode::TwoValue,
                _ => LogicMode::FourValue,
            },
            size: VectorSize::new(v >> 13),
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rs as u32) << 8)
            | ((self.mode as u32) << 12)
            | (self.size.map_or(0u32, |v| v.get()) << 13)
    }
}

impl Encoding for EncIntrinsic {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            id: NonMaxU16::new((v >> 16) as u16),
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8) | (self.id.map_or(0u32, |v| v.get() as u32) << 16)
    }
}

#[derive(Clone)]
pub struct BytecodeListeners {
    map: Vec<InstructionPtr>,
    active: Vec<u64>,
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

pub struct PreBytecodeTrace<'a> {
    value: Bytecode,
    regs: &'a Regs,
}
pub struct PostBytecodeTrace<'a> {
    value: Bytecode,
    regs: &'a Regs,
}

pub trait Trace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, regs: &Regs) -> fmt::Result;
}

impl<T: fmt::Display> Trace for T {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, _regs: &Regs) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub struct TraceReg(Reg, LogicMode, Option<VectorSize>);

impl Trace for TraceReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, regs: &Regs) -> fmt::Result {
        let size = self.2.unwrap_or(VSIZE_64);
        let bits = match self.1 {
            LogicMode::TwoValue => Bits::from_u64(size, regs[self.0]),
            LogicMode::FourValue if size <= VSIZE_32 => {
                let (spc, val) = self.0.to_spc_and_val();
                Bits::from_four_value_u64(size, regs[spc] as u32, regs[val] as u32)
            }
            LogicMode::FourValue => {
                let (spc, val) = self.0.to_spc_and_val();
                Bits::from_boxed_slice(
                    LogicMode::FourValue.into(),
                    size,
                    [regs[spc], regs[val]].into(),
                )
            }
        };
        fmt::Display::fmt(
            &bits.display(&vogls_bits::format::BitsFormatOptions {
                prefix: true,
                base: BitsFormatBase::Binary,
                separator: Some('_'),
                align: None,
                fill: '0',
                width: BitsFormatWidth::Shrink,
            }),
            f,
        )
    }
}

macro_rules! define_instructions {
    (
        $regs:ident, $pc: ident, $state:ident, $schedule:ident, $listeners:ident, $cldctx:ident, $formatter:ident,
        [$(
        $name:ident ($enc_variant:ident { $($param:ident: $param_ty:ty),* }) $blk:block $fmt:block
        $(: MNEMONIC: $mnemonic:block)?
        $(($($pre_trace_arg:ident = $pre_trace_value:expr),* $(,)?) ($($post_trace_arg:ident = $post_trace_value:expr),* $(,)?))?
    )+]) => {
        #[repr(u8)]
        #[expect(non_camel_case_types)]
        pub enum BytecodeKind {
            $($name,)*
        }

        impl TryFrom<u8> for BytecodeKind {
            type Error = ();
            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    $(_ if value == BytecodeKind::$name as u8 => Ok(BytecodeKind::$name),)*
                    _ => Err(()),
                }
            }
        }

        impl fmt::Display for Bytecode {
            fn fmt(&self, $formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Ok(kind) = BytecodeKind::try_from(self.opcode()) else {
                    return $formatter.write_str("unknown opcode");
                };

                match kind {
                    $(BytecodeKind::$name => {
                        #[allow(unused_variables)]
                        let $enc_variant { $($param,)* .. } = $enc_variant::extract(*self);
                        #[allow(unused_assignments, unused_mut)]
                        let mut mnemonic: &str = stringify!($name);
                        $( mnemonic = $mnemonic; )?
                        write!($formatter, "{mnemonic:<0$}", MAX_MNEMONIC_SIZE + 2)?;
                        $fmt
                    })*
                }
            }
        }

        impl<'a> fmt::Display for PreBytecodeTrace<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Ok(kind) = BytecodeKind::try_from(self.value.opcode()) else {
                    return f.write_str("unknown opcode");
                };

                match kind {
                    $(BytecodeKind::$name => {
                        #[allow(unused_variables)]
                        let $enc_variant { $($param,)* .. } = $enc_variant::extract(self.value);
                        $(
                            #[allow(unused_mut, unused_variables)]
                            let mut fst = true;
                            $(
                                if !fst {
                                    f.write_str(", ")?;
                                }
                                f.write_str(stringify!($pre_trace_arg))?;
                                f.write_str(" = ")?;
                                Trace::fmt(&$pre_trace_value, f, self.regs)?;
                                #[allow(unused_assignments)]
                                {
                                    fst = false;
                                }
                            )*
                        )?
                    },)*
                }
                Ok(())
            }
        }
        impl<'a> fmt::Display for PostBytecodeTrace<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Ok(kind) = BytecodeKind::try_from(self.value.opcode()) else {
                    return f.write_str("unknown opcode");
                };

                match kind {
                    $(BytecodeKind::$name => {
                        #[allow(unused_variables)]
                        let $enc_variant { $($param,)* .. } = $enc_variant::extract(self.value);
                        $(
                            #[allow(unused_mut, unused_variables)]
                            let mut fst = true;
                            $(
                                if !fst {
                                    f.write_str(", ")?;
                                }
                                f.write_str(stringify!($post_trace_arg))?;
                                f.write_str(" = ")?;
                                Trace::fmt(&$post_trace_value, f, self.regs)?;
                                #[allow(unused_assignments)]
                                {
                                    fst = false;
                                }
                            )*
                        )?
                    },)*
                }
                Ok(())
            }
        }

        const NUM_INSTRUCTIONS: usize = {
            0 $(+
            { _ = $name; 1 }
            )+
        };

        static INSTRUCTION_FNS: [
            fn(
                c: Bytecode,
                regs: &mut Regs,
                pc: u64,
                state: &mut RuntimeState,
                schedule: &mut Schedule,
                listeners: &mut BytecodeListeners,
                cldctx: &mut ColdContext,
            ) -> u64;
            NUM_INSTRUCTIONS
        ] = [$($name),+];

        const MNEMONICS: [&'static str; NUM_INSTRUCTIONS] = [$(stringify!($name)),+];
        const MAX_MNEMONIC_SIZE: usize = {
            let mut max = 0usize;
            let mut i = 0usize;
            while i < MNEMONICS.len() {
                if MNEMONICS[i].len() > max {
                    max = MNEMONICS[i].len();
                }
                i += 1;
            }
            max
        };

        $(
        fn $name(
            c: Bytecode,
            $regs: &mut Regs,
            $pc: u64,
            $state: &mut RuntimeState,
            $schedule: &mut Schedule,
            $listeners: &mut BytecodeListeners,
            $cldctx: &mut ColdContext<'_>,
        ) -> u64 {
            _ = $regs;
            _ = $state;
            _ = $schedule;
            _ = $listeners;
            _ = $cldctx;
            let $enc_variant { $($param,)* .. } = $enc_variant::extract(c);
            #[allow(unused_mut, unused_assignments)]
            let mut $pc = $pc + 1;
            $blk
            $pc
        }
        )+

        impl BytecodeEncoder {
            $(
            pub fn $name(&mut self, $($param: $param_ty),*) {
                let bits = $enc_variant { $($param: $param.into(),)* ..Default::default() }.encode();
                let bits = bits | BytecodeKind::$name as u32;
                self.data.push(Bytecode(bits));
            }
            )+
        }
    };
}

define_instructions! {
    regs, pc, state, schedule, listeners, cldctx, f,
    [
        // TV Operations
        and (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            { regs[rd] = regs[rs1] & regs[rs2]; }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, None), rs2 = TraceReg(rs2, LogicMode::TwoValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        or (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            { regs[rd] = regs[rs1] | regs[rs2]; }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, None), rs2 = TraceReg(rs2, LogicMode::TwoValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        xor (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            { regs[rd] = regs[rs1] ^ regs[rs2]; }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, None), rs2 = TraceReg(rs2, LogicMode::TwoValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        or_not (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize })
            { regs[rd] = size.mask(regs[rs1] | !regs[rs2]); }
            { write!(f, "{rd}, {rs1}, {rs2}, |{size}|") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, None), rs2 = TraceReg(rs2, LogicMode::TwoValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        and_not (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize })
            { regs[rd] = size.mask(regs[rs1] & !regs[rs2]); }
            { write!(f, "{rd}, {rs1}, {rs2}, |{size}|") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, None), rs2 = TraceReg(rs2, LogicMode::TwoValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        xnor (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize })
            { regs[rd] = size.mask(!(regs[rs1] ^ regs[rs2])); }
            { write!(f, "{rd}, {rs1}, {rs2}, |{size}|") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, None), rs2 = TraceReg(rs2, LogicMode::TwoValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        ceq (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            { regs[rd] = u64::from(regs[rs1] == regs[rs2]); }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, None), rs2 = TraceReg(rs2, LogicMode::TwoValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )
        andi (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            { regs[rd] = regs[rs] & size.mask(i64::from(imm10) as u64); }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::TwoValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(size.into())) )
        ori (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            { regs[rd] = regs[rs] | size.mask(i64::from(imm10) as u64); }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::TwoValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(size.into())) )
        xori (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            { regs[rd] = regs[rs] ^ size.mask(i64::from(imm10) as u64); }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::TwoValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(size.into())) )
        addi (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            { regs[rd] = size.mask(regs[rs].wrapping_add(i64::from(imm10) as u64)); }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::TwoValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(size.into())) )
        ceqi (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            { regs[rd] = u64::from(regs[rs] == size.mask(i64::from(imm10) as u64)); }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::TwoValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(size.into())) )
        cnei (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            { regs[rd] = u64::from(regs[rs] != size.mask(i64::from(imm10) as u64)); }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::TwoValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(size.into())) )
        slli (EncBinaryUImm { rd: Reg, rs: Reg, imm10: u16, size: SixBitSize })
            { regs[rd] = size.mask(regs[rs] << imm10); }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::TwoValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(size.into())) )

        count_ones (EncUnary { rd: Reg, rs: Reg })
            { regs[rd] = u64::from(regs[rs].count_ones()); }
            { write!(f, "{rd}, {rs}") }
            ( rs = TraceReg(rs, LogicMode::TwoValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )

        add (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize })
            { regs[rd] = size.mask(regs[rs1].wrapping_add(regs[rs2])); }
            { write!(f, "{rd}, {rs1}, {rs2}, |{size}|") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, Some(size.into())), rs2 = TraceReg(rs2, LogicMode::TwoValue, Some(size.into())) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        sub (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize })
            { regs[rd] = size.mask(regs[rs1].wrapping_sub(regs[rs2])); }
            { write!(f, "{rd}, {rs1}, {rs2}, |{size}|") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, Some(size.into())), rs2 = TraceReg(rs2, LogicMode::TwoValue, Some(size.into())) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        mul (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize })
            { regs[rd] = size.mask(regs[rs1].wrapping_mul(regs[rs2])); }
            { write!(f, "{rd}, {rs1}, {rs2}, |{size}|") }
            ( rs1 = TraceReg(rs1, LogicMode::TwoValue, Some(size.into())), rs2 = TraceReg(rs2, LogicMode::TwoValue, Some(size.into())) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )

        // FV Operations
        fv_not (EncUnary { rd: Reg, rs: Reg })
            {
                let (dspc, dval) = rd.to_spc_and_val();
                let (spc, val) = rs.to_spc_and_val();
                (regs[dspc], regs[dval]) = fv_bitwise_inv_elem(regs[spc], regs[val]);
            }
            { write!(f, "{rd}, {rs}") }
            ( rs = TraceReg(rs, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::FourValue, None) )
        fv_reduce_or (EncUnary { rd: Reg, rs: Reg, size: SixBitSize })
            {
                let (dspc, dval) = rd.to_spc_and_val();
                let (spc, val) = rs.to_spc_and_val();
                let value = fv_reduce_or_elem(regs[spc], regs[val], size.into());
                (regs[dspc], regs[dval]) = ((value as u64) & 1, (value as u64) >> 1);
            }
            { write!(f, "{rd}, {rs}, |{size}|") }
            ( rs = TraceReg(rs, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::FourValue, None) )
        fv_reduce_and (EncUnary { rd: Reg, rs: Reg, size: SixBitSize })
            {
                let (dspc, dval) = rd.to_spc_and_val();
                let (spc, val) = rs.to_spc_and_val();
                let value = fv_reduce_and_elem(regs[spc], regs[val], size.into());
                (regs[dspc], regs[dval]) = ((value as u64) & 1, (value as u64) >> 1);
            }
            { write!(f, "{rd}, {rs}, |{size}|") }
            ( rs = TraceReg(rs, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::FourValue, None) )
        fv_reduce_xor (EncUnary { rd: Reg, rs: Reg, size: SixBitSize })
            {
                let (dspc, dval) = rd.to_spc_and_val();
                let (spc, val) = rs.to_spc_and_val();
                let value = fv_reduce_xor_elem(regs[spc], regs[val], size.into());
                (regs[dspc], regs[dval]) = ((value as u64) & 1, (value as u64) >> 1);
            }
            { write!(f, "{rd}, {rs}, |{size}|") }
            ( rs = TraceReg(rs, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::FourValue, None) )
        fv_and (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            {
                let (dspc, dval) = rd.to_spc_and_val();
                let (spc1, val1) = rs1.to_spc_and_val();
                let (spc2, val2) = rs2.to_spc_and_val();
                (regs[dspc], regs[dval]) = fv_bitwise_and_elem(regs[spc1], regs[val1], regs[spc2], regs[val2]);
            }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::FourValue, None), rs2 = TraceReg(rs2, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::FourValue, None) )
        fv_or (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            {
                let (dspc, dval) = rd.to_spc_and_val();
                let (spc1, val1) = rs1.to_spc_and_val();
                let (spc2, val2) = rs2.to_spc_and_val();
                (regs[dspc], regs[dval]) = fv_bitwise_or_elem(regs[spc1], regs[val1], regs[spc2], regs[val2]);
            }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::FourValue, None), rs2 = TraceReg(rs2, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::FourValue, None) )
        fv_xor (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            {
                let (dspc, dval) = rd.to_spc_and_val();
                let (spc1, val1) = rs1.to_spc_and_val();
                let (spc2, val2) = rs2.to_spc_and_val();
                (regs[dspc], regs[dval]) = fv_bitwise_xor_elem(regs[spc1], regs[val1], regs[spc2], regs[val2]);
            }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::FourValue, None), rs2 = TraceReg(rs2, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::FourValue, None) )
        fv_ceq (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            {
                let (spc1, val1) = rs1.to_spc_and_val();
                let (spc2, val2) = rs2.to_spc_and_val();
                regs[rd] = u64::from((regs[spc1] == regs[spc2]) & (regs[val1] == regs[val2]));
            }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::FourValue, None), rs2 = TraceReg(rs2, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )
        fv_posedge (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            {
                let (spc1, val1) = rs1.to_spc_and_val();
                let (spc2, val2) = rs2.to_spc_and_val();
                regs[rd] = vogls_bits::edge::fv_posedge_u64(regs[spc1], regs[val1], regs[spc2], regs[val2]);
            }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::FourValue, None), rs2 = TraceReg(rs2, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        fv_negedge (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg })
            {
                let (spc1, val1) = rs1.to_spc_and_val();
                let (spc2, val2) = rs2.to_spc_and_val();
                regs[rd] = vogls_bits::edge::fv_negedge_u64(regs[spc1], regs[val1], regs[spc2], regs[val2]);
            }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ( rs1 = TraceReg(rs1, LogicMode::FourValue, None), rs2 = TraceReg(rs2, LogicMode::FourValue, None) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )
        fv_andi (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            {
                let imm = size.mask(i64::from(imm10) as u64);
                let (rd_spc, rd_val) = rd.to_spc_and_val();
                let (rs_spc, rs_val) = rs.to_spc_and_val();
                (regs[rd_spc], regs[rd_val]) = fv_bitwise_and_elem(
                    regs[rs_spc], regs[rs_val],
                    size.mask(u64::MAX), imm
                );
            }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::FourValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::FourValue, Some(size.into())) )
        fv_ori (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            {
                let imm = size.mask(i64::from(imm10) as u64);
                let (rd_spc, rd_val) = rd.to_spc_and_val();
                let (rs_spc, rs_val) = rs.to_spc_and_val();
                (regs[rd_spc], regs[rd_val]) = fv_bitwise_or_elem(
                    regs[rs_spc], regs[rs_val],
                    size.mask(u64::MAX), imm
                );
            }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::FourValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::FourValue, Some(size.into())) )
        fv_xori (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            {
                let imm = size.mask(i64::from(imm10) as u64);
                let (rd_spc, rd_val) = rd.to_spc_and_val();
                let (rs_spc, rs_val) = rs.to_spc_and_val();
                (regs[rd_spc], regs[rd_val]) = fv_bitwise_xor_elem(
                    regs[rs_spc], regs[rs_val],
                    size.mask(u64::MAX), imm
                );
            }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::FourValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::FourValue, Some(size.into())) )
        fv_ceqi (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            {
                let imm = size.mask(i64::from(imm10) as u64);
                let (rs_spc, rs_val) = rs.to_spc_and_val();
                regs[rd] = u64::from((regs[rs_spc] == size.mask(u64::MAX)) & (regs[rs_val] == imm));
            }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::FourValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )
        fv_cnei (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            {
                let imm = size.mask(i64::from(imm10) as u64);
                let (rs_spc, rs_val) = rs.to_spc_and_val();
                regs[rd] = u64::from((regs[rs_spc] != size.mask(u64::MAX)) | (regs[rs_val] != imm));
            }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs1 = TraceReg(rs, LogicMode::FourValue, Some(size.into())), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )

        push_argument (EncPushArgument { size: Option<VectorSize>, mode: LogicMode, rs: Reg })
            {
                let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
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
            { write!(f, "{rs}, {mode:?}") }
            (rs = TraceReg(rs, mode, size))
            ()
        intrinsic (EncIntrinsic { rd: Reg, id: Option<NonMaxU16> })
            {
                let id = id.map_or_else(|| regs[Reg::X14], |v| v.get() as u64);
                let intrinsic = &cldctx.intrinsics[id as usize];
                match intrinsic {
                    IntrinsicOp::Time => regs[rd] = state.time,
                    IntrinsicOp::Finish => todo!(),
                    IntrinsicOp::Random => todo!(),
                    IntrinsicOp::Display(_) => todo!(),
                    IntrinsicOp::Assert(f) => {
                        let mut stack_offset = 0usize;
                        let (cond_size, cond_mode) = cldctx.stack_args[0];
                        assert_eq!(cond_size, SCALAR_VSIZE);
                        let condition = match cond_mode {
                            LogicMode::TwoValue => {
                                stack_offset += 1;
                                cldctx.stack[0] != 0
                            },
                            LogicMode::FourValue => {
                                stack_offset += 2;
                                FvLogicValue::from_spc_and_val(cldctx.stack[0] != 0, cldctx.stack[1] != 0) == FvLogicValue::L1
                            }
                        };

                        if !condition {
                            f.write_to(
                                &mut cldctx.stderr,
                                cldctx.stack_args[1..].iter().map(|&(size, mode)| match mode {
                                    LogicMode::TwoValue if size <= VSIZE_64 => {
                                        let value = cldctx.stack[stack_offset];
                                        stack_offset += 1;
                                        Bits::from_u64(size, value)
                                    },
                                    LogicMode::TwoValue => {
                                        let value = cldctx.stack[stack_offset];
                                        let value = value_to_heap_ref(value, size, mode);
                                        stack_offset += 1;
                                        state.heap.load_tv_bits(value)
                                    },
                                    LogicMode::FourValue if size <= VSIZE_32  => {
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
                                    },
                                }),
                            )
                            .unwrap();
                            cldctx.return_value = 1;
                            pc = u64::MAX;
                        }
                    },
                    IntrinsicOp::VcdOpenFile(_) => todo!(),
                    IntrinsicOp::VcdAppendModule(_) => todo!(),
                    IntrinsicOp::VcdPause => todo!(),
                    IntrinsicOp::VcdResume => todo!(),
                    IntrinsicOp::ReadMem(_) => todo!(),
                }
                cldctx.stack_args.clear();
                cldctx.stack.clear();
            }
            { Ok(()) }
            ()
            ()

        tv_heap_bitwise (EncHeapBitwiseReg { rd: Reg, rs1: Reg, rs2: Reg, op: BitwiseOp, size: Option<VectorSize> })
            { execute_heap_bitwise(state, regs, rd, rs1, rs2, op, size) }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ()
            ()
        heap_case_eq (EncHeapCaseEq { rd: Reg, rs1: Reg, rs2: Reg, fv: bool, ne: bool, size: Option<VectorSize> })
            { execute_heap_case_eq(state, regs, rd, rs1, rs2, fv, ne, size) }
            { write!(f, "{rd}, {rs1}, {rs2}") }
            ()
            ()

        heap_unary (EncHeapUnaryReg { rd: Reg, rs: Reg, imm16: u16 })
            { execute_heap_unary(state, regs, rd, rs, imm16); }
            { write!(f, "{rd}, {rs}") }
            ( rd = TraceReg(rd, LogicMode::FourValue, None) )
            ( rs = TraceReg(rs, LogicMode::FourValue, None) )

        load_imm16 (EncLoadImm { rd: Reg, clear: bool, sign_extend: bool, segment: u8, imm: i16 })
            {
                if clear {
                    regs[rd] = 0;
                }
                let imm = if sign_extend {
                    i64::from(imm) as u64
                }  else {
                    imm as u16 as u64
                };
                regs[rd] |= imm << (segment * 16);
            }
            { write!(f, "{rd}, {imm}, c:{clear}, e:{sign_extend}, seg:{segment}") }
            ()
            ( rd = TraceReg(rd, LogicMode::TwoValue, None) )

        load_aligned (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            {
                // @Performance: We can likely make a better specialized implementation for this load.
                let size = VectorSize::from(size);
                let factor = i64::from(size.get().next_power_of_two().min(64));
                let offset = i64::from(imm10) * factor;
                let offset = regs[rs].wrapping_add_signed(i64::from(offset));
                let at = HeapOffset { bit_offset: offset as usize };
                let at = at.to_ref(size);
                regs[rd] = state.heap.get_tv_u64(at);
            }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs = TraceReg(rs, LogicMode::TwoValue, None), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(size.into())) )

        load_fv_aligned (EncBinaryImm { rd: Reg, rs: Reg, imm10: i16, size: SixBitSize })
            {
                // @Performance: We can likely make a better specialized implementation for this load.
                let size = VectorSize::from(size);
                let factor = i64::from((2*size.get()).next_power_of_two().min(64));
                let offset = i64::from(imm10) * factor;
                let offset = regs[rs].wrapping_add_signed(i64::from(offset));
                let at = HeapOffset { bit_offset: offset as usize };
                let at = at.to_ref(size);
                let (spc, val) = rd.to_spc_and_val();
                (regs[spc], regs[val]) = state.heap.get_fv_u64(at);
            }
            { write!(f, "{rd}, {rs}, {imm10}, |{size}|") }
            ( rs = TraceReg(rs, LogicMode::TwoValue, None), imm = imm10 )
            ( rd = TraceReg(rd, LogicMode::FourValue, Some(size.into())) )
        load_heap_aligned (EncUnaryUImm { rd: Reg, rs: Reg, imm: u16 })
            {
                let dst_offset = regs[rd];
                let src_offset = regs[rs];
                let num_words = imm as usize;
                let [dst, src] = state.heap.get_u64_cell_slices([
                    (HeapOffset { bit_offset: dst_offset as usize }, num_words),
                    (HeapOffset { bit_offset: src_offset as usize }, num_words),

                ]);
                for (d, s) in dst.iter().zip(src) {
                    d.set(s.get());
                }
            }
            { write!(f, "{rd}, {rs}, {imm}") }
            ( rs = TraceReg(rs, LogicMode::TwoValue, Some(VSIZE_64)) )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(VSIZE_64)) )

        set_aligned (EncSet { rd: Reg, rs: Reg, roff: Reg, imm: i8, size: SixBitSize })
            {
                // @Performance: We can likely make a better specialized implementation for this load.
                let size = VectorSize::from(size);
                let factor = i64::from(size.get().next_power_of_two());
                let offset = i64::from(imm) * factor;
                let offset = regs[roff].wrapping_add_signed(i64::from(offset));
                let at = HeapOffset { bit_offset: offset as usize };
                let at = at.to_ref(size);
                let value = regs[rs];
                let prev_value = state.heap.set_tv_u64(at, value);
                let updated = value != prev_value;
                regs[rd] = u64::from(updated);
            }
            { write!(f, "{rd}, {rs}, {roff}, {imm}, |{size}|") }
            (
                rs = TraceReg(rs, LogicMode::TwoValue, Some(size.into())),
                roff = TraceReg(roff, LogicMode::TwoValue, None),
                imm = imm
              )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )
        set_fv_aligned (EncSet { rd: Reg, rs: Reg, roff: Reg, imm: i8, size: SixBitSize })
            {
                // @Performance: We can likely make a better specialized implementation for this load.
                let size = VectorSize::from(size);
                let factor = i64::from((size.get() * 2).next_power_of_two().min(64));
                let offset = i64::from(imm) * factor;
                let offset = regs[roff].wrapping_add_signed(i64::from(offset));
                let at = HeapOffset { bit_offset: offset as usize };
                let at = at.to_ref(size);
                let (spc, val) = rs.to_spc_and_val();
                let spc = regs[spc];
                let val = regs[val];
                let (prev_spc, prev_val) = state.heap.set_fv_u64(at, spc, val);
                let updated = (prev_spc != spc) | (prev_val != val);
                regs[rd] = u64::from(updated);
            }
            { write!(f, "{rd}, {rs}, {roff}, {imm}, |{size}|") }
            (
                rs = TraceReg(rs, LogicMode::FourValue, Some(size.into())),
                roff = TraceReg(roff, LogicMode::TwoValue, None),
                imm = imm
              )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )
        set_heap_aligned (EncHeapSet { rd: Reg, rs: Reg, roff: Reg, size: Option<VectorSize> })
            {
                let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
                let roff_offset = regs[roff];
                let src_offset = regs[rs];
                let num_words = size.get().div_ceil(64) as usize;
                let [roff, src] = state.heap.get_u64_cell_slices([
                    (HeapOffset { bit_offset: roff_offset as usize }, num_words),
                    (HeapOffset { bit_offset: src_offset as usize }, num_words),

                ]);
                let mut updated = false;
                for (d, s) in roff.iter().zip(src) {
                    let value = s.get();
                    let prev_value = d.replace(value);
                    updated |= value != prev_value;
                }
                regs[rd] = u64::from(updated);
            }
            { write!(f, "{rd}, {rs}, {roff}") }
            (
                rs = TraceReg(rs, LogicMode::TwoValue, Some(VSIZE_64)),
                roff = TraceReg(roff, LogicMode::TwoValue, None),
              )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )
        set_fv_heap_aligned (EncHeapSet { rd: Reg, rs: Reg, roff: Reg, size: Option<VectorSize> })
            {
                let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
                let roff_offset = regs[roff];
                let src_offset = regs[rs];
                let num_words = size.get().div_ceil(64) as usize * 2;
                let [roff, src] = state.heap.get_u64_cell_slices([
                    (HeapOffset { bit_offset: roff_offset as usize }, num_words),
                    (HeapOffset { bit_offset: src_offset as usize }, num_words),

                ]);
                let mut updated = false;
                for (d, s) in roff.iter().zip(src) {
                    let value = s.get();
                    let prev_value = d.replace(value);
                    updated |= value != prev_value;
                }
                regs[rd] = u64::from(updated);
            }
            { write!(f, "{rd}, {rs}, {roff}") }
            (
                rs = TraceReg(rs, LogicMode::TwoValue, Some(VSIZE_64)),
                roff = TraceReg(roff, LogicMode::TwoValue, None),
              )
            ( rd = TraceReg(rd, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )

        wake (EncWake { rcond: Reg, index: u32 })
            {
                if regs[rcond] != 0 {
                    let i = index as usize;
                    let bit = 1u64 << (i % 64);
                    let is_listening = (listeners.active[i / 64] & bit) != 0;
                    if is_listening {
                        let offset = listeners.map[i];
                        schedule.active.push(offset);
                        listeners.active[i / 64] ^= bit;
                    }
                }
            }
            { write!(f, "{rcond}, {index}") }
            ( rcond = TraceReg(rcond, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )
            ()

        jump (EncJump { imm: i32 })
            { pc = pc.wrapping_sub(1).wrapping_add_signed(i64::from(imm)); }
            { write!(f, "{imm}") }
        reljump (EncRelJump { rs: Reg, imm: i32 })
            { pc = regs[rs].wrapping_add_signed(i64::from(imm)); }
            { write!(f, "{rs}, {imm}") }
            ( rs = TraceReg(rs, LogicMode::TwoValue, None) )
            ()
        branch (EncRelJump { rs: Reg, imm: i32 })
            { if regs[rs] != 0 { pc = pc.wrapping_sub(1).wrapping_add_signed(i64::from(imm)); } }
            { write!(f, "{rs}, {imm}") }
            ( rs = TraceReg(rs, LogicMode::TwoValue, Some(SCALAR_VSIZE)) )
            ()

        reschedule (EncReschedule { rtime: Reg, region: u8, schedule_self: bool })
            {
                if schedule_self {
                    if region == 0 {
                        let time = regs[rtime];
                        if time == 0 {
                            return pc;
                        }

                        let time = state.time + time;
                        schedule.next_time = schedule.next_time.min(time);
                        schedule.future.push(TimedEvent {
                            time,
                            pc: InstructionPtr(pc)
                        });
                    } else {
                        let region = region as usize;
                        if region == 1 {
                            return pc;
                        }

                        schedule.regions[region as usize - 2].push(InstructionPtr(pc));
                    }
                }

                pc = schedule.pop(&mut state.time).map_or(u64::MAX, |ptr| ptr.0);
            }
            {
                if schedule_self {
                    Ok(())
                } else if region == 0 {
                    write!(f, "{rtime}")
                } else {
                    write!(f, "{}", region - 1)
                }
            }
            : MNEMONIC: {
                if schedule_self {
                    "next_event"
                } else if region == 0 {
                    "wait"
                } else {
                    "wait_region"
                }
            }
            ()
            ()

        start_listen (EncUImm { imm: u32 })
            {
                let i = imm as usize;
                listeners.active[i / 64] |= 1u64 << (i % 64);
            }
            { write!(f, "{imm}") }
            ()
            ()

        panic (EncEmpty {})
            { panic!() }
            { Ok(()) }
            ()
            ()
    ]
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
        self.xori(rd, rs, -1, size);
    }
    pub fn copy(&mut self, rd: Reg, rs: Reg) {
        if rd == rs {
            return;
        }
        self.ori(rd, rs, 0, SixBitSize::N64);
    }
    pub fn truncate(&mut self, rd: Reg, rs: Reg, size: SixBitSize) {
        self.andi(rd, rs, -1, size);
    }
    pub fn load_u64(&mut self, rd: Reg, value: u64) {
        if value == 0 {
            self.xor(rd, rd, rd);
            return;
        }

        if value == u64::MAX {
            self.ori(rd, rd, -1, SixBitSize::N64);
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
        self.andi(rd, rs, -1, size)
    }

    pub fn wait(&mut self, rtime: Reg) {
        self.reschedule(rtime, 0, true);
    }

    pub fn wait_region(&mut self, region: u8) {
        self.reschedule(Reg::X0, region + 1, true);
    }

    pub fn next_event(&mut self) {
        self.reschedule(Reg::X0, 0, false);
    }

    pub fn heap_tv_zero_extend(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: VectorSize,
        src_size: VectorSize,
    ) {
        let dst_size = if dst_size.get() >= (1u32 << 7) {
            self.load_u64(Reg::X12, dst_size.get() as u64);
            0u16
        } else {
            dst_size.get() as u16
        };
        let src_size = if src_size.get() >= (1u32 << 6) {
            self.load_u64(Reg::X13, src_size.get() as u64);
            0u16
        } else {
            src_size.get() as u16
        };
        let imm16 = (0b100 << 13) | (dst_size << 6) | (src_size << 0);
        self.data
            .push(Bytecode(EncHeapUnaryReg { rd, rs, imm16 }.encode()));
    }

    pub fn heap_tv_bitwise(
        &mut self,
        rd: Reg,
        rs1: Reg,
        rs2: Reg,
        op: BitwiseOp,
        size: VectorSize,
    ) {
        let size = if size.get() >= (1u32 << 8) {
            self.load_u64(Reg::X12, size.get() as u64);
            None
        } else {
            Some(size)
        };
        self.tv_heap_bitwise(rd, rs1, rs2, op, size);
    }
    fn heap_ceq(
        &mut self,
        rd: Reg,
        rs1: Reg,
        rs2: Reg,
        fv: bool,
        ne: bool,
        size: VectorSize,
    ) {
        let size = if size.get() >= (1u32 << 10) {
            self.load_u64(Reg::X12, size.get() as u64);
            None
        } else {
            Some(size)
        };
        self.heap_case_eq(rd, rs1, rs2, fv, ne, size);
    }

    pub fn heap_tv_and(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_bitwise(rd, rs1, rs2, BitwiseOp::TvAnd, size);
    }
    pub fn heap_tv_or(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_bitwise(rd, rs1, rs2, BitwiseOp::TvOr, size);
    }
    pub fn heap_tv_xor(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_bitwise(rd, rs1, rs2, BitwiseOp::TvXor, size);
    }
    pub fn heap_tv_andnot(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_bitwise(rd, rs1, rs2, BitwiseOp::TvAndNot, size);
    }
    pub fn heap_tv_ornot(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_bitwise(rd, rs1, rs2, BitwiseOp::TvOrNot, size);
    }
    pub fn heap_fv_and(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_bitwise(rd, rs1, rs2, BitwiseOp::FvAnd, size);
    }
    pub fn heap_fv_or(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_bitwise(rd, rs1, rs2, BitwiseOp::FvOr, size);
    }
    pub fn heap_fv_xor(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_tv_bitwise(rd, rs1, rs2, BitwiseOp::FvXor, size);
    }

    pub fn heap_tv_ceq(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_ceq(rd, rs1, rs2, false, false, size);
    }
    pub fn heap_tv_cne(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_ceq(rd, rs1, rs2, false, true, size);
    }
    pub fn heap_fv_ceq(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_ceq(rd, rs1, rs2, true, false, size);
    }
    pub fn heap_fv_cne(&mut self, rd: Reg, rs1: Reg, rs2: Reg, size: VectorSize) {
        self.heap_ceq(rd, rs1, rs2, true, true, size);
    }

    pub fn stack_offset(&mut self, rd: Reg, kind: StackItemKind, offset: u64) {
        // @Performance: Specialized instruction.
        self.load_u64(rd, offset);
        use StackItemKind as K;
        match kind {
            K::B1 => {}
            K::B2 => self.slli(rd, rd, 1, SixBitSize::N64),
            K::B4 => self.slli(rd, rd, 2, SixBitSize::N64),
            K::B8 => self.slli(rd, rd, 3, SixBitSize::N64),
            K::B16 => self.slli(rd, rd, 4, SixBitSize::N64),
            K::B32 => self.slli(rd, rd, 5, SixBitSize::N64),
            K::B64 => self.slli(rd, rd, 6, SixBitSize::N64),
        }
        self.add(rd, rd, Reg::X15, SixBitSize::N64);
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

        let mut pc = entry.0;
        let mut cldctx = ColdContext::new(&self.intrinsics, stdout, stderr);
        let mut regs = Regs([0u64; _]);
        regs[Reg::X15] = self.stack_offset;
        while let Some(&c) = code.get(pc as usize) {
            let opcode = c.opcode();
            if ITRACE {
                writeln!(&mut cldctx.stderr, "[PC={pc:4}]: {c}").unwrap();
                writeln!(
                    &mut cldctx.stderr,
                    "  {}",
                    PreBytecodeTrace {
                        value: c,
                        regs: &regs
                    }
                )
                .unwrap();
            }
            let f = INSTRUCTION_FNS[opcode as usize];
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
                writeln!(
                    &mut cldctx.stderr,
                    "  {}",
                    PostBytecodeTrace {
                        value: c,
                        regs: &regs
                    }
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
        LogicMode::TwoValue => size.get().next_power_of_two().max(64),
        LogicMode::FourValue => (size.get() * 2).next_power_of_two().max(64),
    } as u64;
    let bit_offset = value.checked_mul(alignment).expect("stack overflow") as usize;
    HeapOffset { bit_offset }.to_ref(size)
}

#[derive(Clone, Copy)]
struct HeapUnaryUImm(u16);

enum HeapUnaryOp {
    TvTruncate,
    FvTruncate,

    TvZeroExtend,
    FvZeroExtend,

    TvSignExtend,
    FvSignExtend,
}

impl HeapUnaryUImm {
    pub fn extract_subopcode(self) -> HeapUnaryOp {
        match self.0 >> 13 {
            0b000 | 0b001 => todo!(),
            0b010 => HeapUnaryOp::TvTruncate,
            0b011 => HeapUnaryOp::FvTruncate,
            0b100 => HeapUnaryOp::TvZeroExtend,
            0b101 => HeapUnaryOp::FvZeroExtend,
            0b110 => HeapUnaryOp::TvSignExtend,
            _ => HeapUnaryOp::FvSignExtend,
        }
    }

    pub fn extract_sizes(self, regs: &Regs) -> (VectorSize, VectorSize) {
        let small = VectorSize::new(((self.0 >> 7) & 0x3F) as u32)
            .unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
        let large = VectorSize::new((self.0 & 0x7F) as u32)
            .unwrap_or_else(|| VectorSize::new(regs[Reg::X13] as u32).unwrap());
        (large, small)
    }
}

#[inline(always)]
fn execute_heap_unary(state: &mut RuntimeState, regs: &mut Regs, rd: Reg, rs: Reg, imm: u16) {
    let imm = HeapUnaryUImm(imm);
    match imm.extract_subopcode() {
        HeapUnaryOp::TvTruncate => {
            let (src_size, dst_size) = imm.extract_sizes(regs);
            let src_num_words = src_size.get().div_ceil(64) as usize;
            let src = state.heap.get_u64_slice(
                HeapOffset {
                    bit_offset: regs[rs] as usize,
                },
                src_num_words,
            );
            match SixBitSize::from_vector_size(dst_size) {
                None => {
                    let dst_num_words = dst_size.get().div_ceil(64) as usize;
                    // @Performance: Find a way not to have to allocate and copy here.
                    // The problem is that we want to be able to define compute kernels once,
                    // but the kernels for Bits are fundamentally different than these as it is
                    // exclusive vs. shared reference.
                    let mut dst_buffer = vec![0u64; dst_num_words];
                    tv_l_truncate(&mut dst_buffer, src, dst_size, src_size);
                    state
                        .heap
                        .get_mut_u64_slice(
                            HeapOffset {
                                bit_offset: regs[rd] as usize,
                            },
                            dst_num_words,
                        )
                        .copy_from_slice(&dst_buffer);
                }
                Some(dst_size) => regs[rd] = dst_size.mask(src[0]),
            }
        }
        HeapUnaryOp::FvTruncate => todo!(),
        HeapUnaryOp::TvZeroExtend => {
            let (dst_size, src_size) = imm.extract_sizes(regs);
            let dst_num_words = dst_size.get().div_ceil(64) as usize;
            match SixBitSize::from_vector_size(src_size) {
                None => {
                    let src_num_words = src_size.get().div_ceil(64) as usize;
                    let src = state.heap.get_u64_slice(
                        HeapOffset {
                            bit_offset: regs[rs] as usize,
                        },
                        src_num_words,
                    );
                    // @Performance: Find a way not to have to allocate and copy here.
                    // The problem is that we want to be able to define compute kernels once,
                    // but the kernels for Bits are fundamentally different than these as it is
                    // exclusive vs. shared reference.
                    let mut dst_buffer = vec![0u64; dst_num_words];
                    tv_l_zero_extend(&mut dst_buffer, src, dst_size, src_size);
                    state
                        .heap
                        .get_mut_u64_slice(
                            HeapOffset {
                                bit_offset: regs[rd] as usize,
                            },
                            dst_num_words,
                        )
                        .copy_from_slice(&dst_buffer);
                }
                Some(_) => {
                    let dst = state.heap.get_mut_u64_slice(
                        HeapOffset {
                            bit_offset: regs[rd] as usize,
                        },
                        dst_num_words,
                    );
                    dst[0] = regs[rs];
                    dst[1..].fill(0u64);
                }
            }
        }
        HeapUnaryOp::FvZeroExtend => todo!(),
        HeapUnaryOp::TvSignExtend => todo!(),
        HeapUnaryOp::FvSignExtend => todo!(),
    }
}

pub fn execute_heap_bitwise(
    state: &mut RuntimeState,
    regs: &mut Regs,
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    op: BitwiseOp,
    size: Option<VectorSize>,
) {
    let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
    let mut num_words = size.get().div_ceil(64) as usize;
    if op.is_four_value() {
        num_words *= 2;
    }
    let [dst, src1, src2] = state.heap.get_u64_cell_slices([
        (
            HeapOffset {
                bit_offset: regs[rd] as usize,
            },
            num_words,
        ),
        (
            HeapOffset {
                bit_offset: regs[rs1] as usize,
            },
            num_words,
        ),
        (
            HeapOffset {
                bit_offset: regs[rs2] as usize,
            },
            num_words,
        ),
    ]);

    use BitwiseOp as O;
    match op {
        O::TvAnd => tv_bin_u64_cell_bitwise_op(dst, src1, src2, |l, r| l & r),
        O::TvOr => tv_bin_u64_cell_bitwise_op(dst, src1, src2, |l, r| l | r),
        O::TvXor => tv_bin_u64_cell_bitwise_op(dst, src1, src2, |l, r| l ^ r),
        O::TvAndNot => {
            tv_bin_u64_cell_bitwise_mask_last_op(dst, src1, src2, |l, r| l & !r, size);
        }
        O::TvOrNot => {
            tv_bin_u64_cell_bitwise_mask_last_op(dst, src1, src2, |l, r| l | !r, size);
        }
        O::TvAdd => todo!(),
        O::TvSub => todo!(),
        O::TvMul => todo!(),

        O::FvAnd => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
            fv_bitwise_and_elem(lspc, lval, rspc, rval)
        }),
        O::FvOr => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
            fv_bitwise_or_elem(lspc, lval, rspc, rval)
        }),
        O::FvXor => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
            fv_bitwise_xor_elem(lspc, lval, rspc, rval)
        }),
        O::FvAndNot => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
            fv_bitwise_andnot_elem(lspc, lval, rspc, rval)
        }),
        O::FvOrNot => fv_bin_u64_cell_bitwise_op(dst, src1, src2, |lspc, lval, rspc, rval| {
            fv_bitwise_ornot_elem(lspc, lval, rspc, rval)
        }),
        O::FvAdd => todo!(),
        O::FvSub => todo!(),
        O::FvMul => todo!(),
    }
}

pub fn execute_heap_case_eq(
    state: &mut RuntimeState,
    regs: &mut Regs,
    rd: Reg,
    rs1: Reg,
    rs2: Reg,
    fv: bool,
    ne: bool,
    size: Option<VectorSize>,
) {
    let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
    let mut num_words = size.get().div_ceil(64) as usize;
    if fv {
        num_words *= 2;
    }
    let src1 = state.heap.get_u64_slice(
        HeapOffset {
            bit_offset: (regs[rs1] * 64) as usize,
        },
        num_words,
    );
    let src2 = state.heap.get_u64_slice(
        HeapOffset {
            bit_offset: (regs[rs2] * 64) as usize,
        },
        num_words,
    );

    let is_eq = src1 == src2;

    regs[rd] = u64::from(is_eq ^ ne);
}
