use std::mem::offset_of;

use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{
    Block, InstBuilder, StackSlotData, StackSlotKind, UserFuncName, Value,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Module};
use vogls_codegen::SixBitSize;
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryOp, Instruction, LogicMode, UnaryOp, VariableKey,
};
use vogls_utils::VgHashMap;

use crate::ffi::FfiVec;
use crate::lower::{
    F64, I64, Params, WIDE_HEAP_THRESHOLD_WORDS, WideLoc, cast, instr_is_wide, is_real_unop,
    mask_of, maskv, mem, var_words,
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
                    continue;
                }

                let b = &mut self.b;
                let params = &mut self.params;
                let vmap = &mut self.vmap;
                let spc_map = &mut self.spc_map;

                let get = |b: &mut FunctionBuilder, v: VariableKey| b.use_var(vmap[&v]);
                // A two-value operand used in a four-value op is fully known (spc = mask).
                let spc_get = |b: &mut FunctionBuilder, v: VariableKey| {
                    debug_assert_eq!(v.mode(), LogicMode::FourValue);
                    b.use_var(spc_map[&v])
                };

                match instr {
                    // Phi nodes are realized by the per-block phi copies emitted at the
                    // end of each predecessor block, so the instruction itself is a no-op.
                    Instruction::Phi(..) => {}
                    Instruction::Constant(dst, bits) => {
                        debug_assert!(!bits.contains_special() || dst.mode().is_four_value());
                        debug_assert!(bits.size().get() <= 64);

                        let size = bits.size();
                        match bits.clone_lowering_mode().as_data_ref() {
                            vogls_bits::BitsDataRef::InlineTv(v) => {
                                let val = b.ins().iconst(I64, v as i64);
                                b.def_var(vmap[dst], val);
                                if dst.mode().is_four_value() {
                                    let spc = b.ins().iconst(I64, mask_of(size.get()));
                                    b.def_var(spc_map[dst], spc);
                                }
                            }
                            vogls_bits::BitsDataRef::InlineFv(spc, val) => {
                                let val = b.ins().iconst(I64, val as i64);
                                let spc = b.ins().iconst(I64, spc as i64);
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }
                            vogls_bits::BitsDataRef::SeparateFv(s) => {
                                // 33..=64: [spc, val].
                                let val = b.ins().iconst(I64, s[1] as i64);
                                let spc = b.ins().iconst(I64, s[0] as i64);
                                b.def_var(vmap[dst], val);
                                b.def_var(spc_map[dst], spc);
                            }
                            vogls_bits::BitsDataRef::SeparateTv(_) => unreachable!(),
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

                        let dst_size = self.compiler.gl.vars.size(*dst);
                        let src_size = self.compiler.gl.vars.size(*src);

                        let native =
                            |b: &mut FunctionBuilder, x: Value| b.ins().bitcast(I64, cast(), x);

                        use crate::runtime::real_code as rc;
                        use LogicMode as M;
                        use UnaryOp as O;
                        match (op, dst.mode(), src.mode()) {
                            (O::TvToFv, _, _) => {
                                map!(val => @fv { (val, b.ins().iconst(I64, mask_of(src_size.get()))) })
                            }
                            (O::FvToTv, _, _) => map!(val, spc => @tv { b.ins().band(val, spc) }),
                            (UnaryOp::Neg, M::TwoValue, _) => map!(val => @tv {
                                let n = b.ins().bnot(val);
                                maskv(b, n, dst_size.get())
                            }),
                            (UnaryOp::Neg, M::FourValue, _) => map!(val, spc => @fv {
                                let nsv = b.ins().bnot(val);
                                let val = b.ins().band(spc, nsv);
                                (val, spc)
                            }),
                            (UnaryOp::ReduceOr, M::TwoValue, _) => map!(val => @tv {
                                let c = b.ins().icmp_imm_u(IntCC::NotEqual, val, 0);
                                b.ins().uextend(I64, c)
                            }),
                            (UnaryOp::ReduceOr, M::FourValue, _) => map!(val, spc => @fv {
                                // See fv_reduce_or_elem.
                                let sandv = b.ins().band(spc, val);
                                let z0 = b.ins().icmp_imm_u(IntCC::NotEqual, sandv, 0);
                                let allk = b.ins().icmp_imm_u(IntCC::Equal, spc, mask_of(src_size.get()));
                                let z1 = b.ins().bor(allk, z0);
                                let val = b.ins().uextend(I64, z0);
                                let spc = b.ins().uextend(I64, z1);
                                (val, spc)
                            }),
                            (UnaryOp::ReduceAnd, M::TwoValue, _) => map!(val => @tv {
                                let m = mask_of(src_size.get());
                                let c = b.ins().icmp_imm_u(IntCC::Equal, val, m);
                                b.ins().uextend(I64, c)
                            }),
                            (UnaryOp::ReduceAnd, M::FourValue, _) => map!(val, spc => @fv {
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
                            (UnaryOp::ReduceXor, M::TwoValue, _) => map!(val => @tv {
                                let p = b.ins().popcnt(val);
                                b.ins().band_imm_u(p, 1)
                            }),
                            (UnaryOp::ReduceXor, M::FourValue, _) => map!(val, spc => @fv {
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
                            (UnaryOp::LeadingZeros, M::TwoValue, _) => map!(val => @tv {
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
                            (UnaryOp::LeadingZeros, M::FourValue, _) => map!(val, spc => @fv {
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

                            (O::RealNeg, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().fneg(fs); native(b, r) })
                            }
                            (O::RealSqrt, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().sqrt(fs); native(b, r) })
                            }
                            (O::RealFloor, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().floor(fs); native(b, r) })
                            }
                            (O::RealCeil, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().ceil(fs); native(b, r) })
                            }
                            (O::RealTruncate, _, _) => {
                                map!(val => @tv { let fs = b.ins().bitcast(F64, cast(), val); let r = b.ins().trunc(fs); native(b, r) })
                            }
                            (O::RealToLogical, _, _) => map!(val => @tv {
                                let fs = b.ins().bitcast(F64, cast(), val);
                                let z = b.ins().f64const(0.0);
                                let c = b.ins().fcmp(FloatCC::NotEqual, fs, z);
                                b.ins().uextend(I64, c)
                            }),
                            (O::RealToU64 | O::RealToI64, _, _) => map!(val => @tv {
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
                            (O::RealFromSignedDecimal, _, _) => map!(val => @tv {
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
                            (O::RealFromUnsignedDecimal, _, _) => map!(val => @tv {
                                let x = b.ins().fcvt_from_uint(F64, val);
                                native(b, x)
                            }),
                            (O::RealLn, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::LN, val, val))
                            }
                            (O::RealLog10, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::LOG10, val, val))
                            }
                            (O::RealExp, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::EXP, val, val))
                            }
                            (O::RealSin, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::SIN, val, val))
                            }
                            (O::RealCos, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::COS, val, val))
                            }
                            (O::RealTan, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::TAN, val, val))
                            }
                            (O::RealASin, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ASIN, val, val))
                            }
                            (O::RealACos, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ACOS, val, val))
                            }
                            (O::RealATan, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ATAN, val, val))
                            }
                            (O::RealSinH, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::SINH, val, val))
                            }
                            (O::RealCosH, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::COSH, val, val))
                            }
                            (O::RealTanH, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::TANH, val, val))
                            }
                            (O::RealASinH, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ASINH, val, val))
                            }
                            (O::RealACosH, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ACOSH, val, val))
                            }
                            (O::RealATanH, _, _) => {
                                map!(val => @tv self.compiler.real_shim(b, params, rc::ATANH, val, val))
                            }
                        }
                    }
                    Instruction::Binary(dst, op, lhs, rhs) => {
                        let dst_size = self.compiler.gl.vars.size(*dst);
                        let lhs_size = self.compiler.gl.vars.size(*lhs);
                        let rhs_size = self.compiler.gl.vars.size(*rhs);

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

                                        let a = b.ins().icmp_imm_u(
                                            IntCC::Equal,
                                            lspc,
                                            mask_of(lhs_size.get()),
                                        );
                                        let c = b.ins().icmp_imm_u(
                                            IntCC::Equal,
                                            rspc,
                                            mask_of(rhs_size.get()),
                                        );
                                        let both_known = b.ins().band(a, c);
                                        let zero = b.ins().iconst(I64, 0);
                                        let mfull = b.ins().iconst(I64, mask_of(dst_size.get()));
                                        let val = b.ins().select(both_known, result, zero);
                                        let spc = b.ins().select(both_known, mfull, zero);
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
                            let big = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThanOrEqual, amt, 64);
                            b.ins().select(big, zero, sh)
                        };
                        let ushr = |b: &mut FunctionBuilder, v: Value, amt: Value| {
                            let sh = b.ins().ushr(v, amt);
                            let zero = b.ins().iconst(I64, 0);
                            let big = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThanOrEqual, amt, 64);
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
                        let shift_amount =
                            |b: &mut FunctionBuilder, v: VariableKey, size: u32| -> (Value, Value) {
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
                        match (op, dst.mode(), lhs.mode()) {
                            (O::And, M::TwoValue, _) => {
                                map!(lhs, rhs => @tv b.ins().band(lhs, rhs))
                            }
                            (O::Or, M::TwoValue, _) => map!(lhs, rhs => @tv b.ins().bor(lhs, rhs)),
                            (O::Xor, M::TwoValue, _) => {
                                map!(lhs, rhs => @tv b.ins().bxor(lhs, rhs))
                            }
                            (O::AndNot, M::TwoValue, _) => {
                                map!(lhs, rhs => @tv {
                                    let and_not = b.ins().band_not(lhs, rhs);
                                    maskv(b, and_not, dst_size.get())
                                })
                            }
                            (O::OrNot, M::TwoValue, _) => map!(lhs, rhs => @tv {
                                let or_not = b.ins().bor_not(lhs, rhs);
                                maskv(b, or_not, dst_size.get())
                            }),
                            (O::Xnor, M::TwoValue, _) => {
                                map!(lhs, rhs => @tv {
                                    let xnor = b.ins().bxor_not(lhs, rhs);
                                    maskv(b, xnor, dst_size.get())
                                })
                            }

                            (O::And, M::FourValue, _) => map!((lval, lspc), (rval, rspc) => @fv {
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
                            }),
                            (O::Or, M::FourValue, _) => map!((lval, lspc), (rval, rspc) => @fv {
                                let a = b.ins().band(lspc, lval);
                                let c = b.ins().band(rspc, rval);
                                let val = b.ins().bor(a, c);
                                let lsrs = b.ins().band(lspc, rspc);
                                let spc = b.ins().bor(val, lsrs);
                                (val, spc)
                            }),
                            (O::Xor, M::FourValue, _) => map!((lval, lspc), (rval, rspc) => @fv {
                                let spc = b.ins().band(lspc, rspc);
                                let xv = b.ins().bxor(lval, rval);
                                let val = b.ins().band(spc, xv);
                                (val, spc)
                            }),
                            (O::AndNot, M::FourValue, _) => map!((lval, lspc), (rval, rspc) => @fv {
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
                            }),
                            (O::OrNot, M::FourValue, _) => map!((lval, lspc), (rval, rspc) => @fv {
                                let a = b.ins().band(lspc, lval);
                                let nrv = b.ins().bnot(rval);
                                let c = b.ins().band(rspc, nrv);
                                let val = b.ins().bor(a, c);
                                let lsrs = b.ins().band(lspc, rspc);
                                let spc = b.ins().bor(val, lsrs);
                                (val, spc)
                            }),
                            (O::Xnor, M::FourValue, _) => map!((lval, lspc), (rval, rspc) => @fv {
                                let spc = b.ins().band(lspc, rspc);
                                let xv = b.ins().bxor(lval, rval);
                                let nxv = b.ins().bnot(xv);
                                let val = b.ins().band(spc, nxv);
                                (val, spc)
                            }),

                            (O::Add, _, _) => bin_arith!(lhs, rhs => b.ins().iadd(lhs, rhs), true),
                            (O::Sub, _, _) => bin_arith!(lhs, rhs => b.ins().isub(lhs, rhs), true),
                            (O::Multiply, _, _) => {
                                bin_arith!(lhs, rhs => b.ins().imul(lhs, rhs), true)
                            }
                            (O::Min, _, _) => bin_arith!(lhs, rhs => b.ins().umin(lhs, rhs), false),
                            (O::Max, _, _) => bin_arith!(lhs, rhs => b.ins().umax(lhs, rhs), false),

                            (O::Negedge, _, M::TwoValue) => {
                                map!(lhs, rhs => @tv {
                                    let and_not = b.ins().band_not(lhs, rhs);
                                    maskv(b, and_not, dst_size.get())
                                })
                            }
                            (O::Negedge, _, M::FourValue) => {
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
                            (O::CaseEquality, _, M::TwoValue) => map!(lhs, rhs => @tv {
                                let eq = b.ins().icmp(IntCC::Equal, lhs, rhs);
                                b.ins().uextend(I64, eq)
                            }),
                            (O::CaseEquality, _, M::FourValue) => {
                                map!((lval, lspc), (rval, rspc) => @tv {
                                    let vm = b.ins().icmp(IntCC::Equal, lval, rval);
                                    let sm = b.ins().icmp(IntCC::Equal, lspc, rspc);
                                    let eq = b.ins().band(vm, sm);
                                    b.ins().uextend(I64, eq)
                                })
                            }
                            (O::Power, _, _) => self.compiler.emit_wide_binop(
                                b, params, *op, *dst, *lhs, *rhs, vmap, spc_map, &self.wide_map,
                            ),
                            (O::DivideX | O::ModulusX, _, M::TwoValue) => map!(lhs, rhs => @fv {
                                let nz = b.ins().icmp_imm_u(IntCC::NotEqual, rhs, 0);
                                let one = b.ins().iconst(I64, 1);
                                let safe = b.ins().select(nz, rhs, one);
                                let raw = if matches!(op, O::DivideX) {
                                    b.ins().udiv(lhs, safe)
                                } else {
                                    b.ins().urem(lhs, safe)
                                };
                                let rawm = maskv(b, raw, dst_size.get());
                                let zero = b.ins().iconst(I64, 0);
                                let mfull = b.ins().iconst(I64, mask_of(dst_size.get()));
                                let val = b.ins().select(nz, rawm, zero);
                                let spc = b.ins().select(nz, mfull, zero);
                                (val, spc)
                            }),
                            (O::DivideX | O::ModulusX, _, M::FourValue) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let nz = b.ins().icmp_imm_u(IntCC::NotEqual, rval, 0);
                                    let one = b.ins().iconst(I64, 1);
                                    let safe = b.ins().select(nz, rval, one);
                                    let raw = if matches!(op, O::DivideX) {
                                        b.ins().udiv(lval, safe)
                                    } else {
                                        b.ins().urem(lval, safe)
                                    };
                                    let a = b.ins().icmp_imm_u(IntCC::Equal, lspc, mask_of(lhs_size.get()));
                                    let c = b.ins().icmp_imm_u(IntCC::Equal, rspc, mask_of(rhs_size.get()));
                                    let bk = b.ins().band(a, c);
                                    let gate = b.ins().band(bk, nz);
                                    let rawm = maskv(b, raw, dst_size.get());
                                    let zero = b.ins().iconst(I64, 0);
                                    let mfull = b.ins().iconst(I64, mask_of(dst_size.get()));
                                    let val = b.ins().select(gate, rawm, zero);
                                    let spc = b.ins().select(gate, mfull, zero);
                                    (val, spc)
                                })
                            }
                            (O::Divide0 | O::Modulus0, _, M::TwoValue) => map!(lhs, rhs => @tv {
                                let is_zero = b.ins().icmp_imm_u(IntCC::Equal, rhs, 0);
                                let one = b.ins().iconst(I64, 1);
                                let safe = b.ins().select(is_zero, one, rhs);
                                let q = if matches!(op, O::Divide0) {
                                    b.ins().udiv(lhs, safe)
                                } else {
                                    b.ins().urem(lhs, safe)
                                };
                                let zero = b.ins().iconst(I64, 0);
                                b.ins().select(is_zero, zero, q)
                            }),
                            (O::Divide0 | O::Modulus0, _, M::FourValue) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let is_zero = b.ins().icmp_imm_u(IntCC::Equal, rval, 0);
                                    let one = b.ins().iconst(I64, 1);
                                    let safe = b.ins().select(is_zero, one, rval);
                                    let q = if matches!(op, O::Divide0) {
                                        b.ins().udiv(lval, safe)
                                    } else {
                                        b.ins().urem(lval, safe)
                                    };
                                    let zero = b.ins().iconst(I64, 0);
                                    let raw = b.ins().select(is_zero, zero, q);
                                    let a = b.ins().icmp_imm_u(IntCC::Equal, lspc, mask_of(lhs_size.get()));
                                    let c = b.ins().icmp_imm_u(IntCC::Equal, rspc, mask_of(rhs_size.get()));
                                    let bk = b.ins().band(a, c);
                                    let rawm = maskv(b, raw, dst_size.get());
                                    let mfull = b.ins().iconst(I64, mask_of(dst_size.get()));
                                    let val = b.ins().select(bk, rawm, zero);
                                    let spc = b.ins().select(bk, mfull, zero);
                                    (val, spc)
                                })
                            }
                            (O::UnsignedLessEqual, M::TwoValue, _) => map!(lhs, rhs => @tv {
                                let c = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, lhs, rhs);
                                b.ins().uextend(I64, c)
                            }),
                            (O::UnsignedLessEqual, M::FourValue, _) => {
                                map!((lval, lspc), (rval, rspc) => @fv {
                                    let a = b.ins().icmp_imm_u(IntCC::Equal, lspc, mask_of(lhs_size.get()));
                                    let c = b.ins().icmp_imm_u(IntCC::Equal, rspc, mask_of(rhs_size.get()));
                                    let bk = b.ins().band(a, c);
                                    let cmp = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, lval, rval);
                                    let cmpv = b.ins().uextend(I64, cmp);
                                    let zero = b.ins().iconst(I64, 0);
                                    let one = b.ins().iconst(I64, 1);
                                    let val = b.ins().select(bk, cmpv, zero);
                                    let spc = b.ins().select(bk, one, zero);
                                    (val, spc)
                                })
                            }
                            (O::LogicalShiftLeft, M::TwoValue, _) => map!(lhs, rhs => @tv {
                                let sh = ushl(b, lhs, rhs);
                                maskv(b, sh, dst_size.get())
                            }),
                            (O::LogicalShiftRight, M::TwoValue, _) => map!(lhs, rhs => @tv {
                                // rhs is canonical, so the shifted value stays canonical.
                                ushr(b, lhs, rhs)
                            }),
                            (O::ArithmeticShiftRight, M::TwoValue, _) => map!(lhs, rhs => @tv {
                                let sh = ashr(b, lhs, rhs, dst_size.get());
                                maskv(b, sh, dst_size.get())
                            }),
                            (O::LogicalShiftLeft, M::FourValue, _) => {
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
                            (O::LogicalShiftRight, M::FourValue, _) => {
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
                            (O::ArithmeticShiftRight, M::FourValue, _) => {
                                map!(lval, lspc, amt = shift_amount(b, *rhs, rhs_size.get()) => @fv_shift {
                                    let ssh = ashr(b, lspc, amt, dst_size.get());
                                    let spc = maskv(b, ssh, dst_size.get());
                                    let vsh = ashr(b, lval, amt, dst_size.get());
                                    let val = maskv(b, vsh, dst_size.get());
                                    (val, spc)
                                })
                            }
                            (O::Concat, M::TwoValue, _) => map!(lhs, rhs => @tv {
                                let hi = b.ins().ishl_imm_u(lhs, rhs_size.get() as i64);
                                b.ins().bor(hi, rhs)
                            }),
                            (O::Concat, M::FourValue, _) => map!((lval, lspc), (rval, rspc) => @fv {
                                let vh = b.ins().ishl_imm_u(lval, rhs_size.get() as i64);
                                let val = b.ins().bor(vh, rval);
                                let sh = b.ins().ishl_imm_u(lspc, rhs_size.get() as i64);
                                let spc = b.ins().bor(sh, rspc);
                                (val, spc)
                            }),
                            (O::CopyX, M::TwoValue, _) => map!(lhs, _rhs => @tv lhs),
                            (O::CopyX, M::FourValue, _) => map!((lval, lspc), (rval, rspc) => @fv {
                                let nrs = b.ins().bnot(rspc);
                                let nrv = b.ins().bnot(rval);
                                let cm0 = b.ins().band(nrs, nrv);
                                let cm = maskv(b, cm0, dst_size.get());
                                let ncm = b.ins().bnot(cm);
                                let spc = b.ins().band(lspc, ncm);
                                let val = b.ins().band(lval, ncm);
                                (val, spc)
                            }),
                            (O::CopyZ, M::TwoValue, _) => map!(lhs, _rhs => @tv lhs),
                            (O::CopyZ, M::FourValue, _) => map!((lval, lspc), (rval, rspc) => @fv {
                                let nrs = b.ins().bnot(rspc);
                                let cm0 = b.ins().band(nrs, rval);
                                let cm = maskv(b, cm0, dst_size.get());
                                let ncm = b.ins().bnot(cm);
                                let spc = b.ins().band(lspc, ncm);
                                let v0 = b.ins().bor(lval, cm);
                                let val = maskv(b, v0, dst_size.get());
                                (val, spc)
                            }),
                            (O::RealAdd, _, _) => map!(lhs, rhs => @real b.ins().fadd(lhs, rhs)),
                            (O::RealSub, _, _) => map!(lhs, rhs => @real b.ins().fsub(lhs, rhs)),
                            (O::RealMul, _, _) => map!(lhs, rhs => @real b.ins().fmul(lhs, rhs)),
                            (O::RealDiv, _, _) => map!(lhs, rhs => @real b.ins().fdiv(lhs, rhs)),
                            (O::RealPow, _, _) => {
                                map!(lhs, rhs => @tv self.compiler.real_shim(b, params, rc::POW, lhs, rhs))
                            }
                            (O::RealEq, _, _) => real_cmp(b, FloatCC::Equal),
                            (O::RealNe, _, _) => real_cmp(b, FloatCC::NotEqual),
                            (O::RealLt, _, _) => real_cmp(b, FloatCC::LessThan),
                            (O::RealLeq, _, _) => real_cmp(b, FloatCC::LessThanOrEqual),
                            (O::RealGt, _, _) => real_cmp(b, FloatCC::GreaterThan),
                            (O::RealGeq, _, _) => real_cmp(b, FloatCC::GreaterThanOrEqual),
                            (O::RealATan2, _, _) => {
                                map!(lhs, rhs => @tv self.compiler.real_shim(b, params, rc::ATAN2, lhs, rhs))
                            }
                            (O::RealHypot, _, _) => {
                                map!(lhs, rhs => @tv self.compiler.real_shim(b, params, rc::HYPOT, lhs, rhs))
                            }
                        }
                    }
                    _ => {
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
