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

/// Concatenate every `.rs` file under a repo-relative directory, recursively.
///
/// Boundary assertions must not enumerate individual files: when a surface is
/// split into a module directory (`storage/surreal.rs` -> `storage/surreal/`),
/// a hardcoded list either panics on a missing path or silently stops covering
/// the code that moved. Walking the tree keeps the proof valid across splits.
fn read_repo_tree(relative_dir: &str) -> String {
    fn walk(dir: &std::path::Path, out: &mut String) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        // Deterministic order: the concatenation must not depend on the filesystem.
        paths.sort();
        for path in paths {
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                if let Ok(contents) = fs::read_to_string(&path) {
                    out.push_str(&contents);
                    out.push('\n');
                }
            }
        }
    }

    let root = repo_root().join(relative_dir);
    assert!(
        root.is_dir(),
        "expected a directory to walk at {relative_dir}"
    );
    let mut out = String::new();
    walk(&root, &mut out);
    out
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
        "pub mod bootstrap;",
        "pub mod domain;",
        "pub mod infra;",
        "pub mod introspection;",
        "pub mod json;",
    ] {
        assert!(
            lib_rs.contains(expected),
            "kernel lib.rs should contain `{expected}`"
        );
    }

    // NOTE (2026-09-28 purges): api/ (36 files) and backends/ (3 files)
    // deleted as 0% compiled; none were ever declared below. organ/probe/
    // senses/storage still exist on disk as undeclared orphans (known debt,
    // next purges) — but must never re-enter this manifest. The old test
    // asserted their PRESENCE — aspirational fiction. Lock absence.
    for forbidden in [
        "pub mod api;",
        "pub mod backends;",
        "pub mod organ;",
        "pub mod probe;",
        "pub mod senses;",
        "pub mod storage;",
    ] {
        assert!(
            !lib_rs.contains(forbidden),
            "kernel lib.rs must not re-declare removed god-object surface `{forbidden}`"
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
fn kernel_runtime_orchestrator_stays_an_honest_minimal_supervisor() {
    // NOTE (2026-09-28): the old test asserted bootstrap::server::run_rest_servers
    // + storage wiring inside the orchestrator. bootstrap::server was an orphan
    // (deleted), and the orchestrator has always been a deliberate minimal
    // supervisor: "Full orchestration (storage, API, agents) will be re-enabled
    // as separate crates." The test asserted vapor; the live file is honest.
    let bootstrap_rs = read_repo_file("crates/cynic-kernel/src/bootstrap/runtime_orchestrator.rs");

    for expected in [
        "pub async fn run_runtime",
        "Runtime supervisor starting (minimal mode)",
        "will be re-enabled as separate crates",
    ] {
        assert!(
            bootstrap_rs.contains(expected),
            "kernel runtime orchestrator should contain `{expected}`"
        );
    }

    for forbidden in [
        "bootstrap::server::run_rest_servers",
        "connect_storage_runtime",
    ] {
        assert!(
            !bootstrap_rs.contains(forbidden),
            "kernel runtime orchestrator must not resurrect dead wiring: {forbidden}"
        );
    }
}

#[test]
fn kernel_infra_surfaces_stay_split_by_role() {
    // NOTE (2026-09-28 api/ purge): the api/ + restmod structure blocks died
    // with the 32-file purge of the uncompiled REST surface. What remains
    // pins the live side: infra split, MCP-mode failure closure, task spawns.
    // A standalone cynic-mcp remains the single MCP command surface (no api/
    // left to contradict it).
    let infra_mod = read_repo_file("crates/cynic-kernel/src/infra/mod.rs");
    let bootstrap_entry = read_repo_file("crates/cynic-kernel/src/bootstrap/entry.rs");
    let tasks_mod = read_repo_file("crates/cynic-kernel/src/infra/tasks/mod.rs");

    // NOTE (2026-09-28): the old list demanded `boot;` `health_loop;`
    // `probes;` `tasks;` — none were ever declared by infra/mod.rs, and
    // their directories are now purged. Assert the LIVE manifest exactly,
    // and lock the purged names out.
    for expected in [
        "pub mod config;",
        "pub mod config_loader;",
        "pub mod config_schema;",
        "pub mod config_storage;",
        "pub mod config_types;",
        "pub mod crypto;",
        "pub mod evidence_journal_file;",
        "pub mod node_identity;",
        "pub mod runtime_secret;",
        "pub mod task_health;",
    ] {
        assert!(
            infra_mod.contains(expected),
            "infra/mod.rs should contain `{expected}`"
        );
    }

    for forbidden in [
        "pub mod boot;",
        "pub mod health_loop;",
        "pub mod probes;",
        "pub mod tasks;",
    ] {
        assert!(
            !infra_mod.contains(forbidden),
            "infra/mod.rs must not re-declare purged surface `{forbidden}`"
        );
    }

    assert!(
        bootstrap_entry.contains("blocked_deprecated_surface")
            && bootstrap_entry.contains("standalone cynic-mcp")
            && !bootstrap_entry.contains("CYNIC_API_KEY"),
        "deprecated kernel --mcp mode must fail explicitly without API-key fallback"
    );

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
    // Extraction is complete (4613b710): the kernel facades are gone and the
    // law lives in cynic-host. This guard fails if anyone reintroduces a
    // kernel-owned lifecycle copy, or if the host law disappears.
    for kernel_path in [
        "crates/cynic-kernel/src/domain/lifecycle_activation_evidence.rs",
        "crates/cynic-kernel/src/domain/lifecycle_activation_requests.rs",
        "crates/cynic-kernel/src/domain/lifecycle_diagnostics.rs",
        "crates/cynic-kernel/src/domain/lifecycle_workflow_state.rs",
    ] {
        assert!(
            !repo_root().join(kernel_path).exists(),
            "{kernel_path} must stay deleted; lifecycle law lives in cynic-host"
        );
    }
    for (host_path, expected) in [
        (
            "crates/cynic-host/src/lifecycle/activation_evidence.rs",
            "activation",
        ),
        (
            "crates/cynic-host/src/lifecycle/activation_requests.rs",
            "activation",
        ),
        ("crates/cynic-host/src/lifecycle/diagnostics.rs", "gap"),
        (
            "crates/cynic-host/src/lifecycle/workflow_state.rs",
            "workflow",
        ),
    ] {
        let content = read_repo_file(host_path);
        assert!(
            content.to_lowercase().contains(expected),
            "{host_path} must carry the lifecycle law"
        );
    }
}

#[test]
fn kernel_storage_must_not_own_inference_model_registry() {
    let storage_mod = read_repo_file("crates/cynic-kernel/src/domain/storage/mod.rs");
    // Walk the whole storage surface instead of listing files: `storage/surreal.rs`
    // and `storage/memory.rs` have all been split into module directories.
    let storage_tree = format!(
        "{}\n{}",
        read_repo_tree("crates/cynic-kernel/src/domain/storage"),
        read_repo_tree("crates/cynic-kernel/src/storage"),
    );

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

    // NOTE (2026-09-28): registry types moved domain/model_registry.rs ->
    // infra/model_registry.rs in cynic-inference. Pointer follows the owner.
    let inference_domain = read_repo_file("crates/cynic-inference/src/infra/model_registry.rs");
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
fn workload_identity_stays_fail_closed_in_node_dispatch() {
    // NOTE (2026-09-28 api/ purge): the kernel RBAC middleware
    // (authorize_role_request/resolve_signer) died with the uncompiled REST
    // surface and has NO live successor anywhere — recorded as REST-rebuild
    // backlog, not silently dropped. What remains pins the live side: node
    // dispatch must never regress to unsigned acknowledgment.
    let node_dispatch = read_repo_file("crates/cynic-node/src/websocket/domain_dispatch.rs");

    for forbidden in [
        "Default to Cortex for signed requests",
        "TODO: Implement Ed25519 signature verification",
        "For now, just acknowledge",
        "bearer != api_key",
        "NATIVE_ADAPTER_UNSIGNED_YET",
    ] {
        assert!(
            !node_dispatch.contains(forbidden),
            "workload identity must not regress to `{forbidden}`"
        );
    }
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

    // NOTE (2026-09-28 backends/ purge): backends/ dir deleted (3 orphan
    // files, 0% compiled). Absence of the dir subsumes the old mod-graph
    // assertion; the live owner contract below is unchanged.
    assert!(
        !repo_root()
            .join("crates/cynic-kernel/src/backends")
            .exists(),
        "kernel backends/ must stay deleted; adapters live in domain crates"
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

    let kernel_domain_mod = read_repo_file("crates/cynic-kernel/src/domain/mod.rs");
    assert!(
        !kernel_domain_mod.contains("summarization"),
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
    // NOTE (2026-09-28 api/ purge): the kernel router side died with the
    // uncompiled REST surface. What remains pins the live owner contract.
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
fn kernel_inference_router_delegates_node_operations_to_cynic_inference_application() {
    // NOTE (2026-09-28 api/ purge): kernel router side dead with the REST
    // surface. Live owner contract below.
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
    // NOTE (2026-09-28 api/ purge): kernel router side dead with the REST
    // surface. Live owner contract below.
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
    // NOTE (2026-09-28 api/ purge): kernel proxy side dead with the REST
    // surface. Live owner contract below.
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
    // NOTE (2026-09-28 backends/ purge): health_probe.rs died with the
    // uncompiled backends/ dir (it also failed its own api_key assertion).
    // Backend verification lives in cynic-node::verify (asserted below);
    // the api_key-forwarding gap is REST-rebuild backlog.
    assert!(
        !repo_root()
            .join("crates/cynic-kernel/src/backends/health_probe.rs")
            .exists(),
        "kernel health probe must stay deleted; verification lives in cynic-node::verify"
    );
}

#[test]
fn kernel_rug_risk_facade_stays_extracted_to_cynic_intel() {
    // Extraction proof: the kernel facade is gone; law lives only in cynic-intel.
    // This guard fails if anyone reintroduces a kernel-owned copy.
    assert!(
        !repo_root()
            .join("crates/cynic-kernel/src/domain/rug_risk.rs")
            .exists(),
        "kernel rug_risk facade must stay deleted; law lives in cynic-intel::rug_risk"
    );

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
    // NOTE (2026-09-28): the kernel kscore facade is gone (it was an orphan
    // purging target). Extraction proof is now ABSENCE + live owner, the
    // pattern the rug/wallet facades already use.
    assert!(
        !repo_root()
            .join("crates/cynic-kernel/src/domain/kscore.rs")
            .exists(),
        "kernel kscore facade must stay deleted; law lives in cynic-intel::kscore"
    );

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
fn kernel_rug_prefilter_facade_stays_extracted_to_cynic_intel() {
    // Extraction proof: the kernel facade is gone; law lives only in cynic-intel.
    // This guard fails if anyone reintroduces a kernel-owned copy.
    assert!(
        !repo_root()
            .join("crates/cynic-kernel/src/domain/rug_prefilter.rs")
            .exists(),
        "kernel rug_prefilter facade must stay deleted; law lives in cynic-intel::rug_prefilter"
    );

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
fn kernel_wallet_judgment_facade_stays_extracted_to_cynic_intel() {
    // Extraction proof: the kernel facade is gone; law lives only in cynic-intel.
    // This guard fails if anyone reintroduces a kernel-owned copy.
    assert!(
        !repo_root()
            .join("crates/cynic-kernel/src/domain/wallet_judgment/mod.rs")
            .exists(),
        "kernel wallet_judgment facade must stay deleted; law lives in cynic-intel::wallet_judgment"
    );

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
fn kernel_wallet_enrichment_facade_stays_extracted_to_cynic_intel() {
    // Extraction proof: profile shape, view impl, and scoring law all live in
    // cynic-intel; the kernel facade is gone. This guard fails if anyone
    // reintroduces a kernel-owned copy.
    assert!(
        !repo_root()
            .join("crates/cynic-kernel/src/domain/wallet_enrichment.rs")
            .exists(),
        "kernel wallet_enrichment facade must stay deleted; law lives in cynic-intel::wallet_enrichment"
    );

    let intel_wallet = read_repo_file("crates/cynic-intel/src/wallet_enrichment.rs");
    for expected in [
        "pub trait WalletBehaviorView",
        "pub enum BehavioralVerdictKind",
        "pub struct WalletBehavioralProfile",
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
fn kernel_runtime_project_root_resolution_stays_host_owned() {
    // NOTE (2026-09-28 api/ purge): dispatch.rs died with the REST surface.
    for path in [
        "crates/cynic-kernel/src/bootstrap/services.rs",
        "crates/cynic-kernel/src/bootstrap/entry.rs",
        "crates/cynic-kernel/src/infra/config/mod.rs",
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
