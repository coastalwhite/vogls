use crate::RuntimeState;

pub type RuntimePluginState = Box<dyn RuntimePlugin>;
pub trait RuntimePlugin: std::any::Any + Send + Sync + 'static {
    fn clone(&self) -> RuntimePluginState;
    fn timestep(&mut self, state: &mut RuntimeState);
    fn finish(&mut self, state: &mut RuntimeState);
}
