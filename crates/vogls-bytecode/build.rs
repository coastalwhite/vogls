use std::process::Command;

// We conditionally use the tailcall feature feature which is only available on nightly and is
// compiler internal. Therefore, we have to do some hackery here.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(nightly)");
    println!("cargo::rerun-if-changed=build.rs");

    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let Ok(output) = Command::new(rustc).args(["--version", "--verbose"]).output() else {
        return;
    };

    let version = String::from_utf8_lossy(&output.stdout);
    let is_nightly = version
        .lines()
        .find_map(|line| line.strip_prefix("release:"))
        .map(|release| release.contains("nightly") || release.contains("dev"))
        .unwrap_or(false);

    if is_nightly {
        println!("cargo::rustc-cfg=nightly");
    }
}
