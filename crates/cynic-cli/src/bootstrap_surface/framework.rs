use std::path::Path;
use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::framework_bootstrap;
use cynic_host::update_lifecycle::LocalArgvRunner;

use crate::fs_json::write_pretty_json_file;
use crate::{emit_json, fail};

mod substrate;

#[derive(Subcommand)]
pub(crate) enum BootstrapFrameworkCommand {
    Bundle {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output_dir: String,
        #[arg(long, default_value = "unassigned")]
        role: String,
        #[arg(long)]
        artifact_dir: Option<String>,
        #[arg(long)]
        registry: Option<String>,
    },
    PropagationPlan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        bundle_dir: String,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        output: Option<String>,
    },
    Propagate {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        bundle_dir: String,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        output: Option<String>,
    },
    SubstratePlan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        node: String,
        #[arg(long)]
        observation: Option<String>,
    },
    SubstrateApply {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        node: String,
        #[arg(long)]
        yes: bool,
    },
    InstallCli {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        target: Option<String>,
    },
    EnrollmentRequest {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        seed_manifest: String,
        #[arg(long)]
        machine_id: String,
        #[arg(long)]
        role: String,
    },
    LifecycleAudit {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        role: String,
        #[arg(long)]
        registry: Option<String>,
    },
}

pub(crate) fn execute_framework(command: BootstrapFrameworkCommand) -> ExitCode {
    match command {
        BootstrapFrameworkCommand::Bundle {
            json,
            output_dir,
            role,
            artifact_dir,
            registry,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match framework_bootstrap::write_framework_bootstrap_bundle_for_role(
                &output_dir,
                &role,
                artifact_dir.as_deref(),
                registry.as_deref(),
            ) {
                Ok(report) => emit_json(&report, report.status != "written"),
                Err(error) => fail(error),
            }
        }
        BootstrapFrameworkCommand::InstallCli {
            json,
            apply,
            source,
            target,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match framework_bootstrap::install_standalone_cli(
                source.as_deref(),
                target.as_deref(),
                apply,
            ) {
                Ok(report) => emit_json(&report, report.status == "blocked"),
                Err(error) => fail(error),
            }
        }
        BootstrapFrameworkCommand::PropagationPlan {
            json,
            bundle_dir,
            registry,
            output,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match framework_bootstrap::plan_framework_propagation(
                &bundle_dir,
                registry.as_deref(),
                &LocalArgvRunner,
            ) {
                Ok(plan) => {
                    if let Some(output) = output
                        && let Err(error) = write_pretty_json_file(
                            Path::new(&output),
                            &plan,
                            "framework propagation plan",
                        )
                    {
                        return fail(error);
                    }
                    emit_json(&plan, plan.status == "blocked")
                }
                Err(error) => fail(error),
            }
        }
        BootstrapFrameworkCommand::Propagate {
            json,
            bundle_dir,
            registry,
            yes,
            output,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let plan = match framework_bootstrap::plan_framework_propagation(
                &bundle_dir,
                registry.as_deref(),
                &LocalArgvRunner,
            ) {
                Ok(plan) => plan,
                Err(error) => return fail(error),
            };
            let report =
                framework_bootstrap::apply_framework_propagation(&plan, yes, &LocalArgvRunner);
            if let Some(output) = output
                && let Err(error) = write_pretty_json_file(
                    Path::new(&output),
                    &report,
                    "framework propagation report",
                )
            {
                return fail(error);
            }
            emit_json(&report, report.status != "converged")
        }
        BootstrapFrameworkCommand::SubstratePlan {
            json,
            registry,
            node,
            observation,
        } => substrate::execute_substrate_plan(json, registry, node, observation),
        BootstrapFrameworkCommand::SubstrateApply {
            json,
            registry,
            node,
            yes,
        } => substrate::execute_substrate_apply(json, registry, node, yes),
        BootstrapFrameworkCommand::EnrollmentRequest {
            json,
            seed_manifest,
            machine_id,
            role,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let role = match framework_bootstrap::FrameworkRole::parse(&role) {
                Ok(role) => role,
                Err(error) => return fail(error),
            };
            match framework_bootstrap::build_framework_enrollment_request(
                &seed_manifest,
                &machine_id,
                role,
            ) {
                Ok(report) => emit_json(&report, true),
                Err(error) => fail(error),
            }
        }
        BootstrapFrameworkCommand::LifecycleAudit {
            json,
            role,
            registry,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match framework_bootstrap::audit_framework_lifecycle(registry.as_deref(), &role) {
                Ok(report) => emit_json(&report, report.status == "blocked"),
                Err(error) => fail(error),
            }
        }
    }
}
