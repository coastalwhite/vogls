use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use vogls::ExecutionContext;
use vogls_ir::{GlobalContext, LogicMode};

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

    #[arg(long = "trace")]
    trace: bool,
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
        emit_unoptimized_ir: bool,
        #[arg(long, default_value_t = 0)]
        opt_rounds: u8,
        #[arg(long)]
        constant_propagation: bool,
        #[arg(long)]
        deadcode_elimination: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(command) = &args.command {
        match command {
            Commands::Vir {
                path,
                emit_unoptimized_ir,
                opt_rounds,
                constant_propagation,
                deadcode_elimination,
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
                        constant_propagation: *constant_propagation,
                        deadcode_elimination: *deadcode_elimination,
                    },
                );

                for signal in gl.signals.values() {
                    println!("{}", signal.display());
                }
                println!();
                for process in gl.processes.values() {
                    println!("{}", process.display(&gl));
                }
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
            trace: args.trace,
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
