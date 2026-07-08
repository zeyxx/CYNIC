use std::process::Command;

fn main() {
    // 1. Get the current Git commit hash
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string());

    // 2. Get the current branch
    let git_branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string());

    // 3. Get the build timestamp (UTC)
    let build_date = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string());

    // Inject these values as environment variables during compilation
    println!("cargo:rustc-env=CYNIC_BUILD_GIT_HASH={}", git_hash.trim());
    println!(
        "cargo:rustc-env=CYNIC_BUILD_GIT_BRANCH={}",
        git_branch.trim()
    );
    println!("cargo:rustc-env=CYNIC_BUILD_DATE={}", build_date.trim());

    // Tell cargo to rerun this script only if .git/HEAD changes
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=build.rs");
}
