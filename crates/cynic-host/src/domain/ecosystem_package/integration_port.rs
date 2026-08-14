//! Pure integration observation policy over an injected probe port.

use super::integration_catalog::integration_specs;
use super::integration_types::{
    ConnectionState, IntegrationConnectionObservation, IntegrationDependencyView,
    IntegrationObservation, IntegrationObservationContext, IntegrationPrincipal, IntegrationSpec,
    IntegrationStatusSummary, IntegrationToolingAuthorityView, IntegrationsStatusReport,
    RawIntegrationObservation,
};
use crate::domain::ecosystem_package::{
    observe_python_install_authority, observe_tooling_authority, plan_for_dep, python_cli_policy,
    resolve_binary, resolve_principal_install_layout,
};
use crate::domain::shared::{BoundaryKind, LifecycleProjection, LifecycleStatus};

pub const INTEGRATIONS_STATUS_SCHEMA: &str = "cynic.ecosystem_integrations.status.v1";

pub trait IntegrationObservationPort: Send + Sync {
    fn observe(
        &self,
        spec: &IntegrationSpec,
        context: &IntegrationObservationContext,
    ) -> RawIntegrationObservation;
}

pub fn observe_integrations(
    port: &dyn IntegrationObservationPort,
    context: &IntegrationObservationContext,
) -> IntegrationsStatusReport {
    let integrations = integration_specs()
        .iter()
        .map(|spec| effective_observation(spec, port.observe(spec, context)))
        .collect::<Vec<_>>();

    let summary = IntegrationStatusSummary {
        total: integrations.len(),
        ready: count_status(&integrations, "ready"),
        missing: count_status(&integrations, "missing"),
        principal_mismatch: count_status(&integrations, "principal_mismatch"),
        connection_required: count_status(&integrations, "connection_required"),
    };
    let dependency_tooling_authorities = integrations
        .iter()
        .filter_map(|integration| {
            let dependency = integration.dependency.as_ref()?;
            let authority = dependency.tooling_authority.as_ref()?;
            Some(IntegrationToolingAuthorityView {
                integration_id: Some(integration.id.clone()),
                dependency_id: Some(dependency.id.clone()),
                family: authority.family,
                observed_provenance: authority.observed_provenance,
                expected_provenance: authority.expected_provenance,
                status: authority.status.clone(),
                observed_payload_provenance: authority.observed_payload_provenance.clone(),
                expected_payload_provenance: authority.expected_payload_provenance.clone(),
                canonical_update_command: authority.canonical_update_command.clone(),
                update_authority_note: authority.update_authority_note.clone(),
                normalizable_now: authority.normalizable_now,
                requires_uv: authority.requires_uv,
                requires_runtime_principal: authority.requires_runtime_principal,
                can_auto_apply: authority.can_auto_apply,
            })
        })
        .collect::<Vec<_>>();
    let status = if summary.principal_mismatch > 0 || summary.connection_required > 0 {
        "degraded"
    } else if summary.missing > 0 {
        "partial"
    } else {
        "ok"
    };

    IntegrationsStatusReport {
        schema: INTEGRATIONS_STATUS_SCHEMA,
        status,
        mutates: false,
        runtime_user: context.runtime_user.clone(),
        operator_user: context.operator_user.clone(),
        integrations,
        dependency_tooling_authorities,
        summary,
        rules: vec![
            "observation never returns credential values",
            "connected under the operator does not prove runtime-principal connectivity",
            "package install ownership and connection-principal adoption are distinct facts",
            "adoption must re-probe the same effective principal after explicit apply",
        ],
    }
}

fn effective_observation(
    spec: &IntegrationSpec,
    raw: RawIntegrationObservation,
) -> IntegrationObservation {
    let principal_matches = raw.present
        && raw
            .observed_principal
            .is_some_and(|observed| observed == spec.desired_principal);
    let lifecycle_status = if !raw.present {
        LifecycleStatus::Missing
    } else if !principal_matches {
        LifecycleStatus::PrincipalMismatch
    } else if spec.connection_required && raw.connection != ConnectionState::Connected {
        LifecycleStatus::ConnectionRequired
    } else {
        LifecycleStatus::Ready
    };
    let projection = next_boundary(spec, &raw, lifecycle_status);

    let mut proofs = Vec::new();
    if let Some(path) = &raw.path_or_endpoint {
        proofs.push(format!("location_observed:{path}"));
    }
    if raw.version.is_some() {
        proofs.push("version_observed".to_string());
    }
    if spec.connection_required {
        proofs.push("connection_presence_observed_without_secret".to_string());
    }
    if raw.observed_principal == Some(IntegrationPrincipal::RuntimeUser) {
        proofs.push("runtime_principal_observed".to_string());
    }

    IntegrationObservation {
        id: spec.id.to_string(),
        kind: spec.kind,
        capabilities: spec.capabilities.to_vec(),
        status: lifecycle_status.as_str().to_string(),
        lifecycle_status,
        present: raw.present,
        path_or_endpoint: raw.path_or_endpoint.clone(),
        version: raw.version,
        desired_principal: spec.desired_principal,
        dependency: spec.dependency.map(|dependency| IntegrationDependencyView {
            id: dependency.id.to_string(),
            install_principal: dependency.install_principal,
            tooling_authority: dependency.tooling_authority_family.map(|family| {
                let authority = observe_tooling_authority(
                    family,
                    None,
                    raw.path_or_endpoint.as_deref(),
                    dependency.expected_tooling_provenance,
                );
                let python_policy = python_cli_policy(dependency.id);
                let python_authority = python_policy.map(|policy| {
                    observe_python_install_authority(
                        Some(policy.distribution),
                        raw.path_or_endpoint.as_deref(),
                        Some(policy.expected_python_provenance),
                    )
                });
                let layout = resolve_principal_install_layout();
                let plan = plan_for_dep(dependency.id, "integration tooling policy", &layout);
                IntegrationToolingAuthorityView {
                    integration_id: None,
                    dependency_id: Some(dependency.id.to_string()),
                    family: authority.family,
                    observed_provenance: authority.observed_provenance,
                    expected_provenance: authority.expected_provenance,
                    status: authority.status,
                    observed_payload_provenance: python_authority
                        .as_ref()
                        .and_then(|authority| authority.observed_provenance)
                        .map(format_python_provenance),
                    expected_payload_provenance: python_authority
                        .as_ref()
                        .and_then(|authority| authority.expected_provenance)
                        .map(format_python_provenance),
                    canonical_update_command: python_policy
                        .map(|policy| policy.canonical_update_command.to_string()),
                    update_authority_note: python_policy
                        .map(|policy| policy.update_authority_note.to_string()),
                    normalizable_now: python_policy.is_some()
                        && which_uv_available(&layout, dependency.id)
                        && plan
                            .as_ref()
                            .map(|plan| {
                                !plan.steps.iter().any(|step| step.as_principal) || layout.writable
                            })
                            .unwrap_or(false),
                    requires_uv: python_policy
                        .map(|policy| policy.canonical_update_argv.first() == Some(&"uv"))
                        .unwrap_or(false),
                    requires_runtime_principal: plan
                        .as_ref()
                        .map(|plan| plan.steps.iter().any(|step| step.as_principal))
                        .unwrap_or(false),
                    can_auto_apply: plan.map(|plan| plan.auto_applicable).unwrap_or(false),
                }
            }),
        }),
        observed_principal: raw.observed_principal,
        principal_name: raw.principal_name,
        principal_matches,
        connection: IntegrationConnectionObservation {
            state: raw.connection,
            account: raw.account,
            credential_source: raw.credential_source,
            secret_exposed: false,
        },
        adoption_status: projection.disposition.as_str().to_string(),
        lifecycle_disposition: projection.disposition,
        next_boundary: projection.next_boundary_kind.as_str().to_string(),
        next_boundary_kind: projection.next_boundary_kind,
        next_boundary_reason: projection.next_boundary_reason,
        next_command: projection.next_command,
        notes: raw.notes,
        proofs,
    }
}

fn format_python_provenance(
    provenance: crate::domain::ecosystem_package::PythonInstallProvenance,
) -> String {
    match provenance {
        crate::domain::ecosystem_package::PythonInstallProvenance::UvTool => "uv_tool",
        crate::domain::ecosystem_package::PythonInstallProvenance::UvVenv => "uv_venv",
        crate::domain::ecosystem_package::PythonInstallProvenance::VirtualEnv => "virtual_env",
        crate::domain::ecosystem_package::PythonInstallProvenance::Pipx => "pipx",
        crate::domain::ecosystem_package::PythonInstallProvenance::PipUserSite => "pip_user_site",
        crate::domain::ecosystem_package::PythonInstallProvenance::SystemPython => "system_python",
        crate::domain::ecosystem_package::PythonInstallProvenance::Unknown => "unknown",
    }
    .to_string()
}

fn which_uv_available(
    layout: &crate::domain::ecosystem_package::PrincipalInstallLayout,
    dep_id: &str,
) -> bool {
    python_cli_policy(dep_id)
        .map(|policy| {
            if policy.canonical_update_argv.first() != Some(&"uv") {
                return true;
            }
            resolve_binary("uv", layout).under_principal
        })
        .unwrap_or(false)
}

fn next_boundary(
    spec: &IntegrationSpec,
    raw: &RawIntegrationObservation,
    status: LifecycleStatus,
) -> LifecycleProjection {
    match spec.id {
        "github" if status == LifecycleStatus::Ready => {
            integration_ready_projection("operator-local github cli is connected and observable")
        }
        "github" => integration_blocked_projection(
            status,
            BoundaryKind::Connect,
            "github cli presence does not prove the operator relay account is connected",
            Some("cynic integrations connect github --json".to_string()),
        ),
        "huggingface" if status == LifecycleStatus::Ready => integration_ready_projection(
            "runtime-principal huggingface cli is connected and observable",
        ),
        "huggingface"
            if raw.observed_principal == Some(IntegrationPrincipal::RuntimeUser) && raw.present =>
        {
            integration_blocked_projection(
                status,
                BoundaryKind::Connect,
                "runtime-principal cli is present but credential material or account binding is still missing",
                Some("cynic integrations connect huggingface --json".to_string()),
            )
        }
        "huggingface" => integration_blocked_projection(
            status,
            BoundaryKind::Adopt,
            "operator-local discovery does not satisfy runtime-principal adoption",
            Some("cynic integrations adopt huggingface --json".to_string()),
        ),
        _ => integration_blocked_projection(
            status,
            BoundaryKind::Observe,
            "no further integration boundary is modeled",
            None,
        ),
    }
}

fn integration_ready_projection(reason: impl Into<String>) -> LifecycleProjection {
    LifecycleProjection::ready_observe(reason)
}

fn integration_blocked_projection(
    status: LifecycleStatus,
    next_boundary_kind: BoundaryKind,
    reason: impl Into<String>,
    next_command: Option<String>,
) -> LifecycleProjection {
    LifecycleProjection::new(status, next_boundary_kind, reason, next_command)
}

fn count_status(observations: &[IntegrationObservation], status: &str) -> usize {
    observations
        .iter()
        .filter(|observation| observation.status == status)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus};
    use std::collections::BTreeMap;

    struct FakePort {
        observations: BTreeMap<&'static str, RawIntegrationObservation>,
    }

    impl IntegrationObservationPort for FakePort {
        fn observe(
            &self,
            spec: &IntegrationSpec,
            _context: &IntegrationObservationContext,
        ) -> RawIntegrationObservation {
            self.observations
                .get(spec.id)
                .cloned()
                .unwrap_or_else(|| RawIntegrationObservation::missing("not installed"))
        }
    }

    #[test]
    fn operator_hf_session_does_not_prove_runtime_connection() {
        let mut observations = BTreeMap::new();
        observations.insert(
            "huggingface",
            RawIntegrationObservation {
                present: true,
                path_or_endpoint: Some("/home/operator/.local/bin/hf".into()),
                version: Some("1.23.0".into()),
                observed_principal: Some(IntegrationPrincipal::OperatorLocal),
                principal_name: Some("operator".into()),
                connection: ConnectionState::Connected,
                account: Some("account".into()),
                credential_source: Some("provider_native_store_or_env".into()),
                notes: vec![],
            },
        );
        let report = observe_integrations(
            &FakePort { observations },
            &IntegrationObservationContext {
                runtime_user: "cynic".into(),
                runtime_home: "/var/lib/cynic".into(),
                operator_user: "operator".into(),
            },
        );
        let hf = report
            .integrations
            .iter()
            .find(|item| item.id == "huggingface")
            .unwrap();
        assert_eq!(hf.status, "principal_mismatch");
        assert_eq!(hf.lifecycle_status, LifecycleStatus::PrincipalMismatch);
        assert_eq!(hf.adoption_status, "requires_principal_adoption");
        assert_eq!(
            hf.lifecycle_disposition,
            LifecycleDisposition::RequiresPrincipalAdoption
        );
        assert_eq!(hf.next_boundary, "adopt");
        assert_eq!(hf.next_boundary_kind, BoundaryKind::Adopt);
        assert_eq!(
            hf.next_boundary_reason,
            "operator-local discovery does not satisfy runtime-principal adoption"
        );
        assert_eq!(
            hf.next_command.as_deref(),
            Some("cynic integrations adopt huggingface --json")
        );
        assert!(!hf.connection.secret_exposed);
    }

    #[test]
    fn runtime_hf_presence_without_connection_requires_connect_boundary() {
        let mut observations = BTreeMap::new();
        observations.insert(
            "huggingface",
            RawIntegrationObservation {
                present: true,
                path_or_endpoint: Some("/var/lib/cynic/.local/bin/hf".into()),
                version: Some("1.23.0".into()),
                observed_principal: Some(IntegrationPrincipal::RuntimeUser),
                principal_name: Some("cynic".into()),
                connection: ConnectionState::Disconnected,
                account: None,
                credential_source: None,
                notes: vec![],
            },
        );
        let report = observe_integrations(
            &FakePort { observations },
            &IntegrationObservationContext {
                runtime_user: "cynic".into(),
                runtime_home: "/var/lib/cynic".into(),
                operator_user: "operator".into(),
            },
        );
        let hf = report
            .integrations
            .iter()
            .find(|item| item.id == "huggingface")
            .unwrap();
        assert_eq!(hf.lifecycle_status, LifecycleStatus::ConnectionRequired);
        assert_eq!(hf.next_boundary_kind, BoundaryKind::Connect);
        assert_eq!(
            hf.next_command.as_deref(),
            Some("cynic integrations connect huggingface --json")
        );
    }
}
