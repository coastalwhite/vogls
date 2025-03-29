use std::collections::{BinaryHeap, HashMap};

use slotmap::SlotMap;
use vogls_ir::{ContextFormat, GlobalContext, SectionVariant, Value};
use vogls_sim::{Context, Event, ScheduledEvent, VmProcess, VmProcessKey, lower_process_to_vm};
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
    let mut processes = SlotMap::<VmProcessKey, VmProcess>::default();
    let mut schedule = BinaryHeap::default();
    let mut signals = HashMap::default();
    let mut listeners = SlotMap::default();
    let mut watches = HashMap::default();

    let tl_module = gl.modules.get(tl_module).unwrap();
    for section_key in &tl_module.sections {
        let section = gl.sections.get(*section_key).unwrap();
        if section.variant == SectionVariant::Entity {
            for i in &section.ins {
                signals.insert(*i, Value::Bit(false));
            }
        } else {
            let vm_process = lower_process_to_vm(*section_key, &gl);

            println!("{}", &vm_process);

            let stack = vec![0u8; vm_process.stack_size];
            let vm_process_key = processes.insert(vm_process);

            schedule.push(ScheduledEvent {
                at: 0,
                event: Event {
                    process: vm_process_key,
                    stack,
                    ip: 0,
                },
            });
        }
    }

    println!("{}", tl_module.display(&gl));

    vogls_sim::run(
        &mut ctx,
        &processes,
        &mut schedule,
        &mut signals,
        &mut listeners,
        &mut watches,
        100,
    );

    Ok(())
}
