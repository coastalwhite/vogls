use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use clap::Parser;
use vogls::ExecutionContext;
use vogls_ir::LogicMode;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    filter: Option<String>,

    #[arg(long)]
    skip: Vec<String>,
}

fn main() -> Result<std::process::ExitCode, Box<dyn std::error::Error>> {
    let args = Args::try_parse()?;

    let manifest_path = env!("CARGO_MANIFEST_PATH");
    let manifest_dir = Path::new(manifest_path).parent().unwrap();
    let tests_dir = manifest_dir.join("tests");

    let walker = std::fs::read_dir(&tests_dir)?;
    let mut paths = Vec::new();
    let mut walkers = vec![walker];
    while let Some(mut w) = walkers.pop() {
        let Some(entry) = w.next() else {
            continue;
        };

        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let walker = std::fs::read_dir(entry.path())?;
            walkers.push(w);
            walkers.push(walker);
            continue;
        } else if file_type.is_file() && entry.file_name().as_encoded_bytes().ends_with(b".v") {
            let path = entry.path();
            let path = path.strip_prefix(&tests_dir)?;
            paths.push(path.to_path_buf());
        }
        walkers.push(w);
    }

    let max_size = paths
        .iter()
        .map(|p| p.as_path().as_os_str().len())
        .max()
        .unwrap_or_default()
        + 2;

    // Filter and skip paths accordingly.
    if let Some(f) = args.filter.as_ref() {
        paths.retain(|p| p.to_str().unwrap().contains(f));
    }
    for s in args.skip {
        paths.retain(|p| !p.to_str().unwrap().contains(s.as_str()));
    }
    paths.sort_unstable();

    let mut num_failed = 0;
    let mut num_tests = 0;
    let mut o = std::io::stdout();
    writeln!(&mut o, "Running {} tests...", paths.len())?;
    for path in paths.iter() {
        let path = path.as_path();
        write!(
            &mut o,
            "  {}{:.<2$} ",
            path.display(),
            "",
            max_size - path.as_os_str().len()
        )?;
        std::io::stdout().flush()?;
        let path = tests_dir.join(&path);

        struct TestInfo {
            fail: bool,
            verify_stdout: bool,
            time: u64,
            top_level_module: Option<String>,
            skip: Option<LogicMode>,
        }

        let mut test_information = TestInfo {
            fail: false,
            verify_stdout: false,
            top_level_module: None,
            time: 1000,
            skip: None,
        };
        {
            let s = std::fs::read_to_string(&path)?;
            let mut lines = s.lines();
            loop {
                let Some(line) = lines.next() else {
                    break;
                };
                if !line.starts_with("// vogls:") {
                    break;
                }

                let line = &line["// vogls:".len()..];
                let line = line.trim();

                match line {
                    "fail" => test_information.fail = true,
                    "verify-stdout" => test_information.verify_stdout = true,
                    _ if line.starts_with("tlm=") => {
                        test_information.top_level_module = Some(line[4..].trim().to_string());
                    }
                    _ if line.starts_with("time=") => {
                        test_information.time = line[5..].parse().expect("failed to parse");
                    }
                    _ if line.starts_with("skip=") => match &line[5..] {
                        "two-value-logic" => test_information.skip = Some(LogicMode::TwoValue),
                        "four-value-logic" => test_information.skip = Some(LogicMode::FourValue),
                        _ => panic!("failed to parse"),
                    },
                    _ => {
                        println!();
                        panic!("Invalid vogls test command '{line}'");
                    }
                }
            }
        }

        #[derive(Default, Clone)]
        struct Io(Arc<Mutex<Vec<u8>>>);

        impl io::Write for Io {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().write(buf)
            }

            fn flush(&mut self) -> io::Result<()> {
                self.0.lock().unwrap().flush()
            }
        }

        for (logic_mode, compile) in [
            (LogicMode::TwoValue, false),
            (LogicMode::FourValue, false),
            (LogicMode::TwoValue, true),
        ] {
            if Some(logic_mode) == test_information.skip {
                write!(&mut o, " \x1b[32mS\x1b[0m")?;
                continue;
            }

            let stdout = Io::default();
            let stderr = Io::default();

            let mut ctx = ExecutionContext {
                stdout: Box::new(stdout.clone()) as Box<dyn std::io::Write + Send + Sync>,
                stderr: Box::new(stderr.clone()) as Box<dyn std::io::Write + Send + Sync>,
                defines: Vec::new(),
                emit_hierarchy: false,
                emit_unoptimized_ir: false,
                emit_ir: false,
                emit_vm: false,
                trace: false,
                itrace: false,
                no_run: false,
                time: test_information.time,
                opt_rounds: 0,
                logic_mode,
                vcd: None,
                compile,
            };
            let ctx = std::panic::AssertUnwindSafe(&mut ctx);
            let result = std::panic::catch_unwind(|| {
                let ctx = ctx;
                vogls::run(
                    &[&path],
                    test_information.top_level_module.as_deref(),
                    ctx.0,
                )
            });

            let stdout = stdout.0.lock().unwrap();
            let stdout = std::str::from_utf8(&stdout).unwrap();
            let stderr = stderr.0.lock().unwrap();
            let stderr = std::str::from_utf8(&stderr).unwrap();

            let mut failed = false;
            if result.is_err() {
                failed = true;
            } else {
                failed |= result.as_ref().is_ok_and(|r| r.is_err()) ^ test_information.fail;
                if test_information.verify_stdout {
                    let s = std::fs::read_to_string(&path.with_extension("v.stdout"))?;
                    failed |= stdout != s;
                }
            }

            num_tests += 1;
            num_failed += usize::from(failed);
            if result.is_err() {
                writeln!(&mut o, "\x1b[31mPANIC\x1b[0m")?;
            } else if failed {
                writeln!(&mut o, "\x1b[31mERR\x1b[0m")?;
                if let Err(err) = result {
                    writeln!(&mut o, "ERROR: {err:?}")?;
                };
                writeln!(&mut o, "--- [START STDOUT] ---")?;
                write!(&mut o, "{stdout}")?;
                writeln!(&mut o, "---  [END STDOUT]  ---")?;
                writeln!(&mut o)?;
                writeln!(&mut o, "--- [START STDERR] ---")?;
                write!(&mut o, "{stderr}")?;
                writeln!(&mut o, "---  [END STDERR]  ---")?;
                writeln!(&mut o)?;
            } else {
                write!(&mut o, " \x1b[32mP\x1b[0m")?;
            }
            o.flush()?;
        }
        writeln!(&mut o)?;
    }
    if num_failed == 0 {
        writeln!(&mut o, "All {} tests passed!", paths.len())?;
        Ok(ExitCode::SUCCESS)
    } else {
        writeln!(
            &mut o,
            "\x1b[31mFailed {}/{} tests.\x1b[0m",
            num_failed, num_tests,
        )?;
        Ok(ExitCode::FAILURE)
    }
}
