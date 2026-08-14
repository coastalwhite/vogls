use cranelift_codegen::Context;
use cranelift_codegen::ir::{Block, StackSlotData, StackSlotKind, UserFuncName};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::FuncId;
use vogls_codegen::SixBitSize;
use vogls_ir::{BasicBlockKey, Instruction, LogicMode, TemporalRegionKey, VariableKey};
use vogls_utils::VgHashMap;

use crate::lower::{I64, Params, WIDE_HEAP_THRESHOLD_WORDS, WideLoc, instr_is_wide, var_words};

use super::{Compiler, WideMap, wide_load, wide_store};

pub struct TrBuilder<'a, 'b> {
    compiler: &'a mut Compiler<'b>,
    b: FunctionBuilder<'a>,

    blocks: VgHashMap<BasicBlockKey, Block>,
    order: Vec<BasicBlockKey>,

    vmap: VgHashMap<VariableKey, Variable>,
    spc_map: VgHashMap<VariableKey, Variable>,
    wide_map: WideMap,

    params: Params,

    process_idx: usize,
    regions: &'a [TemporalRegionKey],
}

impl<'a, 'b> TrBuilder<'a, 'b> {
    pub fn new(
        ctx: &'a mut Context,
        compiler: &'a mut Compiler<'b>,
        fb: &'a mut FunctionBuilderContext,
        func_id: FuncId,
        entry_bb: BasicBlockKey,
        process_idx: usize,
        regions: &'a [TemporalRegionKey],
    ) -> Self {
        if compiler.disassembly {
            ctx.set_disasm(true);
        }

        ctx.func.signature = compiler.sigs.event.clone();
        ctx.func.name = UserFuncName::user(0, func_id.as_u32());
        let mut b = FunctionBuilder::new(&mut ctx.func, fb);

        // Discover reachable blocks in this TR.
        let mut blocks: VgHashMap<BasicBlockKey, Block> = VgHashMap::default();
        let cl_entry = b.create_block();
        b.append_block_params_for_function_params(cl_entry);
        blocks.insert(entry_bb, cl_entry);
        let mut order = vec![entry_bb];
        let mut stack = vec![entry_bb];
        while let Some(k) = stack.pop() {
            compiler.gl.bbs[k].terminator.for_each_non_temporal_bb(|s| {
                if !blocks.contains_key(&s) {
                    blocks.insert(s, b.create_block());
                    order.push(s);
                    stack.push(s);
                }
            });
        }

        // Narrow (<=64) dst vars get Cranelift Variables (value word + a
        // `spc` word for four-value). Wide (>64) dst vars get a stack slot:
        // TV = n words; FV = n spc-words then n val-words (heap layout).
        let mut vmap: VgHashMap<VariableKey, Variable> = VgHashMap::default();
        let mut spc_map: VgHashMap<VariableKey, Variable> = VgHashMap::default();
        let mut wide_map: WideMap = WideMap::default();
        let scratch_base = compiler.scratch_base;
        // TR-local cursor into the heap scratch region; reset per TR (only one
        // TR runs at a time so the region is reused).
        let mut scratch_cursor: u32 = 0;
        for &k in &order {
            let _ = compiler.gl.bbs[k].for_each_var(|v| {
                debug_assert!(!vmap.contains_key(&v));
                debug_assert!(!wide_map.contains_key(&v));

                let size = compiler.gl.vars.size(v);
                match SixBitSize::from_vector_size(size) {
                    Some(_) => {
                        vmap.insert(v, b.declare_var(I64));
                        if v.mode() == LogicMode::FourValue {
                            spc_map.insert(v, b.declare_var(I64));
                        }
                    }
                    None => {
                        let words = var_words(size, v.mode());
                        let loc = if words as usize > WIDE_HEAP_THRESHOLD_WORDS {
                            // Too large for a stack slot: place in the heap scratch
                            // region at a TR-local offset.
                            let off = scratch_base + scratch_cursor;
                            scratch_cursor += words as u32;
                            WideLoc::Heap(off)
                        } else {
                            let slot = b.create_sized_stack_slot(StackSlotData::new(
                                StackSlotKind::ExplicitSlot,
                                words as u32 * 8,
                                3,
                            ));
                            WideLoc::Slot(slot)
                        };
                        wide_map.insert(v, loc);
                    }
                }
            });
        }

        let params = Params::from_block_params(&mut b, cl_entry);

        Self {
            compiler,
            b,
            order,
            blocks,
            vmap,
            spc_map,
            wide_map,
            params,
            process_idx,
            regions,
        }
    }

    pub fn lower(&mut self, bb_phis: &VgHashMap<BasicBlockKey, Vec<(VariableKey, VariableKey)>>) {
        for &k in &self.order {
            self.b.switch_to_block(self.blocks[&k]);
            let bb = &self.compiler.gl.bbs[k];
            for instr in &bb.instrs {
                if matches!(instr, Instruction::Phi(..)) {
                    continue;
                }
                if instr_is_wide(self.compiler.gl, instr) {
                    self.compiler.lower_wide_instruction(
                        &mut self.b,
                        &self.params,
                        &self.vmap,
                        &self.spc_map,
                        &self.wide_map,
                        instr,
                    );
                } else {
                    self.compiler.lower_instruction(
                        &mut self.b,
                        &self.params,
                        &self.vmap,
                        &self.spc_map,
                        &self.wide_map,
                        instr,
                    );
                }
            }
            // Phi copies for successors, emitted at the end of this
            // (predecessor) block before the terminator.
            if let Some(phis) = bb_phis.get(&k) {
                for (dst, src) in phis {
                    let size = self.compiler.gl.vars.size(*dst);
                    match SixBitSize::from_vector_size(size) {
                        Some(_) => {
                            let sv = self.b.use_var(self.vmap[src]);
                            self.b.def_var(self.vmap[dst], sv);
                            if dst.mode() == LogicMode::FourValue {
                                let ss = self.b.use_var(self.spc_map[src]);
                                self.b.def_var(self.spc_map[dst], ss);
                            }
                        }
                        None => {
                            let dloc = self.wide_map[dst];
                            let sloc = self.wide_map[src];
                            let words = var_words(size, dst.mode());
                            for i in 0..words {
                                let w = wide_load(
                                    &mut self.b,
                                    self.compiler.ptr,
                                    self.params.heap_ptr,
                                    sloc,
                                    i as u32,
                                );
                                wide_store(
                                    &mut self.b,
                                    self.compiler.ptr,
                                    self.params.heap_ptr,
                                    dloc,
                                    i as u32,
                                    w,
                                );
                            }
                        }
                    }
                }
            }
            self.compiler.lower_terminator(
                &mut self.b,
                &self.params,
                &self.blocks,
                &self.vmap,
                &self.spc_map,
                self.process_idx,
                self.regions,
                k,
                &bb.terminator,
            );
        }
    }

    pub fn finalize(mut self) {
        self.b.seal_all_blocks();
        self.b.finalize(self.compiler.fe);
    }
}
