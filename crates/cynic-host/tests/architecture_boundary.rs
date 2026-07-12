use std::fs;
use std::path::Path;

#[test]
fn host_exec_binary_stays_a_thin_adapter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bin = fs::read_to_string(root.join("src/bin/cynic-host-exec.rs"))
        .expect("read cynic-host-exec.rs");

    assert!(
        bin.contains("cynic_host::host_exec_cli::execute(args)"),
        "cynic-host-exec must delegate execution to cynic_host::host_exec_cli"
    );
    assert!(
        !bin.contains("LocalHostObservationAdapter")
            && !bin.contains("LocalHostCommandRunner")
            && !bin.contains("resolve_host_profile"),
        "host-exec orchestration belongs in cynic-host::host_exec_cli, not the bin adapter"
    );
}

#[test]
fn host_exec_cli_surface_stays_decomposed_by_responsibility() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let surface =
        fs::read_to_string(root.join("src/host_exec_cli.rs")).expect("read host_exec_cli.rs");

    for module in [
        "args",
        "bootstrap",
        "bridge_commands",
        "io",
        "network_baseline",
        "profile_selection",
        "validation",
    ] {
        assert!(
            root.join(format!("src/host_exec_cli/{module}.rs"))
                .is_file(),
            "host_exec_cli::{module} must exist as a bounded responsibility module"
        );
        assert!(
            surface.contains(&format!("mod {module};")),
            "host_exec_cli.rs must declare module {module}"
        );
    }

    assert!(
        !surface.contains("LocalHostObservationAdapter")
            && !surface.contains("LocalHostCommandRunner")
            && !surface.contains("observe_host_with_resolution"),
        "host_exec_cli.rs must stay a dispatcher; adapters/usecases belong in submodules"
    );
}

#[test]
fn host_usecases_surface_stays_a_thin_orchestrator() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let surface = fs::read_to_string(root.join("src/usecases/mod.rs")).expect("read usecases");
    let reports =
        fs::read_to_string(root.join("src/usecases/reports.rs")).expect("read usecases/reports.rs");
    let status =
        fs::read_to_string(root.join("src/usecases/status.rs")).expect("read usecases/status.rs");
    let execution = fs::read_to_string(root.join("src/usecases/execution.rs"))
        .expect("read usecases/execution.rs");

    assert!(
        surface.lines().count() <= 220,
        "usecases/mod.rs must stay a thin orchestrator; current line count is {}",
        surface.lines().count()
    );
    for module in ["reports", "status", "execution"] {
        assert!(
            surface.contains(&format!("mod {module};")),
            "usecases/mod.rs must declare bounded module {module}"
        );
        assert!(
            root.join(format!("src/usecases/{module}.rs")).is_file(),
            "usecases::{module} must exist as a bounded responsibility module"
        );
    }
    assert!(
        surface.contains("#[cfg(test)]\nmod tests;"),
        "usecases tests must live outside the production orchestrator body"
    );
    for forbidden in [
        "pub struct HostObservationReports",
        "pub struct HostApplyExecutionReport",
        "fn extend_commands",
        "fn executed_result",
        "fn blocked_domain_statuses",
    ] {
        assert!(
            !surface.contains(forbidden),
            "usecases/mod.rs must not own extracted host usecase detail: {forbidden}"
        );
    }
    assert!(
        reports.contains("pub struct HostObservationReports")
            && reports.contains("pub struct HostApplyExecutionReport"),
        "usecases reports DTOs must live in reports.rs"
    );
    assert!(
        status.contains("pub(super) fn blocked_domain_statuses")
            && status.contains("pub(super) fn next_actions"),
        "host usecase status helpers must live in status.rs"
    );
    assert!(
        execution.contains("pub fn execute_host_apply") && execution.contains("fn extend_commands"),
        "host apply execution must live in execution.rs"
    );
}
