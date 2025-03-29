mod scope;

use std::collections::HashSet;

use scope::Scope;

use vogls_ir::{
    BasicBlockBuilder, GlobalContext, IntrinsicArg, IntrinsicOp, ModuleBuilder, ModuleKey, Signal,
    SignalKey, Time, Type, Value,
};

use crate::ast::expr::Expr;
use crate::ast::module::{Module, ModuleOrGenerateItem, NonPortModuleItem};
use crate::ast::statement::{
    DelayControl, EventControl, EventExpression, ProceduralTimingControl, Statement, VariableLValue,
};
use crate::number::Decimal;
use crate::parser::{Ast, AstArenas};

pub struct SignalScopeItem {
    ty: Type,
    key: SignalKey,
}

pub enum ScopeItem {
    Signal(SignalScopeItem),
    LocalVariable,
}

pub fn lower_module_to_ir(ast: &Ast, gl: &mut GlobalContext) -> ModuleKey {
    let Ast {
        root,
        arenas,
        path: _,
    } = ast;

    let Module {
        module_identifier,
        module_items,
    } = arenas.get(*root);

    let mut scope = Scope::<&str, ScopeItem>::new();
    let mut processes = Vec::new();

    let clk_key = gl.signals.insert(Signal {
        name: "clk".into(),
        ty: Type::Bit,
    });
    scope.push(
        "clk",
        ScopeItem::Signal(SignalScopeItem { ty: Type::Bit, key: clk_key }),
    );

    let module_identifier = arenas.get_ident(module_identifier.item.0);

    let mut module_builder = ModuleBuilder::new(module_identifier.to_string());
    for module_item in module_items.node.iter() {
        match arenas.nodes.get(module_item) {
            NonPortModuleItem::ModuleOrGenerateItem(id) => match arenas.get(*id) {
                ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration => todo!(),
                ModuleOrGenerateItem::LocalParameterDeclaration => todo!(),
                ModuleOrGenerateItem::ParameterOverride => todo!(),
                ModuleOrGenerateItem::ContinuousAssign => todo!(),
                ModuleOrGenerateItem::GateInstantiation => todo!(),
                ModuleOrGenerateItem::UdpInstantiation => todo!(),
                ModuleOrGenerateItem::ModuleInstantiation => todo!(),
                ModuleOrGenerateItem::InitialConstruct(id) => {
                    let statement = arenas.get(*id).0;
                    let (section_key, bb_builder) = module_builder.process(gl, "initial".into());
                    let bb_builder = statements_to_process(
                        bb_builder,
                        &mut scope,
                        std::slice::from_ref(arenas.get(statement)),
                        &arenas,
                    );
                    bb_builder.halt();
                    processes.push(section_key);
                }
                ModuleOrGenerateItem::AlwaysConstruct(id) => {
                    let statement = arenas.get(*id).0;
                    let (section_key, bb_builder) = module_builder.process(gl, "always".into());
                    let bb_key = bb_builder.key();
                    let bb_builder = statements_to_process(
                        bb_builder,
                        &mut scope,
                        std::slice::from_ref(arenas.get(statement)),
                        &arenas,
                    );
                    bb_builder.wait_to(Time(0), bb_key);
                    processes.push(section_key);
                }
                ModuleOrGenerateItem::LoopGenerateConstruct => todo!(),
                ModuleOrGenerateItem::ConditionalGenerateConstruct => todo!(),
            },
            NonPortModuleItem::GenerateRegion => todo!(),
            NonPortModuleItem::SpecifyBlock => todo!(),
            NonPortModuleItem::ParameterDeclaration => todo!(),
            NonPortModuleItem::SpecParamDeclaration => todo!(),
        }
    }

    let (_, mut bb_builder) = module_builder.entity(gl, format!("{module_identifier}_entity"));
    for process in processes {
        bb_builder.instantiate(process);
    }
    bb_builder.signal(clk_key);
    bb_builder.halt();

    let module = module_builder.finish();
    gl.modules.insert(module)
}

enum WatchCondition {
    None,
    Posedge,
    Negedge,
}

fn statements_to_process<'a, 'b>(
    mut builder: BasicBlockBuilder<'b>,
    scope: &mut Scope<&'a str, ScopeItem>,
    stmts: &[Statement],
    arenas: &'a AstArenas,
) -> BasicBlockBuilder<'b> {
    for statement in stmts.iter() {
        match statement {
            Statement::BlockingAssignment(_) => todo!(),
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

                let value = builder.constant(Value::Bit(decimal != 0));
                builder.drive(scope_item.key, value); // @TODO: Resolve ident
            }
            Statement::ParBlock => todo!(),
            Statement::ProceduralContinuousAssignments => todo!(),
            Statement::ProceduralTimingControlStatement(ptc, statement) => {
                match arenas.get(*ptc) {
                    ProceduralTimingControl::DelayControl(delay_control) => {
                        let delay_control = arenas.get(*delay_control);
                        match delay_control {
                            DelayControl::DelayValue(value) => {
                                let value = arenas.get(*value);
                                let value = &arenas.decimals[value.0.item.at];
                                let value = match value {
                                    Decimal::Small(v) => *v as usize,
                                    _ => todo!(),
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

                                builder = builder.wait(Time(value as u64));
                            }
                        }
                    }
                    ProceduralTimingControl::EventControl(event_control) => {
                        builder = builder.jump();
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
                            before.push(builder.probe(*signal));
                        }

                        builder = builder.watch(signals);

                        let mut acc = builder.constant(Value::Bit(true));
                        for ((condition, signal), before) in conditions.into_iter().zip(before) {
                            use WatchCondition as C;

                            let cond = match condition {
                                C::Posedge => {
                                    let after = builder.probe(signal);
                                    let t = builder.neg(before);
                                    builder.and(t, after)
                                }
                                C::Negedge => {
                                    let after = builder.probe(signal);
                                    let t = builder.neg(after);
                                    builder.and(before, t)
                                }
                                C::None => builder.constant(Value::Bit(true)),
                            };
                            acc = builder.and(acc, cond);
                        }

                        builder = builder.branch_false_to(acc, start_key);
                    }
                }

                if let Some(stmt) = statement {
                    let stmt = arenas.get(*stmt);
                    builder =
                        statements_to_process(builder, scope, std::slice::from_ref(stmt), arenas);
                }
            }
            Statement::SeqBlock(id) => {
                let seq_block = arenas.get(*id);
                builder = statements_to_process(
                    builder,
                    scope,
                    arenas.nodes.get_slice(seq_block.statements.node),
                    arenas,
                );
            }
            Statement::SystemTaskEnable(id) => {
                let system_task_enable = arenas.get(*id);

                let ident = system_task_enable.system_task_identifier.item;
                let ident = &arenas.idents[ident.0.start..ident.0.end];

                match ident {
                    "display" => {
                        let expressions = system_task_enable.expressions;
                        assert_eq!(expressions.len(), 1); // @Improve: Error message

                        let expr = arenas.get(expressions.first().unwrap());
                        let str_literal = expr.into_str_literal().unwrap();
                        let str_literal = &arenas.idents[str_literal.0.start..str_literal.0.end];

                        builder.intrinsic(
                            IntrinsicOp::Display,
                            vec![IntrinsicArg::StringLiteral(str_literal.to_string())],
                        );
                    }
                    "finish" => builder.intrinsic(IntrinsicOp::Finish, vec![]),

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
