use std::fs;
use std::path::{Path, PathBuf};

fn network_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn network_domain_has_no_orchestrator_dependencies() {
    let cargo_toml_path = network_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    let forbidden_orchestrators = ["cynic-node", "cynic-host", "cynic-cli", "cynic-kernel"];
    for forbidden in forbidden_orchestrators {
        assert!(
            !content.contains(&format!("\"{}\"", forbidden)),
            "cynic-network violates downward DAG: depends on orchestrator {}",
            forbidden
        );
    }
}

#[test]
fn network_domain_has_no_uncontrolled_async_runtimes() {
    let cargo_toml_path = network_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    assert!(
        !content.contains("tokio"),
        "cynic-network must not depend on tokio async runtime directly"
    );
}

#[test]
fn network_functional_core_is_strictly_zero_io() {
    // Functional Core / Imperative Shell (FCIS) invariant:
    // Core domain modules must NEVER execute direct raw I/O or HTTP calls.
    // All side-effects stay strictly quarantined in `src/adapters/`.
    let core_files = [
        "src/types.rs",
        "src/probe.rs",
        "src/discovery.rs",
        "src/firewall_policy.rs",
        "src/mesh_plan.rs",
        "src/dns.rs",
        "src/actuator.rs",
        "src/freebox.rs",
        "src/port.rs",
        "src/registry_maintenance.rs",
        "src/telemetry.rs",
        "src/transport_readback.rs",
        "src/transport_security.rs",
        "src/watchdog_alerter.rs",
    ];

    for rel_path in &core_files {
        let path = network_crate_root().join(rel_path);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", path, e));

        assert!(
            !content.contains("std::process::Command"),
            "FCIS Core Violation: {} contains direct std::process::Command!",
            rel_path
        );
        assert!(
            !content.contains("reqwest"),
            "FCIS Core Violation: {} contains direct reqwest HTTP dependency!",
            rel_path
        );
        assert!(
            !content.contains("tokio::spawn"),
            "FCIS Core Violation: {} spawns async runtimes inside core!",
            rel_path
        );
    }
}

#[test]
fn network_adapters_are_isolated_imperative_shell() {
    let adapters_dir = network_crate_root().join("src/adapters");
    assert!(
        adapters_dir.exists(),
        "src/adapters must exist as the imperative shell"
    );
    assert!(
        adapters_dir.join("headscale.rs").exists(),
        "headscale adapter must be in adapters/"
    );
    assert!(
        adapters_dir.join("finance.rs").exists(),
        "finance adapter must be in adapters/"
    );
    assert!(
        adapters_dir.join("host_probe.rs").exists(),
        "host_probe adapter must be in adapters/"
    );
    assert!(
        adapters_dir.join("freebox.rs").exists(),
        "freebox adapter must be in adapters/"
    );
}
