mod scope;
mod statement;

use std::collections::{HashMap, HashSet};

use scope::Scope;

use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, IntrinsicArg, IntrinsicOp, ModuleBuilder, ModuleKey,
    Signal, SignalKey, Time, Type, Value, VariableKey,
};

use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression, ConstantPrimary};
use crate::ast::expr::{BinaryOperator, BitPartSelect, Expr, UnaryOperator};
use crate::ast::module::{
    GateInstantiation, ListOfPortConnections, Module, ModuleInstance, ModuleInstantiation,
    ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration, ModulePorts,
    NInputGateInstance, NInputGateType, NetDeclAssignment, NetDeclarationNets, NonPortModuleItem,
    ParamAssignment, ParameterDeclaration, Port, PortDeclaration,
};
use crate::ast::statement::{
    BlockingAssignment, DelayControl, DelayValue, EventControl, EventExpression,
    NonBlockingAssignment, ProceduralTimingControl, Statement, VariableLValue,
};
use crate::ast::{AstId, AstIdRange};
use crate::number::Decimal;
use crate::parser::{Ast, AstArenas};

use self::scope::{Symbol, SymbolKey, SymbolVariant};

pub fn lower_module_to_ir(
    ast: &Ast,
    root: AstId<Module>,
    gl: &mut GlobalContext,
    instantiated_modules: &HashMap<&str, ModuleKey>,
) -> ModuleKey {
    let Ast {
        modules: _,
        arenas,
        path: _,
    } = ast;

    let Module {
        module_identifier,
        ports,
        module_items,
    } = arenas.get(root);

    let module_identifier = arenas.get_ident(module_identifier.item.0);
    let mut module_builder = ModuleBuilder::new(module_identifier.to_string(), gl);
    let mut scope = Scope::new();
    let mut processes = Vec::new();

    match ports {
        ModulePorts::PortDeclarations(m) => {
            for port_declaration in m.iter() {
                let port_declaration = arenas.get(port_declaration);

                let idents = match port_declaration {
                    PortDeclaration::Inout(i) => arenas.get(*i).port_identifiers,
                    PortDeclaration::Input(i) => arenas.get(*i).port_identifiers,
                    PortDeclaration::Output(i) => arenas.get(*i).identifiers,
                };

                for ast_ident in idents.iter() {
                    let ident = arenas.get_ident(arenas.get(ast_ident).0);

                    let key = gl.signals.insert(Signal {
                        name: ident.into(),
                        ty: Type::Bits(1),
                    });
                    let symbol_key = scope.symbols.insert(Symbol {
                        name: ident.to_string(),
                        definition_site: arenas.get_span(ast_ident),
                        ty: Type::Bits(1),
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
                let port_identifier = arenas.get(port.references).identifier;
                let port = port_identifier.item.0;
                let ident = arenas.get_ident(port);

                let key = gl.signals.insert(Signal {
                    name: ident.into(),
                    ty: Type::Bits(1),
                });
                let symbol_key = scope.symbols.insert(Symbol {
                    name: ident.to_string(),
                    definition_site: arenas.get_item_span(port_identifier),
                    ty: Type::Bits(1),
                    variant: SymbolVariant::Signal(key),
                });
                scope.push(ident, symbol_key);

                module_builder.entity.signal(gl, key);
            }
        }
    }

    for module_item in module_items.iter() {
        match arenas.get(module_item) {
            ModuleItem::PortDeclaration(port_declaration) => {
                let port_declaration = arenas.get(*port_declaration);

                let idents = match port_declaration {
                    PortDeclaration::Inout(i) => arenas.get(*i).port_identifiers,
                    PortDeclaration::Input(i) => arenas.get(*i).port_identifiers,
                    PortDeclaration::Output(i) => arenas.get(*i).identifiers,
                };

                for ast_ident in idents.iter() {
                    let ident = arenas.get_ident(arenas.get(ast_ident).0);

                    let key = gl.signals.insert(Signal {
                        name: ident.into(),
                        ty: Type::Bits(1),
                    });
                    let symbol_key = scope.symbols.insert(Symbol {
                        name: ident.to_string(),
                        definition_site: arenas.get_span(ast_ident),
                        ty: Type::Bits(1),
                        variant: SymbolVariant::Signal(key),
                    });
                    scope.push(ident, symbol_key);

                    module_builder.entity.signal(gl, key);
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
                                        let range = arenas.get(range);
                                        let msb = eval_constant_expr(
                                            gl,
                                            &mut scope,
                                            arenas.get(range.msb),
                                            arenas,
                                        );
                                        let lsb = eval_constant_expr(
                                            gl,
                                            &mut scope,
                                            arenas.get(range.lsb),
                                            arenas,
                                        );
                                        msb - lsb + 1
                                    }
                                } as u32;
                                match net_declaration.nets {
                                    NetDeclarationNets::Idents(identifiers) => {
                                        for ast_ident in identifiers.iter() {
                                            let ident = arenas.get(ast_ident);
                                            let ident = arenas.get_ident(ident.0);
                                            let ty = Type::Bits(width);
                                            let key = gl.signals.insert(Signal {
                                                name: ident.into(),
                                                ty: ty.clone(),
                                            });
                                            let symbol_key = scope.symbols.insert(Symbol {
                                                name: ident.to_string(),
                                                definition_site: arenas.get_span(ast_ident),
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
                                            );

                                            bb_builder.drive(gl, key, variable);
                                            bb_builder.watch_for_ins_to(gl, bb_key);
                                            processes.push(section_key);
                                        }
                                    }
                                }
                            }
                            ModuleOrGenerateItemDeclaration::Reg(id) => {
                                let reg_declaration = arenas.get(*id);
                                for ast_ident in reg_declaration.identifiers.iter() {
                                    let ident = arenas.get(ast_ident);
                                    let ident = arenas.get_ident(ident.0);
                                    let key = gl.signals.insert(Signal {
                                        name: ident.into(),
                                        ty: Type::Bits(1),
                                    });
                                    let symbol_key = scope.symbols.insert(Symbol {
                                        name: ident.to_string(),
                                        definition_site: arenas.get_span(ast_ident),
                                        ty: Type::Bits(1),
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
                        for net_assignment in assign.list_of_net_assignments {
                            let net_assignment = arenas.get(net_assignment);

                            let (section_key, mut bb_builder) =
                                module_builder.process(gl, "assign".into());
                            let bb_key = bb_builder.key();
                            let variable = lower_expr(
                                &mut bb_builder,
                                gl,
                                &mut scope,
                                arenas.get(net_assignment.expression),
                                arenas,
                            );

                            let lvalue = arenas.get(net_assignment.net_lvalue);
                            let lvalue = lvalue.ident.item;

                            let ident = arenas.get_ident(lvalue.0);
                            let symbol_key = scope.get(ident).unwrap();

                            let SymbolVariant::Signal(signal_key) =
                                &scope.symbols[symbol_key].variant
                            else {
                                panic!("not a signal");
                            };
                            bb_builder.drive(gl, *signal_key, variable);

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
                                    );
                                    match ninput_gate_instantiation.gatetype.item {
                                        NInputGateType::And | NInputGateType::Nand => {
                                            for input in input_terminals.iter().skip(1) {
                                                let input = lower_expr(
                                                    &mut bb_builder,
                                                    gl,
                                                    &mut scope,
                                                    arenas.get(input),
                                                    arenas,
                                                );
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
                                                );
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
                                                );
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
                        let instance_module_key =
                            instantiated_modules.get(instantiation_ident).unwrap();
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
                                    .map(|p| lower_to_signal(gl, arenas.get(p), &mut scope, arenas))
                                    .collect(),
                                ListOfPortConnections::Named(ports) => ports
                                    .iter()
                                    .map(|_| {
                                        todo!()
                                        // lower_to_signal(gl, arenas.get(p), &mut scope, arenas)
                                    })
                                    .collect(),
                            };
                            module_builder
                                .entity
                                .instantiate(gl, *instance_module_key, ports);
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
                        );
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
                        );
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

    module_builder.finish(gl)
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
) -> BasicBlockBuilder {
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

                let value = lower_expr(&mut builder, gl, scope, arenas.get(*expression), arenas);
                assign_variable_lvalue(
                    gl,
                    &mut builder,
                    scope,
                    arenas.get(*variable_lvalue),
                    value,
                    arenas,
                );
            }
            Statement::CaseStatement => todo!(),
            Statement::ConditionalStatement => todo!(),
            Statement::DisableStatement => todo!(),
            Statement::EventTrigger => todo!(),
            Statement::LoopStatement(ls) => {
                builder =
                    statement::loop_statement::lower_loop_statement(builder, gl, scope, *ls, arenas)
            }
            Statement::NonBlockingAssignment(nba) => {
                let NonBlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control,
                    expression,
                } = arenas.get(*nba);
                assert!(delay_or_event_control.is_none());

                let value = lower_expr(&mut builder, gl, scope, arenas.get(*expression), arenas);
                assign_variable_lvalue(
                    gl,
                    &mut builder,
                    scope,
                    arenas.get(*variable_lvalue),
                    value,
                    arenas,
                );
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
                    );
                }
            }
            Statement::SeqBlock(id) => {
                let seq_block = arenas.get(*id);
                let statements = seq_block
                    .statements
                    .iter()
                    .map(|v| arenas.get(v).clone())
                    .collect::<Vec<_>>();
                builder = statements_to_process(builder, gl, scope, &statements, arenas);
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
                            let Expr::Ident(ident) = expr else {
                                panic!("Invalid display argument");
                            };
                            let ident = arenas.get_ident(ident.item.0);
                            let symbol_key = scope.get(ident).unwrap();
                            let var = match &scope.symbols[symbol_key].variant {
                                SymbolVariant::Signal(key) => builder.probe(gl, *key),
                                SymbolVariant::Variable(None) => todo!(),
                                SymbolVariant::Variable(Some(v)) => *v,
                            };
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

                        let lhs = lower_expr(&mut builder, gl, scope, lhs, arenas);
                        let rhs = lower_expr(&mut builder, gl, scope, rhs, arenas);

                        let mut predicate = builder.equals(gl, lhs, rhs);
                        if ident == "vogls_assert_ne" {
                            predicate = builder.logical_neg(gl, predicate);
                        }

                        builder.intrinsic(
                            gl,
                            IntrinsicOp::Assert,
                            vec![IntrinsicArg::Variable(predicate)],
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

    builder
}

fn get_intersect_symbols_generated<'a>(
    _gl: &mut GlobalContext,
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
                Statement::CaseStatement => todo!(),
                Statement::ConditionalStatement => todo!(),
                Statement::DisableStatement => todo!(),
                Statement::EventTrigger => todo!(),
                Statement::LoopStatement(_) => todo!(),
                Statement::NonBlockingAssignment(_) => todo!(),
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
    _gl: &mut GlobalContext,
    expr: &Expr,
    scope: &mut Scope<'a>,
    arenas: &'a AstArenas,
) -> SignalKey {
    let Expr::Ident(ident) = expr else { todo!() };

    let ident = arenas.get_ident(ident.item.0);
    let symbol_key = scope.get(&ident).unwrap();
    match &scope.symbols[symbol_key].variant {
        SymbolVariant::Signal(key) => *key,
        SymbolVariant::Variable(_) => todo!(),
        // SymbolVariant::Constant(_) => todo!(),
    }
}

fn lower_expr<'a>(
    builder: &mut BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<'a>,
    expr: &Expr,
    arenas: &'a AstArenas,
) -> VariableKey {
    match expr {
        Expr::BitPartSelect(select) => {
            let BitPartSelect { subject, braced } = select;
            let subject = arenas.get(*subject);
            let braced = arenas.get(*braced);

            let subject_v = lower_expr(builder, gl, scope, subject, arenas);
            let braced_v = lower_expr(builder, gl, scope, braced, arenas);

            builder.select_bit(gl, subject_v, braced_v)
        }
        Expr::Unary(op, child) => {
            let child = lower_expr(builder, gl, scope, arenas.get(*child), arenas);
            use UnaryOperator as O;
            match op {
                O::LogicalNegation => builder.logical_neg(gl, child),
                O::BitwiseNegation => builder.binary_neg(gl, child),
                O::ReductionAnd => todo!(),
                O::ReductionOr => todo!(),
                O::ReductionNand => todo!(),
                O::ReductionNor => todo!(),
                O::ReductionXor => todo!(),
                O::ReductionXnor => todo!(),
                O::SignPlus => todo!(),
                O::SignMinus => todo!(),
            }
        }
        Expr::Binary(op, l, r) => {
            let l = lower_expr(builder, gl, scope, arenas.get(*l), arenas);
            let r = lower_expr(builder, gl, scope, arenas.get(*r), arenas);
            use BinaryOperator as O;
            match op {
                O::Multiply => todo!(),
                O::Divide => todo!(),
                O::Modulus => todo!(),
                O::BinaryPlus => builder.plus(gl, l, r),
                O::BinaryMinus => todo!(),
                O::ShiftLeft => todo!(),
                O::ShiftRight => todo!(),
                O::GreaterThan => builder.unsigned_gt(gl, l, r),
                O::GreaterThanEqual => builder.unsigned_ge(gl, l, r),
                O::LessThan => builder.unsigned_lt(gl, l, r),
                O::LessThanEqual => builder.unsigned_le(gl, l, r),
                O::LogicalEquality => todo!(),
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
        Expr::Concatenation(_) => todo!(),
        Expr::Replication(_) => todo!(),
        Expr::Ternary(_, _, _) => todo!(),
        Expr::Ident(ident) => {
            let ident = arenas.get_ident(ident.item.0);
            let symbol_key = scope.get(&ident).expect("Variable not found");
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
    }
}

fn assign_variable_lvalue<'a>(
    gl: &mut GlobalContext,
    builder: &mut BasicBlockBuilder,
    scope: &mut Scope<'a>,
    lvalue: &VariableLValue,
    variable: VariableKey,
    arenas: &'a AstArenas,
) {
    let ident = arenas.get_ident(lvalue.ident.item.0);
    let symbol_key = scope.get(&ident).unwrap();
    match &mut scope.symbols[symbol_key].variant {
        SymbolVariant::Signal(key) => builder.drive(gl, *key, variable),
        SymbolVariant::Variable(v) => *v = Some(variable),
    }
}

fn eval_constant_expr<'a>(
    _gl: &mut GlobalContext,
    _scope: &mut Scope<'a>,
    expr: &ConstantExpr,
    arenas: &'a AstArenas,
) -> u64 {
    let ConstantExpr::Primary(primary) = expr;
    let ConstantPrimary::Number(number) = primary else {
        todo!();
    };
    let Decimal::Small(v) = arenas.decimals[number.at] else {
        todo!()
    };
    v
}
