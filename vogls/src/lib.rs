use std::collections::{BinaryHeap, HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use slotmap::SlotMap;
use vogls_ir::{Bits, ContextFormat, GlobalContext, Type};
use vogls_sim::{
    Context, Event, ScheduledEvent, SignalValue, VmProcess, VmProcessKey, lower_process_to_vm,
};
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{
    CaseGenerateConstruct, CaseGenerateItem, GenerateBlock, IfGenerateConstruct,
    LoopGenerateConstruct, Module, ModuleItem, ModuleOrGenerateItem, NonPortModuleItem,
};
use vogls_verilog::lower::{
    Diagnostics as LowerDiagnostics, ModuleArgs, ModuleInitialization, VTypeTable,
    fetch_module_interface, lower_module_to_ir,
};
use vogls_verilog::parser::{
    AstArenas, Diagnostics as ParserDiagnostics, ParserScratches, TokenWalker, parse_file, report,
    report_error,
};
use vogls_verilog::tokenizer::Tokenized;

pub struct ExecutionContext {
    pub stdout: Box<dyn std::io::Write>,
    pub stderr: Box<dyn std::io::Write>,
    pub output_ir: bool,
    pub output_elaborated: bool,
    pub output_sim_ir: bool,
    pub output_schedule: bool,
}

fn append_referenced_modules_generate_block<'a>(
    arenas: &'a AstArenas,
    generate_block: AstId<GenerateBlock>,
    referenced: &mut HashSet<&'a str>,
) {
    match arenas.get(generate_block) {
        GenerateBlock::ModuleOrGenerateItem(id) => {
            append_referenced_modules(arenas, *id, referenced)
        }
        GenerateBlock::BeginEnd(_, ids) => {
            for id in ids.iter() {
                append_referenced_modules(arenas, id, referenced);
            }
        }
    }
}

fn append_referenced_modules_opt_generate_block<'a>(
    arenas: &'a AstArenas,
    generate_block: AstId<Option<GenerateBlock>>,
    referenced: &mut HashSet<&'a str>,
) {
    match arenas.get(generate_block) {
        None => {}
        Some(GenerateBlock::ModuleOrGenerateItem(id)) => {
            append_referenced_modules(arenas, *id, referenced)
        }
        Some(GenerateBlock::BeginEnd(_, ids)) => {
            for id in ids.iter() {
                append_referenced_modules(arenas, id, referenced);
            }
        }
    }
}

fn append_referenced_modules<'a>(
    arenas: &'a AstArenas,
    module_or_generate_item: AstId<ModuleOrGenerateItem>,
    referenced: &mut HashSet<&'a str>,
) {
    match arenas.get(module_or_generate_item) {
        ModuleOrGenerateItem::ModuleInstantiation(module_instantiation) => {
            let module_instantiation = arenas.get(*module_instantiation);
            let module_name = arenas.get_ident(module_instantiation.module_identifier.item.0);
            referenced.insert(module_name);
        }
        ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(_) => {}
        ModuleOrGenerateItem::LocalParameterDeclaration => {}
        ModuleOrGenerateItem::ParameterOverride => {}
        ModuleOrGenerateItem::ContinuousAssign(_) => {}
        ModuleOrGenerateItem::GateInstantiation(_) => {}
        ModuleOrGenerateItem::UdpInstantiation => {}
        ModuleOrGenerateItem::InitialConstruct(_) => {}
        ModuleOrGenerateItem::AlwaysConstruct(_) => {}
        ModuleOrGenerateItem::LoopGenerateConstruct(loop_generate_construct) => {
            let LoopGenerateConstruct {
                initialization: _,
                condition: _,
                iteration: _,
                block,
            } = arenas.get(*loop_generate_construct);
            append_referenced_modules_generate_block(arenas, *block, referenced);
        }
        ModuleOrGenerateItem::IfGenerateConstruct(if_generate_construct) => {
            let IfGenerateConstruct {
                condition: _,
                truthy,
                falsy,
            } = arenas.get(*if_generate_construct);
            append_referenced_modules_opt_generate_block(arenas, *truthy, referenced);
            if let Some(falsy) = falsy {
                append_referenced_modules_opt_generate_block(arenas, *falsy, referenced);
            }
        }
        ModuleOrGenerateItem::CaseGenerateConstruct(case_generate_construct) => {
            let CaseGenerateConstruct { value: _, items } = arenas.get(*case_generate_construct);
            for item in items.iter() {
                let CaseGenerateItem { pattern: _, block } = arenas.get(item);
                append_referenced_modules_opt_generate_block(arenas, *block, referenced);
            }
        }
    }
}

pub fn run(
    path: &Path,
    top_level_module: Option<&str>,
    ectx: &mut ExecutionContext,
) -> Result<(), Box<dyn std::error::Error>> {
    let content: Rc<str> = std::fs::read_to_string(&path)?.into();
    let token_buffer = Tokenized::tokenize(content.clone(), Some(path.into()));
    let mut tkw = TokenWalker::new(&token_buffer);
    let mut diagnostics = ParserDiagnostics::default();

    let ast = match parse_file(
        &mut tkw,
        &mut ParserScratches::default(),
        Some(&mut diagnostics),
    ) {
        Ok(ast) => ast,
        Err(_) => {
            for (location, err) in &diagnostics.errors {
                let mut out = String::new();
                report_error(&token_buffer, err.clone(), *location, &mut out)?;
                write!(ectx.stderr, "{out}")?;
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
    let module_to_ast_lut =
        HashMap::<&str, AstId<Module>>::from_iter(ast.modules.iter().map(|module_id| {
            let module = ast.arenas.get(module_id);
            (
                ast.arenas.get_ident(module.module_identifier.item.0),
                module_id,
            )
        }));

    let tl_module_name = match top_level_module {
        Some(v) => v,
        None => {
            let mut referenced = HashSet::new();
            for module_id in ast.modules {
                let Module {
                    attribute_instances: _,
                    module_identifier: _,
                    module_parameter_port_list: _,
                    module_items,
                    ports: _,
                } = ast.arenas.get(module_id);
                for module_item in module_items.iter() {
                    let ModuleItem::NonPortModuleItem(p) = ast.arenas.get(module_item) else {
                        continue;
                    };

                    if let NonPortModuleItem::ModuleOrGenerateItem(module_item) = ast.arenas.get(*p)
                    {
                        append_referenced_modules(&ast.arenas, *module_item, &mut referenced);
                    }
                }
            }

            let mut top_level_modules = Vec::new();
            for module_id in ast.modules {
                let Module {
                    attribute_instances: _,
                    module_identifier,
                    module_parameter_port_list: _,
                    module_items: _,
                    ports: _,
                } = ast.arenas.get(module_id);
                let module_name = ast.arenas.get_ident(module_identifier.item.0);
                if referenced.contains(module_name) {
                    continue;
                }
                top_level_modules.push((module_id, module_name));
            }

            if top_level_modules.len() == 0 {
                return Err(<Box<dyn std::error::Error>>::from(
                    "no top-level module found".to_string(),
                ));
            } else if top_level_modules.len() > 1 {
                let names = top_level_modules
                    .iter()
                    .map(|(_, n)| *n)
                    .collect::<Vec<&str>>();
                writeln!(
                    ectx.stderr,
                    "[ERR]: Found {} possible top-level modules: {names:?}",
                    top_level_modules.len()
                )?;
                let mut out = String::new();
                for (m, _) in top_level_modules {
                    out.clear();
                    let m = ast.arenas.get(m);
                    let span = ast.arenas.get_item_span(m.module_identifier);
                    report(&token_buffer, span, &mut out)?;
                    writeln!(ectx.stderr, "{out}").unwrap();
                }
                return Err("ambiguous top-level module".into());
            } else {
                top_level_modules[0].1
            }
        }
    };
    let Some(tl_module) = module_lut.get(tl_module_name) else {
        return Err(<Box<dyn std::error::Error>>::from(
            "cannot find top-level module".to_string(),
        ));
    };

    // Walk the modules in depth-first order and lower to IR.
    let mut error = false;
    let mut diagnostics = LowerDiagnostics::default();
    let mut types = VTypeTable::new();
    let mut next_modules = Vec::<ModuleInitialization>::new();
    let Ok((top_level_params, top_level_io, parameters)) = fetch_module_interface(
        &mut gl,
        &ast.arenas,
        &mut types,
        ast.modules.get(*tl_module),
        &[],
        &mut diagnostics,
    ) else {
        return Err("top_level fetch_module error".into());
    };
    assert!(top_level_io.ports.is_empty());
    next_modules.push(ModuleInitialization {
        name: tl_module_name,
        parameters: top_level_params,
        io: top_level_io,
        args: ModuleArgs {
            parameters,
            signals: Vec::new(),
        },
    });
    while let Some(init) = next_modules.pop() {
        let ModuleInitialization {
            name,
            parameters,
            io,
            args,
        } = &init;
        let module_id = ast.modules.get(module_lut[name]);
        let module_key = lower_module_to_ir(
            &mut gl,
            &ast.arenas,
            &mut types,
            module_id,
            &parameters,
            &io,
            &args,
            &module_to_ast_lut,
            &mut next_modules,
            &mut diagnostics,
        );
        error |= module_key.is_err();
    }

    if !diagnostics.warnings.is_empty() {
        for (location, warning) in &diagnostics.warnings {
            writeln!(ectx.stderr, "[WARN]: {warning}")?;
            let mut out = String::new();
            report(&token_buffer, *location, &mut out)?;
            writeln!(ectx.stderr, "{out}")?;
        }
    }

    if error {
        for (location, err, context) in &diagnostics.errors {
            let mut out = String::new();
            report_error(&token_buffer, err.clone(), *location, &mut out)?;
            write!(ectx.stderr, "{out}")?;
            if !context.is_empty() {
                writeln!(ectx.stderr, "context:")?;
                for c in context {
                    writeln!(ectx.stderr, "- {c}")?;
                }
            }
            writeln!(ectx.stderr)?;
        }
        return Err("failed to lower".into());
    }

    if ectx.output_ir {
        for process in gl.processes.values() {
            writeln!(ectx.stdout, "{}", process.display(&gl))?;
        }
    }

    let mut processes = SlotMap::<VmProcessKey, VmProcess>::default();
    let mut schedule = BinaryHeap::default();
    let mut signals = HashMap::default();
    let mut listeners = SlotMap::default();
    let mut watches = HashMap::default();

    // // Find the entity for the Top-Level Module.
    // let mut elab_processes = Vec::new();
    // elaborate::elaborate(tl_module_key, &mut gl, &mut elab_processes);

    let mut io_signals = HashMap::new();
    for process in gl.processes.keys() {
        if ectx.output_elaborated {
            println!();
            println!("{}", gl.processes[process].display(&gl));
        }
        let vm_process = lower_process_to_vm(process, &gl, &mut io_signals);

        if ectx.output_sim_ir {
            print!("{}", &vm_process);
        }

        let bit_stack = vec![0u8; vm_process.bit_stack_size];
        let decimal_stack = vec![0i64; vm_process.decimal_stack_size];
        let vm_process_key = processes.insert(vm_process);

        if ectx.output_sim_ir {
            println!(": {vm_process_key:?}");
        }

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
        let value = match (
            gl.types[gl.signals[ir_signal].ty.key],
            gl.signals[ir_signal].ty.width,
        ) {
            (Type::Bits(n), None) if n < 64 => SignalValue::Bits(Bits::Small(0, n)),
            (Type::Bits(n), Some(width)) if n < 64 => SignalValue::BitsArray(
                std::iter::repeat_n(Bits::Small(0, n), width.get() as usize).collect(),
            ),
            (Type::Bits(n), None) => SignalValue::Bits(Bits::Big(
                n,
                std::iter::repeat_n(0, n.div_ceil(8) as usize).collect(),
            )),
            (Type::Bits(n), Some(width)) => SignalValue::BitsArray(
                std::iter::repeat_n(
                    Bits::Big(n, std::iter::repeat_n(0, n.div_ceil(8) as usize).collect()),
                    width.get() as usize,
                )
                .collect(),
            ),
            (Type::Decimal, None) => SignalValue::Decimal(0),
            (Type::Decimal, Some(width)) => {
                SignalValue::DecimalArray(std::iter::repeat_n(0, width.get() as usize).collect())
            }
        };
        signals.insert(signal, value);
    }

    if ectx.output_schedule {
        println!("{schedule:?}");
    }

    let stdout = std::mem::replace(&mut ectx.stdout, Box::new(Vec::new()) as _);
    let stderr = std::mem::replace(&mut ectx.stderr, Box::new(Vec::new()) as _);

    let mut ctx = Context::new(stdout, stderr);
    let fail = vogls_sim::run(
        &mut ctx,
        &processes,
        &mut schedule,
        &mut signals,
        &mut listeners,
        &mut watches,
        &gl.types,
        100,
    )
    .is_err();

    ectx.stdout = ctx.stdout;
    ectx.stderr = ctx.stderr;

    if fail {
        return Err("execution failed.".into());
    }

    Ok(())
}
