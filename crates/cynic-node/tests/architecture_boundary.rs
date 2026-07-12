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
    let command_plan_verification =
        fs::read_to_string(root.join("src/network_mesh_apply_plan/command_plan_verification.rs"))
            .expect("read network_mesh_apply_plan/command_plan_verification.rs");
    let context = fs::read_to_string(root.join("src/network_mesh_apply_plan/context.rs"))
        .expect("read network_mesh_apply_plan/context.rs");
    let policy_module = fs::read_to_string(root.join("src/network_mesh_apply_plan/policy.rs"))
        .expect("read network_mesh_apply_plan/policy.rs");
    let policy_verification =
        fs::read_to_string(root.join("src/network_mesh_apply_plan/policy_verification.rs"))
            .expect("read network_mesh_apply_plan/policy_verification.rs");
    let profile_execution =
        fs::read_to_string(root.join("src/network_mesh_apply_plan/profile_execution.rs"))
            .expect("read network_mesh_apply_plan/profile_execution.rs");
    let planner_without_context_factory = production_planner
        .replace("fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "");

    assert!(
        planner.lines().count() <= 300
            && planner.contains("mod command_plan;")
            && planner.contains("mod command_plan_verification;")
            && planner.contains("mod context;")
            && planner.contains("mod policy;")
            && planner.contains("mod policy_verification;")
            && planner.contains("mod profile_execution;")
            && planner.contains("build_network_mesh_apply_plan_with_reader_and_context"),
        "network_mesh_apply_plan.rs must stay a thin orchestration surface"
    );
    assert!(
        planner_without_context_factory
            .contains("NetworkMeshApplyPlanContext::from_current_runtime()")
            && !planner_without_context_factory.contains("current_runtime_layout()"),
        "runtime layout probing must be isolated in NetworkMeshApplyPlanContext::from_current_runtime"
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
        context.contains("pub(super) struct NetworkMeshApplyPlanContext")
            && context.contains("pub(super) fn from_current_runtime() -> Self")
            && context.contains("let runtime_layout = current_runtime_layout();"),
        "network mesh apply plan runtime probing must live in its dedicated context module"
    );
    assert!(
        command_plan.contains("pub(super) fn route_install")
            && command_plan.contains("pub(super) fn rollback_plan"),
        "mesh apply command construction must live in command_plan"
    );
    assert!(
        command_plan_verification.contains("#[kani::proof]")
            && command_plan_verification
                .contains("proof_route_install_uses_injected_ip_bin_and_mesh_interface")
            && command_plan_verification
                .contains("proof_post_apply_observation_adds_dns_probe_only_when_dns_exists"),
        "mesh apply Kani command proofs must live in command_plan_verification.rs"
    );
    assert!(
        policy_module.contains("pub(super) fn mesh_actuator_policy")
            && policy_module.contains("pub(crate) struct MeshActuatorPolicyProjection"),
        "mesh apply policy projection must live in policy"
    );
    assert!(
        policy_verification.contains("#[kani::proof]")
            && policy_verification
                .contains("proof_mesh_policy_exposes_stable_profile_order_and_count")
            && policy_verification
                .contains("proof_mesh_policy_forbidden_tokens_keep_shell_escape_vectors_blocked"),
        "mesh apply Kani policy proofs must live in policy_verification.rs"
    );
    assert!(
        profile_execution.contains("pub(super) fn apply_network_mesh_profile_from_plan")
            && profile_execution.contains("fn validate_mesh_profile_request"),
        "mesh apply profile validation and execution must live in profile_execution"
    );
    assert!(
        planner.contains("NetworkMeshBundlePort")
            && planner.contains("LocalNetworkMeshBundleReader")
            && planner.contains("build_network_mesh_apply_plan_with_reader_and_context"),
        "network_mesh_apply_plan.rs must plan through the bundle port with a thin local façade"
    );
    let tests = planner.split("#[cfg(test)]").nth(1).unwrap_or_default();
    assert!(
        tests.contains("/portable/bin/ip")
            && tests.contains("/portable/bin/wg")
            && tests.contains("/portable/bin/getent"),
        "network mesh apply plan tests must prove portable binary injection"
    );
    for stale_fixture in ["\"/sbin/ip\"", "\"/usr/bin/wg\"", "\"/usr/bin/getent\""] {
        assert!(
            !tests.contains(stale_fixture),
            "network mesh apply plan tests must not preserve host-specific fixture: {stale_fixture}"
        );
    }
}

#[test]
fn network_mesh_plan_reads_registry_through_a_port() {
    let root = crate_root();
    let planner = fs::read_to_string(root.join("src/network_mesh_plan.rs"))
        .expect("read network_mesh_plan.rs");
    let domain_model = fs::read_to_string(root.join("src/network_mesh_plan/model.rs"))
        .expect("read network_mesh_plan/model.rs");
    let verification = fs::read_to_string(root.join("src/network_mesh_plan/verification.rs"))
        .expect("read network_mesh_plan/verification.rs");
    let port = fs::read_to_string(root.join("src/ports/network_mesh_registry_port.rs"))
        .expect("read ports/network_mesh_registry_port.rs");
    let adapter = fs::read_to_string(root.join("src/adapters/local_network_mesh_registry/mod.rs"))
        .expect("read adapters/local_network_mesh_registry/mod.rs");

    assert!(
        planner.lines().count() <= 300
            && planner.contains("pub(crate) mod model;")
            && planner.contains("mod verification;")
            && planner.contains("build_network_mesh_plan_with_reader"),
        "network_mesh_plan.rs must stay a thin mesh planning surface"
    );
    for forbidden in [
        "std::fs::read_to_string",
        "serde_json::from_str(&raw)",
        "Deserialize",
    ] {
        assert!(
            !planner.contains(forbidden),
            "network_mesh_plan.rs must not own registry file loading/parsing: {forbidden}"
        );
    }
    assert!(
        verification.contains("#[kani::proof]")
            && verification.contains("proof_address_plane_prefers_declared_service_plane")
            && verification.contains("proof_default_dns_names_follow_zone_presence"),
        "mesh planning Kani proofs must live in network_mesh_plan/verification.rs"
    );
    assert!(
        domain_model.contains("pub(crate) struct MeshRegistry")
            && domain_model.contains("pub(crate) struct NetworkPolicy")
            && domain_model.contains("pub(crate) struct RegistryNode"),
        "mesh registry DTOs must live in network_mesh_plan/model.rs"
    );
    assert!(
        port.contains("pub(crate) trait NetworkMeshRegistryPort")
            && port.contains("fn load_mesh_registry"),
        "mesh plan registry loading must expose a dedicated port"
    );
    assert!(
        adapter.contains("impl NetworkMeshRegistryPort for LocalNetworkMeshRegistryReader")
            && adapter.contains("fs::read_to_string")
            && adapter.contains("serde_json::from_str"),
        "local mesh registry file loading must live in the local adapter"
    );
    let tests = planner.split("#[cfg(test)]").nth(1).unwrap_or_default();
    assert!(
        tests.contains("FakeRegistryReader") && tests.contains("/virtual/registry.json"),
        "network mesh plan tests must prove substituted registry loading"
    );
}

#[test]
fn network_mesh_bundle_writes_through_a_port() {
    let root = crate_root();
    let bundle = fs::read_to_string(root.join("src/network_mesh_bundle.rs"))
        .expect("read network_mesh_bundle.rs");
    let port = fs::read_to_string(root.join("src/ports/network_mesh_bundle_writer_port.rs"))
        .expect("read ports/network_mesh_bundle_writer_port.rs");
    let adapter =
        fs::read_to_string(root.join("src/adapters/local_network_mesh_bundle_writer/mod.rs"))
            .expect("read adapters/local_network_mesh_bundle_writer/mod.rs");

    assert!(
        bundle.lines().count() <= 240
            && bundle.contains("build_network_mesh_bundle_with_writer")
            && bundle.contains("LocalNetworkMeshBundleWriter")
            && bundle.contains("NetworkMeshBundleWriterPort"),
        "network_mesh_bundle.rs must stay a thin bundle projection surface"
    );
    for forbidden in ["use std::fs;", "fs::create_dir_all", "fs::write("] {
        assert!(
            !bundle.contains(forbidden),
            "network_mesh_bundle.rs must not own direct filesystem writes: {forbidden}"
        );
    }
    assert!(
        port.contains("pub(crate) trait NetworkMeshBundleWriterPort")
            && port.contains("fn create_dir_all")
            && port.contains("fn write_string"),
        "mesh bundle writing must expose a dedicated writer port"
    );
    assert!(
        adapter.contains("impl NetworkMeshBundleWriterPort for LocalNetworkMeshBundleWriter")
            && adapter.contains("fs::create_dir_all")
            && adapter.contains("fs::write"),
        "local mesh bundle filesystem writes must live in the local adapter"
    );
    let tests = bundle.split("#[cfg(test)]").nth(1).unwrap_or_default();
    assert!(
        tests.contains("RecordingBundleWriter") && tests.contains("/virtual/bundle"),
        "network mesh bundle tests must prove substituted writer usage"
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

#[test]
fn network_actuator_apply_uses_explicit_command_context() {
    let root = crate_root();
    let apply =
        fs::read_to_string(root.join("src/network_actuator/apply.rs")).expect("read apply.rs");
    let production_apply = apply.split("#[cfg(test)]").next().unwrap_or(&apply);
    let apply_without_context_factory = production_apply
        .replace("fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "");

    assert!(
        production_apply.contains("struct NetworkActuatorApplyContext")
            && production_apply.contains("fn from_current_runtime() -> Self")
            && production_apply.contains("fn apply_profile_with_context")
            && production_apply.contains("fn profile_commands_with_context"),
        "network actuator apply must expose explicit context-driven entrypoints"
    );
    assert!(
        apply_without_context_factory
            .contains("NetworkActuatorApplyContext::from_current_runtime()")
            && !apply_without_context_factory.contains("current_runtime_layout()"),
        "runtime layout probing must be isolated in NetworkActuatorApplyContext::from_current_runtime"
    );
    assert!(
        production_apply.contains("context.ip_bin.clone()")
            && production_apply.contains("context.systemctl_bin.clone()")
            && production_apply.contains("context.wg_bin.clone()"),
        "network actuator apply commands must consume injected binaries"
    );
    let tests = apply.split("#[cfg(test)]").nth(1).unwrap_or_default();
    assert!(
        tests.contains("/portable/bin/ip")
            && tests.contains("/portable/bin/systemctl")
            && tests.contains("/portable/bin/wg"),
        "network actuator apply tests must prove portable binary injection"
    );
    for stale_fixture in ["\"/sbin/ip\"", "\"/usr/bin/wg\"", "\"/bin/systemctl\""] {
        assert!(
            !tests.contains(stale_fixture),
            "network actuator apply tests must not preserve host-specific fixture: {stale_fixture}"
        );
    }
}

#[test]
fn network_actuator_policy_uses_explicit_command_context() {
    let root = crate_root();
    let policy =
        fs::read_to_string(root.join("src/network_actuator/policy.rs")).expect("read policy.rs");
    let production_policy = policy.split("#[cfg(test)]").next().unwrap_or(&policy);
    let policy_without_context_factory = production_policy
        .replace("fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "");

    assert!(
        production_policy.contains("struct NetworkActuatorPolicyContext")
            && production_policy.contains("fn from_current_runtime() -> Self")
            && production_policy.contains("fn build_policy_document_with_context"),
        "network actuator policy must expose explicit context-driven entrypoints"
    );
    assert!(
        policy_without_context_factory
            .contains("NetworkActuatorPolicyContext::from_current_runtime()")
            && !policy_without_context_factory.contains("current_runtime_layout()"),
        "runtime layout probing must be isolated in NetworkActuatorPolicyContext::from_current_runtime"
    );
    assert!(
        production_policy.contains("context.ip_bin")
            && production_policy.contains("context.systemctl_bin.clone()")
            && production_policy.contains("context.wg_bin.clone()"),
        "network actuator policy command prefixes must consume injected binaries"
    );
    let tests = policy.split("#[cfg(test)]").nth(1).unwrap_or_default();
    assert!(
        tests.contains("/portable/bin/ip")
            && tests.contains("/portable/bin/systemctl")
            && tests.contains("/portable/bin/wg")
            && tests.contains("/portable/bin/cynic-network-actuator"),
        "network actuator policy tests must prove portable binary injection"
    );
    for stale_fixture in ["\"/sbin/ip\"", "\"/usr/bin/wg\"", "\"/bin/systemctl\""] {
        assert!(
            !tests.contains(stale_fixture),
            "network actuator policy tests must not preserve host-specific fixture: {stale_fixture}"
        );
    }
}

#[test]
fn network_root_policy_uses_explicit_command_context() {
    let root = crate_root();
    let policy = fs::read_to_string(root.join("src/network_root_policy.rs"))
        .expect("read network_root_policy.rs");
    let production_policy = policy.split("#[cfg(test)]").next().unwrap_or(&policy);
    let policy_without_context_factory = production_policy
        .replace("fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "");

    assert!(
        production_policy.contains("struct NetworkRootPolicyContext")
            && production_policy.contains("fn from_current_runtime() -> Self")
            && production_policy.contains("fn build_root_policy_with_context"),
        "network root policy must expose explicit context-driven entrypoints"
    );
    assert!(
        policy_without_context_factory.contains("NetworkRootPolicyContext::from_current_runtime()")
            && !policy_without_context_factory.contains("current_runtime_layout()"),
        "runtime layout probing must be isolated in NetworkRootPolicyContext::from_current_runtime"
    );
    assert!(
        production_policy.contains("context.helper_path")
            && production_policy.contains("context.sudoers_target")
            && production_policy.contains("context.sudoers_rule")
            && production_policy.contains("context\n            .forbidden_patterns"),
        "network root policy must consume the injected root context"
    );
    let tests = policy.split("#[cfg(test)]").nth(1).unwrap_or_default();
    assert!(
        tests.contains("/portable/bin/cynic-network-actuator")
            && tests.contains("/portable/etc/sudoers.d/cynic-network-actuator")
            && tests.contains("/portable/bin/ip")
            && tests.contains("/portable/bin/wg"),
        "network root policy tests must prove portable path injection"
    );
    for stale_fixture in [
        "\"/usr/local/bin/cynic-network-actuator\"",
        "\"/etc/sudoers.d/cynic-network-actuator\"",
        "\"/sbin/ip\"",
        "\"/usr/bin/wg\"",
    ] {
        assert!(
            !tests.contains(stale_fixture),
            "network root policy tests must not preserve host-specific fixture: {stale_fixture}"
        );
    }
}

#[test]
fn network_repair_uses_explicit_command_context() {
    let root = crate_root();
    let repair =
        fs::read_to_string(root.join("src/network_repair.rs")).expect("read network_repair.rs");
    let production_repair = repair.split("#[cfg(test)]").next().unwrap_or(&repair);
    let repair_without_context_factory = production_repair
        .replace("fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "");

    assert!(
        production_repair.contains("struct NetworkRepairContext")
            && production_repair.contains("fn from_current_runtime() -> Self")
            && production_repair.contains("fn build_dry_run_plan_with_context")
            && production_repair.contains("fn command_preview_with_context"),
        "network repair must expose explicit context-driven entrypoints"
    );
    assert!(
        repair_without_context_factory.contains("NetworkRepairContext::from_current_runtime()")
            && !repair_without_context_factory.contains("current_runtime_layout()"),
        "runtime layout probing must be isolated in NetworkRepairContext::from_current_runtime"
    );
    assert!(
        production_repair.contains("context.route_command.clone()")
            && production_repair.contains("context.network_actuator_bundle_dir.clone()")
            && production_repair.contains("context.provider_fallback_bundle_dir.clone()")
            && production_repair.contains("context.dns_bundle_dir.clone()"),
        "network repair previews must consume injected runtime paths and commands"
    );
    let tests = repair.split("#[cfg(test)]").nth(1).unwrap_or_default();
    assert!(
        tests.contains("/portable/bin/ip")
            && tests.contains("/portable/runtime/actuator")
            && tests.contains("/portable/runtime/provider-fallback")
            && tests.contains("/portable/runtime/dns"),
        "network repair tests must prove portable path injection"
    );
    for stale_fixture in [
        "\"/sbin/ip\"",
        "\"/tmp/actuator\"",
        "\"/tmp/provider\"",
        "\"/tmp/dns\"",
    ] {
        assert!(
            !tests.contains(stale_fixture),
            "network repair tests must not preserve host-specific fixture: {stale_fixture}"
        );
    }
}

#[test]
fn node_observation_surfaces_use_explicit_command_contexts() {
    let root = crate_root();
    let observe = fs::read_to_string(root.join("src/observe.rs")).expect("read src/observe.rs");
    let observe_uc = fs::read_to_string(root.join("src/usecases/observe_node_uc.rs"))
        .expect("read src/usecases/observe_node_uc.rs");
    let production_observe = observe.split("#[cfg(test)]").next().unwrap_or(&observe);
    let production_observe_uc = observe_uc
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(&observe_uc);
    let observe_without_context_factory = production_observe
        .replace("pub(crate) fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "")
        .replace("let host_capabilities = runtime_layout.capabilities();", "");
    let observe_uc_without_context_factory = production_observe_uc
        .replace("fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "")
        .replace("let capabilities = runtime_layout.capabilities();", "");

    assert!(
        observe.contains("pub(crate) struct SyntheticNodeObservationContext")
            && observe.contains("synthetic_for_tests_with_context"),
        "observe.rs must expose an explicit synthetic observation context"
    );
    assert!(
        production_observe_uc.contains("struct NodeObservationCommandContext")
            && production_observe_uc.contains("fn from_current_runtime() -> Self")
            && production_observe_uc.contains("fn build_node_observation_with_context")
            && production_observe_uc
                .contains("fn from_context(context: &NodeObservationCommandContext) -> Self"),
        "observe_node_uc.rs must expose explicit command context and local port factory"
    );
    assert!(
        !observe_without_context_factory.contains("current_runtime_layout()")
            && !observe_uc_without_context_factory.contains("current_runtime_layout()"),
        "runtime layout probing must be isolated in observation context factories"
    );
    let observe_tests = observe.split("#[cfg(test)]").nth(1).unwrap_or_default();
    let observe_uc_tests = observe_uc.split("#[cfg(test)]").nth(1).unwrap_or_default();
    for expected in [
        "/portable/bin/ip",
        "/portable/bin/wg",
        "/portable/bin/systemctl",
        "wg-quick@wg-portable",
        "portable-resolved",
    ] {
        assert!(
            observe_tests.contains(expected) || observe_uc_tests.contains(expected),
            "observation tests must prove portable context injection: {expected}"
        );
    }
}

#[test]
fn network_actuator_status_family_uses_explicit_command_contexts() {
    let root = crate_root();
    let status = fs::read_to_string(root.join("src/network_actuator_status.rs"))
        .expect("read network_actuator_status.rs");
    let proofs = fs::read_to_string(root.join("src/network_actuator_status/proofs.rs"))
        .expect("read network_actuator_status/proofs.rs");
    let contract = fs::read_to_string(root.join("src/network_actuator_contract.rs"))
        .expect("read network_actuator_contract.rs");
    let production_status = status.split("#[cfg(test)]").next().unwrap_or(&status);
    let production_proofs = proofs.split("#[cfg(test)]").next().unwrap_or(&proofs);
    let production_contract = contract.split("#[cfg(test)]").next().unwrap_or(&contract);

    let status_without_context_factory = production_status
        .replace("fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "")
        .replace("let capabilities = runtime_layout.capabilities();", "");
    let proofs_without_context_factory = production_proofs
        .replace("fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "")
        .replace("let capabilities = runtime_layout.capabilities();", "");
    let contract_without_context_factory = production_contract
        .replace("fn from_current_runtime() -> Self", "")
        .replace("let runtime_layout = current_runtime_layout();", "");

    assert!(
        production_status.contains("struct NetworkActuatorStatusContext")
            && production_status.contains("fn from_current_runtime() -> Self")
            && production_status.contains("activation_status_with_host_context"),
        "network_actuator_status.rs must expose an explicit status context"
    );
    assert!(
        production_proofs.contains("struct NetworkActuatorProofContext")
            && production_proofs.contains("fn from_current_runtime() -> Self")
            && production_proofs.contains("proof_requirements_with_context"),
        "network_actuator_status/proofs.rs must expose an explicit proof context"
    );
    assert!(
        production_contract.contains("struct NetworkActuatorContractContext")
            && production_contract.contains("fn from_current_runtime() -> Self"),
        "network_actuator_contract.rs must expose an explicit contract context"
    );
    assert!(
        !status_without_context_factory.contains("current_runtime_layout()")
            && !proofs_without_context_factory.contains("current_runtime_layout()")
            && !contract_without_context_factory.contains("current_runtime_layout()"),
        "runtime layout probing must be isolated in actuator status family context factories"
    );
    let status_tests = status.split("#[cfg(test)]").nth(1).unwrap_or_default();
    let proofs_tests = proofs.split("#[cfg(test)]").nth(1).unwrap_or_default();
    let contract_tests = contract.split("#[cfg(test)]").nth(1).unwrap_or_default();
    for expected in [
        "/portable/bin/ip",
        "/portable/bin/wg",
        "/portable/bin/sh",
        "/portable/bin/cynic-network-actuator",
        "/portable/etc/cynic/network-actuator.policy.json",
        "/portable/etc/sudoers.d/cynic-network-actuator",
        "/portable/runtime/actuator",
    ] {
        assert!(
            status_tests.contains(expected)
                || proofs_tests.contains(expected)
                || contract_tests.contains(expected),
            "actuator status family tests must prove portable path injection: {expected}"
        );
    }
}
