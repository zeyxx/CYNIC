use std::process::ExitCode;

use crate::agent_exposure_audit::audit_agent_exposure;
use crate::crate_audit::audit_core_crates;
use crate::debt_audit::audit_debt;
use crate::emit_json;
use crate::kernel_boundary_audit::audit_kernel_surfaces;
use crate::ownership_audit::audit_ownership_surfaces;
use crate::policy_formality_audit::audit_policy_formality;
use crate::schema_registry_audit::audit_schema_registry;
use crate::surface_audit::audit_canonical_surfaces;

use super::audit_persist::{maybe_persist_dev_audit_report, maybe_persist_kernel_audit_report};
use super::surface_matrix;

pub(crate) fn execute_crate_audit(
    json: bool,
    output: Option<std::path::PathBuf>,
    persist: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let report = audit_core_crates();
    if let Err(error) = maybe_persist_dev_audit_report(&report, output, persist, "crate-audit.json")
    {
        return crate::fail(error);
    }
    emit_json(&report, report.status != "ok")
}

pub(crate) fn execute_kernel_audit(
    json: bool,
    output: Option<std::path::PathBuf>,
    persist: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let report = audit_kernel_surfaces();
    if let Err(error) = maybe_persist_kernel_audit_report(&report, output, persist) {
        return crate::fail(error);
    }
    emit_json(&report, report.status != "ok")
}

pub(crate) fn execute_ownership_audit(
    json: bool,
    output: Option<std::path::PathBuf>,
    persist: bool,
) -> ExitCode {
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

pub(crate) fn execute_surface_audit(
    json: bool,
    output: Option<std::path::PathBuf>,
    persist: bool,
) -> ExitCode {
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

pub(crate) fn execute_policy_formality_audit(
    json: bool,
    output: Option<std::path::PathBuf>,
    persist: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let report = audit_policy_formality();
    if let Err(error) =
        maybe_persist_dev_audit_report(&report, output, persist, "policy-formality-audit.json")
    {
        return crate::fail(error);
    }
    emit_json(&report, report.status != "ok")
}

pub(crate) fn execute_schema_registry_audit(
    json: bool,
    output: Option<std::path::PathBuf>,
    persist: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let report = audit_schema_registry();
    if let Err(error) =
        maybe_persist_dev_audit_report(&report, output, persist, "schema-registry-audit.json")
    {
        return crate::fail(error);
    }
    emit_json(&report, report.status != "ok")
}

pub(crate) fn execute_surface_matrix(
    json: bool,
    output: Option<std::path::PathBuf>,
    persist: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    surface_matrix::execute_surface_matrix(json, output, persist)
}

pub(crate) fn execute_agent_exposure(
    json: bool,
    output: Option<std::path::PathBuf>,
    persist: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let report = audit_agent_exposure();
    if let Err(error) =
        maybe_persist_dev_audit_report(&report, output, persist, "agent-exposure.json")
    {
        return crate::fail(error);
    }
    emit_json(&report, report.status != "ok")
}

pub(crate) fn execute_debt_audit(
    json: bool,
    output: Option<std::path::PathBuf>,
    persist: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let report = audit_debt();
    if let Err(error) = maybe_persist_dev_audit_report(&report, output, persist, "debt-audit.json")
    {
        return crate::fail(error);
    }
    emit_json(&report, report.status != "ok")
}
