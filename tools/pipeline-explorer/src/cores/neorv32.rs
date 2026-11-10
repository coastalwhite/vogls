use vogls::design::RtSignal;

use super::{
    Core, DATA_MEMORY, DATA_MEMORY_ADDR, Description, INSTRUCTION_MEMORY, INSTRUCTION_MEMORY_ADDR,
    PipelineTrace, Sim, Testbench, signal,
};
use crate::TraceError;
use crate::program::ProgramSections;

const BOOT: u32 = 0x0000_0000;

pub struct Neorv32;

pub struct Neorv32Design {
    tb: Testbench,
    trap: RtSignal,
    // Registered stage-active flags and PCs from tb.v. Fetch has a PC of its
    // own; every later stage works on `pc_q`.
    if_q: RtSignal,
    if_pc_q: RtSignal,
    pc_q: RtSignal,
    /// The active flags of IS, EX, ALU, BR and MA.
    stages: [RtSignal; 5],
}

impl Core for Neorv32 {
    type Design = Neorv32Design;
    type State = Sim;
    type Params = ();

    fn description(&self, _params: &()) -> Description {
        Description {
            pipeline_stages: vec!["IF", "IS", "EX", "ALU", "BR", "MA"],
            boot: BOOT,
            imem_addr: INSTRUCTION_MEMORY_ADDR,
            imem_size: INSTRUCTION_MEMORY,
            dmem_addr: DATA_MEMORY_ADDR,
            dmem_size: DATA_MEMORY,
        }
    }

    fn compile(&self, params: &()) -> Result<(Neorv32Design, Sim), TraceError> {
        let (tb, sim, handles) = Testbench::compile(
            &[
                include_str!("../../instances/neorv32/neorv32_verilog_wrapper.v"),
                include_str!("../../instances/neorv32/tb.v"),
            ],
            [],
            "rstn",
            ["memory", "primes"],
            &self.description(params),
            |design, root| {
                let mut get = |name: &str| signal(design, root, name);
                (
                    get("trap_q"),
                    get("if_q"),
                    get("if_pc_q"),
                    get("pc_q"),
                    ["is_q", "ex_q", "alu_q", "br_q", "ma_q"].map(get),
                )
            },
        )?;
        let (trap, if_q, if_pc_q, pc_q, stages) = handles;

        let design = Neorv32Design {
            trap: tb.resolve(trap),
            if_q: tb.resolve(if_q),
            if_pc_q: tb.resolve(if_pc_q),
            pc_q: tb.resolve(pc_q),
            stages: stages.map(|h| tb.resolve(h)),
            tb,
        };
        Ok((design, sim))
    }

    fn prologue(
        &self,
        design: &Neorv32Design,
        state: &mut Sim,
        sections: &ProgramSections,
    ) -> Result<(), TraceError> {
        design.tb.reset(state, sections)
    }

    fn iterate_cycle(
        &self,
        design: &Neorv32Design,
        state: &mut Sim,
        pt: &mut PipelineTrace,
    ) -> Result<(), TraceError> {
        let tb = &design.tb;
        tb.step(state)?;

        pt.push(0, tb.stage_pc(state, design.if_pc_q, design.if_q));
        for (i, &active) in design.stages.iter().enumerate() {
            pt.push(i + 1, tb.stage_pc(state, design.pc_q, active));
        }
        Ok(())
    }

    fn is_trapped(&self, design: &Neorv32Design, state: &Sim) -> bool {
        design.tb.is_one(state, design.trap)
    }
}
