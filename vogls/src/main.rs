use std::collections::{BinaryHeap, HashMap};

use slotmap::SlotMap;
use vogls_ir::{ContextFormat, GlobalContext, SectionVariant, Value};
use vogls_sim::{Context, Event, ScheduledEvent};
use vogls_verilog::lexer::Lexer;
use vogls_verilog::lower::lower_module_to_ir;
use vogls_verilog::parser::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(Lexer::new(
        r#"
module abc(
    input clk
);

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

    let ast = parser.parse_file().unwrap();

    let mut gl = GlobalContext::default();
    let tl_module = lower_module_to_ir(&ast, &mut gl);

    let mut ctx = Context::new();
    let mut schedule = BinaryHeap::default();
    let mut variables = HashMap::default();
    let mut signals = HashMap::default();
    let mut listeners = SlotMap::default();
    let mut watches = HashMap::default();

    let tl_module = gl.modules.get(tl_module).unwrap();
    for section in &tl_module.sections {
        let section = gl.sections.get(*section).unwrap();
        if section.variant == SectionVariant::Entity {
            schedule.push(ScheduledEvent {
                at: 0,
                event: Event { bb: section.entry },
            });
            for i in &section.ins {
                signals.insert(*i, Value::Bit(false));
            }
        }
    }

    println!("{}", tl_module.display(&gl));

    vogls_sim::run(
        &gl,
        &mut ctx,
        &mut schedule,
        &mut variables,
        &mut signals,
        &mut listeners,
        &mut watches,
        100,
    );

    Ok(())
}
