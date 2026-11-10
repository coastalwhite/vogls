use vogls::design::RtSignal;

use super::{
    Core, DATA_MEMORY, Description, INSTRUCTION_MEMORY, PipelineTrace, Sim, Testbench, enabled,
    lookup, memory_map, signal,
};
use crate::TraceError;
use crate::program::{MemoryMap, ProgramSections};

/// Ibex fetches its first instruction from `boot_addr_i + 0x80`.
const BOOT: u32 = 0x0000_0080;

#[derive(Debug, Clone, Default)]
pub struct IbexConfig {
    pub wb_stage: bool,
}

pub struct Ibex;

/// The program counter of a stage and the flag saying it holds an instruction.
struct Stage {
    pc: RtSignal,
    valid: RtSignal,
}

pub struct IbexDesign {
    tb: Testbench,
    trap: RtSignal,
    /// IF, ID/EX, and WB when the core has a writeback stage.
    stages: Vec<Stage>,
}

impl Core for Ibex {
    type Design = IbexDesign;
    type State = Sim;
    type Params = IbexConfig;

    fn memory_map(&self) -> MemoryMap {
        memory_map(BOOT)
    }

    fn description(&self, params: &IbexConfig) -> Description {
        let mut pipeline_stages = vec!["IF", "ID"];
        if params.wb_stage {
            pipeline_stages.push("WB");
        }
        Description {
            pipeline_stages,
            imem_size: INSTRUCTION_MEMORY,
            dmem_size: DATA_MEMORY,
        }
    }

    fn compile(&self, params: &IbexConfig) -> Result<(IbexDesign, Sim), TraceError> {
        let (tb, sim, (trap, stages)) = Testbench::compile(
            &[
                include_str!("../../instances/ibex/ibex_fixed.v"),
                include_str!("../../instances/ibex/tb.v"),
            ],
            enabled([(params.wb_stage, "WRITEBACK_STAGE")]),
            "rst_n",
            ["memory", "primes"],
            &self.description(params),
            |design, root| {
                // Navigate: tb -> u_top (ibex_top) -> u_ibex_core (ibex_core)
                let u_top = lookup(design, root, "u_top");
                let core = lookup(design, u_top, "u_ibex_core");
                let mut get = |name: &str| signal(design, core, name);

                let mut stages = vec![
                    // IF: pc_if is the current fetch PC; it is valid when
                    // instr_req_int is high.
                    (get("pc_if"), get("instr_req_int")),
                    // ID/EX: pc_id is the PC of the instruction in
                    // decode/execute.
                    (get("pc_id"), get("instr_valid_id")),
                ];
                if params.wb_stage {
                    // WB: pc_wb + instr_done_wb are the proper signals with
                    // WritebackStage=1.
                    stages.push((get("pc_wb"), get("instr_done_wb")));
                }
                (get("csr_save_cause"), stages)
            },
        )?;

        let design = IbexDesign {
            trap: tb.resolve(trap),
            stages: stages
                .into_iter()
                .map(|(pc, valid)| Stage {
                    pc: tb.resolve(pc),
                    valid: tb.resolve(valid),
                })
                .collect(),
            tb,
        };
        Ok((design, sim))
    }

    fn prologue(
        &self,
        design: &IbexDesign,
        state: &mut Sim,
        sections: &ProgramSections,
    ) -> Result<(), TraceError> {
        design.tb.reset(state, &self.memory_map(), sections)
    }

    fn iterate_cycle(
        &self,
        design: &IbexDesign,
        state: &mut Sim,
        pt: &mut PipelineTrace,
    ) -> Result<(), TraceError> {
        let tb = &design.tb;
        tb.step(state)?;

        for (i, stage) in design.stages.iter().enumerate() {
            pt.push(i, tb.stage_pc(state, stage.pc, stage.valid));
        }
        Ok(())
    }

    fn is_trapped(&self, design: &IbexDesign, state: &Sim) -> bool {
        design.tb.is_one(state, design.trap)
    }
}
