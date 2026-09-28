use std::cell::RefCell;
use std::env::temp_dir;
use std::error::Error;
use std::fs::read_to_string;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fmt, io, io::Write};

use vogls::design::{Arena, Macro};
use vogls::{DesignBuilder, LogicMode, Optimizations, StdWorldCaptured, VirDesignBuilder};
use vogls_ir::time::{TimeResolution, TimeSize};

use self::info::{Backend, ExpectedFail, SelectLogicMode, TestInfo, TestPhase};
use crate::{ANSI_END, ANSI_GREEN, ANSI_RED};

mod info;

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct UnitArgs {
    #[arg(short, long)]
    filter: Option<String>,

    #[arg(long)]
    skip: Vec<String>,

    #[arg(short = 'T')]
    tv: bool,
    #[arg(short = 'F')]
    fv: bool,
    #[arg(short = 'B', long)]
    bytecode: bool,
    #[arg(short = 'C', long)]
    cranelift: bool,
    #[arg(long)]
    opt_rounds: Option<u8>,

    #[arg(short = 'n', long, default_value_t = 0)]
    num_threads: usize,
}
impl UnitArgs {
    pub fn run(&self) -> Result<std::process::ExitCode, Box<dyn Error>> {
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
        if let Some(f) = self.filter.as_ref() {
            paths.retain(|p| p.to_str().unwrap().contains(f));
        }
        for s in &self.skip {
            paths.retain(|p| !p.to_str().unwrap().contains(s.as_str()));
        }
        paths.sort_unstable();

        let modes: &[LogicMode] = match (self.tv, self.fv) {
            (true, false) => &[LogicMode::TwoValue],
            (false, true) => &[LogicMode::FourValue],
            _ => &[LogicMode::TwoValue, LogicMode::FourValue],
        };
        let mut backends = Vec::new();
        if self.bytecode {
            backends.push(Backend::Bytecode);
        }
        if self.cranelift {
            backends.push(Backend::Cranelift);
        }
        if !(self.bytecode | self.cranelift) {
            backends.extend([Backend::Bytecode, Backend::Cranelift]);
        }

        let mut num_tests = 0;
        let opt_rounds_configurations: &[u8] = match self.opt_rounds {
            None => &[0, 2],
            Some(o) => &[o],
        };
        let mut o = std::io::stdout();

        let mut configurations = Vec::<TestCase>::new();

        for path in paths.iter() {
            let offset_path = path.as_path();
            let path = tests_dir.join(offset_path);
            let s = std::fs::read_to_string(&path)?;
            let test_information = TestInfo::parse(&path, &s)?;

            for &opt_rounds in opt_rounds_configurations {
                for &logic_mode in test_information.mode.selection(modes) {
                    for &backend in test_information.backend.selection(&backends) {
                        configurations.push(TestCase {
                            offset_path: offset_path.to_path_buf(),
                            path: path.clone(),
                            information: test_information.clone(),
                            opt: Optimizations {
                                rounds: opt_rounds,
                                flags: test_information.opt_flags,
                            },
                            logic_mode,
                            backend,
                        });
                    }
                }
            }
        }
        writeln!(&mut o, "Running {} tests...", configurations.len())?;

        let hook = panic::take_hook();
        panic::set_hook(Box::new(|info| {
            PANIC_INFO.set(Some(PanicInfo {
                backtrace: Arc::new(std::backtrace::Backtrace::force_capture()),
                message: info.payload_as_str().unwrap_or("<no info>").to_string(),
            }));
        }));
        let fails = if self.num_threads == 1 {
            let mut fails = Vec::<Fail>::new();
            let mut prev_file = None;
            for (i, t) in configurations.iter().enumerate() {
                if prev_file != Some(&t.path) {
                    prev_file = Some(&t.path);
                    if i != 0 {
                        writeln!(&mut o)?;
                    }
                    write!(
                        &mut o,
                        "  {}{:.<2$} ",
                        t.offset_path.display(),
                        "",
                        max_size - t.offset_path.as_os_str().len()
                    )?;
                    std::io::stdout().flush()?;
                }

                let result = run_test(&t.path, &t.information, t.logic_mode, t.backend, t.opt);
                num_tests += 1;

                match result {
                    Ok(()) => write!(&mut o, " {ANSI_GREEN}P{ANSI_END}")?,
                    Err(info) => {
                        write!(&mut o, " {ANSI_RED}{}{ANSI_END}", info.as_char())?;
                        fails.push(Fail {
                            name: t.offset_path.display().to_string(),
                            mode: t.logic_mode,
                            opt_rounds: t.opt.rounds,
                            backend: t.backend,
                            info,
                        });
                    }
                }
                o.flush()?;
            }
            fails
        } else {
            use rayon::prelude::*;
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(self.num_threads)
                .build()?;

            num_tests = configurations.len();
            pool.install(|| {
                configurations
                    .into_par_iter()
                    .filter_map(|t| {
                        match run_test(&t.path, &t.information, t.logic_mode, t.backend, t.opt) {
                            Ok(()) => {
                                io::stdout().write_all(b".").unwrap();
                                io::stdout().flush().unwrap();
                                None
                            }
                            Err(info) => {
                                let s = format!("{ANSI_RED}{}{ANSI_END}", info.as_char());
                                io::stdout().write_all(s.as_bytes()).unwrap();
                                io::stdout().flush().unwrap();
                                Some(Fail {
                                    name: t.offset_path.display().to_string(),
                                    mode: t.logic_mode,
                                    opt_rounds: t.opt.rounds,
                                    backend: t.backend,
                                    info,
                                })
                            }
                        }
                    })
                    .collect()
            })
        };

        panic::set_hook(hook);

        writeln!(&mut o)?;

        let exit_code = if fails.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };

        report_fails(&mut o, fails, num_tests)?;

        Ok(exit_code)
    }
}

enum FailureInfo {
    Panic(PanicInfo),
    Execution {
        stdout: String,
        stderr: String,
    },
    Mismatch {
        expected: String,
        gotten: String,
    },
    VcdMismatch {
        expected: String,
        gotten: String,
    },
    VirMismatch {
        expected: String,
        gotten: String,
    },
    VirOptMismatch {
        expected: String,
        gotten: String,
    },
    CompileFailure(
        TestPhase,
        Box<dyn std::error::Error + Send + Sync + 'static>,
    ),
    IoFailure(io::Error),
    ExpectPanic,
    ExpectFail(ExpectedFail),
}

impl FailureInfo {
    pub fn as_char(&self) -> char {
        match self {
            Self::Panic(..) => '!',
            Self::Execution { .. } => 'E',
            Self::Mismatch { .. } => 'M',
            Self::VcdMismatch { .. } => 'V',
            Self::VirMismatch { .. } => 'M',
            Self::VirOptMismatch { .. } => 'O',
            Self::CompileFailure(..) => 'C',
            Self::IoFailure(..) => 'I',
            Self::ExpectPanic => 'X',
            Self::ExpectFail(_) => 'F',
        }
    }
}
impl From<io::Error> for FailureInfo {
    fn from(value: io::Error) -> Self {
        Self::IoFailure(value)
    }
}

pub struct TestCase {
    offset_path: PathBuf,
    path: PathBuf,
    information: TestInfo,
    opt: Optimizations,
    logic_mode: LogicMode,
    backend: Backend,
}

pub struct Fail {
    name: String,
    mode: LogicMode,
    opt_rounds: u8,
    backend: Backend,
    info: FailureInfo,
}

#[derive(Clone)]
pub enum VerifyOutput {
    No,
    SortLines,
    Yes,
}

fn display_section(o: &mut io::Stdout, section: &str, content: &str) -> io::Result<()> {
    if !content.is_empty() {
        writeln!(o, "  --- [START {section}] ---")?;
        let stdout = content.strip_suffix("\n").unwrap_or(content);
        writeln!(o, "  {}", stdout.replace("\n", "\n  "))?;
        writeln!(o, "  ---  [END {section}]  ---")?;
    }
    Ok(())
}

fn report_fails(o: &mut io::Stdout, fails: Vec<Fail>, num_tests: usize) -> io::Result<()> {
    if fails.is_empty() {
        writeln!(o, "All {} tests passed!", num_tests)?;
    } else {
        let num_fails = fails.len();
        for fail in fails {
            let Fail {
                name,
                mode,
                opt_rounds,
                backend,
                info,
            } = fail;
            let mode_str = match mode {
                LogicMode::TwoValue => "tvl",
                LogicMode::FourValue => "fvl",
            };
            let backend = match backend {
                Backend::Bytecode => "bytecode",
                Backend::Cranelift => "cranelift",
            };

            write!(o, "+ {name}[{mode_str}-{backend}-O{opt_rounds}]")?;

            match info {
                FailureInfo::Panic(panic) => {
                    writeln!(o, ": Panic")?;
                    writeln!(o)?;
                    let PanicInfo { backtrace, message } = panic;
                    struct X(String, Arc<std::backtrace::Backtrace>);
                    impl fmt::Display for X {
                        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                            self.1.fmt(f)?;
                            f.write_str(&self.0)?;
                            Ok(())
                        }
                    }
                    let output = X(message, backtrace).to_string();
                    display_section(o, "PANIC", &output)?;
                }
                FailureInfo::Execution { stdout, stderr } => {
                    writeln!(o, ": Error")?;
                    writeln!(o)?;
                    display_section(o, "STDOUT", &stdout)?;
                    display_section(o, "STDERR", &stderr)?;
                }
                FailureInfo::Mismatch { expected, gotten } => {
                    writeln!(o, ": Mismatch")?;
                    writeln!(o)?;
                    display_section(o, "EXPECTED", &expected)?;
                    display_section(o, "GOTTEN", &gotten)?;
                }
                FailureInfo::VcdMismatch { expected, gotten } => {
                    writeln!(o, ": VCD Mismatch")?;
                    writeln!(o)?;
                    display_section(o, "EXPECTED", &expected)?;
                    display_section(o, "GOTTEN", &gotten)?;
                }
                FailureInfo::VirMismatch { expected, gotten } => {
                    writeln!(o, ": VIR mismatch")?;
                    writeln!(o)?;
                    display_section(o, "EXPECTED", &expected)?;
                    display_section(o, "GOTTEN", &gotten)?;
                }
                FailureInfo::VirOptMismatch { expected, gotten } => {
                    writeln!(o, ": VIR Optimization mismatch")?;
                    writeln!(o)?;
                    display_section(o, "EXPECTED", &expected)?;
                    display_section(o, "GOTTEN", &gotten)?;
                }
                FailureInfo::CompileFailure(phase, error) => {
                    writeln!(o, ": Compilation failure during {phase:?}")?;
                    writeln!(o, "  {error}")?;
                }
                FailureInfo::IoFailure(error) => {
                    writeln!(o, ": Io failure")?;
                    writeln!(o, "  {error}")?;
                }
                FailureInfo::ExpectPanic => {
                    writeln!(o, ": Expected panic")?;
                }
                FailureInfo::ExpectFail(fail) => {
                    writeln!(o, ": Expected panic")?;
                    writeln!(o, "{:?}", fail.phase)?;
                }
            }
            writeln!(o)?;
        }
        writeln!(
            o,
            "{ANSI_RED}Failed {}/{} tests.{ANSI_END}",
            num_fails, num_tests,
        )?;
    }
    Ok(())
}

#[derive(Clone)]
struct PanicInfo {
    backtrace: Arc<std::backtrace::Backtrace>,
    message: String,
}
thread_local! {
    static PANIC_INFO: RefCell<Option<PanicInfo>> = const { RefCell::new(None) };
}

fn run_test(
    path: &Path,
    test_information: &TestInfo,
    logic_mode: LogicMode,
    backend: Backend,
    opts: Optimizations,
) -> Result<(), FailureInfo> {
    let sdf = test_information
        .annotate_sdf
        .then(|| path.with_extension("sdf"));

    if test_information.verify_ir {
        let design = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let arena = Arena::new();
            let mut builder = DesignBuilder::new();
            match logic_mode {
                LogicMode::TwoValue => {
                    builder.define_macro("__VOGLS__TWO_VALUE_LOGIC", Macro::default());
                }
                LogicMode::FourValue => {}
            }
            for include_dir in &test_information.include_dirs {
                builder.push_include_dir(include_dir.clone());
            }
            builder
                .define_macro("__VOGLS_VERIFY_IR", Macro::default())
                .add_source(path)
                .map_err(|_| {
                    FailureInfo::CompileFailure(TestPhase::LEXING, "failed to tokenize".into())
                })?;

            let parsed = builder.parse(&arena).map_err(|_| {
                FailureInfo::CompileFailure(TestPhase::PARSING, "failed to parse".into())
            })?;
            let mut elab =
                match parsed.elaborate(logic_mode, test_information.top_level_module.as_deref()) {
                    Ok(v) => v,
                    Err(_) => {
                        return Err(FailureInfo::CompileFailure(
                            TestPhase::ELABORATION,
                            "failed to elaborate".into(),
                        ));
                    }
                };
            if let Some(sdf) = sdf.as_deref() {
                if elab.annotate_sdf(sdf).is_err() {
                    return Err(FailureInfo::CompileFailure(
                        TestPhase::LOWERING,
                        "failed to annotate sdf".into(),
                    ));
                }
            }
            if elab.annotate_specify().is_err() {
                return Err(FailureInfo::CompileFailure(
                    TestPhase::LOWERING,
                    "failed to annotate specify".into(),
                ));
            }
            let mut lowered = match elab.lower(vec![]) {
                Ok(l) => l,
                Err(_) => {
                    return Err(FailureInfo::CompileFailure(
                        TestPhase::LOWERING,
                        "failed to lower".into(),
                    ));
                }
            };
            lowered.optimize(opts);
            Result::<_, FailureInfo>::Ok(lowered.emit_ir().to_string())
        }));

        match design {
            Ok(_) if test_information.expect_panic => return Err(FailureInfo::ExpectPanic),
            Err(_) if test_information.expect_panic => {}

            Ok(design) => match design {
                Ok(design) => {
                    let mut asserted = std::fs::read_to_string(path.with_extension("v.ir"))?;
                    if matches!(test_information.mode, SelectLogicMode::Template) {
                        replace_templates(&mut asserted, logic_mode, opts);
                    }
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
            },
            Err(_) => {
                return Err(FailureInfo::Panic(
                    PANIC_INFO.with_borrow(|v| v.clone()).unwrap(),
                ));
            }
        }
    }

    let mut gotten_diagnostics = test_information.verify_diagnostics.then_some(String::new());

    let mut world = StdWorldCaptured::default();
    let result: Result<Result<(), FailureInfo>, Box<dyn std::any::Any + Send + 'static>> =
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            let arena = Arena::new();
            let design = if path
                .extension()
                .is_some_and(|ext| ext.as_encoded_bytes() == b"vir")
            {
                let mut s = std::fs::read_to_string(path)?;
                if matches!(test_information.mode, SelectLogicMode::Template) {
                    replace_templates(&mut s, logic_mode, opts);
                }
                let optimized = read_to_string(path.with_extension("vir.opt")).ok();
                let mut design = VirDesignBuilder::new(&s);
                design.with_logic_mode(logic_mode);
                let mut design = design.parse().map_err(|_| {
                    FailureInfo::CompileFailure(TestPhase::PARSING, "failed to parse VIR".into())
                })?;
                design.optimize(opts);

                if opts.rounds > 0
                    && let Some(mut optimized) = optimized
                {
                    if matches!(test_information.mode, SelectLogicMode::Template) {
                        replace_templates(&mut optimized, logic_mode, opts);
                    }
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
                for include_dir in &test_information.include_dirs {
                    builder.push_include_dir(include_dir.clone());
                }
                builder.add_source(path).map_err(|_| {
                    FailureInfo::CompileFailure(TestPhase::LEXING, "failed to tokenize".into())
                })?;

                let parsed = builder.parse(&arena).map_err(|err| {
                    if let Some(d) = gotten_diagnostics.as_mut() {
                        d.push_str(&err.to_string());
                    }
                    FailureInfo::CompileFailure(TestPhase::PARSING, "failed to parse".into())
                })?;
                let mut elab = match parsed
                    .elaborate(logic_mode, test_information.top_level_module.as_deref())
                {
                    Ok(v) => v,
                    Err(err) => {
                        if let Some(d) = gotten_diagnostics.as_mut() {
                            d.push_str(&err.to_string());
                        }
                        return Err(FailureInfo::CompileFailure(
                            TestPhase::ELABORATION,
                            "failed to elaborate".into(),
                        ));
                    }
                };
                if let Some(sdf) = sdf.as_deref() {
                    if elab.annotate_sdf(sdf).is_err() {
                        return Err(FailureInfo::CompileFailure(
                            TestPhase::LOWERING,
                            "failed to annotate SDF".into(),
                        ));
                    }
                }
                if elab.annotate_specify().is_err() {
                    return Err(FailureInfo::CompileFailure(
                        TestPhase::LOWERING,
                        "failed to annotate specify".into(),
                    ));
                }
                let mut lowered = match elab.lower(vec![]) {
                    Ok(l) => l,
                    Err(err) => {
                        if let Some(d) = gotten_diagnostics.as_mut() {
                            d.push_str(&err.to_string());
                        }
                        return Err(FailureInfo::CompileFailure(
                            TestPhase::LOWERING,
                            "failed to lower".into(),
                        ));
                    }
                };
                lowered.optimize(opts);
                Result::<_, FailureInfo>::Ok(lowered)
            }?;

            let (design, mut state) = match backend {
                Backend::Bytecode => design.to_bytecode(),
                Backend::Cranelift => design.to_cranelift(),
            }
            .map_err(|_| {
                FailureInfo::CompileFailure(
                    TestPhase::COMPILATION,
                    "failed to convert to execution format".into(),
                )
            })?;
            let timeout = test_information.timeout.map_or(u64::MAX, |(v, unit)| {
                TimeResolution {
                    unit,
                    size: TimeSize::N1,
                }
                .truncate_or_multiply_to(v, design.time_resolution())
            });
            design.run(&mut state, &mut world, timeout).map_err(|_| {
                let stdout = world.stdout.read_to_string().unwrap();
                let stderr = world.stderr.read_to_string().unwrap();
                FailureInfo::Execution { stdout, stderr }
            })
        }));

    if let Some(gotten_diagnostics) = gotten_diagnostics {
        let mut diagnostics_path = path.to_path_buf();
        diagnostics_path.add_extension("diagnostics");
        let expected = std::fs::read_to_string(&diagnostics_path)?;
        let expected = expected
            .trim()
            .replace("{path}", &path.display().to_string());
        let gotten = gotten_diagnostics.trim();

        if expected != gotten {
            return Err(FailureInfo::Mismatch {
                expected: expected.to_string(),
                gotten: gotten.to_string(),
            });
        }
    }

    let result = match result {
        Ok(_) if test_information.expect_panic => return Err(FailureInfo::ExpectPanic),
        Err(_) if test_information.expect_panic => return Ok(()),

        Ok(v) => v,
        Err(_) => {
            return Err(FailureInfo::Panic(
                PANIC_INFO.with_borrow(|v| v.clone()).unwrap(),
            ));
        }
    };
    match result {
        Err(err) => match &test_information.fail {
            None => return Err(err),
            Some(fail) => match err {
                FailureInfo::CompileFailure(err_phase, _) if fail.phase.contains(err_phase) => {
                    return Ok(());
                }
                FailureInfo::Execution { .. } if fail.phase.contains(TestPhase::EXECUTION) => {
                    return Ok(());
                }
                err => return Err(err),
            },
        },
        Ok(_) => match &test_information.fail {
            None => {}
            Some(fail) => return Err(FailureInfo::ExpectFail(fail.clone())),
        },
    }

    if matches!(
        test_information.verify_stdout,
        VerifyOutput::Yes | VerifyOutput::SortLines
    ) {
        let stdout = world.stdout.read_to_string().unwrap();

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

    if test_information.verify_vcd {
        static CTR: AtomicU64 = AtomicU64::new(0);
        let idx = CTR.fetch_add(1, Ordering::Relaxed);
        let vcd_path_dir = temp_dir();
        let vcd_path = vcd_path_dir.join(format!("trace-{idx}.vcd"));

        let arena = Arena::new();
        let mut builder = DesignBuilder::new();
        match logic_mode {
            LogicMode::TwoValue => {
                builder.define_macro("__VOGLS__TWO_VALUE_LOGIC", Macro::default());
            }
            LogicMode::FourValue => {}
        }
        builder.add_source(path).expect("failed to tokenize");
        let parsed = builder.parse(&arena).expect("failed to parse");
        let mut elab = parsed
            .elaborate(logic_mode, test_information.top_level_module.as_deref())
            .expect("failed to elaborate");
        if let Some(sdf) = sdf.as_deref() {
            elab.annotate_sdf(sdf).expect("failed to annotate SDF");
        }
        elab.annotate_specify().expect("failed to annotate specify");
        let mut design = match elab.lower(vec![]) {
            Ok(d) => d,
            Err(_) => panic!("failed to lower"),
        };
        design.trace_vcd(vcd_path.clone());
        design.optimize(opts);

        let (design, mut state) = match backend {
            Backend::Bytecode => design.to_bytecode(),
            Backend::Cranelift => design.to_cranelift(),
        }
        .expect("failed to convert to execution format");
        let timeout = test_information.timeout.map_or(u64::MAX, |(v, unit)| {
            TimeResolution {
                unit,
                size: TimeSize::N1,
            }
            .truncate_or_multiply_to(v, design.time_resolution())
        });
        let mut world = StdWorldCaptured::default();
        design
            .run(&mut state, &mut world, timeout)
            .expect("failed to execute");

        let mut fixture_vcd_path = path.to_path_buf();
        fixture_vcd_path.add_extension("vcd");

        if !fixture_vcd_path.exists() {
            match logic_mode {
                LogicMode::TwoValue => _ = fixture_vcd_path.add_extension("tv"),
                LogicMode::FourValue => _ = fixture_vcd_path.add_extension("fv"),
            }
        }

        let fixture = std::fs::read_to_string(&fixture_vcd_path)?;
        let generated = std::fs::read_to_string(&vcd_path)?;

        std::fs::remove_file(&vcd_path)?;

        if fixture != generated {
            return Err(FailureInfo::VcdMismatch {
                expected: fixture,
                gotten: generated,
            });
        }
    }

    Ok(())
}

fn replace_templates(s: &mut String, mode: LogicMode, opts: Optimizations) {
    use regex::{Captures, regex};

    let mode_str = match mode {
        LogicMode::TwoValue => "tv",
        LogicMode::FourValue => "fv",
    };
    let other_mode_str = match mode {
        LogicMode::TwoValue => "fv",
        LogicMode::FourValue => "tv",
    };
    let opt_str = if opts.rounds == 0 { "O0" } else { "On" };
    let want = format!("{mode_str}{opt_str}");

    *s = s.replace("{mode}", mode_str);
    *s = s.replace("{!mode}", other_mode_str);
    *s = regex!(r"(?m)^\{\?mode=([A-Za-z0-9]+)\}(.*)(\n?)")
        .replace_all(s, |c: &Captures| {
            if c[1] == want {
                format!("{}{}", &c[2], &c[3])
            } else {
                String::new()
            }
        })
        .into_owned();
    *s = regex!(r"(?m)^\{\?!mode=([A-Za-z0-9]+)\}(.*)(\n?)")
        .replace_all(s, |c: &Captures| {
            if c[1] != want {
                format!("{}{}", &c[2], &c[3])
            } else {
                String::new()
            }
        })
        .into_owned();
}
