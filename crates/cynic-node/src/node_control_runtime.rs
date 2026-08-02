use std::process::ExitCode;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::adapters::http_node_control::HttpNodeControlAdapter;
use crate::adapters::local_node_control_observer::LocalNodeControlObserver;
use crate::adapters::local_node_enrollment::LocalNodeEnrollmentReader;
use crate::{config, shutdown::install_signal_handler, usecases};

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
    let control_shutdown = shutdown.clone();
    let control_adapter = adapter.clone();
    let control_config = control.clone();
    let control_task = tokio::spawn(async move {
        usecases::node_control_plane_uc::run_node_control_loop(
            &control_config.node_id,
            Duration::from_secs(control_config.interval_secs),
            &control_adapter,
            &observer,
            &control_adapter,
            &control_shutdown,
        )
        .await;
    });
    let renewal_task = tokio::spawn(
        usecases::node_enrollment_lifecycle_uc::run_enrollment_renewal_loop(
            adapter,
            enrollment_reader,
            control,
            shutdown,
        ),
    );
    Ok(vec![control_task, renewal_task])
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
