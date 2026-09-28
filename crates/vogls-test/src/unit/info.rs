use std::fmt;
use std::ops::BitOrAssign;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use vogls::{LogicMode, OptFlags};
use vogls_ir::time::TimeUnit;

use super::VerifyOutput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Bytecode,
    Cranelift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectLogicMode {
    All,
    Only(LogicMode),
    Template,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectBackend {
    All,
    Only(Backend),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TestPhase(u8);

#[derive(Clone)]
pub struct ExpectedFail {
    pub phase: TestPhase,
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
pub struct TestInfo {
    pub fail: Option<ExpectedFail>,
    pub expect_panic: bool,
    pub verify_stdout: VerifyOutput,
    pub verify_ir: bool,
    pub verify_vcd: bool,
    pub verify_diagnostics: bool,
    pub annotate_sdf: bool,
    pub timeout: Option<(u64, TimeUnit)>,
    pub top_level_module: Option<String>,
    pub mode: SelectLogicMode,
    pub backend: SelectBackend,
    pub opt_flags: OptFlags,
    pub include_dirs: Vec<PathBuf>,
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
    pub fn parse(path: &Path, content: &str) -> Result<Self, String> {
        let mut info = TestInfo {
            fail: None,
            expect_panic: false,
            verify_stdout: VerifyOutput::No,
            verify_ir: false,
            verify_vcd: false,
            verify_diagnostics: false,
            annotate_sdf: false,
            top_level_module: None,
            timeout: None,
            mode: SelectLogicMode::All,
            backend: SelectBackend::All,
            opt_flags: OptFlags::ALL,
            include_dirs: Vec::new(),
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
                "verify-diagnostics" => info.verify_diagnostics = true,
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
                                return Err(format!("unknown compile phase '{kind}'"));
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
                _ if line.starts_with("include-dir=") => {
                    info.include_dirs.push(
                        path.parent()
                            .unwrap()
                            .join(line["include-dir=".len()..].trim()),
                    );
                }
                _ => return Err(format!("Invalid vogls test command '{line}'")),
            }
        }

        Ok(info)
    }
}
