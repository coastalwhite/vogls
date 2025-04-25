use std::collections::{BinaryHeap, HashMap, HashSet};

use slotmap::SlotMap;
use vogls_ir::{
    BasicBlockTerminator, ContextFormat, GlobalContext, Instruction, SectionVariant, Value,
};
use vogls_sim::{
    Context, Event, ScheduledEvent, VmProcess, VmProcessKey, VmSignalKey, lower_process_to_vm,
};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{Module, ModuleOrGenerateItem, NonPortModuleItem};
use vogls_verilog::lexer::Lexer;
use vogls_verilog::lower::lower_module_to_ir;
use vogls_verilog::parser::Parser;

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
    let content = std::fs::read_to_string(path)?;
    let mut parser = Parser::new(Lexer::new(&content, None));

    let ast = parser.parse_file().unwrap();
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
                port_declarations: _,
            } = ast.arenas.get(module_id);
            for module_item in module_items.node.iter() {
                if let NonPortModuleItem::ModuleOrGenerateItem(module_item) =
                    ast.arenas.nodes.get(module_item)
                {
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

    let tl_module = *instantiated_modules.get(tl_module_name.as_str()).unwrap();

    let mut ctx = Context::new();
    let mut processes = SlotMap::<VmProcessKey, VmProcess>::default();
    let mut schedule = BinaryHeap::default();
    let mut signals = HashMap::default();
    let mut listeners = SlotMap::default();
    let mut watches = HashMap::default();

    let tl_module = gl.modules.get(tl_module).unwrap();

    for module in gl.modules.values() {
        println!("{}", module.display(&gl));
    }

    // Find the entity for the Top-Level Module.
    let mut entity_stack = Vec::new();
    for section_key in &tl_module.sections {
        let section = gl.sections.get(*section_key).unwrap();
        if section.variant != SectionVariant::Entity {
            continue;
        }
        entity_stack.push(*section_key);
    }

    // Recursively Instantiate the entities and processes in
    let mut next_vm_signal_key = VmSignalKey(0);
    let mut io_signals = HashMap::new();
    while let Some(entity_section_key) = entity_stack.pop() {
        let entity = gl.sections.get(entity_section_key).unwrap();
        assert_eq!(entity.variant, SectionVariant::Entity);
        let bb = gl.bbs.get(entity.entry).unwrap();
        if !matches!(bb.terminator, BasicBlockTerminator::Halt) {
            todo!("evaluation with vogls-sim");
        }

        io_signals.clear();
        for instr in &bb.instrs {
            match instr {
                Instruction::Signal(signal_key) => {
                    io_signals.insert(*signal_key, next_vm_signal_key);
                    signals.insert(next_vm_signal_key, Value::Bit(false));
                    next_vm_signal_key.0 += 1;
                }
                Instruction::Instantiate(section_key) => {
                    let section = gl.sections.get(*section_key).unwrap();

                    match section.variant {
                        SectionVariant::Entity => entity_stack.push(*section_key),
                        SectionVariant::Function => todo!(),
                        SectionVariant::Process => {
                            let vm_process = lower_process_to_vm(*section_key, &gl, &io_signals);

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
                }
                _ => todo!("evaluation with vogls-sim"),
            }
        }
    }

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
