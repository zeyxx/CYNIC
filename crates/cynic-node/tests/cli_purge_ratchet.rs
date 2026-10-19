//! Purge ratchet for the `cynic-node` binary CLI surface.
//!
//! History: the daemon used to own 16 hyphenated top-level arms
//! (`cynic-node agent-workspace`, `registry-observation`,
//! `storage-reconcile`, ...). Commits 8bcc4b2e and 035a93a4 moved those
//! surfaces out of the daemon into cynic-cli — nested under
//! `cynic node ...`, never hyphenated — and left the daemon owning
//! exactly one subcommand: `control`.
//!
//! This ratchet locks that purge in, at two levels:
//!   * static: no hyphenated dispatch arm may re-enter `main.rs`,
//!     and the old `cli_dispatch` module must stay deleted;
//!   * argv: the built binary must reject every historical hyphenated
//!     arm with the migration message pointing operators at `cynic`.

#![allow(clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The 16 hyphenated top-level arms removed by 8bcc4b2e / 035a93a4.
const PURGED_HYPHENATED_ARMS: [&str; 16] = [
    "node-identity",
    "consensus-status",
    "observe",
    "registry-observation",
    "network",
    "peer",
    "agent-workspace",
    "agent-workspace-lifecycle",
    "agent-scheduler-step",
    "agent-workload-status",
    "agent-workload-transition",
    "build-workload-grant-verify",
    "storage-reconcile",
    "source-admission-plan",
    "source-materialize",
    "source-bootstrap",
];

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn purge_ratchet_main_rs_dispatches_only_control() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");

    assert!(
        main_rs.contains("\"control\""),
        "main.rs must keep dispatching the daemon `control` arm"
    );

    for arm in PURGED_HYPHENATED_ARMS {
        assert!(
            !main_rs.contains(&format!("\"{arm}\"")),
            "main.rs must not re-introduce hyphenated arm `{arm}` \
             (CLI surfaces belong in cynic-cli, nested under `cynic node ...`)"
        );
    }

    assert!(
        !main_rs.contains("cli_dispatch"),
        "main.rs must not reference the deleted cli_dispatch module"
    );
}

#[test]
fn purge_ratchet_cli_dispatch_stays_deleted() {
    let root = crate_root();
    assert!(
        !root.join("src/cli_dispatch.rs").exists(),
        "cli_dispatch.rs must not be reintroduced; the daemon owns only `control`"
    );

    let lib_rs = fs::read_to_string(root.join("src/lib.rs")).expect("read lib.rs");
    assert!(
        !lib_rs.contains("cli_dispatch"),
        "lib.rs must not declare a cli_dispatch module"
    );
}

#[test]
fn purge_ratchet_binary_rejects_hyphenated_arms() {
    for arm in PURGED_HYPHENATED_ARMS {
        let output = Command::new(env!("CARGO_BIN_EXE_cynic-node"))
            .arg(arm)
            .arg("--json")
            .output()
            .unwrap_or_else(|e| panic!("spawn cynic-node for arm `{arm}`: {e}"));

        assert!(
            !output.status.success(),
            "hyphenated arm `{arm}` must be rejected by the daemon binary"
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("cynic"),
            "rejection of `{arm}` must point operators at `cynic <command>`, got: {stderr}"
        );
    }
}
