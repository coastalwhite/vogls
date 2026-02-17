use vogls_ir::{
    Bits, ConnectionDirection, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, VectorSize, new_process,
};

use crate::ast::module::{
    Dimension, GateInstantiation, GenerateBlock, ListOfPortConnections, ModuleInstance,
    ModuleInstantiation, ModuleOrGenerateItem, ModuleOrGenerateItemContent,
    ModuleOrGenerateItemDeclaration, NInputGateInstance, NInputGateType, NamedPortConnection,
    NetDeclAssignment, NetDeclarationNets, VariableType, VariableTypeVariant,
};
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::VSymbol;
use crate::lower::assign::{assign_net_lvalue, net_lvalue_width};
use crate::lower::expression::{self, lower_expr, truncate_or_extend};
use crate::lower::statement::statements_to_process;
use crate::lower::{
    VType, assign_port_output, eval_constant_expr, evaluate_range, lower_to_signal,
    resolve_symbol_id, try_resolve_net, unwrap_get_module, unwrap_get_net, unwrap_get_net_mut,
};
use crate::parser::AstArenas;

use super::Scope;
use super::{Diagnostics, EvalScope};

pub mod function;

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    id: AstId<ModuleOrGenerateItem>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match arenas.get(id).content {
        ModuleOrGenerateItemContent::ModuleOrGenerateItemDeclaration(id) => {
            let module_or_generate_item_declaration = arenas.get(id);
            match module_or_generate_item_declaration {
                ModuleOrGenerateItemDeclaration::Net(id) => {
                    let net_declaration = arenas.get(*id);
                    let (_, _, width) = match net_declaration.range {
                        None => (0, 0, SCALAR_VSIZE),
                        Some(range) => {
                            evaluate_range(gl, arenas, scope.eval(), diagnostics, range)?
                        }
                    };
                    let ty = VType::net(width, net_declaration.signed);
                    match net_declaration.nets {
                        NetDeclarationNets::Idents(_) => {}
                        NetDeclarationNets::Assignments(assignments) => {
                            for assignment in assignments.iter() {
                                let NetDeclAssignment {
                                    ident: ast_ident,
                                    expr,
                                } = arenas.get(assignment);
                                let net = try_resolve_net(
                                    scope.key,
                                    scope.table,
                                    arenas,
                                    *ast_ident,
                                    diagnostics,
                                )?;

                                let mut bb_builder =
                                    new_process(gl, "decl_assign".into(), arenas.get_span(*expr));
                                let bb_key = bb_builder.key();
                                let (v, v_ty) = lower_expr(
                                    gl,
                                    arenas,
                                    scope,
                                    diagnostics,
                                    &mut bb_builder,
                                    *expr,
                                )?;
                                let v = expression::sign_or_zero_extend(
                                    gl,
                                    &mut bb_builder,
                                    v,
                                    v_ty,
                                    ty.force_net_width(),
                                );
                                bb_builder.drive(gl, net.signal, v);
                                bb_builder.watch_for_ins_to(gl, bb_key);
                            }
                        }
                    }
                }
                ModuleOrGenerateItemDeclaration::Reg(_) => {}
                ModuleOrGenerateItemDeclaration::Integer(id) => {
                    let integer_declaration = arenas.get(*id);
                    let ty = VType::SignedNet(INTEGER_VSIZE);
                    for variable_type in integer_declaration.variable_types.iter() {
                        let (_, _, initialize) =
                            lower_variable_type(gl, arenas, scope, diagnostics, variable_type, ty)?;
                        let Some(initialize) = initialize else {
                            continue;
                        };

                        let net = try_resolve_net(
                            scope.key,
                            scope.table,
                            arenas,
                            arenas.get(variable_type).identifier,
                            diagnostics,
                        )?;
                        gl.signals[net.signal].initialize = Some(initialize);
                    }
                }
                ModuleOrGenerateItemDeclaration::Genvar(_) => {}
                ModuleOrGenerateItemDeclaration::Task(_) => {}
                ModuleOrGenerateItemDeclaration::Function(_) => {}
            }
        }
        ModuleOrGenerateItemContent::LocalParameterDeclaration(_) => {}
        ModuleOrGenerateItemContent::ParameterOverride => todo!(),
        ModuleOrGenerateItemContent::ContinuousAssign(id) => {
            let assign = arenas.get(id);
            for ast_net_assignment in assign.list_of_net_assignments {
                let net_assignment = arenas.get(ast_net_assignment);

                let mut bb_builder = new_process(gl, "assign".into(), arenas.get_span(id));
                let bb_key = bb_builder.key();
                let (variable, variable_ty) = lower_expr(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    &mut bb_builder,
                    net_assignment.expression,
                )?;

                assign_net_lvalue(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    &mut bb_builder,
                    net_assignment.net_lvalue,
                    variable,
                    variable_ty,
                )?;

                bb_builder.watch_for_ins_to(gl, bb_key);
            }
        }
        ModuleOrGenerateItemContent::GateInstantiation(id) => {
            let gate_instantiation = arenas.get(id);
            match gate_instantiation {
                GateInstantiation::NInput(id) => {
                    let ninput_gate_instantiation = arenas.get(*id);
                    for instance in ninput_gate_instantiation.instances.iter() {
                        let NInputGateInstance {
                            name: _,
                            output_terminal,
                            input_terminals,
                        } = arenas.get(instance);

                        let mut bb_builder = new_process(gl, "gate".into(), arenas.get_span(*id));
                        let bb_key = bb_builder.key();

                        let output_size =
                            net_lvalue_width(gl, arenas, scope, diagnostics, *output_terminal)?;

                        assert!(!input_terminals.is_empty());
                        let value = input_terminals.first().unwrap();
                        let (value, value_ty) =
                            lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, value)?;
                        let mut value =
                            truncate_or_extend(gl, &mut bb_builder, value, value_ty, output_size);
                        for input in input_terminals.iter().skip(1) {
                            let (input, input_ty) =
                                lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, input)?;
                            let input = truncate_or_extend(
                                gl,
                                &mut bb_builder,
                                input,
                                input_ty,
                                output_size,
                            );
                            match ninput_gate_instantiation.gatetype.item {
                                NInputGateType::And | NInputGateType::Nand => {
                                    value = bb_builder.and(gl, value, input);
                                }
                                NInputGateType::Or | NInputGateType::Nor => {
                                    value = bb_builder.or(gl, value, input);
                                }
                                NInputGateType::Xor | NInputGateType::Xnor => {
                                    value = bb_builder.xor(gl, value, input);
                                }
                            }
                        }

                        if matches!(
                            ninput_gate_instantiation.gatetype.item,
                            NInputGateType::Nand | NInputGateType::Nor | NInputGateType::Xnor
                        ) {
                            value = bb_builder.binary_neg(gl, value);
                        }

                        assign_net_lvalue(
                            gl,
                            arenas,
                            scope,
                            diagnostics,
                            &mut bb_builder,
                            *output_terminal,
                            value,
                            VType::UnsignedNet(output_size),
                        )?;

                        bb_builder.watch_for_ins_to(gl, bb_key);
                    }
                }
            }
        }
        ModuleOrGenerateItemContent::UdpInstantiation => todo!(),
        ModuleOrGenerateItemContent::ModuleInstantiation(id) => {
            let ModuleInstantiation {
                module_instances, ..
            } = arenas.get(id);

            for instance in module_instances.iter() {
                let ModuleInstance {
                    name_of_module_instance,
                    list_of_port_connections,
                } = arenas.get(instance);

                let instance_sid =
                    resolve_symbol_id(scope.key, scope.table, name_of_module_instance.item.0)
                        .unwrap();

                match arenas.get(*list_of_port_connections) {
                    ListOfPortConnections::Ordered(ports) => {
                        // @TODO:
                        // Icarus Verilog has as good perspective on how to deal with unequal port
                        // lengths. We should always warn here, but allow less ports.
                        //
                        // https://steveicarus.github.io/iverilog/developer/guide/misc/ieee1364-notes.html.
                        if unwrap_get_module(scope.table, instance_sid).ports.len() != ports.len() {
                            diagnostics.not_yet_implemented(
                                arenas.get_range_span(*ports),
                                "unequal number of ports",
                            );
                            return Err(());
                        }

                        for (pi, l_p) in ports.iter().enumerate() {
                            let (net, connection) =
                                unwrap_get_module(scope.table, instance_sid).ports[pi];
                            let VSymbol::Net(n) = &scope.table[net].content else {
                                unreachable!();
                            };
                            let ty = n.ty;
                            let is_input = matches!(
                                connection,
                                ConnectionDirection::In | ConnectionDirection::Both
                            );
                            if is_input {
                                let signal =
                                    lower_to_signal(gl, arenas, scope, diagnostics, l_p, ty)?;
                                // @TODO: Just never allocate this signal.
                                let old_signal = std::mem::replace(
                                    &mut unwrap_get_net_mut(scope.table, net).signal,
                                    signal,
                                );
                                scope.signal_map.insert(old_signal, signal);
                                gl.signals.remove(old_signal);
                            } else {
                                assign_port_output(gl, arenas, scope, diagnostics, l_p, net, ty)?;
                            }
                        }
                    }
                    ListOfPortConnections::Named(ports) => {
                        let mut error = false;
                        let mut signals_assigned =
                            vec![false; unwrap_get_module(scope.table, instance_sid).ports.len()];
                        for p in ports.iter() {
                            let named_port_connection = arenas.get(p);
                            let NamedPortConnection {
                                port_identifier: ast_port_identifier,
                                expression,
                            } = *named_port_connection;

                            let port = scope
                                .table
                                .resolve(instance_sid, ast_port_identifier.item.0)
                                .and_then(|symid| {
                                    let VSymbol::Net(n) = &scope.table[symid].content else {
                                        return None;
                                    };
                                    n.port_idx.map(|i| (i, &n.ty))
                                });

                            let Some((port_idx, port_ty)) = port else {
                                diagnostics.port_not_found(
                                    arenas,
                                    unwrap_get_module(scope.table, instance_sid),
                                    ast_port_identifier,
                                );
                                error = true;
                                continue;
                            };

                            let (net, connection) =
                                unwrap_get_module(scope.table, instance_sid).ports[port_idx];

                            let is_input = matches!(
                                connection,
                                ConnectionDirection::In | ConnectionDirection::Both
                            );

                            match expression {
                                None => {
                                    if is_input {
                                        let size = port_ty.force_net_width();
                                        gl.signals[unwrap_get_net(scope.table, net).signal]
                                            .initialize = Some(Bits::new_zeroed(size));
                                    }
                                }
                                Some(e) if is_input => {
                                    let signal = lower_to_signal(
                                        gl,
                                        arenas,
                                        scope,
                                        diagnostics,
                                        e,
                                        *port_ty,
                                    )?;
                                    // @TODO: Just never allocate this signal.
                                    let old_signal = std::mem::replace(
                                        &mut unwrap_get_net_mut(scope.table, net).signal,
                                        signal,
                                    );
                                    scope.signal_map.insert(old_signal, signal);
                                    gl.signals.remove(old_signal);
                                }
                                Some(e) => {
                                    assign_port_output(
                                        gl,
                                        arenas,
                                        scope,
                                        diagnostics,
                                        e,
                                        net,
                                        *port_ty,
                                    )?;
                                }
                            };

                            if std::mem::replace(&mut signals_assigned[port_idx], true) {
                                diagnostics.duplicate_definition(arenas, ast_port_identifier);
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
            let statement = arenas.get(id).0;
            let bb_builder = new_process(gl, "initial".into(), arenas.get_span(id));
            let bb_builder = statements_to_process(
                gl,
                arenas,
                scope,
                diagnostics,
                bb_builder,
                AstIdRange::single(statement),
            )?;
            bb_builder.halt(gl);
        }
        ModuleOrGenerateItemContent::AlwaysConstruct(id) => {
            let statement = arenas.get(id).0;
            let bb_builder = new_process(gl, "always".into(), arenas.get_span(id));
            let bb_key = bb_builder.key();
            let bb_builder = statements_to_process(
                gl,
                arenas,
                scope,
                diagnostics,
                bb_builder,
                AstIdRange::single(statement),
            )?;
            bb_builder.jump_to(gl, bb_key);
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
    scope: EvalScope<'a>,
    diagnostics: &mut Diagnostics,
    dimensions: AstIdRange<Dimension>,
) -> Result<Vec<u32>, ()> {
    let mut dims = Vec::with_capacity(dimensions.len());
    for dim in dimensions.iter().rev() {
        let Dimension { lhs, rhs } = arenas.get(dim);
        let lhs = eval_constant_expr(gl, arenas, scope, diagnostics, *lhs);
        let rhs = eval_constant_expr(gl, arenas, scope, diagnostics, *rhs);

        let lhs = lhs?.into_bits().as_i64().unwrap();
        let rhs = rhs?.into_bits().as_i64().unwrap();

        dims.push((lhs.abs_diff(rhs) + 1) as u32);
    }
    Ok(dims)
}

pub fn lower_opt_generate_block<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    opt_generate_block: AstId<Option<GenerateBlock>>,
) -> Result<(), ()> {
    match arenas.get(opt_generate_block) {
        None => Ok(()),
        Some(GenerateBlock::BeginEnd(_, module_or_generate_items)) => {
            for m in module_or_generate_items.iter() {
                lower(gl, arenas, scope, m, diagnostics)?;
            }
            Ok(())
        }
        Some(GenerateBlock::ModuleOrGenerateItem(module_or_generate_item)) => {
            lower(gl, arenas, scope, *module_or_generate_item, diagnostics)
        }
    }
}

pub fn lower_variable_type<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    variable_type: AstId<VariableType>,
    ty: VType,
) -> Result<(Vec<u32>, VectorSize, Option<Bits>), ()> {
    match arenas.get(variable_type).variant {
        VariableTypeVariant::Dimensions(dimensions) => {
            let dims = dims_to_array(gl, arenas, scope.eval(), diagnostics, dimensions)?;
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
            let value = eval_constant_expr(gl, arenas, scope.eval(), diagnostics, expr)?;
            let value = value.truncate_or_extend(ty.force_net_width());
            Ok((Vec::new(), ty.force_net_width(), Some(value.into_bits())))
        }
    }
}
