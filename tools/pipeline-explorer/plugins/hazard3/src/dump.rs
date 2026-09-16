//! Prints a Hazard3 trace as canonical JSON, natively.
//!
//! The webapp's end-to-end test runs the same program through the uploaded
//! wasm plugin and diffs the two, which checks the plugin ABI (allocation,
//! the manifest, the trace encoding) without a browser in the loop.
//!
//! ```sh
//! hazard3-plugin-dump <asm-file> <cycles> <comma-separated config>
//! ```

use pipeline_explorer_plugin_hazard3::run;

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: hazard3-plugin-dump <asm> <cycles> <config>");
    let cycles: u32 = args.next().expect("missing cycle count").parse().unwrap();
    let cfg: Vec<u32> = match args.next() {
        None => Vec::new(),
        Some(cfg) if cfg.is_empty() => Vec::new(),
        Some(cfg) => cfg.split(',').map(|v| v.parse().unwrap()).collect(),
    };

    let assembly = std::fs::read_to_string(&path).expect("failed to read the assembly");
    let trace = run(&assembly, &cfg, cycles).expect("failed to trace the design");

    let instructions: Vec<String> = trace
        .instructions
        .iter()
        .map(|i| format!("\"{}\"", escape(i)))
        .collect();
    let stages: Vec<String> = trace.stages.iter().map(|s| format!("\"{}\"", escape(s))).collect();
    let traces: Vec<String> = trace
        .traces
        .iter()
        .map(|t| {
            let values: Vec<String> = t.iter().map(u32::to_string).collect();
            format!("[{}]", values.join(","))
        })
        .collect();

    println!(
        "{{\"cycles\":{},\"instructions\":[{}],\"stages\":[{}],\"traces\":[{}]}}",
        trace.cycles,
        instructions.join(","),
        stages.join(","),
        traces.join(","),
    );
}
