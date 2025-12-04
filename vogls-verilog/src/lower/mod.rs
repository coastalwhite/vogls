mod scope;

use std::collections::HashMap;

use scope::Scope;

use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, IntrinsicArg, IntrinsicOp, ModuleBuilder, ModuleKey,
    Signal, SignalKey, Time, Type, Value, VariableKey,
};

use crate::ast::AstId;
use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression, ConstantPrimary};
use crate::ast::expr::{BinaryOperator, Expr, UnaryOperator};
use crate::ast::module::{
    GateInstantiation, ListOfPortConnections, Module, ModuleInstance, ModuleInstantiation,
    ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration, ModulePorts,
    NInputGateInstance, NInputGateType, NonPortModuleItem, ParamAssignment, ParameterDeclaration,
    Port, PortDeclaration,
};
use crate::ast::statement::{
    DelayControl, DelayValue, EventControl, EventExpression, ProceduralTimingControl, Statement,
};
use crate::number::Decimal;
use crate::parser::{Ast, AstArenas};

#[derive(Debug)]
pub struct SignalScopeItem {
    #[expect(unused)]
    ty: Type,
    key: SignalKey,
}

#[derive(Debug)]
pub enum ScopeItem {
    Signal(SignalScopeItem),
    LocalVariable,
    Constant(u64),
}

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
    let mut scope = Scope::<&str, ScopeItem>::new();
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

                for ident in idents.iter() {
                    let ident = arenas.get_ident(arenas.get(ident).0);

                    let key = gl.signals.insert(Signal {
                        name: ident.into(),
                        ty: Type::Bits(1),
                    });
                    scope.push(
                        ident,
                        ScopeItem::Signal(SignalScopeItem {
                            ty: Type::Bits(1),
                            key,
                        }),
                    );

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
                let port = arenas.get(port.references).identifier.item.0;
                let ident = arenas.get_ident(port);

                let key = gl.signals.insert(Signal {
                    name: ident.into(),
                    ty: Type::Bits(1),
                });
                scope.push(
                    ident,
                    ScopeItem::Signal(SignalScopeItem {
                        ty: Type::Bits(1),
                        key,
                    }),
                );

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

                for ident in idents.iter() {
                    let ident = arenas.get_ident(arenas.get(ident).0);

                    let key = gl.signals.insert(Signal {
                        name: ident.into(),
                        ty: Type::Bits(1),
                    });
                    scope.push(
                        ident,
                        ScopeItem::Signal(SignalScopeItem {
                            ty: Type::Bits(1),
                            key,
                        }),
                    );

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
                                for ident in net_declaration.identifiers.iter() {
                                    let ident = arenas.get(ident);
                                    let ident = arenas.get_ident(ident.0);
                                    let key = gl.signals.insert(Signal {
                                        name: ident.into(),
                                        ty: Type::Bits(1),
                                    });
                                    scope.push(
                                        ident,
                                        ScopeItem::Signal(SignalScopeItem {
                                            ty: Type::Bits(1),
                                            key,
                                        }),
                                    );
                                    module_builder.entity.signal(gl, key);
                                }
                            }
                            ModuleOrGenerateItemDeclaration::Reg(id) => {
                                let reg_declaration = arenas.get(*id);
                                for ident in reg_declaration.identifiers.iter() {
                                    let ident = arenas.get(ident);
                                    let ident = arenas.get_ident(ident.0);
                                    let key = gl.signals.insert(Signal {
                                        name: ident.into(),
                                        ty: Type::Bits(1),
                                    });
                                    scope.push(
                                        ident,
                                        ScopeItem::Signal(SignalScopeItem {
                                            ty: Type::Bits(1),
                                            key,
                                        }),
                                    );
                                    module_builder.entity.signal(gl, key);
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

                            let ScopeItem::Signal(scope_item) = scope.get(&ident).unwrap() else {
                                panic!("not a signal");
                            };
                            bb_builder.drive(gl, scope_item.key, variable); // @TODO: Resolve ident

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

                                    let ScopeItem::Signal(scope_item) = scope.get(&ident).unwrap()
                                    else {
                                        panic!("not a signal");
                                    };
                                    bb_builder.drive(gl, scope_item.key, value);
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
                        let ParamAssignment { param, constant } = arenas.get(assignment);
                        let ConstantMinTypMaxExpression::Single(constant) = arenas.get(*constant)
                        else {
                            todo!();
                        };
                        let ConstantExpr::Primary(primary) = arenas.get(*constant);
                        let ConstantPrimary::Number(number) = primary else {
                            todo!();
                        };
                        let Decimal::Small(v) = arenas.decimals[number.at] else {
                            todo!()
                        };
                        scope.push(arenas.get_ident(param.item.0), ScopeItem::Constant(v));
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
    scope: &mut Scope<&'a str, ScopeItem>,
    stmts: &[Statement],
    arenas: &'a AstArenas,
) -> BasicBlockBuilder {
    for statement in stmts.iter() {
        match statement {
            Statement::BlockingAssignment(ba) => {
                // @Incorrect
                let ba = arenas.get(*ba);
                assert!(ba.delay_or_event_control.is_none());

                let lvalue = arenas.get(ba.variable_lvalue);
                let lvalue = lvalue.ident.item;

                let ident = arenas.get_ident(lvalue.0);

                let ScopeItem::Signal(scope_item) = scope.get(&ident).expect("cannot find ident")
                else {
                    panic!("not a signal");
                };

                let decimal = arenas.get(ba.expression).into_decimal_literal().unwrap();
                let decimal = &arenas.decimals[decimal.at];
                let decimal = match decimal {
                    Decimal::Small(v) => *v as usize,
                    _ => todo!(),
                };

                let value =
                    builder.constant(gl, Value::Bits(Bits::Small(u64::from(decimal != 0), 1)));
                builder.drive(gl, scope_item.key, value); // @TODO: Resolve ident
            }
            Statement::CaseStatement => todo!(),
            Statement::ConditionalStatement => todo!(),
            Statement::DisableStatement => todo!(),
            Statement::EventTrigger => todo!(),
            Statement::LoopStatement => todo!(),
            Statement::NonBlockingAssignment(nba) => {
                let nba = arenas.get(*nba);
                assert!(nba.delay_or_event_control.is_none());

                let lvalue = arenas.get(nba.variable_lvalue);
                let lvalue = lvalue.ident.item;

                let ident = arenas.get_ident(lvalue.0);

                let ScopeItem::Signal(scope_item) = scope.get(&ident).unwrap() else {
                    panic!("not a signal");
                };

                let decimal = arenas.get(nba.expression).into_decimal_literal().unwrap();
                let decimal = &arenas.decimals[decimal.at];
                let decimal = match decimal {
                    Decimal::Small(v) => *v as usize,
                    _ => todo!(),
                };

                let value =
                    builder.constant(gl, Value::Bits(Bits::Small(u64::from(decimal != 0), 1)));
                builder.drive(gl, scope_item.key, value); // @TODO: Resolve ident
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
                                    DelayValue::Identifier(value) => {
                                        let ScopeItem::Constant(v) = scope
                                            .get(&arenas.get_ident(value.0))
                                            .expect("unknown ident")
                                        else {
                                            todo!();
                                        };
                                        *v as usize
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
                                match arenas.get(*event_expression) {
                                    EventExpression::Expression(expr) => {
                                        let Expr::Ident(ident) = arenas.get(*expr) else {
                                            panic!("not an ident");
                                        };
                                        let ident = arenas.get_ident(ident.item.0);
                                        let ScopeItem::Signal(scope_item) =
                                            scope.get(&ident).unwrap()
                                        else {
                                            panic!("not a signal");
                                        };

                                        conditions.push((WatchCondition::None, scope_item.key));
                                        signals.push(scope_item.key);
                                    }
                                    EventExpression::Posedge(expr) => {
                                        let Expr::Ident(ident) = arenas.get(*expr) else {
                                            panic!("not an ident");
                                        };
                                        let ident = arenas.get_ident(ident.item.0);
                                        let ScopeItem::Signal(scope_item) =
                                            scope.get(&ident).unwrap()
                                        else {
                                            panic!("not a signal");
                                        };

                                        conditions.push((WatchCondition::Posedge, scope_item.key));
                                        signals.push(scope_item.key);
                                    }
                                    EventExpression::Negedge(expr) => {
                                        let Expr::Ident(ident) = arenas.get(*expr) else {
                                            panic!("not an ident");
                                        };
                                        let ident = arenas.get_ident(ident.item.0);
                                        let ScopeItem::Signal(scope_item) =
                                            scope.get(&ident).unwrap()
                                        else {
                                            panic!("not a signal");
                                        };

                                        conditions.push((WatchCondition::Negedge, scope_item.key));
                                        signals.push(scope_item.key);
                                    }
                                    EventExpression::OrList(_, _) => todo!(),
                                }
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
                        let str_literal = expr.into_str_literal().unwrap();
                        let str_literal = &arenas.text[str_literal.0.start..str_literal.0.end];

                        builder.intrinsic(
                            gl,
                            IntrinsicOp::Display,
                            vec![IntrinsicArg::StringLiteral(str_literal.to_string())],
                        );
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

fn lower_to_signal<'a>(
    _gl: &mut GlobalContext,
    expr: &Expr,
    scope: &mut Scope<&'a str, ScopeItem>,
    arenas: &'a AstArenas,
) -> SignalKey {
    let Expr::Ident(ident) = expr else { todo!() };

    let ident = arenas.get_ident(ident.item.0);
    match scope.get(&ident).unwrap() {
        ScopeItem::Signal(i) => i.key,
        ScopeItem::LocalVariable => todo!(),
        ScopeItem::Constant(_) => todo!(),
    }
}

fn lower_expr<'a>(
    builder: &mut BasicBlockBuilder,
    gl: &mut GlobalContext,
    scope: &mut Scope<&'a str, ScopeItem>,
    expr: &Expr,
    arenas: &'a AstArenas,
) -> VariableKey {
    match expr {
        Expr::BitPartSelect(_) => todo!(),
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
            let scope_item = scope.get(&ident).expect("Variable not found");
            match scope_item {
                ScopeItem::Signal(si) => builder.probe(gl, si.key),
                ScopeItem::LocalVariable => todo!(),
                ScopeItem::Constant(_) => todo!(),
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
        Expr::Sized(_) => todo!(),
        Expr::String(_) => todo!(),
    }
}
