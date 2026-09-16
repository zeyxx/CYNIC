use std::path::Path;
use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::framework_bootstrap;
use cynic_lifecycle::LocalArgvRunner;

use crate::fs_json::write_pretty_json_file;
use crate::{emit_json, fail};

mod source_snapshot;
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
    SourceSnapshotPlan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        repository_root: String,
        #[arg(long)]
        source_commit: String,
        #[arg(long)]
        output_dir: String,
        #[arg(long)]
        output: Option<String>,
    },
    SourceSnapshotCreate {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        repository_root: String,
        #[arg(long)]
        source_commit: String,
        #[arg(long)]
        output_dir: String,
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
    EnrollmentAuthorityPlan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        seed_manifest: String,
        #[arg(long)]
        node: String,
        #[arg(long)]
        staging_root: String,
        #[arg(long)]
        registry: Option<String>,
    },
    EnrollmentAuthorityApply {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        seed_manifest: String,
        #[arg(long)]
        node: String,
        #[arg(long)]
        staging_root: String,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    RuntimeBindingPlan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
    RuntimeBindingApply {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    ProviderEnrollmentPlan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        token_secret_file: String,
        #[arg(long)]
        ca_file: Option<String>,
    },
    ProviderEnrollmentApply {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        token_secret_file: String,
        #[arg(long)]
        ca_file: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    LifecycleAudit {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        role: String,
        #[arg(long)]
        registry: Option<String>,
    },
    EpochPlan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        source_commit: String,
        #[arg(long)]
        desired_state_digest: String,
        #[arg(long)]
        target_scope: String,
        #[arg(long)]
        created_at_unix_ms: u64,
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
                    if let Some(out) = output {
                        if let Err(error) = write_pretty_json_file(
                            Path::new(&out),
                            &plan,
                            "framework propagation plan",
                        ) {
                            return fail(error);
                        }
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
            if let Some(out) = output {
                if let Err(error) =
                    write_pretty_json_file(Path::new(&out), &report, "framework propagation report")
                {
                    return fail(error);
                }
            }
            emit_json(&report, report.status != "converged")
        }
        BootstrapFrameworkCommand::SourceSnapshotPlan {
            json,
            repository_root,
            source_commit,
            output_dir,
            output,
        } => {
            source_snapshot::execute_plan(json, repository_root, source_commit, output_dir, output)
        }
        BootstrapFrameworkCommand::SourceSnapshotCreate {
            json,
            repository_root,
            source_commit,
            output_dir,
            yes,
            output,
        } => source_snapshot::execute_create(
            json,
            repository_root,
            source_commit,
            output_dir,
            yes,
            output,
        ),
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
        BootstrapFrameworkCommand::EnrollmentAuthorityPlan {
            json,
            seed_manifest,
            node,
            staging_root,
            registry,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry = registry
                .as_deref()
                .unwrap_or(cynic_host::registry::DEFAULT_REGISTRY_PATH);
            let spec = match framework_bootstrap::resolve_local_machine_enrollment_authority_spec(
                Path::new(registry),
                Path::new(&seed_manifest),
                &node,
                Path::new(&staging_root),
            ) {
                Ok(spec) => spec,
                Err(error) => return fail(error),
            };
            let observed =
                framework_bootstrap::machine_enrollment::observe_machine_enrollment_authority(
                    &spec,
                );
            let plan = framework_bootstrap::machine_enrollment::plan_machine_enrollment_authority(
                &spec, &observed,
            );
            emit_json(&plan, plan.status == "blocked")
        }
        BootstrapFrameworkCommand::EnrollmentAuthorityApply {
            json,
            seed_manifest,
            node,
            staging_root,
            registry,
            yes,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry = registry
                .as_deref()
                .unwrap_or(cynic_host::registry::DEFAULT_REGISTRY_PATH);
            let spec = match framework_bootstrap::resolve_local_machine_enrollment_authority_spec(
                Path::new(registry),
                Path::new(&seed_manifest),
                &node,
                Path::new(&staging_root),
            ) {
                Ok(spec) => spec,
                Err(error) => return fail(error),
            };
            let report = framework_bootstrap::apply_machine_enrollment_authority(&spec, yes);
            emit_json(&report, report.status == "blocked")
        }
        BootstrapFrameworkCommand::RuntimeBindingPlan { json, registry } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry = registry
                .as_deref()
                .unwrap_or(cynic_host::registry::DEFAULT_REGISTRY_PATH);
            let spec = match framework_bootstrap::resolve_local_runtime_binding_spec(Path::new(
                registry,
            )) {
                Ok(spec) => spec,
                Err(error) => return fail(error),
            };
            let observed = framework_bootstrap::runtime_binding::observe_runtime_binding(&spec);
            let plan = framework_bootstrap::runtime_binding::plan_runtime_binding(&spec, &observed);
            emit_json(&plan, plan.status == "blocked")
        }
        BootstrapFrameworkCommand::RuntimeBindingApply {
            json,
            registry,
            yes,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry = registry
                .as_deref()
                .unwrap_or(cynic_host::registry::DEFAULT_REGISTRY_PATH);
            let spec = match framework_bootstrap::resolve_local_runtime_binding_spec(Path::new(
                registry,
            )) {
                Ok(spec) => spec,
                Err(error) => return fail(error),
            };
            let report = framework_bootstrap::apply_runtime_binding(&spec, yes, &LocalArgvRunner);
            emit_json(&report, report.status == "blocked")
        }
        BootstrapFrameworkCommand::ProviderEnrollmentPlan {
            json,
            registry,
            token_secret_file,
            ca_file,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry = registry
                .as_deref()
                .unwrap_or(cynic_host::registry::DEFAULT_REGISTRY_PATH);
            let spec = match framework_bootstrap::resolve_local_provider_control_enrollment_spec_from_registry(
                Path::new(registry),
                Path::new(&token_secret_file),
                ca_file.as_deref().map(Path::new),
            ) {
                Ok(spec) => spec,
                Err(error) => return fail(error),
            };
            let observed = framework_bootstrap::observe_provider_control_enrollment(&spec);
            let plan = framework_bootstrap::plan_provider_control_enrollment(&spec, &observed);
            emit_json(&plan, plan.status == "blocked")
        }
        BootstrapFrameworkCommand::ProviderEnrollmentApply {
            json,
            registry,
            token_secret_file,
            ca_file,
            yes,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry = registry
                .as_deref()
                .unwrap_or(cynic_host::registry::DEFAULT_REGISTRY_PATH);
            let spec = match framework_bootstrap::resolve_local_provider_control_enrollment_spec_from_registry(
                Path::new(registry),
                Path::new(&token_secret_file),
                ca_file.as_deref().map(Path::new),
            ) {
                Ok(spec) => spec,
                Err(error) => return fail(error),
            };
            let report = framework_bootstrap::apply_provider_control_enrollment(&spec, yes);
            emit_json(&report, report.status == "blocked")
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
        BootstrapFrameworkCommand::EpochPlan {
            json,
            source_commit,
            desired_state_digest,
            target_scope,
            created_at_unix_ms,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match framework_bootstrap::plan_framework_epoch(
                &source_commit,
                &desired_state_digest,
                &target_scope,
                created_at_unix_ms,
            ) {
                Ok(plan) => emit_json(&plan, plan.status == "blocked"),
                Err(error) => fail(error),
            }
        }
    }
}
