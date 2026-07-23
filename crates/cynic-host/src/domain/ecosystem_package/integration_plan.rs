use super::integration_decision::decide_integration_action;
use super::integration_port::IntegrationObservationPort;
use super::integration_recipe::build_integration_recipe;
use super::integration_types::{IntegrationObservation, IntegrationObservationContext};
use super::{
    IntegrationPlanAction, IntegrationPlanItem, IntegrationsPlanReport, IntegrationsPlanSummary,
    observe_integrations,
};

pub const INTEGRATIONS_PLAN_SCHEMA: &str = "cynic.ecosystem_integrations.plan.v1";

pub fn plan_integrations(
    port: &dyn IntegrationObservationPort,
    context: &IntegrationObservationContext,
) -> IntegrationsPlanReport {
    let status_report = observe_integrations(port, context);
    let integrations = status_report
        .integrations
        .iter()
        .map(|observation| plan_item(observation, context))
        .collect::<Vec<_>>();

    let summary = IntegrationsPlanSummary {
        total: integrations.len(),
        no_action: integrations
            .iter()
            .filter(|item| item.action == IntegrationPlanAction::None)
            .count(),
        planned_actions: integrations
            .iter()
            .filter(|item| item.requires_apply)
            .count(),
        privilege_required: integrations
            .iter()
            .filter(|item| item.privilege_required)
            .count(),
    };
    let status = if summary.planned_actions > 0 {
        "attention_required"
    } else {
        "ok"
    };

    IntegrationsPlanReport {
        schema: INTEGRATIONS_PLAN_SCHEMA,
        status,
        mutates: false,
        runtime_user: context.runtime_user.clone(),
        operator_user: context.operator_user.clone(),
        integrations,
        dependency_tooling_authorities: status_report.dependency_tooling_authorities,
        summary,
        rules: vec![
            "plan is read-only and does not execute apply steps",
            "apply steps are explicit proposals and must be re-probed after execution",
            "runtime-principal adoption is distinct from operator-local login discovery",
            "credential presence may be proven, but secret values must never appear in plan output",
        ],
    }
}

fn plan_item(
    observation: &IntegrationObservation,
    context: &IntegrationObservationContext,
) -> IntegrationPlanItem {
    let decision = decide_integration_action(observation);
    let recipe = build_integration_recipe(observation, context, decision.action);
    match observation.id.as_str() {
        "github" => plan_github(observation, decision.action, recipe),
        "huggingface" => plan_huggingface(observation, context, decision.action, recipe),
        _ => IntegrationPlanItem {
            id: observation.id.clone(),
            current_status: observation.status.clone(),
            action: decision.action,
            requires_apply: recipe.requires_apply,
            privilege_required: recipe.privilege_required,
            dependency: observation.dependency.clone(),
            desired_principal: observation.desired_principal,
            observed_principal: observation.observed_principal,
            execution: recipe.proposal,
            notes: observation.notes.clone(),
        },
    }
}

fn plan_github(
    observation: &IntegrationObservation,
    action: IntegrationPlanAction,
    recipe: super::IntegrationRecipe,
) -> IntegrationPlanItem {
    IntegrationPlanItem {
        id: observation.id.clone(),
        current_status: observation.status.clone(),
        action,
        requires_apply: recipe.requires_apply,
        privilege_required: recipe.privilege_required,
        dependency: observation.dependency.clone(),
        desired_principal: observation.desired_principal,
        observed_principal: observation.observed_principal,
        execution: recipe.proposal,
        notes: observation.notes.clone(),
    }
}

fn plan_huggingface(
    observation: &IntegrationObservation,
    context: &IntegrationObservationContext,
    action: IntegrationPlanAction,
    recipe: super::IntegrationRecipe,
) -> IntegrationPlanItem {
    if action == IntegrationPlanAction::None {
        return IntegrationPlanItem {
            id: observation.id.clone(),
            current_status: observation.status.clone(),
            action,
            requires_apply: recipe.requires_apply,
            privilege_required: recipe.privilege_required,
            dependency: observation.dependency.clone(),
            desired_principal: observation.desired_principal,
            observed_principal: observation.observed_principal,
            execution: recipe.proposal,
            notes: observation.notes.clone(),
        };
    }

    if action == IntegrationPlanAction::ConnectRuntimeCli {
        let mut notes = observation.notes.clone();
        notes.push(format!(
            "runtime principal `{}` already owns the CLI; use `cynic integrations connect huggingface` for credential adoption",
            context.runtime_user
        ));

        return IntegrationPlanItem {
            id: observation.id.clone(),
            current_status: observation.status.clone(),
            action,
            requires_apply: recipe.requires_apply,
            privilege_required: recipe.privilege_required,
            dependency: observation.dependency.clone(),
            desired_principal: observation.desired_principal,
            observed_principal: observation.observed_principal,
            execution: recipe.proposal,
            notes,
        };
    }

    let mut notes = observation.notes.clone();
    notes.push(format!(
        "runtime adoption target is `{}` with HOME `{}`",
        context.runtime_user, context.runtime_home
    ));

    IntegrationPlanItem {
        id: observation.id.clone(),
        current_status: observation.status.clone(),
        action,
        requires_apply: recipe.requires_apply,
        privilege_required: recipe.privilege_required,
        dependency: observation.dependency.clone(),
        desired_principal: observation.desired_principal,
        observed_principal: observation.observed_principal,
        execution: recipe.proposal,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::ecosystem_package::{
        ConnectionState, IntegrationObservationPort, IntegrationPrincipal, IntegrationSpec,
        RawIntegrationObservation,
    };

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
    fn huggingface_operator_login_plans_runtime_adoption() {
        let mut observations = BTreeMap::new();
        observations.insert(
            "huggingface",
            RawIntegrationObservation {
                present: true,
                path_or_endpoint: Some("/home/operator/.local/bin/hf".into()),
                version: Some("1.17.0".into()),
                observed_principal: Some(IntegrationPrincipal::OperatorLocal),
                principal_name: Some("operator".into()),
                connection: ConnectionState::Connected,
                account: Some("jeanterre13".into()),
                credential_source: Some("provider_native_store_or_env".into()),
                notes: vec!["operator discovery does not satisfy runtime principal `cynic`".into()],
            },
        );
        let report = plan_integrations(
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
            .expect("huggingface plan item");
        assert_eq!(hf.action, IntegrationPlanAction::AdoptRuntimeCli);
        assert!(hf.requires_apply);
        assert!(hf.privilege_required);
        assert_eq!(
            hf.dependency
                .as_ref()
                .map(|dependency| dependency.id.as_str()),
            Some("huggingface-cli")
        );
        assert!(hf.execution.apply.iter().any(|step| matches!(
            step.invocation,
            crate::command_surface::HostCommandInvocation::Expectation { .. }
        )));
    }

    #[test]
    fn huggingface_runtime_cli_without_connection_plans_runtime_connect() {
        let mut observations = BTreeMap::new();
        observations.insert(
            "huggingface",
            RawIntegrationObservation {
                present: true,
                path_or_endpoint: Some("/var/lib/cynic/.local/bin/hf".into()),
                version: Some("1.17.0".into()),
                observed_principal: Some(IntegrationPrincipal::RuntimeUser),
                principal_name: Some("cynic".into()),
                connection: ConnectionState::Disconnected,
                account: None,
                credential_source: None,
                notes: vec![],
            },
        );
        let report = plan_integrations(
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
            .expect("huggingface plan item");
        assert_eq!(hf.action, IntegrationPlanAction::ConnectRuntimeCli);
        assert!(hf.requires_apply);
    }
}
