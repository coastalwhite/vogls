use std::collections::{BinaryHeap, HashMap, HashSet};

mod elaborate;

use slotmap::SlotMap;
use vogls_ir::{ContextFormat, GlobalContext, Value};
use vogls_sim::{Context, Event, ScheduledEvent, VmProcess, VmProcessKey, lower_process_to_vm};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{Module, ModuleItem, ModuleOrGenerateItem, NonPortModuleItem};
use vogls_verilog::lexer::Lexer;
use vogls_verilog::lower::lower_module_to_ir;
use vogls_verilog::parser::Parser;

use self::elaborate::elaborate;

fn usage() {
    eprintln!(
        "usage: {} <path/to/file.v> <top-level module>",
        std::env::args().next().unwrap()
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        usage();
        std::process::exit(2);
    };
    let Some(tl_module_name) = std::env::args().nth(2) else {
        usage();
        std::process::exit(2);
    };
    let content = std::fs::read_to_string(&path)?;
    let mut parser = Parser::new(Lexer::new(&content, None));

    let ast = match parser.parse_file() {
        Ok(ast) => ast,
        Err(err) => {
            let mut out = String::new();
            err.report(&path, &content, &mut out)?;
            eprint!("{out}");
            std::process::exit(1);
        }
    };
    let mut gl = GlobalContext::default();

    let module_lut =
        HashMap::<&str, usize>::from_iter(ast.modules.iter().enumerate().map(|(i, module_id)| {
            let module = ast.arenas.get(module_id);
            (ast.arenas.get_ident(module.module_identifier.item.0), i)
        }));

    let Some(tl_module) = module_lut.get(tl_module_name.as_str()) else {
        return Err(<Box<dyn std::error::Error>>::from(
            "cannot find top-level module".to_string(),
        ));
    };

    // Create a list of all module in breadth- first order.
    // @TODO: Add a mechanism for detecting dependency loops.
    let mut module_stack: Vec<AstId<Module>> = Vec::new();
    let mut module_seen: HashSet<&str> = HashSet::new();
    module_stack.push(ast.modules.get(*tl_module));
    module_seen.insert(tl_module_name.as_str());
    let mut start = 0;
    while start != module_stack.len() {
        let end = module_stack.len();
        for j in start..end {
            let module_id = module_stack[j];
            let Module {
                module_identifier: _,
                module_items,
                ports: _,
            } = ast.arenas.get(module_id);
            for module_item in module_items.iter() {
                let ModuleItem::NonPortModuleItem(p) = ast.arenas.get(module_item) else {
                    continue;
                };

                if let NonPortModuleItem::ModuleOrGenerateItem(module_item) = ast.arenas.get(*p) {
                    if let ModuleOrGenerateItem::ModuleInstantiation(module_instantiation) =
                        ast.arenas.get(*module_item)
                    {
                        let module_instantiation = ast.arenas.get(*module_instantiation);
                        let module_name = ast
                            .arenas
                            .get_ident(module_instantiation.module_identifier.item.0);

                        if !module_seen.insert(module_name) {
                            continue;
                        }

                        let i = module_lut
                            .get(module_name)
                            .ok_or(format!("module '{module_name}' does not exist"))?;
                        module_stack.push(ast.modules.get(*i));
                    }
                }
            }
        }
        start = end;
    }

    // Walk the modules in depth-first order and lower to IR.
    let mut instantiated_modules = HashMap::with_capacity(module_stack.len());
    for module_id in module_stack.iter().rev() {
        let module_identifier = ast.arenas.get(*module_id).module_identifier;
        let module_identifier = ast.arenas.get_ident(module_identifier.item.0);

        let module_key = lower_module_to_ir(&ast, *module_id, &mut gl, &instantiated_modules);
        instantiated_modules.insert(module_identifier, module_key);
    }

    let tl_module_key = *instantiated_modules.get(tl_module_name.as_str()).unwrap();

    let mut ctx = Context::new();
    let mut processes = SlotMap::<VmProcessKey, VmProcess>::default();
    let mut schedule = BinaryHeap::default();
    let mut signals = HashMap::default();
    let mut listeners = SlotMap::default();
    let mut watches = HashMap::default();

    for module in gl.modules.values() {
        println!("{}", module.display(&gl));
    }

    // Find the entity for the Top-Level Module.
    let mut elab_processes = Vec::new();
    elaborate(tl_module_key, &mut gl, &mut elab_processes);

    let mut io_signals = HashMap::new();
    for &process in elab_processes.iter() {
        println!();
        println!("{}", gl.processes[process].display(&gl));
        let vm_process = lower_process_to_vm(process, &gl, &mut io_signals);

        print!("{}", &vm_process);

        let stack = vec![0u8; vm_process.stack_size];
        let vm_process_key = processes.insert(vm_process);

        println!(": {vm_process_key:?}");

        schedule.push(ScheduledEvent {
            at: 0,
            event: Event {
                process: vm_process_key,
                stack,
                ip: 0,
            },
        });
    }

    for (_, signal) in io_signals {
        signals.insert(signal, Value::Bit(false));
    }

    dbg!(&schedule);

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
