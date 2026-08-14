use std::mem::offset_of;

use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, InstBuilder, StackSlotData, StackSlotKind, UserFuncName};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Module};
use vogls_codegen::SixBitSize;
use vogls_ir::{BasicBlockKey, BasicBlockTerminator, Instruction, LogicMode, VariableKey};
use vogls_utils::VgHashMap;

use crate::ffi::FfiVec;
use crate::lower::{
    I64, Params, WIDE_HEAP_THRESHOLD_WORDS, WideLoc, instr_is_wide, mem, var_words,
};
use crate::runtime::{EventT, ScheduleT};

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
}

impl<'a, 'b> TrBuilder<'a, 'b> {
    pub fn new(
        ctx: &'a mut Context,
        compiler: &'a mut Compiler<'b>,
        fb: &'a mut FunctionBuilderContext,
        func_id: FuncId,
        entry_bb: BasicBlockKey,
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
            let _ = compiler.gl.bbs[k].for_each_dst_var(|v| {
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

            let b = &mut self.b;
            let params = &mut self.params;
            let blocks = &mut self.blocks;
            let vmap = &mut self.vmap;
            let spc_map = &mut self.spc_map;

            use BasicBlockTerminator as T;
            match &bb.terminator {
                T::Halt => self.compiler.tail_pop_next_or_return(b, params),
                T::Jump(t) => _ = b.ins().jump(blocks[t], &[]),
                T::Branch(cond, t, f) => {
                    let c = match cond.mode() {
                        LogicMode::TwoValue => b.use_var(vmap[cond]),
                        LogicMode::FourValue => {
                            let val = b.use_var(vmap[cond]);
                            let spc = b.use_var(spc_map[cond]);
                            b.ins().band(spc, val)
                        }
                    };

                    b.ins().brif(c, blocks[t], &[], blocks[f], &[]);
                }

                T::Wait(tr, time) => {
                    let next_tr = self.compiler.tr_funcs[tr];
                    let next_tr_ref = self.compiler.module.declare_func_in_func(next_tr, b.func);

                    if time.0 == 0 {
                        b.ins().return_call(next_tr_ref, params.as_slice());
                    } else {
                        let next_time = b.ins().iadd_imm_u(params.time, time.0 as i64);
                        let next_tr_addr = b.ins().func_addr(self.compiler.ptr, next_tr_ref);
                        let sfe = self
                            .compiler
                            .module
                            .declare_func_in_func(self.compiler.sfe, b.func);
                        b.ins()
                            .call(sfe, &[params.schedule, next_time, next_tr_addr]);

                        self.compiler.tail_pop_next_or_return(b, params);
                    }
                }
                T::VariableWait(tr, delay) => {
                    let next_tr = self.compiler.tr_funcs[tr];
                    let next_tr_ref = self.compiler.module.declare_func_in_func(next_tr, b.func);

                    let now_bb = b.create_block();
                    let later_bb = b.create_block();

                    let d = match delay.mode() {
                        LogicMode::TwoValue => b.use_var(vmap[delay]),
                        LogicMode::FourValue => {
                            // Unknown delay collapses to 0 (matches bytecode semantics).
                            let dv = b.use_var(vmap[delay]);
                            let ds = b.use_var(spc_map[delay]);
                            let known = b.ins().icmp_imm_u(IntCC::Equal, ds, -1);
                            let zero = b.ins().iconst(I64, 0);
                            b.ins().select(known, dv, zero)
                        }
                    };

                    // if delay == 0: Continue to the next TR
                    b.ins().brif(d, later_bb, &[], now_bb, &[]);
                    b.switch_to_block(now_bb);
                    b.ins().return_call(next_tr_ref, params.as_slice());

                    // if delay != 0: Push and continue to the next active event.
                    b.switch_to_block(later_bb);
                    let next_time = b.ins().iadd(params.time, d);
                    let next_tr_addr = b.ins().func_addr(self.compiler.ptr, next_tr_ref);
                    let sfe = self
                        .compiler
                        .module
                        .declare_func_in_func(self.compiler.sfe, b.func);
                    b.ins().call(sfe, &[params.schedule, next_time, next_tr_addr]);
                    self.compiler.tail_pop_next_or_return(b, params);
                }
                T::WaitRegion(tr, region) => {
                    let next_tr = self.compiler.tr_funcs[tr];
                    let next_tr_ref = self.compiler.module.declare_func_in_func(next_tr, b.func);
                    let next_tr_addr = b.ins().func_addr(self.compiler.ptr, next_tr_ref);

                    // regions_base = &schedule->regions
                    let regions_base = b.ins().load(
                        self.compiler.ptr,
                        mem(),
                        params.schedule,
                        offset_of!(ScheduleT, regions) as i32,
                    );
                    // region_vec = &schedule->regions[region]
                    let region_vec = b.ins().iadd_imm_u(
                        regions_base,
                        (*region as usize * size_of::<FfiVec<EventT>>()) as i64,
                    );

                    // Call the push function.
                    let push = self
                        .compiler
                        .module
                        .declare_func_in_func(self.compiler.push, b.func);
                    b.ins().call(push, &[region_vec, next_tr_addr]);
                    self.compiler.tail_pop_next_or_return(b, params);
                }
                T::Watch(_tr, _signals) => {
                    // Offset + listener registration were assigned by the pre-pass
                    // (collect_listeners); here we only arm the listener bit.
                    let offset = self.compiler.watch_offset[&k];
                    // Arm the listener: set the bit in `listening`.
                    let w = b
                        .ins()
                        .load(I64, mem(), params.listening, ((offset / 64) * 8) as i32);
                    let set = b.ins().bor_imm_u(w, 1i64 << (offset % 64));
                    b.ins()
                        .store(mem(), set, params.listening, ((offset / 64) * 8) as i32);
                    self.compiler.tail_pop_next_or_return(b, params);
                }
            }
        }
    }

    pub fn finalize(mut self) {
        self.b.seal_all_blocks();
        self.b.finalize(self.compiler.fe);
    }
}
