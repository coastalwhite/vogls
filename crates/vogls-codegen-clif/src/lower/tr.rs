use std::cmp;
use std::mem::offset_of;

use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types::{self, I32};
use cranelift_codegen::ir::{
    AbiParam, Block, BlockArg, InstBuilder, Signature, StackSlotData, StackSlotKind, Type,
    UserFuncName, Value,
};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Module};
use vogls_bits::arithmetic::FvLogicValue;
use vogls_codegen::{HeapAlignment, SixBitSize};
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, BinaryOp, Instruction, IntrinsicOp,
    LogicMode, ResizeOp, ShiftImmOp, UnaryOp, VSIZE_32, VariableKey, VectorSize,
};
use vogls_utils::VgHashMap;

use crate::ffi::FfiVec;
use crate::lower::{
    F64, I64, Params, WIDE_HEAP_THRESHOLD_WORDS, WideLoc, cast, dyn_slice_read, fv_load, mask_of,
    maskv, maskvsbs, mem, nwords, var_words, wide_addr,
};
use crate::runtime::{EventT, ScheduleT, layout};

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

                let b = &mut self.b;
                let gl = self.compiler.gl;
                let info = &self.compiler.info;
                let ptr = self.compiler.ptr;
                let params = &mut self.params;
                let vmap = &mut self.vmap;
                let spc_map = &mut self.spc_map;
                let heap = params.heap_ptr;
                let wide_map = &self.wide_map;

                let get = |b: &mut FunctionBuilder, v: VariableKey| b.use_var(vmap[&v]);
                // A two-value operand used in a four-value op is fully known (spc = mask).
                let spc_get = |b: &mut FunctionBuilder, v: VariableKey| {
                    debug_assert_eq!(v.mode(), LogicMode::FourValue);
                    b.use_var(spc_map[&v])
                };

                // Write value/special word `i` of `v` (narrow => i == 0, Cranelift
                // Variable; wide => stack/heap store). Mirror the accessors used by
                // the wide lowering so a single word-loop covers both widths.
                let wv = |b: &mut FunctionBuilder, v: VariableKey, i: u32, val: Value| {
                    let size = gl.vars.size(v).get();
                    if size > 64 {
                        let off = if v.mode() == LogicMode::FourValue {
                            nwords(size) + i
                        } else {
                            i
                        };
                        wide_store(b, ptr, heap, wide_map[&v], off, val);
                    } else {
                        b.def_var(vmap[&v], val);
                    }
                };
                let ws = |b: &mut FunctionBuilder, v: VariableKey, i: u32, val: Value| {
                    let size = gl.vars.size(v).get();
                    if size > 64 {
                        wide_store(b, ptr, heap, wide_map[&v], i, val);
                    } else {
                        b.def_var(spc_map[&v], val);
                    }
                };

                match instr {
                    // Phi nodes are realized by the per-block phi copies emitted at the
                    // end of each predecessor block, so the instruction itself is a no-op.
                    Instruction::Phi(..) => {}
                    Instruction::Constant(dst, bits) => {
                        // Normalize the constant representation such that
                        //   contains_special() => representation is fv
                        let bits = bits.clone_lowering_mode();

                        debug_assert!(!bits.contains_special() || dst.mode().is_four_value());

                        let dst_size = gl.vars.size(*dst);

                        use vogls_bits::BitsDataRef as R;
                        match bits.as_data_ref() {
                            R::InlineTv(v) => {
                                let val = b.ins().iconst(I64, v as i64);
                                b.def_var(vmap[dst], val);
                                if dst.mode().is_four_value() {
                                    let spc = b.ins().iconst(I64, mask_of(dst_size.get()));
                                    b.def_var(spc_map[dst], spc);
                                }
                            }
                            R::InlineFv(spc, val) => {
                                let val = b.ins().iconst(I64, val as i64);
                                let spc = b.ins().iconst(I64, spc as i64);
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }
                            // Four-value data with size 33..=64.
                            R::SeparateFv(s) if dst_size.get() <= 64 => {
                                let spc = b.ins().iconst(I64, s[0] as i64);
                                let val = b.ins().iconst(I64, s[1] as i64);
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }

                            // @TODO: Don't inline the value. Instead put it on the heap and
                            // memcpy it.
                            R::SeparateFv(s) => {
                                let n = nwords(dst_size.get());
                                for i in 0..n {
                                    let sc = b.ins().iconst(I64, s[i as usize] as i64);
                                    ws(b, *dst, i, sc);
                                    let vc = b.ins().iconst(I64, s[(n + i) as usize] as i64);
                                    wv(b, *dst, i, vc);
                                }
                            }
                            R::SeparateTv(w) => {
                                let n = nwords(dst_size.get());
                                for i in 0..n {
                                    let c = b.ins().iconst(I64, w[i as usize] as i64);
                                    wv(b, *dst, i, c);
                                    if dst.mode().is_four_value() {
                                        let m = if i == n - 1 {
                                            super::top_i64(dst_size.get())
                                        } else {
                                            -1
                                        };
                                        let sc = b.ins().iconst(I64, m);
                                        ws(b, *dst, i, sc);
                                    }
                                }
                            }
                        }
                    }
                    Instruction::Unary(dst, op, src) => {
                        macro_rules! map {
                            ($val:ident => @tv $blk:expr) => {{
                                let $val = get(b, *src);
                                let val = $blk;
                                b.def_var(vmap[dst], val);
                            }};
                            ($val:ident => @fv $blk:expr) => {{
                                let $val = get(b, *src);
                                let (val, spc) = $blk;
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }};
                            ($val:ident, $spc:ident => @tv $blk:expr) => {{
                                let $val = get(b, *src);
                                let $spc = spc_get(b, *src);
                                let val = $blk;
                                b.def_var(vmap[dst], val);
                            }};
                            ($val:ident, $spc:ident => @fv $blk:expr) => {{
                                let $val = get(b, *src);
                                let $spc = spc_get(b, *src);
                                let (val, spc) = $blk;
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }};
                        }

                        let dst_size = gl.vars.size(*dst);
                        let src_size = gl.vars.size(*src);

                        let native =
                            |b: &mut FunctionBuilder, x: Value| b.ins().bitcast(I64, cast(), x);

                        use crate::runtime::real_code as rc;
                        use LogicMode as M;
                        use UnaryOp as O;
                        match (
                            op,
                            dst.mode(),
                            SixBitSize::from_vector_size(dst_size),
                            SixBitSize::from_vector_size(src_size),
                        ) {
                            (O::TvToFv, _, Some(_), Some(_)) => {
                                map!(val => @fv { (val, b.ins().iconst(I64, mask_of(src_size.get()))) })
                            }
                            (O::TvToFv, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::FvToTv, _, Some(_), Some(_)) => {
                                map!(val, spc => @tv { b.ins().band(val, spc) })
                            }
                            (O::FvToTv, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Neg, M::TwoValue, Some(_), Some(_)) => map!(val => @tv {
                                let n = b.ins().bnot(val);
                                maskv(b, n, dst_size.get())
                            }),
                            (O::Neg, M::FourValue, Some(_), Some(_)) => map!(val, spc => @fv {
                                let nsv = b.ins().bnot(val);
                                let val = b.ins().band(spc, nsv);
                                (val, spc)
                            }),
                            (O::Neg, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::ReduceOr, M::TwoValue, _, Some(_)) => map!(val => @tv {
                                let c = b.ins().icmp_imm_u(IntCC::NotEqual, val, 0);
                                b.ins().uextend(I64, c)
                            }),
                            (O::ReduceOr, M::FourValue, _, Some(_)) => map!(val, spc => @fv {
                                // See fv_reduce_or_elem.
                                let sandv = b.ins().band(spc, val);
                                let z0 = b.ins().icmp_imm_u(IntCC::NotEqual, sandv, 0);
                                let allk = b.ins().icmp_imm_u(IntCC::Equal, spc, mask_of(src_size.get()));
                                let z1 = b.ins().bor(allk, z0);
                                let val = b.ins().uextend(I64, z0);
                                let spc = b.ins().uextend(I64, z1);
                                (val, spc)
                            }),
                            (O::ReduceOr, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::ReduceAnd, M::TwoValue, _, Some(_)) => map!(val => @tv {
                                let m = mask_of(src_size.get());
                                let c = b.ins().icmp_imm_u(IntCC::Equal, val, m);
                                b.ins().uextend(I64, c)
                            }),
                            (O::ReduceAnd, M::FourValue, _, Some(_)) => map!(val, spc => @fv {
                                // See fv_reduce_and_elem.
                                let m = mask_of(src_size.get());
                                let allk = b.ins().icmp_imm_u(IntCC::Equal, spc, m);
                                let nsv = b.ins().bnot(val);
                                let ssnv = b.ins().band(spc, nsv);
                                let known0 = b.ins().icmp_imm_u(IntCC::NotEqual, ssnv, 0);
                                let z1 = b.ins().bor(allk, known0);
                                let allval = b.ins().icmp_imm_u(IntCC::Equal, val, m);
                                let z0 = b.ins().band(allk, allval);
                                let val = b.ins().uextend(I64, z0);
                                let spc = b.ins().uextend(I64, z1);
                                (val, spc)
                            }),
                            (O::ReduceAnd, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::ReduceXor, M::TwoValue, _, Some(_)) => map!(val => @tv {
                                let p = b.ins().popcnt(val);
                                b.ins().band_imm_u(p, 1)
                            }),
                            (O::ReduceXor, M::FourValue, _, Some(_)) => map!(val, spc => @fv {
                                // See fv_reduce_xor_elem.
                                let allk = b.ins().icmp_imm_u(IntCC::Equal, spc, mask_of(src_size.get()));
                                let pc = b.ins().popcnt(val);
                                let par = b.ins().band_imm_u(pc, 1);
                                let parb = b.ins().icmp_imm_u(IntCC::NotEqual, par, 0);
                                let z0 = b.ins().band(allk, parb);
                                let val = b.ins().uextend(I64, z0);
                                let spc = b.ins().uextend(I64, allk);
                                (val, spc)
                            }),
                            (O::ReduceXor, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::LeadingZeros, M::TwoValue, _, Some(_)) => map!(val => @tv {
                                let shifted = if src_size.get() < 64 {
                                    b.ins().ishl_imm_u(val, (64 - src_size.get()) as i64)
                                } else {
                                    val
                                };
                                let c = b.ins().clz(shifted);
                                let is_zero = b.ins().icmp_imm_u(IntCC::Equal, val, 0);
                                let full = b.ins().iconst(I64, src_size.get() as i64);
                                let sel = b.ins().select(is_zero, full, c);
                                maskv(b, sel, dst_size.get())
                            }),
                            (O::LeadingZeros, M::FourValue, _, Some(_)) => map!(val, spc => @fv {
                                // vogls-bits::fv_leading_zeros: any x/z bit anywhere makes the whole
                                // result x; otherwise it is the two-value leading-zeros of the value
                                // plane (mirrors the two-value arm in lower_unary).
                                let known = b.ins().icmp_imm_u(IntCC::Equal, spc, mask_of(src_size.get()));
                                let shifted = if src_size.get() < 64 {
                                    b.ins().ishl_imm_u(val, (64 - src_size.get()) as i64)
                                } else {
                                    val
                                };
                                let c = b.ins().clz(shifted);
                                let is_zero = b.ins().icmp_imm_u(IntCC::Equal, val, 0);
                                let full = b.ins().iconst(I64, src_size.get() as i64);
                                let lz = b.ins().select(is_zero, full, c);
                                let lz = maskv(b, lz, dst_size.get());
                                let zero = b.ins().iconst(I64, 0);
                                let val = b.ins().select(known, lz, zero);
                                let spc_full = b.ins().iconst(I64, mask_of(dst_size.get()));
                                let spc = b.ins().select(known, spc_full, zero);
                                (val, spc)
                            }),
                            (O::LeadingZeros, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),

                            (O::RealNeg, _, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().fneg(fs); native(b, r) })
                            }
                            (O::RealSqrt, _, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().sqrt(fs); native(b, r) })
                            }
                            (O::RealFloor, _, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().floor(fs); native(b, r) })
                            }
                            (O::RealCeil, _, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().ceil(fs); native(b, r) })
                            }
                            (O::RealTruncate, _, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().trunc(fs); native(b, r) })
                            }
                            (O::RealToLogical, _, _, _) => map!(val => @tv {
                                let fs = b.ins().bitcast(F64, cast(), val);
                                let z = b.ins().f64const(0.0);
                                let c = b.ins().fcmp(FloatCC::NotEqual, fs, z);
                                b.ins().uextend(I64, c)
                            }),
                            (O::RealToU64 | O::RealToI64, _, _, _) => map!(val => @tv {
                                // Verilog real->int rounds half AWAY from zero (bytecode uses
                                // f64::round()); CLIF has no such instr, so bias by +/-0.5 then
                                // truncate toward zero. (CLIF `nearest` is round-half-to-EVEN,
                                // which would give 2.5 -> 2, so it can't be used.)
                                let fs = b.ins().bitcast(F64, cast(), val);
                                let z = b.ins().f64const(0.0);
                                let half = b.ins().f64const(0.5);
                                let neg_half = b.ins().f64const(-0.5);
                                let is_neg = b.ins().fcmp(FloatCC::LessThan, fs, z);
                                let bias = b.ins().select(is_neg, neg_half, half);
                                let biased = b.ins().fadd(fs, bias);
                                let rounded = b.ins().trunc(biased);
                                if matches!(op, O::RealToU64) {
                                    b.ins().fcvt_to_uint_sat(I64, rounded)
                                } else {
                                    b.ins().fcvt_to_sint_sat(I64, rounded)
                                }
                            }),
                            (O::RealFromSignedDecimal, _, _, _) => map!(val => @tv {
                                // Sign-extend from the operand's declared width before the
                                // signed convert; the value word is only zero-extended to i64.
                                let se = if src_size.get() >= 64 {
                                    val
                                } else {
                                    let shb = (64 - src_size.get()) as i64;
                                    let up = b.ins().ishl_imm_u(val, shb);
                                    b.ins().sshr_imm_u(up, shb)
                                };
                                let x = b.ins().fcvt_from_sint(F64, se);
                                native(b, x)
                            }),
                            (O::RealFromUnsignedDecimal, _, _, _) => map!(val => @tv {
                                let x = b.ins().fcvt_from_uint(F64, val);
                                native(b, x)
                            }),
                            (O::RealLn, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::LN, val, val))
                            }
                            (O::RealLog10, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::LOG10, val, val))
                            }
                            (O::RealExp, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::EXP, val, val))
                            }
                            (O::RealSin, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::SIN, val, val))
                            }
                            (O::RealCos, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::COS, val, val))
                            }
                            (O::RealTan, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::TAN, val, val))
                            }
                            (O::RealASin, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ASIN, val, val))
                            }
                            (O::RealACos, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ACOS, val, val))
                            }
                            (O::RealATan, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ATAN, val, val))
                            }
                            (O::RealSinH, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::SINH, val, val))
                            }
                            (O::RealCosH, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::COSH, val, val))
                            }
                            (O::RealTanH, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::TANH, val, val))
                            }
                            (O::RealASinH, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ASINH, val, val))
                            }
                            (O::RealACosH, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ACOSH, val, val))
                            }
                            (O::RealATanH, _, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ATANH, val, val))
                            }
                        }
                    }
                    Instruction::Binary(dst, op, lhs, rhs) => {
                        let dst_size = gl.vars.size(*dst);
                        let lhs_size = gl.vars.size(*lhs);
                        let rhs_size = gl.vars.size(*rhs);

                        macro_rules! map {
                            ($lval:ident, $rval:ident => @tv $blk:expr) => {{
                                let $lval = get(b, *lhs);
                                let $rval = get(b, *rhs);
                                let val = $blk;
                                b.def_var(vmap[dst], val);
                            }};
                            ($lval:ident, $rval:ident => @real $blk:expr) => {{
                                let $lval = get(b, *lhs);
                                let $rval = get(b, *rhs);
                                let $lval = b.ins().bitcast(F64, cast(), $lval);
                                let $rval = b.ins().bitcast(F64, cast(), $rval);
                                let val = $blk;
                                let val = b.ins().bitcast(I64, cast(), val);
                                b.def_var(vmap[dst], val);
                            }};
                            ($lval:ident, $rval:ident => @fv $blk:expr) => {{
                                let $lval = get(b, *lhs);
                                let $rval = get(b, *rhs);
                                let (val, spc) = $blk;
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }};
                            // Four-Value shifts.
                            ($lval:ident, $lspc:ident, $amt:ident = $amtexpr:expr => @fv_shift $blk:expr) => {{
                                let $lval = get(b, *lhs);
                                let $lspc = spc_get(b, *lhs);
                                let ($amt, amt_known) = $amtexpr;
                                let (val, spc) = $blk;
                                let zero = b.ins().iconst(I64, 0);
                                let val = b.ins().select(amt_known, val, zero);
                                let spc = b.ins().select(amt_known, spc, zero);
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }};
                            (($lval:ident, $lspc:ident), ($rval:ident, $rspc:ident) => @tv $blk:expr) => {{
                                let $lval = get(b, *lhs);
                                let $rval = get(b, *rhs);
                                let $lspc = spc_get(b, *lhs);
                                let $rspc = spc_get(b, *rhs);
                                let val = $blk;
                                b.def_var(vmap[dst], val);
                            }};
                            (($lval:ident, $lspc:ident), ($rval:ident, $rspc:ident) => @fv $blk:expr) => {{
                                let $lval = get(b, *lhs);
                                let $rval = get(b, *rhs);
                                let $lspc = spc_get(b, *lhs);
                                let $rspc = spc_get(b, *rhs);
                                let (val, spc) = $blk;
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }};
                        }
                        macro_rules! bin_arith {
                            ($lhs:ident, $rhs:ident => $blk:expr, $mask:literal) => {{
                                let $lhs = get(b, *lhs);
                                let $rhs = get(b, *rhs);

                                let mut result = $blk;
                                if $mask {
                                    result = maskv(b, result, dst_size.get());
                                }

                                match lhs.mode() {
                                    LogicMode::TwoValue => b.def_var(vmap[dst], result),
                                    LogicMode::FourValue => {
                                        let lspc = spc_get(b, *lhs);
                                        let rspc = spc_get(b, *rhs);
                                        let (val, spc) = clif_fv_arith(
                                            b,
                                            result,
                                            lspc,
                                            lhs_size.get(),
                                            rspc,
                                            rhs_size.get(),
                                            dst_size.get(),
                                            None,
                                        );
                                        b.def_var(vmap[dst], val);
                                        b.def_var(spc_map[dst], spc);
                                    }
                                }
                            }};
                        }

                        let real_cmp = |b: &mut FunctionBuilder, cc: FloatCC| {
                            let lval = get(b, *lhs);
                            let rval = get(b, *rhs);
                            let lval = b.ins().bitcast(F64, cast(), lval);
                            let rval = b.ins().bitcast(F64, cast(), rval);
                            let val = b.ins().fcmp(cc, lval, rval);
                            let val = b.ins().uextend(I64, val);
                            b.def_var(vmap[dst], val);
                        };

                        // Emulate u64::unbounded_shl/shr: a shift amount >= 64 yields 0
                        // (CLIF ishl/ushr mask the amount mod 64, so guard explicitly).
                        let ushl = |b: &mut FunctionBuilder, v: Value, amt: Value| {
                            let sh = b.ins().ishl(v, amt);
                            let zero = b.ins().iconst(I64, 0);
                            let big =
                                b.ins()
                                    .icmp_imm_u(IntCC::UnsignedGreaterThanOrEqual, amt, 64);
                            b.ins().select(big, zero, sh)
                        };
                        let ushr = |b: &mut FunctionBuilder, v: Value, amt: Value| {
                            let sh = b.ins().ushr(v, amt);
                            let zero = b.ins().iconst(I64, 0);
                            let big =
                                b.ins()
                                    .icmp_imm_u(IntCC::UnsignedGreaterThanOrEqual, amt, 64);
                            b.ins().select(big, zero, sh)
                        };
                        // 1u64.unbounded_shl(amt).wrapping_sub(1): a low mask of `amt`
                        // ones, saturating to all-ones at amt >= 64.
                        let low_mask = |b: &mut FunctionBuilder, amt: Value| {
                            let one = b.ins().iconst(I64, 1);
                            let bit = ushl(b, one, amt);
                            b.ins().iadd_imm_u(bit, -1)
                        };
                        // Arithmetic shift right of a `size`-wide value by `amt`:
                        // ((v << unused) as i64).unbounded_shr(unused + amt). Sign bit is
                        // moved to bit 63, then shifted back; the count is clamped to 63
                        // so an out-of-range amount fills entirely with the sign bit.
                        // The result is not masked (callers mask).
                        let ashr = |b: &mut FunctionBuilder, v: Value, amt: Value, size: u32| {
                            let unused = (64 - size) as i64;
                            let up = b.ins().ishl_imm_u(v, unused);
                            let total = b.ins().iadd_imm_u(amt, unused);
                            let cap = b.ins().iconst(I64, 63);
                            let camt = b.ins().umin(total, cap);
                            b.ins().sshr(up, camt)
                        };
                        // Four-value shift amount: use the value word, but the whole shift
                        // collapses to all-x if the amount isn't fully known. Returns
                        // (amount, known-flag). A two-value amount is always known.
                        let shift_amount = |b: &mut FunctionBuilder,
                                            v: VariableKey,
                                            size: u32|
                         -> (Value, Value) {
                            let amt = get(b, v);
                            match v.mode() {
                                LogicMode::TwoValue => (amt, b.ins().iconst(I64, 1)),
                                LogicMode::FourValue => {
                                    let spc = spc_get(b, v);
                                    let known =
                                        b.ins().icmp_imm_u(IntCC::Equal, spc, mask_of(size));
                                    (amt, known)
                                }
                            }
                        };

                        use crate::runtime::real_code as rc;
                        use BinaryOp as O;
                        use LogicMode as M;
                        // Wide (>64 dst/lhs/rhs) binary ops delegate to the multi-word
                        // lowering; each op has a `(.., _, _, _)` fallback below. Power
                        // and the Real* ops handle their own widths and aren't qualified.
                        match (
                            op,
                            dst.mode(),
                            lhs.mode(),
                            SixBitSize::from_vector_size(dst_size),
                            SixBitSize::from_vector_size(lhs_size),
                            SixBitSize::from_vector_size(rhs_size),
                        ) {
                            (O::And, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv b.ins().band(lhs, rhs))
                            }
                            (O::Or, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv b.ins().bor(lhs, rhs))
                            }
                            (O::Xor, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv b.ins().bxor(lhs, rhs))
                            }
                            (O::AndNot, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv {
                                    let and_not = b.ins().band_not(lhs, rhs);
                                    maskv(b, and_not, dst_size.get())
                                })
                            }
                            (O::OrNot, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv {
                                    let or_not = b.ins().bor_not(lhs, rhs);
                                    maskv(b, or_not, dst_size.get())
                                })
                            }
                            (O::Xnor, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv {
                                    let xnor = b.ins().bxor_not(lhs, rhs);
                                    maskv(b, xnor, dst_size.get())
                                })
                            }

                            (O::And, M::FourValue, _, Some(_), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    clif_fv_and(b, lval, lspc, rval, rspc)
                                })
                            }
                            (O::Or, M::FourValue, _, Some(_), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    clif_fv_or(b, lval, lspc, rval, rspc)
                                })
                            }
                            (O::Xor, M::FourValue, _, Some(_), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    clif_fv_xor(b, lval, lspc, rval, rspc)
                                })
                            }
                            (O::AndNot, M::FourValue, _, Some(size), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let (val, spc) = clif_fv_andnot(b, lval, lspc, rval, rspc);
                                    let val = maskvsbs(b, val, size);
                                    let spc = maskvsbs(b, spc, size);
                                    (val, spc)
                                })
                            }
                            (O::OrNot, M::FourValue, _, Some(size), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let (val, spc) = clif_fv_ornot(b, lval, lspc, rval, rspc);
                                    let val = maskvsbs(b, val, size);
                                    let spc = maskvsbs(b, spc, size);
                                    (val, spc)
                                })
                            }
                            (O::Xnor, M::FourValue, _, Some(size), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let (val, spc) = clif_fv_xnor(b, lval, lspc, rval, rspc);
                                    let val = maskvsbs(b, val, size);
                                    let spc = maskvsbs(b, spc, size);
                                    (val, spc)
                                })
                            }

                            (O::And, _, _, _, _, _) => wide_binary_bitwise(
                                b,
                                wide_map,
                                dst_size,
                                ptr,
                                heap,
                                *dst,
                                *lhs,
                                *rhs,
                                |b, l, r| b.ins().band(l, r),
                                clif_fv_and,
                                false,
                            ),
                            (O::Or, _, _, _, _, _) => wide_binary_bitwise(
                                b,
                                wide_map,
                                dst_size,
                                ptr,
                                heap,
                                *dst,
                                *lhs,
                                *rhs,
                                |b, l, r| b.ins().bor(l, r),
                                clif_fv_or,
                                false,
                            ),
                            (O::Xor, _, _, _, _, _) => wide_binary_bitwise(
                                b,
                                wide_map,
                                dst_size,
                                ptr,
                                heap,
                                *dst,
                                *lhs,
                                *rhs,
                                |b, l, r| b.ins().bxor(l, r),
                                clif_fv_xor,
                                false,
                            ),
                            (O::AndNot, _, _, _, _, _) => wide_binary_bitwise(
                                b,
                                wide_map,
                                dst_size,
                                ptr,
                                heap,
                                *dst,
                                *lhs,
                                *rhs,
                                |b, l, r| b.ins().band_not(l, r),
                                clif_fv_andnot,
                                false,
                            ),
                            (O::OrNot, _, _, _, _, _) => wide_binary_bitwise(
                                b,
                                wide_map,
                                dst_size,
                                ptr,
                                heap,
                                *dst,
                                *lhs,
                                *rhs,
                                |b, l, r| b.ins().bor_not(l, r),
                                clif_fv_ornot,
                                false,
                            ),
                            (O::Xnor, _, _, _, _, _) => wide_binary_bitwise(
                                b,
                                wide_map,
                                dst_size,
                                ptr,
                                heap,
                                *dst,
                                *lhs,
                                *rhs,
                                |b, l, r| b.ins().bxor_not(l, r),
                                clif_fv_xnor,
                                false,
                            ),
                            (O::CopyX, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, _rhs => @tv lhs)
                            }
                            (O::CopyX, M::FourValue, _, Some(size), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let (val, spc) = clif_fv_copyx(b, lval, lspc, rval, rspc);
                                    let val = maskvsbs(b, val, size);
                                    let spc = maskvsbs(b, spc, size);
                                    (val, spc)
                                })
                            }
                            (O::CopyX, _, _, _, _, _) => wide_binary_bitwise(
                                b,
                                wide_map,
                                dst_size,
                                ptr,
                                heap,
                                *dst,
                                *lhs,
                                *rhs,
                                |_b, l, _r| l,
                                clif_fv_copyx,
                                false,
                            ),
                            (O::CopyZ, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, _rhs => @tv lhs)
                            }
                            (O::CopyZ, M::FourValue, _, Some(size), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let (val, spc) = clif_fv_copyz(b, lval, lspc, rval, rspc);
                                    let val = maskvsbs(b, val, size);
                                    let spc = maskvsbs(b, spc, size);
                                    (val, spc)
                                })
                            }
                            (O::CopyZ, _, _, _, _, _) => wide_binary_bitwise(
                                b,
                                wide_map,
                                dst_size,
                                ptr,
                                heap,
                                *dst,
                                *lhs,
                                *rhs,
                                |_b, l, _r| l,
                                clif_fv_copyz,
                                false,
                            ),

                            (O::Add, _, _, Some(_), _, _) => {
                                bin_arith!(lhs, rhs => b.ins().iadd(lhs, rhs), true)
                            }
                            (O::Add, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Sub, _, _, Some(_), _, _) => {
                                bin_arith!(lhs, rhs => b.ins().isub(lhs, rhs), true)
                            }
                            (O::Sub, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Multiply, _, _, Some(_), _, _) => {
                                bin_arith!(lhs, rhs => b.ins().imul(lhs, rhs), true)
                            }
                            (O::Multiply, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Min, _, _, Some(_), _, _) => {
                                bin_arith!(lhs, rhs => b.ins().umin(lhs, rhs), false)
                            }
                            (O::Min, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Max, _, _, Some(_), _, _) => {
                                bin_arith!(lhs, rhs => b.ins().umax(lhs, rhs), false)
                            }
                            (O::Max, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),

                            // Negedge only ever has size-1 operands, so it is always narrow.
                            (O::Negedge, _, M::TwoValue, _, _, _) => {
                                map!(lhs, rhs => @tv {
                                    let and_not = b.ins().band_not(lhs, rhs);
                                    maskv(b, and_not, dst_size.get())
                                })
                            }
                            (O::Negedge, _, M::FourValue, _, _, _) => {
                                map!((lval, lspc), (rval, rspc) => @tv {
                                    // Negedge: (xspc & xval & (!yspc | !yval)) | (!xspc & yspc & !yval)
                                    let nxspc = b.ins().bnot(lspc);
                                    let nyspc = b.ins().bnot(rspc);
                                    let nyval = b.ins().bnot(rval);
                                    let t0 = b.ins().band(lspc, lval);
                                    let t1 = b.ins().bor(nyspc, nyval);
                                    let a = b.ins().band(t0, t1);
                                    let t2 = b.ins().band(nxspc, rspc);
                                    let c = b.ins().band(t2, nyval);
                                    b.ins().bor(a, c)
                                })
                            }
                            (O::CaseEquality, _, M::TwoValue, _, Some(_), _) => {
                                map!(lhs, rhs => @tv {
                                    let eq = b.ins().icmp(IntCC::Equal, lhs, rhs);
                                    b.ins().uextend(I64, eq)
                                })
                            }
                            (O::CaseEquality, _, M::FourValue, _, Some(_), _) => {
                                map!((lval, lspc), (rval, rspc) => @tv {
                                    let vm = b.ins().icmp(IntCC::Equal, lval, rval);
                                    let sm = b.ins().icmp(IntCC::Equal, lspc, rspc);
                                    let eq = b.ins().band(vm, sm);
                                    b.ins().uextend(I64, eq)
                                })
                            }
                            (O::CaseEquality, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Power, _, _, _, _, _) => self.compiler.emit_wide_binop(
                                b, params, *op, *dst, *lhs, *rhs, vmap, spc_map, wide_map,
                            ),
                            // DivideX/ModulusX: operands are known; div-by-zero yields x,
                            // so gate the four-value dst on a non-zero divisor.
                            (O::DivideX, _, M::TwoValue, Some(_), _, _) => {
                                map!(lhs, rhs => @fv {
                                    let (raw, nz) = clif_tv_divx(b, lhs, rhs);
                                    clif_divx_gate(b, raw, nz, dst_size.get())
                                })
                            }
                            (O::DivideX, _, M::FourValue, Some(_), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let (raw, nz) = clif_tv_divx(b, lval, rval);
                                    clif_fv_arith(b, raw, lspc, lhs_size.get(), rspc, rhs_size.get(), dst_size.get(), Some(nz))
                                })
                            }
                            (O::DivideX, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::ModulusX, _, M::TwoValue, Some(_), _, _) => {
                                map!(lhs, rhs => @fv {
                                    let (raw, nz) = clif_tv_modx(b, lhs, rhs);
                                    clif_divx_gate(b, raw, nz, dst_size.get())
                                })
                            }
                            (O::ModulusX, _, M::FourValue, Some(_), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let (raw, nz) = clif_tv_modx(b, lval, rval);
                                    clif_fv_arith(b, raw, lspc, lhs_size.get(), rspc, rhs_size.get(), dst_size.get(), Some(nz))
                                })
                            }
                            (O::ModulusX, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Divide0, _, M::TwoValue, Some(_), _, _) => {
                                map!(lhs, rhs => @tv clif_tv_div0(b, lhs, rhs))
                            }
                            (O::Divide0, _, M::FourValue, Some(_), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let raw = clif_tv_div0(b, lval, rval);
                                    clif_fv_arith(b, raw, lspc, lhs_size.get(), rspc, rhs_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::Divide0, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Modulus0, _, M::TwoValue, Some(_), _, _) => {
                                map!(lhs, rhs => @tv clif_tv_mod0(b, lhs, rhs))
                            }
                            (O::Modulus0, _, M::FourValue, Some(_), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let raw = clif_tv_mod0(b, lval, rval);
                                    clif_fv_arith(b, raw, lspc, lhs_size.get(), rspc, rhs_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::Modulus0, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::UnsignedLessEqual, M::TwoValue, _, _, Some(_), Some(_)) => {
                                map!(lhs, rhs => @tv {
                                    let c = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, lhs, rhs);
                                    b.ins().uextend(I64, c)
                                })
                            }
                            (O::UnsignedLessEqual, M::FourValue, _, _, Some(_), Some(_)) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let cmp = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, lval, rval);
                                    let cmpv = b.ins().uextend(I64, cmp);
                                    clif_fv_arith(b, cmpv, lspc, lhs_size.get(), rspc, rhs_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::UnsignedLessEqual, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::LogicalShiftLeft, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv {
                                    let sh = ushl(b, lhs, rhs);
                                    maskv(b, sh, dst_size.get())
                                })
                            }
                            (O::LogicalShiftRight, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv {
                                    // rhs is canonical, so the shifted value stays canonical.
                                    ushr(b, lhs, rhs)
                                })
                            }
                            (O::ArithmeticShiftRight, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv {
                                    let sh = ashr(b, lhs, rhs, dst_size.get());
                                    maskv(b, sh, dst_size.get())
                                })
                            }

                            (O::LogicalShiftLeft, M::FourValue, _, Some(_), _, _) => {
                                map!(lval, lspc, amt = shift_amount(b, *rhs, rhs_size.get()) => @fv_shift {
                                    // Shifted-in low bits are known zeros.
                                    let sv = ushl(b, lspc, amt);
                                    let low = low_mask(b, amt);
                                    let spc0 = b.ins().bor(sv, low);
                                    let vv = ushl(b, lval, amt);
                                    let spc = maskv(b, spc0, dst_size.get());
                                    let val = maskv(b, vv, dst_size.get());
                                    (val, spc)
                                })
                            }
                            (O::LogicalShiftRight, M::FourValue, _, Some(_), _, _) => {
                                map!(lval, lspc, amt = shift_amount(b, *rhs, rhs_size.get()) => @fv_shift {
                                    // Shifted-in high bits (the top `amt` bits of the size
                                    // window) are known zeros: low_mask(amt) << (size - amt),
                                    // with size - amt saturating to 0 so amt >= size marks
                                    // the whole window known.
                                    let sv = ushr(b, lspc, amt);
                                    let sz = b.ins().iconst(I64, dst_size.get() as i64);
                                    let diff = b.ins().isub(sz, amt);
                                    let zero = b.ins().iconst(I64, 0);
                                    let ge = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, amt, sz);
                                    let rem = b.ins().select(ge, zero, diff);
                                    let lm = low_mask(b, amt);
                                    let hi = ushl(b, lm, rem);
                                    let spc0 = b.ins().bor(sv, hi);
                                    let vv = ushr(b, lval, amt);
                                    let spc = maskv(b, spc0, dst_size.get());
                                    let val = maskv(b, vv, dst_size.get());
                                    (val, spc)
                                })
                            }
                            (O::ArithmeticShiftRight, M::FourValue, _, Some(_), _, _) => {
                                map!(lval, lspc, amt = shift_amount(b, *rhs, rhs_size.get()) => @fv_shift {
                                    let ssh = ashr(b, lspc, amt, dst_size.get());
                                    let spc = maskv(b, ssh, dst_size.get());
                                    let vsh = ashr(b, lval, amt, dst_size.get());
                                    let val = maskv(b, vsh, dst_size.get());
                                    (val, spc)
                                })
                            }
                            (O::LogicalShiftLeft, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::LogicalShiftRight, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::ArithmeticShiftRight, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),

                            (O::Concat, M::TwoValue, _, Some(_), _, _) => {
                                map!(lhs, rhs => @tv {
                                    let hi = b.ins().ishl_imm_u(lhs, rhs_size.get() as i64);
                                    b.ins().bor(hi, rhs)
                                })
                            }
                            (O::Concat, M::FourValue, _, Some(_), _, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let vh = b.ins().ishl_imm_u(lval, rhs_size.get() as i64);
                                    let val = b.ins().bor(vh, rval);
                                    let sh = b.ins().ishl_imm_u(lspc, rhs_size.get() as i64);
                                    let spc = b.ins().bor(sh, rspc);
                                    (val, spc)
                                })
                            }
                            (O::Concat, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // Real* ops are always 64-bit, so never wide.
                            (O::RealAdd, _, _, _, _, _) => {
                                map!(lhs, rhs => @real b.ins().fadd(lhs, rhs))
                            }
                            (O::RealSub, _, _, _, _, _) => {
                                map!(lhs, rhs => @real b.ins().fsub(lhs, rhs))
                            }
                            (O::RealMul, _, _, _, _, _) => {
                                map!(lhs, rhs => @real b.ins().fmul(lhs, rhs))
                            }
                            (O::RealDiv, _, _, _, _, _) => {
                                map!(lhs, rhs => @real b.ins().fdiv(lhs, rhs))
                            }
                            (O::RealPow, _, _, _, _, _) => {
                                map!(lhs, rhs => @tv self.compiler.real_shim(b, params, rc::POW, lhs, rhs))
                            }
                            (O::RealEq, _, _, _, _, _) => real_cmp(b, FloatCC::Equal),
                            (O::RealNe, _, _, _, _, _) => real_cmp(b, FloatCC::NotEqual),
                            (O::RealLt, _, _, _, _, _) => real_cmp(b, FloatCC::LessThan),
                            (O::RealLeq, _, _, _, _, _) => real_cmp(b, FloatCC::LessThanOrEqual),
                            (O::RealGt, _, _, _, _, _) => real_cmp(b, FloatCC::GreaterThan),
                            (O::RealGeq, _, _, _, _, _) => real_cmp(b, FloatCC::GreaterThanOrEqual),
                            (O::RealATan2, _, _, _, _, _) => {
                                map!(lhs, rhs => @tv self.compiler.real_shim(b, params, rc::ATAN2, lhs, rhs))
                            }
                            (O::RealHypot, _, _, _, _, _) => {
                                map!(lhs, rhs => @tv self.compiler.real_shim(b, params, rc::HYPOT, lhs, rhs))
                            }
                        }
                    }
                    Instruction::BinaryImm(dst, op, src, imm) => {
                        let dst_size = gl.vars.size(*dst);
                        let src_size = gl.vars.size(*src);
                        let imm_size = imm.size();

                        // Materialize the immediate's value and special-plane words as
                        // constants. A two-value immediate is fully known (spc = mask).
                        let (iv, is) = match imm.as_data_ref() {
                            vogls_bits::BitsDataRef::InlineTv(v) => {
                                (v as i64, mask_of(imm_size.get()))
                            }
                            vogls_bits::BitsDataRef::InlineFv(spc, val) => (val as i64, spc as i64),
                            vogls_bits::BitsDataRef::SeparateFv(s) => (s[1] as i64, s[0] as i64),
                            vogls_bits::BitsDataRef::SeparateTv(s) => {
                                (s[0] as i64, mask_of(imm_size.get()))
                            }
                        };

                        // Like `map!` for Binary, but the rhs operand is the immediate:
                        // `$sval`/`$sspc` read the src register, `$ival`/`$ispc` are the
                        // immediate constants.
                        macro_rules! imap {
                            ($sval:ident, $ival:ident => @tv $blk:expr) => {{
                                let $sval = get(b, *src);
                                let $ival = b.ins().iconst(I64, iv);
                                let val = $blk;
                                b.def_var(vmap[dst], val);
                            }};
                            (($sval:ident, $sspc:ident), ($ival:ident, $ispc:ident) => @fv $blk:expr) => {{
                                let $sval = get(b, *src);
                                let $sspc = spc_get(b, *src);
                                let $ival = b.ins().iconst(I64, iv);
                                let $ispc = b.ins().iconst(I64, is);
                                let (val, spc) = $blk;
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }};
                            // Plain two-value operand + immediate, four-value dst (for the
                            // x-injecting variants with two-value operands).
                            ($sval:ident, $ival:ident => @fv $blk:expr) => {{
                                let $sval = get(b, *src);
                                let $ival = b.ins().iconst(I64, iv);
                                let (val, spc) = $blk;
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }};
                            // Four-value operand + immediate, two-value dst (for the
                            // case-equality ops, which reduce both planes to a known bit).
                            (($sval:ident, $sspc:ident), ($ival:ident, $ispc:ident) => @tv $blk:expr) => {{
                                let $sval = get(b, *src);
                                let $sspc = spc_get(b, *src);
                                let $ival = b.ins().iconst(I64, iv);
                                let $ispc = b.ins().iconst(I64, is);
                                let val = $blk;
                                b.def_var(vmap[dst], val);
                            }};
                        }

                        use BinaryImmOp as O;
                        use LogicMode as M;
                        // Wide (>64 dst/src/imm) ops delegate to the multi-word lowering;
                        // each op has a `(.., _, _, _)` fallback below. Power/RevPower run
                        // their own shim and aren't qualified.
                        match (
                            op,
                            dst.mode(),
                            src.mode(),
                            SixBitSize::from_vector_size(dst_size),
                            SixBitSize::from_vector_size(src_size),
                            SixBitSize::from_vector_size(imm_size),
                        ) {
                            (O::And, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv b.ins().band(sval, ival))
                            }
                            (O::And, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    clif_fv_and(b, sval, sspc, ival, ispc)
                                })
                            }
                            (O::And, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Or, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv b.ins().bor(sval, ival))
                            }
                            (O::Or, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    clif_fv_or(b, sval, sspc, ival, ispc)
                                })
                            }
                            (O::Or, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Xor, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv b.ins().bxor(sval, ival))
                            }
                            (O::Xor, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    clif_fv_xor(b, sval, sspc, ival, ispc)
                                })
                            }
                            (O::Xor, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),

                            (O::Add, M::TwoValue, _, Some(_), _, _) => imap!(sval, ival => @tv {
                                let r = b.ins().iadd(sval, ival);
                                maskv(b, r, dst_size.get())
                            }),
                            (O::Add, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = b.ins().iadd(sval, ival);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::Add, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // Sub: Operand - Imm.
                            (O::Sub, M::TwoValue, _, Some(_), _, _) => imap!(sval, ival => @tv {
                                let r = b.ins().isub(sval, ival);
                                maskv(b, r, dst_size.get())
                            }),
                            (O::Sub, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = b.ins().isub(sval, ival);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::Sub, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // RevSub: Imm - Operand.
                            (O::RevSub, M::TwoValue, _, Some(_), _, _) => imap!(sval, ival => @tv {
                                let r = b.ins().isub(ival, sval);
                                maskv(b, r, dst_size.get())
                            }),
                            (O::RevSub, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = b.ins().isub(ival, sval);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::RevSub, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Multiply, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv {
                                    let r = b.ins().imul(sval, ival);
                                    maskv(b, r, dst_size.get())
                                })
                            }
                            (O::Multiply, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = b.ins().imul(sval, ival);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::Multiply, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // Divide/Modulus: Operand / Imm, div-by-zero yields 0.
                            (O::Divide, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv clif_tv_div0(b, sval, ival))
                            }
                            (O::Divide, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = clif_tv_div0(b, sval, ival);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::Divide, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Modulus, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv clif_tv_mod0(b, sval, ival))
                            }
                            (O::Modulus, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = clif_tv_mod0(b, sval, ival);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::Modulus, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // RevDivide0/RevModulus0: Imm / Operand, div-by-zero yields 0.
                            (O::RevDivide0, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv clif_tv_div0(b, ival, sval))
                            }
                            (O::RevDivide0, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = clif_tv_div0(b, ival, sval);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::RevDivide0, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::RevModulus0, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv clif_tv_mod0(b, ival, sval))
                            }
                            (O::RevModulus0, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = clif_tv_mod0(b, ival, sval);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::RevModulus0, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // RevDivideX/RevModulusX: Imm / Operand, div-by-zero yields x.
                            // dst is always four-value; gate on a non-zero divisor (the
                            // operand). Two-value operands are fully known.
                            (O::RevDivideX, _, M::TwoValue, Some(_), _, _) => {
                                imap!(sval, ival => @fv {
                                    let (raw, nz) = clif_tv_divx(b, ival, sval);
                                    clif_divx_gate(b, raw, nz, dst_size.get())
                                })
                            }
                            (O::RevDivideX, _, M::FourValue, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let (raw, nz) = clif_tv_divx(b, ival, sval);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), Some(nz))
                                })
                            }
                            (O::RevDivideX, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::RevModulusX, _, M::TwoValue, Some(_), _, _) => {
                                imap!(sval, ival => @fv {
                                    let (raw, nz) = clif_tv_modx(b, ival, sval);
                                    clif_divx_gate(b, raw, nz, dst_size.get())
                                })
                            }
                            (O::RevModulusX, _, M::FourValue, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let (raw, nz) = clif_tv_modx(b, ival, sval);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), Some(nz))
                                })
                            }
                            (O::RevModulusX, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // UnsignedLessEqual: Operand <= Imm (dst is one bit).
                            (O::UnsignedLessEqual, M::TwoValue, _, _, Some(_), _) => {
                                imap!(sval, ival => @tv {
                                    let c = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, sval, ival);
                                    b.ins().uextend(I64, c)
                                })
                            }
                            (O::UnsignedLessEqual, M::FourValue, _, _, Some(_), _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let c = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, sval, ival);
                                    let cmpv = b.ins().uextend(I64, c);
                                    clif_fv_arith(b, cmpv, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::UnsignedLessEqual, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // UnsignedGreaterEqual: Imm <= Operand, i.e. Operand >= Imm.
                            (O::UnsignedGreaterEqual, M::TwoValue, _, _, Some(_), _) => {
                                imap!(sval, ival => @tv {
                                    let c = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, sval, ival);
                                    b.ins().uextend(I64, c)
                                })
                            }
                            (O::UnsignedGreaterEqual, M::FourValue, _, _, Some(_), _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let c = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, sval, ival);
                                    let cmpv = b.ins().uextend(I64, c);
                                    clif_fv_arith(b, cmpv, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::UnsignedGreaterEqual, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Min, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv b.ins().umin(sval, ival))
                            }
                            (O::Min, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = b.ins().umin(sval, ival);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::Min, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (O::Max, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv b.ins().umax(sval, ival))
                            }
                            (O::Max, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let raw = b.ins().umax(sval, ival);
                                    clif_fv_arith(b, raw, sspc, src_size.get(), ispc, imm_size.get(), dst_size.get(), None)
                                })
                            }
                            (O::Max, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),

                            // ConcatRight: { Operand, Imm } — operand high, imm low.
                            (O::ConcatRight, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv {
                                    let hi = b.ins().ishl_imm_u(sval, imm_size.get() as i64);
                                    b.ins().bor(hi, ival)
                                })
                            }
                            (O::ConcatRight, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let vh = b.ins().ishl_imm_u(sval, imm_size.get() as i64);
                                    let val = b.ins().bor(vh, ival);
                                    let sh = b.ins().ishl_imm_u(sspc, imm_size.get() as i64);
                                    let spc = b.ins().bor(sh, ispc);
                                    (val, spc)
                                })
                            }
                            (O::ConcatRight, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // ConcatLeft: { Imm, Operand } — imm high, operand low.
                            (O::ConcatLeft, M::TwoValue, _, Some(_), _, _) => {
                                imap!(sval, ival => @tv {
                                    let hi = b.ins().ishl_imm_u(ival, src_size.get() as i64);
                                    b.ins().bor(hi, sval)
                                })
                            }
                            (O::ConcatLeft, M::FourValue, _, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @fv {
                                    let vh = b.ins().ishl_imm_u(ival, src_size.get() as i64);
                                    let val = b.ins().bor(vh, sval);
                                    let sh = b.ins().ishl_imm_u(ispc, src_size.get() as i64);
                                    let spc = b.ins().bor(sh, sspc);
                                    (val, spc)
                                })
                            }
                            (O::ConcatLeft, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),

                            // CaseEquality (===): whole-word compare of both planes; the
                            // result is a fully-known single bit (dst is one bit).
                            (O::CaseEquality, _, M::TwoValue, _, Some(_), _) => {
                                imap!(sval, ival => @tv {
                                    let eq = b.ins().icmp(IntCC::Equal, sval, ival);
                                    b.ins().uextend(I64, eq)
                                })
                            }
                            (O::CaseEquality, _, M::FourValue, _, Some(_), _) => {
                                imap!((sval, sspc), (ival, ispc) => @tv {
                                    let vm = b.ins().icmp(IntCC::Equal, sval, ival);
                                    let sm = b.ins().icmp(IntCC::Equal, sspc, ispc);
                                    let eq = b.ins().band(vm, sm);
                                    b.ins().uextend(I64, eq)
                                })
                            }
                            (O::CaseEquality, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            // BitwiseCaseEquality (per-bit ===): bit = 1 iff operand and
                            // immediate agree in BOTH planes. Same-width, fully-known,
                            // two-value result.
                            (O::BitwiseCaseEquality, _, M::TwoValue, Some(dst_size), _, _) => {
                                imap!(sval, ival => @tv {
                                    let vx = b.ins().bxor(sval, ival);
                                    let veq = b.ins().bnot(vx);
                                    maskvsbs(b, veq, dst_size)
                                })
                            }
                            (O::BitwiseCaseEquality, _, M::FourValue, Some(_), _, _) => {
                                imap!((sval, sspc), (ival, ispc) => @tv {
                                    let vx = b.ins().bxor(sval, ival);
                                    let veq = b.ins().bnot(vx);
                                    let sx = b.ins().bxor(sspc, ispc);
                                    let seq = b.ins().bnot(sx);
                                    let both = b.ins().band(veq, seq);
                                    maskv(b, both, dst_size.get())
                                })
                            }
                            (O::BitwiseCaseEquality, _, _, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),

                            // Power/RevPower have no clean inline form: run the vogls-bits
                            // shim (handles narrow and wide).
                            (O::Power | O::RevPower, _, _, _, _, _) => {
                                self.compiler.emit_wide_binop_imm(
                                    b, params, gl, *op, *dst, *src, imm, vmap, spc_map, wide_map,
                                )
                            }
                        }
                    }
                    Instruction::Resize(dst, op, src) => {
                        macro_rules! map {
                            ($val:ident => @tv $blk:expr) => {{
                                let $val = get(b, *src);
                                let val = $blk;
                                b.def_var(vmap[dst], val);
                            }};
                            ($val:ident, $spc:ident => @fv $blk:expr) => {{
                                let $val = get(b, *src);
                                let $spc = spc_get(b, *src);
                                let (val, spc) = $blk;
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }};
                        }

                        let dst_size = gl.vars.size(*dst);
                        let src_size = gl.vars.size(*src);

                        use LogicMode as M;
                        use ResizeOp as O;
                        match (
                            op,
                            dst.mode(),
                            SixBitSize::from_vector_size(dst_size),
                            SixBitSize::from_vector_size(src_size),
                        ) {
                            (O::Truncate, M::TwoValue, Some(dst_size), Some(_)) => {
                                map!(val => @tv maskvsbs(b, val, dst_size))
                            }
                            (O::Truncate, M::FourValue, Some(dst_size), Some(_)) => {
                                map!(val, spc => @fv {
                                    (maskvsbs(b, val, dst_size), maskvsbs(b, spc, dst_size))
                                })
                            }
                            (O::Truncate, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),

                            (O::ZeroExtend, LogicMode::TwoValue, Some(_), Some(_)) => {
                                map!(val => @tv val)
                            }
                            (O::ZeroExtend, M::FourValue, Some(dst_size), Some(src_size)) => {
                                map!(val, spc => @fv {
                                    let extension_mask = dst_size.mask(u64::MAX) & !src_size.mask(u64::MAX);
                                    let spc = b.ins().bor_imm_u(spc, extension_mask as i64);
                                    (val, spc)
                                })
                            }
                            (O::ZeroExtend, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),

                            (O::SignExtend, M::TwoValue, Some(dst_size), Some(src_size)) => {
                                map!(val => @tv sign_extend(b, val, dst_size, src_size))
                            }
                            (O::SignExtend, M::FourValue, Some(dst_size), Some(src_size)) => {
                                map!(val, spc => @fv {
                                    let val = sign_extend(b, val, dst_size, src_size);
                                    let spc = sign_extend(b, spc, dst_size, src_size);
                                    (val, spc)
                                })
                            }
                            (O::SignExtend, _, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                        }
                    }
                    Instruction::ShiftImm(dst, op, src, amount) => {
                        macro_rules! map {
                            ($val:ident => @tv $blk:expr) => {{
                                let $val = get(b, *src);
                                let val = $blk;
                                b.def_var(vmap[dst], val);
                            }};
                            ($val:ident, $spc:ident => @fv $blk:expr) => {{
                                let $val = get(b, *src);
                                let $spc = spc_get(b, *src);
                                let (val, spc) = $blk;
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }};
                        }

                        // Shifts preserve width, so dst_size == src_size. The amount is a
                        // compile-time constant: all width comparisons fold away.
                        let size = gl.vars.size(*dst).get();
                        let amount = *amount;

                        use LogicMode as M;
                        use ShiftImmOp as S;
                        // Wide (>64) shifts delegate to the multi-word lowering; each op
                        // has a `(.., _)` fallback below.
                        match (
                            op,
                            dst.mode(),
                            SixBitSize::from_vector_size(gl.vars.size(*dst)),
                        ) {
                            (S::LogicalShiftLeft, M::TwoValue, Some(_)) => map!(sv => @tv {
                                if amount >= 64 {
                                    b.ins().iconst(I64, 0)
                                } else {
                                    let sh = b.ins().ishl_imm_u(sv, amount as i64);
                                    maskv(b, sh, size)
                                }
                            }),
                            (S::LogicalShiftLeft, M::FourValue, Some(_)) => map!(sv, ss => @fv {
                                if amount >= size {
                                    // Everything shifted out: all known zeros.
                                    let z = b.ins().iconst(I64, 0);
                                    let m = b.ins().iconst(I64, mask_of(size));
                                    (z, m)
                                } else {
                                    let shv = b.ins().ishl_imm_u(sv, amount as i64);
                                    let val = maskv(b, shv, size);
                                    // Shifted-in low bits are known zeros.
                                    let shs = b.ins().ishl_imm_u(ss, amount as i64);
                                    let low = ((1u64 << amount) - 1) as i64;
                                    let s0 = b.ins().bor_imm_u(shs, low);
                                    let spc = maskv(b, s0, size);
                                    (val, spc)
                                }
                            }),
                            (S::LogicalShiftLeft, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (S::LogicalShiftRight, M::TwoValue, Some(_)) => map!(sv => @tv {
                                if amount >= 64 {
                                    b.ins().iconst(I64, 0)
                                } else {
                                    b.ins().ushr_imm_u(sv, amount as i64)
                                }
                            }),
                            (S::LogicalShiftRight, M::FourValue, Some(_)) => map!(sv, ss => @fv {
                                if amount >= size {
                                    let z = b.ins().iconst(I64, 0);
                                    let m = b.ins().iconst(I64, mask_of(size));
                                    (z, m)
                                } else {
                                    let val = b.ins().ushr_imm_u(sv, amount as i64);
                                    // Shifted-in high bits are known zeros.
                                    let shs = b.ins().ushr_imm_u(ss, amount as i64);
                                    let high = (((1u64 << amount) - 1) << (size - amount)) as i64;
                                    let s0 = b.ins().bor_imm_u(shs, high);
                                    let spc = maskv(b, s0, size);
                                    (val, spc)
                                }
                            }),
                            (S::LogicalShiftRight, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                            (S::ArithmeticShiftRight, M::TwoValue, Some(_)) => map!(sv => @tv {
                                let sh = sign_shift_right(b, sv, amount, size);
                                maskv(b, sh, size)
                            }),
                            (S::ArithmeticShiftRight, M::FourValue, Some(_)) => {
                                map!(sv, ss => @fv {
                                    // Arithmetic shift both planes (the special plane's sign
                                    // bit replicates, matching fv_shift_arith_right).
                                    let vsh = sign_shift_right(b, sv, amount, size);
                                    let val = maskv(b, vsh, size);
                                    let ssh = sign_shift_right(b, ss, amount, size);
                                    let spc = maskv(b, ssh, size);
                                    (val, spc)
                                })
                            }
                            (S::ArithmeticShiftRight, _, _) => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                        }
                    }
                    Instruction::Select(dst, cond, truthy, falsy) => {
                        // Condition is truthy i.f.f. it is `1`.
                        let cond = match cond.mode() {
                            LogicMode::TwoValue => get(b, *cond),
                            LogicMode::FourValue => {
                                let cv = get(b, *cond);
                                let cs = spc_get(b, *cond);
                                b.ins().band(cs, cv)
                            }
                        };

                        let size = gl.vars.size(*dst);
                        match SixBitSize::from_vector_size(size) {
                            Some(_) => {
                                let truthy_val = get(b, *truthy);
                                let falsy_val = get(b, *falsy);
                                let val = b.ins().select(cond, truthy_val, falsy_val);
                                b.def_var(vmap[dst], val);
                                if dst.mode().is_four_value() {
                                    let truthy_spc = spc_get(b, *truthy);
                                    let falsy_spc = spc_get(b, *falsy);
                                    let spc = b.ins().select(cond, truthy_spc, falsy_spc);
                                    b.def_var(spc_map[dst], spc);
                                }
                            }
                            None => {
                                let truthy_ptr = wide_addr(b, ptr, heap, wide_map[truthy], 0);
                                let falsy_ptr = wide_addr(b, ptr, heap, wide_map[falsy], 0);
                                let src_ptr = b.ins().select(cond, truthy_ptr, falsy_ptr);
                                let dst_ptr = wide_addr(b, ptr, heap, wide_map[dst], 0);
                                let words = var_words(size, dst.mode()) as u32;
                                wide_copy(b, dst_ptr, src_ptr, words);
                            }
                        }
                    }
                    // Constant-offset slice: extract dst-width bits of src at `offset`.
                    Instruction::SliceImm(dst, src, offset) => {
                        let dst_size = gl.vars.size(*dst);
                        let src_size = gl.vars.size(*src);

                        // Over-sliced. All destination bits are zero.
                        let Some(rem_src_bits) =
                            VectorSize::new(src_size.get().saturating_sub(*offset))
                        else {
                            match SixBitSize::from_vector_size(dst_size) {
                                Some(_) => {
                                    let zero = b.ins().iconst(I64, 0);
                                    b.def_var(vmap[dst], zero);
                                    if dst.mode().is_four_value() {
                                        let all_one = b.ins().iconst(I64, mask_of(dst_size.get()));
                                        b.def_var(spc_map[dst], all_one);
                                    }
                                }
                                None => wide_fill(
                                    b,
                                    ptr,
                                    heap,
                                    wide_map[dst],
                                    dst_size.get(),
                                    dst.mode(),
                                    vogls_bits::arithmetic::FvLogicValue::L0,
                                ),
                            }
                            continue;
                        };

                        let (Some(dst_size), Some(src_size)) = (
                            SixBitSize::from_vector_size(dst_size),
                            SixBitSize::from_vector_size(src_size),
                        ) else {
                            self.compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr);
                            continue;
                        };

                        // Truncation / Zero-extend.
                        let val = get(b, *src);
                        if *offset == 0 {
                            let mut val = val;
                            if dst_size < src_size {
                                val = maskvsbs(b, val, dst_size);
                            }

                            b.def_var(vmap[dst], val);
                            if dst.mode().is_four_value() {
                                let spc = spc_get(b, *src);
                                let spc = match dst_size.cmp(&src_size) {
                                    // Truncate.
                                    cmp::Ordering::Less => maskvsbs(b, spc, dst_size),
                                    // Zero Extend.
                                    cmp::Ordering::Greater => {
                                        let fill =
                                            dst_size.mask(u64::MAX) & !src_size.mask(u64::MAX);
                                        b.ins().bor_imm_u(spc, fill as i64)
                                    }

                                    cmp::Ordering::Equal => spc,
                                };

                                b.def_var(spc_map[dst], spc);
                            }
                            continue;
                        }

                        let mut shifted_val = b.ins().ushr_imm_u(val, *offset as i64);
                        if rem_src_bits > VectorSize::from(dst_size) {
                            shifted_val = maskvsbs(b, shifted_val, dst_size);
                        }
                        b.def_var(vmap[dst], shifted_val);
                        if dst.mode().is_four_value() {
                            let spc = spc_get(b, *src);
                            let shifted_spc = b.ins().ushr_imm_u(spc, *offset as i64);
                            let shifted_spc = match dst_size.to_vector_size().cmp(&rem_src_bits) {
                                // Truncate to dst size.
                                cmp::Ordering::Less => maskvsbs(b, shifted_spc, dst_size),
                                // Extend with zeros to dst size.
                                cmp::Ordering::Greater => {
                                    let fill = dst_size.mask(u64::MAX) as i64
                                        & !mask_of(rem_src_bits.get());
                                    b.ins().bor_imm_u(shifted_spc, fill)
                                }
                                cmp::Ordering::Equal => shifted_spc,
                            };
                            b.def_var(spc_map[dst], shifted_spc);
                        }
                    }
                    Instruction::Slice(dst, src, offset) => {
                        let dst_size = gl.vars.size(*dst);
                        let src_size = gl.vars.size(*src);

                        let offset_val = get(b, *offset);

                        // Guard (predictable branch) and fill destination with `x` if
                        // - `offset >= src_size` (which guards against shift overflow)
                        // - `offset` contains `x` or `z`.
                        let all_x = b.ins().icmp_imm_u(
                            IntCC::UnsignedGreaterThanOrEqual,
                            offset_val,
                            src_size.get() as i64,
                        );
                        let all_x = match offset.mode() {
                            LogicMode::TwoValue => all_x,
                            LogicMode::FourValue => {
                                let offset_spc = spc_get(b, *offset);
                                let special = b.ins().icmp_imm_u(
                                    IntCC::NotEqual,
                                    offset_spc,
                                    mask_of(VSIZE_32.get()),
                                );
                                b.ins().bor(all_x, special)
                            }
                        };

                        let all_x_bb = b.create_block();
                        let slice_guarded_bb = b.create_block();
                        let done_bb = b.create_block();
                        b.ins().brif(all_x, all_x_bb, &[], slice_guarded_bb, &[]);

                        b.switch_to_block(slice_guarded_bb);
                        match (
                            SixBitSize::from_vector_size(dst_size),
                            SixBitSize::from_vector_size(src_size),
                        ) {
                            (Some(_), Some(_)) => {
                                // Value plane.
                                let src_val = get(b, *src);
                                let mut dst_val = b.ins().ushr(src_val, offset_val);
                                if dst_size < src_size {
                                    dst_val = maskv(b, dst_val, dst_size.get());
                                }
                                b.def_var(vmap[dst], dst_val);
                                // Special plane.
                                let src_spc = match src.mode() {
                                    LogicMode::TwoValue => {
                                        b.ins().iconst(I64, mask_of(src_size.get()))
                                    }
                                    LogicMode::FourValue => spc_get(b, *src),
                                };
                                let mut dst_spc = b.ins().ushr(src_spc, offset_val);
                                if dst_size < src_size {
                                    dst_spc = maskv(b, dst_spc, dst_size.get());
                                }
                                b.def_var(spc_map[dst], dst_spc);
                            }
                            _ => self
                                .compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr),
                        }
                        b.ins().jump(done_bb, &[]);

                        // Guard condition. Output is all `x`.
                        b.switch_to_block(all_x_bb);
                        match SixBitSize::from_vector_size(dst_size) {
                            Some(_) => {
                                let zero = b.ins().iconst(I64, 0);
                                b.def_var(vmap[dst], zero);
                                b.def_var(spc_map[dst], zero);
                            }
                            None => wide_fill(
                                b,
                                ptr,
                                heap,
                                wide_map[dst],
                                dst_size.get(),
                                dst.mode(),
                                FvLogicValue::X,
                            ),
                        }
                        b.ins().jump(done_bb, &[]);

                        b.switch_to_block(done_bb);
                    }
                    Instruction::LastUpdateTime(dst, signal) => {
                        let rt = info.rt_signal_map[signal];
                        let &idx = self
                            .compiler
                            .info
                            .lupdt_indexes
                            .get(&rt)
                            .expect("Should have been populated in a prepass.");
                        let lat = params.last_active_time;
                        let t = b.ins().load(I64, mem(), lat, (idx * 8) as i32);
                        b.def_var(vmap[dst], t);
                    }
                    Instruction::Intrinsic(dst, op, items) => {
                        let dst_size = gl.vars.size(*dst);
                        match op.as_ref() {
                            IntrinsicOp::Time => {
                                let t = params.time;
                                b.def_var(vmap[dst], t);
                            }
                            IntrinsicOp::Finish => {
                                let one = b.ins().iconst(I32, 1);
                                b.ins().return_(&[one]);
                                // Subsequent (dead) instructions land in a fresh block.
                                let dead = b.create_block();
                                b.switch_to_block(dead);
                            }
                            IntrinsicOp::Display(fmt) => {
                                let idx = self.compiler.intern_fmt(fmt);
                                self.compiler
                                    .emit_fmt_call(b, params, vmap, spc_map, wide_map, items, idx);
                                let z = b.ins().iconst(I64, 0);
                                b.def_var(vmap[dst], z);
                            }
                            IntrinsicOp::Assert(fmt) => {
                                // Assert condition truthiness: two-value nonzero, or the
                                // known-1 pattern (spc & val) for four-value; a wide operand
                                // is or-reduced word by word.
                                let cond = if gl.vars.size(items[0]).get() > 64 {
                                    let n = nwords(gl.vars.size(items[0]).get());
                                    let fv = items[0].mode() == LogicMode::FourValue;
                                    let mut acc = b.ins().iconst(I64, 0);
                                    for i in 0..n {
                                        let voff = if fv { n + i } else { i };
                                        let vw = wide_load(
                                            b,
                                            ptr,
                                            params.heap_ptr,
                                            wide_map[&items[0]],
                                            voff,
                                        );
                                        let known = if fv {
                                            let sw = wide_load(
                                                b,
                                                ptr,
                                                params.heap_ptr,
                                                wide_map[&items[0]],
                                                i,
                                            );
                                            b.ins().band(sw, vw)
                                        } else {
                                            vw
                                        };
                                        acc = b.ins().bor(acc, known);
                                    }
                                    acc
                                } else if items[0].mode() == LogicMode::FourValue {
                                    let cv = b.use_var(vmap[&items[0]]);
                                    let cs = b.use_var(spc_map[&items[0]]);
                                    b.ins().band(cs, cv)
                                } else {
                                    b.use_var(vmap[&items[0]])
                                };
                                let fail = b.create_block();
                                let cont = b.create_block();
                                b.ins().brif(cond, cont, &[], fail, &[]);
                                b.switch_to_block(fail);
                                let idx = self.compiler.intern_fmt(fmt);
                                self.compiler.emit_fmt_call(
                                    b,
                                    params,
                                    vmap,
                                    spc_map,
                                    wide_map,
                                    &items[1..],
                                    idx,
                                );
                                let two = b.ins().iconst(I32, 2);
                                b.ins().return_(&[two]);
                                b.switch_to_block(cont);
                                let z = b.ins().iconst(I64, 0);
                                b.def_var(vmap[dst], z);
                            }
                            IntrinsicOp::Random(kind) => {
                                use vogls_ir::RandomKind as RK;
                                let off = match kind {
                                    RK::Uniform => layout::FN_RTL_UNIFORM,
                                    RK::Normal => layout::FN_RTL_NORMAL,
                                    RK::Exponential => layout::FN_RTL_EXPONENTIAL,
                                    RK::Poisson => layout::FN_RTL_POISSON,
                                    RK::ChiSquare => layout::FN_RTL_CHI_SQUARE,
                                    RK::T => layout::FN_RTL_T,
                                    RK::Erlang => layout::FN_RTL_ERLANG,
                                };
                                let cldctx = params.cldctx;
                                let fn_ptr = b.ins().load(
                                    ptr,
                                    mem(),
                                    cldctx,
                                    (layout::CTX_FN_TABLE + off) as i32,
                                );
                                let stderr =
                                    b.ins().load(ptr, mem(), cldctx, layout::CTX_STDERR as i32);
                                // seed + distribution params, each passed as i32, then the io ptr.
                                let mut args = Vec::new();
                                let mut sig = Signature::new(CallConv::SystemV);
                                for item in items.iter() {
                                    let v = get(b, *item);
                                    let v32 = b.ins().ireduce(I32, v);
                                    args.push(v32);
                                    sig.params.push(AbiParam::new(I32));
                                }
                                args.push(stderr);
                                sig.params.push(AbiParam::new(ptr));
                                sig.returns.push(AbiParam::new(I64));
                                let sr = b.import_signature(sig);
                                let call = b.ins().call_indirect(sr, fn_ptr, &args);
                                let r = b.inst_results(call)[0];
                                b.def_var(vmap[dst], r);
                                // The shim returns a fully-known packed result; for a
                                // four-value dst mark the whole value known (else spc stays
                                // 0 => the draw reads back as all-x).
                                if dst.mode().is_four_value() {
                                    let known = b.ins().iconst(I64, mask_of(dst_size.get()));
                                    b.def_var(spc_map[dst], known);
                                }
                            }
                            IntrinsicOp::ReadMem(rm) => {
                                let (href, _rt, mode) = info.heap_ref(rm.signal);
                                let idx = self.compiler.read_mems.len();
                                self.compiler.read_mems.push((href, (**rm).clone()));
                                let cldctx = params.cldctx;
                                let heap = params.heap_ptr;
                                let heap_len =
                                    b.ins()
                                        .load(I64, mem(), cldctx, layout::CTX_HEAP_LEN as i32);
                                let mode_c = b
                                    .ins()
                                    .iconst(types::I8, i64::from(mode == LogicMode::FourValue));
                                let base =
                                    b.ins()
                                        .load(ptr, mem(), cldctx, layout::CTX_READMEMS as i32);
                                let entry = b
                                    .ins()
                                    .iadd_imm_u(base, (idx * layout::READMEM_ENTRY_SIZE) as i64);
                                let readmem_ptr =
                                    b.ins().load(ptr, mem(), cldctx, layout::CTX_READMEM as i32);
                                let mut sig = Signature::new(CallConv::SystemV);
                                sig.params
                                    .extend([ptr, I64, types::I8, ptr].map(AbiParam::new));
                                let sr = b.import_signature(sig);
                                b.ins().call_indirect(
                                    sr,
                                    readmem_ptr,
                                    &[heap, heap_len, mode_c, entry],
                                );
                                let z = b.ins().iconst(I64, 0);
                                b.def_var(vmap[dst], z);
                            }
                            IntrinsicOp::BlackBox => {
                                let size = gl.vars.size(*dst);
                                match SixBitSize::from_vector_size(size) {
                                    Some(_) => {
                                        let v = get(b, items[0]);
                                        b.def_var(vmap[dst], v);
                                        if dst.mode().is_four_value() {
                                            let s = spc_get(b, items[0]);
                                            b.def_var(spc_map[dst], s);
                                        }
                                    }
                                    None => {
                                        let dst_ptr = wide_addr(b, ptr, heap, wide_map[dst], 0);
                                        let src_ptr =
                                            wide_addr(b, ptr, heap, wide_map[&items[0]], 0);
                                        let nwords = var_words(size, dst.mode()) as u32;
                                        wide_copy(b, dst_ptr, src_ptr, nwords);
                                    }
                                }
                            }
                            // VCD dump intrinsics ($dumpfile/$dumpvars/$dumpoff/$dumpon) are
                            // a bytecode-only feature; the compiled backends (this one and
                            // the C transpiler) do not emit VCD. Fail cleanly at compile
                            // time rather than executing a machine trap.
                            IntrinsicOp::VcdOpenFile(_)
                            | IntrinsicOp::VcdAppendModule(_)
                            | IntrinsicOp::VcdPause
                            | IntrinsicOp::VcdResume => {
                                panic!("cranelift backend: VCD dump intrinsics are not supported")
                            }
                        }
                    }
                    Instruction::Probe(dst, signal, offset) => {
                        let dst_size = gl.vars.size(*dst);
                        let signal_size = gl.signals[*signal].size;

                        // A wide dst needs the multi-word extraction; the inline path
                        // below reads into a single-word (<=64) dst.
                        if SixBitSize::from_vector_size(dst_size).is_none() {
                            self.compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr);
                            continue;
                        }

                        let (href, _rt, mode) = info.heap_ref(*signal);
                        let heap = params.heap_ptr;
                        if mode == LogicMode::TwoValue {
                            let bit = href.offset.bit_offset + *offset as usize;
                            let word = bit / 64;
                            let shift = bit % 64;
                            let loaded = b.ins().load(I64, mem(), heap, (word * 8) as i32);
                            let shifted = if shift == 0 {
                                loaded
                            } else {
                                b.ins().ushr_imm_u(loaded, shift as i64)
                            };
                            let r = maskv(b, shifted, dst_size.get());
                            b.def_var(vmap[dst], r);
                        } else if *offset == 0 && dst_size == signal_size {
                            // Whole-signal probe: dst width matches the signal's storage.
                            let (val, spc) = fv_load(b, heap, href, dst_size.get());
                            b.def_var(vmap[dst], val);
                            b.def_var(spc_map[dst], spc);
                        } else {
                            // Four-value probe at a constant bit offset. The heap layout
                            // depends on the signal's own size: packed (<=32), split
                            // spc/val words (33..=64), or separate spc/val regions (>64).
                            let s_size = signal_size.get();
                            let d_mask = mask_of(dst_size.get());
                            let off = *offset as usize;
                            let base = href.offset.bit_offset;
                            if s_size <= 32 {
                                let word = base / 64;
                                let shift = base % 64;
                                let loaded = b.ins().load(I64, mem(), heap, (word * 8) as i32);
                                let s0 = b.ins().ushr_imm_u(loaded, (shift + off) as i64);
                                let spc = b.ins().band_imm_u(s0, d_mask);
                                let v0 = b
                                    .ins()
                                    .ushr_imm_u(loaded, (shift + s_size as usize + off) as i64);
                                let val = b.ins().band_imm_u(v0, d_mask);
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            } else if s_size <= 64 {
                                let word = base / 64;
                                let sw = b.ins().load(I64, mem(), heap, (word * 8) as i32);
                                let vw = b.ins().load(I64, mem(), heap, ((word + 1) * 8) as i32);
                                let s0 = b.ins().ushr_imm_u(sw, off as i64);
                                let spc = b.ins().band_imm_u(s0, d_mask);
                                let v0 = b.ins().ushr_imm_u(vw, off as i64);
                                let val = b.ins().band_imm_u(v0, d_mask);
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            } else {
                                let src_nwords = nwords(s_size);
                                let base_ptr = b.ins().iadd_imm_u(heap, ((base / 64) * 8) as i64);
                                let val_ptr = b
                                    .ins()
                                    .iadd_imm_u(base_ptr, (src_nwords as usize * 8) as i64);
                                let one = b.ins().iconst(types::I8, 1);
                                let offc = b.ins().iconst(I64, off as i64);
                                let (val, spc) = dyn_slice_read(
                                    b,
                                    val_ptr,
                                    Some(base_ptr),
                                    offc,
                                    one,
                                    s_size,
                                    dst_size.get(),
                                    0,
                                );
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }
                        }
                    }
                    Instruction::ProbeSlice(dst, signal, offset) => {
                        let dst_size = gl.vars.size(*dst);
                        let signal_size = gl.signals[*signal].size;
                        let offset_size = gl.vars.size(*offset);

                        let (href, _rt, mode) = info.heap_ref(*signal);
                        let heap = params.heap_ptr;
                        let src_nwords = nwords(signal_size.get());
                        let base_word = href.offset.bit_offset / 64;
                        let off = get(b, *offset);

                        // Like Slice: the whole result is x if the offset is fully out of
                        // range, or (four-value offset) the offset has any x/z bit. Guard
                        // it with an explicit (predictable) branch so the in-range extract
                        // needs no per-read oob/known selects.
                        let all_x = b.ins().icmp_imm_u(
                            IntCC::UnsignedGreaterThanOrEqual,
                            off,
                            signal_size.get() as i64,
                        );
                        let all_x = if offset.mode().is_four_value() {
                            let os = spc_get(b, *offset);
                            let special =
                                b.ins()
                                    .icmp_imm_u(IntCC::NotEqual, os, mask_of(offset_size.get()));
                            b.ins().bor(all_x, special)
                        } else {
                            all_x
                        };

                        let all_x_bb = b.create_block();
                        let probe_bb = b.create_block();
                        let done_bb = b.create_block();
                        b.ins().brif(all_x, all_x_bb, &[], probe_bb, &[]);

                        // Offset in range: read dst_size bits at `off`. The guard is
                        // shared, but a wide dst needs the multi-word extraction.
                        b.switch_to_block(probe_bb);
                        if SixBitSize::from_vector_size(dst_size).is_none() {
                            self.compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr);
                        } else if mode == LogicMode::FourValue && signal_size.get() <= 32 {
                            // Packed four-value signal: spc | (val << s_size) in one word.
                            // val/spc share the word, so a plain shift past the field pulls
                            // in the other plane's bits; mask any partial overhang past
                            // s_size to x (spc=0) with the in-range mask.
                            let d_mask = mask_of(dst_size.get());
                            let bshift = (href.offset.bit_offset % 64) as i64;
                            let word = href.offset.bit_offset / 64;
                            let loaded = b.ins().load(I64, mem(), heap, (word * 8) as i32);
                            let base_off = b.ins().iadd_imm_u(off, bshift);
                            let s0 = b.ins().ushr(loaded, base_off);
                            let spc_raw = b.ins().band_imm_u(s0, d_mask);
                            let val_off =
                                b.ins().iadd_imm_u(off, bshift + signal_size.get() as i64);
                            let v0 = b.ins().ushr(loaded, val_off);
                            let val_raw = b.ins().band_imm_u(v0, d_mask);
                            // in-range mask: off<=diff ? mask : mask>>(off-diff).
                            let diff = signal_size.get() - dst_size.get();
                            let le = b.ins().icmp_imm_u(
                                IntCC::UnsignedLessThanOrEqual,
                                off,
                                diff as i64,
                            );
                            let mc = b.ins().iconst(I64, d_mask);
                            let over = b.ins().iadd_imm_u(off, -(diff as i64));
                            let sh = b.ins().ushr(mc, over);
                            let inr = b.ins().select(le, mc, sh);
                            let val = b.ins().band(val_raw, inr);
                            let spc = b.ins().band(spc_raw, inr);
                            b.def_var(vmap[dst], val);
                            b.def_var(spc_map[dst], spc);
                        } else {
                            let base_ptr = b.ins().iadd_imm_u(heap, (base_word * 8) as i64);
                            let (val_ptr, spc_ptr) = if mode == LogicMode::FourValue {
                                let vp = b
                                    .ins()
                                    .iadd_imm_u(base_ptr, (src_nwords as usize * 8) as i64);
                                (vp, Some(base_ptr))
                            } else {
                                (base_ptr, None)
                            };
                            // The branch already handled a fully-out-of-range/unknown
                            // offset, so read with off_known = true.
                            let known = b.ins().iconst(types::I8, 1);
                            let (val, spc) = dyn_slice_read(
                                b,
                                val_ptr,
                                spc_ptr,
                                off,
                                known,
                                signal_size.get(),
                                dst_size.get(),
                                (href.offset.bit_offset % 64) as u32,
                            );
                            b.def_var(vmap[dst], val);
                            b.def_var(spc_map[dst], spc);
                        }
                        b.ins().jump(done_bb, &[]);

                        // Guard condition. Output is all `x`.
                        b.switch_to_block(all_x_bb);
                        match SixBitSize::from_vector_size(dst_size) {
                            Some(_) => {
                                let zero = b.ins().iconst(I64, 0);
                                b.def_var(vmap[dst], zero);
                                b.def_var(spc_map[dst], zero);
                            }
                            None => wide_fill(
                                b,
                                ptr,
                                heap,
                                wide_map[dst],
                                dst_size.get(),
                                dst.mode(),
                                vogls_bits::arithmetic::FvLogicValue::X,
                            ),
                        }
                        b.ins().jump(done_bb, &[]);

                        b.switch_to_block(done_bb);
                    }
                    Instruction::Drive(dst, signal, src, offset) => {
                        let ssize = gl.vars.size(*src).get();
                        let d_size = gl.signals[*signal].size.get();

                        let (_href, _rt, mode) = info.heap_ref(*signal);
                        // A wide (>64) signal driven by a four-value or wide source needs
                        // the multi-word drive. A two-value narrow source into a wide
                        // signal is still a single-word partial write (lower_drive_tv).
                        if SixBitSize::from_vector_size(gl.signals[*signal].size).is_none()
                            && (mode == LogicMode::FourValue || ssize > 64)
                        {
                            self.compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr);
                        } else if mode == LogicMode::TwoValue {
                            // lower_drive_tv already handles two-value partial writes.
                            let sv = get(b, *src);
                            self.compiler
                                .lower_drive_tv(b, params, *signal, sv, ssize, *offset);
                        } else if *offset == 0 && ssize == d_size {
                            let sv = get(b, *src);
                            let ss = spc_get(b, *src);
                            self.compiler
                                .lower_drive_fv(b, params, *signal, sv, ss, ssize);
                        } else {
                            // Four-value partial drive at a constant offset.
                            let sv = get(b, *src);
                            let ss = spc_get(b, *src);
                            let offc = b.ins().iconst(I64, *offset as i64);
                            let one = b.ins().iconst(types::I8, 1);
                            self.compiler.drive_partial(
                                b,
                                params,
                                *signal,
                                sv,
                                Some(ss),
                                ssize,
                                offc,
                                one,
                            );
                        }
                        // The dst is the changed-bits mask (src-sized, two-value); it is
                        // currently stubbed to zero. A wide dst lives in the wide storage.
                        match SixBitSize::from_vector_size(gl.vars.size(*dst)) {
                            Some(_) => {
                                let zero = b.ins().iconst(I64, 0);
                                b.def_var(vmap[dst], zero);
                            }
                            None => wide_fill(
                                b,
                                ptr,
                                heap,
                                wide_map[dst],
                                gl.vars.size(*dst).get(),
                                dst.mode(),
                                vogls_bits::arithmetic::FvLogicValue::L0,
                            ),
                        }
                    }
                    Instruction::DriveSlice(dst, signal, src, index) => {
                        let (_href, _rt, mode) = info.heap_ref(*signal);
                        let ssize = gl.vars.size(*src).get();
                        let index_size = gl.vars.size(*index).get();
                        // A wide (>64) signal needs the multi-word drive; drive_partial
                        // below writes a single-word (<=64) signal.
                        if SixBitSize::from_vector_size(gl.signals[*signal].size).is_none() {
                            self.compiler
                                .lower_wide_instruction(b, params, vmap, spc_map, wide_map, instr);
                        } else {
                            let off = get(b, *index);
                            let off_known = if index.mode().is_four_value() {
                                let os = spc_get(b, *index);
                                b.ins().icmp_imm_u(IntCC::Equal, os, mask_of(index_size))
                            } else {
                                b.ins().iconst(types::I8, 1)
                            };
                            let sv = get(b, *src);
                            let src_spc = if mode == LogicMode::FourValue {
                                Some(spc_get(b, *src))
                            } else {
                                None
                            };
                            self.compiler.drive_partial(
                                b, params, *signal, sv, src_spc, ssize, off, off_known,
                            );
                        }
                        // The dst is the changed-bits mask (src-sized, two-value); it is
                        // currently stubbed to zero. A wide dst lives in the wide storage.
                        match SixBitSize::from_vector_size(gl.vars.size(*dst)) {
                            Some(_) => {
                                let zero = b.ins().iconst(I64, 0);
                                b.def_var(vmap[dst], zero);
                            }
                            None => wide_fill(
                                b,
                                ptr,
                                heap,
                                wide_map[dst],
                                gl.vars.size(*dst).get(),
                                dst.mode(),
                                vogls_bits::arithmetic::FvLogicValue::L0,
                            ),
                        }
                    }
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
                    b.ins()
                        .call(sfe, &[params.schedule, next_time, next_tr_addr]);
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

/// Four-value bitwise AND of two operands given as `(val, spc)` planes,
/// returning the result `(val, spc)`. Shared by the `Binary` and `BinaryImm`
/// lowerings.
fn clif_fv_and(
    b: &mut FunctionBuilder,
    lval: Value,
    lspc: Value,
    rval: Value,
    rspc: Value,
) -> (Value, Value) {
    let a = b.ins().band(lspc, lval);
    let c = b.ins().band(rspc, rval);
    let val = b.ins().band(a, c);
    let nlv = b.ins().bnot(lval);
    let t1 = b.ins().band(lspc, nlv);
    let nrv = b.ins().bnot(rval);
    let t2 = b.ins().band(rspc, nrv);
    let t3 = b.ins().bor(t1, t2);
    let spc = b.ins().bor(t3, val);
    (val, spc)
}

/// Four-value bitwise OR of two operands given as `(val, spc)` planes.
/// Shared by the `Binary` and `BinaryImm` lowerings.
fn clif_fv_or(
    b: &mut FunctionBuilder,
    lval: Value,
    lspc: Value,
    rval: Value,
    rspc: Value,
) -> (Value, Value) {
    let a = b.ins().band(lspc, lval);
    let c = b.ins().band(rspc, rval);
    let val = b.ins().bor(a, c);
    let lsrs = b.ins().band(lspc, rspc);
    let spc = b.ins().bor(val, lsrs);
    (val, spc)
}

/// Four-value bitwise XOR of two operands given as `(val, spc)` planes.
/// Shared by the `Binary` and `BinaryImm` lowerings.
fn clif_fv_xor(
    b: &mut FunctionBuilder,
    lval: Value,
    lspc: Value,
    rval: Value,
    rspc: Value,
) -> (Value, Value) {
    let spc = b.ins().band(lspc, rspc);
    let xv = b.ins().bxor(lval, rval);
    let val = b.ins().band(spc, xv);
    (val, spc)
}

fn clif_fv_andnot(
    b: &mut FunctionBuilder,
    lval: Value,
    lspc: Value,
    rval: Value,
    rspc: Value,
) -> (Value, Value) {
    let a = b.ins().band(lspc, lval);
    let nrv = b.ins().bnot(rval);
    let c = b.ins().band(rspc, nrv);
    let val = b.ins().band(a, c);
    let nlv = b.ins().bnot(lval);
    let t1 = b.ins().band(lspc, nlv);
    let t2 = b.ins().band(rspc, rval);
    let t3 = b.ins().bor(t1, t2);
    let spc = b.ins().bor(t3, val);
    (val, spc)
}

fn clif_fv_ornot(
    b: &mut FunctionBuilder,
    lval: Value,
    lspc: Value,
    rval: Value,
    rspc: Value,
) -> (Value, Value) {
    let a = b.ins().band(lspc, lval);
    let nrv = b.ins().bnot(rval);
    let c = b.ins().band(rspc, nrv);
    let val = b.ins().bor(a, c);
    let lsrs = b.ins().band(lspc, rspc);
    let spc = b.ins().bor(val, lsrs);
    (val, spc)
}

fn clif_fv_xnor(
    b: &mut FunctionBuilder,
    lval: Value,
    lspc: Value,
    rval: Value,
    rspc: Value,
) -> (Value, Value) {
    let spc = b.ins().band(lspc, rspc);
    let xv = b.ins().bxor(lval, rval);
    let nxv = b.ins().bnot(xv);
    let val = b.ins().band(spc, nxv);
    (val, spc)
}

fn clif_fv_copyx(
    b: &mut FunctionBuilder,
    lval: Value,
    lspc: Value,
    rval: Value,
    rspc: Value,
) -> (Value, Value) {
    let nrs = b.ins().bnot(rspc);
    let nrv = b.ins().bnot(rval);
    let cm0 = b.ins().band(nrs, nrv);
    let ncm = b.ins().bnot(cm0);
    let spc = b.ins().band(lspc, ncm);
    let val = b.ins().band(lval, ncm);
    (val, spc)
}

fn clif_fv_copyz(
    b: &mut FunctionBuilder,
    lval: Value,
    lspc: Value,
    rval: Value,
    rspc: Value,
) -> (Value, Value) {
    let nrs = b.ins().bnot(rspc);
    let cm0 = b.ins().band(nrs, rval);
    let ncm = b.ins().bnot(cm0);
    let spc = b.ins().band(lspc, ncm);
    let val = b.ins().bor(lval, cm0);
    (val, spc)
}

fn sign_extend(
    b: &mut FunctionBuilder,
    val: Value,
    dst_size: SixBitSize,
    src_size: SixBitSize,
) -> Value {
    debug_assert!(dst_size >= src_size);
    if dst_size == src_size {
        return val;
    }

    let sh = 64 - src_size as i64;
    let up = b.ins().ishl_imm_u(val, sh);
    let sext = b.ins().sshr_imm_u(up, sh);
    maskvsbs(b, sext, dst_size)
}

/// Arithmetic shift right of a `size`-wide value by a compile-time constant
/// `amount`: sign-extend bit `size-1` up to i64, arithmetic-shift by `amount`
/// (clamped to 63 so a shift >= width fills with the sign bit), then re-mask is
/// left to the caller. Shared by the two- and four-value `ShiftImm` lowerings.
fn sign_shift_right(b: &mut FunctionBuilder, s: Value, amount: u32, size: u32) -> Value {
    let shb = (64 - size) as i64;
    let up = b.ins().ishl_imm_u(s, shb);
    let se = b.ins().sshr_imm_u(up, shb);
    let amt = amount.min(63) as i64;
    b.ins().sshr_imm_u(se, amt)
}

/// Copy `words` 64-bit words from `src_ptr` to `dst_ptr`. Used to move a wide
/// value's whole word block (both planes for four-value) in one go.
fn wide_copy(b: &mut FunctionBuilder, dst_ptr: Value, src_ptr: Value, words: u32) {
    for i in 0..words {
        let off = (i * 8) as i32;
        let w = b.ins().load(I64, mem(), src_ptr, off);
        b.ins().store(mem(), w, dst_ptr, off);
    }
}

/// Fill a wide (>64) value's word block with a single four-value logic value
/// (every bit set to `value`), masking the top word. For a four-value dst the
/// heap layout is n spc words then n val words.
fn wide_fill(
    b: &mut FunctionBuilder,
    ptr: Type,
    heap: Value,
    loc: WideLoc,
    size: u32,
    mode: LogicMode,
    value: vogls_bits::arithmetic::FvLogicValue,
) {
    let n = nwords(size);
    let plane = |b: &mut FunctionBuilder, base: u32, bit: bool| {
        let word = if bit { -1i64 } else { 0 };
        for i in 0..n {
            let w = if i == n - 1 {
                word & super::top_i64(size)
            } else {
                word
            };
            let c = b.ins().iconst(I64, w);
            wide_store(b, ptr, heap, loc, base + i, c);
        }
    };
    match mode {
        LogicMode::TwoValue => plane(b, 0, value.val()),
        LogicMode::FourValue => {
            plane(b, 0, value.spc()); // spc words first
            plane(b, n, value.val()); // then val words
        }
    }
}

/// Two-value unsigned divide with div-by-zero yielding 0 (substitute a divisor
/// of 1 to avoid a hardware trap, then force the result to 0). Shared by the
/// `Binary` and `BinaryImm` lowerings.
fn clif_tv_div0(b: &mut FunctionBuilder, l: Value, r: Value) -> Value {
    let is_zero = b.ins().icmp_imm_u(IntCC::Equal, r, 0);
    let one = b.ins().iconst(I64, 1);
    let safe = b.ins().select(is_zero, one, r);
    let q = b.ins().udiv(l, safe);
    let zero = b.ins().iconst(I64, 0);
    b.ins().select(is_zero, zero, q)
}

/// Two-value unsigned remainder with mod-by-zero yielding 0. See [`clif_tv_div0`].
fn clif_tv_mod0(b: &mut FunctionBuilder, l: Value, r: Value) -> Value {
    let is_zero = b.ins().icmp_imm_u(IntCC::Equal, r, 0);
    let one = b.ins().iconst(I64, 1);
    let safe = b.ins().select(is_zero, one, r);
    let q = b.ins().urem(l, safe);
    let zero = b.ins().iconst(I64, 0);
    b.ins().select(is_zero, zero, q)
}

/// Two-value unsigned divide for the x-on-div-by-zero variants. Substitutes a
/// divisor of 1 to avoid a hardware trap and returns `(quotient, nz)` where `nz`
/// is the non-zero-divisor flag; the caller injects x when `nz` is false (a
/// four-value dst). Shared by the `Binary` and `BinaryImm` lowerings.
fn clif_tv_divx(b: &mut FunctionBuilder, l: Value, r: Value) -> (Value, Value) {
    let nz = b.ins().icmp_imm_u(IntCC::NotEqual, r, 0);
    let one = b.ins().iconst(I64, 1);
    let safe = b.ins().select(nz, r, one);
    let q = b.ins().udiv(l, safe);
    (q, nz)
}

/// Two-value unsigned remainder for the x-on-mod-by-zero variants. See
/// [`clif_tv_divx`].
fn clif_tv_modx(b: &mut FunctionBuilder, l: Value, r: Value) -> (Value, Value) {
    let nz = b.ins().icmp_imm_u(IntCC::NotEqual, r, 0);
    let one = b.ins().iconst(I64, 1);
    let safe = b.ins().select(nz, r, one);
    let q = b.ins().urem(l, safe);
    (q, nz)
}

/// Gate an x-on-div-by-zero result (`raw`, `nz` from [`clif_tv_divx`]/
/// [`clif_tv_modx`]) for a four-value dst with fully-known operands: the result
/// is `raw` masked to `dsize` when the divisor is non-zero, else all-x. Shared
/// by the two-value-operand `Binary` and `BinaryImm` x-division lowerings.
fn clif_divx_gate(b: &mut FunctionBuilder, raw: Value, nz: Value, dsize: u32) -> (Value, Value) {
    let rawm = maskv(b, raw, dsize);
    let zero = b.ins().iconst(I64, 0);
    let mfull = b.ins().iconst(I64, mask_of(dsize));
    let val = b.ins().select(nz, rawm, zero);
    let spc = b.ins().select(nz, mfull, zero);
    (val, spc)
}

/// Gate a four-value arithmetic result on both operands being fully known:
/// if either operand has an x/z bit (or the optional `extra_known` flag is
/// false) the whole result is x, else it is `raw` masked to `dsize` with a
/// full special plane. Shared by the `Binary` and `BinaryImm` lowerings.
#[expect(clippy::too_many_arguments)]
fn clif_fv_arith(
    b: &mut FunctionBuilder,
    raw: Value,
    lspc: Value,
    lsize: u32,
    rspc: Value,
    rsize: u32,
    dsize: u32,
    extra_known: Option<Value>,
) -> (Value, Value) {
    let a = b.ins().icmp_imm_u(IntCC::Equal, lspc, mask_of(lsize));
    let c = b.ins().icmp_imm_u(IntCC::Equal, rspc, mask_of(rsize));
    let bk = b.ins().band(a, c);
    let gate = match extra_known {
        Some(x) => b.ins().band(bk, x),
        None => bk,
    };
    let rawm = maskv(b, raw, dsize);
    let zero = b.ins().iconst(I64, 0);
    let mfull = b.ins().iconst(I64, mask_of(dsize));
    let val = b.ins().select(gate, rawm, zero);
    let spc = b.ins().select(gate, mfull, zero);
    (val, spc)
}

fn wide_binary_bitwise(
    b: &mut FunctionBuilder,
    wide_map: &WideMap,
    size: VectorSize,
    ptr: Type,
    heap: Value,
    dst: VariableKey,
    lhs: VariableKey,
    rhs: VariableKey,
    tv: impl Fn(&mut FunctionBuilder, Value, Value) -> Value,
    fv: impl Fn(&mut FunctionBuilder, Value, Value, Value, Value) -> (Value, Value),
    needs_mask: bool,
) {
    let dst_base = wide_addr(b, ptr, heap, wide_map[&dst], 0);
    let lhs_base = wide_addr(b, ptr, heap, wide_map[&lhs], 0);
    let rhs_base = wide_addr(b, ptr, heap, wide_map[&rhs], 0);

    let val_offset = (HeapAlignment::spc_offset_to_val_offset(size, 0) / 8) as i32;
    let words = nwords(size.get());

    let bitwise_bb = b.create_block();
    let done_bb = b.create_block();
    let it = b.append_block_param(bitwise_bb, I64);

    let zero = b.ins().iconst(I64, 0);
    b.ins().jump(bitwise_bb, &[BlockArg::from(zero)]);

    b.switch_to_block(bitwise_bb);
    let lhs_ptr = b.ins().iadd(lhs_base, it);
    let rhs_ptr = b.ins().iadd(rhs_base, it);
    let dst_ptr = b.ins().iadd(dst_base, it);

    match dst.mode() {
        LogicMode::TwoValue => {
            let lhs_val = b.ins().load(I64, mem(), lhs_ptr, 0);
            let rhs_val = b.ins().load(I64, mem(), rhs_ptr, 0);
            let dst_val = tv(b, lhs_val, rhs_val);
            b.ins().store(mem(), dst_val, dst_ptr, 0);
        }
        LogicMode::FourValue => {
            let lhs_spc = b.ins().load(I64, mem(), lhs_ptr, 0);
            let rhs_spc = b.ins().load(I64, mem(), rhs_ptr, 0);
            let lhs_val = b.ins().load(I64, mem(), lhs_ptr, val_offset);
            let rhs_val = b.ins().load(I64, mem(), rhs_ptr, val_offset);
            let (dst_val, dst_spc) = fv(b, lhs_val, lhs_spc, rhs_val, rhs_spc);
            b.ins().store(mem(), dst_spc, dst_ptr, 0);
            b.ins().store(mem(), dst_val, dst_ptr, val_offset);
        }
    }
    let next = b.ins().iadd_imm_u(it, 8);
    let lt = b
        .ins()
        .icmp_imm_u(IntCC::UnsignedLessThan, next, (words * 8) as i64);
    b.ins()
        .brif(lt, bitwise_bb, &[BlockArg::from(next)], done_bb, &[]);

    b.switch_to_block(done_bb);
    if needs_mask && let Some(mask_size) = SixBitSize::last_word_size(size) {
        let last = b.ins().iadd_imm_u(dst_base, ((words - 1) * 8) as i64);
        match dst.mode() {
            LogicMode::TwoValue => {
                let last_word = b.ins().load(I64, mem(), last, 0);
                let masked = maskvsbs(b, last_word, mask_size);
                b.ins().store(mem(), masked, last, 0);
            }
            LogicMode::FourValue => {
                let last_word_spc = b.ins().load(I64, mem(), last, 0);
                let last_word_val = b.ins().load(I64, mem(), last, val_offset);
                let masked_spc = maskvsbs(b, last_word_spc, mask_size);
                let masked_val = maskvsbs(b, last_word_val, mask_size);
                b.ins().store(mem(), masked_spc, last, 0);
                b.ins().store(mem(), masked_val, last, val_offset);
            }
        }
    }
}
