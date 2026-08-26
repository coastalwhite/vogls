use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{ProcessBuilder, ProcessKind};
use vogls_utils::OrderedSet;

use crate::ast::AstId;
use crate::ast::module::{
    GateInstantiation, NInputGateInstance, NInputGateType, NOutputGateInstance, NOutputGateType,
};
use crate::lower::assign::{assign_net_lvalue, net_lvalue_bit_length};
use crate::lower::expression::{get_used_signals, lower_expr, truncate_or_extend};
use crate::lower::{LowerContext, MutLowerContext, VType};

pub fn lower<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, GateInstantiation<'a>>,
) -> Result<(), ()> {
    use GateInstantiation as G;
    match &*id {
        G::Enable(_)
        | G::Mos(_)
        | G::Cmos(_)
        | G::PassEn(_)
        | G::PassSwitch(_)
        | G::Pullup(_)
        | G::Pulldown(_) => {
            mctx.diagnostics.not_yet_implemented(
                ctx.arenas.get_span(id),
                "gate instantiation type not yet implemented",
            );
            return Err(());
        }
        G::NInput(id) => {
            use NInputGateType as G;
            for instance in id.instances.iter() {
                let NInputGateInstance {
                    name: _,
                    output_terminal,
                    input_terminals,
                } = &*instance;

                assert!(!input_terminals.is_empty());

                let (proc_builder, mut bb_builder) =
                    ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(*id));
                let entry_tr = proc_builder.entry();

                let output_bit_length = net_lvalue_bit_length(ctx, mctx, scope, *output_terminal)?;

                let mut sensitivity_list = OrderedSet::new();
                let input = input_terminals.first().unwrap();

                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, input)?;
                let (value, value_ty) = lower_expr(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    input,
                    Some(output_bit_length),
                )?;
                let mut value = truncate_or_extend(
                    mctx.gl(),
                    &mut bb_builder,
                    value,
                    value_ty,
                    output_bit_length,
                );
                for input in input_terminals.iter().skip(1) {
                    get_used_signals(ctx, mctx, scope, &mut sensitivity_list, input)?;
                    let (input, input_ty) = lower_expr(
                        ctx,
                        mctx,
                        scope,
                        &mut bb_builder,
                        input,
                        Some(output_bit_length),
                    )?;
                    let input = truncate_or_extend(
                        mctx.gl(),
                        &mut bb_builder,
                        input,
                        input_ty,
                        output_bit_length,
                    );

                    match id.gatetype.item {
                        G::And | G::Nand => value = bb_builder.and(mctx.gl(), value, input),
                        G::Or | G::Nor => value = bb_builder.or(mctx.gl(), value, input),
                        G::Xor | G::Xnor => value = bb_builder.xor(mctx.gl(), value, input),
                    }
                }

                if matches!(id.gatetype.item, G::Nand | G::Nor | G::Xnor) {
                    value = bb_builder.binary_not(mctx.gl(), value);
                }

                assign_net_lvalue(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    *output_terminal,
                    value,
                    VType::UnsignedNet(output_bit_length),
                )?;

                let sensitivity_list = sensitivity_list.items;
                bb_builder.watch_to(mctx.gl(), sensitivity_list, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
        G::NOutput(id) => {
            use NOutputGateType as G;
            for instance in id.instances.iter() {
                let NOutputGateInstance {
                    name: _,
                    output_terminals,
                    input_terminal,
                } = &*instance;

                let (proc_builder, mut bb_builder) =
                    ProcessBuilder::new(mctx.gl(), ProcessKind::Udp, ctx.arenas.get_span(*id));
                let entry_tr = proc_builder.entry();

                let mut sensitivity_list = OrderedSet::new();
                get_used_signals(ctx, mctx, scope, &mut sensitivity_list, *input_terminal)?;
                let sensitivity_list = sensitivity_list.items;

                for output_terminal in output_terminals.iter() {
                    let output_bit_length =
                        net_lvalue_bit_length(ctx, mctx, scope, output_terminal)?;
                    let (value, value_ty) = lower_expr(
                        ctx,
                        mctx,
                        scope,
                        &mut bb_builder,
                        *input_terminal,
                        Some(output_bit_length),
                    )?;
                    let mut value = truncate_or_extend(
                        mctx.gl(),
                        &mut bb_builder,
                        value,
                        value_ty,
                        output_bit_length,
                    );
                    match id.gatetype.item {
                        G::Buf => value = bb_builder.x_to_z(mctx.gl(), value),
                        G::Not => value = bb_builder.binary_not(mctx.gl(), value),
                    }

                    assign_net_lvalue(
                        ctx,
                        mctx,
                        scope,
                        &mut bb_builder,
                        output_terminal,
                        value,
                        VType::UnsignedNet(output_bit_length),
                    )?;
                }

                bb_builder.watch_to(mctx.gl(), sensitivity_list, entry_tr);
                proc_builder.finalize(mctx.gl());
            }
        }
    }

    Ok(())
}
