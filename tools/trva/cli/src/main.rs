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
#[command(version, after_long_help = POSITION_HELP)]
struct Args {
    /// Path to assembly file
    asm: PathBuf,

    /// Path to output file (default to standard output)
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Target ISA specifier, e.g. `rv32im` or `rv64i`
    #[arg(long, value_name = "ISA", default_value = "rv32im")]
    isa: Isa,

    /// Position of the `.text` section: an address, or `after:<section>`
    #[arg(long, value_name = "POS")]
    text: SectionPosition,
    /// Position of the `.data` section: an address, or `after:<section>`
    #[arg(long, value_name = "POS")]
    data: SectionPosition,
    /// Position of the `.rodata` section: an address, or `after:<section>`
    #[arg(long, value_name = "POS")]
    rodata: SectionPosition,
    /// Position of the `.bss` section: an address, or `after:<section>`
    #[arg(long, value_name = "POS")]
    bss: SectionPosition,
}

const POSITION_HELP: &str = "\
Section positions:
  Each of --text, --data, --rodata and --bss takes either a fixed address or a
  position relative to another section:

    <address>                       0x8000_0000, 0o400, 0b1010, 4096
    after:<section>[:<attribute>]*  after:data, after:.data:align=4

  A section named by `after` may itself be placed relatively, so the sections can
  be chained in any order, as long as the chain does not close on itself.

  Sections are `text`, `data`, `rodata` and `bss`, with or without a leading dot.
  Attributes are:

    gap=<n>    leave n bytes after the preceding section
    align=<n>  round the address up to a multiple of n bytes

  `gap` is applied before `align`. Like the `.align` directive, `align` is a byte
  count rather than a power-of-two exponent.

Examples:
  Pack every section, keeping .bss 4-byte aligned:

    trva prog.s --text 0x8000_0000 --data after:text:align=4 \\
                --rodata after:data:align=4 --bss after:rodata:align=4

  Put .bss directly after .data and leave .rodata somewhere of its own:

    trva prog.s --text 0x8000_0000 --data 0x8000_1000 \\
                --rodata 0x9000_0000 --bss after:data:align=4
";

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

    let positions = SectionPositions {
        text: text.0,
        data: data.0,
        rodata: ro_data.0,
        bss: bss.0,
    };

    // Catch an unresolvable layout before doing any work on the source.
    if let Err(err) = positions.layout_order() {
        eprintln!("Unable to lay out sections: {err}");
        return ExitCode::FAILURE;
    }

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

    let mut b = Assembler::new(isa, positions);

    if let Err(err) = b.add_source(&asm_str) {
        eprintln!("Unable to assemble {}: {err}", asm.display());
        return ExitCode::FAILURE;
    };

    let result = match b.assemble() {
        Ok(result) => result,
        Err(err) => {
            eprintln!("Unable to lay out sections: {err}");
            return ExitCode::FAILURE;
        }
    };
    let mut output: io::BufWriter<Box<dyn io::Write>> = match output {
        Some(path) => {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create(true).truncate(true);
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
        use io::Write;
        output_section(&mut output, "text", text.as_ref(), text_start)?;
        output_section(&mut output, "data", data.as_ref(), data_start)?;
        output_section(&mut output, "rodata", rodata.as_ref(), rodata_start)?;
        let end = bss_start + bss as u32;
        writeln!(&mut output, "-- .bss ({bss_start:#010X}-{end:#010X}) --")?;
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
    for &b in bytes {
        write!(f, "{b:02X}")?;
    }
    writeln!(f)?;

    Ok(())
}
