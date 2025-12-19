mod diagnostics;
mod scope;
mod statement;

use std::collections::{HashMap, HashSet};

use scope::Scope;

use vogls_ir::{
    BasicBlockBuilder, Bits, ConnectionDirection, GlobalContext, IntrinsicArg, IntrinsicOp,
    ModuleBuilder, ModuleKey, ProcessKey, Signal, SignalKey, Time, Type, Value, VariableKey,
    VectorSize,
};

use crate::ast::constant_expr::{
    ConstantExpr, ConstantMinTypMaxExpression, ConstantPrimary, ConstantRangeExpression,
};
use crate::ast::expr::{BinaryOperator, BitPartSelect, BitSlice, Expr, UnaryOperator};
use crate::ast::module::{
    GateInstantiation, ListOfPortConnections, Module, ModuleInstance, ModuleInstantiation,
    ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration, ModulePorts,
    NInputGateInstance, NInputGateType, NamedPortConnection, NetDeclAssignment, NetDeclarationNets,
    NonPortModuleItem, ParamAssignment, ParameterDeclaration, Port, PortDeclaration, Range,
};
use crate::ast::statement::{
    BlockingAssignment, DelayControl, DelayValue, EventControl, EventExpression,
    LoopStatementVariant, NetLValue, NonBlockingAssignment, ProceduralTimingControl, Statement,
    StatementOrNull, VariableAssignment, VariableLValue,
};
use crate::ast::{AstId, AstIdRange, RangeExpression};
use crate::number::Decimal;
use crate::parser::{Ast, AstArenas};

use self::scope::{Symbol, SymbolKey, SymbolVariant};
pub use diagnostics::Diagnostics;

pub fn lower_module_to_ir(
    ast: &Ast,
    root: AstId<Module>,
    gl: &mut GlobalContext,
    instantiated_modules: &HashMap<&str, Result<ModuleKey, ()>>,
    diagnostics: &mut Diagnostics,
) -> Result<ModuleKey, ()> {
    let Ast {
        modules: _,
        arenas,
        path: _,
    } = ast;

    let Module {
        attribute_instances: _,
        module_identifier,
        ports,
        module_items,
    } = arenas.get(root);

    let module_identifier = arenas.get_ident(module_identifier.item.0);
    let mut module_builder = ModuleBuilder::new(module_identifier.to_string(), gl);
    let mut scope = Scope::new();
    let mut processes = Vec::new();

    let mut defined_ports = HashMap::new();

    match ports {
        ModulePorts::PortDeclarations(m) => {
            for port_declaration in m.iter() {
                let port_declaration = arenas.get(port_declaration);

                let (idents, range) = match port_declaration {
                    PortDeclaration::Inout(i) => {
                        let i = arenas.get(*i);
                        (i.port_identifiers, i.range)
                    }
                    PortDeclaration::Input(i) => {
                        let i = arenas.get(*i);
                        (i.port_identifiers, i.range)
                    }
                    PortDeclaration::Output(i) => {
                        let i = arenas.get(*i);
                        (i.identifiers, i.range)
                    }
                };

                let width = match range {
                    None => 1,
                    Some(range) => range_to_width(gl, &mut scope, range, arenas, diagnostics)?,
                };

                for ast_ident in idents.iter() {
                    let ident = arenas.get_ident(arenas.get(ast_ident).0);

                    let key = gl.signals.insert(Signal {
                        name: ident.into(),
                        ty: Type::Bits(width),
                    });
                    let symbol_key = scope.symbols.insert(Symbol {
                        name: ident.to_string(),
                        definition_site: arenas.get_span(ast_ident),
                        ty: Type::Bits(width),
                        variant: SymbolVariant::Signal(key),
                    });
                    scope.push(ident, symbol_key);

                    module_builder.entity.signal(gl, key);
                    match port_declaration {
                        PortDeclaration::Inout(_) => todo!(),
                        PortDeclaration::Input(_) => {
                            module_builder.entity.push_in_port(gl, ident.into(), key)
                        }
                        PortDeclaration::Output(_) => {
                            module_builder.entity.push_out_port(gl, ident.into(), key)
                        }
                    }
                }
            }
        }
        ModulePorts::Ports(m) => {
            for port in m.iter() {
                let port = arenas.get(port);
                let port = match port {
                    Port::PortExpression(p) => arenas.get(*p),
                };
                let port_references = arenas.get(port.references);
                let port_identifier = port_references.identifier;
                let port = port_identifier.item.0;
                let ident = arenas.get_ident(port);

                defined_ports.insert(ident, defined_ports.len());
            }
        }
    }

    for module_item in module_items.iter() {
        match arenas.get(module_item) {
            ModuleItem::PortDeclaration(port_declaration) => {
                let port_declaration = arenas.get(*port_declaration);

                let (idents, range) = match port_declaration {
                    PortDeclaration::Inout(i) => {
                        let i = arenas.get(*i);
                        (i.port_identifiers, i.range)
                    }
                    PortDeclaration::Input(i) => {
                        let i = arenas.get(*i);
                        (i.port_identifiers, i.range)
                    }
                    PortDeclaration::Output(i) => {
                        let i = arenas.get(*i);
                        (i.identifiers, i.range)
                    }
                };

                let width = match range {
                    None => 1,
                    Some(range) => range_to_width(gl, &mut scope, range, arenas, diagnostics)?,
                };

                for ast_ident in idents.iter() {
                    let ident = arenas.get_ident(arenas.get(ast_ident).0);

                    let key = gl.signals.insert(Signal {
                        name: ident.into(),
                        ty: Type::Bits(width),
                    });
                    let symbol_key = scope.symbols.insert(Symbol {
                        name: ident.to_string(),
                        definition_site: arenas.get_span(ast_ident),
                        ty: Type::Bits(width),
                        variant: SymbolVariant::Signal(key),
                    });
                    scope.push(ident, symbol_key);

                    module_builder.entity.signal(gl, key);
                    match port_declaration {
                        PortDeclaration::Inout(_) => todo!(),
                        PortDeclaration::Input(_) => {
                            module_builder.entity.push_in_port(gl, ident.into(), key)
                        }
                        PortDeclaration::Output(_) => {
                            module_builder.entity.push_out_port(gl, ident.into(), key)
                        }
                    };
                }
            }
            ModuleItem::NonPortModuleItem(p) => match arenas.get(*p) {
                NonPortModuleItem::ModuleOrGenerateItem(id) => match arenas.get(*id) {
                    ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(id) => {
                        let module_or_generate_item_declaration = arenas.get(*id);
                        match module_or_generate_item_declaration {
                            ModuleOrGenerateItemDeclaration::Net(id) => {
                                let net_declaration = arenas.get(*id);
                                let width = match net_declaration.range {
                                    None => 1,
                                    Some(range) => {
                                        range_to_width(gl, &mut scope, range, arenas, diagnostics)?
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
                                            module_builder.entity.signal(gl, key);
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
                                            module_builder.entity.signal(gl, key);

                                            let (section_key, mut bb_builder) =
                                                module_builder.process(gl, "decl_assign".into());
                                            let bb_key = bb_builder.key();
                                            let variable = lower_expr(
                                                &mut bb_builder,
                                                gl,
                                                &mut scope,
                                                arenas.get(*expr),
                                                arenas,
                                                diagnostics,
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
                                    Some(range) => {
                                        range_to_width(gl, &mut scope, range, arenas, diagnostics)?
                                    }
                                };
                                for ast_ident in reg_declaration.identifiers.iter() {
                                    let ident = arenas.get(ast_ident);
                                    let ident = arenas.get_ident(ident.0);
                                    let key = gl.signals.insert(Signal {
                                        name: ident.into(),
                                        ty: Type::Bits(width),
                                    });
                                    let symbol_key = scope.symbols.insert(Symbol {
                                        name: ident.to_string(),
                                        definition_site: arenas.get_span(ast_ident),
                                        ty: Type::Bits(width),
                                        variant: SymbolVariant::Signal(key),
                                    });
                                    scope.push(ident, symbol_key);
                                    module_builder.entity.signal(gl, key);
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
                        }
                    }
                    ModuleOrGenerateItem::LocalParameterDeclaration => todo!(),
                    ModuleOrGenerateItem::ParameterOverride => todo!(),
                    ModuleOrGenerateItem::ContinuousAssign(assign) => {
                        let assign = arenas.get(*assign);
                        for ast_net_assignment in assign.list_of_net_assignments {
                            let net_assignment = arenas.get(ast_net_assignment);

                            let (section_key, mut bb_builder) =
                                module_builder.process(gl, "assign".into());
                            let bb_key = bb_builder.key();
                            let variable = lower_expr(
                                &mut bb_builder,
                                gl,
                                &mut scope,
                                arenas.get(net_assignment.expression),
                                arenas,
                                diagnostics,
                            )?;

                            assign_net_lvalue(
                                &mut bb_builder,
                                gl,
                                &mut scope,
                                net_assignment.net_lvalue,
                                variable,
                                arenas,
                                diagnostics,
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

                                    let (section_key, mut bb_builder) =
                                        module_builder.process(gl, "gate".into());
                                    let bb_key = bb_builder.key();

                                    assert!(!input_terminals.is_empty());
                                    let value = input_terminals.first().unwrap();
                                    let mut value = lower_expr(
                                        &mut bb_builder,
                                        gl,
                                        &mut scope,
                                        arenas.get(value),
                                        arenas,
                                        diagnostics,
                                    )?;
                                    match ninput_gate_instantiation.gatetype.item {
                                        NInputGateType::And | NInputGateType::Nand => {
                                            for input in input_terminals.iter().skip(1) {
                                                let input = lower_expr(
                                                    &mut bb_builder,
                                                    gl,
                                                    &mut scope,
                                                    arenas.get(input),
                                                    arenas,
                                                    diagnostics,
                                                )?;
                                                value = bb_builder.and(gl, value, input);
                                            }
                                        }
                                        NInputGateType::Or | NInputGateType::Nor => {
                                            for input in input_terminals.iter().skip(1) {
                                                let input = lower_expr(
                                                    &mut bb_builder,
                                                    gl,
                                                    &mut scope,
                                                    arenas.get(input),
                                                    arenas,
                                                    diagnostics,
                                                )?;
                                                value = bb_builder.or(gl, value, input);
                                            }
                                        }
                                        NInputGateType::Xor | NInputGateType::Xnor => {
                                            for input in input_terminals.iter().skip(1) {
                                                let input = lower_expr(
                                                    &mut bb_builder,
                                                    gl,
                                                    &mut scope,
                                                    arenas.get(input),
                                                    arenas,
                                                    diagnostics,
                                                )?;
                                                value = bb_builder.xor(gl, value, input);
                                            }
                                        }
                                    };

                                    if matches!(
                                        ninput_gate_instantiation.gatetype.item,
                                        NInputGateType::Nand
                                            | NInputGateType::Nor
                                            | NInputGateType::Xnor
                                    ) {
                                        value = bb_builder.binary_neg(gl, value);
                                    }

                                    let symbol_key = scope.get(ident).unwrap();
                                    let SymbolVariant::Signal(signal_key) =
                                        &scope.symbols[symbol_key].variant
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
                            module_instances,
                        } = arenas.get(*id);
                        let instantiation_ident = arenas.get_ident(module_identifier.item.0);
                        let Ok(instance_module_key) =
                            instantiated_modules.get(instantiation_ident).unwrap()
                        else {
                            diagnostics.not_yet_implemented(
                                arenas.get_item_span(*module_identifier),
                                "module error",
                            );
                            return Err(());
                        };
                        let instance_module_key = *instance_module_key;

                        // let entity = gl.modules.get(*entity).unwrap();

                        // let section_key = *entity
                        //     .sections
                        //     .iter()
                        //     .find(|k| {
                        //         gl.sections.get(**k).unwrap().variant == SectionVariant::Entity
                        //     })
                        //     .unwrap();

                        for instance in module_instances.iter() {
                            let ModuleInstance {
                                name_of_module_instance: _,
                                list_of_port_connections,
                            } = arenas.get(instance);

                            let ports: Vec<SignalKey> = match arenas.get(*list_of_port_connections)
                            {
                                ListOfPortConnections::Ordered(ports) => ports
                                    .iter()
                                    .enumerate()
                                    .map(|(i, p)| {
                                        let connection = gl.modules[instance_module_key]
                                            .io
                                            .get_index(i)
                                            .unwrap()
                                            .1;
                                        let signal = connection.signal;
                                        let is_input = matches!(
                                            connection.direction,
                                            ConnectionDirection::In | ConnectionDirection::Both
                                        );
                                        let ty = gl.signals[signal].ty.clone();
                                        if is_input {
                                            lower_to_signal(
                                                &mut module_builder,
                                                &mut processes,
                                                gl,
                                                p,
                                                &mut scope,
                                                ty,
                                                arenas,
                                                diagnostics,
                                            )
                                        } else {
                                            let Type::Bits(width) = ty else { todo!() };
                                            assign_port_output(
                                                &mut module_builder,
                                                &mut processes,
                                                gl,
                                                p,
                                                &mut scope,
                                                width,
                                                arenas,
                                                diagnostics,
                                            )
                                        }
                                    })
                                    .collect::<Result<Vec<SignalKey>, ()>>()?,
                                ListOfPortConnections::Named(ports) => {
                                    let target_module = &gl.modules[instance_module_key];
                                    if ports.iter().enumerate().all(|(i, p)| {
                                        let named_port_connection = arenas.get(p);
                                        let NamedPortConnection {
                                            port_identifier,
                                            expression: _,
                                        } = *named_port_connection;

                                        let port_ident = arenas.get_ident(port_identifier.item.0);
                                        let Some((j, _, _)) = target_module.io.get_full(port_ident)
                                        else {
                                            diagnostics.port_not_found(
                                                gl,
                                                arenas,
                                                instance_module_key,
                                                port_identifier,
                                            );
                                            return true;
                                        };

                                        if i != j {
                                            diagnostics.not_yet_implemented(
                                                arenas.get_span(p),
                                                "out-of-order named ports",
                                            );
                                            return true;
                                        }

                                        false
                                    }) {
                                        return Err(());
                                    }
                                    ports
                                        .iter()
                                        .enumerate()
                                        .map(|(i, p)| {
                                            let connection = gl.modules[instance_module_key]
                                                .io
                                                .get_index(i)
                                                .unwrap()
                                                .1;
                                            let signal = connection.signal;
                                            let is_input = matches!(
                                                connection.direction,
                                                ConnectionDirection::In | ConnectionDirection::Both
                                            );
                                            let ty = gl.signals[signal].ty.clone();
                                            let named_port_connection = arenas.get(p);
                                            let NamedPortConnection {
                                                port_identifier: _,
                                                expression,
                                            } = *named_port_connection;

                                            match expression {
                                                None => {
                                                    diagnostics.not_yet_implemented(
                                                        arenas.get_span(p),
                                                        "anonymous ports",
                                                    );
                                                    return Err(());
                                                }

                                                Some(e) => {
                                                    if is_input {
                                                        lower_to_signal(
                                                            &mut module_builder,
                                                            &mut processes,
                                                            gl,
                                                            e,
                                                            &mut scope,
                                                            ty,
                                                            arenas,
                                                            diagnostics,
                                                        )
                                                    } else {
                                                        let Type::Bits(width) = ty else { todo!() };
                                                        assign_port_output(
                                                            &mut module_builder,
                                                            &mut processes,
                                                            gl,
                                                            e,
                                                            &mut scope,
                                                            width,
                                                            arenas,
                                                            diagnostics,
                                                        )
                                                    }
                                                }
                                            }
                                        })
                                        .collect::<Result<Vec<SignalKey>, ()>>()?
                                }
                            };
                            module_builder
                                .entity
                                .instantiate(gl, instance_module_key, ports);
                        }
                    }
                    ModuleOrGenerateItem::InitialConstruct(id) => {
                        let statement = arenas.get(*id).0;
                        let (section_key, bb_builder) =
                            module_builder.process(gl, "initial".into());
                        let bb_builder = statements_to_process(
                            bb_builder,
                            gl,
                            &mut scope,
                            std::slice::from_ref(arenas.get(statement)),
                            &arenas,
                            diagnostics,
                        )?;
                        bb_builder.halt(gl);
                        processes.push(section_key);
                    }
                    ModuleOrGenerateItem::AlwaysConstruct(id) => {
                        let statement = arenas.get(*id).0;
                        let (section_key, bb_builder) = module_builder.process(gl, "always".into());
                        let bb_key = bb_builder.key();
                        let bb_builder = statements_to_process(
                            bb_builder,
                            gl,
                            &mut scope,
                            std::slice::from_ref(arenas.get(statement)),
                            &arenas,
                            diagnostics,
                        )?;
                        bb_builder.watch_for_ins_to(gl, bb_key);
                        processes.push(section_key);
                    }
                    ModuleOrGenerateItem::LoopGenerateConstruct => todo!(),
                    ModuleOrGenerateItem::ConditionalGenerateConstruct => todo!(),
                },
                NonPortModuleItem::GenerateRegion => todo!(),
                NonPortModuleItem::SpecifyBlock => todo!(),
                NonPortModuleItem::ParameterDeclaration(id) => {
                    let ParameterDeclaration { assignments } = arenas.get(*id);
                    for assignment in assignments.iter() {
                        let ParamAssignment { param: _, constant } = arenas.get(assignment);
                        let ConstantMinTypMaxExpression::Single(constant) = arenas.get(*constant)
                        else {
                            todo!();
                        };
                        let ConstantExpr::Primary(primary) = arenas.get(*constant);
                        let ConstantPrimary::Number(number) = primary else {
                            todo!();
                        };
                        let Decimal::Small(_) = arenas.decimals[number.at] else {
                            todo!()
                        };
                        todo!()
                        // scope.push(arenas.get_ident(param.item.0), ScopeItem::Constant(v));
                    }
                }
                NonPortModuleItem::SpecParamDeclaration => todo!(),
            },
        }
    }

    for process_key in processes {
        let process = &gl.processes[process_key];
        let mut ports = Vec::with_capacity(process.ins.len() + process.outs.len());
        ports.extend(process.ins.iter().copied());
        ports.extend(process.outs.iter().copied());
        module_builder.entity.spawn(gl, process_key, ports);
    }

    Ok(module_builder.finish(gl))
}

enum WatchCondition {
    None,
    Posedge,
    Negedge,
}

fn statements_to_process<'a>(
    mut builder: BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    stmts: &[Statement],
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
) -> Result<BasicBlockBuilder, ()> {
    for statement in stmts.iter() {
        match statement {
            Statement::BlockingAssignment(ba) => {
                // @Incorrect
                let ba = arenas.get(*ba);
                let BlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control,
                    expression,
                } = ba;
                assert!(delay_or_event_control.is_none());

                let value = lower_expr(
                    &mut builder,
                    gl,
                    scope,
                    arenas.get(*expression),
                    arenas,
                    diagnostics,
                )?;
                assign_variable_lvalue(
                    gl,
                    &mut builder,
                    scope,
                    *variable_lvalue,
                    value,
                    arenas,
                    diagnostics,
                )?;
            }
            Statement::CaseStatement(case_statement) => {
                builder = statement::conditional::lower_case_statement(
                    builder,
                    gl,
                    scope,
                    *case_statement,
                    arenas,
                    diagnostics,
                )?
            }
            Statement::ConditionalStatement(conditional) => {
                builder = statement::conditional::lower(
                    builder,
                    gl,
                    scope,
                    *conditional,
                    arenas,
                    diagnostics,
                )?
            }
            Statement::DisableStatement => todo!(),
            Statement::EventTrigger => todo!(),
            Statement::LoopStatement(ls) => {
                builder = statement::loop_statement::lower_loop_statement(
                    builder,
                    gl,
                    scope,
                    *ls,
                    arenas,
                    diagnostics,
                )?
            }
            Statement::NonBlockingAssignment(nba) => {
                let NonBlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control,
                    expression,
                } = arenas.get(*nba);
                assert!(delay_or_event_control.is_none());

                let value = lower_expr(
                    &mut builder,
                    gl,
                    scope,
                    arenas.get(*expression),
                    arenas,
                    diagnostics,
                )?;
                assign_variable_lvalue(
                    gl,
                    &mut builder,
                    scope,
                    *variable_lvalue,
                    value,
                    arenas,
                    diagnostics,
                )?;
            }
            Statement::ParBlock => todo!(),
            Statement::ProceduralContinuousAssignments => todo!(),
            Statement::ProceduralTimingControlStatement(ptc, statement) => {
                match arenas.get(*ptc) {
                    ProceduralTimingControl::DelayControl(delay_control) => {
                        let delay_control = arenas.get(*delay_control);
                        match delay_control {
                            DelayControl::DelayValue(value) => {
                                let value = match arenas.get(*value) {
                                    DelayValue::UnsignedNumber(value) => {
                                        let value = &arenas.decimals[value.at];
                                        let value = match value {
                                            Decimal::Small(v) => *v as usize,
                                            _ => todo!(),
                                        };
                                        value
                                    }
                                    DelayValue::Identifier(_) => {
                                        todo!()
                                        // let ScopeItem::Constant(v) = scope
                                        //     .get(&arenas.get_ident(value.0))
                                        //     .expect("unknown ident")
                                        // else {
                                        //     todo!();
                                        // };
                                        // *v as usize
                                    }
                                };

                                // @TODO:
                                // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 159
                                //
                                // """
                                // An explicit zero delay (#0) requires that the process be
                                // suspended and added as an inactive event for the current time so
                                // that the process is resumed in the next simulation cycle in the
                                // current time.
                                // """
                                assert_ne!(value, 0);

                                builder = builder.wait(gl, Time(value as u64));
                            }
                        }
                    }
                    ProceduralTimingControl::EventControl(event_control) => {
                        builder = builder.jump(gl);
                        let start_key = builder.key();

                        let mut conditions = Vec::new();
                        let mut signals = Vec::new();
                        match arenas.get(*event_control) {
                            EventControl::EventExpression(event_expression) => {
                                let (expr, condition) = match arenas.get(*event_expression) {
                                    EventExpression::Expression(expr) => {
                                        (expr, WatchCondition::None)
                                    }
                                    EventExpression::Posedge(expr) => {
                                        (expr, WatchCondition::Posedge)
                                    }
                                    EventExpression::Negedge(expr) => {
                                        (expr, WatchCondition::Negedge)
                                    }
                                    EventExpression::OrList(_, _) => todo!(),
                                };

                                let Expr::Ident(ident) = arenas.get(*expr) else {
                                    panic!("not an ident");
                                };
                                let ident = arenas.get_ident(ident.item.0);
                                let symbol_key = scope.get(ident).unwrap();
                                let SymbolVariant::Signal(key) = &scope.symbols[symbol_key].variant
                                else {
                                    panic!("not a signal");
                                };

                                conditions.push((condition, *key));
                                signals.push(*key);
                            }
                        }

                        let mut before = Vec::new();
                        for (_, signal) in &conditions {
                            before.push(builder.probe(gl, *signal));
                        }

                        builder = builder.watch(gl, signals);

                        let mut acc = builder.constant(gl, Value::Bits(Bits::Small(1, 1)));
                        for ((condition, signal), before) in conditions.into_iter().zip(before) {
                            use WatchCondition as C;

                            let cond = match condition {
                                C::Posedge => {
                                    let after = builder.probe(gl, signal);
                                    let t = builder.binary_neg(gl, before);
                                    builder.and(gl, t, after)
                                }
                                C::Negedge => {
                                    let after = builder.probe(gl, signal);
                                    let t = builder.binary_neg(gl, after);
                                    builder.and(gl, before, t)
                                }
                                C::None => builder.constant(gl, Value::Bits(Bits::Small(1, 1))),
                            };
                            acc = builder.and(gl, acc, cond);
                        }

                        builder = builder.branch_false_to(gl, acc, start_key);
                    }
                }

                if let Some(stmt) = statement {
                    let stmt = arenas.get(*stmt);
                    builder = statements_to_process(
                        builder,
                        gl,
                        scope,
                        std::slice::from_ref(stmt),
                        arenas,
                        diagnostics,
                    )?;
                }
            }
            Statement::SeqBlock(id) => {
                let seq_block = arenas.get(*id);
                let statements = seq_block
                    .statements
                    .iter()
                    .map(|v| arenas.get(v).clone())
                    .collect::<Vec<_>>();
                builder =
                    statements_to_process(builder, gl, scope, &statements, arenas, diagnostics)?;
            }
            Statement::SystemTaskEnable(id) => {
                let system_task_enable = arenas.get(*id);

                let ident = system_task_enable.system_task_identifier.item;
                let ident = &arenas.text[ident.0.start..ident.0.end];

                match ident {
                    "display" => {
                        let expressions = system_task_enable.expressions;
                        assert_eq!(expressions.len(), 1); // @Improve: Error message

                        let expr = arenas.get(expressions.first().unwrap());
                        let arg = if let Some(str_literal) = expr.into_str_literal() {
                            let str_literal = &arenas.text[str_literal.0.start..str_literal.0.end];
                            IntrinsicArg::StringLiteral(str_literal.to_string())
                        } else {
                            let var =
                                lower_expr(&mut builder, gl, scope, expr, arenas, diagnostics)?;
                            IntrinsicArg::Variable(var)
                        };

                        builder.intrinsic(gl, IntrinsicOp::Display, vec![arg]);
                    }
                    "vogls_assert_eq" | "vogls_assert_ne" => {
                        let expressions = system_task_enable.expressions;
                        assert_eq!(expressions.len(), 2); // @Improve: Error message

                        let lhs = expressions.get(0);
                        let rhs = expressions.get(1);

                        let lhs = arenas.get(lhs);
                        let rhs = arenas.get(rhs);

                        let lhs = lower_expr(&mut builder, gl, scope, lhs, arenas, diagnostics)?;
                        let rhs = lower_expr(&mut builder, gl, scope, rhs, arenas, diagnostics)?;

                        let (lhs, rhs) = builder.coerce_binary_bitwise_srcs(gl, lhs, rhs);

                        builder.intrinsic(
                            gl,
                            IntrinsicOp::AssertEq(ident == "vogls_assert_eq"),
                            vec![IntrinsicArg::Variable(lhs), IntrinsicArg::Variable(rhs)],
                        )
                    }
                    "finish" => builder.intrinsic(gl, IntrinsicOp::Finish, vec![]),

                    // @Incomplete: Many variants here.
                    _ => todo!(),
                }
            }
            Statement::TaskEnable => todo!(),
            Statement::WaitStatement => todo!(),
        }
    }

    Ok(builder)
}

fn add_var_assign_intersect_symbols_generated<'a>(
    _gl: &mut GlobalContext,
    scope: &Scope<'a>,
    var_assign: AstId<VariableAssignment>,
    arenas: &'a AstArenas,
    black_list: &mut HashSet<&'a str>,
    symbol_keys: &mut Vec<SymbolKey>,
) {
    let va = arenas.get(var_assign);
    let lvalue = arenas.get(va.lvalue);
    let ident = arenas.get_ident(lvalue.ident.item.0);
    if black_list.insert(ident) {
        symbol_keys.push(scope.get(ident).unwrap());
    }
}

fn get_intersect_symbols_generated<'a>(
    gl: &mut GlobalContext,
    scope: &Scope<'a>,
    stmts: AstIdRange<Statement>,
    arenas: &'a AstArenas,
) -> Vec<SymbolKey> {
    let mut symbols = Vec::new();
    let mut black_list = HashSet::<&str>::new();
    let mut stack = Vec::new();
    stack.push(stmts);
    while let Some(mut stmts) = stack.pop() {
        while let Some(stmt) = stmts.pop_front() {
            let stmt = arenas.get(stmt);
            match stmt {
                Statement::BlockingAssignment(id) => {
                    let ba = arenas.get(*id);
                    let lvalue = arenas.get(ba.variable_lvalue);
                    let ident = arenas.get_ident(lvalue.ident.item.0);
                    if black_list.insert(ident) {
                        symbols.push(scope.get(ident).unwrap());
                    }
                }
                Statement::CaseStatement(id) => {
                    let c = arenas.get(*id);
                    stack.push(stmts);
                    stack.extend(c.items.iter().filter_map(|c| {
                        match arenas.get(arenas.get(c).statement_or_null) {
                            StatementOrNull::Attribute(_) => None,
                            StatementOrNull::Statement(stmt) => Some(AstIdRange::single(*stmt)),
                        }
                    }));
                    break;
                }
                Statement::ConditionalStatement(id) => {
                    let c = arenas.get(*id);
                    stack.push(stmts);
                    match arenas.get(c.if_branch.statement) {
                        StatementOrNull::Attribute(_) => {}
                        StatementOrNull::Statement(stmt) => stack.push(AstIdRange::single(*stmt)),
                    }
                    stack.extend(c.else_ifs.iter().filter_map(|c| {
                        match arenas.get(arenas.get(c).statement) {
                            StatementOrNull::Attribute(_) => None,
                            StatementOrNull::Statement(stmt) => Some(AstIdRange::single(*stmt)),
                        }
                    }));
                    if let Some(else_branch) = c.else_branch {
                        match arenas.get(else_branch) {
                            StatementOrNull::Attribute(_) => {}
                            StatementOrNull::Statement(stmt) => {
                                stack.push(AstIdRange::single(*stmt))
                            }
                        }
                    }
                    break;
                }
                Statement::DisableStatement => todo!(),
                Statement::EventTrigger => todo!(),
                Statement::LoopStatement(id) => {
                    let ls = arenas.get(*id);
                    if let LoopStatementVariant::For(init, _, step) = &ls.variant {
                        add_var_assign_intersect_symbols_generated(
                            gl,
                            scope,
                            *init,
                            arenas,
                            &mut black_list,
                            &mut symbols,
                        );
                        add_var_assign_intersect_symbols_generated(
                            gl,
                            scope,
                            *step,
                            arenas,
                            &mut black_list,
                            &mut symbols,
                        );
                    }
                    stack.push(stmts);
                    stack.push(AstIdRange::single(ls.statement));
                    break;
                }
                Statement::NonBlockingAssignment(id) => {
                    let nba = arenas.get(*id);
                    let lvalue = arenas.get(nba.variable_lvalue);
                    let ident = arenas.get_ident(lvalue.ident.item.0);
                    if black_list.insert(ident) {
                        symbols.push(scope.get(ident).unwrap());
                    }
                }
                Statement::ParBlock => todo!(),
                Statement::ProceduralContinuousAssignments => todo!(),
                Statement::ProceduralTimingControlStatement(_, statement) => {
                    if let Some(statement) = statement {
                        stack.push(stmts);
                        stack.push(AstIdRange::single(*statement));
                        break;
                    }
                }
                Statement::SeqBlock(id) => {
                    let seq_block = arenas.get(*id);
                    stack.push(stmts);
                    stack.push(seq_block.statements);
                    break;
                }
                Statement::SystemTaskEnable(_) => continue,
                Statement::TaskEnable => todo!(),
                Statement::WaitStatement => todo!(),
            }
        }
    }
    symbols
}

fn lower_to_signal<'a>(
    module_builder: &mut ModuleBuilder,
    processes: &mut Vec<ProcessKey>,
    gl: &mut GlobalContext,
    expr: AstId<Expr>,
    scope: &mut Scope<'a>,
    ty: Type,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
) -> Result<SignalKey, ()> {
    if let Expr::Ident(ast_ident) = arenas.get(expr) {
        let ident = arenas.get_ident(ast_ident.item.0);
        let Some(symbol_key) = scope.get(&ident) else {
            diagnostics.var_not_found(arenas, *ast_ident);
            return Err(());
        };
        if let SymbolVariant::Signal(key) = &scope.symbols[symbol_key].variant {
            return Ok(*key);
        }
    }

    let signal = gl.signals.insert(Signal {
        name: "anon_port_assignment".to_string(),
        ty: ty.clone(),
    });
    module_builder.entity.signal(gl, signal);

    let (section_key, mut bb_builder) = module_builder.process(gl, "port_assignment".into());
    let bb_key = bb_builder.key();
    let variable = lower_expr(
        &mut bb_builder,
        gl,
        scope,
        arenas.get(expr),
        arenas,
        diagnostics,
    )?;

    bb_builder.drive(gl, signal, variable);

    bb_builder.watch_for_ins_to(gl, bb_key);
    processes.push(section_key);
    Ok(signal)
}

fn assign_port_output<'a>(
    module_builder: &mut ModuleBuilder,
    processes: &mut Vec<ProcessKey>,
    gl: &mut GlobalContext,
    expr: AstId<Expr>,
    scope: &mut Scope<'a>,
    width: VectorSize,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
) -> Result<SignalKey, ()> {
    if let Expr::Ident(ast_ident) = arenas.get(expr) {
        let ident = arenas.get_ident(ast_ident.item.0);
        let Some(symbol_key) = scope.get(&ident) else {
            diagnostics.var_not_found(arenas, *ast_ident);
            return Err(());
        };
        if let SymbolVariant::Signal(key) = &scope.symbols[symbol_key].variant {
            return Ok(*key);
        }
    }

    let mut driving: Vec<(
        AstId<Expr>,
        Option<VariableKey>,
        VectorSize,
        Option<VariableKey>,
        VectorSize,
    )> = Vec::new();
    driving.push((expr, None, width, None, width));

    let signal = gl.signals.insert(Signal {
        name: "anon_port_assignment".to_string(),
        ty: Type::Bits(width),
    });
    module_builder.entity.signal(gl, signal);

    let (section_key, mut bb_builder) = module_builder.process(gl, "port_assignment".into());
    let bb_key = bb_builder.key();

    let probed = bb_builder.probe(gl, signal);

    let mut error = false;
    while let Some((expr, offset_src, length_src, offset_dst, length_dst)) = driving.pop() {
        match arenas.get(expr) {
            Expr::BitPartSelect(bit_part_select) => {
                let BitPartSelect { subject, braced } = bit_part_select;
                let offset = lower_expr(
                    &mut bb_builder,
                    gl,
                    scope,
                    arenas.get(*braced),
                    arenas,
                    diagnostics,
                )?;

                let offset_dst = match offset_dst {
                    None => offset,
                    Some(offset_dst) => bb_builder.plus(gl, offset_dst, offset),
                };
                let length_dst = 1;

                driving.push((
                    *subject,
                    offset_src,
                    length_src,
                    Some(offset_dst),
                    length_dst,
                ));
            }
            Expr::BitSlice(subject, slice) => {
                let (offset, length) = match slice {
                    BitSlice::MsbLsb(msb, lsb) => {
                        let (_, lsb, width) =
                            msb_lsb_to_width(gl, scope, *msb, *lsb, arenas, diagnostics)?;
                        let offset = bb_builder.constant(gl, Value::Decimal(lsb as i64));
                        (offset, width as VectorSize)
                    }
                    BitSlice::PlusWidth(base, width) => {
                        let offset = lower_expr(
                            &mut bb_builder,
                            gl,
                            scope,
                            arenas.get(*base),
                            arenas,
                            diagnostics,
                        );
                        let width = eval_constant_expr(gl, scope, *width, arenas, diagnostics);
                        (offset?, width? as VectorSize)
                    }
                    BitSlice::MinusWidth(base, width) => {
                        let offset = lower_expr(
                            &mut bb_builder,
                            gl,
                            scope,
                            arenas.get(*base),
                            arenas,
                            diagnostics,
                        );
                        let width = eval_constant_expr(gl, scope, *width, arenas, diagnostics)?
                            as VectorSize;
                        let width_v = bb_builder.constant(gl, Value::Decimal((width + 1) as i64));
                        let offset = bb_builder.minus(gl, offset?, width_v);
                        (offset, width)
                    }
                };

                let offset_dst = match offset_dst {
                    None => offset,
                    Some(offset_dst) => bb_builder.plus(gl, offset_dst, offset),
                };
                let length_dst = length;

                driving.push((
                    *subject,
                    offset_src,
                    length_src,
                    Some(offset_dst),
                    length_dst,
                ));
            }
            Expr::Concatenation(_) => {
                todo!()
            }
            Expr::Ident(ast_ident) => {
                let ident = arenas.get_ident(ast_ident.item.0);
                let Some(symbol_key) = scope.get(&ident) else {
                    diagnostics.var_not_found(arenas, *ast_ident);
                    error = true;
                    continue;
                };
                let SymbolVariant::Signal(key) = &scope.symbols[symbol_key].variant else {
                    diagnostics.output_expr_not_allowed(arenas.get_span(expr));
                    error = true;
                    continue;
                };

                let offset_dst = match offset_dst {
                    None => bb_builder.constant(gl, Value::Decimal(0)),
                    Some(v) => v,
                };

                let mut src = probed;
                if let Some(offset_src) = offset_src {
                    src = bb_builder.lsr(gl, src, offset_src);
                }
                let src = bb_builder.slice(gl, src, length_src);
                bb_builder.drive_partial(gl, *key, src, offset_dst, length_dst);
            }

            Expr::Replication(_) => {
                diagnostics.not_yet_implemented(arenas.get_span(expr), "repetition in net assign");
                error = true;
            }

            Expr::Decimal(..)
            | Expr::Sized(..)
            | Expr::Ternary(..)
            | Expr::String(..)
            | Expr::Unary(..)
            | Expr::Binary(..) => {
                diagnostics.output_expr_not_allowed(arenas.get_span(expr));
                error = true;
            }
        }
    }

    bb_builder.watch_for_ins_to(gl, bb_key);
    processes.push(section_key);

    if error {
        return Err(());
    }

    Ok(signal)
}

fn expr_to_type<'a>(
    gl: &mut GlobalContext,
    scope: &Scope<'a>,
    expr: AstId<Expr>,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
) -> Result<Type, ()> {
    Ok(match arenas.get(expr) {
        Expr::BitPartSelect(select) => {
            let BitPartSelect { subject, braced } = select;
            let subject_v = expr_to_type(gl, scope, *subject, arenas, diagnostics)?;
            let braced_v = expr_to_type(gl, scope, *braced, arenas, diagnostics)?;
            Type::Bits(1)
        }
        Expr::BitSlice(subject, slice) => {
            let subject_v = expr_to_type(gl, scope, *subject, arenas, diagnostics)?;

            match slice {
                BitSlice::MsbLsb(msb, lsb) => {
                    let (_, _, width) =
                        msb_lsb_to_width(gl, scope, *msb, *lsb, arenas, diagnostics)?;
                    Type::Bits(width)
                }
                BitSlice::PlusWidth(_base, width) => {
                    let width = eval_constant_expr(gl, scope, *width, arenas, diagnostics)?;
                    Type::Bits(width as VectorSize)
                }
                BitSlice::MinusWidth(_base, width) => {
                    let width = eval_constant_expr(gl, scope, *width, arenas, diagnostics)?;
                    Type::Bits(width as VectorSize)
                }
            }
        }
        Expr::Unary(op, child) => {
            let child = expr_to_type(gl, scope, *child, arenas, diagnostics)?;
            use UnaryOperator as O;
            match op {
                O::LogicalNegation | O::BitwiseNegation => child,
                O::ReductionAnd
                | O::ReductionOr
                | O::ReductionNand
                | O::ReductionNor
                | O::ReductionXor
                | O::ReductionXnor => Type::Bits(1),
                O::SignPlus => todo!(),
                O::SignMinus => todo!(),
            }
        }
        Expr::Binary(op, l, r) => {
            let l = expr_to_type(gl, scope, *l, arenas, diagnostics)?;
            let r = expr_to_type(gl, scope, *r, arenas, diagnostics)?;
            _ = (l, r);
            use BinaryOperator as O;
            match op {
                O::Multiply => todo!(),
                O::Divide => todo!(),
                O::Modulus => todo!(),
                O::BinaryPlus => todo!(),
                O::BinaryMinus => todo!(),
                O::ShiftLeft => todo!(),
                O::ShiftRight => todo!(),
                O::GreaterThan => todo!(),
                O::GreaterThanEqual => todo!(),
                O::LessThan => todo!(),
                O::LessThanEqual => todo!(),
                O::LogicalEquality => todo!(),
                O::LogicalInequality => todo!(),
                O::CaseEquality => todo!(),
                O::CaseInequality => todo!(),
                O::BitwiseAnd => todo!(),
                O::BitwiseXor => todo!(),
                O::BitwiseXnor => todo!(),
                O::BitwiseOr => todo!(),
                O::LogicalAnd => todo!(),
                O::LogicalOr => todo!(),
            }
        }
        Expr::Concatenation(exprs) => {
            let mut width = 0;
            let mut error = false;
            for expr in exprs.iter() {
                match expr_to_type(gl, scope, expr, arenas, diagnostics)
                    .and_then(|t| t.try_net_width())
                {
                    Ok(ew) => width += ew,
                    Err(_) => error = true,
                }
            }
            if error {
                return Err(());
            }
            Type::Bits(width)
        }
        Expr::Replication(_) => todo!(),
        Expr::Ternary(_, _, _) => todo!(),
        Expr::Ident(ast_ident) => {
            let ident = arenas.get_ident(ast_ident.item.0);
            let Some(symbol_key) = scope.get(&ident) else {
                diagnostics.var_not_found(arenas, *ast_ident);
                return Err(());
            };
            scope.symbols[symbol_key].ty.clone()
        }
        Expr::Decimal(_) => Type::Decimal,
        Expr::Sized(sized) => {
            let sized = &arenas.sized_numbers[sized.item.at];
            let Some(size) = sized.size else { todo!() };
            Type::Bits(size.as_u32())
        }
        Expr::String(_) => todo!(),
    })
}

fn lower_expr<'a>(
    builder: &mut BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    expr: &Expr,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
) -> Result<VariableKey, ()> {
    Ok(match expr {
        Expr::BitPartSelect(select) => {
            let BitPartSelect { subject, braced } = select;
            let subject = arenas.get(*subject);
            let braced = arenas.get(*braced);

            let subject_v = lower_expr(builder, gl, scope, subject, arenas, diagnostics)?;
            let braced_v = lower_expr(builder, gl, scope, braced, arenas, diagnostics)?;

            builder.select_bit(gl, subject_v, braced_v)
        }
        Expr::BitSlice(subject, slice) => {
            let subject = arenas.get(*subject);
            let subject_v = lower_expr(builder, gl, scope, subject, arenas, diagnostics)?;

            let (lsb, width) = match slice {
                BitSlice::MsbLsb(msb, lsb) => {
                    let (_msb, lsb, width) =
                        msb_lsb_to_width(gl, scope, *msb, *lsb, arenas, diagnostics)?;
                    let lsb_v = builder.constant(gl, Value::Decimal(lsb as i64));
                    (lsb_v, width)
                }
                BitSlice::PlusWidth(base, width) => {
                    let lsb =
                        lower_expr(builder, gl, scope, arenas.get(*base), arenas, diagnostics)?;
                    let width =
                        eval_constant_expr(gl, scope, *width, arenas, diagnostics)? as VectorSize;
                    (lsb, width)
                }
                BitSlice::MinusWidth(base, width) => {
                    let width = eval_constant_expr(gl, scope, *width, arenas, diagnostics)?;
                    let width_v = builder.constant(gl, Value::Decimal(width as i64 - 1));
                    let lsb =
                        lower_expr(builder, gl, scope, arenas.get(*base), arenas, diagnostics)?;
                    let lsb = builder.minus(gl, lsb, width_v);
                    (lsb, width as VectorSize)
                }
            };

            let shifted = builder.lsr(gl, subject_v, lsb);
            builder.slice(gl, shifted, width as VectorSize)
        }
        Expr::Unary(op, child) => {
            let child = lower_expr(builder, gl, scope, arenas.get(*child), arenas, diagnostics)?;
            use UnaryOperator as O;
            match op {
                O::LogicalNegation => builder.logical_neg(gl, child),
                O::BitwiseNegation => builder.binary_neg(gl, child),
                O::ReductionAnd => todo!(),
                O::ReductionOr => todo!(),
                O::ReductionNand => todo!(),
                O::ReductionNor => todo!(),
                O::ReductionXor => builder.reduce_xor(gl, child),
                O::ReductionXnor => todo!(),
                O::SignPlus => todo!(),
                O::SignMinus => todo!(),
            }
        }
        Expr::Binary(op, l, r) => {
            let l = lower_expr(builder, gl, scope, arenas.get(*l), arenas, diagnostics)?;
            let r = lower_expr(builder, gl, scope, arenas.get(*r), arenas, diagnostics)?;
            use BinaryOperator as O;
            match op {
                O::Multiply => builder.multiply(gl, l, r),
                O::Divide => todo!(),
                O::Modulus => todo!(),
                O::BinaryPlus => builder.plus(gl, l, r),
                O::BinaryMinus => builder.minus(gl, l, r),
                O::ShiftLeft => todo!(),
                O::ShiftRight => todo!(),
                O::GreaterThan => builder.unsigned_gt(gl, l, r),
                O::GreaterThanEqual => builder.unsigned_ge(gl, l, r),
                O::LessThan => builder.unsigned_lt(gl, l, r),
                O::LessThanEqual => builder.unsigned_le(gl, l, r),
                O::LogicalEquality => builder.equals(gl, l, r),
                O::LogicalInequality => todo!(),
                O::CaseEquality => todo!(),
                O::CaseInequality => todo!(),
                O::BitwiseAnd => builder.and(gl, l, r),
                O::BitwiseXor => builder.xor(gl, l, r),
                O::BitwiseXnor => builder.xnor(gl, l, r),
                O::BitwiseOr => builder.or(gl, l, r),
                O::LogicalAnd => todo!(),
                O::LogicalOr => todo!(),
            }
        }
        Expr::Concatenation(exprs) => {
            let Some(fst) = exprs.first() else {
                return Ok(builder.constant(gl, Value::Bits(Bits::Small(0, 0))));
            };

            let mut output = lower_expr(builder, gl, scope, arenas.get(fst), arenas, diagnostics)?;
            for expr in exprs.iter().skip(1) {
                let lexpr = lower_expr(builder, gl, scope, arenas.get(expr), arenas, diagnostics)?;
                output = builder.concat(gl, output, lexpr);
            }
            output
        }
        Expr::Replication(_) => todo!(),
        Expr::Ternary(_, _, _) => todo!(),
        Expr::Ident(ast_ident) => {
            let ident = arenas.get_ident(ast_ident.item.0);
            let Some(symbol_key) = scope.get(&ident) else {
                diagnostics.var_not_found(arenas, *ast_ident);
                return Err(());
            };
            match &scope.symbols[symbol_key].variant {
                SymbolVariant::Signal(key) => builder.probe(gl, *key),
                SymbolVariant::Variable(None) => todo!(),
                SymbolVariant::Variable(Some(key)) => *key,
            }
        }
        Expr::Decimal(decimal) => {
            let decimal = &arenas.decimals[decimal.at];
            let decimal = match decimal {
                Decimal::Small(v) => *v as i64,
                _ => todo!(),
            };

            builder.constant(gl, Value::Decimal(decimal))
        }
        Expr::Sized(sized) => {
            let sized = &arenas.sized_numbers[sized.item.at];
            let Some(size) = sized.size else { todo!() };
            let crate::number::Bits::Small(v) = sized.value else {
                todo!()
            };
            builder.constant(gl, Value::Bits(Bits::Small(v, size.as_u32())))
        }
        Expr::String(_) => todo!(),
    })
}

fn assign_variable_lvalue<'a>(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    scope: &mut Scope<'a>,
    lvalue: AstId<VariableLValue>,
    variable: VariableKey,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let VariableLValue {
        ident,
        exprs,
        range_expression,
    } = arenas.get(lvalue);

    let lvalue_ident = arenas.get_ident(ident.item.0);
    let Some(symbol_key) = scope.get(&lvalue_ident) else {
        diagnostics.var_not_found(arenas, *ident);
        return Err(());
    };

    if !exprs.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_range_span(*exprs), "variable_lvalue::exprs");
        return Err(());
    }

    match &mut scope.symbols[symbol_key].variant {
        SymbolVariant::Signal(key) => {
            let key = *key;
            match range_expression {
                None => {
                    if &gl.signals[key].ty != &gl.vars[variable].ty {
                        diagnostics.warn_assign_type_mismatch(
                            arenas.get_span(lvalue),
                            gl.signals[key].ty.clone(),
                            gl.vars[variable].ty.clone(),
                        );
                    }
                    builder.drive(gl, key, variable)
                }
                Some(range_expression) => {
                    let (offset, length) = match arenas.get(*range_expression) {
                        RangeExpression::Expr(expr) => (
                            lower_expr(builder, gl, scope, expr, arenas, diagnostics)?,
                            1,
                        ),
                        RangeExpression::MsbLsb(_, _) => todo!("MsbLsb"),
                        RangeExpression::BasePlus(_, _) => todo!("BasePlus"),
                        RangeExpression::BaseMinus(_, _) => todo!("BaseMinus"),
                    };

                    if Type::Bits(length) != gl.vars[variable].ty {
                        diagnostics.warn_assign_type_mismatch(
                            arenas.get_span(lvalue),
                            Type::Bits(length),
                            gl.vars[variable].ty.clone(),
                        );
                    }

                    builder.drive_partial(gl, key, variable, offset, length);
                }
            }
        }
        SymbolVariant::Variable(v) => {
            if let Some(range_expression) = range_expression {
                diagnostics.not_yet_implemented(
                    arenas.get_span(*range_expression),
                    "variable_lvalue::range_expression[variable]",
                );
                return Err(());
            }

            *v = Some(variable);
            scope.assign(symbol_key, variable);
        }
    }
    Ok(())
}

fn assign_net_lvalue<'a>(
    builder: &mut BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    lvalue: AstId<NetLValue>,
    variable: VariableKey,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let lvalue = arenas.get(lvalue);
    let lvalue_ident = lvalue.ident.item;

    let ident = arenas.get_ident(lvalue_ident.0);
    let Some(symbol_key) = scope.get(ident) else {
        diagnostics.var_not_found(arenas, lvalue.ident);
        return Err(());
    };

    let SymbolVariant::Signal(signal_key) = &scope.symbols[symbol_key].variant else {
        panic!("not a signal");
    };
    let signal_key = *signal_key;

    if !lvalue.constant_exprs.is_empty() {
        diagnostics.not_yet_implemented(
            arenas.get_range_span(lvalue.constant_exprs),
            "net_lvalue::constant_exprs",
        );
        return Err(());
    }
    match lvalue.constant_range_expression {
        None => builder.drive(gl, signal_key, variable),
        Some(range_expression) => {
            let (offset, length) = match arenas.get(range_expression) {
                ConstantRangeExpression::Single(expr) => (
                    eval_constant_expr(gl, scope, *expr, arenas, diagnostics)?,
                    1,
                ),
                ConstantRangeExpression::MsbLsb { msb, lsb } => {
                    let (_, offset, length) =
                        msb_lsb_to_width(gl, scope, *msb, *lsb, arenas, diagnostics)?;
                    (offset, length)
                }
            };

            let offset = builder.constant(gl, Value::Decimal(offset as i64));
            builder.drive_partial(gl, signal_key, variable, offset, length);
        }
    }

    Ok(())
}

fn eval_constant_expr<'a>(
    _gl: &mut GlobalContext,
    _scope: &Scope<'a>,
    expr: AstId<ConstantExpr>,
    arenas: &'a AstArenas,
    _diagnostics: &mut Diagnostics,
) -> Result<u64, ()> {
    let expr = arenas.get(expr);
    let ConstantExpr::Primary(primary) = expr;
    let ConstantPrimary::Number(number) = primary else {
        todo!();
    };
    let Decimal::Small(v) = arenas.decimals[number.at] else {
        todo!()
    };
    Ok(v)
}

fn msb_lsb_to_width<'a>(
    gl: &mut GlobalContext,
    scope: &Scope<'a>,
    msb: AstId<ConstantExpr>,
    lsb: AstId<ConstantExpr>,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
) -> Result<(u64, u64, VectorSize), ()> {
    let msb = eval_constant_expr(gl, scope, msb, arenas, diagnostics);
    let lsb = eval_constant_expr(gl, scope, lsb, arenas, diagnostics);

    let (Ok(msb), Ok(lsb)) = (msb, lsb) else {
        return Err(());
    };
    Ok((msb, lsb, (msb - lsb + 1) as VectorSize))
}

fn range_to_width<'a>(
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    range: AstId<Range>,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
) -> Result<u32, ()> {
    let range = arenas.get(range);
    msb_lsb_to_width(gl, scope, range.msb, range.lsb, arenas, diagnostics).map(|(_, _, w)| w)
}
