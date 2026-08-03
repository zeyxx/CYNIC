use std::process::ExitCode;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::adapters::http_node_control::HttpNodeControlAdapter;
use crate::adapters::http_runtime_artifact_transport::HttpRuntimeArtifactTransport;
use crate::adapters::local_agent_workload_completion_receipt::LocalAgentWorkloadCompletionReceipt;
use crate::adapters::local_agent_workload_delivery_lease::LocalAgentWorkloadDeliveryLeaseStore;
use crate::adapters::local_agent_workload_store::LocalAgentWorkloadStore;
use crate::adapters::local_node_control_observer::LocalNodeControlObserver;
use crate::adapters::local_node_enrollment::LocalNodeEnrollmentReader;
use crate::adapters::local_node_identity::LocalNodeIdentity;
use crate::adapters::local_runtime_seed_actuation::LocalRuntimeSeedActuation;
use crate::ports::node_identity_port::NodeIdentityPort;
use crate::{config, shutdown::install_signal_handler, usecases};
use cynic_host::adapters::local_evidence_store::LocalContentAddressedEvidenceStore;

fn load_or_create_incarnation(control: &config::NodeControlConfig) -> Result<String, String> {
    let desired = cynic_protocol::NodeIdentityDesiredState {
        schema: cynic_protocol::NODE_IDENTITY_DESIRED_STATE_SCHEMA.to_string(),
        node_id: format!("{}-incarnation", control.node_id),
        signing_key_path: control
            .agent_workload_state_root
            .join("identity/node-incarnation.key")
            .display()
            .to_string(),
    };
    let adapter = LocalNodeIdentity;
    let mut observed = adapter.observe(&desired);
    if observed.key_state == cynic_protocol::SecurityEvidenceState::Absent {
        adapter.generate(&desired)?;
        observed = adapter.observe(&desired);
    }
    if observed.key_state != cynic_protocol::SecurityEvidenceState::Proven {
        return Err(format!(
            "node incarnation is not proven: {}",
            observed.unknowns.join(",")
        ));
    }
    observed
        .machine_identity_digest
        .ok_or_else(|| "node incarnation digest missing after read-back".to_string())
}

pub(crate) async fn start_tasks(
    control: config::NodeControlConfig,
    shutdown: CancellationToken,
) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
    let adapter = HttpNodeControlAdapter::new(
        control.kernel_url.clone(),
        control.signing_key_path.clone(),
        Duration::from_secs(control.timeout_secs),
    )
    .map_err(|error| format!("node-control configuration is invalid: {error}"))?;
    let enrollment_reader = LocalNodeEnrollmentReader::new(
        control.enrollment_request_path.clone(),
        control.enrollment_grant_path.clone(),
        control.enrollment_authority_public_key_path.clone(),
        control.signing_key_path.clone(),
    );
    let mut enrollment = enrollment_reader.read_back(&control.node_id);
    let mut enrollment_issued = false;
    if enrollment.state != cynic_protocol::SecurityEvidenceState::Proven
        && !control.enrollment_grant_path.is_file()
    {
        match adapter
            .bootstrap_enrollment(
                &control.node_id,
                &control.enrollment_role,
                &control.enrollment_seed_id,
                control.enrollment_requested_capabilities.clone(),
                control.enrollment_request_ttl_secs,
                &control.enrollment_token_path,
                &control.enrollment_request_path,
                &control.enrollment_grant_path,
            )
            .await
        {
            Ok(()) => {
                enrollment = enrollment_reader.read_back(&control.node_id);
                enrollment_issued = true;
            }
            Err(error) => enrollment
                .unknowns
                .push(format!("automatic_enrollment_failed:{error}")),
        }
    }
    if enrollment_issued && enrollment.state == cynic_protocol::SecurityEvidenceState::Proven {
        if let Err(error) = std::fs::remove_file(&control.enrollment_token_path) {
            enrollment
                .unknowns
                .push(format!("consumed_enrollment_token_cleanup_failed:{error}"));
        }
    }
    let observer = LocalNodeControlObserver::new(
        control.node_id.clone(),
        enrollment_reader.clone(),
        control.observed_capabilities.clone(),
        control.observed_services.clone(),
    );
    let enrollment_proven = enrollment.state == cynic_protocol::SecurityEvidenceState::Proven;
    let build_scheduler_authorized = enrollment_proven
        && cynic_protocol::BUILD_SCHEDULER_RUNTIME_CAPABILITIES
            .iter()
            .all(|capability| {
                enrollment
                    .granted_capabilities
                    .iter()
                    .any(|granted| granted == capability)
            });
    // A one-time enrollment token is bootstrap input only.  Once the node is
    // proven enrolled, the scheduler must be fenced by the durable grant;
    // accepting the token here would allow a stale/replayed bootstrap secret
    // to masquerade as steady-state build authority.
    let build_scheduler_inputs_ready = control.signing_key_path.is_file()
        && control.enrollment_grant_path.is_file()
        && control
            .build_scheduler
            .as_ref()
            .is_none_or(|scheduler| scheduler.registry_path.is_file());
    let artifact_transport = HttpRuntimeArtifactTransport::new(
        control.runtime_artifact_inbox_root.clone(),
        control.runtime_artifact_max_bytes,
        Duration::from_secs(control.runtime_artifact_timeout_secs),
    )?;
    let release_actuation = LocalRuntimeSeedActuation;
    let control_shutdown = shutdown.clone();
    let control_adapter = adapter.clone();
    let control_config = control.clone();
    let delivery_adapter = control_adapter.clone();
    let delivery_node_id = control_config.node_id.clone();
    let delivery_interval = Duration::from_secs(control_config.interval_secs);
    let delivery_store = LocalAgentWorkloadStore::new(
        control_config
            .agent_workload_state_root
            .join("agent-scheduler/workloads"),
    );
    let delivery_leases = LocalAgentWorkloadDeliveryLeaseStore::new(
        control_config
            .agent_workload_state_root
            .join("agent-scheduler/delivery-leases"),
    );
    let delivery_consumer_instance_id = load_or_create_incarnation(&control_config)?;
    let delivery_lease_ttl_ms = control_config
        .workload_delivery_lease_ttl_secs
        .checked_mul(1_000)
        .ok_or_else(|| "workload delivery lease TTL overflows milliseconds".to_string())?;
    let delivery_evidence_store = LocalContentAddressedEvidenceStore::new(
        control_config.agent_workload_state_root.join("evidence"),
    );
    let delivery_receipts = LocalAgentWorkloadCompletionReceipt::new(
        control_config
            .agent_workload_state_root
            .join("agent-scheduler/completion-receipts"),
    );
    let delivery_shutdown = control_shutdown.clone();
    let control_task = tokio::spawn(async move {
        usecases::node_control_plane_uc::run_node_control_loop(
            &control_config.node_id,
            Duration::from_secs(control_config.interval_secs),
            &control_adapter,
            &observer,
            &control_adapter,
            &artifact_transport,
            &release_actuation,
            &control_shutdown,
        )
        .await;
    });
    let scheduler_control = control.clone();
    let scheduler_shutdown = shutdown.clone();
    let renewal_task = tokio::spawn(
        crate::node_enrollment_lifecycle_uc::run_enrollment_renewal_loop(
            adapter,
            enrollment_reader,
            control,
            shutdown,
        ),
    );
    let mut tasks = vec![control_task, renewal_task];
    if enrollment_proven {
        tasks.push(tokio::spawn(async move {
            usecases::agent_workload_delivery_uc::run_delivery_loop(
                &delivery_node_id,
                &delivery_consumer_instance_id,
                delivery_lease_ttl_ms,
                delivery_interval,
                &delivery_adapter,
                &delivery_store,
                &delivery_leases,
                &delivery_evidence_store,
                &delivery_receipts,
                &delivery_shutdown,
            )
            .await;
        }));
    } else {
        tracing::warn!(
            node_id = %scheduler_control.node_id,
            "workload delivery remains inactive until machine enrollment is proved"
        );
    }
    if scheduler_control.build_scheduler.is_some()
        && build_scheduler_authorized
        && build_scheduler_inputs_ready
    {
        tasks.push(tokio::spawn(crate::build_scheduler::run_loop(
            scheduler_control,
            scheduler_shutdown,
        )));
    } else if scheduler_control.build_scheduler.is_some() {
        tracing::warn!(
            node_id = %scheduler_control.node_id,
            enrollment_proven,
            build_scheduler_authorized,
            build_scheduler_inputs_ready,
            "build scheduler remains inactive until enrollment, capabilities, and runtime inputs are proved"
        );
    }
    Ok(tasks)
}

pub(crate) async fn run(config_path: &str, check: bool) -> ExitCode {
    let control = match config::load_node_control(config_path) {
        Ok(control) => control,
        Err(error) => {
            if check {
                println!(
                    "{}",
                    serde_json::json!({
                        "schema": "cynic.node_control_config_check.v1",
                        "status": "blocked",
                        "config_path": config_path,
                        "unknowns": [error]
                    })
                );
                return ExitCode::FAILURE;
            }
            tracing::error!("node-control config error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let node_id = control.node_id.clone();
    let role = control.enrollment_role.clone();
    if check {
        let has_signing_key = control.signing_key_path.is_file();
        let has_enrollment_grant = control.enrollment_grant_path.is_file();
        let has_enrollment_token = control.enrollment_token_path.is_file();
        let has_enrollment_authority_public_key =
            control.enrollment_authority_public_key_path.is_file();
        let enrollment = LocalNodeEnrollmentReader::new(
            control.enrollment_request_path.clone(),
            control.enrollment_grant_path.clone(),
            control.enrollment_authority_public_key_path.clone(),
            control.signing_key_path.clone(),
        )
        .read_back(&node_id);
        let enrollment_proven = enrollment.state == cynic_protocol::SecurityEvidenceState::Proven;
        let scheduler_capabilities_proven = enrollment_proven
            && cynic_protocol::BUILD_SCHEDULER_RUNTIME_CAPABILITIES
                .iter()
                .all(|capability| {
                    enrollment
                        .granted_capabilities
                        .iter()
                        .any(|granted| granted == capability)
                });
        let mut missing = Vec::new();
        if !has_signing_key {
            missing.push("node_signing_key");
        }
        if !has_enrollment_authority_public_key {
            missing.push("enrollment_authority_public_key");
        }
        if !has_enrollment_grant && !has_enrollment_token {
            missing.push("enrollment_grant_or_one_time_token");
        }
        let build_scheduler_configured = control.build_scheduler.is_some();
        let build_scheduler_registry_present = control
            .build_scheduler
            .as_ref()
            .is_none_or(|scheduler| scheduler.registry_path.is_file());
        let build_scheduler_inputs_present = !build_scheduler_configured
            || (has_signing_key && has_enrollment_grant && build_scheduler_registry_present);
        let build_scheduler_activation_ready = !build_scheduler_configured
            || (build_scheduler_inputs_present && scheduler_capabilities_proven);
        if build_scheduler_configured && !has_enrollment_grant {
            missing.push("build_scheduler_durable_enrollment_grant");
        }
        if !build_scheduler_registry_present {
            missing.push("build_scheduler_runtime_registry");
        }
        if build_scheduler_configured && !enrollment_proven {
            missing.push("build_scheduler_enrollment_read_back");
        }
        if build_scheduler_configured && enrollment_proven && !scheduler_capabilities_proven {
            missing.push("build_scheduler_capability_read_back");
        }
        let status = if missing.is_empty() {
            "ready"
        } else {
            "blocked"
        };
        println!(
            "{}",
            serde_json::json!({
                "schema": "cynic.node_control_config_check.v1",
                "status": status,
                "config_path": config_path,
                "node_id": node_id,
                "role": role,
                "kernel_url": control.kernel_url,
                "has_signing_key": has_signing_key,
                "has_enrollment_grant": has_enrollment_grant,
                "has_enrollment_token": has_enrollment_token,
                "has_enrollment_authority_public_key": has_enrollment_authority_public_key,
                "build_scheduler_configured": build_scheduler_configured,
                "build_scheduler_registry_present": build_scheduler_registry_present,
                "build_scheduler_inputs_present": build_scheduler_inputs_present,
                "build_scheduler_enrollment_proven": enrollment_proven,
                "build_scheduler_capabilities_proven": scheduler_capabilities_proven,
                "build_scheduler_activation_ready": build_scheduler_activation_ready,
                "enrollment_read_back_unknowns": enrollment.unknowns,
                "runtime_artifact_inbox_root": control.runtime_artifact_inbox_root,
                "runtime_artifact_max_bytes": control.runtime_artifact_max_bytes,
                "runtime_artifact_timeout_secs": control.runtime_artifact_timeout_secs,
                "missing_requirements": missing
            })
        );
        return if status == "ready" {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    let shutdown = CancellationToken::new();
    install_signal_handler(shutdown.clone());
    let tasks = match start_tasks(control, shutdown.clone()).await {
        Ok(tasks) => tasks,
        Err(error) => {
            tracing::error!("{error}");
            return ExitCode::FAILURE;
        }
    };
    tracing::info!(%node_id, %role, "node-control runtime started");
    shutdown.cancelled().await;
    for task in tasks {
        if let Err(error) = task.await {
            tracing::warn!(%error, "node-control task join failed");
        }
    }
    ExitCode::SUCCESS
}
