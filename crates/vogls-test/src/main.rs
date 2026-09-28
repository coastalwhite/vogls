use clap::Parser;

use self::unit::UnitArgs;

static ANSI_RED: &str = "\x1b[31m";
static ANSI_GREEN: &str = "\x1b[32m";
static ANSI_END: &str = "\x1b[0m";

mod unit;

#[derive(clap::Parser)]
enum Command {
    Unit(UnitArgs),
}

fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error>> {
    match Command::parse() {
        Command::Unit(args) => args.run(),
    }
}
