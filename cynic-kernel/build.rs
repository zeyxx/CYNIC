fn git_stdout(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

const RECOMMENDED_RUST_MIN_STACK: u64 = 67_108_864;

fn has_recommended_debuginfo(rustflags: &str) -> bool {
    rustflags.contains("-C debuginfo=1")
        || rustflags.contains("-Cdebuginfo=1")
        || rustflags.contains("debuginfo=1")
}

fn effective_rustflags() -> String {
    let mut flags = Vec::new();

    if let Ok(rustflags) = std::env::var("RUSTFLAGS")
        && !rustflags.trim().is_empty()
    {
        flags.push(rustflags);
    }

    if let Ok(encoded) = std::env::var("CARGO_ENCODED_RUSTFLAGS") {
        for flag in encoded.split('\x1f').filter(|flag| !flag.trim().is_empty()) {
            flags.push(flag.to_string());
        }
    }

    flags.join(" ")
}

fn parse_stack_bytes(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Keep rebuilds targeted: rerun when Git state affecting `describe --dirty`
    // changes, not on every unrelated filesystem event.
    for git_path in [
        git_stdout(&["rev-parse", "--git-path", "HEAD"]),
        git_stdout(&["rev-parse", "--git-path", "index"]),
        git_stdout(&["rev-parse", "--git-path", "packed-refs"]),
    ]
    .into_iter()
    .flatten()
    {
        println!("cargo:rerun-if-changed={git_path}");
    }

    if let Some(head_ref) = git_stdout(&["symbolic-ref", "-q", "HEAD"])
        && let Some(ref_path) = git_stdout(&["rev-parse", "--git-path", &head_ref])
    {
        println!("cargo:rerun-if-changed={ref_path}");
    }

    println!("cargo:rerun-if-env-changed=RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS");
    println!("cargo:rerun-if-env-changed=RUST_MIN_STACK");

    let profile = std::env::var("PROFILE").unwrap_or_default();
    let rustflags = effective_rustflags();
    let rust_min_stack = std::env::var("RUST_MIN_STACK").unwrap_or_default();

    if profile == "debug" && !has_recommended_debuginfo(&rustflags) {
        println!(
            "cargo:warning=debug builds should use RUSTFLAGS='-C debuginfo=1' to avoid rmcp DWARF overflow; .cargo/config.toml provides this default"
        );
    }

    match parse_stack_bytes(&rust_min_stack) {
        Some(bytes) if bytes >= RECOMMENDED_RUST_MIN_STACK => {}
        Some(bytes) => println!(
            "cargo:warning=RUST_MIN_STACK={bytes} is below the recommended {RECOMMENDED_RUST_MIN_STACK} bytes; 16 MiB can pass intermittently but is not the robust repo default for rmcp-heavy builds"
        ),
        None => println!(
            "cargo:warning=RUST_MIN_STACK is unset; use {RECOMMENDED_RUST_MIN_STACK} bytes as the robust default for rmcp-heavy builds"
        ),
    }

    // Inject git describe into the binary — traceable builds without manual versioning.
    // Fallback to Cargo.toml version if git is unavailable (container builds).
    let git_version = git_stdout(&["describe", "--tags", "--always", "--dirty"])
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=CYNIC_VERSION={git_version}");
    Ok(())
}
