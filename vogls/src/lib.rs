use std::collections::{BinaryHeap, HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use slotmap::SlotMap;
use vogls_ir::{Bits, ContextFormat, GlobalContext, Type, Value};
use vogls_sim::{Context, Event, ScheduledEvent, VmProcess, VmProcessKey, lower_process_to_vm};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{Module, ModuleItem, ModuleOrGenerateItem, NonPortModuleItem};
use vogls_verilog::lower::lower_module_to_ir;
use vogls_verilog::parser::{Diagnostics, Parser, TokenWalker, report_error};
use vogls_verilog::tokenizer::Tokenized;

mod elaborate;

pub struct ExecutionContext {
    pub stdout: Box<dyn std::io::Write>,
    pub stderr: Box<dyn std::io::Write>,
}

pub fn run(
    path: &Path,
    top_level_module: Option<&str>,
    ectx: &mut ExecutionContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let content: Rc<str> = std::fs::read_to_string(&path)?.into();
    let token_buffer = Tokenized::tokenize(content.clone(), Some(path.into()));
    let mut parser = Parser::new(TokenWalker::new(&token_buffer));
    let mut diagnostics = Diagnostics::default();

    let ast = match parser.parse_file(Some(&mut diagnostics)) {
        Ok(ast) => ast,
        Err(_) => {
            for (location, err) in &diagnostics.errors {
                let mut out = String::new();
                report_error(&token_buffer, err.clone(), *location, &mut out)?;
                write!(ectx.stdout, "{out}")?;
            }
            return Err("failed to parse".into());
        }
    };
    let mut gl = GlobalContext::default();

    let module_lut =
        HashMap::<&str, usize>::from_iter(ast.modules.iter().enumerate().map(|(i, module_id)| {
            let module = ast.arenas.get(module_id);
            (ast.arenas.get_ident(module.module_identifier.item.0), i)
        }));

    let tl_module_name = match top_level_module {
        Some(v) => v,
        None => {
            let mut referenced = HashSet::new();
            for module_id in ast.modules {
                let Module {
                    module_identifier: _,
                    module_items,
                    ports: _,
                } = ast.arenas.get(module_id);
                for module_item in module_items.iter() {
                    let ModuleItem::NonPortModuleItem(p) = ast.arenas.get(module_item) else {
                        continue;
                    };

                    if let NonPortModuleItem::ModuleOrGenerateItem(module_item) = ast.arenas.get(*p)
                    {
                        if let ModuleOrGenerateItem::ModuleInstantiation(module_instantiation) =
                            ast.arenas.get(*module_item)
                        {
                            let module_instantiation = ast.arenas.get(*module_instantiation);
                            let module_name = ast
                                .arenas
                                .get_ident(module_instantiation.module_identifier.item.0);
                            referenced.insert(module_name);
                        }
                    }
                }
            }

            let mut top_level_modules = Vec::new();
            for module_id in ast.modules {
                let Module {
                    module_identifier,
                    module_items: _,
                    ports: _,
                } = ast.arenas.get(module_id);
                let module_name = ast.arenas.get_ident(module_identifier.item.0);
                if referenced.contains(module_name) {
                    continue;
                }
                top_level_modules.push(module_name);
            }

            if top_level_modules.len() == 0 {
                return Err(<Box<dyn std::error::Error>>::from(
                    "no top-level module found".to_string(),
                ));
            } else if top_level_modules.len() > 1 {
                return Err(<Box<dyn std::error::Error>>::from(format!(
                    "multiple top-level modules: {top_level_modules:?}"
                )));
            } else {
                top_level_modules[0]
            }
        }
    };
    let Some(tl_module) = module_lut.get(tl_module_name) else {
        return Err(<Box<dyn std::error::Error>>::from(
            "cannot find top-level module".to_string(),
        ));
    };

    // Create a list of all module in breadth- first order.
    // @TODO: Add a mechanism for detecting dependency loops.
    let mut module_stack: Vec<AstId<Module>> = Vec::new();
    let mut module_seen: HashSet<&str> = HashSet::new();
    module_stack.push(ast.modules.get(*tl_module));
    module_seen.insert(tl_module_name);
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

    let tl_module_key = *instantiated_modules.get(tl_module_name).unwrap();

    for module in gl.modules.values() {
        writeln!(ectx.stdout, "{}", module.display(&gl))?;
    }

    let mut processes = SlotMap::<VmProcessKey, VmProcess>::default();
    let mut schedule = BinaryHeap::default();
    let mut signals = HashMap::default();
    let mut listeners = SlotMap::default();
    let mut watches = HashMap::default();

    // Find the entity for the Top-Level Module.
    let mut elab_processes = Vec::new();
    elaborate::elaborate(tl_module_key, &mut gl, &mut elab_processes);

    let mut io_signals = HashMap::new();
    for &process in elab_processes.iter() {
        writeln!(ectx.stdout)?;
        writeln!(ectx.stdout, "{}", gl.processes[process].display(&gl))?;
        let vm_process = lower_process_to_vm(process, &gl, &mut io_signals);

        write!(ectx.stdout, "{}", &vm_process)?;

        let bit_stack = vec![0u8; vm_process.bit_stack_size];
        let decimal_stack = vec![0i64; vm_process.decimal_stack_size];
        let vm_process_key = processes.insert(vm_process);

        writeln!(ectx.stdout, ": {vm_process_key:?}")?;

        schedule.push(ScheduledEvent {
            at: 0,
            event: Event {
                process: vm_process_key,
                bit_stack,
                decimal_stack,
                ip: 0,
            },
        });
    }

    for (ir_signal, signal) in io_signals {
        let value = match &gl.signals[ir_signal].ty {
            Type::Bits(n) => Value::Bits(Bits::Small(0, *n)),
            Type::Decimal => Value::Decimal(0),
        };
        signals.insert(signal, value);
    }

    writeln!(ectx.stdout, "{schedule:?}")?;

    let stdout = std::mem::replace(&mut ectx.stdout, Box::new(Vec::new()) as _);
    let stderr = std::mem::replace(&mut ectx.stdout, Box::new(Vec::new()) as _);

    let mut ctx = Context::new(stdout, stderr);
    vogls_sim::run(
        &mut ctx,
        &processes,
        &mut schedule,
        &mut signals,
        &mut listeners,
        &mut watches,
        100,
    );

    ectx.stdout = ctx.stdout;
    ectx.stderr = ctx.stderr;

    Ok(())
}
