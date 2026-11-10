use vogls::design::RtSignal;

use super::{
    Core, DATA_MEMORY, Description, INSTRUCTION_MEMORY, PipelineTrace, Sim, Testbench, enabled,
    memory_map, signal,
};
use crate::TraceError;
use crate::program::{MemoryMap, ProgramSections};

const BOOT: u32 = 0x0000_0000;

#[derive(Debug, Clone, Default)]
pub struct Hazard3Config {
    pub extension_m: bool,
    pub mul_fast: bool,
    pub mulh_fast: bool,
    pub muldiv_unroll_2: bool,
    pub reduced_bypass: bool,
    pub branch_predictor: bool,
    pub fast_branchcmp: bool,
}

pub struct Hazard3;

pub struct Hazard3Design {
    tb: Testbench,
    trap: RtSignal,
    /// The `(pc, active)` signals of F, X and M.
    stages: [(RtSignal, RtSignal); 3],
}

impl Core for Hazard3 {
    type Design = Hazard3Design;
    type State = Sim;
    type Params = Hazard3Config;

    fn memory_map(&self) -> MemoryMap {
        memory_map(BOOT)
    }

    fn description(&self, _params: &Hazard3Config) -> Description {
        // Fetch, decode/execute, and memory/writeback.
        Description {
            pipeline_stages: vec!["F", "X", "M"],
            imem_size: INSTRUCTION_MEMORY,
            dmem_size: DATA_MEMORY,
        }
    }

    fn compile(&self, params: &Hazard3Config) -> Result<(Hazard3Design, Sim), TraceError> {
        let macros = enabled([
            (params.extension_m, "EXTENSION_M"),
            (params.mul_fast, "MUL_FAST"),
            (params.mulh_fast, "MULH_FAST"),
            (params.muldiv_unroll_2, "MULDIV_UNROLL_2"),
            (params.reduced_bypass, "REDUCED_BYPASS"),
            (params.branch_predictor, "BRANCH_PREDICTOR"),
            (params.fast_branchcmp, "FAST_BRANCHCMP"),
        ]);
        let (tb, sim, (trap, stages)) = Testbench::compile(
            &[
                include_str!("../../instances/hazard3/hazard3.v"),
                include_str!("../../instances/hazard3/tb.v"),
            ],
            macros,
            "rst_n",
            ["memory", "data_mem"],
            &self.description(params),
            // The testbench exposes the pipeline state of all three stages at
            // its own scope, so nothing here has to reach into the core
            // hierarchy.
            |design, root| {
                let mut get = |name: &str| signal(design, root, name);
                (
                    get("trap_q"),
                    [
                        (get("f_pc"), get("f_active")),
                        (get("x_pc"), get("x_active")),
                        (get("m_pc"), get("m_active")),
                    ],
                )
            },
        )?;

        let design = Hazard3Design {
            trap: tb.resolve(trap),
            stages: stages.map(|(pc, active)| (tb.resolve(pc), tb.resolve(active))),
            tb,
        };
        Ok((design, sim))
    }

    fn prologue(
        &self,
        design: &Hazard3Design,
        state: &mut Sim,
        sections: &ProgramSections,
    ) -> Result<(), TraceError> {
        design.tb.reset(state, &self.memory_map(), sections)
    }

    fn iterate_cycle(
        &self,
        design: &Hazard3Design,
        state: &mut Sim,
        pt: &mut PipelineTrace,
    ) -> Result<(), TraceError> {
        let tb = &design.tb;
        tb.step(state)?;

        for (i, &(pc, active)) in design.stages.iter().enumerate() {
            pt.push(i, tb.stage_pc(state, pc, active));
        }
        Ok(())
    }

    fn is_trapped(&self, design: &Hazard3Design, state: &Sim) -> bool {
        design.tb.is_one(state, design.trap)
    }
}
