use crate::{Simulation, SimulationState, VmInstruction, VmSignalKey};

pub type PluginState = Box<dyn Plugin + Send + Sync>;

pub trait Plugin: std::any::Any {
    fn update_signal(
        &mut self,
        simulation: &Simulation,
        state: &mut SimulationState,
        signal: VmSignalKey,
    );
    fn timestep(&mut self, simulation: &Simulation, state: &mut SimulationState);
    fn finish(&mut self, simulation: &Simulation, state: &mut SimulationState);
}

pub type InstructionPluginState = Box<dyn InstructionPlugin + Send + Sync>;

pub trait InstructionPlugin: std::any::Any {
    fn instruction(
        &mut self,
        simulation: &Simulation,
        state: &mut SimulationState,
        instruction: &VmInstruction,
    );
}
