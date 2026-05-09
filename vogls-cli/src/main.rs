use std::fs::read_to_string;
use std::io::stdout;
use std::path::{Path, PathBuf};

use clap::Parser;
use vogls::design::Design;
use vogls::{ExecutionContext, SimulationIo};
use vogls::ir::LogicMode;
use vogls::ir::optimize::OptFlags;
use vogls::utils::TimerStack;

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

    #[arg(long)]
    itrace: bool,
    #[arg(long)]
    stats: bool,
    #[arg(long)]
    debug_symbols: bool,

    #[arg(long = "emit-hierarchy")]
    emit_hierarchy: bool,
    #[arg(long = "emit-unoptimized-ir")]
    emit_unoptimized_ir: bool,
    #[arg(long = "emit-ir")]
    emit_ir: bool,
    #[arg(long = "emit-vm")]
    emit_vm: bool,
    #[arg(long = "emit-process-stats")]
    emit_process_stats: bool,

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
    #[arg(long = "sdf")]
    sdf: Option<PathBuf>,

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
        emit_process_stats: args.emit_process_stats,
        itrace: args.itrace,
        stats: args.stats,
        debug_symbols: args.debug_symbols,
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
        sdf: args.sdf,
        compile: args.compile,
        output_source: args.output_source,
        timings: args.timings,
        print_unoptimized_fuse_signals: args.print_unoptimized_fuse_signals,
        print_round_fuse_signals: args.print_round_fuse_signals,
        print_optimized_fuse_signals: args.print_optimized_fuse_signals,
    };

    let mut timers = TimerStack::new(ectx.timings);
    let mut design = timers.timed("compilation", |timers| {
        if args.vir {
            let content = read_to_string(&args.path[0])?;
            Design::new_vir(&content, timers, &mut ectx)
        } else {
            let paths: Vec<&Path> = args.path.iter().map(|p| p.as_path()).collect();
            Design::new(
                &paths,
                timers,
                args.top_level_module.as_deref(),
                &mut ectx,
                Vec::new(),
            )
        }
    })?;

    if args.no_run {
        if timers.enabled {
            timers.print();
        }

        return Ok(());
    }

    timers.start("simulation");
    design
        .run(
            &mut SimulationIo::new(Box::new(std::io::stdout()), Box::new(std::io::stderr())),
            ectx.time,
        )
        .map_err(|_| "simulation failed")?;
    timers.stop();

    if args.stats {
        design.initial_state.runtime().dump_stats(&mut stdout())?;
    }

    if timers.enabled {
        timers.print();
    }

    Ok(())
}
