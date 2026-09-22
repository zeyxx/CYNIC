//! Domain operations runner — orchestrates ports and pure domain logic.
//!
//! Phase 1: stub. Full orchestration is Phase 4.
//!
//! The runner is the ONLY place that coordinates between ports and domain logic.
//! It must never contain business logic itself — only orchestration:
//! claim → runbook → observe → reconcile → bounded action (with consent
//! gates) → read-back proof.
//!
//! Authority: foundation.json §37 (portable_host_contract).

use crate::port::{CloudflareDnsPort, EdgeProbePort, RegistrarPort, SecretProvider};
use cynic_protocol::domain_ops::{DomainOperationProof, DomainOpsDesiredState};

/// Execute the domain operations workflow from a desired-state contract.
///
/// Phase 1 stub: returns an empty proof vector. The full implementation
/// (Phase 4) will wire the pure domain logic (transform/reconcile/idempotency)
/// between the injected provider ports, enforcing consent gates:
/// - `apply=false` → observe-only, no mutations
/// - `ns_cutover_confirmed=false` → refuses NS cutover writes
///
/// # Arguments
/// * `desired` — machine-readable runbook + desired state
/// * `registrar` — Porkbun/other registrar port
/// * `cloudflare` — Cloudflare DNS port
/// * `probe` — authoritative DNS + edge HTTP probe port
/// * `secrets` — resolves `SecretReference` to actual secret values
/// * `apply` — master consent flag for any mutation
/// * `ns_cutover_confirmed` — separate consent gate for nameserver changes
///
/// # Returns
/// Vec of `DomainOperationProof` — one per operation executed.
pub fn run_domain_ops(
    _desired: &DomainOpsDesiredState,
    _registrar: &dyn RegistrarPort,
    _cloudflare: &dyn CloudflareDnsPort,
    _probe: &dyn EdgeProbePort,
    _secrets: &dyn SecretProvider,
    _apply: bool,
    _ns_cutover_confirmed: bool,
) -> Vec<DomainOperationProof> {
    // Phase 4: orchestration of domain logic + ports with consent gates.
    Vec::new()
}
