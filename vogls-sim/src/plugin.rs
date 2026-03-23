use crate::{Simulation, SimulationState, VmInstruction};

pub type InstructionPluginState = Box<dyn InstructionPlugin + Send + Sync>;

pub trait InstructionPlugin: std::any::Any {
    fn instruction(
        &mut self,
        simulation: &Simulation,
        state: &mut SimulationState,
        instruction: &VmInstruction,
    );
}
