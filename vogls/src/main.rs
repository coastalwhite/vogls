use std::path::Path;

use vogls::ExecutionContext;

fn usage() {
    eprintln!(
        "usage: {} <path/to/file.v> <top-level module>",
        std::env::args().next().unwrap()
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::args().nth(1) else {
        usage();
        std::process::exit(2);
    };
    let Some(tl_module_name) = std::env::args().nth(2) else {
        usage();
        std::process::exit(2);
    };

    let path = Path::new(&path);
    vogls::run(
        &path,
        Some(&tl_module_name),
        &mut ExecutionContext {
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            output_ir: true,
            // output_sim_ir: true,
            // output_schedule: true,
            // output_ir: false,
            output_sim_ir: false,
            output_schedule: false,
        },
    )?;

    Ok(())
}
