use crate::{RtSignalKey, RuntimeState};

pub type RuntimePluginState = Box<dyn RuntimePlugin>;
pub trait RuntimePlugin: std::any::Any + Send + Sync + 'static {
    fn clone(&self) -> RuntimePluginState;
    fn timestep(&mut self, state: &mut RuntimeState, txs: &[RtSignalKey]);
    fn finish(&mut self, state: &mut RuntimeState, txs: &[RtSignalKey]);
}
