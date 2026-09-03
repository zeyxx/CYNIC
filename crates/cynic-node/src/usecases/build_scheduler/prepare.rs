use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::{
    DurableAgentWorkloadRecord, LeaseGuard, build_source_reference_from_workload,
    consume_agent_workload_action, digest_json,
};

use crate::{
    adapters::{
        ed25519_build_workload_grant::Ed25519BuildWorkloadGrantSigner,
        http_runtime_artifact_transport::HttpRuntimeArtifactTransport,
        local_agent_workload_store::LocalAgentWorkloadStore,
        local_build_workload_grant_store::LocalBuildWorkloadGrantStore,
        local_build_workspace::LocalBuildWorkspace, registry::RegistryAgentWorkspaceLayoutAdapter,
    },
    config::{BuildSchedulerConfig, NodeControlConfig},
    ports::{
        agent_workload_store_port::AgentWorkloadStorePort,
        agent_workspace_port::AgentWorkspaceLayoutPort,
        build_scheduler_port::{BuildExecutionRequest, BuildWorkspacePort, BuildWorkspaceReadBack},
        build_workload_grant_store_port::BuildWorkloadGrantStorePort,
    },
    usecases::build_workload_grant_uc::issue_signed_build_workload_grant,
};

use super::source_prepare::prepare_build_source;

pub(super) struct PreparedBuild {
    pub durable: DurableAgentWorkloadRecord,
    pub workspace: BuildWorkspaceReadBack,
    pub execution: BuildExecutionRequest,
    pub provenance: Vec<String>,
}

pub(super) async fn prepare(
    control: &NodeControlConfig,
    scheduler: &BuildSchedulerConfig,
    store: &LocalAgentWorkloadStore,
    claimed: &DurableAgentWorkloadRecord,
    guard: &LeaseGuard,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<PreparedBuild, String> {
    if cancellation.is_cancelled() {
        return Err("build_scheduler_cancelled_before_prepare".into());
    }
    let source = build_source_reference_from_workload(&claimed.record.envelope)
        .map_err(|blockers| blockers.join(","))?;
    if source.repository_id != scheduler.repository_id {
        return Err("build_scheduler_repository_id_mismatch".into());
    }
    let layout_port = RegistryAgentWorkspaceLayoutAdapter::new(
        &scheduler.registry_path,
        &control.agent_workload_state_root,
    );
    let layout = layout_port
        .resolve(
            &source.repository_id,
            &claimed.record.envelope.workload_id,
            &control.node_id,
        )
        .map_err(|error| error.code)?;
    let desired_working_directory = claimed
        .record
        .envelope
        .action
        .working_directory
        .as_deref()
        .ok_or_else(|| "build_scheduler_working_directory_missing".to_string())?;
    if PathBuf::from(desired_working_directory) != layout.workspace_root {
        return Err("build_scheduler_working_directory_layout_mismatch".into());
    }

    let runner = LocalArgvRunner;
    let snapshot_root = control.agent_workload_state_root.join("source/snapshots");
    let artifact_transport = HttpRuntimeArtifactTransport::new(
        snapshot_root,
        control.runtime_artifact_max_bytes,
        Duration::from_secs(control.runtime_artifact_timeout_secs),
    )?;
    let (source_layout, mut source_provenance) = prepare_build_source(
        control,
        scheduler,
        &source,
        &claimed.record.envelope.workload_id,
        layout,
        &runner,
        &artifact_transport,
        cancellation,
    )
    .await?;
    if cancellation.is_cancelled() {
        return Err("build_scheduler_cancelled_before_workspace".into());
    }
    let workspace = LocalBuildWorkspace::new(&runner, "git")
        .prepare(&source_layout, &source.revision)
        .map_err(|error| error.code)?;

    let signer = Ed25519BuildWorkloadGrantSigner::load_hex_file(&control.signing_key_path)
        .map_err(|_| "build_grant_machine_signer_unavailable".to_string())?;
    if cancellation.is_cancelled() {
        return Err("build_scheduler_cancelled_before_action_reservation".into());
    }
    let (consumed, action_now_ms) =
        reserve_action(store, &claimed.record.envelope.workload_id, guard)?;
    let signed = issue_signed_build_workload_grant(
        &consumed,
        &control.node_id,
        action_now_ms / 1_000,
        &signer,
    )
    .map_err(|blockers| blockers.join(","))?;
    let grant_store = LocalBuildWorkloadGrantStore::new(
        control
            .agent_workload_state_root
            .join("agent-scheduler/build-grants"),
    );
    let grant = grant_store.persist(&signed).map_err(|error| error.code)?;
    let public_key_path = control
        .agent_workload_state_root
        .join("agent-scheduler/build-authority.pub");
    signer
        .persist_public_key(&public_key_path)
        .map_err(|_| "build_grant_public_key_persist_failed".to_string())?;
    let workload_record_path = store.durable_path(&consumed).map_err(|error| error.code)?;
    let verifier = std::env::current_exe()
        .map_err(|_| "build_scheduler_verifier_path_unobservable".to_string())?;
    let mut environment = BTreeMap::new();
    environment.insert(
        "CYNIC_BUILD_ADMISSION_GRANT".into(),
        grant.path.to_string_lossy().into_owned(),
    );
    environment.insert(
        "CYNIC_BUILD_WORKLOAD_RECORD".into(),
        workload_record_path.to_string_lossy().into_owned(),
    );
    environment.insert(
        "CYNIC_BUILD_AUTHORITY_PUBLIC_KEY".into(),
        public_key_path.to_string_lossy().into_owned(),
    );
    environment.insert(
        "CYNIC_BUILD_ADMISSION_VERIFIER".into(),
        verifier.to_string_lossy().into_owned(),
    );
    source_provenance.extend([
        consumed.record_digest.clone(),
        digest_json(&signed).map_err(|_| "build_grant_digest_failed".to_string())?,
        grant.evidence_ref.clone(),
        source_layout.evidence_digest,
    ]);
    Ok(PreparedBuild {
        durable: consumed,
        execution: BuildExecutionRequest {
            argv: claimed.record.envelope.action.argv.clone(),
            working_directory: workspace.workspace_root.clone(),
            environment,
            timeout: Duration::from_secs(scheduler.command_timeout_secs),
        },
        workspace,
        provenance: source_provenance,
    })
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn reserve_action(
    store: &LocalAgentWorkloadStore,
    workload_id: &str,
    guard: &LeaseGuard,
) -> Result<(DurableAgentWorkloadRecord, u64), String> {
    for _ in 0..3 {
        let latest = store
            .read(workload_id)
            .map_err(|error| error.code)?
            .ok_or_else(|| "build_scheduler_workload_disappeared".to_string())?;
        let now_ms = current_unix_ms();
        let consumed = consume_agent_workload_action(&latest.record, guard, now_ms)
            .map_err(|error| error.code)?;
        match store.compare_and_swap(workload_id, Some(latest.revision), &consumed, now_ms) {
            Ok(durable) => return Ok((durable, now_ms)),
            Err(error) if error.code == "workload_store_revision_conflict" => continue,
            Err(error) => return Err(error.code),
        }
    }
    Err("build_scheduler_action_reservation_contended".into())
}
