use std::fmt;
use std::ops::{Index, IndexMut};

use vogls_bits::arithmetic::{tv_addition, tv_bin_u64_bitwise_op, tv_multiplication, tv_subtraction};
use vogls_codegen::HeapOffset;

use vogls_ir::{VSIZE_64, VectorSize};
use vogls_runtime::RuntimeState;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bytecode(pub u32);

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
}

impl Bytecode {
    fn opcode(self) -> u8 {
        (self.0 & 0xFF) as u8
    }
}

impl fmt::Debug for Bytecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opcode = self.opcode();
        let mnemonic = MNEMONICS[opcode as usize];
        write!(f, "{mnemonic:<0$}", MAX_MNEMONIC_SIZE + 4)?;
        let arg_formatter = ARG_FORMATTER[opcode as usize];
        (arg_formatter)(*self, f)
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
        v & ((1u64 << self.0) - 1)
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

    // 10 bits
    imm: i16,
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

#[derive(Default, Debug)]
pub enum BitwiseOp {
    #[default]
    And,
    Or,
    Xor,
    AndNot,
    OrNot,
    Add,
    Sub,
    Mul,
}
impl BitwiseOp {
    fn new_masked(v: u32) -> Self {
        match v & 0x7 {
            0 => Self::And,
            1 => Self::Or,
            2 => Self::Xor,
            3 => Self::AndNot,
            4 => Self::OrNot,
            5 => Self::Add,
            6 => Self::Sub,
            _ => Self::Mul,
        }
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

pub trait Encoding: Sized {
    fn extract(bytecode: Bytecode) -> Self;
    fn encode(self) -> u32;
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
    fn extract_and_fmt(bytecode: Bytecode, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Self::extract(bytecode).fmt(f)
    }
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

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs, size } = self;
        let size = VectorSize::from(*size);
        write!(f, "{rd}, {rs}, |{size}|")
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
            imm: (v >> 22) as i32 as i16,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.rs as u32) << 12)
            | ((self.size.0 as u32) << 16)
            | ((self.imm as u16 as u32) << 22)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs, imm, size } = self;
        let size = VectorSize::from(*size);
        write!(f, "{rd}, {rs}, {imm}, |{size}|")
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

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            roff,
            imm,
            size,
        } = self;
        let size = VectorSize::from(*size);
        write!(f, "{rd}, {rs}, {roff}, {imm}, |{size}|")
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

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rd, rs1, rs2, size } = self;
        let size = VectorSize::from(*size);
        write!(f, "{rd}, {rs1}, {rs2}, |{size}|")
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
            size: VectorSize::new(v >> 23),
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rd as u32) << 8)
            | ((self.rs1 as u32) << 12)
            | ((self.rs2 as u32) << 16)
            | ((self.op as u32) << 20)
            | (self.size.map_or(0, |v| v.get()) << 23)
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs1,
            rs2,
            op,
            size,
        } = self;
        match size {
            None => write!(f, "{rd}, {rs1}, {rs2}, {op:?}, |x12|"),
            Some(size) => write!(f, "{rd}, {rs1}, {rs2}, {op:?} |{size}|"),
        }
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
            imm: (v >> 16) as i16,
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

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            clear,
            sign_extend,
            segment,
            imm,
        } = self;
        write!(f, "{rd}, {imm}, s:{segment}, c:{clear}, e:{sign_extend}")
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

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rcond, index } = self;
        write!(f, "{rcond}, {index}")
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

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { imm } = self;
        write!(f, "{imm}")
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

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { rs, imm } = self;
        write!(f, "{rs}, {imm}")
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

    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl Encoding for EncReschedule {
    #[inline(always)]
    fn extract(bytecode: Bytecode) -> Self {
        let v = bytecode.0;
        Self {
            rtime: Reg::new_masked(v >> 8),
            region: ((v >> 12) & 0xFF) as u8,
            schedule_self: (v >> 13) & 1 != 0,
        }
    }
    #[inline(always)]
    fn encode(self) -> u32 {
        ((self.rtime as u32) << 8)
            | ((self.region as u32) << 12)
            | (self.schedule_self as u32) << 13
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rtime,
            region,
            schedule_self,
        } = self;
        write!(f, "{rtime}, {region}, {schedule_self}")
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

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { imm } = self;
        write!(f, "{imm}")
    }
}

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

macro_rules! define_instructions {
    (
        $regs:ident, $pc_var: ident, $state:ident, $schedule:ident, $listeners:ident,
        [$(
        $name:ident ($enc_variant:ident { $($param:ident: $param_ty:ty),* }) $blk:block $($pc:block)?
    )+]) => {
        #[repr(u8)]
        #[expect(non_camel_case_types)]
        pub enum BytecodeKind {
            $($name,)*
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
        const ARG_FORMATTER: [fn(v: Bytecode, &mut fmt::Formatter<'_>) -> fmt::Result; NUM_INSTRUCTIONS] = [$($enc_variant::extract_and_fmt),+];

        $(
        fn $name(
            c: Bytecode,
            $regs: &mut Regs,
            $pc_var: u64,
            $state: &mut RuntimeState,
            $schedule: &mut Schedule,
            $listeners: &mut BytecodeListeners,
        ) -> u64 {
            _ = $regs;
            _ = $state;
            _ = $schedule;
            _ = $listeners;
            let $enc_variant { $($param,)* .. } = $enc_variant::extract(c);
            $blk
            #[allow(path_statements, unused_must_use)]
            { $pc_var + 1 }
            $(; $pc )?
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
    regs, pc, state, schedule, listeners,
    [
        // TV Operations
        and (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg }) {
            regs[rd] = regs[rs1] & regs[rs2];
        }
        or (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg }) {
            regs[rd] = regs[rs1] | regs[rs2];
        }
        xor (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg }) {
            regs[rd] = regs[rs1] ^ regs[rs2];
        }
        or_not (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg }) {
            regs[rd] = regs[rs1] | !regs[rs2];
        }
        and_not (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg }) {
            regs[rd] = regs[rs1] & !regs[rs2];
        }
        xnor (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg }) {
            regs[rd] = !(regs[rs1] ^ regs[rs2]);
        }
        ceq (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg }) {
            regs[rd] = u64::from(regs[rs1] == regs[rs2]);
        }
        andi (EncBinaryImm { rd: Reg, rs: Reg, imm: i16, size: SixBitSize }) {
            let imm = i64::from(imm);
            let imm = imm as u64;
            let imm = size.mask(imm);
            regs[rd] = regs[rs] & imm;
        }
        ori (EncBinaryImm { rd: Reg, rs: Reg, imm: i16, size: SixBitSize }) {
            let imm = i64::from(imm);
            let imm = imm as u64;
            let imm = size.mask(imm);
            regs[rd] = regs[rs] | imm;
        }
        xori (EncBinaryImm { rd: Reg, rs: Reg, imm: i16, size: SixBitSize }) {
            let imm = i64::from(imm);
            let imm = imm as u64;
            let imm = size.mask(imm);
            regs[rd] = regs[rs] ^ imm;
        }
        ceqi (EncBinaryImm { rd: Reg, rs: Reg, imm: i16, size: SixBitSize }) {
            let imm = i64::from(imm);
            let imm = imm as u64;
            let imm = size.mask(imm);
            regs[rd] = u64::from(regs[rs] == imm);
        }
        cnei (EncBinaryImm { rd: Reg, rs: Reg, imm: i16, size: SixBitSize }) {
            let imm = i64::from(imm);
            let imm = imm as u64;
            let imm = size.mask(imm);
            regs[rd] = u64::from(regs[rs] != imm);
        }
        count_ones (EncUnary { rd: Reg, rs: Reg }) {
            regs[rd] = u64::from(regs[rs].count_ones());
        }
        truncate (EncUnary { rd: Reg, rs: Reg, size: SixBitSize }) {
            regs[rd] = size.mask(regs[rs]);
        }

        add (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize }) {
            let sum = regs[rs1].wrapping_add(regs[rs2]);
            regs[rd] = size.mask(sum);
        }
        sub (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize }) {
            let sum = regs[rs1].wrapping_sub(regs[rs2]);
            regs[rd] = size.mask(sum);
        }
        mul (EncBinaryReg { rd: Reg, rs1: Reg, rs2: Reg, size: SixBitSize }) {
            let sum = regs[rs1].wrapping_mul(regs[rs2]);
            regs[rd] = size.mask(sum);
        }

        tv_heap_bitwise (EncHeapBitwiseReg { rd: Reg, rs1: Reg, rs2: Reg, op: BitwiseOp, size: Option<VectorSize> }) {
            let size = size.unwrap_or_else(|| VectorSize::new(regs[Reg::X12] as u32).unwrap());
            let num_words = size.get().div_ceil(64) as usize;
            let dst = HeapOffset { bit_offset: (regs[rd] * 64) as usize };
            let src1 = HeapOffset { bit_offset: (regs[rs1] * 64) as usize };
            let src2 = HeapOffset { bit_offset: (regs[rs2] * 64) as usize };
            let (dst, src1, src2) = state.heap.get_disjoint_u64_dst_s1_s2(
                (dst, num_words),
                (src1, num_words),
                (src2, num_words),
            );
            
            // @TODO: These are not disjoint, so this is wrong.
            use BitwiseOp as O;
            match op {
                O::And => tv_bin_u64_bitwise_op(dst, src1, src2, |l, r| l & r),
                O::Or => tv_bin_u64_bitwise_op(dst, src1, src2, |l, r| l | r),
                O::Xor => tv_bin_u64_bitwise_op(dst, src1, src2, |l, r| l ^ r),
                O::AndNot => todo!(),
                O::OrNot => todo!(),
                O::Add => tv_addition(dst, src1, src2, size),
                O::Sub => tv_subtraction(dst, src1, src2, size),
                O::Mul => tv_multiplication(dst, src1, src2, size),
            }
        }

        load_imm16 (EncLoadImm { rd: Reg, clear: bool, sign_extend: bool, segment: u8, imm: i16 }) {
            if clear {
                regs[rd] = 0;
            }
            let imm = if sign_extend {
                i64::from(imm) as u64
            }  else {
                imm as u64
            };
            regs[rd] |= imm << (segment * 16);
        }

        load_aligned (EncBinaryImm { rd: Reg, rs: Reg, imm: i16, size: SixBitSize }) {
            // @Performance: We can likely make a better specialized implementation for this load.
            let size = VectorSize::from(size);
            let factor = i64::from(size.get().next_power_of_two());
            let offset = i64::from(imm) * factor;
            let offset = regs[rs].wrapping_add_signed(i64::from(offset));
            let at = HeapOffset { bit_offset: offset as usize };
            let at = at.to_ref(size);
            regs[rd] = state.heap.get_tv_u64(at);
        }
        set_aligned (EncSet { rd: Reg, rs: Reg, roff: Reg, imm: i8, size: SixBitSize }) {
            // @Performance: We can likely make a better specialized implementation for this load.
            let size = VectorSize::from(size);
            let factor = i64::from(size.get().next_power_of_two());
            let offset = i64::from(imm) * factor;
            let offset = regs[roff].wrapping_add_signed(i64::from(offset));
            let at = HeapOffset { bit_offset: offset as usize };
            let at = at.to_ref(size);
            let value = regs[rs];
            let updated = value != state.heap.set_tv_u64(at, value);
            regs[rd] = u64::from(updated);
        }

        wake (EncWake { rcond: Reg, index: u32 }) {
            if regs[rcond] == 0{
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

        jump (EncJump { imm: i32 }) {} {
            pc.wrapping_add_signed(i64::from(imm))
        }
        reljump (EncRelJump { rs: Reg, imm: i32 }) {} {
            regs[rs].wrapping_add_signed(i64::from(imm))
        }
        branch (EncRelJump { rs: Reg, imm: i32 }) {} {
            if regs[rs] != 0 {
                pc.wrapping_add_signed(i64::from(imm))
            } else {
                pc
            }
        }

        reschedule (EncReschedule { rtime: Reg, region: u8, schedule_self: bool }) {} {
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
                        pc: InstructionPtr(pc + 1)
                    });
                } else {
                    let region = region as usize;
                    if region == 1 {
                        return pc;
                    }

                    schedule.regions[region as usize - 2].push(InstructionPtr(pc + 1));
                }
            }

            schedule.pop(&mut state.time).map_or(u64::MAX, |ptr| ptr.0)
        }

        start_listen (EncUImm { imm: u32 }) {
            let i = imm as usize;
            listeners.active[i / 64] |= 1u64 << (i % 64);
        }
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
}

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
}

pub fn execute(
    code: &[Bytecode],
    entry: u64,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
) {
    let mut pc = entry;
    let mut regs = Regs([0u64; _]);
    while let Some(&c) = code.get(pc as usize) {
        eprintln!("[PC={pc:4}]: {c:?}");
        let opcode = c.opcode();
        let f = INSTRUCTION_FNS[opcode as usize];
        pc = (f)(c, &mut regs, pc, state, schedule, listeners);
    }
}
