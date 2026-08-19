use std::io::{self, ErrorKind};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

mod position;

use position::SectionPosition;
use trva::isa::Isa;
use trva::{Assembled, Assembler, SectionPositions};

/// A tiny RISC-V assembler that outputs raw section data.
///
/// This performs two-pass assembly with basic relaxation.
#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Path to assembly file
    asm: PathBuf,

    /// Path to output file (default to standard output)
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Target ISA specifier, e.g. `rv32im` or `rv64i`
    #[arg(long, value_name = "ISA", default_value = "rv32im")]
    isa: Isa,

    /// Start address of the `.text` section
    #[arg(long, value_name = "ADDR")]
    text: SectionPosition,
    /// Start address of the `.data` section
    #[arg(long, value_name = "ADDR")]
    data: SectionPosition,
    /// Start address of the `.rodata` section
    #[arg(long, value_name = "ADDR")]
    rodata: SectionPosition,
    /// Start address of the `.bss` section
    #[arg(long, value_name = "ADDR")]
    bss: SectionPosition,
}

fn main() -> ExitCode {
    let Args {
        asm,
        output,
        isa,
        text,
        data,
        rodata: ro_data,
        bss,
    } = Args::parse();

    let asm_str = match std::fs::read_to_string(&asm) {
        Ok(asm) => asm,
        Err(err) => {
            match err.kind() {
                ErrorKind::NotFound => {
                    eprintln!("No such file: {}", asm.display());
                }
                ErrorKind::PermissionDenied => {
                    eprintln!("Unable to read {}: permission denied", asm.display());
                }
                ErrorKind::IsADirectory => {
                    eprintln!(
                        "Unable to read {}: is a directory, not a file",
                        asm.display()
                    );
                }
                ErrorKind::InvalidData => {
                    eprintln!("Unable to read {}: invalid UTF-8", asm.display());
                }
                _ => {
                    eprintln!("Unable to read {}: {err}", asm.display());
                }
            }
            return ExitCode::FAILURE;
        }
    };

    let mut b = Assembler::new(
        isa,
        SectionPositions {
            text: text.0,
            data: data.0,
            rodata: ro_data.0,
            bss: bss.0,
        },
    );

    if let Err(err) = b.add_source(&asm_str) {
        eprintln!("Unable to assemble {}: {err}", asm.display());
        return ExitCode::FAILURE;
    };

    let result = b.assemble();
    let mut output: io::BufWriter<Box<dyn io::Write>> = match output {
        Some(path) => {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create(true);
            match options.open(&path) {
                Ok(f) => io::BufWriter::new(Box::new(f)),
                Err(err) => {
                    eprintln!("Unable to open output file {}: {err}", path.display());
                    return ExitCode::FAILURE;
                }
            }
        }
        None => std::io::BufWriter::new(Box::new(io::stdout())),
    };

    let Assembled {
        labels: _,
        symbols: _,
        text_start,
        data_start,
        rodata_start,
        bss_start,
        text,
        data,
        rodata,
        bss,
    } = result;

    let result = (|| {
        output_section(&mut output, "text", text.as_ref(), text_start)?;
        output_section(&mut output, "data", data.as_ref(), data_start)?;
        output_section(&mut output, "rodata", rodata.as_ref(), rodata_start)?;
        output_section(&mut output, "bss", bss.as_ref(), bss_start)?;
        io::Result::Ok(())
    })();

    if let Err(err) = result {
        eprintln!("Unable to write output: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn output_section(f: &mut impl io::Write, name: &str, bytes: &[u8], start: u32) -> io::Result<()> {
    let end = start + bytes.len() as u32;
    writeln!(f, "-- .{name} ({start:#010X}-{end:#010X}) --")?;
    for &b in bytes.as_ref() {
        write!(f, "{b:02X}")?;
    }
    writeln!(f)?;

    Ok(())
}
