use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");

    let date = Command::new("date")
        .args(["+%y.%m.%d"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "00.00.00".to_string());

    let git_sha = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let profile = env::var("PROFILE").unwrap_or_else(|_| "dev".to_string());

    let short_sha = git_sha.chars().take(7).collect::<String>();
    let version = format!("{date}-{short_sha}");

    let build_admission_verified = env_truthy("CYNIC_BUILD_ADMISSION_VERIFIED");

    if !build_admission_verified {
        panic!(
            "BUILD BLOCKED: cynic-kernel compilation requires a grant verified by the canonical Cargo wrapper."
        );
    }

    println!("cargo:rustc-env=CYNIC_VERSION={version}");
    println!("cargo:rustc-env=CYNIC_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=CYNIC_BUILD_PROFILE={profile}");
}

fn env_truthy(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}
