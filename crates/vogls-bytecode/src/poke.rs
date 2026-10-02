use std::fmt;

use vogls_bits::arithmetic::FvLogicValue;
use vogls_bits::edge::{fv_negedge, fv_posedge};
use vogls_codegen::HeapAlignment;
use vogls_ir::{LogicMode, SCALAR_VSIZE, VSIZE_32, VSIZE_64, VectorSize};
use vogls_runtime::RuntimeState;

use crate::reg::{Reg, RegInfo, RegStorage, Regs};
use crate::{
    Bytecode, BytecodeEncoder, BytecodeInstruction, BytecodeListeners, BytecodeOpcode, ColdContext,
    InlineNBitSize, Schedule, decode_bytecode_u64, encode_bytecode_u64, wake_at_index,
    write_padded_mnemonic,
};

pub struct TvPokePosedge {
    rs: Reg,
    offset6: u8,
    imm14: u16,
}

pub struct FvPokePosedge {
    rs: Reg,
    offset6: u8,
    imm14: u16,
}

pub struct TvPokeNegedge {
    rs: Reg,
    offset6: u8,
    imm14: u16,
}

pub struct FvPokeNegedge {
    rs: Reg,
    offset6: u8,
    imm14: u16,
}

/// Trigger an watcher if a specific two-valued/four-valued positive/negative edge occurs.
///
/// The `before` is loaded from the heap as a specific offset into a heapslot with the given
/// `size`.
/// The `after` is loaded from `(regs[rs] >> offset6) & 1`.
pub struct PokeEdgeRelative {
    rs: Reg,
    roff: Reg,
    is_fv: bool,
    is_posedge: bool,
    target_size: InlineNBitSize<8>,
    source_size: InlineNBitSize<6>,
}

macro_rules! impl_rs_edge {
    ($name:ident, $mnemonic:literal) => {
        #[inline(always)]
        fn extract(c: Bytecode) -> Self {
            debug_assert_eq!(c.opcode(), BytecodeOpcode::$name as u8);
            let v = c.0;
            Self {
                rs: Reg::new_masked(v >> 8),
                offset6: ((v >> 12) & 0x3F) as u8,
                imm14: ((v >> 18) as u16),
            }
        }

        #[inline(always)]
        fn encode(&self) -> Bytecode {
            Bytecode(
                (BytecodeOpcode::$name as u32)
                    | ((self.rs as u32) << 8)
                    | ((self.offset6 as u32) << 12)
                    | ((self.imm14 as u32) << 18),
            )
        }

        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
            let Self {
                rs,
                offset6,
                imm14: _,
            } = self;
            write_padded_mnemonic(f, $mnemonic)?;
            write!(f, "{rs}, {offset6}")
        }

        #[inline(always)]
        fn num_additional_slots(&self) -> u8 {
            let mut slots = 0;
            slots += 1; // Offset
            slots += 2; // Watch start
            slots += 2; // Watch end
            slots
        }

        fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
            operands.push(RegInfo::new(
                "rs",
                self.rs,
                LogicMode::TwoValue,
                None,
                RegStorage::Register,
            ));
        }

        fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}
    };
}

impl BytecodeInstruction for TvPokePosedge {
    impl_rs_edge!(TvPokePosedge, "tv_poke_posedge");

    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rs, offset6, imm14 } = self;
        execute_edge1::<false, true>(
            rs, offset6, imm14, code, regs, pc, state, schedule, listeners, cldctx,
        );
    }
}

impl BytecodeInstruction for TvPokeNegedge {
    impl_rs_edge!(TvPokeNegedge, "tv_poke_negedge");

    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rs, offset6, imm14 } = self;
        execute_edge1::<false, false>(
            rs, offset6, imm14, code, regs, pc, state, schedule, listeners, cldctx,
        );
    }
}

impl BytecodeInstruction for FvPokePosedge {
    impl_rs_edge!(FvPokePosedge, "fv_poke_posedge");

    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rs, offset6, imm14 } = self;
        execute_edge1::<true, true>(
            rs, offset6, imm14, code, regs, pc, state, schedule, listeners, cldctx,
        );
    }
}

impl BytecodeInstruction for FvPokeNegedge {
    impl_rs_edge!(FvPokeNegedge, "fv_poke_negedge");

    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self { rs, offset6, imm14 } = self;
        execute_edge1::<true, false>(
            rs, offset6, imm14, code, regs, pc, state, schedule, listeners, cldctx,
        );
    }
}

fn execute_edge1<const IS_FV: bool, const IS_POSEDGE: bool>(
    rs: Reg,
    offset6: u8,
    imm14: u16,
    code: &[Bytecode],
    regs: &mut Regs,
    pc: &mut u64,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    cldctx: &mut ColdContext<'_>,
) {
    // @NOTE: We ensured the code buffer is padded with at least 16 elements.
    let additional_slots: &[Bytecode; 16] = code
        .get(*pc as usize..)
        .and_then(<[_]>::first_chunk)
        .expect("Should be padded");
    *pc += 5;

    let bit_offset = (imm14 as u64) | ((additional_slots[0].0 as u64) << 14);
    let woff = bit_offset / 64;
    let boff = bit_offset % 64;

    let word = state.heap.0[woff as usize];
    let is_edge = if IS_FV {
        let before = FvLogicValue::from_repr(((word >> boff) & 3) as u8);
        let (spc, val) = rs.to_spc_and_val();
        let spc = (regs[spc] >> offset6) & 1 != 0;
        let val = (regs[val] >> offset6) & 1 != 0;
        let after = FvLogicValue::from_spc_and_val(spc, val);
        if IS_POSEDGE {
            fv_posedge(before, after)
        } else {
            fv_negedge(before, after)
        }
    } else {
        let before = (word >> boff) & 1;
        let after = (regs[rs] >> offset6) & 1;
        if IS_POSEDGE {
            (after & !before) != 0
        } else {
            (before & !after) != 0
        }
    };

    if is_edge {
        let watchers = cldctx.watchers.watchers.as_slice();
        let watch_start = decode_bytecode_u64(&additional_slots[1..]);
        let watch_end = decode_bytecode_u64(&additional_slots[3..]);
        for watch_index in &watchers[watch_start as usize..watch_end as usize] {
            wake_at_index(*watch_index, schedule, listeners);
        }
    }
}

impl BytecodeInstruction for PokeEdgeRelative {
    #[inline(always)]
    fn extract(c: Bytecode) -> Self {
        debug_assert_eq!(c.opcode(), BytecodeOpcode::PokeEdgeRelative as u8);
        let v = c.0;
        Self {
            rs: Reg::new_masked(v >> 8),
            roff: Reg::new_masked(v >> 12),
            is_fv: ((v >> 16) & 1) != 0,
            is_posedge: ((v >> 17) & 1) != 0,
            target_size: InlineNBitSize::new_masked(v >> 18),
            source_size: InlineNBitSize::new_masked(v >> 26),
        }
    }

    #[inline(always)]
    fn encode(&self) -> Bytecode {
        Bytecode(
            (BytecodeOpcode::PokeEdgeRelative as u32)
                | ((self.rs as u32) << 8)
                | ((self.roff as u32) << 12)
                | ((self.is_fv as u32) << 16)
                | ((self.is_posedge as u32) << 17)
                | (self.target_size.encode() << 18)
                | (self.source_size.encode() << 26),
        )
    }

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            rs,
            roff,
            is_fv,
            is_posedge,
            target_size,
            source_size,
        } = self;
        let mnemonic = match (is_fv, is_posedge) {
            (false, false) => "tv.poke_negedge_rel",
            (false, true) => "tv.poke_posedge_rel",
            (true, false) => "fv.poke_negedge_rel",
            (true, true) => "fv.poke_posedge_rel",
        };
        write_padded_mnemonic(f, mnemonic)?;
        write!(f, "{rs}, {roff}, {target_size}, {source_size}")
    }

    fn num_additional_slots(&self) -> u8 {
        let mut slots = 0;
        slots += 2; // Base
        slots += 1; // Offset
        slots += 2; // Watch index
        if !self.source_size.is_inline() {
            slots += 1;
        }
        if !self.target_size.is_inline() {
            slots += 1;
        }
        slots
    }

    fn source_operands(&self, _code: &[Bytecode], _pc: u64, operands: &mut Vec<RegInfo>) {
        operands.extend([
            RegInfo::new(
                "rs",
                self.rs,
                LogicMode::TwoValue,
                None,
                RegStorage::Register,
            ),
            RegInfo::new(
                "roff",
                self.roff,
                LogicMode::TwoValue,
                Some(VSIZE_32),
                RegStorage::Register,
            ),
        ]);
    }

    fn dest_operands(&self, _code: &[Bytecode], _pc: u64, _operands: &mut Vec<RegInfo>) {}

    fn execute(
        self,
        code: &[Bytecode],
        regs: &mut Regs,
        pc: &mut u64,
        state: &mut RuntimeState,
        schedule: &mut Schedule,
        listeners: &mut BytecodeListeners,
        cldctx: &mut ColdContext,
    ) {
        let Self {
            rs,
            roff,
            is_fv,
            is_posedge,
            target_size,
            source_size,
        } = self;
        // @NOTE: We ensured the code buffer is padded with at least 16 elements.
        let additional_slots: &[Bytecode; 16] = code
            .get(*pc as usize..)
            .and_then(<[_]>::first_chunk)
            .expect("Should be padded");
        *pc += self.num_additional_slots() as u64;

        let offset = additional_slots[2].0;
        let mut size_offset = 0;
        let target_size = target_size.get(&mut size_offset, &additional_slots[5..]);
        let source_size = source_size.get(&mut size_offset, &additional_slots[5..]);

        let lower_bound = (regs[roff] & 0xFFFF_FFFF) as u32;
        let upper_bound = lower_bound + source_size.get();

        if !(lower_bound..upper_bound).contains(&offset) {
            return;
        }

        let rel_offset = offset - lower_bound;
        let heap = state.heap.0.as_mut();
        let base = decode_bytecode_u64(&additional_slots[..]);

        let is_edge = if is_fv {
            let before = {
                let spc_offset = base + offset as u64;
                let val_offset = HeapAlignment::spc_offset_to_val_offset(target_size, spc_offset);
                let spc = ((heap[(spc_offset / 64) as usize] >> (spc_offset % 64)) & 1) != 0;
                let val = ((heap[(val_offset / 64) as usize] >> (val_offset % 64)) & 1) != 0;
                FvLogicValue::from_spc_and_val(spc, val)
            };
            let after = if source_size > VSIZE_64 {
                let spc_offset = regs[rs] + rel_offset as u64;
                let val_offset = HeapAlignment::spc_offset_to_val_offset(source_size, spc_offset);
                let spc = ((heap[(spc_offset / 64) as usize] >> (spc_offset % 64)) & 1) != 0;
                let val = ((heap[(val_offset / 64) as usize] >> (val_offset % 64)) & 1) != 0;
                FvLogicValue::from_spc_and_val(spc, val)
            } else {
                let (spc, val) = rs.to_spc_and_val();
                let spc = (regs[spc] >> rel_offset) & 1 != 0;
                let val = (regs[val] >> rel_offset) & 1 != 0;
                FvLogicValue::from_spc_and_val(spc, val)
            };

            if is_posedge {
                fv_posedge(before, after)
            } else {
                fv_negedge(before, after)
            }
        } else {
            let before = base + offset as u64;
            let before = ((heap[(before / 64) as usize] >> (before % 64)) & 1) != 0;

            let after = if source_size > VSIZE_64 {
                let after = regs[rs] + rel_offset as u64;
                ((heap[(after / 64) as usize] >> (after % 64)) & 1) != 0
            } else {
                (regs[rs] >> rel_offset) & 1 != 0
            };

            if is_posedge {
                after & !before
            } else {
                before & !after
            }
        };

        if is_edge {
            let watchers = cldctx.watchers.watchers.as_slice();
            let watch_start = decode_bytecode_u64(&additional_slots[3..]);
            let watch_end = decode_bytecode_u64(&additional_slots[5..]);
            for watch_index in &watchers[watch_start as usize..watch_end as usize] {
                wake_at_index(*watch_index, schedule, listeners);
            }
        }
    }
}

impl BytecodeEncoder {
    pub fn tv_poke_negedge(
        &mut self,
        rs: Reg,
        bit_offset: u64,
        offset6: u8,
        watch_start: u64,
        watch_end: u64,
        scratch: Reg,
    ) {
        if bit_offset >= 1u64 << (14 + 32) {
            self.load_u64(scratch, 0);
            self.tv_poke_negedge_rel(
                rs,
                scratch,
                bit_offset,
                0,
                SCALAR_VSIZE,
                SCALAR_VSIZE,
                watch_start,
                watch_end,
            );
            return;
        }

        let imm14 = (bit_offset & 0x3FFF) as u16;
        let slot = (bit_offset >> 14) as u32;

        self.data
            .push(TvPokeNegedge { rs, offset6, imm14 }.encode());
        self.data.push(Bytecode(slot));
        encode_bytecode_u64(self, watch_start);
        encode_bytecode_u64(self, watch_end);
    }
    pub fn tv_poke_posedge(
        &mut self,
        rs: Reg,
        bit_offset: u64,
        offset6: u8,
        watch_start: u64,
        watch_end: u64,
        scratch: Reg,
    ) {
        if bit_offset >= 1u64 << (14 + 32) {
            self.load_u64(scratch, 0);
            self.tv_poke_posedge_rel(
                rs,
                scratch,
                bit_offset,
                0,
                SCALAR_VSIZE,
                SCALAR_VSIZE,
                watch_start,
                watch_end,
            );
            return;
        }

        let imm14 = (bit_offset & 0x3FFF) as u16;
        let slot = (bit_offset >> 14) as u32;

        self.data
            .push(TvPokePosedge { rs, offset6, imm14 }.encode());
        self.data.push(Bytecode(slot));
        encode_bytecode_u64(self, watch_start);
        encode_bytecode_u64(self, watch_end);
    }

    fn poke_edge_rel(
        &mut self,
        rs: Reg,
        roff: Reg,
        base: u64,
        offset: u32,
        target_size: VectorSize,
        source_size: VectorSize,
        watch_start: u64,
        watch_end: u64,
        is_fv: bool,
        is_posedge: bool,
    ) {
        let inline_target_size = InlineNBitSize::new(target_size);
        let inline_source_size = InlineNBitSize::new(source_size);

        self.data.push(
            PokeEdgeRelative {
                rs,
                roff,
                is_fv,
                is_posedge,
                target_size: inline_target_size,
                source_size: inline_source_size,
            }
            .encode(),
        );
        encode_bytecode_u64(self, base);
        self.data.push(Bytecode(offset));
        encode_bytecode_u64(self, watch_start);
        encode_bytecode_u64(self, watch_end);
        if !inline_target_size.is_inline() {
            self.data.push(Bytecode(target_size.get()));
        }
        if !inline_source_size.is_inline() {
            self.data.push(Bytecode(source_size.get()));
        }
    }
    pub fn fv_poke_negedge(
        &mut self,
        rs: Reg,
        bit_offset: u64,
        offset6: u8,
        watch_start: u64,
        watch_end: u64,
        scratch: Reg,
    ) {
        if bit_offset >= 1u64 << (14 + 32) {
            self.load_u64(scratch, 0);
            self.fv_poke_negedge_rel(
                rs,
                scratch,
                bit_offset,
                0,
                SCALAR_VSIZE,
                SCALAR_VSIZE,
                watch_start,
                watch_end,
            );
            return;
        }

        let imm14 = (bit_offset & 0x3FFF) as u16;
        let slot = (bit_offset >> 14) as u32;

        self.data
            .push(FvPokeNegedge { rs, offset6, imm14 }.encode());
        self.data.push(Bytecode(slot));
        encode_bytecode_u64(self, watch_start);
        encode_bytecode_u64(self, watch_end);
    }
    pub fn fv_poke_posedge(
        &mut self,
        rs: Reg,
        bit_offset: u64,
        offset6: u8,
        watch_start: u64,
        watch_end: u64,
        scratch: Reg,
    ) {
        if bit_offset >= 1u64 << (14 + 32) {
            self.load_u64(scratch, 0);
            self.fv_poke_posedge_rel(
                rs,
                scratch,
                bit_offset,
                0,
                SCALAR_VSIZE,
                SCALAR_VSIZE,
                watch_start,
                watch_end,
            );
            return;
        }

        let imm14 = (bit_offset & 0x3FFF) as u16;
        let slot = (bit_offset >> 14) as u32;

        self.data
            .push(FvPokePosedge { rs, offset6, imm14 }.encode());
        self.data.push(Bytecode(slot));
        encode_bytecode_u64(self, watch_start);
        encode_bytecode_u64(self, watch_end);
    }

    pub fn tv_poke_negedge_rel(
        &mut self,
        rs: Reg,
        roff: Reg,
        base: u64,
        offset: u32,
        target_size: VectorSize,
        source_size: VectorSize,
        watch_start: u64,
        watch_end: u64,
    ) {
        self.poke_edge_rel(
            rs,
            roff,
            base,
            offset,
            target_size,
            source_size,
            watch_start,
            watch_end,
            false,
            false,
        );
    }
    pub fn tv_poke_posedge_rel(
        &mut self,
        rs: Reg,
        roff: Reg,
        base: u64,
        offset: u32,
        target_size: VectorSize,
        source_size: VectorSize,
        watch_start: u64,
        watch_end: u64,
    ) {
        self.poke_edge_rel(
            rs,
            roff,
            base,
            offset,
            target_size,
            source_size,
            watch_start,
            watch_end,
            false,
            true,
        );
    }
    pub fn fv_poke_negedge_rel(
        &mut self,
        rs: Reg,
        roff: Reg,
        base: u64,
        offset: u32,
        target_size: VectorSize,
        source_size: VectorSize,
        watch_start: u64,
        watch_end: u64,
    ) {
        self.poke_edge_rel(
            rs,
            roff,
            base,
            offset,
            target_size,
            source_size,
            watch_start,
            watch_end,
            true,
            false,
        );
    }
    pub fn fv_poke_posedge_rel(
        &mut self,
        rs: Reg,
        roff: Reg,
        base: u64,
        offset: u32,
        target_size: VectorSize,
        source_size: VectorSize,
        watch_start: u64,
        watch_end: u64,
    ) {
        self.poke_edge_rel(
            rs,
            roff,
            base,
            offset,
            target_size,
            source_size,
            watch_start,
            watch_end,
            true,
            true,
        );
    }
}
