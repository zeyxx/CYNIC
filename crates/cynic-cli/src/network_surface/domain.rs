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
//! Authority: `contracts/foundation.json` → `portable_host_contract`.
//! The thin-facade shape is a projection property, not an AGENTS.md section.

use std::process::ExitCode;

use clap::Subcommand;
use cynic_domain_ops::{
    env_secret, parse_desired_state, run_domain_ops, CloudflareClient, DnsAuthorityPort,
    EdgeProbePort, PorkbunRegistrar,
};
use cynic_domain_ops::ProofStatus;
use cynic_secrets::CompositeSecretProvider;

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

    // 3. Construct adapters from credentials — resolved via CompositeSecretProvider.
    // V2: env → file → OS keyring. Secrets are NEVER inline.
    let secrets = CompositeSecretProvider::default();

    // Porkbun registrar — credentials resolved through the SecretProvider port.
    // The adapter needs api_key + api_secret. `credential_secret` carries the
    // second reference; when absent we keep the historical single-credential
    // fallback (both resolve to the same env/file value) for backwards compat.
    let credential_secret_ref = desired
        .registrar
        .credential_secret
        .as_ref()
        .unwrap_or(&desired.registrar.credential);
    let registrar = match PorkbunRegistrar::from_secret_refs(
        &desired.registrar.credential,
        credential_secret_ref,
        &secrets,
    ) {
        Ok(r) => r,
        Err(e) => {
            return fail(format!(
                "porkbun credential resolution failed for '{}': {}",
                desired.registrar.credential.locator, e
            ));
        }
    };

    // 3. DNS authority and EdgeProbe resolution
    let (dns_authority, probe): (Box<dyn DnsAuthorityPort>, Box<dyn EdgeProbePort>) =
        match desired.dns_authority.provider.as_str() {
            "porkbun" => {
                let porkbun_auth = match PorkbunRegistrar::from_secret_refs(
                    &desired.dns_authority.credential,
                    &desired
                        .registrar
                        .credential_secret
                        .as_ref()
                        .unwrap_or(&desired.dns_authority.credential),
                    &secrets,
                ) {
                    Ok(p) => p,
                    Err(e) => {
                        return fail(format!("porkbun dns authority credential resolution failed: {}", e));
                    }
                };
                let boxed = Box::new(porkbun_auth);
                (boxed.clone(), boxed)
            }
            _ => {
                let cf_account_ref = desired
                    .dns_authority_account_id
                    .clone()
                    .unwrap_or_else(|| env_secret("cf-account-id", "CF_ACCOUNT_ID"));

                let cloudflare = match CloudflareClient::from_secret_refs(
                    &desired.dns_authority.credential,
                    &cf_account_ref,
                    &secrets,
                    desired.expected_nameservers.clone(),
                ) {
                    Ok(c) => c,
                    Err(e) => {
                        return fail(format!("cloudflare credential resolution failed: {}", e));
                    }
                };
                (Box::new(cloudflare.clone()), Box::new(cloudflare))
            }
        };

    // 4. Run domain operations (delegates all logic to the runner)
    let proofs = run_domain_ops(
        &desired,
        &registrar,
        dns_authority.as_ref(),
        probe.as_ref(),
        &secrets,
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
