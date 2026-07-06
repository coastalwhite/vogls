use std::fmt;

use vogls_bits::extend::{
    fv_cell_sign_extend, fv_cell_zero_extend, fv_l_sign_extend, fv_l_zero_extend,
    tv_cell_sign_extend, tv_cell_zero_extend, tv_l_sign_extend, tv_l_zero_extend,
};
use vogls_ir::VectorSize;
use vogls_runtime::RuntimeState;

use crate::bytecode::BytecodeOpcode;

use super::reg::{Reg, Regs};
use super::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, ColdContext, InlineNBitSize,
    Schedule, SixBitSize, write_padded_mnemonic,
};

pub struct SignExtend {
    rd: Reg,
    rs: Reg,
    dst_size: SixBitSize,
    src_size: SixBitSize,
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

    fn execute(
        self,
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

        let [dst, src] = state
            .heap
            .get_u64_cell_slices([(dst, dst_num_words), (src, src_num_words)]);

        match op {
            ExtendOp::TvZeroExtend => tv_cell_zero_extend(dst, src, dst_size, src_size),
            ExtendOp::TvSignExtend => tv_cell_sign_extend(dst, src, dst_size, src_size),
            ExtendOp::FvZeroExtend => fv_cell_zero_extend(dst, src, dst_size, src_size),
            ExtendOp::FvSignExtend => fv_cell_sign_extend(dst, src, dst_size, src_size),
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
                | ((self.src_size.0 as u32) << 26),
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

    fn execute(
        self,
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
                | ((self.dst_size.get() as u32) << 16)
                | ((self.src_size.get() as u32) << 22),
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

    fn execute(
        self,
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

        regs[rd] = (((regs[rs] as i64) << (64 - src_size.get())) >> (64 - dst_size.get())) as u64;
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
}
