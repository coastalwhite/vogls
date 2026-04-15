use std::fs::read_to_string;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use clap::Parser;
use vogls::design::Design;
use vogls::utils::TimerStack;
use vogls::{ExecutionContext, SimulationIo};
use vogls_ir::LogicMode;
use vogls_ir::optimize::OptFlags;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    filter: Option<String>,

    #[arg(long)]
    skip: Vec<String>,

    #[arg(short = 'T')]
    tv: bool,
    #[arg(short = 'F')]
    fv: bool,
    #[arg(short = 'I')]
    interpretted: bool,
    #[arg(short = 'C')]
    compiled: bool,
    #[arg(long)]
    opt_rounds: Option<u8>,
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
        } else if file_type.is_file()
            && (entry.file_name().as_encoded_bytes().ends_with(b".v")
                || entry.file_name().as_encoded_bytes().ends_with(b".vir"))
        {
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

    struct Fail {
        name: String,
        mode: LogicMode,
        opt_rounds: u8,
        compile: bool,
        error: Option<Box<dyn std::error::Error>>,
        mismatch: Option<(String, String)>,
        stdout: String,
        stderr: String,
    }

    enum VerifyOutput {
        No,
        SortLines,
        Yes,
    }

    let modes: &[LogicMode] = match (args.tv, args.fv) {
        (true, false) => &[LogicMode::TwoValue],
        (false, true) => &[LogicMode::FourValue],
        _ => &[LogicMode::TwoValue, LogicMode::FourValue],
    };
    let compiled: &[bool] = match (args.interpretted, args.compiled) {
        (true, false) => &[false],
        (false, true) => &[true],
        _ => &[false, true],
    };

    let mut fails = Vec::<Fail>::new();
    let mut num_tests = 0;
    let mut o = std::io::stdout();
    writeln!(&mut o, "Running {} tests...", paths.len())?;
    for path in paths.iter() {
        let offset_path = path.as_path();
        write!(
            &mut o,
            "  {}{:.<2$} ",
            offset_path.display(),
            "",
            max_size - offset_path.as_os_str().len()
        )?;
        std::io::stdout().flush()?;

        let path = tests_dir.join(&offset_path);

        struct TestInfo {
            fail: bool,
            verify_stdout: VerifyOutput,
            verify_ir: bool,
            time: u64,
            top_level_module: Option<String>,
            skip: Option<LogicMode>,
        }

        let mut test_information = TestInfo {
            fail: false,
            verify_stdout: VerifyOutput::No,
            verify_ir: false,
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
                    "verify-stdout" => test_information.verify_stdout = VerifyOutput::Yes,
                    "verify-stdout[sort-lines]" => {
                        test_information.verify_stdout = VerifyOutput::SortLines
                    }
                    "verify-ir" => test_information.verify_ir = true,
                    _ if line.starts_with("tlm=") => {
                        test_information.top_level_module = Some(line[4..].trim().to_string());
                    }
                    _ if line.starts_with("time=") => {
                        if &line[5..] == "none" {
                            test_information.time = u64::MAX;
                        } else {
                            test_information.time = line[5..].parse().expect("failed to parse");
                        }
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

        let opt_rounds_configurations: &[u8] = match args.opt_rounds {
            None => &[0, 2],
            Some(o) => &[o],
        };

        for &opt_rounds in opt_rounds_configurations {
            for &logic_mode in modes {
                for &compile in compiled {
                    if Some(logic_mode) == test_information.skip {
                        write!(&mut o, " \x1b[32mS\x1b[0m")?;
                        continue;
                    }

                    num_tests += 1;

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
                        itrace: false,
                        stats: false,
                        debug_symbols: false,
                        no_run: false,
                        time: test_information.time,
                        opt: OptFlags {
                            opt_rounds,
                            constant_propagation: true,
                            deadcode_elimination: true,
                            common_subexpr_elim: true,
                            peephole: true,
                        },
                        logic_mode,
                        vcd: None,
                        compile,
                        output_source: None,
                        timings: false,
                        print_optimized_fuse_signals: false,
                        print_round_fuse_signals: false,
                        print_unoptimized_fuse_signals: false,
                    };

                    if test_information.verify_ir {
                        let design = std::panic::catch_unwind(|| {
                            let stdout = Io::default();
                            let stderr = Io::default();

                            let mut ctx = ExecutionContext {
                                stdout: Box::new(stdout.clone())
                                    as Box<dyn std::io::Write + Send + Sync>,
                                stderr: Box::new(stderr.clone())
                                    as Box<dyn std::io::Write + Send + Sync>,
                                defines: vec!["__VOGLS_VERIFY_IR".to_string()],
                                emit_hierarchy: false,
                                emit_unoptimized_ir: false,
                                emit_ir: false,
                                emit_vm: false,
                                itrace: false,
                                stats: false,
                                debug_symbols: false,
                                no_run: false,
                                time: test_information.time,
                                opt: OptFlags {
                                    opt_rounds,
                                    constant_propagation: true,
                                    deadcode_elimination: true,
                                    common_subexpr_elim: true,
                                    peephole: true,
                                },
                                logic_mode,
                                vcd: None,
                                compile,
                                output_source: None,
                                timings: false,
                                print_optimized_fuse_signals: false,
                                print_round_fuse_signals: false,
                                print_unoptimized_fuse_signals: false,
                            };
                            vogls::design::Design::new(
                                &[&path],
                                &mut TimerStack::new(false),
                                test_information.top_level_module.as_deref(),
                                &mut ctx,
                                Vec::new(),
                            )
                        });

                        let mut panic = false;
                        let mut failed = false;
                        let mut error = None;
                        let mut mismatch = None;

                        if let Ok(design) = design {
                            match design {
                                Ok(design) => {
                                    let design_ir = design.emit_ir();
                                    let asserted =
                                        std::fs::read_to_string(&path.with_extension("v.ir"))?;
                                    failed = design_ir != asserted;
                                    mismatch = Some((asserted, design_ir));
                                }
                                Err(err) => {
                                    failed = true;
                                    error = Some(err);
                                }
                            }
                        } else {
                            panic = true;
                        }

                        if panic {
                            write!(&mut o, " \x1b[31mP\x1b[0m")?;
                        } else if failed {
                            let stdout = stdout.0.lock().unwrap();
                            let stdout = std::str::from_utf8(&stdout).unwrap();
                            let stderr = stderr.0.lock().unwrap();
                            let stderr = std::str::from_utf8(&stderr).unwrap();

                            fails.push(Fail {
                                name: offset_path.display().to_string(),
                                mode: logic_mode,
                                opt_rounds,
                                compile,
                                error,
                                mismatch,
                                stdout: stdout.to_string(),
                                stderr: stderr.to_string(),
                            });
                            write!(&mut o, " \x1b[31mI\x1b[0m")?;
                        }
                    }

                    let ctx = std::panic::AssertUnwindSafe(&mut ctx);
                    let result: Result<
                        Result<(), Box<dyn std::error::Error>>,
                        Box<dyn std::any::Any + Send + 'static>,
                    > = std::panic::catch_unwind(|| {
                        if path
                            .extension()
                            .is_some_and(|ext| ext.as_encoded_bytes() == b"vir")
                        {
                            let s = std::fs::read_to_string(&path)?;
                            let optimized = read_to_string(path.with_extension("vir.opt")).ok();
                            let mut design =
                                Design::new_vir(&s, &mut TimerStack::new(false), ctx.0)?;

                            if opt_rounds > 0
                                && let Some(optimized) = optimized
                            {
                                use std::fmt::Write;
                                let mut out = String::new();
                                for signal in design.gl.signals.values() {
                                    writeln!(&mut out, "{}", signal.display()).unwrap();
                                }
                                writeln!(&mut out).unwrap();
                                for process in design.gl.processes.values() {
                                    writeln!(&mut out, "{}", process.display(&design.gl)).unwrap();
                                }
                                let optimized = optimized.trim();
                                let out = out.trim();
                                if optimized != out {
                                    return Err("optimization mismatch".into());
                                }
                            }

                            let stdout =
                                std::mem::replace(&mut ctx.0.stdout, Box::new(Vec::new()) as _);
                            let stderr =
                                std::mem::replace(&mut ctx.0.stderr, Box::new(Vec::new()) as _);
                            let mut io = SimulationIo::new(stdout, stderr);

                            design.run(&mut io, test_information.time)?;

                            ctx.0.stdout = io.stdout;
                            ctx.0.stderr = io.stderr;

                            Ok(())
                        } else {
                            let ctx = ctx;
                            Ok(vogls::run(
                                &[&path],
                                &mut TimerStack::new(false),
                                test_information.top_level_module.as_deref(),
                                ctx.0,
                            )?)
                        }
                    });

                    let stdout = stdout.0.lock().unwrap();
                    let stdout = std::str::from_utf8(&stdout).unwrap();
                    let stderr = stderr.0.lock().unwrap();
                    let stderr = std::str::from_utf8(&stderr).unwrap();

                    let mut failed = false;
                    let mut panic = false;
                    let mut mismatch = None;
                    if result.is_err() {
                        failed = true;
                        panic = true;
                    } else {
                        failed |= result.as_ref().is_ok_and(|r| r.is_err()) ^ test_information.fail;
                        if matches!(
                            test_information.verify_stdout,
                            VerifyOutput::Yes | VerifyOutput::SortLines
                        ) {
                            let mut stdout_path = path.clone();
                            stdout_path.add_extension("stdout");
                            let s = std::fs::read_to_string(&stdout_path)?;

                            if matches!(test_information.verify_stdout, VerifyOutput::SortLines) {
                                let mut lines = stdout.lines().collect::<Vec<&str>>();
                                lines.sort_unstable();
                                failed |= lines != s.lines().collect::<Vec<_>>();
                            } else {
                                failed |= s != stdout;
                            }

                            if failed {
                                mismatch = Some((s, stdout.to_string()));
                            }
                        }
                    }

                    if failed {
                        let error = match result {
                            Ok(Ok(_)) => None,
                            Ok(Err(err)) => Some(err),
                            Err(_) => None,
                        };
                        fails.push(Fail {
                            name: offset_path.display().to_string(),
                            mode: logic_mode,
                            opt_rounds,
                            compile,
                            error,
                            mismatch,
                            stdout: stdout.to_string(),
                            stderr: stderr.to_string(),
                        });
                    }
                    if panic {
                        write!(&mut o, " \x1b[31mP\x1b[0m")?;
                    } else if failed {
                        write!(&mut o, " \x1b[31mE\x1b[0m")?;
                    } else {
                        write!(&mut o, " \x1b[32mP\x1b[0m")?;
                    }
                    o.flush()?;
                }
            }
        }
        writeln!(&mut o)?;
    }

    writeln!(&mut o)?;
    if fails.is_empty() {
        writeln!(&mut o, "All {} tests passed!", paths.len())?;
        Ok(ExitCode::SUCCESS)
    } else {
        for fail in fails.iter() {
            let Fail {
                name,
                mode,
                opt_rounds,
                compile,
                mismatch,
                error,
                stdout,
                stderr,
            } = fail;
            let mode_str = match mode {
                LogicMode::TwoValue => "tvl",
                LogicMode::FourValue => "fvl",
            };

            write!(
                &mut o,
                "+ {name}[{mode_str}-compile={compile}-O{opt_rounds}]"
            )?;
            if let Some(err) = error {
                write!(&mut o, ": ERROR={err:?}")?;
            };
            writeln!(&mut o)?;

            if !stdout.is_empty() {
                writeln!(&mut o, "  --- [START STDOUT] ---")?;
                let stdout = if stdout.ends_with('\n') {
                    &stdout[..stdout.len() - 1]
                } else {
                    stdout
                };
                writeln!(&mut o, "  {}", stdout.replace("\n", "\n  "))?;
                writeln!(&mut o, "  ---  [END STDOUT]  ---")?;
            }
            if !stderr.is_empty() {
                writeln!(&mut o, "  --- [START STDERR] ---")?;
                let stderr = if stderr.ends_with('\n') {
                    &stderr[..stderr.len() - 1]
                } else {
                    stderr
                };
                writeln!(&mut o, "  {}", stderr.replace("\n", "\n  "))?;
                writeln!(&mut o, "  ---  [END STDERR]  ---")?;
            }
            if let Some((snapshot, given)) = mismatch {
                writeln!(&mut o, "  --- [START SNAPSHOT] ---")?;
                let snapshot = if snapshot.ends_with('\n') {
                    &snapshot[..snapshot.len() - 1]
                } else {
                    snapshot
                };
                writeln!(&mut o, "  {}", snapshot.replace("\n", "\n  "))?;
                writeln!(&mut o, "  ---  [END SNAPSHOT]  ---")?;

                writeln!(&mut o, "  ---   [START GIVEN]  ---")?;
                let given = if given.ends_with('\n') {
                    &given[..given.len() - 1]
                } else {
                    given
                };
                writeln!(&mut o, "  {}", given.replace("\n", "\n  "))?;
                writeln!(&mut o, "  ---   [END GIVEN]   ---")?;
            }
        }
        writeln!(
            &mut o,
            "\x1b[31mFailed {}/{} tests.\x1b[0m",
            fails.len(),
            num_tests,
        )?;
        Ok(ExitCode::FAILURE)
    }
}
