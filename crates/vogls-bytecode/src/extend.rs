use std::fmt;

use vogls_bits::extend::{fv_l_sign_extend, fv_l_zero_extend, tv_l_sign_extend, tv_l_zero_extend};
use vogls_bits::truncate::{fv_cell_truncate, tv_cell_truncate};
use vogls_ir::{LogicMode, VectorSize};
use vogls_runtime::RuntimeState;

use crate::BytecodeOpcode;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, ColdContext,
    EXEC_ITRACE_INDENT, InlineNBitSize, Schedule, SixBitSize, write_padded_mnemonic,
    write_register,
};

pub struct SignExtend {
    rd: Reg,
    rs: Reg,
    dst_size: SixBitSize,
    src_size: SixBitSize,
}

pub struct HeapHeapTruncate {
    rd: Reg,
    rs: Reg,

    src_size: Reg,
    fv: bool,
    dst_size: InlineNBitSize<13>,
}

pub struct HeapHeapExtend {
    rd: Reg,
    rs: Reg,

    dst_size: Reg,
    op: ExtendOp,
    src_size: InlineNBitSize<12>,
}

pub struct HeapRegExtend {
    rd: Reg,
    rs: Reg,

    op: ExtendOp,
    dst_size: InlineNBitSize<8>,
    src_size: SixBitSize,
}

#[derive(Clone, Copy)]
pub enum ExtendOp {
    TvZeroExtend,
    TvSignExtend,
    FvZeroExtend,
    FvSignExtend,
}

impl ExtendOp {
    pub fn new_masked(v: u32) -> Self {
        match v & 0x3 {
            0 => Self::TvZeroExtend,
            1 => Self::TvSignExtend,
            2 => Self::FvZeroExtend,
            _ => Self::FvSignExtend,
        }
    }

    pub fn is_four_value(self) -> bool {
        match self {
            Self::TvZeroExtend | Self::TvSignExtend => false,
            Self::FvZeroExtend | Self::FvSignExtend => true,
        }
    }
}

impl BytecodeInstruction for HeapHeapExtend {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::HeapHeapExtend as u8);
        let v = v.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            dst_size: Reg::new_masked(v >> 16),
            op: ExtendOp::new_masked(v >> 20),
            src_size: InlineNBitSize::new_masked(v >> 22),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapHeapExtend as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.dst_size as u32) << 16)
                | ((self.op as u32) << 20)
                | (self.src_size.encode() << 22),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            dst_size,
            op,
            src_size,
        } = self;
        let mnemonic = match op {
            ExtendOp::TvZeroExtend => "tv.heapheap_zero_extend",
            ExtendOp::TvSignExtend => "tv.heapheap_sign_extend",
            ExtendOp::FvZeroExtend => "fv.heapheap_zero_extend",
            ExtendOp::FvSignExtend => "fv.heapheap_sign_extend",
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs}, {dst_size}, {src_size}")
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs,
            dst_size,
            op,
            src_size,
        } = self;
        let dst_size = VectorSize::new(regs[dst_size].try_into().unwrap()).unwrap();
        let src_size = src_size.get(regs);
        let dst = regs.get_as_addr(rd);
        let src = regs.get_as_addr(rs);

        let mut dst_num_words = dst_size.get().div_ceil(64) as usize;
        let mut src_num_words = src_size.get().div_ceil(64) as usize;

        if op.is_four_value() {
            dst_num_words *= 2;
            src_num_words *= 2;
        }

        let src = state.heap.get_u64_slice(src, src_num_words);
        cldctx.heap_scratch.clear();
        cldctx.heap_scratch.resize(dst_num_words, 0u64);
        match op {
            ExtendOp::TvZeroExtend => {
                tv_l_zero_extend(&mut cldctx.heap_scratch, src, dst_size, src_size)
            }
            ExtendOp::TvSignExtend => {
                tv_l_sign_extend(&mut cldctx.heap_scratch, src, dst_size, src_size)
            }
            ExtendOp::FvZeroExtend => {
                fv_l_zero_extend(&mut cldctx.heap_scratch, src, dst_size, src_size)
            }
            ExtendOp::FvSignExtend => {
                fv_l_sign_extend(&mut cldctx.heap_scratch, src, dst_size, src_size)
            }
        }
        state
            .heap
            .get_mut_u64_slice(dst, dst_num_words)
            .copy_from_slice(&cldctx.heap_scratch);
    }
}

impl BytecodeInstruction for HeapHeapTruncate {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::HeapHeapTruncate as u8);
        let v = v.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            src_size: Reg::new_masked(v >> 16),
            fv: (v >> 20) & 1 != 0,
            dst_size: InlineNBitSize::new_masked(v >> 21),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapHeapTruncate as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.dst_size.encode() << 16)
                | ((self.fv as u32) << 20)
                | ((self.src_size as u32) << 21),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            dst_size,
            fv,
            src_size,
        } = self;
        let mnemonic = if *fv {
            "fv.heapheap_truncate"
        } else {
            "tv.heapheap_truncate"
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs}, {dst_size}, {src_size}")
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs,
            dst_size,
            fv,
            src_size,
        } = self;
        let src_size = VectorSize::new(regs[src_size].try_into().unwrap()).unwrap();
        let dst_size = dst_size.get(regs);
        let dst = regs.get_as_addr(rd);
        let src = regs.get_as_addr(rs);

        let mut dst_num_words = dst_size.get().div_ceil(64) as usize;
        let mut src_num_words = src_size.get().div_ceil(64) as usize;

        if fv {
            dst_num_words *= 2;
            src_num_words *= 2;
        }

        let [dst, src] = state
            .heap
            .get_u64_cell_slices([(dst, dst_num_words), (src, src_num_words)]);

        if fv {
            fv_cell_truncate(dst, src, dst_size, src_size);
        } else {
            tv_cell_truncate(dst, src, dst_size, src_size);
        }
    }
}

impl BytecodeInstruction for HeapRegExtend {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::HeapRegExtend as u8);
        let v = v.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            op: ExtendOp::new_masked(v >> 16),
            dst_size: InlineNBitSize::new_masked(v >> 18),
            src_size: SixBitSize::new_masked(v >> 26),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::HeapRegExtend as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | ((self.op as u32) << 16)
                | (self.dst_size.encode() << 18)
                | (self.src_size.encode() << 26),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            dst_size,
            op,
            src_size,
        } = self;
        let mnemonic = match op {
            ExtendOp::TvZeroExtend => "tv.heapheap_zero_extend",
            ExtendOp::TvSignExtend => "tv.heapheap_sign_extend",
            ExtendOp::FvZeroExtend => "fv.heapheap_zero_extend",
            ExtendOp::FvSignExtend => "fv.heapheap_sign_extend",
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rd}, {rs}, {dst_size}, {src_size}")
    }

    #[inline(always)]
    fn execute(
        self,
        _code: &[Bytecode],
        regs: &mut Regs,
        _pc: &mut u64,
        state: &mut RuntimeState,
        _schedule: &mut Schedule,
        _listeners: &mut BytecodeListeners,
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs,
            dst_size,
            op,
            src_size,
        } = self;
        let dst_size = dst_size.get(regs);
        let dst = regs.get_as_addr(rd);

        let mut dst_num_words = dst_size.get().div_ceil(64) as usize;
        if op.is_four_value() {
            dst_num_words *= 2;
        }

        let dst = state.heap.get_mut_u64_slice(dst, dst_num_words);
        let mut src = [0u64, 0u64];
        let src = if op.is_four_value() {
            let (spc, val) = rs.to_spc_and_val();
            src = [regs[spc], regs[val]];
            &src[..]
        } else {
            src[0] = regs[rs];
            &src[..1]
        };

        match op {
            ExtendOp::TvZeroExtend => tv_l_zero_extend(dst, src, dst_size, src_size.into()),
            ExtendOp::TvSignExtend => tv_l_sign_extend(dst, src, dst_size, src_size.into()),
            ExtendOp::FvZeroExtend => fv_l_zero_extend(dst, src, dst_size, src_size.into()),
            ExtendOp::FvSignExtend => fv_l_sign_extend(dst, src, dst_size, src_size.into()),
        }
    }
}

impl BytecodeInstruction for SignExtend {
    fn extract(v: Bytecode) -> Self {
        debug_assert_eq!(v.opcode(), BytecodeOpcode::SignExtend as u8);
        let v = v.0;
        Self {
            rd: Reg::new_masked(v >> 8),
            rs: Reg::new_masked(v >> 12),
            dst_size: SixBitSize::new_masked(v >> 16),
            src_size: SixBitSize::new_masked(v >> 22),
        }
    }

    fn encode(&self) -> Bytecode {
        Bytecode(
            BytecodeOpcode::SignExtend as u32
                | ((self.rd as u32) << 8)
                | ((self.rs as u32) << 12)
                | (self.dst_size.encode() << 16)
                | (self.src_size.encode() << 22),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            rd,
            rs,
            dst_size,
            src_size,
        } = self;
        write_padded_mnemonic(f, "sign_extend")?;
        write!(f, "{rd}, {rs}, {dst_size}, {src_size}")
    }

    fn post_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rs", self.rs, LogicMode::TwoValue)?;
        writeln!(f)
    }
    fn pre_exec_itrace(
        &self,
        f: &mut fmt::Formatter<'_>,
        _code: &[Bytecode],
        _pc: u64,
        regs: &Regs,
        _state: &RuntimeState,
    ) -> fmt::Result {
        f.write_str(EXEC_ITRACE_INDENT)?;
        write_register(f, regs, "rd", self.rd, LogicMode::TwoValue)?;
        writeln!(f)
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
        _cldctx: &mut ColdContext,
    ) {
        let Self {
            rd,
            rs,
            dst_size,
            src_size,
        } = self;

        let shift = 64 - src_size as u32;
        regs[rd] = dst_size.mask((((regs[rs] as i64) << shift) >> shift) as u64);
    }
}

impl BytecodeEncoder {
    pub fn heapheap_tv_zero_extend(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: Reg,
        src_size: InlineNBitSize<12>,
    ) {
        self.data.push(
            HeapHeapExtend {
                rd,
                rs,
                dst_size,
                op: ExtendOp::TvZeroExtend,
                src_size,
            }
            .encode(),
        );
    }
    pub fn heapheap_tv_sign_extend(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: Reg,
        src_size: InlineNBitSize<12>,
    ) {
        self.data.push(
            HeapHeapExtend {
                rd,
                rs,
                dst_size,
                op: ExtendOp::TvSignExtend,
                src_size,
            }
            .encode(),
        );
    }
    pub fn heapheap_fv_zero_extend(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: Reg,
        src_size: InlineNBitSize<12>,
    ) {
        self.data.push(
            HeapHeapExtend {
                rd,
                rs,
                dst_size,
                op: ExtendOp::FvZeroExtend,
                src_size,
            }
            .encode(),
        );
    }
    pub fn heapheap_fv_sign_extend(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: Reg,
        src_size: InlineNBitSize<12>,
    ) {
        self.data.push(
            HeapHeapExtend {
                rd,
                rs,
                dst_size,
                op: ExtendOp::FvSignExtend,
                src_size,
            }
            .encode(),
        );
    }

    pub fn heapreg_tv_zero_extend(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: InlineNBitSize<8>,
        src_size: SixBitSize,
    ) {
        self.data.push(
            HeapRegExtend {
                rd,
                rs,
                op: ExtendOp::TvZeroExtend,
                dst_size,
                src_size,
            }
            .encode(),
        );
    }
    pub fn heapreg_tv_sign_extend(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: InlineNBitSize<8>,
        src_size: SixBitSize,
    ) {
        self.data.push(
            HeapRegExtend {
                rd,
                rs,
                dst_size,
                op: ExtendOp::TvSignExtend,
                src_size,
            }
            .encode(),
        );
    }
    pub fn heapreg_fv_zero_extend(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: InlineNBitSize<8>,
        src_size: SixBitSize,
    ) {
        self.data.push(
            HeapRegExtend {
                rd,
                rs,
                dst_size,
                op: ExtendOp::FvZeroExtend,
                src_size,
            }
            .encode(),
        );
    }
    pub fn heapreg_fv_sign_extend(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: InlineNBitSize<8>,
        src_size: SixBitSize,
    ) {
        self.data.push(
            HeapRegExtend {
                rd,
                rs,
                dst_size,
                op: ExtendOp::FvSignExtend,
                src_size,
            }
            .encode(),
        );
    }

    pub fn sign_extend(&mut self, rd: Reg, rs: Reg, dst_size: SixBitSize, src_size: SixBitSize) {
        self.data.push(
            SignExtend {
                rd,
                rs,
                dst_size,
                src_size,
            }
            .encode(),
        );
    }

    pub fn heapheap_tv_truncate(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: InlineNBitSize<13>,
        src_size: Reg,
    ) {
        self.data.push(
            HeapHeapTruncate {
                rd,
                rs,
                dst_size,
                fv: false,
                src_size,
            }
            .encode(),
        );
    }
    pub fn heapheap_fv_truncate(
        &mut self,
        rd: Reg,
        rs: Reg,
        dst_size: InlineNBitSize<13>,
        src_size: Reg,
    ) {
        self.data.push(
            HeapHeapTruncate {
                rd,
                rs,
                dst_size,
                fv: true,
                src_size,
            }
            .encode(),
        );
    }
}
