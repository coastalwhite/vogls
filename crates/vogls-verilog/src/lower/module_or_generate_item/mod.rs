use vogls_frontend::symbol_table::SymbolId;

use crate::ast::AstId;
use crate::ast::module::{ModuleOrGenerateItem, ModuleOrGenerateItemContent};

use super::LowerContext;
use super::MutLowerContext;

mod always;
mod continuous_assign;
pub mod function;
mod gate_instantiation;
mod initial;
mod module_instantiation;
mod net_assignment;
mod udp;

pub(crate) use continuous_assign::lower_assign_delay3;
pub(crate) use gate_instantiation::{
    AssignOutputFn, GateDelayFn, lower_delay_process, lower_delay2, lower_delay3,
    lower_gate_primitive,
};

/// Lower a Verilog module or generate item to Vogls IR.
pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    id: AstId<'a, ModuleOrGenerateItem<'a>>,
) -> Result<(), ()> {
    use ModuleOrGenerateItemContent as I;
    match id.content {
        I::ModuleOrGenerateItemDeclaration(item_decl) => {
            net_assignment::lower(ctx, mctx, scope, item_decl)?
        }
        I::ContinuousAssign(id) => continuous_assign::lower(ctx, mctx, scope, id)?,
        I::GateInstantiation(id) => gate_instantiation::lower(ctx, mctx, scope, id)?,
        I::UdpInstantiation(id) => udp::lower(ctx, mctx, scope, id)?,
        I::ModuleInstantiation(id) => module_instantiation::lower(ctx, mctx, scope, id)?,
        I::InitialConstruct(id) => initial::lower(ctx, mctx, scope, id)?,
        I::AlwaysConstruct(id) => always::lower(ctx, mctx, scope, id)?,

        I::ParameterOverride => todo!(),

        // Fully handled during elaboration.
        I::LocalParameterDeclaration(_) => {}

        // Handled by a combination of elaboration + module level elaboration.
        I::LoopGenerateConstruct(_) | I::IfGenerateConstruct(_) | I::CaseGenerateConstruct(_) => {}
    }

    Ok(())
}
