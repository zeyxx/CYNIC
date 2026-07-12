use serde::Serialize;

use crate::bridge;

#[derive(Debug, Clone, Serialize)]
pub struct TransportMatrixReport {
    schema: &'static str,
    status: &'static str,
    transports: Vec<TransportSurface>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransportSurface {
    id: &'static str,
    class: &'static str,
    status: &'static str,
    authority_boundary: &'static str,
    canonical_use: &'static str,
    industrial_grade: &'static str,
    command_surface: &'static str,
    notes: Vec<&'static str>,
}

pub fn build_transport_matrix_report() -> TransportMatrixReport {
    let bridge_status = bridge::bridge_status();
    let host_exec_status = if bridge_status.status == "active" {
        "active"
    } else {
        "missing_activation"
    };

    TransportMatrixReport {
        schema: "cynic.host_transport_matrix.v1",
        status: if host_exec_status == "active" {
            "bounded_transport_ready"
        } else {
            "bootstrap_required"
        },
        transports: vec![
            TransportSurface {
                id: "host_exec",
                class: "local_bounded_privilege_bridge",
                status: host_exec_status,
                authority_boundary: "cynic user -> fixed root helper",
                canonical_use: "local root-owned host bootstrap and bounded OS mutation",
                industrial_grade: "yes",
                command_surface: "cynic bootstrap bridge plan",
                notes: vec![
                    "single helper path and sudoers rule",
                    "no shell escape and no raw package manager/systemctl sudo surface",
                    "proof available through bridge-status and bootstrap-chain",
                ],
            },
            TransportSurface {
                id: "fleet_node_exec",
                class: "remote_bounded_node_exec",
                status: "declared_contract",
                authority_boundary: "kernel/MCP tool -> bounded node action",
                canonical_use: "remote bounded execution on registered nodes",
                industrial_grade: "target",
                command_surface: "fleet_node_exec via cynic-coord MCP",
                notes: vec![
                    "preferred remote transport in registry contracts",
                    "must stay bounded and evidence-producing",
                    "should not degrade into raw ssh command tunneling",
                ],
            },
            TransportSurface {
                id: "ssh",
                class: "bootstrap_breakglass_transport",
                status: "degraded_fallback_only",
                authority_boundary: "operator shell on remote host",
                canonical_use: "initial reachability, emergency diagnostics, repo transport fallback",
                industrial_grade: "no",
                command_surface: "ssh <host>",
                notes: vec![
                    "not acceptable as the canonical runtime mutation plane",
                    "weak evidence boundary compared to bounded helper surfaces",
                    "must shrink as host_exec and fleet_node_exec coverage expands",
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_matrix_classifies_ssh_as_fallback_only() {
        let report = build_transport_matrix_report();
        assert_eq!(report.schema, "cynic.host_transport_matrix.v1");
        assert!(
            report
                .transports
                .iter()
                .any(|item| item.id == "ssh" && item.status == "degraded_fallback_only")
        );
        assert!(
            report
                .transports
                .iter()
                .any(|item| item.id == "host_exec" && item.industrial_grade == "yes")
        );
    }
}
