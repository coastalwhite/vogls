use std::sync::Arc;

use vogls_codegen::HeapRef;
use vogls_runtime::plugins::RuntimePlugin;
use vogls_utils::VgHashMap;

use crate::SignalHandle;
use crate::design::RtSignal;
use crate::elaborated_design::ElaboratedDesign;

pub trait VoglsPlugin: RuntimePlugin {
    fn clone(&self) -> Box<dyn VoglsPlugin>;
    fn register_handles(&mut self, design: &mut ElaboratedDesign<'_>);
    fn finalize(
        &mut self,
        handle_map: &VgHashMap<SignalHandle, RtSignal>,
        signal_to_heap: &Arc<[HeapRef]>,
    );
}
