mod ops;
mod lower;

use vogls_codegen::Heap;
use vogls_ir::VectorSize;

pub fn execute(op: &[u32], pc: &mut usize, heap: &mut Heap) {
    let mut regs = ops::Regs {
        dst: 0,
        lhs: 0,
        rhs: 0,
        size: VectorSize::new(1).unwrap(),
        size1offset: 0,
        size2offset: 0,
        size4offset: 0,
        size8offset: 0,
        size16offset: 0,
        size32offset: 0,
        size64offset: 0,
    };
    while let Some(&op) = op.get(*pc) {
        *pc += 1;
        let opcode = op >> 24;
        let f = ops::OP_TABLE[opcode as usize];
        f(op, heap, pc, &mut regs);
    }
}
