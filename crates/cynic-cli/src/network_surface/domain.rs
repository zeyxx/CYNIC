//! `cynic network domain` — thin CLI facade for external domain operations.
//!
//! All domain logic lives in `cynic-domain-ops::runner`. This module:
//! 1. Reads desired-state JSON file (machine-readable contract)
//! 2. Constructs provider adapters from credentials (env-injected, never inline)
//! 3. Delegates to `run_domain_ops` for observe → reconcile → act → prove → verify
//! 4. Emits proof chain as JSON
//!
//! Consent gates:
//! - `plan`/`status`/`verify`: apply=false (observe-only)
//! - `apply --yes`: apply=true (execute mutations)
//! - `apply --ns-cutover`: also sets ns_cutover_confirmed=true
//!
//! Authority: foundation.json §37 (portable_host_contract), AGENTS.md §CLI thin facade.

use std::process::ExitCode;

use clap::Subcommand;
use cynic_domain_cloudflare::CloudflareClient;
use cynic_domain_ops::{SecretProvider, parse_desired_state, run_domain_ops};
use cynic_domain_porkbun::{EnvSecretProvider, PorkbunRegistrar};
use cynic_protocol::domain_ops::ProofStatus;

use crate::network_surface::NetworkCommand;
use crate::runtime_paths::RuntimePaths;
use crate::{emit_json, fail};

#[derive(Subcommand)]
pub(crate) enum NetworkDomainCommand {
    /// Generate a domain operation plan (read-only, observe-only).
    Plan {
        #[arg(long)]
        desired_state: String,
    },
    /// Execute domain operations with explicit consent.
    ///
    /// `--yes` authorizes paid operations (domain registration, zone writes).
    /// `--ns-cutover` authorizes nameserver delegation changes (separate gate).
    Apply {
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        ns_cutover: bool,
        #[arg(long)]
        desired_state: String,
    },
    /// Read-only domain status snapshot (current state, no mutations).
    Status {
        #[arg(long)]
        desired_state: String,
    },
    /// Verify domain delegation and edge HTTP state (read-only).
    Verify {
        #[arg(long)]
        desired_state: String,
    },
}

pub(crate) fn try_execute_domain(
    command: NetworkCommand,
    _runtime: &RuntimePaths,
) -> Result<ExitCode, NetworkCommand> {
    let NetworkCommand::Domain {
        command: domain_cmd,
    } = command
    else {
        return Err(command);
    };

    let exit = match domain_cmd {
        NetworkDomainCommand::Plan { desired_state } => {
            execute_domain(&desired_state, false, false)
        }
        NetworkDomainCommand::Apply {
            yes,
            ns_cutover,
            desired_state,
        } => execute_domain(&desired_state, yes, ns_cutover),
        NetworkDomainCommand::Status { desired_state } => {
            execute_domain(&desired_state, false, false)
        }
        NetworkDomainCommand::Verify { desired_state } => {
            execute_domain(&desired_state, false, false)
        }
    };

    Ok(exit)
}

fn execute_domain(desired_state_path: &str, apply: bool, ns_cutover_confirmed: bool) -> ExitCode {
    // 1. Read desired-state contract (machine-readable JSON)
    let json = match std::fs::read_to_string(desired_state_path) {
        Ok(j) => j,
        Err(e) => {
            return fail(format!(
                "cannot read desired state file '{desired_state_path}': {e}"
            ));
        }
    };

    // 2. Parse + validate desired state
    let desired = match parse_desired_state(&json) {
        Ok(d) => d,
        Err(e) => return fail(format!("invalid desired state: {e}")),
    };

    // 3. Construct adapters from credentials (env-injected, never inline)
    let env_provider = EnvSecretProvider;

    // Porkbun registrar — credentials resolved via SecretProvider port.
    // V1: same SecretReference used for both key and secret (Porkbun API key
    // doubles as secret). Separate SecretReference fields reserved for V2.
    let registrar = match PorkbunRegistrar::from_secret_refs(
        &desired.registrar.credential,
        &desired.registrar.credential,
        &env_provider,
    ) {
        Ok(r) => r,
        Err(e) => {
            return fail(format!(
                "porkbun credential resolution failed for '{}': {}",
                desired.registrar.credential.locator, e
            ));
        }
    };

    // Cloudflare client — API token from SecretReference, account ID from env.
    let cf_token = match env_provider.resolve(&desired.dns_authority.credential) {
        Some(v) => v,
        None => {
            return fail(format!(
                "cloudflare credential not resolved: env var '{}' is not set",
                desired.dns_authority.credential.locator
            ));
        }
    };
    let cf_account_id = std::env::var("CF_ACCOUNT_ID").unwrap_or_default();
    let cloudflare = CloudflareClient::new(
        cf_token,
        cf_account_id,
        desired.expected_nameservers.clone(),
    );

    // 4. Run domain operations (delegates all logic to the runner)
    // CloudflareClient implements both CloudflareDnsPort and EdgeProbePort,
    // so the same instance serves as both dns port and probe port.
    let proofs = run_domain_ops(
        &desired,
        &registrar,
        &cloudflare,
        &cloudflare,
        &env_provider,
        apply,
        ns_cutover_confirmed,
    );

    // 5. Emit proof chain as JSON
    let output = serde_json::json!({
        "domain": &desired.domain,
        "apply": apply,
        "ns_cutover_confirmed": ns_cutover_confirmed,
        "proofs": &proofs,
    });

    let has_violations = proofs.iter().any(|p| p.status == ProofStatus::Violates);

    emit_json(&output, has_violations)
}
