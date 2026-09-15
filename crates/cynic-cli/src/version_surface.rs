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
        "build_proof_sha": env!("CYNIC_BUILD_PROOF_SHA"),
    })
    .to_string()
}

/// Execute `cynic version`: JSON report or one human line.
pub fn execute_version(json: bool) {
    if json {
        println!("{}", version_json());
    } else {
        let proof = env!("CYNIC_BUILD_PROOF_SHA");
        let proof_tag = if proof.is_empty() {
            "untracked".to_string()
        } else {
            format!("proof={}", proof)
        };
        println!(
            "cynic {} {} build_profile={} {}",
            env!("CARGO_PKG_VERSION"),
            env!("CYNIC_GIT_SHA"),
            env!("CYNIC_BUILD_PROFILE"),
            proof_tag
        );
    }
}
