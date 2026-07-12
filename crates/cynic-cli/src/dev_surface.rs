use std::process::ExitCode;

use clap::Subcommand;
use cynic_protocol::{
    DEVELOPMENT_SURFACE_MANIFEST_SCHEMA, FRAMEWORK_SPEC_SCHEMA, OUTCOME_SPEC_SCHEMA,
    RUNTIME_EXECUTION_PLAN_SCHEMA, RUNTIME_EXECUTION_SUITE_SCHEMA, RUNTIME_PROOF_BUNDLE_SCHEMA,
    RUNTIME_PROOF_SCHEMA, development_surface_manifest, framework_spec, outcome_spec,
    score_outcomes,
};

use crate::command_manifest::commands_manifest;
use crate::emit_json;
use crate::surface_audit::audit_canonical_surfaces;

#[derive(Subcommand)]
pub(crate) enum DevCommand {
    Surfaces {
        #[arg(long)]
        json: bool,
    },
    Outcomes {
        #[arg(long)]
        json: bool,
    },
    KpiScore {
        #[arg(long)]
        json: bool,
    },
    Framework {
        #[arg(long)]
        json: bool,
    },
    SurfaceAudit {
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_dev(command: DevCommand) -> ExitCode {
    match command {
        DevCommand::Surfaces { json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            emit_json(&development_surface_manifest(), false)
        }
        DevCommand::Outcomes { json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            emit_json(&outcome_spec(), false)
        }
        DevCommand::KpiScore { json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let spec = outcome_spec();
            let evidence = available_evidence();
            let commands = available_development_commands();
            emit_json(&score_outcomes(&spec, &evidence, &commands), false)
        }
        DevCommand::Framework { json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            emit_json(&framework_spec(), false)
        }
        DevCommand::SurfaceAudit { json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let report = audit_canonical_surfaces();
            emit_json(&report, report.status != "ok")
        }
    }
}

fn available_evidence() -> Vec<&'static str> {
    let manifest = development_surface_manifest();
    let mut evidence = vec![
        DEVELOPMENT_SURFACE_MANIFEST_SCHEMA,
        FRAMEWORK_SPEC_SCHEMA,
        OUTCOME_SPEC_SCHEMA,
        RUNTIME_EXECUTION_SUITE_SCHEMA,
        RUNTIME_EXECUTION_PLAN_SCHEMA,
        RUNTIME_PROOF_SCHEMA,
        RUNTIME_PROOF_BUNDLE_SCHEMA,
    ];
    evidence.extend(
        manifest
            .surfaces
            .iter()
            .flat_map(|surface| surface.exit_evidence.iter().copied()),
    );
    evidence
}

fn available_development_commands() -> Vec<&'static str> {
    let manifest = development_surface_manifest();
    let commands = commands_manifest();
    let mut available = commands
        .domains
        .iter()
        .flat_map(|domain| domain.commands.iter().copied())
        .collect::<Vec<_>>();
    available.extend(
        manifest
            .surfaces
            .iter()
            .flat_map(|surface| surface.canonical_commands.iter().copied()),
    );
    available
}
