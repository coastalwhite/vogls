use std::collections::{BinaryHeap, HashMap};

use slotmap::SlotMap;
use vogls_ir::{
    BasicBlockBuilder, ContextFormat, GlobalContext, IntrinsicArg, IntrinsicVariant, ModuleBuilder,
    Signal, SignalKey, Time, Type, Value,
};
use vogls_sim::{Event, ScheduledEvent};
use vogls_verilog::ast::module::{Module, ModuleOrGenerateItem, NonPortModuleItem};
use vogls_verilog::ast::statement::{
    DelayControl, EventControl, EventExpression, ProceduralTimingControl, Statement,
};
use vogls_verilog::lexer::Lexer;
use vogls_verilog::number::Decimal;
use vogls_verilog::parser::{Ast, AstArenas, Parser};

enum WatchCondition {
    None,
    Posedge,
    Negedge,
}

fn statements_to_process<'a, 'b>(
    mut builder: BasicBlockBuilder<'b>,
    stmts: &[Statement],
    signal: SignalKey,
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

                let decimal = arenas.get(nba.expression).into_decimal_literal().unwrap();
                let decimal = &arenas.decimals[decimal.at];
                let decimal = match decimal {
                    Decimal::Small(v) => *v as usize,
                    _ => todo!(),
                };

                let value = builder.constant(Value::Bit(decimal != 0));
                builder.drive(signal, value); // @TODO: Resolve ident
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
                                    EventExpression::Expression(_expr) => {
                                        conditions.push((WatchCondition::None, signal));
                                        signals.push(signal);
                                    }
                                    EventExpression::Posedge(_expr) => {
                                        conditions.push((WatchCondition::Posedge, signal));
                                        signals.push(signal);
                                    }
                                    EventExpression::Negedge(_expr) => {
                                        conditions.push((WatchCondition::Negedge, signal));
                                        signals.push(signal);
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
                        statements_to_process(builder, std::slice::from_ref(stmt), signal, arenas);
                }
            }
            Statement::SeqBlock(id) => {
                let seq_block = arenas.get(*id);
                builder = statements_to_process(
                    builder,
                    arenas.nodes.get_slice(seq_block.statements.node),
                    signal,
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
                            IntrinsicVariant::Display,
                            vec![IntrinsicArg::StringLiteral(str_literal.to_string())],
                        );
                    }
                    "finish" => builder.intrinsic(IntrinsicVariant::Finish, vec![]),

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(Lexer::new(
        r#"
module abc;
    always @ (posedge clk) $display("Hello!");

    initial begin
        clk <= 0;
        #4
        clk <= 1;
        #5
        clk <= 0;
        #7
        clk <= 1;
        #1
        $finish;
    end
endmodule
    "#,
        None,
    ));

    let Ast {
        root,
        arenas,
        path: _,
    } = parser.parse_file().unwrap();

    let Module {
        module_identifier: _,
        module_items,
    } = arenas.get(root);

    let mut gl = GlobalContext::default();
    let mut schedule = BinaryHeap::default();
    let mut variables = HashMap::default();
    let mut signals = HashMap::default();
    let mut listeners = SlotMap::default();
    let mut watches = HashMap::default();

    let signal = gl.signals.insert(Signal {
        name: "clk".into(),
        ty: Type::Bit,
    });

    signals.insert(signal, Value::Bit(false));

    let mut module_builder = ModuleBuilder::new("top_level".into());
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
                    let (_, bb_builder) = module_builder.process(&mut gl, "initial".into());
                    let bb_key = bb_builder.key();
                    let bb_builder = statements_to_process(
                        bb_builder,
                        std::slice::from_ref(arenas.get(statement)),
                        signal,
                        &arenas,
                    );
                    bb_builder.halt();
                    schedule.push(ScheduledEvent {
                        at: 0,
                        event: Event { bb: bb_key },
                    });
                }
                ModuleOrGenerateItem::AlwaysConstruct(id) => {
                    let statement = arenas.get(*id).0;
                    let (_, bb_builder) = module_builder.process(&mut gl, "always".into());
                    let bb_key = bb_builder.key();
                    let bb_builder = statements_to_process(
                        bb_builder,
                        std::slice::from_ref(arenas.get(statement)),
                        signal,
                        &arenas,
                    );
                    bb_builder.wait_to(Time(0), bb_key);
                    schedule.push(ScheduledEvent {
                        at: 0,
                        event: Event { bb: bb_key },
                    });
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

    let module = module_builder.finish();
    println!("{}", module.display(&gl));

    vogls_sim::run(
        &gl,
        &mut schedule,
        &mut variables,
        &mut signals,
        &mut listeners,
        &mut watches,
        100,
    );

    Ok(())
}
