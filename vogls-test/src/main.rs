use std::io::Write;
use std::path::Path;

use clap::Parser;
use vogls::ExecutionContext;

/// Simple program to greet a person
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    filter: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        } else if file_type.is_file() {
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

    let paths = match args.filter {
        None => paths,
        Some(f) => paths
            .into_iter()
            .filter(|p| p.to_str().unwrap().contains(&f))
            .collect(),
    };

    for path in paths.iter() {
        let path = path.as_path();
        print!(
            "  {}{:.<2$} ",
            path.display(),
            "",
            max_size - path.as_os_str().len()
        );
        std::io::stdout().flush()?;

        let stdout = Vec::new();
        let stderr = Vec::new();

        let mut ctx = ExecutionContext {
            stdout: Box::new(stdout) as Box<dyn std::io::Write>,
            stderr: Box::new(stderr) as Box<dyn std::io::Write>,
        };
        let result = vogls::run(&tests_dir.join(&path), None, &mut ctx);

        match result {
            Ok(_) => print!("OK"),
            Err(_) => print!("ERR"),
        }
        println!();
    }

    Ok(())
}
