pub mod adapters;
pub mod application;
pub mod config_loader;
pub mod domain;
pub mod embedding;
pub mod inference;
pub mod math;
/// Historical probe module for attaching to Osmantic ODS / OpenAI-compat peers.
/// Prefer `adapters::ods` (documents ODS as external bootstrap framework).
pub mod ods_discovery;
pub mod ports;
pub mod progress;
pub mod vector_memory;

// Convenience re-exports for downstream crates (CLI, MCP).
pub use adapters::discovery::model_registry::InMemoryModelRegistry;
pub use adapters::hf::HfCliAdapter;
pub use adapters::inference::harness_executor::HttpHarnessExecutor;
pub use adapters::inference::hermes_process_supervisor::{
    ProcessHermesGateway, read_hermes_api_key,
};
pub use adapters::inference::hermes_run_executor::{
    HermesReadinessReport, HermesRunEvent, HermesRunExecutor, HermesRuntimeObservation,
};
pub use adapters::inference::http_chat_client::HttpOpenAiChatClient;
pub use adapters::local::{
    FsChatSessionStore, FsLocalModelInventory, FsModelStore, InferenceSupervisorKind,
    ProcessLocalRuntime, SystemdUserLocalRuntime,
};
pub use adapters::ods::{
    DiscoveredService, DiscoveryReport, ODS_DISCOVERY_SCHEMA, ODS_SURFACE, ODS_UPSTREAM,
    discover_ods_services, run_sync_pipeline, sync_discovered_backends,
};
pub use adapters::publishers::{
    inference_toml::InferenceTomlPublisher, null::NullPublisher, open_webui::OpenWebUiPublisher,
};
pub use application::agent_harness::{
    AgentHarnessReport, BENCHMARK_BASELINE_ASSESSMENT_SCHEMA, BENCHMARK_BASELINE_SCHEMA,
    BENCHMARK_MATRIX_SCHEMA, BENCHMARK_SCHEMA, BenchmarkBaseline, BenchmarkBaselineAssessment,
    BenchmarkPlan, BenchmarkPolicySpec, BenchmarkReport, BenchmarkSummary, CoordinationPort,
    CoordinationReceipt, CycleEnvelope, HarnessExecutor, HarnessObserver, HarnessSample,
    InferenceReply, MemorySample, ModelParameters, ModelSpec, RuntimeKind, RuntimeLayer,
    RuntimePlanSummary, ScenarioPlan, ScenarioSpec, StopDecision, TaskSpec, Thresholds,
    aggregate_reports, assess_baseline, canonicalize_hermes_runtime_id, encode_report,
    evaluate_sample, generate_plan_from_policy, load_benchmark_policy,
    run_scenario_with_coordination,
};
pub use application::backend_verification::{
    ContextResult, IdentityResult, check_context, check_context_with_bearer, check_health,
    check_identity, check_identity_with_bearer, model_identity_matches,
};
pub use application::bench_basket_launcher::{
    BASKET_PLAN_SCHEMA, BasketItem, BasketItemResult, BasketLaunchReport, BasketPlan,
    build_basket_from_ledger, launch_basket,
};
pub use application::chat_endpoint::{
    ChatEndpointResolution, ChatEndpointSource, dns_inference_hostnames,
    inference_config_candidates, probe_live_model_id, probe_live_model_ids, resolve_chat_api_key,
    resolve_chat_endpoint, resolve_chat_model_id,
};
pub use application::chat_service::{
    BuiltinToolInvoker, ChatCompletionClient, ChatService, ChatServiceConfig, ChatServiceError,
    ChatTurnReport, CompletionOutcome, InferenceReadyReport, MockChatClient, ToolCallRequest,
    ToolInvoker, default_tool_specs,
};
pub use application::hermes_runtime_service::{
    HermesGatewayConvergenceReport, HermesGatewayService,
};
pub use application::hf_discovery::{HfDiscoveryService, HfDownloadCommand};
pub use application::local_runtime_service::{LocalRuntimeService, LocalRuntimeServiceAny};
pub use application::model_process_pool::{ActivePoolSlot, ModelPoolReport, ModelProcessPool};
pub use application::model_store_service::ModelStoreService;
pub use application::model_sync_pipeline::ModelSyncPipeline;
pub use application::multi_agent_service::{
    ComparativeMatrixReport, ModelComparisonResult, MultiAgentService,
};
pub use application::node_runtime_ops::{
    DogNodeBinding, DogOperationOutcome, ModelProbeResult, attempt_node_recovery,
    attempt_node_start, dog_to_node, dog_to_node as resolve_dog_node, probe_model_runtime,
    probe_node_model, remediate_dog_runtime, start_dog_runtime,
};
pub use application::peer_attach::{
    PeerAttachOptions, PeerAttachReport, peer_attach, peer_attach_apply, peer_attach_status,
};
pub use application::profile_bootstrap::{
    EnsureProfilesReport, ForgeProfileReport, PreferredChatBind, bind_preferred_chat_model,
    ensure_profiles_from_vault, forge_profile_from_benchmark, forge_profile_from_benchmark_report,
    pin_preferred_alias, resolve_start_alias, select_preferred_alias,
};
pub use application::runtime_planner::{MaterializeReport, RuntimePlanner};
pub use application::vault_consolidate::{
    VaultConsolidateReport, consolidate_vault, discovery_roots,
};
pub use config_loader::load_inference_config_from_path_public;
pub use config_loader::{InferenceConfigError, LoadedInferenceConfig, load_inference_config};
pub use domain::backend_config::InferenceBackendConfig;
pub use domain::chat_session::{
    ChatEvent, ChatEventKind, ChatMessage, SessionMeta, StreamChunk, ToolSpec, messages_from_events,
};
pub use domain::debate::{DebateParticipant, DebateRole, DebateSession, DebateTurn};
pub use domain::gguf_meta::{
    GgufMeta, default_supported_arches_for_llama_b9444, preflight_gguf_for_engine, read_gguf_meta,
};
pub use domain::hardware_profile::HardwareProfile;
pub use domain::hermes_runtime::{
    HermesGatewayMutation, HermesGatewayProfile, HermesGatewayState, HermesGatewayStatus,
};
pub use domain::hf_catalog::{ModelCategory, category_catalog, short_alias_from_repo};
pub use domain::inference_queue::{InferenceQueue, InferenceQueueMap, QueuePermit};
pub use domain::local_runtime::{
    LocalModelInventory, LocalModelProfile, LocalRuntimeMutationReport, LocalRuntimeStatus,
};
pub use domain::matrix_sweep::{
    BenchmarkProvenance, BenchmarkScenario, ConstraintFilter, EmpiricalProfileEntry,
    ModelEmpiricalScorecard, OomSafetyStatus, OptimizationGoal, StreamDynamicsReport,
    aggregate_model_scorecard, can_admit_model_configuration, solve_optimal_configuration,
};
pub use domain::model_layout::resolve_chat_state_dir;
pub use domain::model_layout::{ModelLayout, ModelLayoutReport};
pub use domain::model_registry::{InferenceEngine, ModelBackend, ModelCapability, ModelHealth};
pub use domain::model_retention::{
    ModelRetentionAssessment, ModelRetentionClass, assess_artifact as assess_model_retention,
    assess_report as assess_model_store_retention,
};
pub use domain::model_store::{ModelArtifact, ModelStoreReport};
pub use domain::principal_paths::{PathAuthority, PrincipalPaths};
pub use domain::runtime_plan::{EnginePlugins, RuntimeLaunchPlan};
pub use domain::slot_semaphore::{SlotPermit, SlotPriority, SlotSemaphore, SlotSemaphoreMap};
pub use domain::slot_tracker::{BackendSlots, SlotHealthEntry, SlotTracker, build_backend_slots};
pub use domain::supervision::SupervisionPolicy;
pub use ports::chat_session_port::{ChatSessionStore, SessionStoreError};
pub use ports::hermes_runtime_port::{HermesGatewayRuntimeError, HermesGatewayRuntimePort};
pub use ports::hf_hub_port::{
    HfCliStatus, HfDownloadReport, HfDownloadRequest, HfHubError, HfHubPort, HfModelHit,
    HfSearchReport, HfSearchRequest,
};
pub use ports::local_runtime_port::{
    LocalInferenceRuntimePort, LocalModelInventoryPort, LocalRuntimeError,
};
pub use ports::model_publisher_port::{ModelPublisherPort, NormalisedModel, PublishResult};
pub use ports::model_store_port::ModelStorePort;
pub use vector_memory::{SearchResult, VectorMemoryEntry, VectorMemoryStore};
