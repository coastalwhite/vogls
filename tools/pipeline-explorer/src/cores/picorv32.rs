use vogls::design::RtSignal;

use super::{
    Core, DATA_MEMORY, DATA_MEMORY_ADDR, Description, INSTRUCTION_MEMORY, INSTRUCTION_MEMORY_ADDR,
    PipelineTrace, Sim, Testbench, enabled, lookup, signal,
};
use crate::TraceError;
use crate::program::ProgramSections;

const BOOT: u32 = 0x0000_0000;

/// The states of `cpu_state`, from its most significant one-hot bit down.
const STATES: [&str; 8] = ["TR", "IF", "L1", "L2", "EX", "SH", "ST", "LD"];

#[derive(Debug, Clone, Default)]
pub struct PicoRV32Config {
    pub enable_mul: bool,
    pub enable_div: bool,
    pub two_stage_shift: bool,
    pub barrel_shifter: bool,
    pub two_cycle_compare: bool,
    pub two_cycle_alu: bool,
    pub enable_fast_mul: bool,
}

pub struct PicoRv32;

pub struct PicoRv32Design {
    tb: Testbench,
    trap: RtSignal,
    reg_pc: RtSignal,
    cpu_state: RtSignal,
}

impl Core for PicoRv32 {
    type Design = PicoRv32Design;
    type State = Sim;
    type Params = PicoRV32Config;

    fn description(&self, _params: &PicoRV32Config) -> Description {
        Description {
            pipeline_stages: STATES.to_vec(),
            boot: BOOT,
            imem_addr: INSTRUCTION_MEMORY_ADDR,
            imem_size: INSTRUCTION_MEMORY,
            dmem_addr: DATA_MEMORY_ADDR,
            dmem_size: DATA_MEMORY,
        }
    }

    fn compile(&self, params: &PicoRV32Config) -> Result<(PicoRv32Design, Sim), TraceError> {
        let macros = enabled([
            (params.enable_mul, "ENABLE_MUL"),
            (params.enable_div, "ENABLE_DIV"),
            (params.two_stage_shift, "TWO_STAGE_SHIFT"),
            (params.barrel_shifter, "BARREL_SHIFTER"),
            (params.two_cycle_compare, "TWO_CYCLE_COMPARE"),
            (params.two_cycle_alu, "TWO_CYCLE_ALU"),
            (params.enable_fast_mul, "ENABLE_FAST_MUL"),
        ]);
        let (tb, sim, (trap, reg_pc, cpu_state)) = Testbench::compile(
            &[
                include_str!("../../../../submodules/picorv32/picorv32.v"),
                include_str!("../../instances/picorv32/tb.v"),
            ],
            macros,
            "resetn",
            ["memory", "primes"],
            &self.description(params),
            |design, root| {
                let trap = signal(design, root, "trap");
                let proc = lookup(design, root, "proc");
                (
                    trap,
                    signal(design, proc, "reg_pc"),
                    signal(design, proc, "cpu_state"),
                )
            },
        )?;

        let design = PicoRv32Design {
            trap: tb.resolve(trap),
            reg_pc: tb.resolve(reg_pc),
            cpu_state: tb.resolve(cpu_state),
            tb,
        };
        Ok((design, sim))
    }

    fn prologue(
        &self,
        design: &PicoRv32Design,
        state: &mut Sim,
        sections: &ProgramSections,
    ) -> Result<(), TraceError> {
        design.tb.reset(state, sections)
    }

    fn iterate_cycle(
        &self,
        design: &PicoRv32Design,
        state: &mut Sim,
        pt: &mut PipelineTrace,
    ) -> Result<(), TraceError> {
        let tb = &design.tb;
        tb.step(state)?;

        let pc = tb.get(state, design.reg_pc).extract_exact_u32();
        // `cpu_state` is one-hot, with the first state in its top bit.
        let current = tb
            .get(state, design.cpu_state)
            .extract_u32()
            .and_then(|bits| 7u32.checked_sub(bits.trailing_zeros()));

        for i in 0..STATES.len() {
            pt.push(i, pc.filter(|_| current == Some(i as u32)));
        }
        Ok(())
    }

    fn is_trapped(&self, design: &PicoRv32Design, state: &Sim) -> bool {
        design.tb.is_one(state, design.trap)
    }
}
