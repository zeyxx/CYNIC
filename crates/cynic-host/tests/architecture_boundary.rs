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
