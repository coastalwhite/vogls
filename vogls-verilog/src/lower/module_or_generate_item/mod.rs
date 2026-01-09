use vogls_ir::vcd::NetType;
use vogls_ir::{
    Bits, ConnectionDirection, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, Signal, SignalKey,
    VectorSize, new_process,
};

use crate::ast::constant_expr::ConstantMinTypMaxExpression;
use crate::ast::module::{
    Dimension, GateInstantiation, GenerateBlock, GenvarAssignment, IfGenerateConstruct,
    ListOfPortConnections, LoopGenerateConstruct, ModuleInstance, ModuleInstantiation,
    ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration, NInputGateInstance, NInputGateType,
    NamedParameterAssignment, NamedPortConnection, NetDeclAssignment, NetDeclarationNets,
    ParameterValueAssignment, VariableType, VariableTypeVariant,
};
use crate::ast::{AstId, AstIdRange};
use crate::hierarchy::{HierarchyItem, HierarchyModule, HierarchyNet};
use crate::lower::assign::{assign_net_lvalue, net_lvalue_width};
use crate::lower::expression::{self, lower_expr, truncate_or_extend};
use crate::lower::statement::statements_to_process;
use crate::lower::vvalue::VValue;
use crate::lower::{
    VType, assign_port_output, eval_constant_expr, evaluate_range, lower_to_signal,
};
use crate::parser::AstArenas;

use super::Scope;
use super::{Diagnostics, ModuleContext};

mod function;

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    mc: &mut ModuleContext<'a>,
    scope: &mut Scope<'a>,
    id: AstId<ModuleOrGenerateItem>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match arenas.get(id) {
        ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(id) => {
            let module_or_generate_item_declaration = arenas.get(*id);
            match module_or_generate_item_declaration {
                ModuleOrGenerateItemDeclaration::Net(id) => {
                    let net_declaration = arenas.get(*id);
                    let (msb, lsb, width) = match net_declaration.range {
                        None => (0, 0, SCALAR_VSIZE),
                        Some(range) => evaluate_range(arenas, scope, diagnostics, range)?,
                    };
                    let ty = VType::net(width, net_declaration.signed);
                    match net_declaration.nets {
                        NetDeclarationNets::Idents(net_idents) => {}
                        NetDeclarationNets::Assignments(assignments) => {
                            for assignment in assignments.iter() {
                                let NetDeclAssignment {
                                    ident: ast_ident,
                                    expr,
                                } = arenas.get(assignment);
                                let ident = arenas.get_ident(ast_ident.item.0);
                                let key = gl.signals.insert(Signal {
                                    name: ident.into(),
                                    size: ty.force_net_width(),
                                    initialize: None,
                                    origin: arenas.get_span(assignment),
                                });

                                let symbol_key = scope.get(ident).unwrap();
                                let HierarchyItem::Net(i) =
                                    &scope.hierarchy.items()[symbol_key.as_idx()]
                                else {
                                    panic!();
                                };
                                let net = &scope.hierarchy.net()[*i];

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
                ModuleOrGenerateItemDeclaration::Reg(id) => {}
                ModuleOrGenerateItemDeclaration::Integer(id) => {
                    let integer_declaration = arenas.get(*id);
                    let msb = 31;
                    let lsb = 31;
                    let ty = VType::SignedNet(INTEGER_VSIZE);
                    for variable_type in integer_declaration.variable_types.iter() {
                        let (dims, size, initialize) =
                            lower_variable_type(arenas, scope, diagnostics, variable_type, ty)?;
                        let Some(initialize) = initialize else {
                            continue;
                        };
                        let ident = arenas.get_ident(arenas.get(variable_type).identifier.item.0);

                        let symbol_key = scope.get(ident).unwrap();
                        let HierarchyItem::Net(i) = &scope.hierarchy.items()[symbol_key.as_idx()]
                        else {
                            panic!();
                        };
                        let net = &scope.hierarchy.net()[*i];
                        gl.signals[net.signal].initialize = Some(initialize);
                    }
                }
                ModuleOrGenerateItemDeclaration::Genvar(id) => {}
                ModuleOrGenerateItemDeclaration::Task(id) => {}
                ModuleOrGenerateItemDeclaration::Function(id) => {}
            }
        }
        ModuleOrGenerateItem::LocalParameterDeclaration(id) => {}
        ModuleOrGenerateItem::ParameterOverride => todo!(),
        ModuleOrGenerateItem::ContinuousAssign(id) => {
            let assign = arenas.get(*id);
            for ast_net_assignment in assign.list_of_net_assignments {
                let net_assignment = arenas.get(ast_net_assignment);

                let mut bb_builder = new_process(gl, "assign".into(), arenas.get_span(*id));
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
        ModuleOrGenerateItem::GateInstantiation(id) => {
            let gate_instantiation = arenas.get(*id);
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
                            net_lvalue_width(arenas, scope, diagnostics, *output_terminal)?;

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
        ModuleOrGenerateItem::UdpInstantiation => todo!(),
        ModuleOrGenerateItem::ModuleInstantiation(id) => {
            let ModuleInstantiation {
                module_identifier,
                parameter_value_assignment,
                module_instances,
            } = arenas.get(*id);
            let instantiation_ident = arenas.get_ident(module_identifier.item.0);
            let Some(instant_module) = mc.named_lookup.get(instantiation_ident) else {
                diagnostics.module_not_found(arenas, *module_identifier);
                return Err(());
            };

            let mut params = Vec::new();
            if let Some(parameter_value_assignment) = parameter_value_assignment {
                match arenas.get(*parameter_value_assignment) {
                    ParameterValueAssignment::Ordered(_) => {
                        diagnostics.not_yet_implemented(
                            arenas.get_span(*parameter_value_assignment),
                            "ordered parameter assignment",
                        );
                        return Err(());
                    }
                    ParameterValueAssignment::Named(named) => {
                        for n in named.iter() {
                            let NamedParameterAssignment {
                                identifier,
                                expression,
                            } = arenas.get(n);
                            let key = arenas.get_ident(identifier.item.0);
                            let Some(expression) = expression else {
                                diagnostics.not_yet_implemented(
                                    arenas.get_span(n),
                                    "null parameter assignment",
                                );
                                return Err(());
                            };
                            let ConstantMinTypMaxExpression::Single(expression) =
                                arenas.get(*expression)
                            else {
                                diagnostics.not_yet_implemented(
                                    arenas.get_span(n),
                                    "mintypmax parameter assignment",
                                );
                                return Err(());
                            };
                            let value =
                                eval_constant_expr(arenas, scope, diagnostics, *expression)?;
                            params.push((key, value, arenas.get_span(*expression)));
                        }
                    }
                }
            }

            for instance in module_instances.iter() {
                let ModuleInstance {
                    name_of_module_instance,
                    list_of_port_connections,
                } = arenas.get(instance);

                let name_of_module_instance = arenas.get_ident(name_of_module_instance.item.0);
                let symbol_key = scope.get(name_of_module_instance).unwrap();
                let HierarchyItem::Module(i) = scope.hierarchy.items()[symbol_key.as_idx()] else {
                    panic!();
                };
                let instant_module = &scope.hierarchy.modules()[i];

                let signals: Vec<SignalKey> = match arenas.get(*list_of_port_connections) {
                    ListOfPortConnections::Ordered(ports) => {
                        if instant_module.ports.len() != ports.len() {
                            diagnostics.not_yet_implemented(
                                arenas.get_range_span(*ports),
                                "unequal number of ports",
                            );
                            return Err(());
                        }

                        instant_module
                            .ports
                            .iter()
                            .zip(ports.iter())
                            .map(|((net, connection), l_p)| {
                                let ty = scope.hierarchy.net()[*net].ty;
                                let is_input = matches!(
                                    connection,
                                    ConnectionDirection::In | ConnectionDirection::Both
                                );
                                if is_input {
                                    lower_to_signal(gl, arenas, scope, diagnostics, l_p, ty)
                                } else {
                                    assign_port_output(gl, arenas, scope, diagnostics, l_p, ty)
                                }
                            })
                            .collect::<Result<Vec<SignalKey>, ()>>()?
                    }
                    ListOfPortConnections::Named(ports) => {
                        let mut error = false;
                        let mut signals = vec![None; instant_module.ports.len()];
                        for p in ports.iter() {
                            let named_port_connection = arenas.get(p);
                            let NamedPortConnection {
                                port_identifier: ast_port_identifier,
                                expression,
                            } = *named_port_connection;
                            let port_identifier = arenas.get_ident(ast_port_identifier.item.0);

                            let Some(&port_idx) = instant_module.lut.get(port_identifier) else {
                                diagnostics.port_not_found(
                                    arenas,
                                    &instant_module,
                                    ast_port_identifier,
                                );
                                error = true;
                                continue;
                            };

                            let (net, connection) = instant_module.ports[port_idx];
                            let HierarchyNet {
                                name,
                                signal: _,
                                ty: port_ty,
                                dims: _,
                            } = &scope.hierarchy.net()[net];

                            let is_input = matches!(
                                connection,
                                ConnectionDirection::In | ConnectionDirection::Both
                            );

                            let signal = match expression {
                                None => {
                                    let size = port_ty.force_net_width();
                                    gl.signals.insert(Signal {
                                        name: format!("{name}::UNCONNECTED"),
                                        size,
                                        initialize: None,
                                        origin: arenas.get_span(instance),
                                    })
                                }
                                Some(e) if is_input => {
                                    lower_to_signal(gl, arenas, scope, diagnostics, e, *port_ty)?
                                }
                                Some(e) => {
                                    assign_port_output(gl, arenas, scope, diagnostics, e, *port_ty)?
                                }
                            };

                            if signals[port_idx].replace(signal).is_some() {
                                diagnostics.duplicate_definition(arenas, ast_port_identifier);
                                error = true;
                                continue;
                            }
                        }

                        if error {
                            return Err(());
                        }

                        signals.into_iter().map(|s| s.unwrap()).collect()
                    }
                };
            }
        }
        ModuleOrGenerateItem::InitialConstruct(id) => {
            let statement = arenas.get(*id).0;
            let bb_builder = new_process(gl, "initial".into(), arenas.get_span(*id));
            let bb_builder = statements_to_process(
                gl,
                arenas,
                scope,
                mc,
                diagnostics,
                bb_builder,
                AstIdRange::single(statement),
            )?;
            bb_builder.halt(gl);
        }
        ModuleOrGenerateItem::AlwaysConstruct(id) => {
            let statement = arenas.get(*id).0;
            let bb_builder = new_process(gl, "always".into(), arenas.get_span(*id));
            let bb_key = bb_builder.key();
            let bb_builder = statements_to_process(
                gl,
                arenas,
                scope,
                mc,
                diagnostics,
                bb_builder,
                AstIdRange::single(statement),
            )?;
            bb_builder.jump_to(gl, bb_key);
        }
        ModuleOrGenerateItem::LoopGenerateConstruct(id) => {
            let LoopGenerateConstruct {
                initialization,
                condition,
                iteration,
                block,
            } = arenas.get(*id);

            let GenvarAssignment {
                ident: init_ident,
                expr: init_expr,
            } = arenas.get(*initialization);
            let GenvarAssignment {
                ident: iter_ident,
                expr: iter_expr,
            } = arenas.get(*iteration);

            if arenas.get_ident(init_ident.item.0) != arenas.get_ident(iter_ident.item.0) {
                diagnostics.not_yet_implemented(
                    arenas.get_item_span(*init_ident),
                    "cannot do a generate for-loop with different identifiers",
                );
                return Err(());
            }

            let variable = arenas.get_ident(init_ident.item.0);
            let Some(symbol_key) = scope.get(variable) else {
                diagnostics.var_not_found(arenas, *init_ident);
                return Err(());
            };

            todo!()
            /*
            let SymbolVariant::Genvar(_) = &mut scope.symbols[symbol_key].variant else {
                diagnostics.not_yet_implemented(
                    arenas.get_item_span(*init_ident),
                    "generate for-loop on non-genvar",
                );
                return Err(());
            };

            let v = eval_constant_expr(arenas, &scope, diagnostics, *init_expr)?;
            let mut v = v.as_integer().unwrap();
            scope.symbols[symbol_key].variant = SymbolVariant::Genvar(Some(v));

            loop {
                let condition = eval_constant_expr(arenas, &scope, diagnostics, *condition)?;
                let condition = condition.as_integer().unwrap();
                if condition == 0 {
                    break;
                }

                match arenas.get(*block) {
                    GenerateBlock::ModuleOrGenerateItem(id) => {
                        lower(gl, arenas, mc, scope, *id, diagnostics)?
                    }
                    GenerateBlock::BeginEnd(_, ids) => {
                        for id in ids.iter() {
                            lower(gl, arenas, mc, scope, id, diagnostics)?;
                        }
                    }
                }

                v = eval_constant_expr(arenas, &scope, diagnostics, *iter_expr)?
                    .as_integer()
                    .unwrap();
                scope.symbols[symbol_key].variant = SymbolVariant::Genvar(Some(v as i64));
            }
            */
        }
        ModuleOrGenerateItem::IfGenerateConstruct(id) => {
            let IfGenerateConstruct {
                condition,
                truthy,
                falsy,
            } = arenas.get(*id);

            let v = eval_constant_expr(arenas, &scope, diagnostics, *condition)?;
            if v.logical_equal(VValue::UnsignedNet(Bits::new_zeroed(SCALAR_VSIZE))) {
                if let Some(falsy) = falsy {
                    lower_opt_generate_block(gl, arenas, scope, diagnostics, mc, *falsy)?;
                }
            } else {
                lower_opt_generate_block(gl, arenas, scope, diagnostics, mc, *truthy)?;
            }
        }
        ModuleOrGenerateItem::CaseGenerateConstruct(_id) => todo!(),
    }

    Ok(())
}

pub fn dims_to_array<'a>(
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    dimensions: AstIdRange<Dimension>,
) -> Result<Vec<u32>, ()> {
    let mut dims = Vec::with_capacity(dimensions.len());
    for dim in dimensions.iter().rev() {
        let Dimension { lhs, rhs } = arenas.get(dim);
        let lhs = eval_constant_expr(arenas, scope, diagnostics, *lhs);
        let rhs = eval_constant_expr(arenas, scope, diagnostics, *rhs);

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
    mc: &mut ModuleContext<'a>,
    opt_generate_block: AstId<Option<GenerateBlock>>,
) -> Result<(), ()> {
    match arenas.get(opt_generate_block) {
        None => Ok(()),
        Some(GenerateBlock::BeginEnd(_, module_or_generate_items)) => {
            for m in module_or_generate_items.iter() {
                lower(gl, arenas, mc, scope, m, diagnostics)?;
            }
            Ok(())
        }
        Some(GenerateBlock::ModuleOrGenerateItem(module_or_generate_item)) => {
            lower(gl, arenas, mc, scope, *module_or_generate_item, diagnostics)
        }
    }
}

pub fn lower_variable_type<'a>(
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    variable_type: AstId<VariableType>,
    ty: VType,
) -> Result<(Vec<u32>, VectorSize, Option<Bits>), ()> {
    match arenas.get(variable_type).variant {
        VariableTypeVariant::Dimensions(dimensions) => {
            let dims = dims_to_array(arenas, scope, diagnostics, dimensions)?;
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
            let value = eval_constant_expr(arenas, scope, diagnostics, expr)?;
            let value = value.truncate_or_extend(ty.force_net_width());
            Ok((Vec::new(), ty.force_net_width(), Some(value.into_bits())))
        }
    }
}
