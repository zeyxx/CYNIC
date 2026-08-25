use clap::Subcommand;
use std::process::ExitCode;

use crate::agent_exposure_audit::audit_agent_exposure;
use crate::crate_audit::audit_core_crates;
use crate::emit_json;
use crate::kernel_boundary_audit::audit_kernel_surfaces;
use crate::ownership_audit::audit_ownership_surfaces;
use crate::policy_formality_audit::audit_policy_formality;
use crate::surface_audit::audit_canonical_surfaces;

mod audit_persist;
mod check;
mod data_centric_audit;
mod inference_gateway;
mod inference_matrix;
mod inference_optimal;
mod manifest;
mod organism_audit;
mod surface_matrix;

use audit_persist::{maybe_persist_dev_audit_report, maybe_persist_kernel_audit_report};
use manifest::{execute_framework, execute_kpi_score, execute_outcomes, execute_surfaces};

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
        #[arg(long)]
        markdown: bool,
    },
    CrateAudit {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        persist: bool,
    },
    KernelAudit {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        persist: bool,
    },
    OwnershipAudit {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        persist: bool,
    },
    SurfaceAudit {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        persist: bool,
    },
    /// Audit whether each declarative CYNIC policy is typed law or prose-only.
    PolicyFormalityAudit {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        persist: bool,
    },
    SurfaceMatrix {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        persist: bool,
    },
    AgentExposure {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        persist: bool,
    },
    Check {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        architecture_only: bool,
        #[arg(long)]
        layers: bool,
        #[arg(long)]
        source_root: Option<std::path::PathBuf>,
        #[arg(long)]
        target_root: Option<std::path::PathBuf>,
    },
    /// Exhaustive parametric sweep: context tiers, KV-quant, threads, topologies, harnesses.
    InferenceMatrix {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        model: std::path::PathBuf,
        #[arg(long)]
        params_billions: f32,
        #[arg(long)]
        context_filter: Option<String>,
        #[arg(long)]
        execute: bool,
        #[arg(long)]
        sample: Option<usize>,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// Queries the empirical ledger and solves for the optimal verified execution vector.
    InferenceOptimal {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        model: String,
        #[arg(long)]
        context: Option<String>,
        #[arg(long, default_value = "balanced")]
        goal: String,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        export_toml: Option<std::path::PathBuf>,
    },
    /// Audit data-centric compliance (ledger presence, model coverage, OOM safety proofs).
    DataCentricAudit {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
        #[arg(long)]
        vault_dir: Option<std::path::PathBuf>,
    },
    /// Audit registry claims against the observed substrate (read-only).
    ///
    /// Refuted claims exit non-zero; unobservable ones do not — unknown is not
    /// a fault.
    FleetDivergence {
        #[arg(long)]
        json: bool,
        /// Registry to audit. Defaults to the repo-relative infra/registry.json.
        #[arg(long)]
        registry: Option<std::path::PathBuf>,
        /// Names to check for resolution consistency. Repeatable.
        #[arg(long = "name")]
        names: Vec<String>,
    },
    /// Audit organism liveness, metabolic loops, storage health, and domain task queue.
    OrganismAudit {
        #[arg(long)]
        json: bool,
    },
    /// Start and supervise the sovereign HTTP inference gateway.
    InferenceGateway {
        #[arg(long)]
        listen: Option<String>,
        #[arg(long)]
        backend_port: Option<u16>,
        #[arg(long)]
        default_model: Option<String>,
        #[arg(long)]
        vault_dir: Option<std::path::PathBuf>,
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
    },
}

pub(crate) fn execute_dev(command: DevCommand) -> ExitCode {
    match command {
        DevCommand::Surfaces { json } => execute_surfaces(json),
        DevCommand::Outcomes { json } => execute_outcomes(json),
        DevCommand::KpiScore { json } => execute_kpi_score(json),
        DevCommand::Framework { json, markdown } => execute_framework(json, markdown),
        DevCommand::CrateAudit {
            json,
            output,
            persist,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let report = audit_core_crates();
            if let Err(error) =
                maybe_persist_dev_audit_report(&report, output, persist, "crate-audit.json")
            {
                return crate::fail(error);
            }
            emit_json(&report, report.status != "ok")
        }
        DevCommand::KernelAudit {
            json,
            output,
            persist,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let report = audit_kernel_surfaces();
            if let Err(error) = maybe_persist_kernel_audit_report(&report, output, persist) {
                return crate::fail(error);
            }
            emit_json(&report, report.status != "ok")
        }
        DevCommand::OwnershipAudit {
            json,
            output,
            persist,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let report = audit_ownership_surfaces();
            if let Err(error) =
                maybe_persist_dev_audit_report(&report, output, persist, "ownership-audit.json")
            {
                return crate::fail(error);
            }
            emit_json(&report, report.status != "ok")
        }
        DevCommand::SurfaceAudit {
            json,
            output,
            persist,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let report = audit_canonical_surfaces();
            if let Err(error) =
                maybe_persist_dev_audit_report(&report, output, persist, "surface-audit.json")
            {
                return crate::fail(error);
            }
            emit_json(&report, report.status != "ok")
        }
        DevCommand::PolicyFormalityAudit {
            json,
            output,
            persist,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let report = audit_policy_formality();
            if let Err(error) = maybe_persist_dev_audit_report(
                &report,
                output,
                persist,
                "policy-formality-audit.json",
            ) {
                return crate::fail(error);
            }
            emit_json(&report, report.status != "ok")
        }
        DevCommand::SurfaceMatrix {
            json,
            output,
            persist,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            surface_matrix::execute_surface_matrix(json, output, persist)
        }
        DevCommand::AgentExposure {
            json,
            output,
            persist,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let report = audit_agent_exposure();
            if let Err(error) = maybe_persist_dev_audit_report(
                &report,
                output,
                persist,
                "agent-exposure-audit.json",
            ) {
                return crate::fail(error);
            }
            emit_json(&report, report.status != "ok")
        }
        DevCommand::Check {
            json: _,
            architecture_only,
            layers,
            source_root,
            target_root,
        } => {
            if architecture_only && layers {
                eprintln!("error: --architecture-only and --layers are mutually exclusive");
                return ExitCode::FAILURE;
            }
            match check::run_dev_check(source_root, target_root, architecture_only, layers) {
                Ok(report) => emit_json(&report, report.status != "ok"),
                Err(error) => crate::fail(error),
            }
        }
        DevCommand::InferenceMatrix {
            json,
            model,
            params_billions,
            context_filter,
            execute,
            sample,
            ledger,
            output,
        } => inference_matrix::run(
            json,
            model,
            params_billions,
            context_filter,
            execute,
            sample,
            ledger,
            output,
        ),
        DevCommand::InferenceOptimal {
            json,
            model,
            context,
            goal,
            ledger,
            export_toml,
        } => inference_optimal::run(json, model, context, goal, ledger, export_toml),
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
        DevCommand::InferenceGateway {
            listen,
            backend_port,
            default_model,
            vault_dir,
            ledger,
        } => inference_gateway::run(listen, backend_port, default_model, vault_dir, ledger),
    }
}
