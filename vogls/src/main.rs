use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::{Parser, Subcommand};
use slotmap::SlotMap;
use vogls::{ExecutionContext, SimulationIo, generate_signals_heap};
use vogls_codegen::{HeapBuilder, HeapRef};
use vogls_ir::{GlobalContext, LogicMode};
use vogls_sim::{Event, ListenerKey, Regions, Simulation, VmProcessKey, lower_process_to_vm};
use vogls_utils::VgHashMap;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    path: Vec<PathBuf>,

    #[arg(short = 'm', long = "top-level-module")]
    top_level_module: Option<String>,

    #[arg(short, long)]
    filter: Option<String>,

    #[arg(long = "itrace")]
    itrace: bool,

    #[arg(long = "emit-hierarchy")]
    emit_hierarchy: bool,
    #[arg(long = "emit-unoptimized-ir")]
    emit_unoptimized_ir: bool,
    #[arg(long = "emit-ir")]
    emit_ir: bool,
    #[arg(long = "emit-vm")]
    emit_vm: bool,

    #[arg(long = "no-run")]
    no_run: bool,
    #[arg(short, long, default_value_t = u64::MAX)]
    time: u64,

    #[arg(long = "opt-rounds", default_value_t = 0)]
    opt_rounds: u8,
    #[arg(long = "fv-logic", short = 'F')]
    four_value_logic: bool,

    #[arg(long = "vcd")]
    vcd: Option<PathBuf>,

    #[arg(short = 'D')]
    defines: Vec<String>,

    #[arg(short = 'C')]
    compile: bool,
    #[arg(long)]
    output_source: Option<PathBuf>,
    #[arg(long)]
    timings: bool,

    #[arg(long)]
    print_unoptimized_fuse_signals: bool,
    #[arg(long)]
    print_round_fuse_signals: bool,
    #[arg(long)]
    print_optimized_fuse_signals: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Vir {
        path: PathBuf,
        #[arg(long)]
        itrace: bool,
        #[arg(long)]
        emit_unoptimized_ir: bool,
        #[arg(long)]
        no_run: bool,
        #[arg(long)]
        emit_ir: bool,
        #[arg(long, default_value_t = 0)]
        opt_rounds: u8,

        #[arg(long)]
        no_constant_propagation: bool,
        #[arg(long)]
        no_deadcode_elimination: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(command) = &args.command {
        match command {
            Commands::Vir {
                path,
                emit_unoptimized_ir,
                emit_ir,
                no_run,
                itrace,
                opt_rounds,
                no_constant_propagation,
                no_deadcode_elimination,
            } => {
                let content = read_to_string(path)?;
                let mut gl = GlobalContext::default();
                vogls_ir::parse::parse(&content, &mut gl)?;

                if *emit_unoptimized_ir {
                    for signal in gl.signals.values() {
                        println!("{}", signal.display());
                    }
                    println!();
                    for process in gl.processes.values() {
                        println!("{}", process.display(&gl));
                    }
                }

                let processes = gl.processes.keys().collect::<Vec<_>>();
                vogls_ir::optimize::optimize_processes(
                    &mut gl,
                    &processes,
                    vogls_ir::optimize::OptFlags {
                        opt_rounds: *opt_rounds,
                        constant_propagation: !*no_constant_propagation,
                        deadcode_elimination: !*no_deadcode_elimination,
                    },
                );

                if *emit_ir {
                    for signal in gl.signals.values() {
                        println!("{}", signal.display());
                    }
                    println!();
                    for process in gl.processes.values() {
                        println!("{}", process.display(&gl));
                    }
                }

                if *no_run {
                    return Ok(());
                }

                let mut heap_builder = HeapBuilder::new();
                let mut io_signals = VgHashMap::default();
                let mut signals = Vec::new();
                generate_signals_heap(
                    &mut heap_builder,
                    &mut io_signals,
                    &gl.signals,
                    &mut signals,
                    gl.logic_mode,
                );
                let watches = vec![Vec::new(); gl.signals.len()];
                let signals: Arc<[HeapRef]> = signals.into();
                let mut processes = Vec::new();
                let mut regions = Regions::new(4);
                let listeners = SlotMap::<ListenerKey, _>::default();
                for process in gl.processes.keys() {
                    let vm_process = lower_process_to_vm(
                        process,
                        &gl,
                        &mut heap_builder,
                        &signals,
                        &mut io_signals,
                    );
                    let vm_process_key = VmProcessKey(processes.len() as u64);
                    processes.push(vm_process);
                    regions.active.push(Event {
                        process: vm_process_key,
                        ip: 0,
                    });
                }
                let mut heap = heap_builder.finish();

                for (key, signal) in &gl.signals {
                    if let Some(initialize) = &signal.initialize {
                        assert_eq!(initialize.size(), signal.size);
                        heap.store_bits(
                            signals[io_signals[&key].as_usize()],
                            gl.logic_mode,
                            initialize,
                        );
                    }
                }

                let mut simulation = Simulation::new(processes, signals.clone(), gl.logic_mode);
                simulation.itrace = *itrace;
                let mut initial_state = simulation.new_state(regions, listeners, watches, heap);

                simulation
                    .run(
                        &mut initial_state,
                        &mut SimulationIo::new(
                            Box::new(std::io::stdout()),
                            Box::new(std::io::stderr()),
                        ),
                        1000,
                    )
                    .map_err(|_| "simulation failed")?;

                return Ok(());
            }
        }
    }

    let logic_mode = if args.four_value_logic {
        LogicMode::FourValue
    } else {
        LogicMode::TwoValue
    };

    let paths: Vec<&Path> = args.path.iter().map(|p| p.as_path()).collect();
    vogls::run(
        &paths,
        args.top_level_module.as_deref(),
        &mut ExecutionContext {
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            defines: args.defines,
            emit_hierarchy: args.emit_hierarchy,
            emit_unoptimized_ir: args.emit_unoptimized_ir,
            emit_ir: args.emit_ir,
            emit_vm: args.emit_vm,
            itrace: args.itrace,
            time: args.time,
            no_run: args.no_run,
            opt_rounds: args.opt_rounds,
            logic_mode,
            vcd: args.vcd,
            compile: args.compile,
            output_source: args.output_source,
            timings: args.timings,
            print_unoptimized_fuse_signals: args.print_unoptimized_fuse_signals,
            print_round_fuse_signals: args.print_round_fuse_signals,
            print_optimized_fuse_signals: args.print_optimized_fuse_signals,
        },
    )?;

    Ok(())
}
