fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inject git describe into the binary — traceable builds without manual versioning.
    // Fallback to Cargo.toml version if git is unavailable (container builds).
    // NOT watching .git/ — version updates at deploy time (cargo build --release),
    // not at every commit. 43s rebuilds during dev for a version string is waste.
    let git_version = std::process::Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=CYNIC_VERSION={git_version}");
    Ok(())
}
