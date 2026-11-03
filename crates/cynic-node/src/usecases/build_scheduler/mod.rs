//! Thin build-scheduler surface for cynic-node.
//!
//! The scheduler implementation lives in `cynic-build`. This module adapts
//! node-local config and concrete adapters to the ports expected by the
//! hoisted code, then re-exports the public surface so internal callers
//! keep using `crate::usecases::build_scheduler::...`.

use std::time::Duration;

use cynic_build::build_scheduler::{
    BuildArtefactReclaimer, BuildSchedulerConfig, CapacityAssessment, CapacityAssessmentPort,
    CpuMeterPort, NodeControlConfig, ReleasePublicationConfig, RuntimeArtifactTransportPort,
    WorkspaceLayoutResolver, build_run_loop, release_publication::publish_with_ports,
};
use cynic_build::ports::{
    BuildWorkloadGrantSignerPort, ReleaseArtifactPublicationPort, ReleaseBasketPackagingPort,
    ReleaseBasketSigningPort,
};
use cynic_host::adapters::{
    cpu_usage_meter::CpuUsageMeter, local_build_artefact::LocalBuildArtefactAdapter,
    local_capacity::LocalCapacityAdapter, local_evidence_store::LocalContentAddressedEvidenceStore,
};
use cynic_build::ports::LoadPolicy;
use cynic_protocol::evidence_store_port::ContentAddressedEvidenceStorePort;
use cynic_protocol::{RetentionExecutor, RetentionObserver, RetentionPlanner};
use tokio_util::sync::CancellationToken;

use crate::{
    adapters::{
        ed25519_build_workload_grant::Ed25519BuildWorkloadGrantSigner,
        http_artifact_transport::HttpArtifactTransport,
        local_agent_workload_store::LocalAgentWorkloadStore,
        registry::RegistryAgentWorkspaceLayoutAdapter,
    },
    config::{BuildSchedulerConfig as NodeBuildSchedulerConfig, NodeControlConfig as NodeNodeControlConfig, ReleasePublicationConfig as NodeReleasePublicationConfig},
};
use cynic_agent::workspace::ports::AgentWorkspaceLayoutPort;

impl WorkspaceLayoutResolver for RegistryAgentWorkspaceLayoutAdapter {
    fn resolve(
        &self,
        repository_id: &str,
        workload_id: &str,
        node_id: &str,
    ) -> Result<cynic_agent::workspace::ports::AgentWorkspaceLayout, cynic_agent::workspace::ports::AgentWorkspacePortError> {
        AgentWorkspaceLayoutPort::resolve(self, repository_id, workload_id, node_id)
    }
}

// --- Port implementations that bridge node-local adapters to cynic-build ports ---

pub struct NodeCapacityAssessment;

impl CapacityAssessmentPort for NodeCapacityAssessment {
    fn assess(&self, policy: &LoadPolicy) -> CapacityAssessment {
        let adapter = LocalCapacityAdapter::new();
        let host_policy = cynic_host::capacity::LoadPolicy::with_legacy_defaults(&policy.domain);
        match adapter.assess_capacity(&host_policy) {
            Ok(assessment) => CapacityAssessment {
                state: match assessment.state {
                    cynic_host::capacity::LoadTier::Headroom => cynic_build::ports::LoadTier::Headroom,
                    cynic_host::capacity::LoadTier::Normal => cynic_build::ports::LoadTier::Normal,
                    cynic_host::capacity::LoadTier::Saturated => cynic_build::ports::LoadTier::Saturated,
                },
                residual_unknowns: assessment.residual_unknowns,
            },
            Err(_) => CapacityAssessment {
                state: cynic_build::ports::LoadTier::Normal,
                residual_unknowns: Vec::new(),
            },
        }
    }
}

pub struct NodeCpuMeter;

impl CpuMeterPort for NodeCpuMeter {
    fn mark(&self) -> cynic_build::ports::CpuSample {
        cynic_build::ports::CpuSample
    }
    fn cpu_ms_between(&self, _start: &cynic_build::ports::CpuSample, _end: &cynic_build::ports::CpuSample) -> Option<u64> {
        None
    }
}

pub struct NodeBuildArtefactReclaimer;

impl BuildArtefactReclaimer for NodeBuildArtefactReclaimer {
    fn reclaim(&self) {
        let adapter = LocalBuildArtefactAdapter;
        let Ok(observation) = adapter.observe() else {
            eprintln!("cynic: build artefact observation failed; reclaim skipped");
            return;
        };
        let policy = cynic_protocol::retention::default_build_artefact_policy();
        let plan = adapter.plan(&observation, &policy);
        if plan.items.is_empty() {
            return;
        }
        match adapter.execute(&plan) {
            Ok(result) if result.success => {
                eprintln!(
                    "cynic: reclaimed {} build artefact(s), {} bytes",
                    result.items_executed.len(),
                    plan.total_reclaimable_bytes
                );
            }
            Ok(result) => {
                eprintln!(
                    "cynic: reclaim incomplete: {} of {} artefact(s) reclaimed",
                    result.items_executed.iter().filter(|item| item.success).count(),
                    plan.items.len()
                );
            }
            Err(error) => {
                eprintln!("cynic: reclaim failed: {error}");
            }
        }
    }
}

// --- Config mapping ---

impl From<NodeNodeControlConfig> for NodeControlConfig {
    fn from(value: NodeNodeControlConfig) -> Self {
        Self {
            node_id: value.node_id,
            signing_key_path: value.signing_key_path,
            agent_workload_state_root: value.agent_workload_state_root,
            runtime_artifact_max_bytes: value.runtime_artifact_max_bytes,
            runtime_artifact_timeout_secs: value.runtime_artifact_timeout_secs,
            build_scheduler: value.build_scheduler.map(Into::into),
        }
    }
}

impl From<NodeBuildSchedulerConfig> for BuildSchedulerConfig {
    fn from(value: NodeBuildSchedulerConfig) -> Self {
        Self {
            registry_path: value.registry_path,
            repository_id: value.repository_id,
            transport_uri: value.transport_uri,
            interval_secs: value.interval_secs,
            claim_ttl_secs: value.claim_ttl_secs,
            command_timeout_secs: value.command_timeout_secs,
            sandbox_bwrap: value.sandbox_bwrap,
            shared_target_root: value.shared_target_root,
            stable_workspace_root: value.stable_workspace_root,
            release_publication: value.release_publication.map(Into::into),
            build_jobs: value.build_jobs,
            max_rss_cap_mb: value.max_rss_cap_mb,
            sccache: value.sccache.map(|s| cynic_build::build_scheduler::SccacheConfig {
                cache_dir: s.cache_dir,
                binary: s.binary,
                max_size_bytes: s.max_size_bytes,
            }),
            sccache_measured_hit_rate: value.sccache_measured_hit_rate,
            toolchain_bin_dir: value.toolchain_bin_dir,
        }
    }
}

impl From<NodeReleasePublicationConfig> for ReleasePublicationConfig {
    fn from(value: NodeReleasePublicationConfig) -> Self {
        Self {
            artifact_base_url: value.artifact_base_url,
            authentication_kind: value.authentication_kind,
            work_root: value.work_root,
            signing_key_path: value.signing_key_path,
            credential_path: value.credential_path,
            signer_identity: value.signer_identity,
            signature_namespace: value.signature_namespace,
            max_bytes: value.max_bytes,
            timeout_secs: value.timeout_secs,
        }
    }
}

// --- Public run_loop wrapper ---

pub async fn run_loop(
    control: NodeNodeControlConfig,
    shutdown: CancellationToken,
) {
    let capacity = NodeCapacityAssessment;
    let cpu = NodeCpuMeter;
    let reclaimer = NodeBuildArtefactReclaimer;

    let signer = match Ed25519BuildWorkloadGrantSigner::load_hex_file(&control.signing_key_path) {
        Ok(s) => s,
        Err(_) => {
            tracing::error!("build_grant_machine_signer_unavailable");
            return;
        }
    };

    let artifact_transport = match HttpArtifactTransport::new(
        control.runtime_artifact_inbox_root.clone(),
        control.runtime_artifact_max_bytes,
        Duration::from_secs(control.runtime_artifact_timeout_secs),
    ) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "artifact transport creation failed");
            return;
        }
    };

    let layout = RegistryAgentWorkspaceLayoutAdapter::new(
        control.build_scheduler.as_ref().map(|s| s.registry_path.clone()).unwrap_or_default(),
        &control.agent_workload_state_root,
    );

    let evidence = LocalContentAddressedEvidenceStore::new(
        control.agent_workload_state_root.join("evidence"),
    );

    let packaging = match control.build_scheduler.as_ref() {
        Some(s) => match cynic_build::LocalReleaseBasketPackaging::new(
            s.release_publication.as_ref().map(|r| r.work_root.clone()).unwrap_or_default(),
            s.release_publication.as_ref().map(|r| r.max_bytes).unwrap_or(0),
        ) {
            Ok(p) => Some(p),
            Err(e) => {
                tracing::error!(error = %e, "release basket packaging creation failed");
                None
            }
        },
        None => None,
    };

    let release_signing = match control.build_scheduler.as_ref() {
        Some(s) => match s.release_publication.as_ref() {
            Some(r) => match cynic_build::SshReleaseBasketSigning::new(
                r.signing_key_path.clone(),
                r.signer_identity.clone(),
                r.signature_namespace.clone(),
                r.max_bytes,
            ) {
                Ok(s) => Some(s),
                Err(e) => {
                    tracing::error!(error = %e, "release basket signing creation failed");
                    None
                }
            },
            None => None,
        },
        None => None,
    };

    let release_store = match control.build_scheduler.as_ref() {
        Some(s) => match s.release_publication.as_ref() {
            Some(r) => match HttpArtifactTransport::new_release(
                &r.artifact_base_url,
                &r.authentication_kind,
                r.credential_path.clone(),
                r.max_bytes,
                Duration::from_secs(r.timeout_secs),
            ) {
                Ok(s) => Some(s),
                Err(e) => {
                    tracing::error!(error = %e, "release artifact store creation failed");
                    None
                }
            },
            None => None,
        },
        None => None,
    };

    let packaging = match packaging {
        Some(p) => p,
        None => return,
    };
    let release_signing = match release_signing {
        Some(s) => s,
        None => return,
    };
    let release_store = match release_store {
        Some(s) => s,
        None => return,
    };

    build_run_loop(
        control.into(),
        shutdown,
        &capacity,
        &cpu,
        &evidence,
        &layout,
        &signer,
        &artifact_transport,
        &packaging,
        &release_signing,
        &release_store,
        &reclaimer,
    )
    .await;
}
