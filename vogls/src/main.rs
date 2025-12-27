use std::path::{Path, PathBuf};

use clap::Parser;
use vogls::ExecutionContext;
use vogls_sim::TracingLevel;

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
    #[arg(long = "emit-ir")]
    emit_ir: bool,
    #[arg(long = "emit-vm")]
    emit_vm: bool,

    #[arg(short, long)]
    time: u64,
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
            emit_ir: args.emit_ir,
            emit_vm: args.emit_vm,
            trace: args.trace.then_some(TracingLevel::Events).unwrap_or(TracingLevel::None),
            time: args.time,
        },
    )?;

    Ok(())
}
