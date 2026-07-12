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

#[test]
fn system_probe_has_domain_port_and_local_adapter_boundaries() {
    let root = crate_root();
    let probe = fs::read_to_string(root.join("src/system_probe.rs")).expect("read system_probe.rs");
    let domain = fs::read_to_string(root.join("src/domain/system_observation.rs"))
        .expect("read domain/system_observation.rs");
    let port = fs::read_to_string(root.join("src/ports/system_probe_port.rs"))
        .expect("read ports/system_probe_port.rs");
    let adapter = fs::read_to_string(root.join("src/adapters/local_system_probe/mod.rs"))
        .expect("read adapters/local_system_probe/mod.rs");

    assert!(
        probe.lines().count() <= 140,
        "system_probe.rs must remain a thin orchestrator"
    );
    for forbidden in [
        "sysinfo::",
        "CpuRefreshKind",
        "Disks::",
        "capture_command",
        "pvesh",
        "qm",
        "critical_proxmox_vmids_to_restart",
        "parse_zpool_health",
    ] {
        assert!(
            !probe.contains(forbidden),
            "system_probe.rs must not own concrete host/platform detail: {forbidden}"
        );
    }
    assert!(
        domain.contains("pub struct SystemMetricsSample")
            && domain.contains("pub fn critical_proxmox_vmids_to_restart")
            && domain.contains("pub fn system_observation_payload"),
        "system observation rules and payload builders must live in domain"
    );
    assert!(
        port.contains("pub trait SystemMetricsPort")
            && port.contains("pub trait SystemProbePlatformPort"),
        "system probe must expose metrics and platform ports"
    );
    assert!(
        adapter.contains("impl SystemMetricsPort for SysinfoMetricsSampler")
            && adapter.contains("impl SystemProbePlatformPort for LinuxPlatformProbe")
            && adapter.contains("capture_command"),
        "sysinfo and local command probing must live in the local adapter"
    );
}

#[test]
fn watchdog_reuses_system_metrics_port_instead_of_resampling_os_state() {
    let root = crate_root();
    let sampler =
        fs::read_to_string(root.join("src/watchdog/sampler.rs")).expect("read watchdog/sampler.rs");
    let host_runtime = fs::read_to_string(root.join("src/watchdog/host_runtime.rs"))
        .expect("read watchdog/host_runtime.rs");

    assert!(
        sampler.contains("SysinfoMetricsSampler")
            && sampler.contains("SystemMetricsPort")
            && sampler.contains("sample_metrics()"),
        "watchdog sampler must reuse the shared system metrics port"
    );
    for forbidden in [
        "CpuRefreshKind",
        "MemoryRefreshKind",
        "Disks::",
        "System::load_average",
        "System::uptime",
        "disk_totals_gb",
        "bytes_to_gb",
    ] {
        assert!(
            !sampler.contains(forbidden) && !host_runtime.contains(forbidden),
            "watchdog must not own duplicate OS metric sampling: {forbidden}"
        );
    }
}

#[test]
fn network_mesh_apply_plan_reads_bundles_through_a_port() {
    let root = crate_root();
    let planner = fs::read_to_string(root.join("src/network_mesh_apply_plan.rs"))
        .expect("read network_mesh_apply_plan.rs");
    let production_planner = planner.split("#[cfg(test)]").next().unwrap_or(&planner);
    let domain = fs::read_to_string(root.join("src/domain/network_mesh_apply.rs"))
        .expect("read domain/network_mesh_apply.rs");
    let port = fs::read_to_string(root.join("src/ports/network_mesh_bundle_port.rs"))
        .expect("read ports/network_mesh_bundle_port.rs");
    let adapter = fs::read_to_string(root.join("src/adapters/local_network_mesh_bundle/mod.rs"))
        .expect("read adapters/local_network_mesh_bundle/mod.rs");
    let command_plan = fs::read_to_string(root.join("src/network_mesh_apply_plan/command_plan.rs"))
        .expect("read network_mesh_apply_plan/command_plan.rs");
    let policy_module = fs::read_to_string(root.join("src/network_mesh_apply_plan/policy.rs"))
        .expect("read network_mesh_apply_plan/policy.rs");
    let profile_execution =
        fs::read_to_string(root.join("src/network_mesh_apply_plan/profile_execution.rs"))
            .expect("read network_mesh_apply_plan/profile_execution.rs");

    assert!(
        planner.lines().count() <= 300
            && planner.contains("mod command_plan;")
            && planner.contains("mod policy;")
            && planner.contains("mod profile_execution;"),
        "network_mesh_apply_plan.rs must stay a thin orchestration surface"
    );
    assert!(
        domain.contains("pub(crate) struct MeshApplyBundle")
            && domain.contains("pub(crate) struct MeshBundleManifest")
            && domain.contains("pub(crate) struct MeshPlanFile"),
        "mesh apply bundle DTOs must live in the domain layer"
    );
    assert!(
        port.contains("pub(crate) trait NetworkMeshBundlePort")
            && port.contains("fn load_mesh_bundle"),
        "mesh apply bundle loading must expose a port boundary"
    );
    assert!(
        adapter.contains("impl NetworkMeshBundlePort for LocalNetworkMeshBundleReader")
            && adapter.contains("fs::read_to_string"),
        "local mesh bundle filesystem loading must live in the local adapter"
    );
    for forbidden in ["std::fs", "read_to_string", "PathBuf", "MANIFEST_FILENAME"] {
        assert!(
            !planner.contains(forbidden),
            "network_mesh_apply_plan.rs must not own bundle filesystem loading: {forbidden}"
        );
    }
    for forbidden in [
        "fn preflight",
        "fn route_install",
        "fn mesh_actuator_policy",
        "fn validate_mesh_profile_request",
        "fn non_executable_violations",
    ] {
        assert!(
            !production_planner.contains(forbidden),
            "network_mesh_apply_plan.rs must not own extracted mesh apply detail: {forbidden}"
        );
    }
    assert!(
        command_plan.contains("pub(super) fn route_install")
            && command_plan.contains("pub(super) fn rollback_plan"),
        "mesh apply command construction must live in command_plan"
    );
    assert!(
        policy_module.contains("pub(super) fn mesh_actuator_policy")
            && policy_module.contains("pub(crate) struct MeshActuatorPolicyProjection"),
        "mesh apply policy projection must live in policy"
    );
    assert!(
        profile_execution.contains("pub(super) fn apply_network_mesh_profile_from_plan")
            && profile_execution.contains("fn validate_mesh_profile_request"),
        "mesh apply profile validation and execution must live in profile_execution"
    );
    assert!(
        planner.contains("NetworkMeshBundlePort")
            && planner.contains("LocalNetworkMeshBundleReader")
            && planner.contains("build_network_mesh_apply_plan_with_reader"),
        "network_mesh_apply_plan.rs must plan through the bundle port with a thin local façade"
    );
}

#[test]
fn network_actuator_bundle_uses_portable_command_context() {
    let root = crate_root();
    let bundle =
        fs::read_to_string(root.join("src/network_actuator/bundle.rs")).expect("read bundle.rs");
    let production_bundle = bundle.split("#[cfg(test)]").next().unwrap_or(&bundle);
    let context = fs::read_to_string(root.join("src/network_actuator/bundle/context.rs"))
        .expect("read bundle/context.rs");
    let commands = fs::read_to_string(root.join("src/network_actuator/bundle/commands.rs"))
        .expect("read bundle/commands.rs");

    assert!(
        bundle.lines().count() <= 420
            && bundle.contains("mod commands;")
            && bundle.contains("mod context;"),
        "network_actuator/bundle.rs must stay below monolith size and delegate bounded details"
    );
    assert!(
        context.contains("pub(crate) struct NetworkActuatorBundleContext")
            && context.contains("pub(crate) fn from_current_runtime() -> Self")
            && context.contains("denied_shell_bin"),
        "network actuator bundle must expose an explicit injectable command context"
    );
    assert!(
        commands.contains("pub(super) fn denial_checks")
            && commands.contains("pub(super) fn postflight_commands")
            && commands.contains("context.denied_shell_bin.clone()"),
        "network actuator command/proof surfaces must consume the injected context"
    );
    assert!(
        !commands.contains("\"/bin/sh\"")
            && !commands.contains("\"/usr/bin/sudo\"")
            && !commands.contains("\"/sbin/ip\"")
            && !commands.contains("\"/usr/bin/wg\""),
        "network actuator commands must not hardcode host-specific binaries"
    );
    assert!(
        !production_bundle.contains("current_runtime_layout()")
            && !production_bundle.contains("struct NetworkActuatorRuntimeContext"),
        "network actuator bundle orchestration must not own runtime probing detail"
    );
}
