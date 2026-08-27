//! CLI surface for `cynic runtime forgejo-token`.
//!
//! Thin projection over the host-side `forgejo_token` orchestration module.
//! Observes file presence + API validity through the injected port, projects
//! a convergence plan, and applies (via --yes) through `LocalForgejoTokenApiAdapter`.
//!
//! No Forgejo admin login secret is required: the enrolled machine principal
//! (`cynic-cortex`) creates tokens through the runtime SSH-local Exec adapter.

use std::process::ExitCode;

use cynic_host::forgejo_token::{
    DefaultForgejoTokenSpec, ForgejoTokenRuntimeSpec, observe_forgejo_token_runtime,
    provision_forgejo_token_apply, rotate_forgejo_token,
};
use cynic_protocol::forgejo_token::{ForgejoTokenAuthState, ForgejoTokenStatusReport};

use crate::cli_json;
use crate::runtime_surface::ForgejoTokenMode;

const FORGEJO_TOKEN_STATUS_REPORT_SCHEMA: &str = "cynic.forgejo_token_status.v1";

/// Entry point for `cynic runtime forgejo-token <mode>`.
pub(crate) fn execute_forgejo_token(
    mode: crate::runtime_surface::ForgejoTokenMode,
    json: bool,
    yes: bool,
    token_path: Option<String>,
) -> ExitCode {
    let token_spec = if let Some(p) = token_path {
        DefaultForgejoTokenSpec::with_token_path(&p)
    } else {
        DefaultForgejoTokenSpec::default_with_principal("cynic-cortex".to_string())
    };
    let spec = ForgejoTokenRuntimeSpec {
        token_spec,
        // Local-forgejo (SSH-local Exec) adapter by default. The HTTP variant
        // is selected automatically when the HTTPS endpoint is reachable.
        adapter: super::forgejo_token_adapter_selector(),
    };

    match mode {
        ForgejoTokenMode::Status => execute_token_status(&spec, json, yes),
        ForgejoTokenMode::Create => execute_token_provision(&spec, json, yes, "create"),
        ForgejoTokenMode::Rotate => execute_token_rotate(&spec, json, yes),
    }
}

/// Read-only status: observe token presence + API validity, emit plan.
fn execute_token_status(spec: &ForgejoTokenRuntimeSpec, json: bool, _yes: bool) -> ExitCode {
    let observation = observe_forgejo_token_runtime(spec);
    let plan = cynic_protocol::forgejo_token::plan_forgejo_token_convergence(
        spec.token_spec.clone(),
        &observation,
    );
    let report = ForgejoTokenStatusReport {
        schema: FORGEJO_TOKEN_STATUS_REPORT_SCHEMA.to_string(),
        principal: spec.token_spec.principal.clone(),
        token: observation.clone(),
        forgejo_plan_status: plan.status.clone(),
        token_action_needed: observation.api_authenticated != ForgejoTokenAuthState::Proven,
        requires_human_admin_secret: false,
        plan_action: if plan.actions.is_empty() {
            "observe".to_string()
        } else {
            format!("{:?}", plan.actions[0].kind)
        },
        evidence_refs: observation.evidence_refs.clone(),
    };

    if json {
        let _ = cli_json::emit_pretty_json(&report);
    } else {
        print_token_status(&report);
    }

    if observation.api_authenticated == ForgejoTokenAuthState::Proven {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Mutation: create or rotate the token (requires --yes).
fn execute_token_provision(
    spec: &ForgejoTokenRuntimeSpec,
    json: bool,
    yes: bool,
    operation: &str,
) -> ExitCode {
    if !yes {
        if json {
            let _ = cli_json::emit_pretty_json(&serde_json::json!({
                "schema": "cynic.forgejo_token_provision.v1",
                "status": "blocked",
                "principal": spec.token_spec.principal,
                "reason": "mutation requires --yes consent",
                "operation": operation,
                "requires_human_admin_secret": false,
            }));
        } else {
            eprintln!("Refusing mutation: pass --yes to {operation} the Forgejo token.");
        }
        return ExitCode::FAILURE;
    }

    let report = provision_forgejo_token_apply(spec);

    if json {
        let _ = cli_json::emit_pretty_json(&report);
    } else {
        print_token_provision(&report);
    }

    if report.status == "completed" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Mutation: create or rotate the token (requires `--yes`).
fn execute_token_rotate(spec: &ForgejoTokenRuntimeSpec, json: bool, yes: bool) -> ExitCode {
    if !yes {
        if json {
            let _ = cli_json::emit_pretty_json(&serde_json::json!({
                "schema": "cynic.forgejo_token_provision.v1",
                "status": "blocked",
                "principal": spec.token_spec.principal,
                "reason": "mutation requires --yes consent",
                "operation": "rotate",
            }));
        } else {
            eprintln!("Refusing mutation: pass --yes to rotate the Forgejo token.");
        }
        return ExitCode::FAILURE;
    }
    let report = rotate_forgejo_token(&spec.token_spec, &*spec.adapter);
    if json {
        let _ = cli_json::emit_pretty_json(&report);
    } else {
        print_token_provision(&report);
    }
    if report.status == "completed" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Human-readable projection for `status` mode.
fn print_token_status(report: &ForgejoTokenStatusReport) {
    println!("Forgejo token status (principal: {})", report.principal);
    println!("  path:          {}", report.token.path);
    println!("  secret_present: {}", report.token.secret_present);
    println!(
        "  owner_root:    {}",
        report
            .token
            .owner_root
            .map_or("unknown".to_string(), |v| v.to_string())
    );
    println!(
        "  mode_o640:     {}",
        report
            .token
            .mode_o640
            .map_or("unknown".to_string(), |v| v.to_string())
    );
    println!("  api_authenticated: {:?}", report.token.api_authenticated);
    println!("  action_needed: {}", report.token_action_needed);
    println!("  plan_action:   {}", report.plan_action);
    if !report.token.unknowns.is_empty() {
        println!("  unknowns:");
        for u in &report.token.unknowns {
            println!("    - {u}");
        }
    }
}

/// Human-readable projection for `create`/`rotate` mode.
fn print_token_provision(report: &cynic_protocol::forgejo_token::ForgejoTokenProvisionReport) {
    println!("Forgejo token provision (principal: {})", report.principal);
    println!("  status:   {}", report.status);
    println!("  mutated:  {}", report.mutated);
    println!(
        "  before.api_authenticated: {:?}",
        report.before.api_authenticated
    );
    println!(
        "  after.api_authenticated:  {:?}",
        report.after.api_authenticated
    );
    if let Some(meta) = &report.token_metadata {
        println!("  token metadata: id={} name={}", meta.token_id, meta.name);
    }
    if !report.proofs.is_empty() {
        println!("  proofs:");
        for p in &report.proofs {
            println!("    - {p}");
        }
    }
    if !report.unknowns.is_empty() {
        println!("  unknowns:");
        for u in &report.unknowns {
            println!("    - {u}");
        }
    }
    println!("  rollback: {}", report.rollback_intent);
}
