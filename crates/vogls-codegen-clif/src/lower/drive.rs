use cranelift_codegen::ir::{InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use vogls_codegen::HeapAlignment;
use vogls_ir::{SCALAR_VSIZE, VectorSize};

use super::{I64, mem};

pub fn load_bit(b: &mut FunctionBuilder, base_ptr: Value, bit: u64) -> Value {
    let word = b.ins().load(I64, mem(), base_ptr, ((bit / 64) * 8) as i32);
    let mut v = word;
    if bit % 64 != 0 {
        v = b.ins().ushr_imm_u(v, (bit % 64) as i64);
    }
    if bit % 64 < 63 {
        v = b.ins().band_imm_u(v, 1);
    }
    v
}

pub fn load_fv_bit(
    b: &mut FunctionBuilder,
    signal_size: VectorSize,
    base_ptr: Value,
    offset: u64,
) -> (Value, Value) {
    let woff = offset / 64;
    let boff = offset % 64;

    if signal_size == SCALAR_VSIZE {
        let old_word = b.ins().load(I64, mem(), base_ptr, (woff * 8) as i32);
        let mut old_value = old_word;
        if boff != 0 {
            old_value = b.ins().ushr_imm_u(old_value, boff as i64);
        }
        let old_spc = b.ins().band_imm_u(old_value, 1);
        let mut old_val = b.ins().ushr_imm_u(old_value, 1);
        if boff < 62 {
            old_val = b.ins().band_imm_u(old_val, 1);
        }

        (old_spc, old_val)
    } else {
        let spc_bit = offset as u64;
        let val_bit = HeapAlignment::spc_offset_to_val_offset(signal_size, spc_bit);

        (
            load_bit(b, base_ptr, spc_bit),
            load_bit(b, base_ptr, val_bit),
        )
    }
}

pub fn load_var_bit(b: &mut FunctionBuilder, base_ptr: Value, bit: Value) -> Value {
    let woff = b.ins().ushr_imm_u(bit, 6);
    let woff = b.ins().ishl_imm_u(woff, 2);
    let boff = b.ins().band_imm_u(bit, 0x3F);
    let ptr = b.ins().iadd(base_ptr, woff);
    let word = b.ins().load(I64, mem(), ptr, 0);
    let word = b.ins().ushr(word, boff);
    let word = b.ins().band_imm_u(word, 1);
    word
}
