//! Feature-gated topology SSOT writer for `cynic audit topology`.
//!
//! WHY THIS EXISTS. The default canonical CLI is a reader of the workspace
//! topology. The SSOT generator must be able to write it. Those two roles are
//! not the same trust boundary, so the writer is compiled in only when the
//! `topology_write` feature is enabled.
//!
//! HOW IT WORKS. The writer calls the existing pure renderer in
//! `cynic-audit-engine::topology_generate`, compares the rendered text with the
//! committed document, and writes only when `--apply` is supplied. No manual
//! retyping of the derived half is possible through this path.
//!
//! NEGATIVE BOUNDARY. It never guesses policy. Layers, tolerated debt, and
//! dispositions remain authored in `infra/crate-topology.json`; this module only
//! replaces the derived keys the generator owns.

use std::process::ExitCode;

use serde_json::json;

use crate::emit_json;

pub(crate) fn execute(json: bool, apply: bool) -> ExitCode {
    let root = crate::rust_source::workspace_root();
    let path = root.join(cynic_audit_engine::topology_generate::TOPOLOGY_DOC);

    if !apply {
        let payload = json!({
            "schema": "cynic.cli.audit_topology_write.v1",
            "status": "blocked",
            "reason": "topology writer requires explicit --apply",
        });
        if json {
            return emit_json(&payload, true);
        }
        println!("Status: BLOCKED (topology writer requires explicit --apply)");
        return ExitCode::FAILURE;
    }

    let rendered = match cynic_audit_engine::topology_generate::render(&root) {
        Ok(rendered) => rendered,
        Err(error) => {
            let payload = json!({
                "schema": "cynic.cli.audit_topology_write.v1",
                "status": "blocked",
                "reason": format!("topology render failed: {error}"),
            });
            if json {
                return emit_json(&payload, true);
            }
            println!("Status: BLOCKED (topology render failed: {error})");
            return ExitCode::FAILURE;
        }
    };

    let committed = cynic_audit_engine::topology_generate::committed(&root).ok();
    let changed = committed.as_deref() != Some(rendered.text.as_str());
    let diff = match committed.as_deref() {
        Some(committed) => {
            cynic_audit_engine::topology_generate::diff_paths(committed, &rendered.text)
                .unwrap_or_default()
        }
        None => Vec::new(),
    };

    if let Some(parent) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            let payload = json!({
                "schema": "cynic.cli.audit_topology_write.v1",
                "status": "blocked",
                "reason": format!("could not create parent directory: {error}"),
            });
            if json {
                return emit_json(&payload, true);
            }
            println!("Status: BLOCKED (could not create parent directory: {error})");
            return ExitCode::FAILURE;
        }
    }

    if let Err(error) = std::fs::write(&path, &rendered.text) {
        let payload = json!({
            "schema": "cynic.cli.audit_topology_write.v1",
            "status": "blocked",
            "reason": format!("topology write failed: {error}"),
        });
        if json {
            return emit_json(&payload, true);
        }
        println!("Status: BLOCKED (topology write failed: {error})");
        return ExitCode::FAILURE;
    }

    let payload = json!({
        "schema": "cynic.cli.audit_topology_write.v1",
        "status": if changed { "written" } else { "unchanged" },
        "changed": changed,
        "path": cynic_audit_engine::topology_generate::TOPOLOGY_DOC,
        "diff": diff,
    });
    if json {
        return emit_json(&payload, false);
    }

    println!(
        "Status: {} ({} differences)",
        if changed { "WRITTEN" } else { "UNCHANGED" },
        diff.len()
    );
    for line in diff.iter().take(10) {
        println!("  {line}");
    }
    ExitCode::SUCCESS
}
