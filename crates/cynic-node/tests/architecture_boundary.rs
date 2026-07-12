use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn node_binary_stays_a_thin_runtime_adapter() {
    let root = crate_root();
    let main_rs = fs::read_to_string(root.join("src/main.rs")).expect("read main.rs");
    let line_count = main_rs.lines().count();

    assert!(
        line_count <= 300,
        "cynic-node main.rs must remain a thin adapter; current line count is {line_count}"
    );
    for forbidden in [
        "process_wrap::tokio::ChildWrapper",
        "async fn watch",
        "async fn wait_healthy",
        "async fn register_with_kernel",
        "fn build_registration_payload",
        "WebSocketClient::new",
        "CYNIC_NODE_PUBLIC_KEY",
        "stub-signature-phase",
    ] {
        assert!(
            !main_rs.contains(forbidden),
            "main.rs must not own runtime supervision detail: {forbidden}"
        );
    }
    assert!(
        main_rs.contains("runtime_supervision::run_node"),
        "main.rs must delegate node runtime supervision to runtime_supervision"
    );
}

#[test]
fn node_supervision_state_is_owned_outside_main() {
    let root = crate_root();
    let runtime_supervision = fs::read_to_string(root.join("src/runtime_supervision.rs"))
        .expect("read runtime_supervision.rs");
    let lifecycle = fs::read_to_string(root.join("src/runtime_supervision/lifecycle.rs"))
        .expect("read runtime_supervision/lifecycle.rs");
    let domain_supervision = fs::read_to_string(root.join("src/domain/supervision.rs"))
        .expect("read domain/supervision.rs");

    assert!(
        runtime_supervision.contains("mod lifecycle;")
            && runtime_supervision.contains("use lifecycle::run_lifecycle;"),
        "runtime_supervision.rs must delegate lifecycle detail to a submodule"
    );
    assert!(
        !runtime_supervision.contains("async fn watch")
            && !runtime_supervision.contains("async fn register_with_kernel"),
        "runtime_supervision.rs must stay high-level; lifecycle detail belongs in runtime_supervision/lifecycle.rs"
    );
    assert!(
        lifecycle.contains("pub(crate) async fn run_lifecycle")
            && lifecycle.contains("async fn watch")
            && lifecycle.contains("async fn register_with_kernel"),
        "runtime supervision lifecycle must live in runtime_supervision/lifecycle.rs"
    );
    assert!(
        domain_supervision.contains("pub enum ExitReason"),
        "supervision outcome type must live in the domain layer"
    );
}
