use super::*;

pub extern "rust-preserve-none" fn extract_and_execute_tailcall<I: BytecodeInstruction>(
    code: &[Bytecode],
    regs: &mut Regs,
    pc: u64,
    state: &mut RuntimeState,
    schedule: &mut Schedule,
    listeners: &mut BytecodeListeners,
    cldctx: &mut ColdContext,
) {
    let c = code[pc as usize];
    let slf = I::extract(c);
    let mut pc = pc + 1;
    slf.execute(code, regs, &mut pc, state, schedule, listeners, cldctx);
    let Some(c) = code.get(pc as usize) else {
        return;
    };

    let opcode = c.opcode();
    let f = X_INSTRUCTION_TAILCALL_FNS[opcode as usize];
    become (f)(code, regs, pc, state, schedule, listeners, cldctx)
}
