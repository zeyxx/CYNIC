//! Schema coverage contract (S1): the runtime/storage divergence gate.
//!
//! The 2026-08-24 audit found 8 tables the code reads/writes that were
//! declared in NO schema file (schemaless-by-accident), plus duplicate
//! declarations. This module is the permanent regression proof:
//!
//! 1. `manifest_covers_every_runtime_table` — every table touched by the
//!    Surreal adapters must exist in `schema_manifest.json`.
//! 2. `sql_declarations_match_manifest` — every table declared in any
//!    `.sql` file must be in the manifest, with no duplicates across files.
//! 3. `declared_tables_have_consumers` — no orphan declarations.
//!
//! The manifest is data, not prose: it is consumed here and intended to feed
//! the migrator's required-tables check (single source of truth).

use std::collections::BTreeMap;
use std::path::Path;

const MANIFEST_PATH: &str = "src/storage/schema_manifest.json";
const SQL_DIR: &str = "src/storage";
const ADAPTER_DIRS: &[&str] = &["src/storage/surreal", "src/storage/memory"];
const ADAPTER_FILES: &[&str] = &["src/storage/memory.rs", "src/storage/surreal.rs"];

#[derive(Debug)]
struct ManifestTable {
    name: String,
    mode: String,
}

fn workspace_root() -> PathBuf {
    // Tests run from the crate directory; walk up to find Cargo.toml of the kernel crate.
    for ancestor in std::env::current_dir().unwrap_or_default().ancestors() {
        if ancestor.join(MANIFEST_PATH).is_file() {
            return ancestor.to_path_buf();
        }
    }
    panic!(
        "schema manifest not found relative to {:?}",
        std::env::current_dir()
    );
}

use std::path::PathBuf;

fn load_manifest() -> BTreeMap<String, ManifestTable> {
    let path = workspace_root().join(MANIFEST_PATH);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("manifest unreadable at {path:?}: {e}"));
    let value: serde_json::Value = serde_json::from_str(&raw).expect("manifest is valid JSON");
    assert_eq!(
        value.get("schema").and_then(|s| s.as_str()),
        Some("cynic.storage.schema_manifest.v1"),
        "manifest schema label drifted"
    );
    let mut out = BTreeMap::new();
    for table in value["tables"].as_array().expect("tables array") {
        let name = table["name"].as_str().expect("table name").to_string();
        let mode = table["mode"].as_str().expect("table mode").to_string();
        assert!(
            mode == "schemafull" || mode == "schemaless",
            "table {name}: unknown mode {mode}"
        );
        out.insert(name, ManifestTable { name, mode });
    }
    out
}

/// Tables referenced by adapter code via SurrealQL verbs (FROM/INTO/UPDATE/
/// UPSERT ONLY/CREATE ONLY/CREATE/UPSERT).
fn collect_runtime_tables(root: &Path) -> Vec<String> {
    let mut used = std::collections::BTreeSet::new();
    let verb_re = regex::Regex::new(
        r#"(?:FROM|INTO|UPDATE|UPSERT ONLY|CREATE ONLY|CREATE|UPSERT) ([a-z_]+)"#,
    )
    .unwrap();
    for dir in ADAPTER_DIRS {
        for entry in walkdir_entries(&root.join(dir)) {
            let src = std::fs::read_to_string(&entry).unwrap_or_default();
            for cap in verb_re.captures_iter(&src) {
                used.insert(cap[1].to_string());
            }
        }
    }
    for file in ADAPTER_FILES {
        let path = root.join(file);
        if path.is_file() {
            let src = std::fs::read_to_string(&path).unwrap_or_default();
            for cap in verb_re.captures_iter(&src) {
                used.insert(cap[1].to_string());
            }
        }
    }
    // Filter obvious non-table words that share verbs in comments/format strings.
    let stop: std::collections::BTreeSet<&str> =
        ["on", "only", "verdict_id", "now"].into_iter().collect();
    used.into_iter()
        .filter(|t| !stop.contains(t.as_str()))
        .collect()
}

fn walkdir_entries(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walkdir_entries(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}

/// Tables declared in .sql files next to their declaration sites.
fn collect_sql_declarations(root: &Path) -> BTreeMap<String, Vec<String>> {
    let re = regex::Regex::new(r"DEFINE TABLE (?:IF NOT EXISTS |OVERWRITE )?([a-z_]+)").unwrap();
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in std::fs::read_dir(root.join(SQL_DIR)).expect("storage dir readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("sql") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        for cap in re.captures_iter(&src) {
            let name = cap[1].to_string();
            out.entry(name)
                .or_default()
                .push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    out
}

#[test]
fn manifest_covers_every_runtime_table() {
    let manifest = load_manifest();
    let used = collect_runtime_tables(&workspace_root());
    let missing: Vec<_> = used.iter().filter(|t| !manifest.contains_key(*t)).collect();
    assert!(
        missing.is_empty(),
        "runtime touches tables absent from schema manifest (schemaless-by-accident): {missing:?}"
    );
}

#[test]
fn sql_declarations_are_unique_and_covered_by_manifest() {
    let manifest = load_manifest();
    let sql = collect_sql_declarations(&workspace_root());
    let dupes: Vec<_> = sql
        .iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(name, files)| format!("{name}: {files:?}"))
        .collect();
    assert!(
        dupes.is_empty(),
        "duplicate DEFINE TABLE across sql files (bootstrap vs migration drift): {dupes:?}"
    );
    let undeclared: Vec<_> = sql
        .keys()
        .filter(|name| !manifest.contains_key(*name))
        .collect();
    assert!(
        undeclared.is_empty(),
        ".sql declares tables missing from manifest: {undeclared:?}"
    );
}

#[test]
fn declared_tables_have_runtime_consumers() {
    let manifest = load_manifest();
    let used = collect_runtime_tables(&workspace_root());
    let orphans: Vec<_> = manifest
        .keys()
        .filter(|name| !used.contains(*name))
        .collect();
    assert!(
        orphans.is_empty(),
        "manifest declares tables no adapter code touches (dead declarations): {orphans:?}"
    );
}
