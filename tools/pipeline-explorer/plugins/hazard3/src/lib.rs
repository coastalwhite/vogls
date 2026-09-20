//! Hazard3 packaged as an uploadable pipeline-explorer design plugin.
//!
//! This is the reference plugin: it carries the same design the webapp bundles,
//! which makes it easy to check an uploaded binary against the built-in path.
//! A plugin for a design the explorer does not know about looks exactly like
//! this file — a manifest describing the knobs, and a `run` that turns
//! assembly plus those knobs into a trace.
//!
//! Build it with:
//!
//! ```sh
//! just plugin-hazard3
//! ```

use pipeline_explorer::{Hazard3Config, get_hazard3_trace};
use pipeline_explorer_plugin::{Trace, cfg_bool, custom};

/// The knobs the webapp renders, in the order `run` reads them out of the
/// configuration array, plus the mnemonics this plugin accepts on top of the
/// base ISA. `instructions` is what the editor highlights; [`custom::ALL`] in
/// `run` is what assembles them.
pub const MANIFEST: &str = r#"{
  "abi": 1,
  "id": "hazard3",
  "name": "Hazard3 (plugin)",
  "fields": [
    { "id": "extension_m",     "type": "checkbox", "title": "Enable M Extension",          "default": true  },
    { "id": "mul_fast",        "type": "checkbox", "title": "Single-Cycle Multiply",       "default": false },
    { "id": "mulh_fast",       "type": "checkbox", "title": "Single-Cycle Mulh",           "default": false },
    { "id": "muldiv_unroll_2", "type": "checkbox", "title": "Two Muldiv Steps per Cycle",  "default": false },
    { "id": "reduced_bypass",  "type": "checkbox", "title": "Reduced Bypass Network",      "default": false },
    { "id": "branch_predictor","type": "checkbox", "title": "Branch Predictor",            "default": false },
    { "id": "fast_branchcmp",  "type": "checkbox", "title": "Fast Branch Compare",         "default": true  }
  ],
  "instructions": ["square", "l1"]
}"#;

/// Reads the positional configuration back into the shape the design code
/// wants. The indices line up with `fields` in [`MANIFEST`].
pub fn config_from(cfg: &[u32]) -> Hazard3Config {
    Hazard3Config {
        extension_m: cfg_bool(cfg, 0),
        mul_fast: cfg_bool(cfg, 1),
        mulh_fast: cfg_bool(cfg, 2),
        muldiv_unroll_2: cfg_bool(cfg, 3),
        reduced_bypass: cfg_bool(cfg, 4),
        branch_predictor: cfg_bool(cfg, 5),
        fast_branchcmp: cfg_bool(cfg, 6),
    }
}

/// Assembles `assembly`, runs it on Hazard3, and reports where each
/// instruction sat in the pipeline every cycle.
///
/// The custom mnemonics are registered here rather than implemented in the
/// design: they are shorthands for instructions Hazard3 already runs, so the
/// trace shows what they stand for. `square` is a `mul`, and so needs the M
/// extension like any other.
pub fn run(assembly: &str, cfg: &[u32], num_cycles: u32) -> Result<Trace, String> {
    let value = get_hazard3_trace(assembly, num_cycles, &config_from(cfg), custom::ALL)
        .map_err(|err| err.to_string())?;
    Ok(Trace {
        instructions: value.instructions,
        stages: value.pipeline.keys.iter().map(|s| (*s).to_owned()).collect(),
        traces: value.pipeline.traces,
        cycles: value.pipeline.cycles as u32,
    })
}

pipeline_explorer_plugin::export_plugin!(manifest = MANIFEST, run = run);
