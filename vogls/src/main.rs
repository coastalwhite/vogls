use std::collections::BinaryHeap;

use vogls_sim::{Event, IR, ScheduledEvent};
use vogls_verilog::ast::module::{Module, ModuleOrGenerateItem, NonPortModuleItem};
use vogls_verilog::ast::statement::{DelayControl, ProceduralTimingControl, Statement};
use vogls_verilog::lexer::Lexer;
use vogls_verilog::number::Decimal;
use vogls_verilog::parser::{Ast, AstArenas, Parser};

fn statements_to_events<'a>(
    stmts: &[Statement],
    arenas: &'a AstArenas,
    events: &mut Vec<Event>,
) -> usize {
    let head = events.len();
    let mut event = Event { ir: Vec::new() };
    for statement in stmts.iter() {
        match statement {
            Statement::BlockingAssignment(ast_id) => todo!(),
            Statement::CaseStatement => todo!(),
            Statement::ConditionalStatement => todo!(),
            Statement::DisableStatement => todo!(),
            Statement::EventTrigger => todo!(),
            Statement::LoopStatement => todo!(),
            Statement::NonBlockingAssignment(ast_id) => todo!(),
            Statement::ParBlock => todo!(),
            Statement::ProceduralContinuousAssignments => todo!(),
            Statement::ProceduralTimingControlStatement(ptc) => match arenas.get(*ptc) {
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
                            event.ir.push(IR::Schedule(events.len() + 1, value));

                            events.push(std::mem::replace(&mut event, Event { ir: Vec::new() }));
                        }
                    }
                }
                ProceduralTimingControl::EventControl(ast_id) => todo!(),
            },
            Statement::SeqBlock(id) => {
                let seq_block = arenas.get(*id);
                statements_to_events(
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
                    },

                    // @Incomplete: Many variants here.
                    _ => todo!(),
                }
            },
            Statement::TaskEnable => todo!(),
            Statement::WaitStatement => todo!(),
        }
    }
    events.push(event);
    head
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(Lexer::new(
        r#"
module abc;
    initial begin
        #5
        $display("abc");
        #10
        $display("More text");
        #20
    end
endmodule
    "#,
        None,
    ));

    let Ast { root, arenas, path } = parser.parse_file().unwrap();

    let Module {
        module_identifier,
        module_items,
    } = arenas.get(root);

    let mut schedule = BinaryHeap::default();
    let mut events = vec![];
    let listeners = vec![];
    let mut registers = vec![];

    for module_item in module_items.node.iter() {
        dbg!("module item");
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
                    let head = statements_to_events(
                        std::slice::from_ref(arenas.get(statement)),
                        &arenas,
                        &mut events,
                    );

                    schedule.push(ScheduledEvent { at: 0, id: head });
                }
                ModuleOrGenerateItem::AlwaysConstruct(id) => _ = dbg!("always"),
                ModuleOrGenerateItem::LoopGenerateConstruct => todo!(),
                ModuleOrGenerateItem::ConditionalGenerateConstruct => todo!(),
            },
            NonPortModuleItem::GenerateRegion => todo!(),
            NonPortModuleItem::SpecifyBlock => todo!(),
            NonPortModuleItem::ParameterDeclaration => todo!(),
            NonPortModuleItem::SpecParamDeclaration => todo!(),
        }
    }

    vogls_sim::run(&mut schedule, &events, &listeners, &mut registers);

    Ok(())
}
