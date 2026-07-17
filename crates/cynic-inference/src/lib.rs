pub mod adapters;
pub mod application;
pub mod config_loader;
pub mod domain;
pub mod embedding;
pub mod inference;
/// Historical probe module for attaching to Osmantic ODS / OpenAI-compat peers.
/// Prefer `adapters::ods` (documents ODS as external bootstrap framework).
pub mod ods_discovery;
pub mod ports;

// Convenience re-exports for downstream crates (CLI, MCP).
pub use adapters::discovery::model_registry::InMemoryModelRegistry;
pub use adapters::hf::HfCliAdapter;
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
pub use application::chat_endpoint::{
    ChatEndpointResolution, ChatEndpointSource, dns_inference_hostnames,
    inference_config_candidates, resolve_chat_endpoint,
};
pub use application::chat_service::{
    BuiltinToolInvoker, ChatCompletionClient, ChatService, ChatServiceError, ChatTurnReport,
    CompletionOutcome, InferenceReadyReport, MockChatClient, ToolCallRequest, ToolInvoker,
    default_tool_specs,
};
pub use application::hf_discovery::HfDiscoveryService;
pub use application::local_runtime_service::{LocalRuntimeService, LocalRuntimeServiceAny};
pub use application::model_store_service::ModelStoreService;
pub use application::profile_bootstrap::{
    EnsureProfilesReport, ensure_profiles_from_vault, resolve_start_alias, select_preferred_alias,
};
pub use application::vault_consolidate::{
    VaultConsolidateReport, consolidate_vault, discovery_roots,
};
pub use application::model_sync_pipeline::ModelSyncPipeline;
pub use application::runtime_planner::{MaterializeReport, RuntimePlanner};
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
pub use domain::local_runtime::{
    LocalModelInventory, LocalModelProfile, LocalRuntimeMutationReport, LocalRuntimeStatus,
};
pub use domain::model_layout::resolve_chat_state_dir;
pub use domain::model_layout::{ModelLayout, ModelLayoutReport};
pub use domain::model_registry::{InferenceEngine, ModelBackend, ModelCapability, ModelHealth};
pub use domain::model_store::{ModelArtifact, ModelStoreReport};
pub use domain::principal_paths::{PathAuthority, PrincipalPaths};
pub use domain::runtime_plan::{EnginePlugins, RuntimeLaunchPlan};
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
