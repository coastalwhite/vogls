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

fn lines_with_offset(mut s: &str) -> Vec<(usize, &str)> {
    let original_length = s.len();
    let mut vs = Vec::new();
    while let Some(p) = s.find(['\n', '\r']) {
        if s.as_bytes()[p] == b'\r' {
            todo!();
        }

        let offset = original_length - s.len();
        vs.push((offset, &s[..p]));
        s = &s[p + 1..];
    }

    if !s.is_empty() {
        let offset = original_length - s.len();
        vs.push((offset, s));
    }

    vs
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
            eprintln!("Failed to read file. Reason: {:?}", err.reason);
            if let Some(location) = err.location {
                let lines = lines_with_offset(&content);
                let start_line =
                    match lines.binary_search_by_key(&location.start(), |(offset, _)| *offset) {
                        Ok(v) => v,
                        Err(v) => v - 1,
                    };
                let end_line =
                    match lines.binary_search_by_key(&location.end(), |(offset, _)| *offset) {
                        Ok(v) => v,
                        Err(v) => v - 1,
                    };

                const CTX_LINES: usize = 2;
                let ctx_start_line = start_line.saturating_sub(CTX_LINES);
                let ctx_end_line = end_line.saturating_add(1 + CTX_LINES).min(lines.len());

                eprintln!("[{path}:{}]:", ctx_start_line + 1);
                for line in ctx_start_line..start_line {
                    let (_, line) = lines[line];
                    eprintln!("| {line}");
                }

                if start_line == end_line {
                    let (offset, line) = lines[start_line];
                    eprintln!("> {line}");
                    eprintln!(
                        "  {:start_pad$}{:len$}",
                        "",
                        "^",
                        start_pad = location.start() - offset,
                        len = location.len()
                    );
                } else {
                    let (offset, line) = lines[start_line];
                    eprintln!("> {line}");
                    eprintln!(
                        "  {:start_pad$}{:len$}",
                        "",
                        "^",
                        start_pad = location.start() - offset,
                        len = line.len() - location.start() - offset,
                    );

                    for line in start_line + 1..end_line {
                        let (_, line) = lines[line];
                        eprintln!("> {line}");
                        eprintln!("  {:len$}", "^", len = line.len(),);
                    }

                    let (offset, line) = lines[end_line];
                    eprintln!("> {line}");
                    eprintln!("  {:len$}", "^", len = location.end() - offset,);
                }

                for line in end_line.saturating_add(1).min(ctx_end_line)..ctx_end_line {
                    let (_, line) = lines[line];
                    eprintln!("| {line}");
                }
            }
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
