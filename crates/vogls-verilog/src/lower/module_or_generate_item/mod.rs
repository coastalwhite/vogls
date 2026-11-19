use vogls_frontend::symbol_table::SymbolId;

use crate::ast::AstId;
use crate::ast::module::{ModuleOrGenerateItem, ModuleOrGenerateItemContent};
use crate::lower::observed::Scan;

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

/// Lower a Verilog module or generate item into the drive graph.
pub fn lower_structural<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    scan: &mut Scan,
    id: AstId<'a, ModuleOrGenerateItem<'a>>,
) -> Result<(), ()> {
    use ModuleOrGenerateItemContent as I;
    let mut scan_stmt = None;
    match id.content {
        I::ModuleOrGenerateItemDeclaration(item_decl) => {
            net_assignment::lower(ctx, mctx, scope, item_decl)?
        }
        I::ContinuousAssign(id) => continuous_assign::lower(ctx, mctx, scope, id)?,
        I::GateInstantiation(id) => gate_instantiation::lower(ctx, mctx, scope, id)?,
        I::UdpInstantiation(id) => udp::lower(ctx, mctx, scope, id)?,
        I::ModuleInstantiation(id) => module_instantiation::lower(ctx, mctx, scope, id)?,
        I::InitialConstruct(id) => scan_stmt = Some(id.0),
        I::AlwaysConstruct(id) => scan_stmt = Some(id.0),

        I::ParameterOverride => todo!(),

        // Fully handled during elaboration.
        I::LocalParameterDeclaration(_) => {}

        // Handled by a combination of elaboration + module level elaboration.
        I::LoopGenerateConstruct(_) | I::IfGenerateConstruct(_) | I::CaseGenerateConstruct(_) => {}
    };

    if let Some(stmt) = scan_stmt {
        scan.clear();

        let diagnostics = std::mem::take(&mut mctx.diagnostics);
        scan.stmt(ctx, mctx, scope, stmt);
        mctx.diagnostics = diagnostics;

        for &signal in &scan.watched.items {
            mctx.driver_resolution.observe_watch(signal);
        }
        for &signal in &scan.lupdt.items {
            mctx.driver_resolution.observe_lupdt(signal);
        }
    }

    Ok(())
}

/// Lower a Verilog module or generate item to Vogls IR.
pub fn lower_procedural<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext<'a>,
    scope: SymbolId,
    id: AstId<'a, ModuleOrGenerateItem<'a>>,
) -> Result<(), ()> {
    use ModuleOrGenerateItemContent as I;
    match id.content {
        I::InitialConstruct(id) => initial::lower(ctx, mctx, scope, id)?,
        I::AlwaysConstruct(id) => always::lower(ctx, mctx, scope, id)?,

        I::ParameterOverride => todo!(),

        // Handled during structural phase.
        I::ModuleOrGenerateItemDeclaration(_) => {}
        I::ContinuousAssign(_) => {}
        I::GateInstantiation(_) => {}
        I::UdpInstantiation(_) => {}
        I::ModuleInstantiation(_) => {}

        // Fully handled during elaboration.
        I::LocalParameterDeclaration(_) => {}

        // Handled by a combination of elaboration + module level elaboration.
        I::LoopGenerateConstruct(_) | I::IfGenerateConstruct(_) | I::CaseGenerateConstruct(_) => {}
    }

    Ok(())
}
