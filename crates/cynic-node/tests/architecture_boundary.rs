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
    for forbidden in [
        "std::env::var",
        "WebSocketClient::new",
        "WebSocketFederationAdapter",
        "DETERMINISTIC_FALLBACK",
        "stub-public-key-phase",
        "stub-signature-phase",
    ] {
        assert!(
            !lifecycle.contains(forbidden),
            "runtime_supervision/lifecycle.rs must not own federation detail: {forbidden}"
        );
    }
    assert!(
        domain_supervision.contains("pub enum ExitReason"),
        "supervision outcome type must live in the domain layer"
    );
}

#[test]
fn runtime_federation_has_domain_port_and_adapter_boundaries() {
    let root = crate_root();
    let domain = fs::read_to_string(root.join("src/domain/runtime_federation.rs"))
        .expect("read domain/runtime_federation.rs");
    let port = fs::read_to_string(root.join("src/ports/runtime_federation_port.rs"))
        .expect("read ports/runtime_federation_port.rs");
    let adapter = fs::read_to_string(root.join("src/adapters/websocket_federation/mod.rs"))
        .expect("read adapters/websocket_federation/mod.rs");
    let lifecycle = fs::read_to_string(root.join("src/runtime_supervision/lifecycle.rs"))
        .expect("read runtime_supervision/lifecycle.rs");
    let config_load =
        fs::read_to_string(root.join("src/config/load.rs")).expect("read config/load.rs");

    assert!(
        domain.contains("pub struct RuntimeFederationConfig")
            && domain.contains("pub fn deterministic_fallback_verdict"),
        "runtime federation config and fallback verdict rules must live in domain"
    );
    assert!(
        port.contains("pub trait RuntimeFederationPort")
            && port.contains("pub trait RuntimeFederationFactory"),
        "runtime federation must expose port and factory boundaries"
    );
    assert!(
        adapter.contains("impl RuntimeFederationFactory for WebSocketFederationFactory")
            && adapter.contains("impl RuntimeFederationPort for WebSocketFederationAdapter")
            && adapter.contains("WebSocketClient::new"),
        "WebSocket client wiring must live in the concrete federation adapter"
    );
    let factory_impl = adapter
        .split("impl RuntimeFederationFactory for WebSocketFederationFactory")
        .nth(1)
        .and_then(|tail| tail.split("impl WebSocketFederationAdapter").next())
        .unwrap_or_default();
    assert!(
        factory_impl.contains("fn build(&self, config: RuntimeFederationConfig)")
            && factory_impl.contains("WebSocketFederationAdapter::new(config)")
            && !factory_impl.contains("std::env::var")
            && !factory_impl.contains("from_env"),
        "default federation factory must consume explicit RuntimeFederationConfig, not env"
    );
    assert!(
        lifecycle.contains("RuntimeFederationFactory")
            && lifecycle.contains("federation_factory.build(federation_config)"),
        "lifecycle must consume federation through a factory boundary"
    );
    assert!(
        config_load.contains("pub(crate) fn runtime_federation_config")
            && config_load.contains("std::env::var(\"CYNIC_API_KEY\")"),
        "runtime federation env fallback must be isolated in config loading"
    );
}
