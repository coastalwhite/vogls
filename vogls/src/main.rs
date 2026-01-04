use std::path::{Path, PathBuf};

use clap::Parser;
use vogls::ExecutionContext;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    path: PathBuf,

    #[arg(short = 'm', long = "top-level-module")]
    top_level_module: Option<String>,

    #[arg(short, long)]
    filter: Option<String>,

    #[arg(long = "trace")]
    trace: bool,
    #[arg(long = "emit-hierarchy")]
    emit_hierarchy: bool,
    #[arg(long = "emit-unoptimized-ir")]
    emit_unoptimized_ir: bool,
    #[arg(long = "emit-ir")]
    emit_ir: bool,
    #[arg(long = "emit-vm")]
    emit_vm: bool,

    #[arg(short, long, default_value_t = u64::MAX)]
    time: u64,

    #[arg(long = "opt-rounds", default_value_t = 0)]
    opt_rounds: u8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let path = Path::new(&args.path);
    vogls::run(
        &path,
        args.top_level_module.as_deref(),
        &mut ExecutionContext {
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            emit_hierarchy: args.emit_hierarchy,
            emit_unoptimized_ir: args.emit_unoptimized_ir,
            emit_ir: args.emit_ir,
            emit_vm: args.emit_vm,
            trace: args.trace,
            time: args.time,
            opt_rounds: args.opt_rounds,
        },
    )?;

    Ok(())
}
