use vogls_runtime::plugins::RuntimePlugin;
use vogls_verilog::elaborate::VSymbolTable;

use crate::design::Design;
use crate::elaborated_design::ElaboratedDesign;

pub trait VoglsPlugin: RuntimePlugin {
    fn clone(&self) -> Box<dyn VoglsPlugin>;
    fn register_handles(&mut self, design: &mut ElaboratedDesign<'_>, table: &VSymbolTable);
    fn finalize(&mut self, design: &Design);
}
