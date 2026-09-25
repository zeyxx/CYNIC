use crate::command_surface::HostCommandInvocation;
use crate::execution::{
    HostExecutionPlan, HostExecutionPlanReport, HostExecutionStep, HostExecutionStepExecution,
    command_to_argv, host_execution_plan_from_commands,
};
use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};

use super::integration_decision::{IntegrationDecisionSurface, decide_integration_action};
use super::integration_port::IntegrationObservationPort;
use super::integration_recipe::build_integration_recipe;
use super::integration_types::{IntegrationObservation, IntegrationObservationContext};
use super::{IntegrationApplyReport, observe_integrations};

pub fn apply_local_integration(id: &str, apply: bool) -> Result<IntegrationApplyReport, String> {
    let runner = LocalArgvRunner;
    let adapter =
        crate::adapters::local_integration::LocalIntegrationObservationAdapter::new(&runner);
    apply_integration(id, apply, &runner, &adapter, &adapter.context())
}

pub fn apply_integration(
    id: &str,
    apply: bool,
    runner: &dyn ArgvRunner,
    port: &dyn IntegrationObservationPort,
    context: &IntegrationObservationContext,
) -> Result<IntegrationApplyReport, String> {
    let before = observe_integrations(port, context)
        .integrations
        .into_iter()
        .find(|integration| integration.id == id)
        .ok_or_else(|| format!("unknown integration `{id}`"))?;

    let mut blockers = Vec::new();
    let decision = decide_integration_action(&before);
    let action = decision.action;
    let recipe = build_integration_recipe(&before, context, action);
    let mut execution_plan = execution_plan_for(
        id,
        context,
        decision.surface,
        recipe.adopt_execution,
        &mut blockers,
    );
    annotate_plan(&mut execution_plan, id, &before, context);

    let execution_report = execute_plan_with_runner(&execution_plan, apply, runner);
    blockers.extend(execution_report.blockers.clone());

    let after = observe_integrations(port, context)
        .integrations
        .into_iter()
        .find(|integration| integration.id == id)
        .ok_or_else(|| format!("integration `{id}` disappeared after apply"))?;

    if apply && execution_report.status == "completed" && after.status != "ready" {
        blockers.push(format!(
            "post_apply_status:{}:{}",
            after.status, after.adoption_status
        ));
    }

    let status = if !apply {
        "planned"
    } else if !blockers.is_empty() {
        if execution_report.status == "failed" {
            "failed"
        } else if execution_report.status == "blocked" {
            "blocked"
        } else {
            "partial"
        }
    } else {
        "completed"
    };

    Ok(IntegrationApplyReport {
        schema: "cynic.ecosystem_integrations.adopt.v1",
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
            "apply remains explicit and argv-only",
            "no credential values appear in argv, output, or proof",
            "post-apply success requires re-probe under the effective principal",
            "interactive provider login is never hidden behind this surface",
        ],
    })
}

fn execution_plan_for(
    id: &str,
    context: &IntegrationObservationContext,
    decision_surface: IntegrationDecisionSurface,
    commands: Vec<crate::command_surface::HostCommandSurface>,
    blockers: &mut Vec<String>,
) -> HostExecutionPlan {
    match id {
        "huggingface" => {
            if decision_surface != IntegrationDecisionSurface::Adopt {
                blockers.push("use_connect_surface:huggingface".to_string());
                host_execution_plan_from_commands(
                    "cynic.ecosystem_integrations.adopt.huggingface",
                    "ecosystem_integration_adoption",
                    Vec::new(),
                    true,
                    context.operator_user != context.runtime_user,
                )
            } else {
                host_execution_plan_from_commands(
                    "cynic.ecosystem_integrations.adopt.huggingface",
                    "ecosystem_integration_adoption",
                    commands,
                    true,
                    true,
                )
            }
        }
        "github" => {
            blockers.push("use_connect_surface:github".to_string());
            host_execution_plan_from_commands(
                "cynic.ecosystem_integrations.adopt.github",
                "ecosystem_integration_adoption",
                Vec::new(),
                true,
                false,
            )
        }
        other => {
            blockers.push(format!("unknown_integration:{other}"));
            host_execution_plan_from_commands(
                format!("cynic.ecosystem_integrations.adopt.{other}"),
                "ecosystem_integration_adoption",
                Vec::new(),
                true,
                false,
            )
        }
    }
}

fn annotate_plan(
    plan: &mut HostExecutionPlan,
    id: &str,
    before: &IntegrationObservation,
    context: &IntegrationObservationContext,
) {
    if id == "huggingface" {
        let runtime_hf = format!("{}/.local/bin/hf", context.runtime_home);
        let installer_path = format!(
            "{}/.local/share/cynic/bootstrap/hf-cli-install.sh",
            context.runtime_home
        );
        for step in &mut plan.steps {
            if step.role == "runtime_home_present" || step.role == "runtime_principal_sudo_check" {
                step.mutates = false;
            }
            annotate_hf_step(step, &runtime_hf, &installer_path);
            if before.status == "ready" && step.role != "runtime_hf_version" {
                step.preconditions.push("already_ready_before_apply".into());
            }
        }
    }
}

fn annotate_hf_step(step: &mut HostExecutionStep, runtime_hf: &str, installer_path: &str) {
    match &step.command.invocation {
        HostCommandInvocation::Argv { .. } if step.role == "download_hf_install_script" => {
            step.proof_paths.push(installer_path.to_string());
            step.rollback_hint =
                Some("remove the installer script after adoption if it is no longer needed".into());
        }
        HostCommandInvocation::Argv { .. } if step.role == "run_hf_install_script" => {
            step.proof_paths.push(runtime_hf.to_string());
            step.rollback_hint = Some(
                "remove the runtime `hf` binary if the install channel must be rolled back".into(),
            );
        }
        HostCommandInvocation::Argv { .. } if step.role == "runtime_hf_version" => {
            step.mutates = false;
            step.proof_paths.push(runtime_hf.to_string());
        }
        _ => {}
    }
}

fn execute_plan_with_runner(
    plan: &HostExecutionPlan,
    apply: bool,
    runner: &dyn ArgvRunner,
) -> HostExecutionPlanReport {
    let mut blockers = Vec::new();
    let mut executions = Vec::new();

    if !apply {
        blockers.push("apply_flag_required".to_string());
    }

    if blockers.is_empty() {
        for step in &plan.steps {
            match command_to_argv(&step.command) {
                Ok(argv) => {
                    let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
                    let output = runner.run(&refs);
                    let success = output.spawn_error.is_none()
                        && output
                            .exit_code
                            .map(|code| step.success_exit_codes.contains(&code))
                            .unwrap_or(false);
                    executions.push(HostExecutionStepExecution {
                        step_id: step.id.clone(),
                        role: step.role.clone(),
                        argv: argv.clone(),
                        exit_code: output.exit_code,
                        success,
                        stdout: output.stdout,
                        stderr: output.spawn_error.unwrap_or(output.stderr),
                        proof_paths: step.proof_paths.clone(),
                        rollback_hint: step.rollback_hint.clone(),
                    });
                    if !success {
                        break;
                    }
                }
                Err(error) => {
                    blockers.push(format!("{}: {}: {error}", step.id, step.role));
                    break;
                }
            }
        }
    }

    HostExecutionPlanReport {
        schema: "cynic.host.execution_plan_report.v1",
        status: if !blockers.is_empty() {
            "blocked"
        } else if executions.len() == plan.steps.len() && executions.iter().all(|step| step.success)
        {
            "completed"
        } else {
            "failed"
        },
        plan_id: plan.id.clone(),
        layer: plan.layer.clone(),
        mutates: plan.mutates,
        requires_privilege: plan.requires_privilege,
        requested_apply: apply,
        step_count: plan.steps.len(),
        execution_count: executions.len(),
        executions,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::domain::ecosystem_package::{
        ConnectionState, IntegrationObservationPort, IntegrationPrincipal, IntegrationSpec,
        RawIntegrationObservation, integration_recipe::HF_INSTALL_URL,
    };
    use cynic_lifecycle::{ArgvOutput, MapArgvRunner};

    struct FakePort {
        observations: Vec<BTreeMap<&'static str, RawIntegrationObservation>>,
        cursor: std::sync::Mutex<usize>,
    }

    impl IntegrationObservationPort for FakePort {
        fn observe(
            &self,
            spec: &IntegrationSpec,
            _context: &IntegrationObservationContext,
        ) -> RawIntegrationObservation {
            let mut cursor = self.cursor.lock().expect("cursor lock");
            let index = (*cursor).min(self.observations.len().saturating_sub(1));
            let value = self.observations[index]
                .get(spec.id)
                .cloned()
                .unwrap_or_else(|| RawIntegrationObservation::missing("not installed"));
            if spec.id == "huggingface" && *cursor < self.observations.len().saturating_sub(1) {
                *cursor += 1;
            }
            value
        }
    }

    #[test]
    fn adopt_report_blocks_without_apply() {
        let report = apply_integration(
            "huggingface",
            false,
            &MapArgvRunner::default(),
            &FakePort {
                observations: vec![BTreeMap::from([(
                    "huggingface",
                    RawIntegrationObservation {
                        present: true,
                        path_or_endpoint: Some("/home/user/.local/bin/hf".into()),
                        version: Some("1.17.0".into()),
                        observed_principal: Some(IntegrationPrincipal::OperatorLocal),
                        principal_name: Some("user".into()),
                        connection: ConnectionState::Connected,
                        account: Some("acct".into()),
                        credential_source: Some("provider_native_store_or_env".into()),
                        notes: vec![],
                    },
                )])],
                cursor: std::sync::Mutex::new(0),
            },
            &IntegrationObservationContext {
                runtime_user: "cynic".into(),
                runtime_home: "/var/lib/cynic".into(),
                operator_user: "user".into(),
            },
        )
        .expect("apply report");
        assert_eq!(report.status, "planned");
        assert_eq!(report.execution_report.status, "blocked");
    }

    #[test]
    fn adopt_hf_installs_runtime_cli_then_reprobes() {
        let runner = MapArgvRunner {
            responses: vec![
                (
                    vec!["test".into(), "-d".into(), "/var/lib/cynic".into()],
                    ok(),
                ),
                (
                    vec![
                        "sudo".into(),
                        "-n".into(),
                        "-u".into(),
                        "cynic".into(),
                        "--".into(),
                        "env".into(),
                        "HOME=/var/lib/cynic".into(),
                        "true".into(),
                    ],
                    ok(),
                ),
                (
                    vec![
                        "sudo".into(),
                        "-n".into(),
                        "-u".into(),
                        "cynic".into(),
                        "--".into(),
                        "env".into(),
                        "HOME=/var/lib/cynic".into(),
                        "install".into(),
                        "-d".into(),
                        "-m".into(),
                        "0755".into(),
                        "/var/lib/cynic/.local/bin".into(),
                    ],
                    ok(),
                ),
                (
                    vec![
                        "sudo".into(),
                        "-n".into(),
                        "-u".into(),
                        "cynic".into(),
                        "--".into(),
                        "env".into(),
                        "HOME=/var/lib/cynic".into(),
                        "install".into(),
                        "-d".into(),
                        "-m".into(),
                        "0755".into(),
                        "/var/lib/cynic/.local/share/cynic/bootstrap".into(),
                    ],
                    ok(),
                ),
                (
                    vec![
                        "sudo".into(),
                        "-n".into(),
                        "-u".into(),
                        "cynic".into(),
                        "--".into(),
                        "env".into(),
                        "HOME=/var/lib/cynic".into(),
                        "curl".into(),
                        "-LsSf".into(),
                        HF_INSTALL_URL.into(),
                        "-o".into(),
                        "/var/lib/cynic/.local/share/cynic/bootstrap/hf-cli-install.sh".into(),
                    ],
                    ok(),
                ),
                (
                    vec![
                        "sudo".into(),
                        "-n".into(),
                        "-u".into(),
                        "cynic".into(),
                        "--".into(),
                        "env".into(),
                        "HOME=/var/lib/cynic".into(),
                        "bash".into(),
                        "/var/lib/cynic/.local/share/cynic/bootstrap/hf-cli-install.sh".into(),
                    ],
                    ok(),
                ),
                (
                    vec![
                        "sudo".into(),
                        "-n".into(),
                        "-u".into(),
                        "cynic".into(),
                        "--".into(),
                        "env".into(),
                        "HOME=/var/lib/cynic".into(),
                        "/var/lib/cynic/.local/bin/hf".into(),
                        "version".into(),
                    ],
                    ok(),
                ),
            ],
        };
        let report = apply_integration(
            "huggingface",
            true,
            &runner,
            &FakePort {
                observations: vec![
                    BTreeMap::from([(
                        "huggingface",
                        RawIntegrationObservation {
                            present: true,
                            path_or_endpoint: Some("/home/user/.local/bin/hf".into()),
                            version: Some("1.17.0".into()),
                            observed_principal: Some(IntegrationPrincipal::OperatorLocal),
                            principal_name: Some("user".into()),
                            connection: ConnectionState::Connected,
                            account: Some("acct".into()),
                            credential_source: Some("provider_native_store_or_env".into()),
                            notes: vec![],
                        },
                    )]),
                    BTreeMap::from([(
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
                    )]),
                ],
                cursor: std::sync::Mutex::new(0),
            },
            &IntegrationObservationContext {
                runtime_user: "cynic".into(),
                runtime_home: "/var/lib/cynic".into(),
                operator_user: "user".into(),
            },
        )
        .expect("apply report");
        assert_eq!(report.execution_report.status, "completed");
        assert_eq!(report.status, "partial");
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| blocker
                    == "post_apply_status:connection_required:requires_connection")
        );
    }

    fn ok() -> ArgvOutput {
        ArgvOutput {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: None,
        }
    }
}
