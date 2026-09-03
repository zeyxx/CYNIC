//! Version surface — the binary reports its own source identity.
//!
//! `cynic version` prints the exact git SHA baked at compile time
//! (`CYNIC_GIT_SHA` from build.rs, `unknown` when the build tree had no
//! git metadata). Freshness gates and deployment read-backs consume this
//! instead of guessing which commit a remote binary was built from.

/// Machine-readable version report.
pub fn version_json() -> String {
    serde_json::json!({
        "schema": "cynic.cli.version.v1",
        "git_sha": env!("CYNIC_GIT_SHA"),
        "build_profile": env!("CYNIC_BUILD_PROFILE"),
    })
    .to_string()
}

/// Execute `cynic version`: JSON report or one human line.
pub fn execute_version(json: bool) {
    if json {
        println!("{}", version_json());
    } else {
        println!(
            "cynic {} ({})",
            env!("CYNIC_GIT_SHA"),
            env!("CYNIC_BUILD_PROFILE")
        );
    }
}
