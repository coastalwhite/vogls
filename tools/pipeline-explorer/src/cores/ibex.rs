use vogls::design::RtSignal;

use super::{
    Core, DATA_MEMORY, DATA_MEMORY_ADDR, DataBuilder, DataPoints, Description, INSTRUCTION_MEMORY,
    INSTRUCTION_MEMORY_ADDR, REGISTER_NAMES, Sim, Testbench, enabled, lookup, signal, try_signal,
};
use crate::TraceError;
use crate::program::ProgramSections;

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
    /// The flat register file, when it was asked for and the design has it.
    rf_reg: Option<RtSignal>,
}

impl Core for Ibex {
    type Design = IbexDesign;
    type State = Sim;
    type Params = IbexConfig;

    fn description(&self, params: &IbexConfig) -> Description {
        let mut pipeline_stages = vec!["IF", "ID"];
        if params.wb_stage {
            pipeline_stages.push("WB");
        }
        Description {
            pipeline_stages,
            boot: BOOT,
            imem_addr: INSTRUCTION_MEMORY_ADDR,
            imem_size: INSTRUCTION_MEMORY,
            dmem_addr: DATA_MEMORY_ADDR,
            dmem_size: DATA_MEMORY,
        }
    }

    fn compile(
        &self,
        params: &IbexConfig,
        points: DataPoints,
    ) -> Result<(IbexDesign, Sim), TraceError> {
        let want_regs = points.contains(DataPoints::REGISTERS);
        let (tb, sim, (trap, stages, rf_reg)) = Testbench::compile(
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
                let trap = get("csr_save_cause");
                // `rf_reg` is a flat vector inside the register file, which
                // sits behind the generate block that picks its flavour.
                // The register file is instantiated in `ibex_top`, beside
                // the core rather than inside it, behind the generate block
                // that picks its flavour.
                let rf_reg = want_regs
                    .then(|| {
                        try_signal(
                            design,
                            u_top,
                            &["gen_regfile_ff", "register_file_i", "rf_reg"],
                        )
                    })
                    .flatten();
                (trap, stages, rf_reg)
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
            rf_reg: rf_reg.map(|handle| tb.resolve(handle)),
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
        design.tb.reset(state, sections)
    }

    fn iterate_cycle(
        &self,
        design: &IbexDesign,
        state: &mut Sim,
        data: &mut Vec<DataBuilder>,
    ) -> Result<(), TraceError> {
        let tb = &design.tb;
        tb.step(state)?;

        if let Some(pt) = data.iter_mut().find_map(DataBuilder::as_pipeline) {
            for (i, stage) in design.stages.iter().enumerate() {
                pt.push(i, tb.stage_pc(state, stage.pc, stage.valid));
            }
        }
        if let (Some(rt), Some(rf_reg)) = (
            data.iter_mut().find_map(DataBuilder::as_registers),
            design.rf_reg,
        ) {
            // Ibex packs register 0 at the top of the vector.
            rt.push(tb.words(state, rf_reg, REGISTER_NAMES.len(), true));
        }
        Ok(())
    }

    fn is_trapped(&self, design: &IbexDesign, state: &Sim) -> bool {
        design.tb.is_one(state, design.trap)
    }
}
