use std::fs::read_to_string;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use clap::Parser;
use vogls::design::{Arena, Macro};
use vogls::ir::LogicMode;
use vogls::ir::optimize::OptFlags;
use vogls::utils::TimerStack;
use vogls::{DesignBuilder, DesignBuilderError, Optimizations, SimulationIo, VirDesignBuilder};

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    path: Vec<PathBuf>,

    #[arg(long)]
    vir: bool,

    #[arg(short = 'm', long = "top-level-module")]
    top_level_module: Option<String>,

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
    print_vm_map: bool,

    #[arg(long)]
    emit_unoptimized_fuse_graph: bool,
    #[arg(long)]
    emit_optimized_fuse_graph: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Args {
        path,
        vir,
        top_level_module,
        itrace,
        stats,
        debug_symbols,
        emit_hierarchy,
        emit_unoptimized_ir,
        emit_ir,
        emit_vm,
        emit_process_stats,
        no_run,
        time,
        opt_rounds,
        four_value_logic,
        vcd,
        sdf,
        defines,
        compile,
        output_source,
        timings,
        no_constant_propagation,
        no_deadcode_elimination,
        no_common_subexpr_elim,
        no_peephole_optimization,
        print_vm_map,
        emit_unoptimized_fuse_graph,
        emit_optimized_fuse_graph,
    } = Args::parse();
    let logic_mode = if four_value_logic {
        LogicMode::FourValue
    } else {
        LogicMode::TwoValue
    };

    let mut timers = TimerStack::new(timings);

    let mut lowered = if vir {
        let content = read_to_string(&path[0])?;
        let mut builder = VirDesignBuilder::new(&content);
        builder.with_logic_mode(logic_mode);
        builder.parse()?
    } else {
        let mut builder = DesignBuilder::new();
        match logic_mode {
            LogicMode::TwoValue => {
                builder.define_macro("__VOGLS__TWO_VALUE_LOGIC", Macro::default());
            }
            LogicMode::FourValue => {}
        }
        for name in &defines {
            builder.define_macro(name, Macro::default());
        }
        timers.timed("tokenization", |_| {
            for path in &path {
                builder.add_source(path)?;
            }
            Result::<_, DesignBuilderError>::Ok(())
        })?;

        let mut arena = Arena::default();
        let design = match builder.parse(&mut arena) {
            Ok(design) => design,
            Err(err) => {
                eprintln!("{err}");
                return Err("failed to parse".into());
            }
        };

        let mut elaborate = match design.elaborate(logic_mode, top_level_module) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("{err}");
                return Err("failed to elaborate".into());
            }
        };

        if emit_unoptimized_fuse_graph {
            elaborate.set_unoptimized_fuse_graphs_emit(Arc::new(Mutex::new(stdout())) as _);
        }
        if emit_optimized_fuse_graph {
            elaborate.set_optimized_fuse_graphs_emit(Arc::new(Mutex::new(stdout())) as _);
        }

        if emit_hierarchy {
            eprintln!("{}", elaborate.display_hierarchy());
        }

        if let Some(sdf_path) = sdf.as_deref() {
            if let Err(err) = elaborate.annotate_sdf(sdf_path) {
                eprintln!("{err}");
                return Err("failed to annotate sdf".into());
            }
        }

        timers.start("lower_specify_blocks");
        if let Err(err) = elaborate.annotate_specify() {
            eprintln!("{err}");
            return Err("failed to annotate specify".into());
        }
        timers.stop();

        let lowered = match elaborate.lower(vec![]) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("{err}");
                return Err("failed to lower".into());
            }
        };
        arena.reset();
        lowered
    };

    if emit_unoptimized_ir {
        println!("{}", lowered.emit_ir());
    }

    timers.timed("optimization", |_| {
        let mut flags = OptFlags::ALL;
        flags.set(OptFlags::CONSTANT_PROPAGATION, !no_constant_propagation);
        flags.set(OptFlags::DEADCODE_ELIMINATION, !no_deadcode_elimination);
        flags.set(OptFlags::COMMON_SUBEXPR_ELIM, !no_common_subexpr_elim);
        flags.set(OptFlags::PEEPHOLE, !no_peephole_optimization);
        lowered.optimize(Optimizations {
            rounds: opt_rounds,
            flags,
        });
    });

    if emit_ir {
        println!("{}", lowered.emit_ir());
    }

    if emit_process_stats {
        println!("Process Kind Counts:");
        for (kind, count) in lowered.process_stats().iter() {
            println!("  {kind}: {count}");
        }
    }

    if let Some(vcd) = &vcd {
        lowered.trace_vcd(vcd.clone());
    }
    lowered.itrace = itrace;
    lowered.emit_vm = emit_vm;
    lowered.stats = stats;
    lowered.debug_symbols = debug_symbols;
    lowered.output_source = output_source.clone();
    lowered.print_vm_map = print_vm_map;

    timers.start("compilation");
    let mut design = if compile {
        lowered.compile()
    } else {
        lowered.to_bytecode()
    }?;
    timers.stop();

    if no_run {
        if timers.enabled {
            timers.print();
        }

        return Ok(());
    }

    timers.start("simulation");
    design
        .run(
            &mut SimulationIo::new(Box::new(std::io::stdout()), Box::new(std::io::stderr())),
            time,
        )
        .map_err(|_| "simulation failed")?;
    timers.stop();

    if stats {
        design.initial_state().runtime().dump_stats(&mut stdout())?;
    }

    if timers.enabled {
        timers.print();
    }

    Ok(())
}
