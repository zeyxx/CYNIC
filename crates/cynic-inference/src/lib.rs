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

// Convenience re-exports for downstream crates (CLI, MCP).
pub use adapters::discovery::model_registry::InMemoryModelRegistry;
pub use adapters::hf::HfCliAdapter;
pub use application::model_process_pool::{ActivePoolSlot, ModelPoolReport, ModelProcessPool};

pub use domain::debate::{DebateParticipant, DebateRole, DebateSession, DebateTurn};
pub use domain::supervision::SupervisionPolicy;
pub use domain::backend_health;
pub use domain::backend_health::{
    Backend, BackendHealth, BackendId, DeclaredCapabilities, MeasuredCapabilities,
};
pub use domain::hardware_profile::MemoryBudget;
pub use domain::inference::{InferencePhase, LatencyBreakdown};
pub use math::tensor_geometry::AttentionType;
pub use ports::inference_routing_port::{
    DefaultInferenceRouter, InferenceProxyPort, InferenceRoutingPort,
};
pub use application::failure_gallery::{FailureCategory, FailureGallery, FailureGalleryEntry};
pub use application::model_selection::{
    AdaptiveModelSelector, EscalationAction, EscalationSignal, EscalationTier, ModelEmpiricalStats,
    resolve_escalation_action, select_model,
};

pub use adapters::inference::hermes_process_supervisor::read_hermes_api_key;
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
pub use application::bench_basket_launcher::{build_basket_from_ledger, launch_basket};
pub use application::multi_agent_service::MultiAgentService;

pub use application::backend_verification::{
    ContextResult, IdentityResult, check_context, check_context_with_bearer, check_health,
    check_health_with_bearer, check_identity, check_identity_with_bearer, model_identity_matches,
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

pub use application::hf_discovery::{HfDiscoveryService, HfDownloadCommand};
pub use application::local_runtime_service::{LocalRuntimeService, LocalRuntimeServiceAny};
pub use application::model_store_service::ModelStoreService;
pub use application::model_sync_pipeline::ModelSyncPipeline;
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
    ensure_profiles_from_vault, forge_profile_from_benchmark_report, pin_preferred_alias,
    resolve_start_alias, select_preferred_alias,
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
pub use domain::gguf_meta::{
    GgufMeta, default_supported_arches_for_llama_b9444, preflight_gguf_for_engine, read_gguf_meta,
};
pub use domain::hardware_profile::HardwareProfile;

pub use domain::hf_catalog::{ModelCategory, category_catalog, short_alias_from_repo};
pub use domain::inference_queue::{InferenceQueue, InferenceQueueMap, QueuePermit};
pub use domain::local_runtime::{
    LocalModelInventory, LocalModelProfile, LocalRuntimeMutationReport, LocalRuntimeStatus,
};
pub use domain::model_layout::resolve_chat_state_dir;
pub use domain::model_layout::{ModelLayout, ModelLayoutReport};
pub use domain::model_registry::{InferenceEngine, ModelBackend, ModelCapability, ModelHealth};

pub use domain::model_store::{ModelArtifact, ModelStoreReport};
pub use domain::principal_paths::{PathAuthority, PrincipalPaths};
pub use domain::runtime_plan::{EnginePlugins, RuntimeLaunchPlan};
pub use domain::slot_semaphore::{SlotPermit, SlotPriority, SlotSemaphore, SlotSemaphoreMap};
pub use domain::slot_tracker::{BackendSlots, SlotHealthEntry, SlotTracker, build_backend_slots};
pub use ports::chat_port::ChatPort;
pub use ports::chat_session_port::{ChatSessionStore, SessionStoreError};

pub use ports::hf_hub_port::{
    HfCliStatus, HfDownloadReport, HfDownloadRequest, HfHubError, HfHubPort, HfModelHit,
    HfSearchReport, HfSearchRequest,
};
pub use ports::local_runtime_port::{
    LocalInferenceRuntimePort, LocalModelInventoryPort, LocalRuntimeError,
};
pub use ports::model_publisher_port::{ModelPublisherPort, NormalisedModel, PublishResult};
pub use ports::model_store_port::ModelStorePort;

// Additional re-exports required by cynic-cli / cynic-mcp surfaces.
pub use adapters::empirical_store::{DEFAULT_EMPIRICAL_LEDGER_PATH, FsEmpiricalStore};
pub use application::inference_gateway::{InferenceGateway, InferenceGatewayConfig};
pub use domain::domain_task_queue::{
    DEFAULT_DOMAIN_TASKS_PATH, DomainTaskQueue, DomainTaskQueueError,
};
pub use domain::local_runtime::LocalRuntimeState;
pub use domain::matrix_sweep::empirical_store::{
    ConstraintFilter, ModelEmpiricalScorecard, OptimizationGoal,
};
pub use domain::matrix_sweep::types::HardwareBudget;
pub use math::roofline::assess_roofline;
pub use ports::empirical_store_port::EmpiricalStorePort;
