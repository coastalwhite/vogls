use serde::Deserialize;
use std::error::Error;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use vogls::design::{Arena, Macro};
use vogls::{DesignBuilder, LogicMode, OptFlags, Optimizations, StdWorldCaptured};

use crate::{ANSI_END, ANSI_GREEN, ANSI_RED, Backend};

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct IntegrationArgs {
    name: Option<String>,

    #[arg(short, long)]
    filter: Option<String>,

    #[arg(short = 'n', long, default_value_t = 0)]
    num_threads: usize,

    #[arg(short = 'B', long)]
    bytecode: bool,
    #[arg(short = 'C', long)]
    cranelift: bool,

    #[arg(short = 'T', long)]
    two_value: bool,
    #[arg(short = 'F', long)]
    four_value: bool,

    #[arg(short = 'O', long)]
    optimized: bool,

    /// Dump a VCD trace of each run to this path.
    #[arg(long)]
    vcd: Option<String>,
}

#[derive(Deserialize)]
#[serde(remote = "LogicMode")]
enum LogicModeDef {
    #[serde(rename = "two-valued")]
    TwoValue,
    #[serde(rename = "four-valued")]
    FourValue,
}

fn opt_logic_mode<'de, D>(d: D) -> Result<Option<LogicMode>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    LogicModeDef::deserialize(d).map(Some)
}

#[derive(Deserialize)]
pub struct TestConfig {
    name: String,
    top_module: String,
    sources: Vec<String>,
    defines: Vec<String>,
    #[serde(default)]
    include_dirs: Vec<String>,
    #[serde(default, deserialize_with = "opt_logic_mode")]
    mode: Option<LogicMode>,
    output: Option<String>,
    #[serde(default)]
    opt_rounds: Vec<u8>,
    /// Skip this test, with the reason why (e.g. it does not terminate yet).
    skip: Option<String>,
}

struct Test {
    name: String,
    test_toml: PathBuf,
    top_module: String,
    sources: Vec<String>,
    defines: Vec<String>,
    include_dirs: Vec<String>,
    output: Option<String>,
    optimizations: Optimizations,
    mode: LogicMode,
    backend: Backend,
}

impl Test {
    pub fn display_name(&self) -> String {
        let name = &self.name;
        let mode_str = match self.mode {
            LogicMode::TwoValue => "tv",
            LogicMode::FourValue => "fv",
        };
        let opt_rounds = self.optimizations.rounds;
        let backend_str = match self.backend {
            Backend::Bytecode => "bytecode",
            Backend::Cranelift => "clif",
        };
        format!("{name}[{backend_str},{mode_str},O{opt_rounds}]")
    }
}

fn collect_tests() -> Result<Vec<PathBuf>, Box<dyn Error + Send + Sync>> {
    let manifest_path = env!("CARGO_WORKSPACE_DIR");
    let tests_dir = Path::new(manifest_path).join("tests");

    let walker = std::fs::read_dir(&tests_dir)?;
    let mut tests = Vec::new();
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
        } else if file_type.is_file() && entry.file_name().as_encoded_bytes() == b"test.toml" {
            let path = entry.path();
            tests.push(path.to_path_buf());
        }
        walkers.push(w);
    }
    Ok(tests)
}

impl IntegrationArgs {
    pub fn run(&self) -> Result<std::process::ExitCode, Box<dyn Error + Send + Sync>> {
        let tests = collect_tests()?;
        let mut collected = Vec::new();
        let mut skipped = Vec::new();

        for test in tests.iter() {
            let config = std::fs::read_to_string(test)?;
            let config = toml::from_str::<TestConfig>(&config)?;

            if self.name.as_deref().is_some_and(|name| config.name != name) {
                continue;
            }

            if let Some(filter) = self.filter.as_ref()
                && !config.name.contains(filter)
            {
                continue;
            }

            if let Some(reason) = config.skip {
                skipped.push((config.name, reason));
                continue;
            }

            let opt_rounds = config.opt_rounds.as_slice();
            let mut opt_rounds: Vec<_> = if opt_rounds.is_empty() {
                vec![10]
            } else {
                opt_rounds.to_vec()
            };

            let mut modes: Vec<LogicMode> = match config.mode {
                None => vec![LogicMode::TwoValue, LogicMode::FourValue],
                Some(mode) => vec![mode],
            };
            let mut backends = vec![Backend::Bytecode, Backend::Cranelift];

            if self.bytecode || self.cranelift {
                backends.retain(|b| match b {
                    Backend::Bytecode => self.bytecode,
                    Backend::Cranelift => self.cranelift,
                });
            }
            if self.optimized {
                opt_rounds.retain(|o| *o > 0);
            }
            if self.two_value || self.four_value {
                modes.retain(|m| match m {
                    LogicMode::TwoValue => self.two_value,
                    LogicMode::FourValue => self.four_value,
                });
            }

            for &rounds in &opt_rounds {
                for &mode in &modes {
                    for &backend in &backends {
                        collected.push(Test {
                            name: config.name.clone(),
                            test_toml: test.clone(),
                            sources: config.sources.clone(),
                            defines: config.defines.clone(),
                            include_dirs: config.include_dirs.clone(),
                            output: config.output.clone(),
                            top_module: config.top_module.clone(),
                            optimizations: Optimizations {
                                rounds,
                                flags: OptFlags::ALL,
                            },
                            mode,
                            backend,
                        });
                    }
                }
            }
        }

        let max_name_length = collected
            .iter()
            .map(|t| t.display_name().len())
            .max()
            .unwrap_or_default();

        let num_configurations = collected.len();
        let num_tests = tests.len();
        println!(
            "Running {num_configurations} configurations across {num_tests} integration tests..."
        );
        for (name, reason) in &skipped {
            println!("  skipping {name}: {reason}");
        }

        let fails = if self.num_threads == 1 || self.name.is_some() {
            let mut fails = Vec::new();
            for test in &collected {
                let display_name = test.display_name();
                let padding_len = max_name_length - display_name.len();
                print!("  {display_name}...{:padding_len$}  ", "");
                io::stdout().flush()?;
                let start = std::time::SystemTime::now();
                match run_test_direct(test, self.vcd.as_deref()) {
                    Ok(_) => print!(" {ANSI_GREEN}P{ANSI_END}"),
                    Err(err) => {
                        print!(" {ANSI_RED}F{ANSI_END}");
                        fails.push((display_name, err));
                    }
                }
                let end = start.elapsed().unwrap();
                println!(" {:.02}s", end.as_secs_f64());
            }
            fails
        } else {
            use rayon::prelude::*;
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(self.num_threads)
                .build()?;

            pool.install(|| {
                collected
                    .into_par_iter()
                    .filter_map(|t| match run_test_direct(&t, self.vcd.as_deref()) {
                        Ok(()) => {
                            io::stdout().write_all(b".").unwrap();
                            io::stdout().flush().unwrap();
                            None
                        }
                        Err(info) => {
                            let s = format!("{ANSI_RED}F{ANSI_END}");
                            io::stdout().write_all(s.as_bytes()).unwrap();
                            io::stdout().flush().unwrap();
                            Some((t.display_name(), info))
                        }
                    })
                    .collect()
            })
        };
        println!();

        if fails.is_empty() {
            println!("{ANSI_GREEN}All {num_configurations} configurations succeeded!{ANSI_END}");
            Ok(ExitCode::SUCCESS)
        } else {
            let num_fails = fails.len();
            eprintln!(
                "{ANSI_RED}Failed {num_fails} / {num_configurations} configurations!{ANSI_END}"
            );
            for (name, fail) in fails {
                eprintln!();
                eprintln!("--- START {name} ---");
                eprintln!("{fail}");
                eprintln!("---  END {name}  ---");
            }
            Ok(ExitCode::FAILURE)
        }
    }
}

fn run_test_direct(test: &Test, vcd: Option<&str>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let root = test.test_toml.parent().unwrap();

    let mut builder = DesignBuilder::new();
    let mut world = StdWorldCaptured::default();
    world.set_current_dir(root);

    for key in &test.defines {
        builder.define_macro(key, Macro::default());
    }

    for include_dir in &test.include_dirs {
        builder.push_include_dir(root.join(include_dir));
    }

    for source in &test.sources {
        let source = root.join(Path::new(source));
        builder.add_source_in_world(&mut world, &source)?;
    }

    let arena = Arena::new();
    let design = builder.parse(&arena)?;
    let mut design = design
        .elaborate(test.mode, Some(&test.top_module))
        .map_err(|_| <Box<dyn Error + Send + Sync>>::from("elaboration failure"))?;
    if let Some(vcd) = vcd {
        design.trace_vcd(vcd);
    }
    if let Err(err) = design.annotate_specify() {
        eprintln!("{err}");
        return Err("failed to annotate specify".into());
    }
    let mut design = match design.lower(vec![]) {
        Ok(design) => design,
        Err(_) => return Err("lower failure".into()),
    };
    design.optimize(test.optimizations);
    let (design, mut state) = match test.backend {
        Backend::Bytecode => design.to_bytecode(),
        Backend::Cranelift => design.to_cranelift(),
    }
    .map_err(|_| <Box<dyn Error + Send + Sync>>::from("backend failure"))?;

    design
        .run(&mut state, &mut world, u64::MAX)
        .map_err(|_| <Box<dyn Error + Send + Sync>>::from("execution failure"))?;

    if let Some(output) = test.output.as_ref() {
        let stdout = world.stdout.read_to_string()?;
        if stdout.as_str() != output {
            return Err(format!(
                "output mismatch. expected = {:?}, gotten = {:?}",
                output.as_bytes(),
                stdout.as_bytes()
            )
            .into());
        }
    }

    Ok(())
}
