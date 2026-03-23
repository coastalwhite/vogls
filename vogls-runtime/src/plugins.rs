use crate::{RtSignalKey, RuntimeState};

pub type RuntimePluginState = Box<dyn RuntimePlugin>;
pub trait RuntimePlugin: std::any::Any + Send + Sync + 'static {
    fn clone(&self) -> RuntimePluginState;
    fn poke_signal(&mut self, signal: RtSignalKey);
    fn timestep(&mut self, state: &mut RuntimeState);
    fn finish(&mut self, state: &mut RuntimeState);
}
