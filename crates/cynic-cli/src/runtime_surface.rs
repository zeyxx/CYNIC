use std::process::ExitCode;

use clap::Subcommand;

use crate::bootstrap_surface::ProfileArgs;

mod host_checks;
mod paths;
mod suite;
mod validation;

#[derive(Subcommand)]
pub(crate) enum RuntimeCommand {
    Paths {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
    Validate {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        lab_registry: Option<String>,
    },
    Suite {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long, default_value = "fleet")]
        profile: String,
    },
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long, default_value = "fleet")]
        profile: String,
        #[arg(long)]
        proof: Option<String>,
    },
    Harness {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        kind: Option<String>,
    },
    Conformance {
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        profile: ProfileArgs,
    },
    Metrics {
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        profile: ProfileArgs,
        #[arg(long)]
        proof: Option<String>,
    },
    RepoAccess {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        profile: Option<String>,
    },
}

pub(crate) fn execute_runtime(command: RuntimeCommand) -> ExitCode {
    match command {
        RuntimeCommand::Paths { json, registry } => paths::execute_paths(json, registry),
        RuntimeCommand::Validate {
            json,
            registry,
            lab_registry,
        } => validation::execute_validate(json, registry, lab_registry),
        RuntimeCommand::Suite {
            json,
            registry,
            profile,
        } => suite::execute_suite(json, registry, profile),
        RuntimeCommand::Plan {
            json,
            registry,
            profile,
            proof,
        } => suite::execute_plan(json, registry, profile, proof),
        RuntimeCommand::Harness { json, kind } => host_checks::execute_harness(json, kind),
        RuntimeCommand::Conformance { json, profile } => {
            host_checks::execute_conformance(json, profile)
        }
        RuntimeCommand::Metrics {
            json,
            profile,
            proof,
        } => host_checks::execute_metrics(json, profile, proof),
        RuntimeCommand::RepoAccess { json, profile } => {
            host_checks::execute_repo_access(json, profile)
        }
    }
}
