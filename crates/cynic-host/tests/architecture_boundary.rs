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
