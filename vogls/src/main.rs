use std::collections::BinaryHeap;

use vogls_sim::ir::{IR, IRDisplay, WatchCondition};
use vogls_sim::{Event, Listeners, ScheduledEvent};
use vogls_verilog::ast::module::{Module, ModuleOrGenerateItem, NonPortModuleItem};
use vogls_verilog::ast::statement::{
    DelayControl, EventControl, EventExpression, ProceduralTimingControl, Statement,
};
use vogls_verilog::lexer::Lexer;
use vogls_verilog::number::Decimal;
use vogls_verilog::parser::{Ast, AstArenas, Parser};

fn statements_to_events_impl<'a>(
    mut event: &mut Event,
    stmts: &[Statement],
    arenas: &'a AstArenas,
    events: &mut Vec<Event>,
) {
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

                event.ir.push(IR::Load(decimal as u32));
                event.ir.push(IR::Update(0)); // @TODO: Resolve ident
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

                                event.ir.push(IR::Schedule(events.len() + 1, value));
                                events
                                    .push(std::mem::replace(&mut event, Event { ir: Vec::new() }));
                            }
                        }
                    }
                    ProceduralTimingControl::EventControl(event_control) => {
                        let mut conditions = Vec::new();
                        match arenas.get(*event_control) {
                            EventControl::EventExpression(event_expression) => {
                                match arenas.get(*event_expression) {
                                    EventExpression::Expression(_expr) => {
                                        conditions.push((WatchCondition::None, 0))
                                    }
                                    EventExpression::Posedge(_expr) => {
                                        conditions.push((WatchCondition::Posedge, 0))
                                    }
                                    EventExpression::Negedge(_expr) => {
                                        conditions.push((WatchCondition::Negedge, 0))
                                    }
                                    EventExpression::OrList(_, _) => todo!(),
                                }
                            }
                        }

                        event.ir.push(IR::Watch(events.len() + 1, conditions));
                        events.push(std::mem::replace(&mut event, Event { ir: Vec::new() }));
                    }
                }

                if let Some(stmt) = statement {
                    let stmt = arenas.get(*stmt);
                    statements_to_events_impl(event, std::slice::from_ref(stmt), arenas, events);
                }
            }
            Statement::SeqBlock(id) => {
                let seq_block = arenas.get(*id);
                statements_to_events_impl(
                    event,
                    arenas.nodes.get_slice(seq_block.statements.node),
                    arenas,
                    events,
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

                        event.ir.push(IR::Display(str_literal.to_string()));
                    }
                    "finish" => event.ir.push(IR::Finish),

                    // @Incomplete: Many variants here.
                    _ => todo!(),
                }
            }
            Statement::TaskEnable => todo!(),
            Statement::WaitStatement => todo!(),
        }
    }
}

fn statements_to_events<'a>(
    stmts: &[Statement],
    arenas: &'a AstArenas,
    events: &mut Vec<Event>,
) -> (usize, usize) {
    let head = events.len();
    let mut event = Event { ir: Vec::new() };
    statements_to_events_impl(&mut event, stmts, arenas, events);
    let tail = events.len();
    events.push(event);
    (head, tail)
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

    let mut schedule = BinaryHeap::default();
    let mut events = vec![];
    let mut listeners = vec![Listeners::default()];
    let mut registers = vec![0u32];

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
                    let (head, _tail) = statements_to_events(
                        std::slice::from_ref(arenas.get(statement)),
                        &arenas,
                        &mut events,
                    );

                    schedule.push(ScheduledEvent { at: 0, id: head });
                }
                ModuleOrGenerateItem::AlwaysConstruct(id) => {
                    let statement = arenas.get(*id).0;
                    let (head, tail) = statements_to_events(
                        std::slice::from_ref(arenas.get(statement)),
                        &arenas,
                        &mut events,
                    );

                    events[tail].ir.push(IR::Schedule(head, 0));
                    schedule.push(ScheduledEvent { at: 0, id: head });
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

    for (i, event) in events.iter().enumerate() {
        println!("Event e{i}:");
        for ir in &event.ir {
            println!("{}", ir.display());
        }

        println!();
    }

    vogls_sim::run(&mut schedule, &events, &mut listeners, &mut registers, 100);

    Ok(())
}
