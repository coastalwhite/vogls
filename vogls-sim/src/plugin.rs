use crate::{Simulation, SimulationState, VmSignalKey};

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
