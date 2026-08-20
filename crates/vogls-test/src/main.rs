use std::cell::RefCell;
use std::env::temp_dir;
use std::fmt;
use std::fs::read_to_string;
use std::io::{self, Write};
use std::ops::BitOrAssign;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use clap::Parser;
use vogls::design::{Arena, Macro};
use vogls::{DesignBuilder, SimulationIo, VirDesignBuilder};
use vogls_ir::LogicMode;
use vogls_ir::optimize::{OptFlags, Optimizations};
use vogls_ir::time::{TimeResolution, TimeSize, TimeUnit};

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
    #[arg(short = 'B', long)]
    bytecode: bool,
    #[arg(short = 'C', long)]
    cranelift: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Bytecode,
    Cranelift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectLogicMode {
    All,
    Only(LogicMode),
    Template,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectBackend {
    All,
    Only(Backend),
}

impl SelectLogicMode {
    pub fn selection(self, input: &[LogicMode]) -> &[LogicMode] {
        match self {
            Self::All | Self::Template => input,
            Self::Only(LogicMode::TwoValue) if input.contains(&LogicMode::TwoValue) => {
                &[LogicMode::TwoValue]
            }
            Self::Only(LogicMode::FourValue) if input.contains(&LogicMode::FourValue) => {
                &[LogicMode::FourValue]
            }
            Self::Only(_) => &[],
        }
    }
}
impl SelectBackend {
    pub fn selection(self, input: &[Backend]) -> &[Backend] {
        match self {
            Self::All => input,
            Self::Only(Backend::Bytecode) if input.contains(&Backend::Bytecode) => {
                &[Backend::Bytecode]
            }
            Self::Only(Backend::Cranelift) if input.contains(&Backend::Cranelift) => {
                &[Backend::Cranelift]
            }
            Self::Only(_) => &[],
        }
    }
}

#[derive(Clone)]
pub struct ExpectedFail {
    phase: TestPhase,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TestPhase(u8);

impl TestPhase {
    pub const EMPTY: Self = Self(0);
    pub const LEXING: Self = Self(0b0000_0001u8);
    pub const PARSING: Self = Self(0b0000_0010u8);
    pub const ELABORATION: Self = Self(0b0000_0100u8);
    pub const LOWERING: Self = Self(0b0000_1000u8);
    pub const COMPILATION: Self = Self(0b0001_0000u8);
    pub const EXECUTION: Self = Self(0b0010_0000u8);
    pub const ALL: Self = Self(0b0011_1111u8);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOrAssign for TestPhase {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl fmt::Debug for TestPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;
        let mut fst = true;
        macro_rules! item {
            ($name:ident, $display:literal) => {
                if self.contains(Self::$name) {
                    if !fst {
                        f.write_char('|')?;
                    }
                    f.write_str($display)?;
                    #[allow(unused_assignments)]
                    {
                        fst = false;
                    }
                }
            };
        }
        item!(LEXING, "lex");
        item!(PARSING, "parse");
        item!(ELABORATION, "elaborate");
        item!(LOWERING, "lower");
        item!(COMPILATION, "compile");
        item!(EXECUTION, "execute");
        Ok(())
    }
}

impl FromStr for TestPhase {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lex" => Ok(Self::LEXING),
            "parse" => Ok(Self::PARSING),
            "elaborate" => Ok(Self::ELABORATION),
            "lower" => Ok(Self::LOWERING),
            "compile" => Ok(Self::COMPILATION),
            "execute" => Ok(Self::EXECUTION),
            "*" => Ok(Self::ALL),
            _ => Err(()),
        }
    }
}

#[derive(Clone)]
struct TestInfo {
    fail: Option<ExpectedFail>,
    expect_panic: bool,
    verify_stdout: VerifyOutput,
    verify_ir: bool,
    verify_vcd: bool,
    annotate_sdf: bool,
    timeout: Option<(u64, TimeUnit)>,
    top_level_module: Option<String>,
    mode: SelectLogicMode,
    backend: SelectBackend,
    opt_flags: OptFlags,
}

fn parse_opts(s: &str) -> Option<OptFlags> {
    Some(match s {
        "*" => OptFlags::ALL,
        "0" => OptFlags::EMPTY,
        "constant-propagation" => OptFlags::CONSTANT_PROPAGATION,
        "common_subexpr_elim" => OptFlags::COMMON_SUBEXPR_ELIM,
        "deadcode_elimination" => OptFlags::DEADCODE_ELIMINATION,
        "peephole" => OptFlags::PEEPHOLE,
        _ => return None,
    })
}

impl TestInfo {
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut info = TestInfo {
            fail: None,
            expect_panic: false,
            verify_stdout: VerifyOutput::No,
            verify_ir: false,
            verify_vcd: false,
            annotate_sdf: false,
            top_level_module: None,
            timeout: None,
            mode: SelectLogicMode::All,
            backend: SelectBackend::All,
            opt_flags: OptFlags::ALL,
        };

        for line in content.lines() {
            if !line.starts_with("// vogls:") {
                break;
            }

            let line = &line["// vogls:".len()..];
            let line = line.trim();

            match line {
                "verify-stdout" => info.verify_stdout = VerifyOutput::Yes,
                "verify-stdout[sort-lines]" => info.verify_stdout = VerifyOutput::SortLines,
                "verify-ir" => info.verify_ir = true,
                "verify-vcd" => info.verify_vcd = true,
                "annotate-sdf" => info.annotate_sdf = true,
                "panic" => info.expect_panic = true,
                _ if line.starts_with("fail") => {
                    if line == "fail" {
                        info.fail = Some(ExpectedFail {
                            phase: TestPhase::ALL,
                        });
                    } else {
                        assert!(line.starts_with("fail="));
                        let mut phase = TestPhase::EMPTY;
                        for kind in line["fail=".len()..].split(',') {
                            let kind = kind.trim();
                            let Ok(p) = TestPhase::from_str(kind) else {
                                return Err(format!("unknown compile phase '{kind}'").into());
                            };
                            phase |= p;
                        }
                        info.fail = Some(ExpectedFail { phase });
                    }
                }
                _ if line.starts_with("tlm=") => {
                    info.top_level_module = Some(line[4..].trim().to_string());
                }
                _ if line.starts_with("timeout=") => {
                    let value = &line[8..];
                    let at = value
                        .find(|c: char| !c.is_ascii_digit())
                        .unwrap_or(value.len());
                    let (value, unit) = value.split_at(at);
                    let value = value.parse().expect("failed to parse");
                    let unit = TimeUnit::from_str(unit.trim()).expect("Invalid unit");
                    info.timeout = Some((value, unit));
                }
                _ if line.starts_with("mode=") => match &line["mode=".len()..] {
                    "two-value-logic" => info.mode = SelectLogicMode::Only(LogicMode::TwoValue),
                    "four-value-logic" => info.mode = SelectLogicMode::Only(LogicMode::FourValue),
                    "template" => info.mode = SelectLogicMode::Template,
                    _ => return Err("failed to parse 'mode'".into()),
                },
                _ if line.starts_with("backend=") => match &line["backend=".len()..] {
                    "bytecode" => info.backend = SelectBackend::Only(Backend::Bytecode),
                    "cranelift" => info.backend = SelectBackend::Only(Backend::Cranelift),
                    _ => return Err("failed to parse 'backend'".into()),
                },
                _ if line.starts_with("disable-optimization=") => {
                    let opt = &line["disable-optimization=".len()..].trim();
                    let Some(opt) = parse_opts(opt) else {
                        return Err(format!("Invalid vogls optimization '{opt}'"));
                    };
                    info.opt_flags &= !opt;
                }
                _ if line.starts_with("enable-optimization=") => {
                    let opt = &line["enable-optimization=".len()..].trim();
                    let Some(opt) = parse_opts(opt) else {
                        return Err(format!("Invalid vogls optimization '{opt}'"));
                    };
                    info.opt_flags |= opt;
                }
                _ => return Err(format!("Invalid vogls test command '{line}'")),
            }
        }

        Ok(info)
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

struct TestCase {
    offset_path: PathBuf,
    path: PathBuf,
    information: TestInfo,
    opt: Optimizations,
    logic_mode: LogicMode,
    backend: Backend,
}

struct Fail {
    name: String,
    mode: LogicMode,
    opt_rounds: u8,
    backend: Backend,
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
    let mut backends = Vec::new();
    if args.bytecode {
        backends.push(Backend::Bytecode);
    }
    if args.cranelift {
        backends.push(Backend::Cranelift);
    }
    if !(args.bytecode | args.cranelift) {
        backends.extend([Backend::Bytecode, Backend::Cranelift]);
    }

    let mut num_tests = 0;
    let opt_rounds_configurations: &[u8] = match args.opt_rounds {
        None => &[0, 2],
        Some(o) => &[o],
    };
    let mut o = std::io::stdout();

    let mut configurations = Vec::<TestCase>::new();

    for path in paths.iter() {
        let offset_path = path.as_path();
        let path = tests_dir.join(offset_path);
        let s = std::fs::read_to_string(&path)?;
        let test_information = TestInfo::parse(&s)?;

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
    let fails = if args.num_threads == 1 {
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
            num_tests += usize::from(!matches!(result, Ok(PassKind::Skip)));

            match result {
                Ok(PassKind::Skip) => write!(&mut o, " {ANSI_GREEN}S{ANSI_END}")?,
                Ok(PassKind::Succeed) => write!(&mut o, " {ANSI_GREEN}P{ANSI_END}")?,
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
            .num_threads(args.num_threads)
            .build()?;

        num_tests = configurations.len();
        pool.install(|| {
            configurations
                .into_par_iter()
                .filter_map(|t| {
                    match run_test(&t.path, &t.information, t.logic_mode, t.backend, t.opt) {
                        Ok(PassKind::Skip) => {
                            io::stdout().write_all(b"S").unwrap();
                            io::stdout().flush().unwrap();
                            None
                        }
                        Ok(PassKind::Succeed) => {
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

pub enum PassKind {
    Succeed,
    Skip,
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
) -> Result<PassKind, FailureInfo> {
    let sdf = test_information
        .annotate_sdf
        .then(|| path.with_extension("sdf"));

    let stdout = Io::default();
    let stderr = Io::default();

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

    let result: Result<Result<(), FailureInfo>, Box<dyn std::any::Any + Send + 'static>> =
        std::panic::catch_unwind(|| {
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
                builder.add_source(path).map_err(|_| {
                    FailureInfo::CompileFailure(TestPhase::LEXING, "failed to tokenize".into())
                })?;

                let parsed = builder.parse(&arena).map_err(|_| {
                    FailureInfo::CompileFailure(TestPhase::PARSING, "failed to parse".into())
                })?;
                let mut elab = match parsed
                    .elaborate(logic_mode, test_information.top_level_module.as_deref())
                {
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
                    Err(_) => {
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
            design
                .run(
                    &mut state,
                    &mut SimulationIo {
                        stdout: Box::new(stdout.clone()) as _,
                        stderr: Box::new(stderr.clone()) as _,
                    },
                    timeout,
                )
                .map_err(|_| {
                    let stdout = stdout.0.lock().unwrap();
                    let stdout = std::str::from_utf8(&stdout).unwrap();
                    let stderr = stderr.0.lock().unwrap();
                    let stderr = std::str::from_utf8(&stderr).unwrap();
                    FailureInfo::Execution {
                        stdout: stdout.to_string(),
                        stderr: stderr.to_string(),
                    }
                })
        });

    let result = match result {
        Ok(_) if test_information.expect_panic => return Err(FailureInfo::ExpectPanic),
        Err(_) if test_information.expect_panic => return Ok(PassKind::Succeed),

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
                    return Ok(PassKind::Succeed);
                }
                FailureInfo::Execution { .. } if fail.phase.contains(TestPhase::EXECUTION) => {
                    return Ok(PassKind::Succeed);
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
        design
            .run(
                &mut state,
                &mut SimulationIo {
                    stdout: Box::new(stdout.clone()) as _,
                    stderr: Box::new(stderr.clone()) as _,
                },
                timeout,
            )
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

    Ok(PassKind::Succeed)
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
            if &c[1] == want {
                format!("{}{}", &c[2], &c[3])
            } else {
                String::new()
            }
        })
        .into_owned();
    *s = regex!(r"(?m)^\{\?!mode=([A-Za-z0-9]+)\}(.*)(\n?)")
        .replace_all(s, |c: &Captures| {
            if &c[1] != want {
                format!("{}{}", &c[2], &c[3])
            } else {
                String::new()
            }
        })
        .into_owned();
}
