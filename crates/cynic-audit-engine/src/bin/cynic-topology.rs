//! `cynic-topology` — regenerate the derived half of `infra/crate-topology.json`.
//!
//! WHY THIS BINARY EXISTS. The topology document is part policy and part
//! measurement, and the measurement half was maintained by hand until it drifted
//! from the manifests it described. The policy half cannot be generated — which
//! layer a crate belongs to is a decision — so the document is rendered from the
//! manifests plus the policy keys, and the two are checked against each other
//! before anything is written.
//!
//! WHY A SEPARATE BINARY. The audit surface answers "is the workspace coherent"
//! and must never write. This one answers "what should the document say" and must
//! be able to write. A `--write` flag on the audit surface would make the reader
//! of the topology document capable of mutating it, which is the same mistake the
//! repository has already paid for elsewhere: a reader that can also write cannot
//! be trusted to only read.
//!
//! OUTPUT. Machine-readable JSON on stdout, always, so a CI step reads the same
//! artefact a human does. Exit 0 when the document is up to date or written, 1
//! when `--check` found it stale, 2 on error.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::json;

use cynic_audit_engine::topology_generate;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("cynic-topology: {error}");
            ExitCode::from(2)
        }
    }
}

struct Options {
    root: PathBuf,
    write: bool,
}

fn run() -> Result<ExitCode, String> {
    let options = parse_args()?;
    let root = workspace_root(&options.root)?;
    let path = root.join(topology_generate::TOPOLOGY_DOC);

    if options.write {
        let generated = topology_generate::render(&root).map_err(|e| e.to_string())?;
        let before = topology_generate::committed(&root).map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, &generated.text).map_err(|e| e.to_string())?;
        let changed = before != generated.text;
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema": "cynic.topology.report.v1",
                "tool": topology_generate::GENERATOR_NAME,
                "mode": "write",
                "status": if changed { "written" } else { "unchanged" },
                "path": topology_generate::TOPOLOGY_DOC,
                "changed": changed,
                "diff": if changed {
                    topology_generate::diff_paths(&before, &generated.text)
                        .unwrap_or_else(|_| vec!["unreadable".to_string()])
                } else {
                    Vec::<String>::new()
                },
            }))
            .map_err(|e| e.to_string())?
        );
        return Ok(ExitCode::SUCCESS);
    }

    let rendered = topology_generate::render_text(&root).map_err(|e| e.to_string())?;
    let committed = topology_generate::committed(&root).map_err(|e| e.to_string())?;
    let up_to_date = rendered == committed;
    let diff = if up_to_date {
        Vec::new()
    } else {
        topology_generate::diff_paths(&committed, &rendered).unwrap_or_default()
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema": "cynic.topology.report.v1",
            "tool": topology_generate::GENERATOR_NAME,
            "mode": "check",
            "status": if up_to_date { "up_to_date" } else { "stale" },
            "path": topology_generate::TOPOLOGY_DOC,
            "differences": diff.len(),
            "diff": diff,
            // The package is read, not written: this binary moved out of cynic-host, and a
            // hardcoded `-p cynic-host` left behind an instruction nobody can paste.
            "remediation": format!(
                "cargo run -q -p {} --bin cynic-topology -- --write",
                env!("CARGO_PKG_NAME")
            ),
        }))
        .map_err(|e| e.to_string())?
    );
    if up_to_date {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options {
        root: PathBuf::new(),
        write: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--write" => options.write = true,
            "--check" => options.write = false,
            "--root" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--root requires a path".to_string())?;
                options.root = PathBuf::from(value);
            }
            other => {
                return Err(format!(
                    "unknown argument `{other}`. Usage: cynic-topology [--check|--write] [--root PATH]"
                ));
            }
        }
    }
    Ok(options)
}

fn workspace_root(explicit: &PathBuf) -> Result<PathBuf, String> {
    if !explicit.as_os_str().is_empty() {
        if explicit.join("crates").is_dir() && explicit.join("Cargo.toml").is_file() {
            return Ok(explicit.clone());
        }
        return Err(format!(
            "{} holds no Cargo.toml and crates/",
            explicit.display()
        ));
    }
    if let Ok(value) = std::env::var("CYNIC_WORKSPACE_ROOT") {
        let path = PathBuf::from(value);
        if path.join("crates").is_dir() && path.join("Cargo.toml").is_file() {
            return Ok(path);
        }
        return Err(format!(
            "CYNIC_WORKSPACE_ROOT={} holds no Cargo.toml and crates/",
            path.display()
        ));
    }
    // CARGO_MANIFEST_DIR is .../crates/cynic-host when cargo runs this, and the
    // working directory is the workspace when it does not. Both are checked, and
    // neither is assumed.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors().skip(1) {
        if ancestor.join("Cargo.toml").is_file() && ancestor.join("crates").is_dir() {
            return Ok(ancestor.to_path_buf());
        }
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    if cwd.join("Cargo.toml").is_file() && cwd.join("crates").is_dir() {
        return Ok(cwd);
    }
    Err(format!(
        "no workspace root: neither {} nor the current directory holds Cargo.toml and crates/",
        manifest.display()
    ))
}
