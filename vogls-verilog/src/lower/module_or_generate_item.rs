use std::collections::HashMap;

use vogls_ir::{
    Bits, ConnectionDirection, GlobalContext, ProcessKey, Signal, SignalKey, new_process,
};

use crate::ast::constant_expr::ConstantMinTypMaxExpression;
use crate::ast::module::{
    Dimension, GateInstantiation, GenerateBlock, GenvarAssignment, GenvarDeclaration,
    IfGenerateConstruct, ListOfPortConnections, LocalParameterDeclaration, LoopGenerateConstruct,
    Module, ModuleInstance, ModuleInstantiation, ModuleOrGenerateItem,
    ModuleOrGenerateItemDeclaration, NInputGateInstance, NInputGateType, NamedParameterAssignment,
    NamedPortConnection, NetDeclAssignment, NetDeclarationNets, ParamAssignment,
    ParameterValueAssignment, TaskDeclaration, VariableType,
};
use crate::ast::{AstId, AstIdRange};
use crate::lower::expression::{self, lower_expr};
use crate::lower::scope::{Symbol, SymbolVariant};
use crate::lower::vvalue::VValue;
use crate::lower::{
    ModuleArgs, VType, assign_net_lvalue, assign_port_output, eval_constant_expr,
    fetch_module_interface, lower_to_signal, range_to_width, statements_to_process,
};
use crate::parser::AstArenas;

use super::scope::Scope;
use super::{Diagnostics, ModuleInitialization};

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
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
                    let ty = match net_declaration.range {
                        None => VType::SCALAR_NET,
                        Some(range) => {
                            VType::Net(range_to_width(gl, arenas, scope, diagnostics, range)?)
                        }
                    };
                    match net_declaration.nets {
                        NetDeclarationNets::Idents(net_idents) => {
                            for ast_net_ident in net_idents.iter() {
                                let net_ident = arenas.get(ast_net_ident);
                                let dims = dims_to_array(
                                    gl,
                                    arenas,
                                    scope,
                                    diagnostics,
                                    net_ident.dimension,
                                )?;
                                let Some(size) =
                                    ty.force_net_width().checked_mul(dims.iter().product())
                                else {
                                    diagnostics.not_yet_implemented(
                                        arenas.get_span(ast_net_ident),
                                        "overflow in net width",
                                    );
                                    return Err(());
                                };
                                let ast_ident = net_ident.ident;
                                let ident = ast_ident.item;
                                let ident = arenas.get_ident(ident.0);
                                let key = gl.signals.insert(Signal {
                                    name: ident.into(),
                                    size,
                                });
                                let symbol_key = scope.symbols.insert(Symbol {
                                    name: ident.to_string(),
                                    definition_site: arenas.get_item_span(ast_ident),
                                    variant: SymbolVariant::Signal(dims, ty, key),
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
                                let key = gl.signals.insert(Signal {
                                    name: ident.into(),
                                    size: ty.force_net_width(),
                                });
                                let symbol_key = scope.symbols.insert(Symbol {
                                    name: ident.to_string(),
                                    definition_site: arenas.get_item_span(*ast_ident),
                                    variant: SymbolVariant::Signal(Vec::new(), ty, key),
                                });
                                scope.push(ident, symbol_key);

                                let (section_key, mut bb_builder) =
                                    new_process(gl, "decl_assign".into());
                                let bb_key = bb_builder.key();
                                let (v, v_ty) = lower_expr(
                                    gl,
                                    arenas,
                                    scope,
                                    diagnostics,
                                    &mut bb_builder,
                                    *expr,
                                )?;
                                let v = expression::sign_extend_or_truncate(
                                    gl,
                                    &mut bb_builder,
                                    v,
                                    v_ty,
                                    ty.force_net_width(),
                                );

                                bb_builder.drive(gl, key, v);
                                bb_builder.watch_for_ins_to(gl, bb_key);
                                processes.push(section_key);
                            }
                        }
                    }
                }
                ModuleOrGenerateItemDeclaration::Reg(id) => {
                    let reg_declaration = arenas.get(*id);
                    let ty = match reg_declaration.range {
                        None => VType::SCALAR_NET,
                        Some(range) => {
                            VType::Net(range_to_width(gl, arenas, scope, diagnostics, range)?)
                        }
                    };
                    for ast_variable_type in reg_declaration.variable_types.iter() {
                        let VariableType {
                            identifier,
                            dimensions,
                        } = arenas.get(ast_variable_type);
                        let ident = arenas.get_ident(identifier.item.0);

                        let dims = dims_to_array(gl, arenas, scope, diagnostics, *dimensions)?;
                        let Some(size) = ty.force_net_width().checked_mul(dims.iter().product())
                        else {
                            diagnostics.not_yet_implemented(
                                arenas.get_span(ast_variable_type),
                                "overflow in net width",
                            );
                            return Err(());
                        };
                        let key = gl.signals.insert(Signal {
                            name: ident.into(),
                            size,
                        });
                        let symbol_key = scope.symbols.insert(Symbol {
                            name: ident.to_string(),
                            definition_site: arenas.get_item_span(*identifier),
                            variant: SymbolVariant::Signal(dims, ty, key),
                        });
                        scope.push(ident, symbol_key);
                    }
                }
                ModuleOrGenerateItemDeclaration::Integer(id) => {
                    let integer_declaration = arenas.get(*id);
                    for ast_ident in integer_declaration.identifiers.iter() {
                        let ident = arenas.get(ast_ident);
                        let ident = arenas.get_ident(ident.0);

                        let key = gl.signals.insert(Signal {
                            name: ident.into(),
                            size: VType::Integer.force_net_width(),
                        });
                        let symbol_key = scope.symbols.insert(Symbol {
                            name: ident.to_string(),
                            definition_site: arenas.get_span(ast_ident),
                            variant: SymbolVariant::Signal(Vec::new(), VType::Integer, key),
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
                            variant: SymbolVariant::Genvar(None),
                        });
                        scope.push(ident, symbol_key);
                    }
                }
                ModuleOrGenerateItemDeclaration::Task(id) => {
                    let TaskDeclaration {
                        ident,
                        automatic: _,
                        statement_or_null,
                    } = arenas.get(*id);

                    // @FIXME: Currently the tasks just get lowered with the Scope of the caller,
                    // but this should be the scope of the definer. I think there is probably some
                    // stuff that can be done here by only lowering once and then reusing.
                    let name = arenas.get_ident(ident.item.0);
                    let symbol_key = scope.symbols.insert(Symbol {
                        name: name.to_string(),
                        definition_site: arenas.get_item_span(*ident),
                        variant: SymbolVariant::Task(*statement_or_null),
                    });
                    scope.push(name, symbol_key);
                }
            }
        }
        ModuleOrGenerateItem::LocalParameterDeclaration(id) => {
            let LocalParameterDeclaration {
                typing,
                assignments,
            } = arenas.get(*id);

            // @FIXME: Coerce value to ty.
            let _ty = super::parameter::parameter_typing_to_type(
                gl,
                arenas,
                scope,
                diagnostics,
                *typing,
            )?;
            for assignment in assignments.iter() {
                let ParamAssignment { param, constant } = arenas.get(assignment);
                let key = arenas.get_ident(param.item.0);
                let value = arenas.get(*constant);
                match value {
                    ConstantMinTypMaxExpression::Single(id) => {
                        let value = eval_constant_expr(gl, arenas, &scope, diagnostics, *id)?;
                        let symbol_key = scope.symbols.insert(Symbol {
                            name: key.to_string(),
                            definition_site: arenas.get_item_span(*param),
                            variant: SymbolVariant::Constant(value.clone()),
                        });
                        scope.push(key, symbol_key);
                    }
                    ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
                }
            }
        }
        ModuleOrGenerateItem::ParameterOverride => todo!(),
        ModuleOrGenerateItem::ContinuousAssign(assign) => {
            let assign = arenas.get(*assign);
            for ast_net_assignment in assign.list_of_net_assignments {
                let net_assignment = arenas.get(ast_net_assignment);

                let (section_key, mut bb_builder) = new_process(gl, "assign".into());
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
                        let (mut value, _) =
                            lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, value)?;
                        match ninput_gate_instantiation.gatetype.item {
                            NInputGateType::And | NInputGateType::Nand => {
                                for input in input_terminals.iter().skip(1) {
                                    let (input, _) = lower_expr(
                                        gl,
                                        arenas,
                                        scope,
                                        diagnostics,
                                        &mut bb_builder,
                                        input,
                                    )?;
                                    value = bb_builder.and(gl, value, input);
                                }
                            }
                            NInputGateType::Or | NInputGateType::Nor => {
                                for input in input_terminals.iter().skip(1) {
                                    let (input, _) = lower_expr(
                                        gl,
                                        arenas,
                                        scope,
                                        diagnostics,
                                        &mut bb_builder,
                                        input,
                                    )?;
                                    value = bb_builder.or(gl, value, input);
                                }
                            }
                            NInputGateType::Xor | NInputGateType::Xnor => {
                                for input in input_terminals.iter().skip(1) {
                                    let (input, _) = lower_expr(
                                        gl,
                                        arenas,
                                        scope,
                                        diagnostics,
                                        &mut bb_builder,
                                        input,
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
                        let SymbolVariant::Signal(_dims, _ty, signal_key) =
                            &scope.symbols[symbol_key].variant
                        else {
                            panic!("not a signal");
                        };
                        // @TODO: Coerce inputs and output

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
                            let value =
                                eval_constant_expr(gl, arenas, scope, diagnostics, *expression)?;
                            params.push((key, value, arenas.get_span(*expression)));
                        }
                    }
                }
            }

            let (instant_params, instant_io, parameters) =
                fetch_module_interface(gl, arenas, *instant_module, &params, diagnostics)?;

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
                            .map(|((_name, connection, ty), l_p)| {
                                let is_input = matches!(
                                    connection,
                                    ConnectionDirection::In | ConnectionDirection::Both
                                );
                                if is_input {
                                    lower_to_signal(
                                        gl,
                                        arenas,
                                        scope,
                                        diagnostics,
                                        processes,
                                        l_p,
                                        *ty,
                                    )
                                } else {
                                    assign_port_output(
                                        gl,
                                        arenas,
                                        scope,
                                        diagnostics,
                                        processes,
                                        l_p,
                                        *ty,
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
                                    scope,
                                    diagnostics,
                                    processes,
                                    e,
                                    width,
                                )?
                            } else {
                                assign_port_output(
                                    gl,
                                    arenas,
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
                scope,
                diagnostics,
                bb_builder,
                std::slice::from_ref(arenas.get(statement)),
            )?;
            bb_builder.jump_to(gl, bb_key);
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

            let v = eval_constant_expr(gl, arenas, &scope, diagnostics, *init_expr)?;
            let mut v = v.as_integer().unwrap();
            scope.symbols[symbol_key].variant = SymbolVariant::Genvar(Some(v));

            loop {
                let condition = eval_constant_expr(gl, arenas, &scope, diagnostics, *condition)?;
                let condition = condition.as_integer().unwrap();
                if condition == 0 {
                    break;
                }

                match arenas.get(*block) {
                    GenerateBlock::ModuleOrGenerateItem(id) => lower(
                        gl,
                        arenas,
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

                v = eval_constant_expr(gl, arenas, &scope, diagnostics, *iter_expr)?
                    .as_integer()
                    .unwrap();
                scope.symbols[symbol_key].variant = SymbolVariant::Genvar(Some(v as i64));
            }
        }
        ModuleOrGenerateItem::IfGenerateConstruct(id) => {
            let IfGenerateConstruct {
                condition,
                truthy,
                falsy,
            } = arenas.get(*id);

            let v = eval_constant_expr(gl, arenas, &scope, diagnostics, *condition)?;
            if v.logical_equal(VValue::Net(Bits::new_zeroed(1))) {
                if let Some(falsy) = falsy {
                    lower_opt_generate_block(
                        gl,
                        arenas,
                        scope,
                        diagnostics,
                        module_lut,
                        next_modules,
                        processes,
                        *falsy,
                    )?;
                }
            } else {
                lower_opt_generate_block(
                    gl,
                    arenas,
                    scope,
                    diagnostics,
                    module_lut,
                    next_modules,
                    processes,
                    *truthy,
                )?;
            }
        }
        ModuleOrGenerateItem::CaseGenerateConstruct(_id) => todo!(),
    }

    Ok(())
}

pub fn dims_to_array<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    dimensions: AstIdRange<Dimension>,
) -> Result<Vec<u32>, ()> {
    let mut dims = Vec::with_capacity(dimensions.len());
    for dim in dimensions.iter().rev() {
        let Dimension { lhs, rhs } = arenas.get(dim);
        let lhs = eval_constant_expr(gl, arenas, scope, diagnostics, *lhs);
        let rhs = eval_constant_expr(gl, arenas, scope, diagnostics, *rhs);

        let (Ok(VValue::Integer(lhs)), Ok(VValue::Integer(rhs))) = (lhs, rhs) else {
            return Err(());
        };

        dims.push((lhs.abs_diff(rhs) + 1) as u32);
    }
    Ok(dims)
}

pub fn lower_opt_generate_block<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    module_lut: &HashMap<&'a str, AstId<Module>>,
    next_modules: &mut Vec<ModuleInitialization<'a>>,
    processes: &mut Vec<ProcessKey>,
    opt_generate_block: AstId<Option<GenerateBlock>>,
) -> Result<(), ()> {
    match arenas.get(opt_generate_block) {
        None => Ok(()),
        Some(GenerateBlock::BeginEnd(_, module_or_generate_items)) => {
            for m in module_or_generate_items.iter() {
                lower(
                    gl,
                    arenas,
                    module_lut,
                    next_modules,
                    scope,
                    processes,
                    m,
                    diagnostics,
                )?;
            }
            Ok(())
        }
        Some(GenerateBlock::ModuleOrGenerateItem(module_or_generate_item)) => lower(
            gl,
            arenas,
            module_lut,
            next_modules,
            scope,
            processes,
            *module_or_generate_item,
            diagnostics,
        ),
    }
}
