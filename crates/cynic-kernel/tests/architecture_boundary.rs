use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let crates_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crates_dir
        .parent()
        .unwrap_or_else(|| panic!("failed to resolve workspace crates dir"));

    workspace_root
        .parent()
        .unwrap_or_else(|| panic!("failed to resolve workspace root"))
        .to_path_buf()
}

fn read_repo_file(relative_path: &str) -> String {
    fs::read_to_string(repo_root().join(relative_path))
        .unwrap_or_else(|err| panic!("failed to read {relative_path}: {err}"))
}

#[test]
fn kernel_lib_stays_the_compilation_boundary_surface() {
    let lib_rs = read_repo_file("crates/cynic-kernel/src/lib.rs");
    let line_count = lib_rs.lines().count();

    assert!(
        line_count <= 220,
        "kernel lib.rs should stay compact, got {line_count} lines"
    );

    for expected in [
        "pub static MCP_MODE: AtomicBool = AtomicBool::new(false);",
        "macro_rules! klog",
        "pub mod api;",
        "pub mod backends;",
        "pub mod domain;",
        "pub mod infra;",
        "pub mod introspection;",
        "pub mod organ;",
        "pub mod probe;",
        "pub mod senses;",
        "pub mod storage;",
    ] {
        assert!(
            lib_rs.contains(expected),
            "kernel lib.rs should contain `{expected}`"
        );
    }
}

#[test]
fn kernel_main_remains_the_runtime_composition_root() {
    let main_rs = read_repo_file("crates/cynic-kernel/src/main.rs");

    for expected in [
        "#[tokio::main]",
        "KernelEntryFlags::from_env_args()",
        "run_mcp_proxy_mode_if_requested(entry_flags).await?",
        "bootstrap::runtime_orchestrator::run_runtime(entry_flags).await",
    ] {
        assert!(
            main_rs.contains(expected),
            "kernel main.rs should contain `{expected}`"
        );
    }
}

#[test]
fn kernel_runtime_orchestrator_hosts_bootstrap_composition_logic() {
    let bootstrap_rs = read_repo_file("crates/cynic-kernel/src/bootstrap/runtime_orchestrator.rs");

    for expected in [
        "bootstrap::server::run_rest_servers(bootstrap::server::RestServeInputs",
        "load_kernel_config_bundle()",
        "connect_storage_runtime(&storage_config).await",
    ] {
        assert!(
            bootstrap_rs.contains(expected),
            "bootstrap runtime orchestrator should contain `{expected}`"
        );
    }
}

#[test]
fn kernel_api_and_infra_surfaces_stay_split_by_role() {
    let api_mod = read_repo_file("crates/cynic-kernel/src/api/mod.rs");
    let rest_mod = read_repo_file("crates/cynic-kernel/src/api/rest/mod.rs");
    let infra_mod = read_repo_file("crates/cynic-kernel/src/infra/mod.rs");
    let mcp_proxy = read_repo_file("crates/cynic-kernel/src/api/mcp/proxy.rs");
    let tasks_mod = read_repo_file("crates/cynic-kernel/src/infra/tasks/mod.rs");

    for expected in ["pub mod mcp;", "pub mod rest;", "pub mod websocket;"] {
        assert!(
            api_mod.contains(expected),
            "api/mod.rs should contain `{expected}`"
        );
    }

    for expected in [
        "pub mod agent_tasks;",
        "pub mod coord;",
        "pub mod data;",
        "pub mod health;",
        "pub mod inference_router;",
        "pub mod middleware;",
        "pub mod observe;",
        "pub mod response;",
        "pub mod services;",
        "pub mod types;",
    ] {
        assert!(
            rest_mod.contains(expected),
            "api/rest/mod.rs should contain `{expected}`"
        );
    }

    for expected in [
        "pub mod boot;",
        "pub mod config;",
        "pub mod health_loop;",
        "pub mod probes;",
        "pub mod task_health;",
        "pub mod tasks;",
    ] {
        assert!(
            infra_mod.contains(expected),
            "infra/mod.rs should contain `{expected}`"
        );
    }

    for expected in [
        "pub struct CynicMcpProxy",
        "tool_router()",
        "async fn get(",
        "async fn post(",
    ] {
        assert!(
            mcp_proxy.contains(expected),
            "api/mcp/proxy.rs should contain `{expected}`"
        );
    }

    for expected in [
        "pub fn spawn_signal_handler",
        "spawn_remediation_watcher",
        "spawn_organ_remediation",
    ] {
        assert!(
            tasks_mod.contains(expected),
            "infra/tasks/mod.rs should contain `{expected}`"
        );
    }
}

#[test]
fn kernel_fleet_inventory_probe_stays_a_thin_node_adapter() {
    let probe = read_repo_file("crates/cynic-kernel/src/infra/probes/fleet_inventory.rs");

    assert!(
        probe.contains("cynic_node::fleet_inventory::build_fleet_inventory_details"),
        "kernel fleet inventory probe must delegate provider-facing collection to cynic-node"
    );
    for forbidden in [
        "struct ProxmoxInventoryProvider",
        "struct KubernetesInventoryProvider",
        "tokio::process::Command",
        "request_json(",
    ] {
        assert!(
            !probe.contains(forbidden),
            "kernel fleet inventory probe must not re-own provider inventory logic: {forbidden}"
        );
    }
}

#[test]
fn kernel_lifecycle_surface_stays_host_owned() {
    for (path, expected) in [
        (
            "crates/cynic-kernel/src/domain/lifecycle_activation_evidence.rs",
            "pub use cynic_host::lifecycle::activation_evidence::*;",
        ),
        (
            "crates/cynic-kernel/src/domain/lifecycle_activation_requests.rs",
            "pub use cynic_host::lifecycle::activation_requests::*;",
        ),
        (
            "crates/cynic-kernel/src/domain/lifecycle_diagnostics.rs",
            "pub use cynic_host::lifecycle::diagnostics::*;",
        ),
        (
            "crates/cynic-kernel/src/domain/lifecycle_workflow_state.rs",
            "pub use cynic_host::lifecycle::workflow_state::*;",
        ),
    ] {
        let content = read_repo_file(path);
        assert_eq!(
            content.trim(),
            expected,
            "{path} must stay a compatibility facade over cynic-host"
        );
    }
}

#[test]
fn kernel_storage_must_not_own_inference_model_registry() {
    let storage_mod = read_repo_file("crates/cynic-kernel/src/domain/storage/mod.rs");
    let storage_tree = [
        "crates/cynic-kernel/src/domain/storage/mod.rs",
        "crates/cynic-kernel/src/domain/storage/null.rs",
        "crates/cynic-kernel/src/storage/memory.rs",
        "crates/cynic-kernel/src/storage/reconnectable.rs",
        "crates/cynic-kernel/src/storage/surreal.rs",
    ]
    .into_iter()
    .map(read_repo_file)
    .collect::<Vec<_>>()
    .join("\n");

    for forbidden in [
        "ModelRegistryStorage",
        "ModelBackend",
        "ModelCapability",
        "ModelHealth",
        "InferenceEngine",
        "model_backend",
    ] {
        assert!(
            !storage_mod.contains(forbidden),
            "kernel domain storage must not own inference model registry symbol: {forbidden}"
        );
        assert!(
            !storage_tree.contains(forbidden),
            "kernel storage adapters must not own inference model registry symbol: {forbidden}"
        );
    }

    let inference_domain = read_repo_file("crates/cynic-inference/src/domain/model_registry.rs");
    let inference_port = read_repo_file("crates/cynic-inference/src/ports/model_registry_port.rs");
    let inference_memory_adapter =
        read_repo_file("crates/cynic-inference/src/adapters/discovery/model_registry.rs");
    for expected in [
        "pub enum InferenceEngine",
        "pub enum ModelCapability",
        "pub struct ModelBackend",
        "pub enum ModelHealth",
    ] {
        assert!(
            inference_domain.contains(expected),
            "cynic-inference must own model registry domain type: {expected}"
        );
    }
    assert!(
        inference_port.contains("pub trait ModelRegistryPort")
            && inference_port.contains("upsert_model_backend")
            && inference_port.contains("list_model_backends")
            && inference_port.contains("update_model_health"),
        "cynic-inference must own the model registry port"
    );
    assert!(
        inference_memory_adapter.contains("pub struct InMemoryModelRegistry")
            && inference_memory_adapter
                .contains("impl ModelRegistryPort for InMemoryModelRegistry"),
        "cynic-inference must own the in-memory model registry adapter"
    );
}

#[test]
fn kernel_backends_must_not_own_embedding_adapter() {
    let kernel_embedding_adapter =
        repo_root().join("crates/cynic-kernel/src/backends/embedding.rs");
    assert!(
        !kernel_embedding_adapter.exists(),
        "embedding HTTP adapter belongs in cynic-inference, not cynic-kernel backends"
    );

    let inference_embedding_adapter =
        read_repo_file("crates/cynic-inference/src/adapters/embedding/http.rs");
    assert!(
        inference_embedding_adapter.contains("pub struct EmbeddingBackend")
            && inference_embedding_adapter.contains("impl EmbeddingPort for EmbeddingBackend")
            && inference_embedding_adapter.contains("impl BackendPort for EmbeddingBackend"),
        "cynic-inference must own the HTTP embedding adapter and its ports"
    );
}

#[test]
fn kernel_backends_must_not_own_filesystem_model_discovery() {
    let kernel_model_discovery =
        repo_root().join("crates/cynic-kernel/src/backends/model_discovery.rs");
    assert!(
        !kernel_model_discovery.exists(),
        "filesystem model discovery belongs in cynic-inference, not cynic-kernel backends"
    );

    let kernel_backends_mod = read_repo_file("crates/cynic-kernel/src/backends/mod.rs");
    assert!(
        !kernel_backends_mod.contains("model_discovery"),
        "kernel backends module graph must not expose model discovery"
    );

    let inference_model_store =
        read_repo_file("crates/cynic-inference/src/application/model_store_service.rs");
    let inference_local_store =
        read_repo_file("crates/cynic-inference/src/adapters/local/model_store.rs");
    assert!(
        inference_model_store.contains("ModelStoreService")
            && inference_local_store.contains("FsModelStore"),
        "cynic-inference must own local filesystem model store/discovery surfaces"
    );
}

#[test]
fn kernel_must_not_own_summarization_adapter_or_domain() {
    for path in [
        "crates/cynic-kernel/src/backends/summarizer.rs",
        "crates/cynic-kernel/src/domain/summarization.rs",
    ] {
        assert!(
            !repo_root().join(path).exists(),
            "summarization inference ownership belongs in cynic-inference, not {path}"
        );
    }

    let kernel_backends_mod = read_repo_file("crates/cynic-kernel/src/backends/mod.rs");
    let kernel_domain_mod = read_repo_file("crates/cynic-kernel/src/domain/mod.rs");
    assert!(
        !kernel_backends_mod.contains("summarizer") && !kernel_domain_mod.contains("summarization"),
        "kernel module graph must not expose summarization-owned inference surfaces"
    );

    let inference_summary_domain =
        read_repo_file("crates/cynic-inference/src/domain/summarization.rs");
    let inference_summary_port =
        read_repo_file("crates/cynic-inference/src/ports/summarization_port.rs");
    let inference_openai_adapter =
        read_repo_file("crates/cynic-inference/src/adapters/inference/openai_compat.rs");
    assert!(
        inference_summary_domain.contains("pub struct SummarizeRequest")
            && inference_summary_port.contains("pub trait SummarizationPort")
            && inference_openai_adapter.contains("impl InferPort for OpenAiCompatBackend"),
        "cynic-inference must own summarization domain/port and direct inference adapter capability"
    );
}

#[test]
fn kernel_domain_must_not_reexport_inference_or_embedding_law() {
    for path in [
        "crates/cynic-kernel/src/domain/inference.rs",
        "crates/cynic-kernel/src/domain/embedding.rs",
    ] {
        assert!(
            !repo_root().join(path).exists(),
            "kernel domain must not re-export cynic-inference law through {path}"
        );
    }

    let kernel_domain_mod = read_repo_file("crates/cynic-kernel/src/domain/mod.rs");
    for forbidden in ["pub mod inference;", "pub mod embedding;"] {
        assert!(
            !kernel_domain_mod.contains(forbidden),
            "kernel domain module graph must not expose inference facade: {forbidden}"
        );
    }

    let inference_lib = read_repo_file("crates/cynic-inference/src/lib.rs");
    let inference_embedding = read_repo_file("crates/cynic-inference/src/embedding.rs");
    let inference_inference = read_repo_file("crates/cynic-inference/src/inference.rs");
    assert!(
        inference_lib.contains("pub mod domain;")
            && inference_embedding.contains("pub use crate::domain::embedding")
            && inference_inference.contains("pub use crate::domain::inference"),
        "cynic-inference must remain the public owner for embedding and inference law"
    );
}

#[test]
fn kernel_inference_router_delegates_model_catalog_rules_to_inference() {
    let router = read_repo_file("crates/cynic-kernel/src/api/rest/inference_router.rs");
    assert!(
        router.contains("cynic_inference::application::model_catalog")
            && router.contains("deterministic_model_info()")
            && router.contains("project_model_info(")
            && router.contains("should_probe_model("),
        "kernel inference router must delegate model catalog projection to cynic-inference"
    );

    for forbidden in [
        "\"context_size\": cfg.context_size",
        "\"thinking_mode\": !cfg.disable_thinking",
        "\"json_mode\": cfg.json_mode",
        "\"success_count\": 0",
        "actual_model != \"parse_error\"",
    ] {
        assert!(
            !router.contains(forbidden),
            "kernel inference router must not own model catalog rule: {forbidden}"
        );
    }

    let model_catalog = read_repo_file("crates/cynic-inference/src/application/model_catalog.rs");
    for expected in [
        "pub struct ModelInfo",
        "pub struct ListModelsResponse",
        "pub fn deterministic_model_info()",
        "pub fn should_probe_model",
        "pub fn project_model_info",
    ] {
        assert!(
            model_catalog.contains(expected),
            "cynic-inference model_catalog must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_inference_router_delegates_observation_persistence() {
    let router = read_repo_file("crates/cynic-kernel/src/api/rest/inference_router.rs");
    assert!(
        router.contains("store_recovery_observation") && router.contains("store_probe_observation"),
        "kernel inference router must delegate observation persistence"
    );

    for forbidden in [
        "let obs = Observation {",
        "use crate::domain::storage::Observation",
        "store_observation(",
        "agent_id: \"kernel-probe\"",
    ] {
        assert!(
            !router.contains(forbidden),
            "kernel inference router must not construct storage observations inline: {forbidden}"
        );
    }

    let observation = read_repo_file("crates/cynic-kernel/src/api/rest/inference_observation.rs");
    for expected in [
        "pub async fn store_recovery_observation",
        "pub async fn store_probe_observation",
        "Observation {",
        "store_observation(&obs).await",
    ] {
        assert!(
            observation.contains(expected),
            "inference_observation adapter must contain `{expected}`"
        );
    }
}

#[test]
fn kernel_inference_router_delegates_node_operations_to_cynic_inference_application() {
    let router = read_repo_file("crates/cynic-kernel/src/api/rest/inference_router.rs");
    assert!(
        router.contains("cynic_inference::application::node_runtime_ops")
            && router.contains("start_dog_runtime")
            && router.contains("remediate_dog_runtime")
            && router.contains("probe_model_runtime"),
        "kernel inference router must call cynic-inference node runtime operations"
    );

    for forbidden in [
        "cynic_node::inference_node_ops",
        "attempt_node_start",
        "attempt_node_recovery",
        "probe_node_model",
    ] {
        assert!(
            !router.contains(forbidden),
            "kernel inference router must not call low-level node ops directly: {forbidden}"
        );
    }

    let node_ops = read_repo_file("crates/cynic-node/src/inference_node_ops.rs");
    let node_lib = read_repo_file("crates/cynic-node/src/lib.rs");
    let inference_ops =
        read_repo_file("crates/cynic-inference/src/application/node_runtime_ops.rs");
    assert!(
        node_lib.contains("pub mod inference_node_ops;")
            && node_ops.contains("pub use cynic_inference::application::node_runtime_ops"),
        "cynic-node must expose a compatibility inference operation application surface"
    );
    assert!(
        inference_ops.contains("pub async fn start_dog_runtime")
            && inference_ops.contains("pub async fn remediate_dog_runtime")
            && inference_ops.contains("pub async fn probe_model_runtime"),
        "cynic-inference should own node runtime operation application primitives"
    );
}

#[test]
fn kernel_inference_router_delegates_routing_projection_to_inference() {
    let router = read_repo_file("crates/cynic-kernel/src/api/rest/inference_router.rs");
    assert!(
        router.contains("project_route_response")
            && router.contains("project_candidate_views")
            && router.contains("storage_fleet_stats_to_domain"),
        "kernel inference router must delegate route/candidate projection to cynic-inference"
    );

    for forbidden in [
        "select_best_candidate",
        "rank_healthy_candidates",
        "pub struct InferenceRouteResponse",
        "\"success_rate\": candidate.success_rate",
    ] {
        assert!(
            !router.contains(forbidden),
            "kernel inference router must not own routing projection rule: {forbidden}"
        );
    }

    let routing = read_repo_file("crates/cynic-inference/src/application/fleet_routing.rs");
    for expected in [
        "pub struct InferenceRouteDecisionView",
        "pub struct InferenceCandidateView",
        "pub fn project_route_response",
        "pub fn project_candidate_views",
    ] {
        assert!(
            routing.contains(expected),
            "cynic-inference fleet_routing must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_inference_proxy_delegates_backend_selection_to_inference() {
    let proxy = read_repo_file("crates/cynic-kernel/src/api/rest/inference_proxy.rs");
    assert!(
        proxy.contains("cynic_inference::application::proxy_routing")
            && proxy.contains("proxy.select("),
        "kernel inference proxy must delegate backend selection to cynic-inference"
    );

    for forbidden in [
        "pub struct ProxyTargets",
        "struct ProxyTarget",
        "BTreeMap",
        "from_fleet_meta",
        "first_with_tools",
        "fn first(",
    ] {
        assert!(
            !proxy.contains(forbidden),
            "kernel inference proxy must not own proxy routing rule: {forbidden}"
        );
    }

    let proxy_routing = read_repo_file("crates/cynic-inference/src/application/proxy_routing.rs");
    for expected in [
        "pub struct ProxyTargets",
        "pub struct ProxyTarget",
        "pub enum ProxyTargetSelectionError",
        "pub fn from_fleet_meta",
        "pub fn select",
        "fn first_with_tools",
    ] {
        assert!(
            proxy_routing.contains(expected),
            "cynic-inference proxy_routing must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_fleet_probe_delegates_model_identity_verification_to_node() {
    let fleet_probe = read_repo_file("crates/cynic-kernel/src/infra/probes/fleet.rs");
    assert!(
        fleet_probe.contains("cynic_node::verify")
            && fleet_probe.contains("check_identity_with_bearer"),
        "kernel fleet probe must delegate model identity observation to cynic-node"
    );

    for forbidden in [
        ".pointer(\"/data/0/id\")",
        "actual_lower.contains",
        "expected_lower.contains",
    ] {
        assert!(
            !fleet_probe.contains(forbidden),
            "kernel fleet probe must not re-own model identity verification rule: {forbidden}"
        );
    }

    let node_verify = read_repo_file("crates/cynic-node/src/verify.rs");
    for expected in [
        "pub enum IdentityResult",
        "pub async fn check_identity",
        "pub async fn check_identity_with_bearer",
        "pub fn model_identity_matches",
    ] {
        assert!(
            node_verify.contains(expected),
            "cynic-node verify must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_fleet_probe_delegates_context_observation_to_node() {
    let fleet_probe = read_repo_file("crates/cynic-kernel/src/infra/probes/fleet.rs");
    assert!(
        fleet_probe.contains("check_context_with_bearer")
            && fleet_probe.contains("ContextResult::Observed"),
        "kernel fleet probe must delegate backend context observation to cynic-node"
    );

    for forbidden in [
        ".pointer(\"/default_generation_settings/n_ctx\")",
        "resp.json::<serde_json::Value>().await",
    ] {
        assert!(
            !fleet_probe.contains(forbidden),
            "kernel fleet probe must not re-own backend context observation: {forbidden}"
        );
    }

    let node_verify = read_repo_file("crates/cynic-node/src/verify.rs");
    for expected in [
        "pub enum ContextResult",
        "pub async fn check_context",
        "pub async fn check_context_with_bearer",
        "\"/default_generation_settings/n_ctx\"",
    ] {
        assert!(
            node_verify.contains(expected),
            "cynic-node verify must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_boot_health_probe_delegates_backend_verification_to_node() {
    let health_probe = read_repo_file("crates/cynic-kernel/src/backends/health_probe.rs");
    assert!(
        health_probe.contains("cynic_node::verify")
            && health_probe.contains("check_health(")
            && health_probe.contains("check_identity_with_bearer("),
        "kernel boot health probe must delegate backend verification to cynic-node"
    );

    for forbidden in [
        ".get(health_url).send().await",
        ".get(&models_url).send().await",
        "resp.text().await",
        "body.contains(&cfg.model)",
    ] {
        assert!(
            !health_probe.contains(forbidden),
            "kernel boot health probe must not re-own backend verification: {forbidden}"
        );
    }
}

#[test]
fn kernel_rug_risk_delegates_token_intelligence_to_cynic_intel() {
    let rug_risk = read_repo_file("crates/cynic-kernel/src/domain/rug_risk.rs");
    assert!(
        rug_risk.contains("cynic_intel::rug_risk")
            && rug_risk.contains("impl RugRiskTokenView for TokenData"),
        "kernel rug_risk must be a compatibility adapter over cynic-intel"
    );

    for forbidden in [
        "very_young",
        "young_low_holders",
        "mint_authority_active\"",
        "freeze_authority_active\"",
        "extreme_concentration",
        "thin_liquidity",
    ] {
        assert!(
            !rug_risk.contains(forbidden),
            "kernel rug_risk must not own token intelligence rule: {forbidden}"
        );
    }

    let intel_rug_risk = read_repo_file("crates/cynic-intel/src/rug_risk.rs");
    for expected in [
        "pub trait RugRiskTokenView",
        "pub struct RugRiskAssessment",
        "pub fn assess_rug_risk",
        "pub fn format_rug_risk_section",
        "very_young",
        "thin_liquidity",
    ] {
        assert!(
            intel_rug_risk.contains(expected),
            "cynic-intel rug_risk must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_token_postprocessor_delegates_heuristics_to_cynic_intel() {
    let postprocessor = read_repo_file("crates/cynic-kernel/src/domain/token_postprocessor.rs");
    assert!(
        postprocessor.contains("cynic_intel::token_postprocessor")
            && postprocessor.contains("postprocess_token_scores")
            && postprocessor.contains("infer_token_metadata_from_context"),
        "kernel token postprocessor must delegate heuristics to cynic-intel"
    );

    for forbidden in [
        "fn apply_age_caps",
        "fn invert_inverted_signals",
        "fn apply_class_awareness",
        "fn apply_signal_weighting",
        "fn apply_prior_calibration",
        "fn classify_token",
        "fn parse_metric",
    ] {
        assert!(
            !postprocessor.contains(forbidden),
            "kernel token postprocessor must not own heuristic rule: {forbidden}"
        );
    }

    let intel_postprocessor = read_repo_file("crates/cynic-intel/src/token_postprocessor.rs");
    for expected in [
        "pub enum TokenClass",
        "pub struct TokenMetadata",
        "pub struct AxiomScoreVector",
        "pub struct TokenQScore",
        "pub fn infer_token_metadata_from_context",
        "pub fn postprocess_token_scores",
        "fn apply_age_caps",
    ] {
        assert!(
            intel_postprocessor.contains(expected),
            "cynic-intel token_postprocessor must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_kscore_delegates_heuristics_to_cynic_intel() {
    let kscore = read_repo_file("crates/cynic-kernel/src/domain/kscore.rs");
    assert!(
        kscore.contains("cynic_intel::kscore")
            && kscore.contains("WalletBehaviorAdapter")
            && kscore.contains("map_holder_class_to_intel"),
        "kernel kscore module must be a compatibility adapter over cynic-intel"
    );

    for forbidden in [
        "(acc / ext.max(1.0) / 2.0).tanh()",
        "holder_norm =",
        "accumulators = acc as u32",
        "powf(config.weight_diamond_hands)",
        "median_hold_hours: Option<f64>",
    ] {
        assert!(
            !kscore.contains(forbidden),
            "kernel kscore should not own K-Score math: {forbidden}"
        );
    }

    let intel_kscore = read_repo_file("crates/cynic-intel/src/kscore.rs");
    for expected in [
        "pub enum HolderClass",
        "pub struct KScore",
        "pub trait KScoreConfigView",
        "pub trait WalletBehaviorView",
        "pub fn classify_wallet",
        "pub fn compute_kscore",
    ] {
        assert!(
            intel_kscore.contains(expected),
            "cynic-intel kscore must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_rug_prefilter_delegates_gate_to_cynic_intel() {
    let rug_prefilter = read_repo_file("crates/cynic-kernel/src/domain/rug_prefilter.rs");
    assert!(
        rug_prefilter.contains("cynic_intel::rug_prefilter")
            && rug_prefilter
                .contains("impl cynic_intel::rug_prefilter::RugPrefilterTokenView for TokenData")
            && rug_prefilter.contains("pub fn rug_prefilter(token: &TokenData)"),
        "kernel rug_prefilter must be a compatibility adapter over cynic-intel"
    );

    for forbidden in [
        "holder_count == 0",
        "liquidity_usd < 100.0",
        "trajectory_class == Some(\"DEAD\")",
        "score >= 3",
        "lp_status != \"burned\"",
    ] {
        assert!(
            !rug_prefilter.contains(forbidden),
            "kernel rug_prefilter should not own pre-filter logic: {forbidden}"
        );
    }

    let intel = read_repo_file("crates/cynic-intel/src/rug_prefilter.rs");
    for expected in [
        "pub enum PreFilterResult",
        "pub trait RugPrefilterTokenView",
        "pub fn rug_prefilter",
        "is_none_or(|p| p == 0.0)",
    ] {
        assert!(
            intel.contains(expected),
            "cynic-intel rug_prefilter must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_wallet_judgment_delegates_deterministic_logic_to_cynic_intel() {
    let wallet_judgment = read_repo_file("crates/cynic-kernel/src/domain/wallet_judgment/mod.rs");
    assert!(
        wallet_judgment.contains("cynic_intel::wallet_judgment")
            && wallet_judgment.contains("pub use cynic_intel::wallet_judgment::WalletProfile")
            && wallet_judgment.contains("pub fn deterministic_dog(profile: &WalletProfile)"),
        "kernel wallet_judgment must be a compatibility adapter over cynic-intel"
    );

    for forbidden in [
        "profile.games_completed < 5",
        "engine_penalty",
        "wallet_age_days < 3",
        "let mut q_score",
        "suspicious_cluster || profile.replay_risk",
    ] {
        assert!(
            !wallet_judgment.contains(forbidden),
            "kernel wallet_judgment should not own deterministic logic: {forbidden}"
        );
    }

    let intel = read_repo_file("crates/cynic-intel/src/wallet_judgment.rs");
    for expected in [
        "pub struct WalletProfile",
        "pub enum DeterministicDogKind",
        "pub struct WalletAxiomScores",
        "pub fn deterministic_dog",
        "profile.games_completed < 5",
    ] {
        assert!(
            intel.contains(expected),
            "cynic-intel wallet_judgment must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_wallet_enrichment_delegates_behavioral_scoring_to_cynic_intel() {
    let wallet_enrichment = read_repo_file("crates/cynic-kernel/src/domain/wallet_enrichment.rs");
    assert!(
        wallet_enrichment.contains("cynic_intel::wallet_enrichment")
            && wallet_enrichment.contains("impl WalletBehaviorView for WalletBehavioralProfile")
            && wallet_enrichment.contains("score_wallet_behavior(profile)"),
        "kernel wallet enrichment must delegate behavioral scoring to cynic-intel"
    );

    for forbidden in [
        "if profile.flip_count",
        "profile.swaps_per_day > 20.0",
        "profile.pump_fun_ratio > 0.9",
        "q_capped",
        "PHI_INV",
    ] {
        assert!(
            !wallet_enrichment.contains(forbidden),
            "kernel wallet enrichment must not own behavioral scoring rule: {forbidden}"
        );
    }

    let intel_wallet = read_repo_file("crates/cynic-intel/src/wallet_enrichment.rs");
    for expected in [
        "pub trait WalletBehaviorView",
        "pub enum BehavioralVerdictKind",
        "pub fn behavioral_dog",
        "profile.swaps_per_day() > 20.0",
        "profile.pump_fun_ratio() > 0.9",
    ] {
        assert!(
            intel_wallet.contains(expected),
            "cynic-intel wallet_enrichment must expose `{expected}`"
        );
    }
}

#[test]
fn kernel_fleet_and_mesh_rest_handlers_delegate_runtime_rules_to_node() {
    let event = read_repo_file("crates/cynic-kernel/src/api/rest/event.rs");
    let mesh = read_repo_file("crates/cynic-kernel/src/api/rest/mesh.rs");

    assert!(
        event.contains("cynic_node::fleet_observability")
            && event.contains("validate_event_identity")
            && event.contains("project_fleet_stats"),
        "kernel event REST handler must delegate fleet rules to cynic-node"
    );
    for forbidden in [
        "tool.is_empty()",
        "tool.len() > 64",
        "node.is_empty()",
        "node.len() > 64",
        "success_rate) {",
        "failure_reason.as_str()",
        ".map(|(node, avg_latency",
    ] {
        assert!(
            !event.contains(forbidden),
            "kernel event REST handler must not re-own fleet runtime rules: {forbidden}"
        );
    }

    assert!(
        mesh.contains("cynic_node::mesh_registry::validate_mesh_key_publication"),
        "kernel mesh REST handler must delegate mesh publication validation to cynic-node"
    );
    for forbidden in [
        "mesh_ip.is_empty()",
        "mesh_ip.len() > 20",
        "public_key.is_empty()",
        "public_key.len() > 128",
    ] {
        assert!(
            !mesh.contains(forbidden),
            "kernel mesh REST handler must not re-own mesh validation rules: {forbidden}"
        );
    }
}

#[test]
fn kernel_runtime_project_root_resolution_stays_host_owned() {
    for path in [
        "crates/cynic-kernel/src/api/rest/dispatch.rs",
        "crates/cynic-kernel/src/api/mcp/proxy.rs",
        "crates/cynic-kernel/src/bootstrap/services.rs",
        "crates/cynic-kernel/src/bootstrap/entry.rs",
        "crates/cynic-kernel/src/infra/config/mod.rs",
        "crates/cynic-kernel/src/infra/domain_prompts.rs",
    ] {
        let content = read_repo_file(path);
        assert!(
            !content.contains("cynic_host::repo_access::current_project_root")
                && !content.contains("use cynic_host::repo_access::current_project_root"),
            "{path} must not resolve host project roots from repo-access path discovery"
        );
        assert!(
            !content.contains("resolve_runtime_project_root_or_cwd_fallback")
                && !content.contains("resolve_runtime_project_root_path")
                && !content.contains("RuntimeProjectRootMode"),
            "{path} must not own project-root resolution policy"
        );
    }
}
