//! Sovereign Remote Compute Runner Enrollment Use Case.
//!
//! Generates bounded PreAuthKey via FleetMeshPort (Headscale) to admit external GPU/CPU runners.

use cynic_protocol::compute::{
    RemoteRunnerEnrollmentPlan, RemoteRunnerSpec, plan_remote_runner_enrollment,
};
use cynic_types::fleet_mesh::{FleetMeshPort, PreAuthKeySpec};
use serde::{Deserialize, Serialize};

pub const REMOTE_RUNNER_ENROLLMENT_REPORT_SCHEMA: &str =
    "cynic.node.remote_runner_enrollment_report.v1";

/// Structured report emitted after enrolling a remote compute runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteRunnerEnrollmentReport {
    pub schema: String,
    pub node_name: String,
    pub preauth_key: String,
    pub expiration: String,
    pub acl_tags: Vec<String>,
    pub status: String,
}

/// Executes remote runner enrollment use case.
pub async fn execute_remote_runner_enrollment<P: FleetMeshPort>(
    spec: &RemoteRunnerSpec,
    mesh_port: &P,
) -> Result<RemoteRunnerEnrollmentReport, String> {
    // 1. Pure decision plan
    let plan: RemoteRunnerEnrollmentPlan = plan_remote_runner_enrollment(spec)?;

    // 2. Headscale PreAuthKey generation
    let preauth_spec = PreAuthKeySpec {
        user: plan.user,
        reusable: plan.reusable,
        ephemeral: true,
        expiration_seconds: Some(plan.expiration_seconds),
        acl_tags: plan.acl_tags.clone(),
    };

    let key = mesh_port
        .create_preauth_key(&preauth_spec)
        .await
        .map_err(|e| format!("fleet_mesh_key_generation_failed: {e}"))?;

    Ok(RemoteRunnerEnrollmentReport {
        schema: REMOTE_RUNNER_ENROLLMENT_REPORT_SCHEMA.to_string(),
        node_name: plan.node_name,
        preauth_key: key.key,
        expiration: key.expiration,
        acl_tags: plan.acl_tags,
        status: "enrolled_pending_connect".to_string(),
    })
}
