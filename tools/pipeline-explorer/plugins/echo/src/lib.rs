//! The smallest possible design plugin: it reports its own inputs instead of
//! simulating anything.
//!
//! Two jobs. It is the worked example for the plugin ABI -- a manifest and a
//! `run`, no design in sight -- and it is the fixture the end-to-end test uses
//! to check that assembly, configuration and cycle count actually survive the
//! trip across the ABI. Pinning that down on a plugin whose output is fully
//! determined by its inputs keeps the check independent of how any real design
//! happens to behave.
//!
//! Build it with `just plugin-echo`.

use pipeline_explorer_plugin::{Trace, cfg_u32};

/// Deliberately mixes both field types, so the host's encoding of each is
/// exercised.
pub const MANIFEST: &str = r#"{
  "abi": 1,
  "id": "echo",
  "name": "Echo (test fixture)",
  "fields": [
    { "id": "first",  "type": "checkbox", "title": "First flag",  "default": true  },
    { "id": "second", "type": "checkbox", "title": "Second flag", "default": false },
    { "id": "count",  "type": "number",   "title": "A number",    "default": 7     }
  ]
}"#;

/// Reports what the host sent: one "stage" whose per-cycle trace is the raw
/// configuration array, and a disassembly listing the other two inputs.
pub fn run(assembly: &str, cfg: &[u32], num_cycles: u32) -> Result<Trace, String> {
    if assembly.trim() == "fail" {
        // Gives the test a way to exercise the error path of the ABI.
        return Err("echo plugin asked to fail".to_owned());
    }

    Ok(Trace {
        instructions: vec![
            format!("asm {} bytes", assembly.len()),
            format!("cycles {num_cycles}"),
            format!("fields {}", cfg.len()),
        ],
        stages: vec!["CFG".to_owned()],
        traces: vec![(0..cfg.len()).map(|i| cfg_u32(cfg, i)).collect()],
        cycles: cfg.len() as u32,
    })
}

pipeline_explorer_plugin::export_plugin!(manifest = MANIFEST, run = run);
