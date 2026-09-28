use clap::Parser;

use self::integration::IntegrationArgs;
use self::unit::UnitArgs;

static ANSI_RED: &str = "\x1b[31m";
static ANSI_GREEN: &str = "\x1b[32m";
static ANSI_END: &str = "\x1b[0m";

mod integration;
mod unit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Bytecode,
    Cranelift,
}

#[derive(clap::Parser)]
enum Command {
    Unit(UnitArgs),
    Integration(IntegrationArgs),
}

fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error + Send + Sync>> {
    match Command::parse() {
        Command::Unit(args) => args.run(),
        Command::Integration(args) => args.run(),
    }
}
