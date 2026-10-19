use std::fs;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Remove the `RETIRED_LAYER_BOUNDARY_STRINGS` declaration span (from the
/// `pub const` line through its closing `];`) so the scanner sees uses,
/// not the retired denylist itself.
fn strip_retired_denylist(content: &str) -> String {
    let mut out = Vec::new();
    let mut skipping = false;
    for line in content.lines() {
        if line.contains("RETIRED_LAYER_BOUNDARY_STRINGS") {
            skipping = true;
        }
        if skipping {
            if line.contains("];") {
                skipping = false;
            }
            continue;
        }
        out.push(line);
    }
    out.join("\n")
}

#[test]
fn audit_engine_has_no_orchestrator_dependencies() {
    let cargo_toml_path = crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    let forbidden_orchestrators = ["cynic-node", "cynic-cli", "cynic-kernel"];
    for forbidden in forbidden_orchestrators {
        assert!(
            !content.contains(&format!("\"{}\"", forbidden)),
            "cynic-audit-engine violates downward DAG: depends on {}",
            forbidden
        );
    }
}

#[test]
fn audit_engine_has_no_uncontrolled_async_runtimes() {
    let cargo_toml_path = crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    assert!(
        !content.contains("tokio"),
        "cynic-audit-engine must not depend on tokio async runtime directly"
    );
}

/// A pattern occurrence is a real I/O usage only when followed by an
/// identifier character (`reqwest::Client`, `tokio::spawn`). A trailing
/// quote or comma means the pattern is merely NAMED — as in
/// RETIRED_LAYER_BOUNDARY_STRINGS, which keeps the retired denylist
/// greppable precisely so nobody reintroduces it blindly.
fn is_usage(content: &str, pattern: &str) -> bool {
    content.match_indices(pattern).any(|(idx, _)| {
        content[idx + pattern.len()..].chars().next().is_some_and(|next| {
            next.is_ascii_alphanumeric() || next == '_' || next == ':' || next == '('
        })
    })
}

#[test]
fn audit_engine_has_no_process_spawn_in_domain_core() {
    let src_dir = crate_root().join("src");
    // Match USAGE, not mention: RETIRED_LAYER_BOUNDARY_STRINGS in rules.rs
    // names "std::process::Command" (no trailing identifier) precisely so
    // the retirement stays greppable. A bare contains() false-positives on
    // that constant; is_usage() requires the pattern to continue into an
    // identifier (Command::new, reqwest::Client, tokio::spawn) and ignores
    // mere mentions.
    // (Checked: no `use std::process::Command` import exists in src/.)
    let forbidden_patterns = ["Command::", "reqwest::", "tokio::spawn"];

    for entry in walkdir::WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        let content = fs::read_to_string(entry.path()).expect("read source file");
        // The retired-denylist constant names these tokens so the retirement
        // stays greppable; it is a declaration, not a use — exclude it.
        let content = strip_retired_denylist(&content);
        for pattern in &forbidden_patterns {
            assert!(
                !is_usage(&content, pattern),
                "cynic-audit-engine domain core {} contains forbidden I/O: {}",
                entry.path().display(),
                pattern
            );
        }
    }
}
