use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{Bits, ConnectionDirection, GlobalContext, SCALAR_VSIZE, VectorSize, new_process};
use vogls_utils::OrderedSet;

use crate::ast::module::{
    Dimension, GateInstantiation, GenerateBlock, ListOfPortConnections, ModuleInstance,
    ModuleInstantiation, ModuleOrGenerateItem, ModuleOrGenerateItemContent,
    ModuleOrGenerateItemDeclaration, NInputGateInstance, NInputGateType, NamedPortConnection,
    NetDeclAssignment, NetDeclarationNets, VariableType, VariableTypeVariant,
};
use crate::ast::udp::{UdpInstance, UdpInstantiation};
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::{VSymbol, VSymbolTable};
use crate::lower::assign::{assign_net_lvalue, net_lvalue_width};
use crate::lower::expression::{self, get_used_signals, lower_expr, truncate_or_extend};
use crate::lower::fuse::try_fuse_assign;
use crate::lower::statement::statements_to_process;
use crate::lower::udp::lower_udp;
use crate::lower::{
    VType, assign_input_port, assign_port_output, eval_constant_expr, evaluate_range,
    resolve_symbol_id, try_resolve_net, unwrap_get_module,
};
use crate::parser::AstArenas;

use super::LowerContext;
use super::{Diagnostics, MutLowerContext};

pub mod function;

pub fn lower<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    id: AstId<'a, ModuleOrGenerateItem<'a>>,
) -> Result<(), ()> {
    match id.content {
        ModuleOrGenerateItemContent::ModuleOrGenerateItemDeclaration(id) => {
            let module_or_generate_item_declaration = &*id;
            match module_or_generate_item_declaration {
                ModuleOrGenerateItemDeclaration::Net(id) => {
                    let net_declaration = &**id;
                    let (_, _, width) = match net_declaration.range {
                        None => (0, 0, SCALAR_VSIZE),
                        Some(range) => evaluate_range(
                            &mut mctx.gl,
                            &ctx.arenas,
                            &ctx.table,
                            scope,
                            &mut mctx.diagnostics,
                            range,
                        )?,
                    };
                    let ty = VType::net(width, net_declaration.signed);
                    match net_declaration.nets {
                        NetDeclarationNets::Idents(_) => {}
                        NetDeclarationNets::Assignments(assignments) => {
                            for assignment in assignments.iter() {
                                let NetDeclAssignment {
                                    ident: ast_ident,
                                    expr,
                                } = &*assignment;
                                let net = try_resolve_net(
                                    scope,
                                    &ctx.table,
                                    &ctx.arenas,
                                    *ast_ident,
                                    &mut mctx.diagnostics,
                                )?;

                                let (_, mut bb_builder) = new_process(
                                    &mut mctx.gl,
                                    "decl_assign".into(),
                                    ctx.arenas.get_span(*expr),
                                );
                                let bb_key = bb_builder.key();
                                let (v, v_ty) =
                                    lower_expr(ctx, mctx, scope, &mut bb_builder, *expr)?;
                                let v = expression::sign_or_zero_extend(
                                    &mut mctx.gl,
                                    &mut bb_builder,
                                    v,
                                    v_ty,
                                    ty.force_net_width(),
                                );
                                net.net.drive_blocking(mctx.gl(), &mut bb_builder, v, None);
                                let mut ins = OrderedSet::new();
                                get_used_signals(ctx, mctx, scope, &mut ins, *expr)?;
                                bb_builder.watch_to(mctx.gl(), ins.items, bb_key);
                            }
                        }
                    }
                }
                ModuleOrGenerateItemDeclaration::Reg(_) => {}
                ModuleOrGenerateItemDeclaration::Integer(_) => {}
                ModuleOrGenerateItemDeclaration::Genvar(_) => {}
                ModuleOrGenerateItemDeclaration::Task(_) => {}
                ModuleOrGenerateItemDeclaration::Function(_) => {}
            }
        }
        ModuleOrGenerateItemContent::LocalParameterDeclaration(_) => {}
        ModuleOrGenerateItemContent::ParameterOverride => todo!(),
        ModuleOrGenerateItemContent::ContinuousAssign(id) => {
            let assign = &*id;
            for ast_net_assignment in assign.list_of_net_assignments {
                let net_assignment = &*ast_net_assignment;

                if try_fuse_assign(ctx, mctx, scope, ast_net_assignment)? {
                    continue;
                }

                let (_, mut bb_builder) =
                    new_process(mctx.gl(), "assign".into(), ctx.arenas.get_span(id));
                let bb_key = bb_builder.key();
                let (variable, variable_ty) =
                    lower_expr(ctx, mctx, scope, &mut bb_builder, net_assignment.expression)?;

                assign_net_lvalue(
                    ctx,
                    mctx,
                    scope,
                    &mut bb_builder,
                    net_assignment.net_lvalue,
                    variable,
                    variable_ty,
                )?;

                let mut ins = OrderedSet::new();
                get_used_signals(ctx, mctx, scope, &mut ins, net_assignment.expression)?;
                bb_builder.watch_to(mctx.gl(), ins.items, bb_key);
            }
        }
        ModuleOrGenerateItemContent::GateInstantiation(id) => {
            let gate_instantiation = &*id;
            match gate_instantiation {
                GateInstantiation::NInput(id) => {
                    let ninput_gate_instantiation = &**id;
                    for instance in ninput_gate_instantiation.instances.iter() {
                        let NInputGateInstance {
                            name: _,
                            output_terminal,
                            input_terminals,
                        } = &*instance;

                        let (_, mut bb_builder) =
                            new_process(mctx.gl(), "gate".into(), ctx.arenas.get_span(*id));
                        let bb_key = bb_builder.key();

                        let output_size = net_lvalue_width(ctx, mctx, scope, *output_terminal)?;

                        let mut ins = OrderedSet::new();
                        assert!(!input_terminals.is_empty());
                        let value = input_terminals.first().unwrap();
                        get_used_signals(ctx, mctx, scope, &mut ins, value)?;
                        let (value, value_ty) =
                            lower_expr(ctx, mctx, scope, &mut bb_builder, value)?;
                        let mut value = truncate_or_extend(
                            mctx.gl(),
                            &mut bb_builder,
                            value,
                            value_ty,
                            output_size,
                        );
                        for input in input_terminals.iter().skip(1) {
                            get_used_signals(ctx, mctx, scope, &mut ins, input)?;
                            let (input, input_ty) =
                                lower_expr(ctx, mctx, scope, &mut bb_builder, input)?;
                            let input = truncate_or_extend(
                                mctx.gl(),
                                &mut bb_builder,
                                input,
                                input_ty,
                                output_size,
                            );
                            match ninput_gate_instantiation.gatetype.item {
                                NInputGateType::And | NInputGateType::Nand => {
                                    value = bb_builder.and(mctx.gl(), value, input);
                                }
                                NInputGateType::Or | NInputGateType::Nor => {
                                    value = bb_builder.or(mctx.gl(), value, input);
                                }
                                NInputGateType::Xor | NInputGateType::Xnor => {
                                    value = bb_builder.xor(mctx.gl(), value, input);
                                }
                            }
                        }

                        if matches!(
                            ninput_gate_instantiation.gatetype.item,
                            NInputGateType::Nand | NInputGateType::Nor | NInputGateType::Xnor
                        ) {
                            value = bb_builder.binary_neg(mctx.gl(), value);
                        }

                        assign_net_lvalue(
                            ctx,
                            mctx,
                            scope,
                            &mut bb_builder,
                            *output_terminal,
                            value,
                            VType::UnsignedNet(output_size),
                        )?;

                        bb_builder.watch_to(mctx.gl(), ins.items, bb_key);
                    }
                }
            }
        }
        ModuleOrGenerateItemContent::UdpInstantiation(id) => {
            let UdpInstantiation {
                identifier,
                instances,
            } = &*id;

            let Some(udp) = ctx.udps.get(&identifier.item.0) else {
                mctx.diagnostics.udp_not_found(&ctx.arenas, *identifier);
                return Err(());
            };

            for instance in instances.iter() {
                let UdpInstance {
                    name: _,
                    output_terminal,
                    input_terminals,
                } = &*instance;

                lower_udp(ctx, mctx, scope, *udp, *output_terminal, *input_terminals)?;
            }
        }
        ModuleOrGenerateItemContent::ModuleInstantiation(id) => {
            let ModuleInstantiation {
                module_instances, ..
            } = &*id;

            for instance in module_instances.iter() {
                let ModuleInstance {
                    name_of_module_instance,
                    list_of_port_connections,
                } = &*instance;

                let instance_sid =
                    resolve_symbol_id(scope, &ctx.table, name_of_module_instance.item.0).unwrap();

                match &**list_of_port_connections {
                    ListOfPortConnections::Ordered(ports) => {
                        // @TODO:
                        // Icarus Verilog has as good perspective on how to deal with unequal port
                        // lengths. We should always warn here, but allow less ports.
                        //
                        // https://steveicarus.github.io/iverilog/developer/guide/misc/ieee1364-notes.html.
                        if unwrap_get_module(&ctx.table, instance_sid).ports.len() != ports.len() {
                            mctx.diagnostics.not_yet_implemented(
                                ctx.arenas.get_range_span(*ports),
                                "unequal number of ports",
                            );
                            return Err(());
                        }

                        for (pi, l_p) in ports.iter().enumerate() {
                            let (net, connection) =
                                unwrap_get_module(&ctx.table, instance_sid).ports[pi];
                            let VSymbol::Net(n) = &ctx.table[net].content else {
                                unreachable!();
                            };
                            let ty = n.ty;
                            let is_input = matches!(
                                connection,
                                ConnectionDirection::In | ConnectionDirection::Both
                            );
                            if is_input {
                                assign_input_port(ctx, mctx, scope, l_p, net)?;
                            } else {
                                assign_port_output(ctx, mctx, scope, l_p, net, ty)?;
                            }
                        }
                    }
                    ListOfPortConnections::Named(ports) => {
                        let mut error = false;
                        let mut signals_assigned =
                            vec![false; unwrap_get_module(&ctx.table, instance_sid).ports.len()];
                        for p in ports.iter() {
                            let named_port_connection = &*p;
                            let NamedPortConnection {
                                port_identifier: ast_port_identifier,
                                expression,
                            } = *named_port_connection;

                            let port = ctx
                                .table
                                .resolve(instance_sid, ast_port_identifier.item.0)
                                .and_then(|symid| {
                                    let VSymbol::Net(n) = &ctx.table[symid].content else {
                                        return None;
                                    };
                                    n.port_idx.map(|i| (i, &n.ty))
                                });

                            let Some((port_idx, port_ty)) = port else {
                                mctx.diagnostics.port_not_found(
                                    &ctx.arenas,
                                    unwrap_get_module(&ctx.table, instance_sid),
                                    ast_port_identifier,
                                );
                                error = true;
                                continue;
                            };

                            let (net, connection) =
                                unwrap_get_module(&ctx.table, instance_sid).ports[port_idx];

                            let is_input = matches!(
                                connection,
                                ConnectionDirection::In | ConnectionDirection::Both
                            );

                            if let Some(e) = expression {
                                if is_input {
                                    assign_input_port(ctx, mctx, scope, e, net)?;
                                } else {
                                    assign_port_output(ctx, mctx, scope, e, net, *port_ty)?;
                                }
                            }

                            if std::mem::replace(&mut signals_assigned[port_idx], true) {
                                mctx.diagnostics
                                    .duplicate_definition(&ctx.arenas, ast_port_identifier);
                                error = true;
                                continue;
                            }
                        }

                        if error {
                            return Err(());
                        }
                    }
                };
            }
        }
        ModuleOrGenerateItemContent::InitialConstruct(id) => {
            let statement = id.0;
            let (_, bb_builder) = new_process(mctx.gl(), "initial".into(), ctx.arenas.get_span(id));
            let bb_builder =
                statements_to_process(ctx, mctx, scope, bb_builder, AstIdRange::single(statement))?;
            bb_builder.halt(mctx.gl());
        }
        ModuleOrGenerateItemContent::AlwaysConstruct(id) => {
            let statement = id.0;
            let (_, bb_builder) = new_process(mctx.gl(), "always".into(), ctx.arenas.get_span(id));
            let bb_key = bb_builder.key();
            let bb_builder =
                statements_to_process(ctx, mctx, scope, bb_builder, AstIdRange::single(statement))?;
            bb_builder.jump_to(mctx.gl(), bb_key);
        }

        // Handled by a combination of elaboration + module level elaboration.
        ModuleOrGenerateItemContent::LoopGenerateConstruct(_)
        | ModuleOrGenerateItemContent::IfGenerateConstruct(_)
        | ModuleOrGenerateItemContent::CaseGenerateConstruct(_) => {}
    }

    Ok(())
}

pub fn dims_to_array<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    table: &VSymbolTable,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    dimensions: AstIdRange<'a, Dimension<'a>>,
) -> Result<Vec<u32>, ()> {
    let mut dims = Vec::with_capacity(dimensions.len());
    for dim in dimensions.iter().rev() {
        let Dimension { lhs, rhs } = &*dim;
        let lhs = eval_constant_expr(gl, arenas, table, scope, diagnostics, *lhs);
        let rhs = eval_constant_expr(gl, arenas, table, scope, diagnostics, *rhs);

        let lhs = lhs?.into_bits().as_i64().unwrap();
        let rhs = rhs?.into_bits().as_i64().unwrap();

        dims.push((lhs.abs_diff(rhs) + 1) as u32);
    }
    Ok(dims)
}

pub fn lower_opt_generate_block<'a>(
    ctx: &LowerContext<'a>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    opt_generate_block: AstId<'a, Option<GenerateBlock<'a>>>,
) -> Result<(), ()> {
    match &*opt_generate_block {
        None => Ok(()),
        Some(GenerateBlock::BeginEnd(_, module_or_generate_items)) => {
            for m in module_or_generate_items.iter() {
                lower(ctx, mctx, scope, m)?;
            }
            Ok(())
        }
        Some(GenerateBlock::ModuleOrGenerateItem(module_or_generate_item)) => {
            lower(ctx, mctx, scope, *module_or_generate_item)
        }
    }
}

pub fn lower_variable_type<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    table: &VSymbolTable,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    variable_type: AstId<'a, VariableType<'a>>,
    ty: VType,
) -> Result<(Vec<u32>, VectorSize, Option<Bits>), ()> {
    match variable_type.variant {
        VariableTypeVariant::Dimensions(dimensions) => {
            let dims = dims_to_array(gl, arenas, table, scope, diagnostics, dimensions)?;
            let mut size = ty.force_net_width().get();
            for dim in &dims {
                size = size.checked_mul(*dim).ok_or_else(|| {
                    diagnostics.net_width_overflow(arenas.get_span(variable_type));
                    ()
                })?;
            }
            let Some(size) = VectorSize::new(size) else {
                diagnostics.zero_width_net(arenas.get_span(variable_type));
                return Err(());
            };

            Ok((dims, size, None))
        }
        VariableTypeVariant::ConstantExpr(expr) => {
            let value = eval_constant_expr(gl, arenas, table, scope, diagnostics, expr)?;
            let value = value.truncate_or_extend(ty.force_net_width());
            Ok((Vec::new(), ty.force_net_width(), Some(value.into_bits())))
        }
    }
}
