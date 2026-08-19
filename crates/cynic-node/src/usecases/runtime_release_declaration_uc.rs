//! Node-owned preparation and proof validation for runtime release declaration.

use serde::Serialize;

use crate::ports::runtime_release_declaration_port::RuntimeReleaseDeclarationPort;

#[derive(Debug, Serialize)]
pub struct RuntimeReleaseDeclarationReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutation_attempted: bool,
    pub declaration: cynic_protocol::RuntimeReleaseDeclaration,
    pub kernel_http_status: Option<u16>,
    pub read_back: Option<serde_json::Value>,
    pub read_back_violations: Vec<String>,
    pub unknowns: Vec<String>,
}

pub fn prepare_runtime_release_declaration(
    requested_by: String,
    workload_id: String,
    mut desired_state: cynic_protocol::NodeDesiredState,
) -> Result<cynic_protocol::RuntimeReleaseDeclaration, String> {
    desired_state.policy_digest = cynic_protocol::runtime_release_policy_digest(&desired_state)?;
    Ok(cynic_protocol::RuntimeReleaseDeclaration {
        schema: cynic_protocol::RUNTIME_RELEASE_DECLARATION_SCHEMA.to_string(),
        requested_by,
        workload_id,
        desired_state,
    })
}

pub fn plan_runtime_release_declaration(
    declaration: cynic_protocol::RuntimeReleaseDeclaration,
) -> RuntimeReleaseDeclarationReport {
    RuntimeReleaseDeclarationReport {
        schema: "cynic.runtime_release_declaration_report.v1",
        status: "planned_requires_kernel_admission",
        mutation_attempted: false,
        declaration,
        kernel_http_status: None,
        read_back: None,
        read_back_violations: Vec::new(),
        unknowns: vec!["terminal workload retention and runtime-release admission are verified only by the kernel apply boundary".to_string()],
    }
}

pub async fn apply_runtime_release_declaration(
    port: &impl RuntimeReleaseDeclarationPort,
    declaration: cynic_protocol::RuntimeReleaseDeclaration,
) -> Result<RuntimeReleaseDeclarationReport, String> {
    let read_back = port.declare_runtime_release(&declaration).await?;
    let read_back_violations = if (200..300).contains(&read_back.http_status) {
        cynic_protocol::validate_runtime_release_declaration_read_back(
            &declaration,
            &read_back.body,
        )
    } else {
        vec!["kernel_http_status_not_success".to_string()]
    };
    let status = if read_back_violations.is_empty() {
        "applied_read_back"
    } else {
        "blocked_read_back"
    };
    Ok(RuntimeReleaseDeclarationReport {
        schema: "cynic.runtime_release_declaration_report.v1",
        status,
        mutation_attempted: (200..300).contains(&read_back.http_status),
        declaration,
        kernel_http_status: Some(read_back.http_status),
        read_back: Some(read_back.body),
        read_back_violations,
        unknowns: Vec::new(),
    })
}
