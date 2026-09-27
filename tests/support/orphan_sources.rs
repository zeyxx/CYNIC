//! Shared orphan-source detection for crate architecture-boundary tests.
//!
//! An undeclared `.rs` file is never compiled: no build error, no test run, no
//! warning. It is *invisible* rather than broken, so `cargo check` cannot see
//! it. Bulk integration commits have repeatedly dropped files into crates
//! without adding the matching declaration, leaving contract code that never
//! executed once.
//!
//! Reachability in this workspace uses THREE mechanisms. A detector that knows
//! only the first reports live, load-bearing files as dead:
//!
//! 1. `mod foo;`                  -> `foo.rs` or `foo/mod.rs`
//! 2. `#[path = "x.rs"] mod foo;` -> explicit path
//! 3. `include!("x.rs")`          -> textual inclusion
//!
//! `cynic-mcp` alone has 78 `include!` sites, and its `handler.rs` (2221 lines
//! implementing `ServerHandler`) is reached only that way. Removing it breaks
//! the build, which is how the omission was caught.
//!
//! Each consuming crate keeps its own quarantine list and asserts the list is
//! exact: it may not grow (new orphans are rejected) and it may not go stale
//! (a repaired file must leave the list). The debt is bounded, not tolerated.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Locate the workspace root by walking up for the authority files.
///
/// Never hardcodes an operator machine path: the same test must pass in any
/// checkout, on any host, under any CI harness.
pub fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file()
            && dir.join("crates").is_dir()
            && dir.join("contracts/foundation.json").is_file()
        {
            return dir;
        }
        if !dir.pop() {
            panic!("workspace root not found: no ancestor holds Cargo.toml + crates/ + contracts/");
        }
    }
}

/// Every workspace crate name, from the `crates/` directory.
pub fn crate_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(workspace_root().join("crates"))
        .expect("read crates/")
        .flatten()
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .collect();
    names.sort();
    names
}

/// Per-crate size census, measured with the SAME reachability rules the orphan
/// ratchet uses, so the two can never disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrateLocCensus {
    /// LOC in every `.rs` under `src/`.
    pub on_disk: usize,
    /// LOC in the files a compilation root can actually name.
    pub compiled: usize,
}

impl CrateLocCensus {
    /// LOC that exists on disk but that no compilation root names.
    ///
    /// This is the legacy-debt axis: mass the compiler never sees, that no
    /// warning reports, and that the god-object ceiling alone does not catch.
    pub fn gap(&self) -> usize {
        self.on_disk.saturating_sub(self.compiled)
    }
}

/// Measure a crate. Panics when the crate has no `src/`.
pub fn crate_loc_census(crate_name: &str) -> CrateLocCensus {
    let crate_dir = workspace_root().join("crates").join(crate_name);
    let src_root = crate_dir.join("src");
    assert!(
        src_root.is_dir(),
        "{crate_name}: no src/ directory at {}",
        src_root.display()
    );
    let lines = |rel: &str| -> usize {
        fs::read_to_string(src_root.join(rel))
            .map(|text| text.lines().count())
            .unwrap_or(0)
    };
    let on_disk: usize = all_sources(&src_root).iter().map(|rel| lines(rel)).sum();
    // `reachable` includes lib.rs and main.rs, which are compilation roots in
    // their own right. Excluding them here would count the crate's own roots as
    // dead mass and put a floor under every gap.
    let compiled: usize = reachable(&crate_dir, &src_root)
        .iter()
        .map(|rel| lines(rel))
        .sum();
    CrateLocCensus { on_disk, compiled }
}

/// Every `.rs` file under `src_root`, relative to it, sorted.
fn all_sources(src_root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut stack = vec![src_root.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(rel) = path.strip_prefix(src_root) {
                    found.insert(rel.display().to_string());
                }
            }
        }
    }
    found
}

/// Names declared by `mod name;` lines (not inline `mod name { .. }` blocks).
fn declared_mod_names(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| {
            // Skip commented-out declarations.
            if line.starts_with("//") {
                return None;
            }
            let rest = line
                .strip_prefix("pub mod ")
                .or_else(|| line.strip_prefix("pub(crate) mod "))
                .or_else(|| line.strip_prefix("pub(super) mod "))
                .or_else(|| line.strip_prefix("mod "))?;
            rest.strip_suffix(';').map(|name| name.trim().to_string())
        })
        .collect()
}

/// Paths pulled in by `include!("...")` or `#[path = "..."]`, as written.
fn referenced_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (marker, open) in [("include!", '('), ("#[path", '=')] {
        let mut rest = text;
        while let Some(idx) = rest.find(marker) {
            rest = &rest[idx + marker.len()..];
            let Some(open_idx) = rest.find(open) else {
                continue;
            };
            let after = &rest[open_idx..];
            let Some(start) = after.find('"') else {
                continue;
            };
            let tail = &after[start + 1..];
            if let Some(end) = tail.find('"') {
                out.push(tail[..end].to_string());
            }
        }
    }
    out
}

/// Extra crate roots declared in `Cargo.toml` (`path = "src/..."`).
///
/// A `[[bin]]` / `[lib]` target is a compilation root just like `lib.rs`, and
/// `src/bin/*.rs` is an implicit binary root under Cargo's autodiscovery.
/// Missing these reports load-bearing binaries as dead — `cynic-zeroclaw-solana`
/// has `path = "src/bin/demo.rs"`, and `cynic-node` declares
/// `src/actuator_main.rs`.
fn manifest_roots(crate_dir: &Path, src_root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(manifest) = fs::read_to_string(crate_dir.join("Cargo.toml")) {
        for line in manifest.lines().map(str::trim) {
            let Some(rest) = line.strip_prefix("path") else {
                continue;
            };
            let Some(eq) = rest.find('=') else { continue };
            let value = rest[eq + 1..].trim().trim_matches('"');
            if value.ends_with(".rs") {
                let candidate = crate_dir.join(value);
                if candidate.is_file() {
                    roots.push(candidate);
                }
            }
        }
    }

    // Cargo autodiscovers `src/bin/*.rs` as binaries without any manifest entry.
    let bin_dir = src_root.join("bin");
    if let Ok(entries) = fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                roots.push(path);
            } else if path.is_dir() && path.join("main.rs").is_file() {
                roots.push(path.join("main.rs"));
            }
        }
    }

    roots
}

/// Transitively collect sources reachable from the crate roots.
fn reachable(crate_dir: &Path, src_root: &Path) -> BTreeSet<String> {
    let mut reached: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<PathBuf> = Vec::new();

    for root in ["lib.rs", "main.rs"] {
        let entry = src_root.join(root);
        if entry.is_file() {
            stack.push(entry);
        }
    }
    stack.extend(manifest_roots(crate_dir, src_root));

    while let Some(path) = stack.pop() {
        let Ok(rel) = path.strip_prefix(src_root) else {
            continue;
        };
        let rel = rel.display().to_string();
        if !reached.insert(rel) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let dir = path.parent().unwrap_or(src_root).to_path_buf();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();

        // `mod foo;` in lib.rs/main.rs/mod.rs resolves beside the file;
        // in `bar.rs` it resolves under `bar/`.
        let mod_dir = if matches!(stem.as_str(), "lib" | "main" | "mod") {
            dir.clone()
        } else {
            dir.join(&stem)
        };

        for name in declared_mod_names(&text) {
            for base in [&mod_dir, &dir] {
                let file = base.join(format!("{name}.rs"));
                let dir_mod = base.join(&name).join("mod.rs");
                if file.is_file() {
                    stack.push(file);
                    break;
                }
                if dir_mod.is_file() {
                    stack.push(dir_mod);
                    break;
                }
            }
        }

        // `include!` / `#[path]` are relative to the including file's directory.
        for rel_path in referenced_paths(&text) {
            let candidate = dir.join(&rel_path);
            if candidate.is_file() {
                stack.push(candidate);
            }
        }
    }

    reached
}

/// Assert the crate's orphan set exactly equals `quarantined`.
///
/// Fails in BOTH directions on purpose:
/// - a new orphan means someone added a file nothing compiles;
/// - a stale entry means a file was repaired or deleted but left listed, which
///   would let the quarantine become permanent cover instead of shrinking.
pub fn assert_orphan_sources_match(crate_name: &str, quarantined: &[&str]) {
    let crate_dir = workspace_root().join("crates").join(crate_name);
    let src_root = crate_dir.join("src");
    assert!(
        src_root.is_dir(),
        "{crate_name}: no src/ directory at {}",
        src_root.display()
    );

    let all = all_sources(&src_root);
    let reached = reachable(&crate_dir, &src_root);
    let orphans: BTreeSet<String> = all.difference(&reached).cloned().collect();
    let expected: BTreeSet<String> = quarantined.iter().map(|s| (*s).to_string()).collect();

    let new: Vec<&String> = orphans.difference(&expected).collect();
    let stale: Vec<&String> = expected.difference(&orphans).collect();

    assert!(
        new.is_empty(),
        "{crate_name}: {} source(s) unreachable from lib.rs/main.rs and not \
         quarantined.\n\nUndeclared files are never compiled — no error, no \
         test, no warning. Declare them (`mod`/`include!`), delete them, or add \
         them to the crate's quarantine list with the reason.\n\nnew orphans: \
         {:#?}",
        new.len(),
        new
    );

    assert!(
        stale.is_empty(),
        "{crate_name}: {} quarantined path(s) are no longer orphans — they are \
         now reachable or gone.\n\nRemove them from the list so the quarantine \
         keeps shrinking instead of becoming permanent cover.\n\nstale entries: \
         {:#?}",
        stale.len(),
        stale
    );
}
