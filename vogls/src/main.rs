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

    vogls::run(&path, &tl_module_name, &mut ExecutionContext {
        stdout: Box::new(std::io::stdout()),
        stderr: Box::new(std::io::stderr()),
    })?;

    Ok(())
}
