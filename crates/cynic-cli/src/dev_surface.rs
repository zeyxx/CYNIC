use clap::Subcommand;
use std::process::ExitCode;

use crate::agent_exposure_audit::audit_agent_exposure;
use crate::crate_audit::audit_core_crates;
use crate::emit_json;
use crate::kernel_boundary_audit::audit_kernel_surfaces;
use crate::ownership_audit::audit_ownership_surfaces;
use crate::surface_audit::audit_canonical_surfaces;

mod audit_persist;
mod check;
mod manifest;
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
        source_root: Option<std::path::PathBuf>,
        #[arg(long)]
        target_root: Option<std::path::PathBuf>,
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
            json,
            architecture_only,
            source_root,
            target_root,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match check::run_dev_check(source_root, target_root, architecture_only) {
                Ok(report) => emit_json(&report, report.status != "ok"),
                Err(error) => crate::fail(error),
            }
        }
    }
}
