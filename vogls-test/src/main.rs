use std::fs::read_to_string;
use std::io::{self, Write};
use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use clap::Parser;
use vogls::design::{Arena, Macro};
use vogls::{DesignBuilder, SimulationIo, VirDesignBuilder};
use vogls_ir::LogicMode;
use vogls_ir::optimize::OptFlags;

static ANSI_RED: &str = "\x1b[31m";
static ANSI_GREEN: &str = "\x1b[32m";
static ANSI_END: &str = "\x1b[0m";

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

    #[arg(short = 'n', long, default_value_t = 0)]
    num_threads: usize,
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

#[derive(Clone)]
struct TestInfo {
    fail: bool,
    verify_stdout: VerifyOutput,
    verify_ir: bool,
    annotate_sdf: bool,
    timeout: u64,
    top_level_module: Option<String>,
    skip: Option<LogicMode>,
}

impl TestInfo {
    pub fn parse(content: &str) -> Self {
        let mut info = TestInfo {
            fail: false,
            verify_stdout: VerifyOutput::No,
            verify_ir: false,
            annotate_sdf: false,
            top_level_module: None,
            timeout: u64::MAX,
            skip: None,
        };

        for line in content.lines() {
            if !line.starts_with("// vogls:") {
                break;
            }

            let line = &line["// vogls:".len()..];
            let line = line.trim();

            match line {
                "fail" => info.fail = true,
                "verify-stdout" => info.verify_stdout = VerifyOutput::Yes,
                "verify-stdout[sort-lines]" => info.verify_stdout = VerifyOutput::SortLines,
                "verify-ir" => info.verify_ir = true,
                "annotate-sdf" => info.annotate_sdf = true,
                _ if line.starts_with("tlm=") => {
                    info.top_level_module = Some(line[4..].trim().to_string());
                }
                _ if line.starts_with("timeout=") => {
                    info.timeout = line[8..].parse().expect("failed to parse");
                }
                _ if line.starts_with("skip=") => match &line[5..] {
                    "two-value-logic" => info.skip = Some(LogicMode::TwoValue),
                    "four-value-logic" => info.skip = Some(LogicMode::FourValue),
                    _ => panic!("failed to parse"),
                },
                _ => {
                    println!();
                    panic!("Invalid vogls test command '{line}'");
                }
            }
        }

        info
    }
}

enum FailureInfo {
    Panic,
    Error { stdout: String, stderr: String },
    Mismatch { expected: String, gotten: String },
    VirMismatch { expected: String, gotten: String },
    VirOptMismatch { expected: String, gotten: String },
    CompileFailure(Box<dyn std::error::Error + Send + Sync + 'static>),
    IoFailure(io::Error),
}

impl FailureInfo {
    pub fn into_char(&self) -> char {
        match self {
            Self::Panic => '!',
            Self::Error { .. } => 'E',
            Self::Mismatch { .. } => 'M',
            Self::VirMismatch { .. } => 'M',
            Self::VirOptMismatch { .. } => 'O',
            Self::CompileFailure(..) => 'C',
            Self::IoFailure(..) => 'I',
        }
    }
}
impl From<io::Error> for FailureInfo {
    fn from(value: io::Error) -> Self {
        Self::IoFailure(value)
    }
}

struct Fail {
    name: String,
    mode: LogicMode,
    opt_rounds: u8,
    compile: bool,
    info: FailureInfo,
}

#[derive(Clone)]
enum VerifyOutput {
    No,
    SortLines,
    Yes,
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

    let mut num_tests = 0;
    let opt_rounds_configurations: &[u8] = match args.opt_rounds {
        None => &[0, 2],
        Some(o) => &[o],
    };
    let mut o = std::io::stdout();

    writeln!(&mut o, "Running {} tests...", paths.len())?;
    let fails = if args.num_threads == 1 {
        let mut fails = Vec::<Fail>::new();
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
            let s = std::fs::read_to_string(&path)?;
            let test_information = TestInfo::parse(&s);

            for &opt_rounds in opt_rounds_configurations {
                for &logic_mode in modes {
                    for &compile in compiled {
                        let result =
                            run_test(&path, &test_information, logic_mode, compile, opt_rounds);
                        num_tests += usize::from(!matches!(result, Ok(PassKind::Skip)));

                        match result {
                            Ok(PassKind::Skip) => write!(&mut o, " {ANSI_GREEN}S{ANSI_END}")?,
                            Ok(PassKind::Succeed) => write!(&mut o, " {ANSI_GREEN}P{ANSI_END}")?,
                            Err(info) => {
                                write!(&mut o, " {ANSI_RED}{}{ANSI_END}", info.into_char())?;
                                fails.push(Fail {
                                    name: offset_path.display().to_string(),
                                    mode: logic_mode,
                                    opt_rounds,
                                    compile,
                                    info,
                                });
                            }
                        }
                        o.flush()?;
                    }
                }
            }
            writeln!(&mut o)?;
        }
        fails
    } else {
        use rayon::prelude::*;
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(args.num_threads)
            .build()?;

        let mut configurations = Vec::new();

        for path in paths.iter() {
            let offset_path = path.as_path();
            let path = tests_dir.join(&offset_path);
            let s = std::fs::read_to_string(&path)?;
            let test_information = TestInfo::parse(&s);

            for &opt_rounds in opt_rounds_configurations {
                for &logic_mode in modes {
                    for &compile in compiled {
                        configurations.push((
                            offset_path.to_path_buf(),
                            path.clone(),
                            test_information.clone(),
                            opt_rounds,
                            logic_mode,
                            compile,
                        ));
                    }
                }
            }
        }

        num_tests = configurations.len();
        pool.install(|| {
            configurations
                .into_par_iter()
                .filter_map(
                    |(offset_path, path, test_information, opt_rounds, logic_mode, compile)| {
                        match run_test(&path, &test_information, logic_mode, compile, opt_rounds) {
                            Ok(PassKind::Skip) => {
                                io::stdout().write_all(&[b'S']).unwrap();
                                io::stdout().flush().unwrap();
                                None
                            }
                            Ok(PassKind::Succeed) => {
                                io::stdout().write_all(&[b'.']).unwrap();
                                io::stdout().flush().unwrap();
                                None
                            }
                            Err(info) => {
                                let s = format!("{ANSI_RED}{}{ANSI_END}", info.into_char());
                                io::stdout().write_all(s.as_bytes()).unwrap();
                                io::stdout().flush().unwrap();
                                Some(Fail {
                                    name: offset_path.display().to_string(),
                                    mode: logic_mode,
                                    opt_rounds,
                                    compile,
                                    info,
                                })
                            }
                        }
                    },
                )
                .collect()
        })
    };

    writeln!(&mut o)?;

    report_fails(&mut o, &fails, num_tests)?;

    if fails.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn display_section(o: &mut io::Stdout, section: &str, content: &str) -> io::Result<()> {
    if !content.is_empty() {
        writeln!(o, "  --- [START {section}] ---")?;
        let stdout = if content.ends_with('\n') {
            &content[..content.len() - 1]
        } else {
            content
        };
        writeln!(o, "  {}", stdout.replace("\n", "\n  "))?;
        writeln!(o, "  ---  [END {section}]  ---")?;
    }
    Ok(())
}

fn report_fails(o: &mut io::Stdout, fails: &[Fail], num_tests: usize) -> io::Result<()> {
    if fails.is_empty() {
        writeln!(o, "All {} tests passed!", num_tests)?;
    } else {
        for fail in fails.iter() {
            let Fail {
                name,
                mode,
                opt_rounds,
                compile,
                info,
            } = fail;
            let mode_str = match mode {
                LogicMode::TwoValue => "tvl",
                LogicMode::FourValue => "fvl",
            };

            write!(o, "+ {name}[{mode_str}-compile={compile}-O{opt_rounds}]")?;

            match info {
                FailureInfo::Panic => writeln!(o, ": Panic")?,
                FailureInfo::Error { stdout, stderr } => {
                    writeln!(o, ": Error")?;
                    writeln!(o)?;
                    display_section(o, "STDOUT", stdout)?;
                    display_section(o, "STDERR", stderr)?;
                }
                FailureInfo::Mismatch { expected, gotten } => {
                    writeln!(o, ": Mismatch")?;
                    writeln!(o)?;
                    display_section(o, "EXPECTED", expected)?;
                    display_section(o, "GOTTEN", gotten)?;
                }
                FailureInfo::VirMismatch { expected, gotten } => {
                    writeln!(o, ": VIR mismatch")?;
                    writeln!(o)?;
                    display_section(o, "EXPECTED", expected)?;
                    display_section(o, "GOTTEN", gotten)?;
                }
                FailureInfo::VirOptMismatch { expected, gotten } => {
                    writeln!(o, ": VIR Optimization mismatch")?;
                    writeln!(o)?;
                    display_section(o, "EXPECTED", expected)?;
                    display_section(o, "GOTTEN", gotten)?;
                }
                FailureInfo::CompileFailure(error) => {
                    writeln!(o, ": Compilation failure")?;
                    writeln!(o, "  {error}")?;
                }
                FailureInfo::IoFailure(error) => {
                    writeln!(o, ": Io failure")?;
                    writeln!(o, "  {error}")?;
                }
            }
            writeln!(o)?;
        }
        writeln!(
            o,
            "{ANSI_RED}Failed {}/{} tests.{ANSI_END}",
            fails.len(),
            num_tests,
        )?;
    }
    Ok(())
}

pub enum PassKind {
    Succeed,
    Skip,
}

fn run_test(
    path: &Path,
    test_information: &TestInfo,
    logic_mode: LogicMode,
    compile: bool,
    opt_rounds: u8,
) -> Result<PassKind, FailureInfo> {
    if Some(logic_mode) == test_information.skip {
        return Ok(PassKind::Skip);
    }

    let optflags = OptFlags {
        opt_rounds,
        constant_propagation: true,
        deadcode_elimination: true,
        common_subexpr_elim: true,
        peephole: true,
    };

    let sdf = test_information
        .annotate_sdf
        .then(|| path.with_extension("sdf"));

    let stdout = Io::default();
    let stderr = Io::default();

    if test_information.verify_ir {
        let design = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let mut arena = Arena::new();
            let mut builder = DesignBuilder::new();
            match logic_mode {
                LogicMode::TwoValue => {
                    builder.define_macro("__VOGLS__TWO_VALUE_LOGIC", Macro::default());
                }
                LogicMode::FourValue => {}
            }
            builder
                .define_macro("__VOGLS_VERIFY_IR", Macro::default())
                .add_source(&path)
                .map_err(|_| FailureInfo::CompileFailure("failed to tokenize".into()))?;

            let parsed = builder
                .parse(&mut arena)
                .map_err(|_| FailureInfo::CompileFailure("failed to parse".into()))?;
            let mut elab =
                match parsed.elaborate(logic_mode, test_information.top_level_module.as_deref()) {
                    Ok(v) => v,
                    Err(_) => {
                        return Err(FailureInfo::CompileFailure("failed to elaborate".into()));
                    }
                };
            if let Some(sdf) = sdf.as_deref() {
                if let Err(_) = elab.annotate_sdf(sdf) {
                    return Err(FailureInfo::CompileFailure("failed to annotate sdf".into()));
                }
            }
            if let Err(_) = elab.annotate_specify() {
                return Err(FailureInfo::CompileFailure(
                    "failed to annotate specify".into(),
                ));
            }
            let mut lowered = match elab.lower(vec![]) {
                Ok(l) => l,
                Err(_) => {
                    return Err(FailureInfo::CompileFailure("failed to lower".into()));
                }
            };
            lowered.optimize(optflags);
            Result::<_, FailureInfo>::Ok(lowered.emit_ir().to_string())
        }));

        if let Ok(design) = design {
            match design {
                Ok(design) => {
                    let asserted = std::fs::read_to_string(&path.with_extension("v.ir"))?;
                    if design.trim() != asserted.trim() {
                        return Err(FailureInfo::VirMismatch {
                            expected: asserted,
                            gotten: design,
                        });
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        } else {
            return Err(FailureInfo::Panic);
        }
    }

    let result: Result<Result<(), FailureInfo>, Box<dyn std::any::Any + Send + 'static>> =
        std::panic::catch_unwind(|| {
            let mut arena = Arena::new();
            let design = if path
                .extension()
                .is_some_and(|ext| ext.as_encoded_bytes() == b"vir")
            {
                let s = std::fs::read_to_string(&path)?;
                let optimized = read_to_string(path.with_extension("vir.opt")).ok();
                let mut design = VirDesignBuilder::new(&s);
                design.with_logic_mode(logic_mode);
                let mut design = design
                    .parse()
                    .map_err(|_| FailureInfo::CompileFailure("failed to parse VIR".into()))?;
                design.optimize(optflags);

                if opt_rounds > 0
                    && let Some(optimized) = optimized
                {
                    let out = design.emit_ir().to_string();
                    let optimized = optimized.trim();
                    let out = out.trim();
                    if optimized != out {
                        return Err(FailureInfo::VirOptMismatch {
                            expected: optimized.to_string(),
                            gotten: out.to_string(),
                        });
                    }
                }

                Result::<_, FailureInfo>::Ok(design)
            } else {
                let mut builder = DesignBuilder::new();
                match logic_mode {
                    LogicMode::TwoValue => {
                        builder.define_macro("__VOGLS__TWO_VALUE_LOGIC", Macro::default());
                    }
                    LogicMode::FourValue => {}
                }
                builder
                    .add_source(&path)
                    .map_err(|_| FailureInfo::CompileFailure("failed to tokenize".into()))?;

                let parsed = builder
                    .parse(&mut arena)
                    .map_err(|_| FailureInfo::CompileFailure("failed to parse".into()))?;
                let mut elab = match parsed
                    .elaborate(logic_mode, test_information.top_level_module.as_deref())
                {
                    Ok(v) => v,
                    Err(_) => {
                        return Err(FailureInfo::CompileFailure("failed to elaborate".into()));
                    }
                };
                if let Some(sdf) = sdf.as_deref() {
                    if let Err(_) = elab.annotate_sdf(sdf) {
                        return Err(FailureInfo::CompileFailure("failed to annotate SDF".into()));
                    }
                }
                if let Err(_) = elab.annotate_specify() {
                    return Err(FailureInfo::CompileFailure(
                        "failed to annotate specify".into(),
                    ));
                }
                let mut lowered = match elab.lower(vec![]) {
                    Ok(l) => l,
                    Err(_) => return Err(FailureInfo::CompileFailure("failed to lower".into())),
                };
                lowered.optimize(optflags);
                Result::<_, FailureInfo>::Ok(lowered)
            }?;

            let mut design = if compile {
                design.compile()
            } else {
                design.to_bytecode()
            }
            .map_err(|_| {
                FailureInfo::CompileFailure("failed to convert to execution format".into())
            })?;
            design
                .run(
                    &mut SimulationIo {
                        stdout: Box::new(stdout.clone()) as _,
                        stderr: Box::new(stderr.clone()) as _,
                    },
                    test_information.timeout,
                )
                .map_err(|_| {
                    let stdout = stdout.0.lock().unwrap();
                    let stdout = std::str::from_utf8(&stdout).unwrap();
                    let stderr = stderr.0.lock().unwrap();
                    let stderr = std::str::from_utf8(&stderr).unwrap();
                    FailureInfo::Error {
                        stdout: stdout.to_string(),
                        stderr: stderr.to_string(),
                    }
                })
        });

    let Ok(result) = result else {
        return Err(FailureInfo::Panic);
    };
    match result {
        Err(FailureInfo::CompileFailure(_)) if test_information.fail => {
            return Ok(PassKind::Succeed);
        }
        Err(err) => return Err(err),
        Ok(_) => {}
    }

    if matches!(
        test_information.verify_stdout,
        VerifyOutput::Yes | VerifyOutput::SortLines
    ) {
        let stdout = stdout.0.lock().unwrap();
        let stdout = std::str::from_utf8(&stdout).unwrap();

        let mut stdout_path = path.to_path_buf();
        stdout_path.add_extension("stdout");
        let s = std::fs::read_to_string(&stdout_path)?;

        let failed = if matches!(test_information.verify_stdout, VerifyOutput::SortLines) {
            let mut lines = stdout.lines().collect::<Vec<&str>>();
            lines.sort_unstable();
            lines != s.lines().collect::<Vec<_>>()
        } else {
            s != stdout
        };

        if failed {
            return Err(FailureInfo::Mismatch {
                expected: s,
                gotten: stdout.to_string(),
            });
        }
    }

    Ok(PassKind::Succeed)
}
