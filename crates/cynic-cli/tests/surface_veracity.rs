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
