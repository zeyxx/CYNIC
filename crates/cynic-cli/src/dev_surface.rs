use clap::Subcommand;
use std::path::PathBuf;
use std::process::ExitCode;

mod audit_persist;
mod audits;
mod build_admission;
mod check;
mod commands;
mod data_centric_audit;
mod inference_gateway;
mod inference_matrix;
mod inference_optimal;
mod manifest;
mod opportunities;
mod organism_audit;
mod surface_matrix;

use audits::{
    execute_agent_exposure, execute_crate_audit, execute_debt_audit, execute_kernel_audit,
    execute_ownership_audit, execute_policy_formality_audit, execute_schema_registry_audit,
    execute_surface_audit, execute_surface_matrix,
};
use commands::{
    AuditArgs, BuildAdmissionArgs, CheckArgs, InferenceMatrixArgs, InferenceOptimalArgs,
    OpportunitiesArgs,
};
use manifest::{execute_framework, execute_kpi_score, execute_outcomes, execute_surfaces};
use opportunities::execute_opportunities;

#[derive(Subcommand)]
pub(crate) enum DevCommand {
    /// Validate or explicitly generate the canonical surface registry.
    SurfaceRegistry {
        #[arg(long, conflicts_with = "write")]
        check: bool,
        #[arg(long, conflicts_with = "check")]
        write: bool,
        #[arg(long, default_value = "infra/surface-registry/manifest.json")]
        manifest: std::path::PathBuf,
        #[arg(long, default_value = "infra/surface-registry.json")]
        output: std::path::PathBuf,
    },
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
        #[arg(long)]
        markdown: bool,
    },
    CrateAudit(AuditArgs),
    KernelAudit(AuditArgs),
    OwnershipAudit(AuditArgs),
    SurfaceAudit(AuditArgs),
    PolicyFormalityAudit(AuditArgs),
    SchemaRegistryAudit(AuditArgs),
    SurfaceMatrix(AuditArgs),
    AgentExposure(AuditArgs),
    Check(CheckArgs),
    BuildAdmission(BuildAdmissionArgs),
    InferenceMatrix(InferenceMatrixArgs),
    InferenceOptimal(InferenceOptimalArgs),
    DataCentricAudit {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        ledger: Option<PathBuf>,
        #[arg(long)]
        vault_dir: Option<PathBuf>,
    },
    FleetDivergence {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long = "name")]
        names: Vec<String>,
    },
    OrganismAudit {
        #[arg(long)]
        json: bool,
    },
    DebtAudit(AuditArgs),
    Opportunities(OpportunitiesArgs),
    InferenceGateway {
        #[arg(long)]
        listen: Option<String>,
        #[arg(long)]
        backend_port: Option<u16>,
        #[arg(long)]
        default_model: Option<String>,
        #[arg(long)]
        vault_dir: Option<PathBuf>,
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
}

pub(crate) fn execute_dev(command: DevCommand) -> ExitCode {
    match command {
        DevCommand::SurfaceRegistry {
            check,
            write,
            manifest,
            output,
        } => {
            let _ = check;
            let result = if write {
                crate::surface_registry::write(&manifest, &output)
            } else {
                crate::surface_registry::check(&manifest, &output)
            };
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => crate::fail(format!("surface-registry: {error}")),
            }
        }
        DevCommand::Surfaces { json } => execute_surfaces(json),
        DevCommand::Outcomes { json } => execute_outcomes(json),
        DevCommand::KpiScore { json } => execute_kpi_score(json),
        DevCommand::Framework { json, markdown } => execute_framework(json, markdown),
        DevCommand::CrateAudit(a) => execute_crate_audit(a.json, a.output, a.persist),
        DevCommand::KernelAudit(a) => execute_kernel_audit(a.json, a.output, a.persist),
        DevCommand::OwnershipAudit(a) => execute_ownership_audit(a.json, a.output, a.persist),
        DevCommand::SurfaceAudit(a) => execute_surface_audit(a.json, a.output, a.persist),
        DevCommand::PolicyFormalityAudit(a) => {
            execute_policy_formality_audit(a.json, a.output, a.persist)
        }
        DevCommand::SchemaRegistryAudit(a) => {
            execute_schema_registry_audit(a.json, a.output, a.persist)
        }
        DevCommand::SurfaceMatrix(a) => execute_surface_matrix(a.json, a.output, a.persist),
        DevCommand::AgentExposure(a) => execute_agent_exposure(a.json, a.output, a.persist),
        DevCommand::Check(a) => {
            if !a.json {
                return ExitCode::FAILURE;
            }
            match check::run_dev_check(None, a.target_root, a.architecture_only, a.layered) {
                Ok(report) => {
                    let failed = report.status != "ok";
                    crate::emit_json(&report, failed)
                }
                Err(error) => crate::fail(error),
            }
        }
        DevCommand::BuildAdmission(a) => {
            if !a.json {
                return ExitCode::FAILURE;
            }
            let report = build_admission::observe();
            crate::emit_json(&report, report.status != "ready")
        }
        DevCommand::InferenceMatrix(a) => inference_matrix::run(
            a.json,
            a.model,
            a.params_billions,
            a.context_filter,
            a.execute,
            a.sample,
            a.ledger,
            a.output,
        ),
        DevCommand::InferenceOptimal(a) => {
            inference_optimal::run(a.json, a.model, a.context, a.goal, a.ledger, a.export_toml)
        }
        DevCommand::DataCentricAudit {
            json,
            ledger,
            vault_dir,
        } => data_centric_audit::run(json, ledger, vault_dir),
        DevCommand::FleetDivergence {
            json,
            registry,
            names,
        } => crate::fleet_divergence::run(json, registry, names),
        DevCommand::OrganismAudit { json } => organism_audit::run(json),
        DevCommand::DebtAudit(a) => execute_debt_audit(a.json, a.output, a.persist),
        DevCommand::Opportunities(a) => {
            execute_opportunities(a.json, a.frame, a.output, a.decide, a.action, a.reason)
        }
        DevCommand::InferenceGateway {
            listen,
            backend_port,
            default_model,
            vault_dir,
            ledger,
        } => inference_gateway::run(listen, backend_port, default_model, vault_dir, ledger),
    }
}
