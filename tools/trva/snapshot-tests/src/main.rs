//! Snapshot tests for the assembler.
//!
//! Each `fixtures/<category>/<name>.s` is assembled and rendered into a snapshot, which is
//! compared against `snapshots/<category>/<name>.snap`. Run with `TRVA_UPDATE_SNAPSHOTS=1` to
//! rewrite the snapshots.
//!
//! A fixture may override where its sections go with a comment, using the same syntax as the
//! `trva` CLI:
//!
//! ```text
//! # trva-text: 0x8000_0000
//! # trva-bss: after:data:align=4
//! ```

use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use trva::isa::{ExtensionSet, Isa, XLen};
use trva::{Assembler, SectionKind, SectionPlacement, SectionPositions};

const RV32IM: Isa = Isa {
    exts: ExtensionSet::INTEGER_MULDIV,
    xlen: XLen::Rv32,
};
const DEFAULT_POSITIONS: SectionPositions =
    SectionPositions::absolute(0x8000_0000, 0x8000_0100, 0x8000_0200, 0x8000_0300);

const OBJDUMP: &str = "riscv32-none-elf-objdump";

fn main() -> ExitCode {
    let update = std::env::var("TRVA_UPDATE_SNAPSHOTS").is_ok();

    match run(update) {
        Ok(total) => {
            println!("{total} snapshots ok");
            ExitCode::SUCCESS
        }
        Err(failures) => {
            for f in &failures {
                eprintln!("\nFAIL: {f}");
            }
            eprintln!("\n{} snapshot(s) failed", failures.len());
            ExitCode::FAILURE
        }
    }
}

/// Check every fixture against its snapshot, or rewrite the snapshots when `update`.
///
/// Returns how many fixtures were checked, or the failures.
fn run(update: bool) -> Result<usize, Vec<String>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.join("fixtures");
    let snapshots_dir = manifest_dir.join("snapshots");

    // The disassembly needs a RISC-V objdump, which is not always around. Without it the
    // disassembly block is left out and also stripped from the recorded snapshot, so everything
    // else is still checked.
    let have_objdump = Command::new(OBJDUMP).arg("--version").output().is_ok();
    if update && !have_objdump {
        return Err(vec![format!(
            "refusing to update snapshots without {OBJDUMP} on PATH, \
             as that would drop the disassembly from every snapshot"
        )]);
    }
    if !have_objdump && !update {
        eprintln!("{OBJDUMP} not found on PATH — comparing snapshots without disassembly");
    }

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

            let positions = match parse_positions(&source) {
                Ok(positions) => positions,
                Err(e) => {
                    failures.push(format!("{name}: {e}"));
                    continue;
                }
            };

            let asm_result = std::panic::catch_unwind(|| {
                Assembler::new(RV32IM, positions)
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
                Ok(Ok(Err(e))) => {
                    failures.push(format!("{name}: layout error: {e}"));
                    continue;
                }
                Ok(Ok(Ok(a))) => a,
            };

            let actual = build_snapshot(&name, &asm, have_objdump);
            let snap_path = snapshots_dir.join(category).join(format!("{stem}.snap"));

            if update {
                if let Some(parent) = snap_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                match fs::write(&snap_path, &actual) {
                    Ok(()) => eprintln!("updated: {name}"),
                    Err(e) => failures.push(format!("{name}: could not write snapshot: {e}")),
                }
                continue;
            }

            match fs::read_to_string(&snap_path) {
                Err(_) => {
                    failures.push(format!(
                        "{name}: snapshot missing — run TRVA_UPDATE_SNAPSHOTS=1 cargo run -p snapshot-tests to create it"
                    ));
                }
                Ok(expected) => {
                    let expected = if have_objdump {
                        expected
                    } else {
                        strip_disassembly(&expected)
                    };
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

    if !update {
        println!();
    }

    if failures.is_empty() {
        Ok(total)
    } else {
        Err(failures)
    }
}

/// Read the `# trva-<section>:` overrides out of a fixture.
fn parse_positions(source: &str) -> Result<SectionPositions, String> {
    let mut positions = DEFAULT_POSITIONS;

    for (lineno, line) in source.lines().enumerate() {
        let Some(rest) = line.trim().strip_prefix("# trva-") else {
            continue;
        };
        let Some((name, value)) = rest.split_once(':') else {
            continue;
        };

        let placement = value
            .trim()
            .parse::<SectionPlacement>()
            .map_err(|e| format!("line {}: `{}`: {e}", lineno + 1, line.trim()))?;

        let section = name
            .trim()
            .parse::<SectionKind>()
            .map_err(|_| format!("line {}: unknown section `{name}`", lineno + 1))?;

        *match section {
            SectionKind::Text => &mut positions.text,
            SectionKind::Data => &mut positions.data,
            SectionKind::ReadOnlyData => &mut positions.rodata,
            SectionKind::Bss => &mut positions.bss,
        } = placement;
    }

    Ok(positions)
}

fn build_snapshot(name: &str, asm: &trva::Assembled, have_objdump: bool) -> String {
    let mut out = String::new();

    // Addresses, not just sizes, so that relative placements are visible here.
    writeln!(out, "=== Section Layout ===").unwrap();
    for (label, start, len) in [
        ("text", asm.text_start, asm.text.len()),
        ("data", asm.data_start, asm.data.len()),
        ("rodata", asm.rodata_start, asm.rodata.len()),
        ("bss", asm.bss_start, asm.bss),
    ] {
        let end = start + len as u32;
        writeln!(out, "{label:<7} {start:#010x}-{end:#010x} ({len} bytes)").unwrap();
    }

    if have_objdump && !asm.text.is_empty() {
        writeln!(out, "\n=== Disassembly (text) ===").unwrap();
        match run_objdump(&asm.text, asm.text_start, name) {
            Some(dis) => write!(out, "{dis}").unwrap(),
            None => writeln!(out, "(objdump failed)").unwrap(),
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

/// Drop the disassembly block from a snapshot, so one recorded with [`OBJDUMP`] available can
/// still be compared without it.
fn strip_disassembly(snapshot: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut lines = snapshot.lines();

    while let Some(line) = lines.next() {
        if line.starts_with("=== Disassembly") {
            // Skip to the next section header, which is kept.
            for line in lines.by_ref() {
                if line.starts_with("=== ") {
                    kept.push(line);
                    break;
                }
            }
            continue;
        }
        kept.push(line);
    }

    format!("{}\n", kept.join("\n"))
}

fn run_objdump(bytes: &[u8], start_addr: u32, name: &str) -> Option<String> {
    let sanitized: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let tmp_path = std::env::temp_dir().join(format!("trva_snap_{sanitized}.bin"));

    fs::write(&tmp_path, bytes).ok()?;
    let _guard = TempFile(tmp_path.clone());

    let output = match Command::new(OBJDUMP)
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
            eprintln!("{OBJDUMP} not found on PATH — skipping disassembly for {name}");
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Run the whole snapshot suite under `cargo test`, so CI covers it.
    #[test]
    fn snapshots_match() {
        if let Err(failures) = run(false) {
            panic!("\n{}", failures.join("\n\n"));
        }
    }

    #[test]
    fn fixture_overrides_are_parsed() {
        let positions =
            parse_positions("# trva-text: 0x1234_0000\n# trva-bss: after:data:align=4\n nop\n")
                .expect("should parse");

        assert_eq!(positions.text, SectionPlacement::Absolute(0x1234_0000));
        assert_eq!(
            positions.bss,
            SectionPlacement::after_aligned(SectionKind::Data, 4),
        );
        // Untouched sections keep their default.
        assert_eq!(positions.data, DEFAULT_POSITIONS.data);
    }

    #[test]
    fn a_bad_fixture_override_is_an_error() {
        let err = parse_positions("# trva-bss: after:stack\n").unwrap_err();
        assert!(err.contains("stack"), "{err}");
    }

    #[test]
    fn stripping_disassembly_leaves_the_rest() {
        let snapshot = "\
=== Section Layout ===
text    0x00000000-0x00000004 (4 bytes)

=== Disassembly (text) ===
Disassembly of section .data:

00000000 <.data>:
   0:\t00000013\tnop

=== Hex Dump: text ===
00000000  13 00 00 00
";
        assert_eq!(
            strip_disassembly(snapshot),
            "\
=== Section Layout ===
text    0x00000000-0x00000004 (4 bytes)

=== Hex Dump: text ===
00000000  13 00 00 00
",
        );
    }
}
