use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use trva::isa::Isa;
use trva::{Assembler, SectionPositions};

const DEFAULT_POSITIONS: SectionPositions = SectionPositions {
    text: 0x8000_0000,
    data: 0x8000_0100,
    rodata: 0x8000_0200,
    bss: 0x8000_0300,
};

fn main() -> ExitCode {
    let update = std::env::var("TRVA_UPDATE_SNAPSHOTS").is_ok();
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.join("fixtures");
    let snapshots_dir = manifest_dir.join("snapshots");

    let mut failures: Vec<String> = Vec::new();
    let mut total = 0usize;

    for category in ["units", "programs"] {
        let dir = fixtures_dir.join(category);
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "s"))
            .map(|e| e.path())
            .collect();
        paths.sort();

        for fixture_path in paths {
            let stem = fixture_path.file_stem().unwrap().to_str().unwrap();
            let name = format!("{category}/{stem}");
            total += 1;

            let source = match fs::read_to_string(&fixture_path) {
                Ok(s) => s,
                Err(e) => {
                    failures.push(format!("{name}: could not read fixture: {e}"));
                    continue;
                }
            };

            let positions = parse_positions(&source);

            let asm_result = std::panic::catch_unwind(|| {
                Assembler::new(Isa::RV_I | Isa::INTEGER_MULDIV, positions)
                    .with_source(&source)
                    .map(|a| a.assemble())
            });

            let asm = match asm_result {
                Err(_) => {
                    failures.push(format!("{name}: assembler panicked"));
                    continue;
                }
                Ok(Err(e)) => {
                    failures.push(format!("{name}: assembly error: {e}"));
                    continue;
                }
                Ok(Ok(a)) => a,
            };

            let actual = build_snapshot(&name, &asm);
            let snap_path = snapshots_dir.join(category).join(format!("{stem}.snap"));

            if update {
                if let Some(parent) = snap_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                match fs::write(&snap_path, &actual) {
                    Ok(()) => eprintln!("updated: {name}"),
                    Err(e) => failures.push(format!("{name}: could not write snapshot: {e}")),
                }
            } else {
                match fs::read_to_string(&snap_path) {
                    Err(_) => {
                        failures.push(format!(
                            "{name}: snapshot missing — run TRVA_UPDATE_SNAPSHOTS=1 cargo run -p snapshot-tests to create it"
                        ));
                    }
                    Ok(expected) => {
                        if let Some(diff) = diff_snapshots(&name, &expected, &actual) {
                            failures.push(diff);
                        } else {
                            print!(".");
                            let _ = std::io::stdout().flush();
                        }
                    }
                }
            }
        }
    }

    if !update {
        println!();
    }

    if failures.is_empty() {
        println!("{total} snapshots ok");
        ExitCode::SUCCESS
    } else {
        for f in &failures {
            eprintln!("\nFAIL: {f}");
        }
        eprintln!("\n{} of {total} snapshots failed", failures.len());
        ExitCode::FAILURE
    }
}

fn parse_positions(source: &str) -> SectionPositions {
    let mut pos = DEFAULT_POSITIONS;
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("# trva-text:") {
            if let Ok(v) = parse_hex(rest.trim()) {
                pos.text = v;
            }
        } else if let Some(rest) = line.strip_prefix("# trva-data:") {
            if let Ok(v) = parse_hex(rest.trim()) {
                pos.data = v;
            }
        } else if let Some(rest) = line.strip_prefix("# trva-rodata:") {
            if let Ok(v) = parse_hex(rest.trim()) {
                pos.rodata = v;
            }
        } else if let Some(rest) = line.strip_prefix("# trva-bss:")
            && let Ok(v) = parse_hex(rest.trim())
        {
            pos.bss = v;
        }
    }
    pos
}

fn parse_hex(s: &str) -> Result<u32, ()> {
    let hex = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u32::from_str_radix(hex, 16).map_err(|_| ())
}

fn build_snapshot(name: &str, asm: &trva::Assembled) -> String {
    let mut out = String::new();

    writeln!(out, "=== Section Sizes ===").unwrap();
    writeln!(out, "text:   {} bytes", asm.text.len()).unwrap();
    writeln!(out, "data:   {} bytes", asm.data.len()).unwrap();
    writeln!(out, "rodata: {} bytes", asm.rodata.len()).unwrap();
    writeln!(out, "bss:    {} bytes", asm.bss.len()).unwrap();

    if !asm.text.is_empty() {
        writeln!(out, "\n=== Disassembly (text) ===").unwrap();
        match run_objdump(&asm.text, asm.text_start, name) {
            Some(dis) => write!(out, "{dis}").unwrap(),
            None => writeln!(out, "(objdump not available)").unwrap(),
        }
    }

    for (label, bytes, start) in [
        ("text", &asm.text, asm.text_start),
        ("data", &asm.data, asm.data_start),
        ("rodata", &asm.rodata, asm.rodata_start),
    ] {
        writeln!(out, "\n=== Hex Dump: {label} ===").unwrap();
        if bytes.is_empty() {
            writeln!(out, "(empty)").unwrap();
        } else {
            write!(out, "{}", format_hex_dump(bytes, start)).unwrap();
        }
    }

    // Normalise: strip trailing whitespace per line, single trailing newline.
    let normalised: String = out
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    format!("{normalised}\n")
}

fn run_objdump(bytes: &[u8], start_addr: u32, name: &str) -> Option<String> {
    let sanitized: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let tmp_path = std::env::temp_dir().join(format!("trva_snap_{sanitized}.bin"));

    fs::write(&tmp_path, bytes).ok()?;
    let _guard = TempFile(tmp_path.clone());

    let output = match Command::new("riscv32-none-elf-objdump")
        .args([
            "-b",
            "binary",
            "-m",
            "riscv:rv32",
            &format!("--adjust-vma=0x{start_addr:08x}"),
            "-D",
            tmp_path.to_str().unwrap(),
        ])
        .output()
    {
        Ok(o) => o,
        Err(_) => {
            eprintln!(
                "riscv32-none-elf-objdump not found on PATH — skipping disassembly for {name}"
            );
            return None;
        }
    };

    let raw = String::from_utf8_lossy(&output.stdout);
    // Strip the "path: file format binary" header line and blank lines before
    // the first "Disassembly of section" line.
    let lines: Vec<&str> = raw.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with("Disassembly"))
        .unwrap_or(0);
    let stripped = lines[start..].join("\n");
    Some(format!("{stripped}\n"))
}

fn format_hex_dump(bytes: &[u8], start_addr: u32) -> String {
    let mut out = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let addr = start_addr + (i * 16) as u32;
        write!(out, "{addr:08x} ").unwrap();
        for b in chunk {
            write!(out, " {b:02x}").unwrap();
        }
        out.push('\n');
    }
    out
}

fn diff_snapshots(name: &str, expected: &str, actual: &str) -> Option<String> {
    if expected == actual {
        return None;
    }
    let exp_lines: Vec<&str> = expected.lines().collect();
    let act_lines: Vec<&str> = actual.lines().collect();
    let mut msg = format!("snapshot mismatch for {name}\n");
    let max = exp_lines.len().max(act_lines.len());
    for i in 0..max {
        let e = exp_lines.get(i).copied().unwrap_or("<missing>");
        let a = act_lines.get(i).copied().unwrap_or("<missing>");
        if e != a {
            writeln!(msg, "  line {}: expected {:?}", i + 1, e).unwrap();
            writeln!(msg, "  line {}:   actual {:?}", i + 1, a).unwrap();
        }
    }
    Some(msg)
}

struct TempFile(PathBuf);
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
