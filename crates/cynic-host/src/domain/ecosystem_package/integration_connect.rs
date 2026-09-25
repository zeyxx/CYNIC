use crate::execution::{
    HostExecutionPlan, HostExecutionPlanReport, host_execution_plan_from_commands,
};
use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};

use super::integration_decision::{IntegrationDecisionSurface, decide_integration_action};
use super::integration_port::IntegrationObservationPort;
use super::integration_recipe::build_integration_recipe;
use super::integration_types::{
    ConnectionState, IntegrationObservation, IntegrationObservationContext,
};
use super::{IntegrationConnectReport, observe_integrations};

pub fn connect_local_integration(
    id: &str,
    apply: bool,
) -> Result<IntegrationConnectReport, String> {
    let runner = LocalArgvRunner;
    let adapter =
        crate::adapters::local_integration::LocalIntegrationObservationAdapter::new(&runner);
    connect_integration(id, apply, &runner, &adapter, &adapter.context())
}

pub fn connect_integration(
    id: &str,
    apply: bool,
    _runner: &dyn ArgvRunner,
    port: &dyn IntegrationObservationPort,
    context: &IntegrationObservationContext,
) -> Result<IntegrationConnectReport, String> {
    let before = observe_integrations(port, context)
        .integrations
        .into_iter()
        .find(|integration| integration.id == id)
        .ok_or_else(|| format!("unknown integration `{id}`"))?;

    let decision = decide_integration_action(&before);
    let action = decision.action;
    let recipe = build_integration_recipe(&before, context, action);
    let execution_plan = execution_plan_for(id, context, recipe.connect_execution);
    let mut blockers = blockers_for(id, &before, decision.surface);
    let execution_report = execution_report_for(&execution_plan, apply, &blockers);

    if !apply {
        blockers.extend(execution_report.blockers.clone());
    }

    let after = observe_integrations(port, context)
        .integrations
        .into_iter()
        .find(|integration| integration.id == id)
        .ok_or_else(|| format!("integration `{id}` disappeared after connect"))?;

    let status = if !apply {
        "planned"
    } else if blockers.is_empty() && after.status == "ready" {
        "completed"
    } else {
        "blocked"
    };

    Ok(IntegrationConnectReport {
        schema: "cynic.ecosystem_integrations.connect.v1",
        status,
        mutates: true,
        requested_apply: apply,
        integration_id: id.to_string(),
        action,
        before,
        execution_plan,
        execution_report,
        after,
        blockers,
        rules: vec![
            "connect owns provider login and credential-presence adoption, not binary installation",
            "interactive provider login is never hidden behind argv execution",
            "secret values must not appear in argv, output, or proof",
            "runtime-principal connection is distinct from operator-local connection",
        ],
    })
}

fn execution_plan_for(
    id: &str,
    context: &IntegrationObservationContext,
    commands: Vec<crate::command_surface::HostCommandSurface>,
) -> HostExecutionPlan {
    let requires_privilege = id == "huggingface" && context.operator_user != context.runtime_user;
    host_execution_plan_from_commands(
        format!("cynic.ecosystem_integrations.connect.{id}"),
        "ecosystem_integration_connection",
        commands,
        true,
        requires_privilege,
    )
}

fn blockers_for(
    id: &str,
    before: &IntegrationObservation,
    decision_surface: IntegrationDecisionSurface,
) -> Vec<String> {
    match id {
        "github" if before.status == "ready" => Vec::new(),
        "github" => vec!["interactive_connection_required:github".to_string()],
        "huggingface" if decision_surface != IntegrationDecisionSurface::Connect => {
            vec!["adopt_runtime_cli_required:huggingface".to_string()]
        }
        "huggingface" if before.connection.state == ConnectionState::Connected => Vec::new(),
        "huggingface" => vec!["runtime_connection_material_required:huggingface".to_string()],
        other => vec![format!("unknown_integration:{other}")],
    }
}

fn execution_report_for(
    plan: &HostExecutionPlan,
    apply: bool,
    blockers: &[String],
) -> HostExecutionPlanReport {
    let mut report_blockers = Vec::new();
    if !apply {
        report_blockers.push("apply_flag_required".to_string());
    } else if !blockers.is_empty() {
        report_blockers.extend(blockers.iter().cloned());
    }

    HostExecutionPlanReport {
        schema: "cynic.host.execution_plan_report.v1",
        status: if report_blockers.is_empty() {
            "completed"
        } else {
            "blocked"
        },
        plan_id: plan.id.clone(),
        layer: plan.layer.clone(),
        mutates: plan.mutates,
        requires_privilege: plan.requires_privilege,
        requested_apply: apply,
        step_count: plan.steps.len(),
        execution_count: 0,
        executions: Vec::new(),
        blockers: report_blockers,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::connect_integration;
    use crate::domain::ecosystem_package::{
        ConnectionState, IntegrationObservationContext, IntegrationObservationPort,
        IntegrationPlanAction, IntegrationPrincipal, IntegrationSpec, RawIntegrationObservation,
    };
    use cynic_lifecycle::MapArgvRunner;

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
    fn github_connect_requires_explicit_interactive_login() {
        let report = connect_integration(
            "github",
            true,
            &MapArgvRunner::default(),
            &FakePort {
                observations: BTreeMap::from([(
                    "github",
                    RawIntegrationObservation {
                        present: true,
                        path_or_endpoint: Some("/usr/bin/gh".into()),
                        version: Some("2.0.0".into()),
                        observed_principal: Some(IntegrationPrincipal::OperatorLocal),
                        principal_name: Some("operator".into()),
                        connection: ConnectionState::Disconnected,
                        account: None,
                        credential_source: None,
                        notes: vec![],
                    },
                )]),
            },
            &IntegrationObservationContext {
                runtime_user: "cynic".into(),
                runtime_home: "/var/lib/cynic".into(),
                operator_user: "operator".into(),
            },
        )
        .expect("connect report");

        assert_eq!(report.action, IntegrationPlanAction::ConnectOperatorCli);
        assert_eq!(report.status, "blocked");
        assert_eq!(report.execution_plan.steps.len(), 2);
        assert!(
            report
                .blockers
                .contains(&"interactive_connection_required:github".to_string())
        );
    }

    #[test]
    fn huggingface_connect_requires_runtime_adoption_first() {
        let report = connect_integration(
            "huggingface",
            true,
            &MapArgvRunner::default(),
            &FakePort {
                observations: BTreeMap::from([(
                    "huggingface",
                    RawIntegrationObservation {
                        present: true,
                        path_or_endpoint: Some("/home/operator/.local/bin/hf".into()),
                        version: Some("1.17.0".into()),
                        observed_principal: Some(IntegrationPrincipal::OperatorLocal),
                        principal_name: Some("operator".into()),
                        connection: ConnectionState::Connected,
                        account: Some("acct".into()),
                        credential_source: Some("provider_native_store_or_env".into()),
                        notes: vec![],
                    },
                )]),
            },
            &IntegrationObservationContext {
                runtime_user: "cynic".into(),
                runtime_home: "/var/lib/cynic".into(),
                operator_user: "operator".into(),
            },
        )
        .expect("connect report");

        assert_eq!(report.status, "blocked");
        assert_eq!(report.execution_plan.steps.len(), 2);
        assert!(
            report
                .blockers
                .contains(&"adopt_runtime_cli_required:huggingface".to_string())
        );
    }
}
