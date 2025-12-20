use std::collections::HashMap;

use vogls_ir::{
    ConnectionDirection, GlobalContext, ProcessKey, Signal, SignalKey, Type, new_process,
};

use crate::ast::AstId;
use crate::ast::constant_expr::ConstantMinTypMaxExpression;
use crate::ast::module::{
    GateInstantiation, GenerateBlock, GenvarAssignment, GenvarDeclaration, ListOfPortConnections,
    LoopGenerateConstruct, Module, ModuleInstance, ModuleInstantiation, ModuleOrGenerateItem,
    ModuleOrGenerateItemDeclaration, NInputGateInstance, NInputGateType, NamedParameterAssignment,
    NamedPortConnection, NetDeclAssignment, NetDeclarationNets, ParameterValueAssignment,
};
use crate::lower::scope::{Symbol, SymbolVariant};
use crate::lower::{
    ModuleArgs, assign_net_lvalue, assign_port_output, eval_constant_expr, fetch_module_interface,
    lower_expr, lower_to_signal, range_to_width, statements_to_process,
};
use crate::parser::AstArenas;

use super::scope::Scope;
use super::vtype::VTypeTable;
use super::{Diagnostics, ModuleInitialization};

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    module_lut: &HashMap<&'a str, AstId<Module>>,
    next_modules: &mut Vec<ModuleInitialization<'a>>,
    scope: &mut Scope<'a>,
    processes: &mut Vec<ProcessKey>,
    id: AstId<ModuleOrGenerateItem>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match arenas.get(id) {
        ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(id) => {
            let module_or_generate_item_declaration = arenas.get(*id);
            match module_or_generate_item_declaration {
                ModuleOrGenerateItemDeclaration::Net(id) => {
                    let net_declaration = arenas.get(*id);
                    let width = match net_declaration.range {
                        None => 1,
                        Some(range) => {
                            range_to_width(gl, arenas, types, scope, diagnostics, range)?
                        }
                    };
                    match net_declaration.nets {
                        NetDeclarationNets::Idents(net_idents) => {
                            for ast_net_ident in net_idents.iter() {
                                let net_ident = arenas.get(ast_net_ident);
                                if !net_ident.dimension.is_empty() {
                                    diagnostics.not_yet_implemented(
                                        arenas.get_span(ast_net_ident),
                                        "net_identifier::dimension",
                                    );
                                    return Err(());
                                }
                                let ast_ident = net_ident.ident;
                                let ident = ast_ident.item;
                                let ident = arenas.get_ident(ident.0);
                                let ty = Type::Bits(width);
                                let key = gl.signals.insert(Signal {
                                    name: ident.into(),
                                    ty: ty.clone(),
                                });
                                let symbol_key = scope.symbols.insert(Symbol {
                                    name: ident.to_string(),
                                    definition_site: arenas.get_item_span(ast_ident),
                                    ty,
                                    variant: SymbolVariant::Signal(key),
                                });
                                scope.push(ident, symbol_key);
                            }
                        }
                        NetDeclarationNets::Assignments(assignments) => {
                            for assignment in assignments.iter() {
                                let NetDeclAssignment {
                                    ident: ast_ident,
                                    expr,
                                } = arenas.get(assignment);
                                let ident = arenas.get_ident(ast_ident.item.0);
                                let ty = Type::Bits(width);
                                let key = gl.signals.insert(Signal {
                                    name: ident.into(),
                                    ty: ty.clone(),
                                });
                                let symbol_key = scope.symbols.insert(Symbol {
                                    name: ident.to_string(),
                                    definition_site: arenas.get_item_span(*ast_ident),
                                    ty,
                                    variant: SymbolVariant::Signal(key),
                                });
                                scope.push(ident, symbol_key);

                                let (section_key, mut bb_builder) =
                                    new_process(gl, "decl_assign".into());
                                let bb_key = bb_builder.key();
                                let variable = lower_expr(
                                    gl,
                                    arenas,
                                    types,
                                    scope,
                                    diagnostics,
                                    &mut bb_builder,
                                    arenas.get(*expr),
                                )?;

                                bb_builder.drive(gl, key, variable);
                                bb_builder.watch_for_ins_to(gl, bb_key);
                                processes.push(section_key);
                            }
                        }
                    }
                }
                ModuleOrGenerateItemDeclaration::Reg(id) => {
                    let reg_declaration = arenas.get(*id);
                    let width = match reg_declaration.range {
                        None => 1,
                        Some(range) => range_to_width(gl, arenas, types, scope, diagnostics, range)?,
                    };
                    for ast_variable_type in reg_declaration.variable_types.iter() {
                        let variable_type = arenas.get(ast_variable_type);
                        let ident = arenas.get_ident(variable_type.identifier.item.0);
                        let key = gl.signals.insert(Signal {
                            name: ident.into(),
                            ty: Type::Bits(width),
                        });
                        let symbol_key = scope.symbols.insert(Symbol {
                            name: ident.to_string(),
                            definition_site: arenas.get_item_span(variable_type.identifier),
                            ty: Type::Bits(width),
                            variant: SymbolVariant::Signal(key),
                        });
                        scope.push(ident, symbol_key);
                    }
                }
                ModuleOrGenerateItemDeclaration::Integer(id) => {
                    let integer_declaration = arenas.get(*id);
                    for ast_ident in integer_declaration.identifiers.iter() {
                        let ident = arenas.get(ast_ident);
                        let ident = arenas.get_ident(ident.0);
                        let symbol_key = scope.symbols.insert(Symbol {
                            name: ident.to_string(),
                            definition_site: arenas.get_span(ast_ident),
                            ty: Type::Decimal,
                            variant: SymbolVariant::Variable(None),
                        });
                        scope.push(ident, symbol_key);
                    }
                }
                ModuleOrGenerateItemDeclaration::Genvar(id) => {
                    let GenvarDeclaration { identifiers } = arenas.get(*id);
                    for ast_ident in identifiers.iter() {
                        let ident = arenas.get(ast_ident);
                        let ident = arenas.get_ident(ident.0);

                        let symbol_key = scope.symbols.insert(Symbol {
                            name: ident.to_string(),
                            definition_site: arenas.get_span(ast_ident),
                            ty: Type::Decimal,
                            variant: SymbolVariant::Genvar(None),
                        });
                        scope.push(ident, symbol_key);
                    }
                }
            }
        }
        ModuleOrGenerateItem::LocalParameterDeclaration => todo!(),
        ModuleOrGenerateItem::ParameterOverride => todo!(),
        ModuleOrGenerateItem::ContinuousAssign(assign) => {
            let assign = arenas.get(*assign);
            for ast_net_assignment in assign.list_of_net_assignments {
                let net_assignment = arenas.get(ast_net_assignment);

                let (section_key, mut bb_builder) = new_process(gl, "assign".into());
                let bb_key = bb_builder.key();
                let variable = lower_expr(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    &mut bb_builder,
                    arenas.get(net_assignment.expression),
                )?;

                assign_net_lvalue(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    &mut bb_builder,
                    net_assignment.net_lvalue,
                    variable,
                )?;

                bb_builder.watch_for_ins_to(gl, bb_key);
                processes.push(section_key);
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

                        let lvalue = arenas.get(*output_terminal);
                        let lvalue = lvalue.ident.item;

                        let ident = arenas.get_ident(lvalue.0);

                        let (section_key, mut bb_builder) = new_process(gl, "gate".into());
                        let bb_key = bb_builder.key();

                        assert!(!input_terminals.is_empty());
                        let value = input_terminals.first().unwrap();
                        let mut value = lower_expr(
                            gl,
                            arenas,
                            types,
                            scope,
                            diagnostics,
                            &mut bb_builder,
                            arenas.get(value),
                        )?;
                        match ninput_gate_instantiation.gatetype.item {
                            NInputGateType::And | NInputGateType::Nand => {
                                for input in input_terminals.iter().skip(1) {
                                    let input = lower_expr(
                                        gl,
                                        arenas,
                                        types,
                                        scope,
                                        diagnostics,
                                        &mut bb_builder,
                                        arenas.get(input),
                                    )?;
                                    value = bb_builder.and(gl, value, input);
                                }
                            }
                            NInputGateType::Or | NInputGateType::Nor => {
                                for input in input_terminals.iter().skip(1) {
                                    let input = lower_expr(
                                        gl,
                                        arenas,
                                        types,
                                        scope,
                                        diagnostics,
                                        &mut bb_builder,
                                        arenas.get(input),
                                    )?;
                                    value = bb_builder.or(gl, value, input);
                                }
                            }
                            NInputGateType::Xor | NInputGateType::Xnor => {
                                for input in input_terminals.iter().skip(1) {
                                    let input = lower_expr(
                                        gl,
                                        arenas,
                                        types,
                                        scope,
                                        diagnostics,
                                        &mut bb_builder,
                                        arenas.get(input),
                                    )?;
                                    value = bb_builder.xor(gl, value, input);
                                }
                            }
                        };

                        if matches!(
                            ninput_gate_instantiation.gatetype.item,
                            NInputGateType::Nand | NInputGateType::Nor | NInputGateType::Xnor
                        ) {
                            value = bb_builder.binary_neg(gl, value);
                        }

                        let symbol_key = scope.get(ident).unwrap();
                        let SymbolVariant::Signal(signal_key) = &scope.symbols[symbol_key].variant
                        else {
                            panic!("not a signal");
                        };

                        bb_builder.drive(gl, *signal_key, value);
                        bb_builder.watch_for_ins_to(gl, bb_key);
                        processes.push(section_key);
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
            let Some(instant_module) = module_lut.get(instantiation_ident) else {
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
                            let value = eval_constant_expr(
                                gl,
                                arenas,
                                types,
                                scope,
                                diagnostics,
                                *expression,
                            )?;
                            params.push((key, value as i64, arenas.get_span(*expression)));
                        }
                    }
                }
            }

            let (instant_params, instant_io, parameters) =
                fetch_module_interface(gl, arenas, types, *instant_module, &params, diagnostics)?;

            for instance in module_instances.iter() {
                let ModuleInstance {
                    name_of_module_instance: _,
                    list_of_port_connections,
                } = arenas.get(instance);

                let signals: Vec<SignalKey> = match arenas.get(*list_of_port_connections) {
                    ListOfPortConnections::Ordered(ports) => {
                        if instant_io.ports.len() != ports.len() {
                            diagnostics.not_yet_implemented(
                                arenas.get_range_span(*ports),
                                "unequal number of ports",
                            );
                            return Err(());
                        }

                        instant_io
                            .ports
                            .iter()
                            .zip(ports.iter())
                            .map(|((_name, connection, width), l_p)| {
                                let is_input = matches!(
                                    connection,
                                    ConnectionDirection::In | ConnectionDirection::Both
                                );
                                if is_input {
                                    let ty = Type::Bits(*width);
                                    lower_to_signal(
                                        gl,
                                        arenas,
                                        types,
                                        scope,
                                        diagnostics,
                                        processes,
                                        l_p,
                                        ty,
                                    )
                                } else {
                                    assign_port_output(
                                        gl,
                                        arenas,
                                        types,
                                        scope,
                                        diagnostics,
                                        processes,
                                        l_p,
                                        *width,
                                    )
                                }
                            })
                            .collect::<Result<Vec<SignalKey>, ()>>()?
                    }
                    ListOfPortConnections::Named(ports) => {
                        let mut error = false;
                        let mut signals = vec![None; instant_io.ports.len()];
                        for p in ports.iter() {
                            let named_port_connection = arenas.get(p);
                            let NamedPortConnection {
                                port_identifier: ast_port_identifier,
                                expression,
                            } = *named_port_connection;
                            let port_identifier = arenas.get_ident(ast_port_identifier.item.0);

                            let Some(&port_idx) = instant_io.lut.get(port_identifier) else {
                                diagnostics.port_not_found(
                                    arenas,
                                    &instant_io,
                                    ast_port_identifier,
                                );
                                error = true;
                                continue;
                            };

                            let (_, connection, width) = instant_io.ports[port_idx];

                            let is_input = matches!(
                                connection,
                                ConnectionDirection::In | ConnectionDirection::Both
                            );

                            let Some(e) = expression else {
                                diagnostics
                                    .not_yet_implemented(arenas.get_span(p), "anonymous ports");
                                error = true;
                                continue;
                            };

                            let signal = if is_input {
                                lower_to_signal(
                                    gl,
                                    arenas,
                                    types,
                                    scope,
                                    diagnostics,
                                    processes,
                                    e,
                                    Type::Bits(width),
                                )?
                            } else {
                                assign_port_output(
                                    gl,
                                    arenas,
                                    types,
                                    scope,
                                    diagnostics,
                                    processes,
                                    e,
                                    width,
                                )?
                            };

                            if signals[port_idx].replace(signal).is_some() {
                                diagnostics.duplicate_definition(arenas, ast_port_identifier);
                                error = true;
                                continue;
                            }
                        }

                        for s in signals.iter() {
                            if s.is_none() {
                                diagnostics.not_yet_implemented(
                                    arenas.get_range_span(*ports),
                                    "missing port connection",
                                );
                                error = true;
                            }
                        }

                        if error {
                            return Err(());
                        }

                        signals.into_iter().map(|s| s.unwrap()).collect()
                    }
                };
                next_modules.push(ModuleInitialization {
                    name: instantiation_ident,
                    parameters: instant_params.clone(),
                    io: instant_io.clone(),
                    args: ModuleArgs {
                        parameters: parameters.clone(),
                        signals,
                    },
                });
            }
        }
        ModuleOrGenerateItem::InitialConstruct(id) => {
            let statement = arenas.get(*id).0;
            let (section_key, bb_builder) = new_process(gl, "initial".into());
            let bb_builder = statements_to_process(
                gl,
                arenas,
                types,
                scope,
                diagnostics,
                bb_builder,
                std::slice::from_ref(arenas.get(statement)),
            )?;
            bb_builder.halt(gl);
            processes.push(section_key);
        }
        ModuleOrGenerateItem::AlwaysConstruct(id) => {
            let statement = arenas.get(*id).0;
            let (section_key, bb_builder) = new_process(gl, "always".into());
            let bb_key = bb_builder.key();
            let bb_builder = statements_to_process(
                gl,
                arenas,
                types,
                scope,
                diagnostics,
                bb_builder,
                std::slice::from_ref(arenas.get(statement)),
            )?;
            bb_builder.watch_for_ins_to(gl, bb_key);
            processes.push(section_key);
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

            let SymbolVariant::Genvar(_) = &mut scope.symbols[symbol_key].variant else {
                diagnostics.not_yet_implemented(
                    arenas.get_item_span(*init_ident),
                    "generate for-loop on non-genvar",
                );
                return Err(());
            };

            let mut v = eval_constant_expr(gl, arenas, types, &scope, diagnostics, *init_expr)?;
            scope.symbols[symbol_key].variant = SymbolVariant::Genvar(Some(v as i64));

            loop {
                let condition =
                    eval_constant_expr(gl, arenas, types, &scope, diagnostics, *condition)?;
                if condition == 0 {
                    break;
                }

                match arenas.get(*block) {
                    GenerateBlock::ModuleOrGenerateItem(id) => lower(
                        gl,
                        arenas,
                        types,
                        module_lut,
                        next_modules,
                        scope,
                        processes,
                        *id,
                        diagnostics,
                    )?,
                    GenerateBlock::BeginEnd(_, ids) => {
                        for id in ids.iter() {
                            lower(
                                gl,
                                arenas,
                                types,
                                module_lut,
                                next_modules,
                                scope,
                                processes,
                                id,
                                diagnostics,
                            )?;
                        }
                    }
                }

                v = eval_constant_expr(gl, arenas, types, &scope, diagnostics, *iter_expr)?;
                scope.symbols[symbol_key].variant = SymbolVariant::Genvar(Some(v as i64));
            }
        }
        ModuleOrGenerateItem::IfGenerateConstruct(_id) => todo!(),
        ModuleOrGenerateItem::CaseGenerateConstruct(_id) => todo!(),
    }

    Ok(())
}
