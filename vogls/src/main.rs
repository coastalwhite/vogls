use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use clap::Parser;
use vogls::design::Design;
use vogls::{ExecutionContext, SimulationIo};
use vogls_ir::LogicMode;
use vogls_ir::optimize::OptFlags;
use vogls_utils::TimerStack;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    path: Vec<PathBuf>,

    #[arg(long)]
    vir: bool,

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
    no_constant_propagation: bool,
    #[arg(long)]
    no_deadcode_elimination: bool,
    #[arg(long)]
    no_common_subexpr_elim: bool,
    #[arg(long)]
    no_peephole_optimization: bool,

    #[arg(long)]
    print_unoptimized_fuse_signals: bool,
    #[arg(long)]
    print_round_fuse_signals: bool,
    #[arg(long)]
    print_optimized_fuse_signals: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let logic_mode = if args.four_value_logic {
        LogicMode::FourValue
    } else {
        LogicMode::TwoValue
    };
    let mut ectx = ExecutionContext {
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
        opt: OptFlags {
            opt_rounds: args.opt_rounds,
            constant_propagation: !args.no_constant_propagation,
            deadcode_elimination: !args.no_deadcode_elimination,
            common_subexpr_elim: !args.no_common_subexpr_elim,
            peephole: !args.no_peephole_optimization,
        },
        logic_mode,
        vcd: args.vcd,
        compile: args.compile,
        output_source: args.output_source,
        timings: args.timings,
        print_unoptimized_fuse_signals: args.print_unoptimized_fuse_signals,
        print_round_fuse_signals: args.print_round_fuse_signals,
        print_optimized_fuse_signals: args.print_optimized_fuse_signals,
    };

    let mut timers = TimerStack::new(ectx.timings);
    if args.vir {
        let content = read_to_string(&args.path[0])?;
        let design = timers.timed("compilation", |timers| {
            Design::new_vir(&content, timers, &mut ectx)
        })?;

        if args.no_run {
            return Ok(());
        }

        timers.start("simulation");
        design
            .run(
                &mut SimulationIo::new(Box::new(std::io::stdout()), Box::new(std::io::stderr())),
                1000,
            )
            .map_err(|_| "simulation failed")?;
        timers.stop();
    } else {
        let paths: Vec<&Path> = args.path.iter().map(|p| p.as_path()).collect();
        vogls::run(
            &paths,
            &mut timers,
            args.top_level_module.as_deref(),
            &mut ectx,
        )?;
    }

    if timers.enabled {
        timers.print();
    }

    Ok(())
}
