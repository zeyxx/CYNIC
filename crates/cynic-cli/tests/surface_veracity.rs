//! CLI surface veracity gate: every key read-only command must emit a
//! non-empty JSON report carrying a `schema` field.
//!
//! WHY: stub or empty commands are cognitive debt for human and LLM agents
//! alike — a command returning nothing must fail loudly here, never silently.
//! HOW: run each argv against the built binary and assert exit code, stdout
//! shape, and the presence of `schema`.

use std::process::Command;

/// A read-only probe: argv plus the exit codes that count as a truthful
/// read-back (`1` is a valid supervision verdict, e.g. ssh-supervision
/// reporting a non-converged state — only crashes like 101 are rejected).
struct Probe {
    argv: &'static [&'static str],
    allowed_exits: &'static [i32],
}

const PROBES: &[Probe] = &[
    Probe {
        argv: &["doctor", "--json"],
        allowed_exits: &[0, 1],
    },
    Probe {
        argv: &["status", "--json"],
        allowed_exits: &[0, 1],
    },
    Probe {
        argv: &["version", "--json"],
        allowed_exits: &[0],
    },
    Probe {
        argv: &["agent", "session-context", "--json"],
        allowed_exits: &[0],
    },
    Probe {
        argv: &["transport", "ssh-supervision", "--json"],
        allowed_exits: &[0, 1],
    },
];

#[test]
fn key_surfaces_emit_schema_json() {
    for probe in PROBES {
        let label = probe.argv.join(" ");
        let output = Command::new(env!("CARGO_BIN_EXE_cynic"))
            .args(probe.argv)
            .output()
            .unwrap_or_else(|e| panic!("cynic {label}: failed to spawn: {e}"));

        let code = output.status.code().unwrap_or(-1);
        assert!(
            probe.allowed_exits.contains(&code),
            "cynic {label}: exit {code} not in {:?}; stderr: {}",
            probe.allowed_exits,
            String::from_utf8_lossy(&output.stderr),
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.trim().is_empty(),
            "cynic {label}: empty stdout (exit {code})",
        );

        let payload: serde_json::Value = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|e| panic!("cynic {label}: stdout is not JSON ({e}): {stdout:?}"));
        assert!(
            payload
                .get("schema")
                .and_then(|v| v.as_str())
                .is_some_and(|s| !s.is_empty()),
            "cynic {label}: JSON report has no schema field: {stdout:?}",
        );
    }
}

#[test]
fn build_invoice_bills_canned_proof_wall_time() {
    let commit = "a".repeat(40);
    let dir = std::env::temp_dir().join(format!(
        "cynic-invoice-veracity-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create invoice fixture dir");
    let log = dir.join("proofs.jsonl");
    let proof = serde_json::json!({
        "schema": "cynic.pipeline_build_proof.v1",
        "node": "runner-1",
        "source_commit": commit,
        "action": "cargo:check",
        "status": "converged",
        "broken_layer": null,
        "duration_ms": 27735,
        "layers": [{"layer": "cynic-protocol", "status": "converged", "blocked_by": null}],
    });
    std::fs::write(&log, format!("{proof}\n")).expect("write canned proof");

    let output = Command::new(env!("CARGO_BIN_EXE_cynic"))
        .args([
            "build",
            "invoice",
            "--commit",
            &commit,
            "--proof-log",
            log.to_str().expect("temp log path"),
            "--json",
        ])
        .output()
        .expect("run cynic build invoice");
    assert!(
        output.status.success(),
        "cynic build invoice failed: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    let payload: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .expect("invoice stdout is JSON");
    assert_eq!(payload["schema"], "cynic.workload.metabolic_invoice.v1");
    assert_eq!(payload["proofs_observed"], 1);
    assert_eq!(payload["proofs_invoiced"], 1);
    assert_eq!(payload["total"]["wall_time_ms"], 27735);
    assert!(payload["residual_unknowns"].as_array().unwrap().is_empty());

    std::fs::remove_dir_all(&dir).ok();
}
