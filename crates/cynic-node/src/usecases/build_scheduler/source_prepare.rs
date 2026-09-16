use std::time::{SystemTime, UNIX_EPOCH};

use cynic_lifecycle::ArgvRunner;
use cynic_build::{
    BUILD_SOURCE_REFERENCE_SCHEMA,
    BuildSourceReference,
};
use cynic_protocol::{
    SOURCE_MATERIALIZATION_FORMAT,
    SOURCE_MATERIALIZATION_REQUEST_SCHEMA,
    SOURCE_REPOSITORY_BOOTSTRAP_REQUEST_SCHEMA,
    SourceMaterializationRequest,
    SourceRepositoryBootstrapDecision,
    SourceRepositoryBootstrapRequest,
    digest_json,
    plan_source_materialization,
    reconcile_source_repository_bootstrap,
};

use crate::{
    adapters::{
        local_source_materialization::LocalSourceMaterialization,
        local_source_repository_bootstrap::LocalSourceRepositoryBootstrap,
        source_snapshot_store::snapshot_path,
    },
    config::{BuildSchedulerConfig, NodeControlConfig},
    ports::{
        agent_workspace_port::AgentWorkspaceLayout,
        runtime_artifact_transport_port::RuntimeArtifactTransportPort,
        source_materialization_port::SourceMaterializationPort,
        source_repository_bootstrap_port::SourceRepositoryBootstrapPort,
    },
};

pub(super) async fn prepare_build_source<R: ArgvRunner, T: RuntimeArtifactTransportPort>(
    control: &NodeControlConfig,
    scheduler: &BuildSchedulerConfig,
    source: &BuildSourceReference,
    workload_id: &str,
    layout: AgentWorkspaceLayout,
    runner: &R,
    artifact_transport: &T,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<(AgentWorkspaceLayout, Vec<String>), String> {
    if source.schema != BUILD_SOURCE_REFERENCE_SCHEMA {
        return Err("build_source_reference_schema_mismatch".to_string());
    }
    match &source.snapshot {
        Some(snapshot) => {
            prepare_snapshot_source(
                control,
                scheduler,
                source,
                snapshot,
                workload_id,
                layout,
                runner,
                artifact_transport,
                cancellation,
            )
            .await
        }
        None => prepare_live_git_source(control, source, layout, runner, cancellation),
    }
}

fn prepare_live_git_source<R: ArgvRunner>(
    control: &NodeControlConfig,
    source: &BuildSourceReference,
    layout: AgentWorkspaceLayout,
    runner: &R,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<(AgentWorkspaceLayout, Vec<String>), String> {
    let request = SourceRepositoryBootstrapRequest {
        schema: SOURCE_REPOSITORY_BOOTSTRAP_REQUEST_SCHEMA.into(),
        node_id: control.node_id.clone(),
        repository_id: source.repository_id.clone(),
        source: source.transport_uri.clone(),
        revision: source.revision.clone(),
    };
    let adapter = LocalSourceRepositoryBootstrap::new(
        runner,
        &control.agent_workload_state_root,
        "git",
        "id",
    );
    let before = adapter
        .observe(&layout, &request)
        .map_err(|error| error.code)?;
    match reconcile_source_repository_bootstrap(&request, &before) {
        SourceRepositoryBootstrapDecision::Converged => {}
        SourceRepositoryBootstrapDecision::BootstrapRequired
        | SourceRepositoryBootstrapDecision::SyncRequired => {
            if cancellation.is_cancelled() {
                return Err("build_scheduler_cancelled_before_source_sync".to_string());
            }
            adapter
                .apply(&layout, &request)
                .map_err(|error| error.code)?;
            let after = adapter
                .observe(&layout, &request)
                .map_err(|error| error.code)?;
            if !matches!(
                reconcile_source_repository_bootstrap(&request, &after),
                SourceRepositoryBootstrapDecision::Converged
            ) {
                return Err("build_scheduler_source_read_back_not_converged".into());
            }
        }
        SourceRepositoryBootstrapDecision::Blocked { blockers } => {
            return Err(blockers.join(","));
        }
        SourceRepositoryBootstrapDecision::Unknown { unknowns } => {
            return Err(unknowns.join(","));
        }
    }
    Ok((layout, vec![source.authority_ref.clone()]))
}

async fn prepare_snapshot_source<R: ArgvRunner, T: RuntimeArtifactTransportPort>(
    control: &NodeControlConfig,
    scheduler: &BuildSchedulerConfig,
    source: &BuildSourceReference,
    snapshot: &cynic_build::BuildSourceSnapshotReference,
    workload_id: &str,
    mut layout: AgentWorkspaceLayout,
    runner: &R,
    artifact_transport: &T,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<(AgentWorkspaceLayout, Vec<String>), String> {
    let target_relative_path = format!("source/materializations/{workload_id}");
    let request = SourceMaterializationRequest {
        schema: SOURCE_MATERIALIZATION_REQUEST_SCHEMA.to_string(),
        release_id: workload_id.to_string(),
        repository_id: source.repository_id.clone(),
        source_commit: source.revision.clone(),
        source_tree: None,
        snapshot_digest: snapshot.digest.clone(),
        snapshot_format: snapshot.format.clone(),
        target_node_id: control.node_id.clone(),
        target_relative_path: target_relative_path.clone(),
        required_principal: layout.required_principal.clone(),
        retention_until_unix_ms: retention_deadline(scheduler.command_timeout_secs),
    };
    if request.snapshot_format != SOURCE_MATERIALIZATION_FORMAT {
        return Err("build_source_snapshot_format_unsupported".to_string());
    }
    let snapshot_root = control.agent_workload_state_root.join("source/snapshots");
    let adapter = LocalSourceMaterialization::new(
        runner,
        &control.agent_workload_state_root,
        snapshot_root.clone(),
        &control.node_id,
        "build_runner",
    );
    let mut provenance = vec![snapshot.digest.clone(), snapshot.authority_ref.clone()];
    let mut observed = adapter.observe(&request).map_err(|error| error.code)?;
    let initial = plan_source_materialization(&request, &observed, now_unix_ms());
    if initial.status == "blocked" {
        return Err(initial.blockers.join(","));
    }
    if initial.status == "unknown" {
        return Err(initial.unknowns.join(","));
    }
    if initial.status == "planned" && !observed.snapshot_present {
        if cancellation.is_cancelled() {
            return Err("build_scheduler_cancelled_before_snapshot_acquisition".to_string());
        }
        let destination =
            snapshot_path(&snapshot_root, &snapshot.digest).map_err(|error| error.code)?;
        let source = snapshot
            .source
            .as_ref()
            .ok_or_else(|| "build_source_snapshot_recovery_source_missing".to_string())?;
        let receipt = artifact_transport
            .stage(source, &destination, Some(&snapshot.digest))
            .await?;
        if receipt.status != "proved" || receipt.observed_digest != snapshot.digest {
            return Err("build_source_snapshot_acquisition_read_back_failed".to_string());
        }
        provenance.push(receipt.evidence_ref);
        observed = adapter.observe(&request).map_err(|error| error.code)?;
    }
    let acquired = plan_source_materialization(&request, &observed, now_unix_ms());
    if acquired.status == "planned" {
        if cancellation.is_cancelled() {
            return Err("build_scheduler_cancelled_before_snapshot_materialization".to_string());
        }
        adapter.materialize(&request).map_err(|error| error.code)?;
        let after = adapter.observe(&request).map_err(|error| error.code)?;
        let read_back = plan_source_materialization(&request, &after, now_unix_ms());
        if read_back.status != "converged" {
            return Err("build_source_snapshot_materialization_read_back_failed".to_string());
        }
    } else if acquired.status != "converged" {
        return Err(acquired
            .blockers
            .into_iter()
            .chain(acquired.unknowns)
            .collect::<Vec<_>>()
            .join(","));
    }
    layout.repository_root = control
        .agent_workload_state_root
        .join(&target_relative_path);
    layout.evidence_digest = digest_json(&serde_json::json!({
        "layout": layout.evidence_digest,
        "snapshot": snapshot.digest,
        "commit": source.revision,
    }))
    .map_err(|_| "build_source_snapshot_layout_digest_failed".to_string())?;
    Ok((layout, provenance))
}

fn retention_deadline(command_timeout_secs: u64) -> u64 {
    now_unix_ms().saturating_add(
        command_timeout_secs
            .saturating_mul(1_000)
            .saturating_add(3_600_000),
    )
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(u128::from(u64::MAX)) as u64
        })
}
