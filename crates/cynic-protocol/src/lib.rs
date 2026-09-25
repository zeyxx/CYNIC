<<<<<<< HEAD
pub mod agent_citizenship;
pub mod agent_connection;
pub mod agent_coordination;
pub mod agent_coordination_continuity;
pub mod agent_execution_provenance;
pub mod agent_host_sandbox;
pub mod agent_isolation;
pub mod agent_mutation_gates;
pub mod agent_recursion;
pub mod agent_review;
pub mod agent_session;
pub mod agentic_tool_call;
pub mod allowance;

pub mod argv_allow;
pub mod artifact_release;
pub mod audit;
pub mod autoresearch;
pub mod autoresearch_port;
pub mod bootstrap_federation;
pub mod branch_graph;
pub mod breakglass;
pub mod builder_rbac;
pub mod capability_budget;
pub mod capability_lease;
pub mod capability_responsibility;
pub mod capacity;
pub mod cleanup;
pub mod constitutional_court;
pub mod constitutional_law;
pub mod convergence;
pub mod coord;
pub mod coordination_authority;
pub mod coordination_lease;
pub mod coordination_runtime;

pub mod coordination_score;
pub mod credential_lease;
pub mod cycle_transition;
pub mod deployment_epoch;
pub mod detection;
pub mod development;
pub mod digest;
pub mod discovery;
pub mod domain_ops;
pub mod domain_tasks;
pub mod emergence;
pub mod emergence_gallery;
pub mod endpoint;
pub mod energy_ledger;
pub mod error_catalog;
pub mod eval;
pub use eval::dog;
pub use eval::dog_health;
pub use eval::hygiene as eval_hygiene;
pub mod evidence;
pub mod evidence_journal;
pub mod evidence_projection;
pub mod federation;
pub mod federation_authz;
pub mod federation_envelopes;
pub mod federation_membership;
pub mod fleet_discovery;
pub mod fleet_heartbeat;
pub mod framework;
pub mod freshness;
pub mod hardening;
pub mod health_signal;
pub mod host_substrate;
pub use host_substrate::cpu;
pub use host_substrate::disk;
pub use host_substrate::fleet_disk;
pub use host_substrate::hypervisor;
pub use host_substrate::memory;
pub use host_substrate::proxmox;
pub use host_substrate::vm_disk;
pub mod inference_matrix;
pub mod integrity;
pub mod intent;
pub mod judge_patch;
pub mod judgment;
pub mod kyon;
pub mod llm_port;
pub mod local_autonomy;
pub mod machine_enrollment;
pub mod machine_identity;
pub mod mcp_projection;
pub mod metabolic_invoice;
pub mod node_activation;
pub mod node_control;
pub mod node_enrollment;
pub mod node_execution_mode;
pub mod node_identity;
pub mod observation_record;
pub mod observe;
pub mod one_shot_capability;
pub mod organic_opportunity;
pub mod outcomes;
pub mod pattern_gc;
pub mod peer_resolver;
pub mod phi;
pub mod policy;
pub mod pre_os;
pub mod provider;
pub mod publication_policy;
pub mod queue;
pub mod registry_claim_audit;
pub mod residual_unknown;
pub mod resource_allocation;
pub mod runtime;
pub mod runtime_basket;
pub mod runtime_compatibility;
pub mod runtime_placement;
pub mod runtime_planes;
pub mod runtime_seed;
pub mod runtime_takeover;
pub mod runtime_takeover_activation;
pub mod sanitize;
pub mod sbom;
pub mod secret_reference;
pub mod sensory;
pub mod supervision;
pub mod surface_registry;
pub mod task_admission;
pub mod task_pipeline;
pub mod tasks;
pub mod temporal;
pub mod transport_security;
pub mod window_journal;
pub use eval::wisdom;
pub mod workload_identity;

pub mod exports;
pub use exports::*;
=======
pub mod agent_recursion;
pub mod anchor_client;
pub mod artifact_release;
pub mod audit;
pub mod build;
pub mod build_proof;
pub mod detection;
pub mod coord;
pub mod convergence;
pub mod coordination_lease;
pub mod development;
pub mod discovery;
pub mod endpoint;
pub mod evidence;
pub mod forgejo;
pub mod framework;
pub mod hardening;
pub mod integrity;
pub mod network;
pub mod observe;
pub mod outcomes;
pub mod policy;
pub mod pre_os;
pub mod provider;
pub mod runtime;
pub mod source_control;
pub mod supervision;
pub mod temporal;
pub mod surface_registry;
pub mod tasks;

pub use temporal::{
    Epoch, TemporalProof, TemporalSnapshot, EpochCertificate, TemporalContract,
    TEMPORAL_EPOCH_CERTIFICATE_SCHEMA, TEMPORAL_SNAPSHOT_SCHEMA, TEMPORAL_PROOF_SCHEMA,
};
pub mod transport_security;

pub use agent_recursion::{
    AGENT_COMPLETION_ENVELOPE_SCHEMA, AGENT_DELEGATION_ENVELOPE_SCHEMA,
    AGENT_RECURSION_ADMISSION_SCHEMA, AgentBudget, AgentCapabilityGrant, AgentCompletionEnvelope,
    AgentCompletionStatus, AgentDelegationEnvelope, AgentEvidenceDependency, AgentEvidenceLineage,
    AgentEvidenceRef, AgentEvidenceRelation, AgentRecursionAdmission, AgentRecursionDecision,
    AgentRecursionViolation, admit_child_delegation, admit_completion,
    classify_agent_evidence_dependency, relate_agent_evidence, validate_evidence_refs,
};
pub use audit::{
    AUDIT_REPORT_SCHEMA_V2, AuditCategory, AuditReportV2, AuditRuleSpec, AuditSeverity,
    AuditSpan, AuditSummary, AuditViolation, BoundedMutationContract,
};
pub use detection::{
    DETECTION_REPORT_SCHEMA_V1, DetectedCapability, HardwareCapabilities, InstallProvenance,
    ObservationMethod, PrincipalOwnershipObservation, ResidualRisk, SupervisorKind,
};
pub use artifact_release::{
    RUNTIME_ARTIFACT_RELEASE_EVENT_SCHEMA, RUNTIME_ARTIFACT_RELEASE_REPORT_SCHEMA,
    RUNTIME_ARTIFACT_RELEASE_SPEC_SCHEMA, RuntimeArtifactReleaseAction,
    RuntimeArtifactReleaseEvent, RuntimeArtifactReleaseReport, RuntimeArtifactReleaseSpec,
    RuntimeArtifactSpec, RuntimeBuildEvidence, ServiceIdentityEvidence,
};
pub use build::{BUILD_INPUT_ADMISSION_SCHEMA, BuildInputAdmission, admit_build_input};
pub use coord::{coord_claim_path, coord_register_path, coord_release_path, coord_who_path};
pub use convergence::{
    CONVERGENCE_MATRIX_SCHEMA, ConvergenceCell, ConvergenceMatrixReport, ConvergenceStatus,
    assess_convergence_matrix, merge_convergence_matrices,
};
pub use coordination_lease::{
    BatchClaimResult, LeaseAction, LeaseConflict, LeaseGrant, LeaseGuard, LeaseMutationOutcome,
    LeaseRejectionCode, TargetClaimOutcome,
};
pub use development::{
    DEVELOPMENT_SURFACE_MANIFEST_SCHEMA, DevelopmentSurface, DevelopmentSurfaceManifest,
    development_surface_manifest,
};
pub use discovery::{
    DISCOVERY_PROMOTION_ADMISSION_SCHEMA, DiscoveryPromotionAdmission, admit_candidate_declaration,
};
pub use discovery::{
    DiscoveryState, TARGET_DISCOVERY_REPORT_SCHEMA, TargetCandidate, TargetDiscoveryReport,
};
pub use endpoint::{
    KernelEndpointError, KernelEndpointPolicy, auth_header_value, kernel_url,
    normalize_kernel_base_url,
};
pub use evidence::{EVIDENCE_ENVELOPE_SCHEMA, EvidenceEnvelope, build_evidence_envelope};
pub use forgejo::{
    FORGEJO_PLAN_SCHEMA, ForgejoActionKind, ForgejoDesiredState, ForgejoObservedState, ForgejoPlan,
    ForgejoPlannedAction, plan_forgejo_convergence,
};
pub use framework::{
    FRAMEWORK_SPEC_SCHEMA, FrameworkIdentity, FrameworkLayer, FrameworkPrimitiveFamily,
    FrameworkPrinciple, FrameworkRule, FrameworkSpec, FrameworkUseCase, framework_spec,
};
pub use integrity::{canonical_json_bytes, digest_json};
pub use network::{
    NETWORK_MESH_PLAN_SCHEMA, NetworkMeshAclRule, NetworkMeshDnsRecord, NetworkMeshPeer,
    NetworkMeshPlan, NetworkMeshRoute, NetworkMeshService,
};
pub use observe::OBSERVE_PATH;
pub use source_control::{
    SOURCE_ADMISSION_REPORT_SCHEMA, SOURCE_CHANGE_PROPOSAL_SCHEMA, SourceChangeAdmission,
    SourceChangeProposal, SourceMatterClass, SourcePromotionStage, SourceSnapshot,
    admit_source_change, admit_source_stage,
};
pub use outcomes::{
    KPI_SCORE_SCHEMA, KpiScoreReport, OUTCOME_SPEC_SCHEMA, Outcome, OutcomeScore, OutcomeSpec,
    SuccessKpi, outcome_spec, score_outcomes,
};
pub use policy::{
    EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization, MutationContract,
    POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority, PolicyDecision,
    PolicyIntentRef, PolicyScope, PolicySetRef, PolicyViolation, RequiredProof,
};
pub use pre_os::{
    PRE_OS_ARTIFACT_MANIFEST_SCHEMA, PRE_OS_ATTESTATION_SCHEMA, PRE_OS_EXECUTION_PLAN_SCHEMA,
    PRE_OS_PLATFORM_OBSERVATION_SCHEMA, PRE_OS_VERIFICATION_SCHEMA, PreOsArtifactManifest,
    PreOsArtifactSpec, PreOsAttestation, PreOsControlKind, PreOsExecutionPlan, PreOsOutcome,
    PreOsPhase, PreOsPlatformObservation, PreOsVerification, PreOsVerificationCheck,
    verify_pre_os_attestation,
};
pub use provider::{
    PROVIDER_CONTROL_CAPABILITY_SCHEMA, PROVIDER_MACHINE_IDENTITY_SCHEMA,
    ProviderControlCapability, ProviderIdentityKind, ProviderMachineIdentity,
};
pub use runtime::{
    RUNTIME_EXECUTION_PLAN_SCHEMA, RUNTIME_EXECUTION_SUITE_SCHEMA, RUNTIME_PROOF_BUNDLE_SCHEMA,
    RUNTIME_PROOF_PERSISTENCE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeExecutionCommand,
    RuntimeExecutionLayer, RuntimeExecutionPlan, RuntimeExecutionSuite,
    RuntimePlannedExecutionLayer, RuntimeProof, RuntimeProofBundle, RuntimeProofEvidence,
    RuntimeProofPersistenceRecord, RuntimeRegistrySummary, RuntimeSharedResponsibility,
    build_runtime_execution_plan_from_gates, runtime_execution_layers,
    runtime_initial_observed_gates, runtime_missing_gates, runtime_proof_persistence_record,
    runtime_shared_responsibility,
};
pub use source_control::SourceAdmissionReport;
pub use supervision::{
    SERVICE_SUPERVISOR_OBSERVATION_EVIDENCE_ID, SERVICE_SUPERVISOR_OBSERVATION_SCHEMA,
    ServiceSupervisorObservation, ServiceSupervisorProviderAttempt,
    ServiceSupervisorUnitObservation, service_supervisor_status,
};
pub use surface_registry::{CanonicalCliSurface, SURFACE_REGISTRY_SCHEMA, SurfaceRegistryDocument};
pub use tasks::{
    AGENT_TASKS_COMPLETED_PATH, AGENT_TASKS_PATH, agent_task_claim_path, agent_task_claim_payload,
    agent_task_collection_path, agent_task_listing_params, agent_task_listing_query,
    agent_task_result_path, agent_task_result_payload, task_id_in_agent_task_listings,
};
pub use transport_security::{
    NETWORK_PATH_EVIDENCE_SCHEMA, NetworkPathEvidence, SecurityEvidenceState,
    TRANSPORT_SECURITY_REPORT_SCHEMA, TransportScope, TransportSecurityReport,
    TransportSecuritySurface,
};
>>>>>>> github/main
