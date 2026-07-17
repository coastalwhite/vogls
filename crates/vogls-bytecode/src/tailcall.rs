use super::*;

pub type TailcallFn = extern "rust-preserve-none" fn(
    c: Bytecode,
    code: &[Bytecode],
    regs: &mut Regs,
    pc: u64,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    cldctx: &mut ColdContext,
);

pub extern "rust-preserve-none" fn extract_and_execute_tailcall<I: BytecodeInstruction>(
    c: Bytecode,
    code: &[Bytecode],
    regs: &mut Regs,
    pc: u64,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    cldctx: &mut ColdContext,
) {
    let slf = I::extract(c);
    let mut pc = pc + 1;
    slf.execute(code, regs, &mut pc, state, schedule, listeners, cldctx);
    let Some(c) = code.get(pc as usize) else {
        return;
    };

    let opcode = c.opcode();
    let f = TAILCALL_INSTR_FNS[opcode as usize];
    become (f)(*c, code, regs, pc, state, schedule, listeners, cldctx)
}
