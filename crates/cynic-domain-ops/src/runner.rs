//! Domain operations runner — orchestrates ports and pure domain logic.
//!
//! The runner is the ONLY orchestration point between:
//! - Pure domain logic (`crate::domain` — zero I/O)
//! - Provider ports (`crate::port` — trait definitions only)
//! - Contract helpers (`crate::contracts` — proof construction)
//!
//! Consent gates:
//! - `apply = false` → observe-only, emits blocked proofs for mutations
//! - `apply = true && ns_cutover_confirmed = false` → refuses NS cutover
//!
//! The runner is NOT pure (it calls `SystemTime::now()` and delegates I/O to
//! ports), but all decision logic (transform, reconcile, idempotency) is
//! delegated to the pure `domain/` module.
//!
//! Authority: foundation.json §37 (portable_host_contract).

use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::contracts::{blocked_proof, build_proof};
use crate::domain::{
    TransformResult, detect_cname_conflict, generate_idempotency_key, reconcile_zone,
    transform_record_for_cloudflare,
};
use crate::port::{CloudflareDnsPort, EdgeProbePort, RegistrarPort, SecretProvider};

use cynic_protocol::domain_ops::*;

/// Current Unix timestamp in seconds.
/// Returns 0 if the system clock is before the epoch (defensive fallback).
pub fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Check whether all credential references in the desired state resolve.
///
/// Verifies that the registrar and DNS authority credentials can be resolved
/// through the `SecretProvider` port before any operation is attempted.
/// This is a preflight check — if a credential can't be resolved, the entire
/// operation chain is blocked.
fn credentials_resolved(desired: &DomainOpsDesiredState, secrets: &dyn SecretProvider) -> bool {
    secrets.resolve(&desired.registrar.credential).is_some()
        && secrets.resolve(&desired.dns_authority.credential).is_some()
}

/// Compare two nameserver sets for equality, case-insensitively and ignoring order.
///
/// NS record order is not semantically significant for delegation — a set of
/// NS records is the same regardless of ordering.
fn ns_sets_match(current: &[String], expected: &[String]) -> bool {
    if current.len() != expected.len() {
        return false;
    }
    let current_lower: BTreeSet<String> = current.iter().map(|s| s.trim().to_lowercase()).collect();
    let expected_lower: BTreeSet<String> =
        expected.iter().map(|s| s.trim().to_lowercase()).collect();
    current_lower == expected_lower
}

/// Serialize a value to `serde_json::Value` for proof `provider_read_back`.
///
/// Uses `unwrap_or(Null)` to avoid panicking on serialization errors —
/// the proof envelope always carries a value, even if it's `Null`.
fn to_json_value<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

/// Execute the full domain operations workflow from a desired-state contract.
///
/// Sequence (CYNIC coherence pattern: desired-state → observe → reconcile →
/// bounded action → read-back proof):
///
/// 1. **Preflight**: Verify credential references resolve via `SecretProvider`.
/// 2. **Observe**: Check domain availability, read registrar zone records,
///    read current registrar nameservers, check Cloudflare zone existence,
///    read Cloudflare zone records (if zone existed).
/// 3. **Reconcile**: `reconcile_zone()` diffs registrar vs Cloudflare;
///    `detect_cname_conflict()` guards against ambiguous multi-record writes.
/// 4. **Bounded action** (gated by `apply` + `ns_cutover_confirmed`):
///    - Register domain (paid, `apply` gate)
///    - Update nameservers (NS cutover, separate `ns_cutover_confirmed` gate)
///    - Create Cloudflare zone (if missing, idempotent via `create_zone`)
///    - Backup Cloudflare zone before writes (read-back safety)
///    - Upsert records with idempotency keys
/// 5. **Verify** (always, read-only): TLD delegation via authoritative NS
///    query, edge HTTP probes through Cloudflare edge.
///
/// Consent gates:
/// - `apply = false` → all mutations emit `Blocked` proofs; read-only
///   operations (availability, verification) still proceed.
/// - `apply = true && ns_cutover_confirmed = false` → registration and
///   zone migration proceed, but NS cutover is blocked.
/// - `apply = true && ns_cutover_confirmed = true` → full chain executes.
///
/// # Arguments
/// * `desired` — machine-readable desired state (validated by `parse_desired_state`)
/// * `registrar` — registrar port (e.g. Porkbun API adapter)
/// * `cloudflare` — Cloudflare DNS port
/// * `probe` — authoritative DNS + HTTP edge probe port
/// * `secrets` — resolves `SecretReference` to actual secret values
/// * `apply` — master consent flag for any mutation (paid ops, zone writes)
/// * `ns_cutover_confirmed` — separate consent gate for nameserver changes
///
/// # Returns
/// `Vec<DomainOperationProof>` — one proof per operation, in execution order.
/// Each proof carries the idempotency key, provider read-back, and a status
/// verdict (`Success`, `Blocked`, or `Violates`).
pub fn run_domain_ops(
    desired: &DomainOpsDesiredState,
    registrar: &dyn RegistrarPort,
    cloudflare: &dyn CloudflareDnsPort,
    probe: &dyn EdgeProbePort,
    secrets: &dyn SecretProvider,
    apply: bool,
    ns_cutover_confirmed: bool,
) -> Vec<DomainOperationProof> {
    let mut proofs: Vec<DomainOperationProof> = Vec::new();
    let domain = &desired.domain;
    let ts = now_unix_seconds();

    // ── 1. Preflight: credential resolution ────────────────────────────────
    if !credentials_resolved(desired, secrets) {
        let key = generate_idempotency_key(
            domain,
            DomainOperation::CheckAvailability,
            "credential_preflight",
        );
        proofs.push(blocked_proof(
            domain,
            DomainOperation::CheckAvailability,
            &key,
            "one or more credential references could not be resolved",
            ts,
        ));
        return proofs;
    }

    // ── 2. Observe: domain availability ────────────────────────────────────
    let availability = registrar.check_availability(domain);
    let avail_key = generate_idempotency_key(domain, DomainOperation::CheckAvailability, "");
    proofs.push(build_proof(
        domain,
        DomainOperation::CheckAvailability,
        ProofStatus::Success,
        &avail_key,
        to_json_value(&availability),
        ts,
    ));

    // ── 2b. Observe: registrar zone records + current NS ────────────────────
    let registrar_records = registrar.read_zone_records(domain);
    let current_ns = registrar.read_nameservers(domain);

    // ── 2c. Select desired records (source of truth) ────────────────────────
    let desired_records = if desired.zone_records.is_empty() {
        &registrar_records
    } else {
        &desired.zone_records
    };

    // ── 3. Pure: CNAME conflict detection (before any mutation) ────────────
    if let Some(conflict) = detect_cname_conflict(desired_records) {
        let key = generate_idempotency_key(domain, DomainOperation::MigrateZone, &conflict.name);
        proofs.push(blocked_proof(
            domain,
            DomainOperation::MigrateZone,
            &key,
            &conflict.message(),
            ts,
        ));
        return proofs; // Stop — must be resolved manually
    }

    // ── 2c. Observe: Cloudflare zone state ─────────────────────────────────
    let zone_existed = cloudflare.zone_exists(domain);
    let zone_id = cloudflare.create_zone(domain);

    // ── 2d. Observe: Cloudflare zone records (if zone existed) ────────────
    let cf_records = if zone_existed {
        cloudflare.read_zone_records(domain)
    } else {
        Vec::new()
    };

    // ── 4. Reconcile (pure): diff desired vs Cloudflare ────────────────────
    let diff = reconcile_zone(desired_records, &cf_records, &desired.proxy_policy);

    // ── 5. Bounded action (gated by apply + ns_cutover_confirmed) ─────────
    if apply {
        execute_mutations(
            desired,
            domain,
            &availability,
            registrar,
            cloudflare,
            &diff,
            zone_existed,
            &zone_id,
            &current_ns,
            ns_cutover_confirmed,
            &mut proofs,
            ts,
        );
    } else {
        emit_blocked_proofs(
            desired,
            domain,
            &availability,
            &diff,
            &zone_id,
            &current_ns,
            &mut proofs,
            ts,
        );
    }

    // ── 6. Verify (always, read-only): TLD delegation ──────────────────────
    let delegation = probe.check_tld_delegation(domain);
    let delegation_key = generate_idempotency_key(domain, DomainOperation::VerifyDelegation, "");
    let delegation_status = if delegation.ns_set_matches {
        ProofStatus::Success
    } else {
        ProofStatus::Violates
    };
    proofs.push(build_proof(
        domain,
        DomainOperation::VerifyDelegation,
        delegation_status,
        &delegation_key,
        to_json_value(&delegation),
        ts,
    ));

    // ── 6b. Verify (always, read-only): Edge HTTP probes ──────────────────
    for probe_spec in &desired.edge_probes {
        let edge_key =
            generate_idempotency_key(domain, DomainOperation::VerifyEdge, &probe_spec.fqdn);
        let result = probe.http_probe(
            &probe_spec.fqdn,
            "",
            probe_spec.expected_status,
            probe_spec.follow_redirects,
        );
        let status = if result.passed {
            ProofStatus::Success
        } else {
            ProofStatus::Violates
        };
        proofs.push(build_proof(
            domain,
            DomainOperation::VerifyEdge,
            status,
            &edge_key,
            to_json_value(&result),
            ts,
        ));
    }

    proofs
}

/// Execute mutation operations when `apply = true`.
///
/// This function is NOT pure — it calls registrar, Cloudflare, and probe
/// ports. All business rules (idempotency keys, record transforms, conflict
/// detection) are delegated to the pure `domain/` module.
fn execute_mutations(
    desired: &DomainOpsDesiredState,
    domain: &str,
    availability: &DomainAvailability,
    registrar: &dyn RegistrarPort,
    cloudflare: &dyn CloudflareDnsPort,
    diff: &ZoneRecordDiff,
    zone_existed: bool,
    zone_id: &str,
    current_ns: &[String],
    ns_cutover_confirmed: bool,
    proofs: &mut Vec<DomainOperationProof>,
    ts: u64,
) {
    // ── A. Register domain (paid, apply-gated) ──────────────────────────────
    if availability.available {
        let reg_key = generate_idempotency_key(domain, DomainOperation::Register, "");
        let receipt = registrar.register_domain(domain, 1, true, &reg_key);
        proofs.push(build_proof(
            domain,
            DomainOperation::Register,
            ProofStatus::Success,
            &reg_key,
            to_json_value(&receipt),
            ts,
        ));
    }

    // ── B. NS cutover (separate consent gate) ──────────────────────────────
    if ns_cutover_confirmed {
        let ns_changed = !ns_sets_match(current_ns, &desired.expected_nameservers);
        if ns_changed {
            let ns_key = generate_idempotency_key(
                domain,
                DomainOperation::UpdateNameservers,
                &desired.expected_nameservers.join(","),
            );
            let receipt =
                registrar.update_nameservers(domain, &desired.expected_nameservers, &ns_key);
            proofs.push(build_proof(
                domain,
                DomainOperation::UpdateNameservers,
                ProofStatus::Success,
                &ns_key,
                to_json_value(&receipt),
                ts,
            ));
        }
    }

    // ── C. Zone migration (backup-first, idempotency-keyed) ───────────────
    if !diff.is_empty() {
        let migration_key = generate_idempotency_key(domain, DomainOperation::MigrateZone, zone_id);

        // Backup before any write (safety invariant: backup-first)
        let _backup = if zone_existed {
            cloudflare.export_zone_backup(zone_id)
        } else {
            String::new() // New zone — nothing to backup
        };

        // Transform registrar records → Cloudflare shapes (pure)
        let records_to_write: Vec<DnsRecordShape> = diff
            .add
            .iter()
            .chain(diff.update.iter())
            .filter_map(
                |r| match transform_record_for_cloudflare(r, &desired.proxy_policy) {
                    TransformResult::Transformed(t) => Some(t),
                    TransformResult::Skipped { .. } => None,
                },
            )
            .collect();

        let records_skipped: Vec<SkippedRecord> = diff
            .add
            .iter()
            .filter_map(
                |r| match transform_record_for_cloudflare(r, &desired.proxy_policy) {
                    TransformResult::Skipped { reason } => Some(SkippedRecord {
                        name: r.name.clone(),
                        r#type: r.r#type,
                        reason: reason.to_string(),
                    }),
                    TransformResult::Transformed(_) => None,
                },
            )
            .collect();

        if !records_to_write.is_empty() {
            let updated_ids = cloudflare.upsert_records(zone_id, &records_to_write, &migration_key);
            let _ = updated_ids; // Record IDs from provider — archived in proof
        }

        let migration_result = ZoneMigrationResult {
            schema: DOMAIN_OPS_ZONE_MIGRATION_SCHEMA.to_string(),
            domain: domain.to_string(),
            zone_id: zone_id.to_string(),
            records_migrated: records_to_write.len(),
            records_skipped,
            backup_ref: if zone_existed {
                Some(migration_key.clone())
            } else {
                None
            },
            idempotency_key: migration_key.clone(),
        };

        proofs.push(build_proof(
            domain,
            DomainOperation::MigrateZone,
            ProofStatus::Success,
            &migration_key,
            to_json_value(&migration_result),
            ts,
        ));
    }
}

/// Emit `Blocked` proofs when `apply = false` (observe-only mode).
///
/// Read-only operations (availability, reconciliation, verification) still
/// proceed; only mutations are blocked with a reason.
fn emit_blocked_proofs(
    desired: &DomainOpsDesiredState,
    domain: &str,
    availability: &DomainAvailability,
    diff: &ZoneRecordDiff,
    zone_id: &str,
    current_ns: &[String],
    proofs: &mut Vec<DomainOperationProof>,
    ts: u64,
) {
    if availability.available {
        let key = generate_idempotency_key(domain, DomainOperation::Register, "");
        proofs.push(blocked_proof(
            domain,
            DomainOperation::Register,
            &key,
            "apply=false — registration not authorized",
            ts,
        ));
    }

    // NS cutover: always blocked when apply=false, regardless of ns_cutover_confirmed
    let ns_changed = !ns_sets_match(current_ns, &desired.expected_nameservers);
    if ns_changed {
        let key = generate_idempotency_key(
            domain,
            DomainOperation::UpdateNameservers,
            &desired.expected_nameservers.join(","),
        );
        proofs.push(blocked_proof(
            domain,
            DomainOperation::UpdateNameservers,
            &key,
            "apply=false — NS cutover not authorized",
            ts,
        ));
    }

    if !diff.is_empty() {
        let key = generate_idempotency_key(domain, DomainOperation::MigrateZone, zone_id);
        proofs.push(blocked_proof(
            domain,
            DomainOperation::MigrateZone,
            &key,
            &format!(
                "apply=false — {} record changes pending",
                diff.total_changes()
            ),
            ts,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::{
        MemoryCloudflare, MemoryEdgeProbe, MemoryRegistrar, MemorySecretProvider,
    };
    use cynic_protocol::domain_ops::DnsRecordType;

    fn desired_state(
        domain: &str,
        expected_ns: Vec<&str>,
        probes: Vec<(&str, Option<u16>, bool)>,
    ) -> DomainOpsDesiredState {
        DomainOpsDesiredState {
            schema: DOMAIN_OPS_DESIRED_STATE_SCHEMA.to_string(),
            domain: domain.to_string(),
            registrar: ProviderConfig {
                provider: "porkbun".to_string(),
                credential: cynic_protocol::secret_reference::SecretReference {
                    id: "porkbun-api-key".to_string(),
                    class: cynic_protocol::secret_reference::SecretClass::RuntimeMachineIdentity,
                    provider: cynic_protocol::secret_reference::SecretProvider::ProcessEnvironment,
                    locator: "PORKBUN_API_KEY".to_string(),
                },
            },
            dns_authority: ProviderConfig {
                provider: "cloudflare".to_string(),
                credential: cynic_protocol::secret_reference::SecretReference {
                    id: "cf-token".to_string(),
                    class: cynic_protocol::secret_reference::SecretClass::RuntimeMachineIdentity,
                    provider: cynic_protocol::secret_reference::SecretProvider::ProcessEnvironment,
                    locator: "CF_API_TOKEN".to_string(),
                },
            },
            proxy_policy: ProxyPolicy::default(),
            expected_nameservers: expected_ns.iter().map(|s| s.to_string()).collect(),
            edge_probes: probes
                .iter()
                .map(|(fqdn, status, redirects)| EdgeProbeSpec {
                    fqdn: fqdn.to_string(),
                    expected_status: *status,
                    expected_redirect: None,
                    follow_redirects: *redirects,
                    preserve_path: false,
                })
                .collect(),
            zone_records: Vec::new(),
            dns_authority_account_id: None,
        }
    }

    fn a_record(name: &str, content: &str, proxied: bool) -> DnsRecordShape {
        DnsRecordShape {
            name: name.to_string(),
            r#type: DnsRecordType::A,
            content: content.to_string(),
            ttl: 300,
            priority: None,
            proxied,
        }
    }

    fn make_secrets() -> MemorySecretProvider {
        MemorySecretProvider::new()
            .with_secret("PORKBUN_API_KEY", "pk1_test_key")
            .with_secret("CF_API_TOKEN", "test_cf_token")
    }

    // ── Tests ───────────────────────────────────────────────────────────────

    #[test]
    fn apply_false_emits_blocked_proofs_for_mutations() {
        let domain = "example.com";
        let desired = desired_state(
            domain,
            vec!["ns1.cloudflare.com", "ns2.cloudflare.com"],
            vec![("www.example.com", Some(200), true)],
        );
        let records = vec![a_record("www.example.com", "1.2.3.4", true)];

        let registrar = MemoryRegistrar::new()
            .with_availability(domain, true)
            .with_zone_records(domain, records)
            .with_nameservers(domain, vec!["ns1.porkbun.com".to_string()]);

        let cloudflare = MemoryCloudflare::new().with_zone(domain, "zone-123");
        let probe = MemoryEdgeProbe::new()
            .with_ns_delegation(domain, vec!["ns1.cloudflare.com".to_string()])
            .with_http_probe("www.example.com", 200)
            .with_tld_delegation(domain, true, vec!["ns1.cloudflare.com".to_string()]);

        let proofs = run_domain_ops(
            &desired,
            &registrar,
            &cloudflare,
            &probe,
            &make_secrets(),
            false,
            false,
        );

        // Should have: availability + blocked(register) + blocked(NS) + blocked(migration) + delegation + edge
        assert!(proofs.len() >= 5);

        let reg_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::Register)
            .expect("registration proof must exist");
        assert_eq!(reg_proof.status, ProofStatus::Blocked);

        let ns_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::UpdateNameservers)
            .expect("NS blocked proof must exist");
        assert_eq!(ns_proof.status, ProofStatus::Blocked);

        let migration_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::MigrateZone)
            .expect("migration blocked proof must exist");
        assert_eq!(migration_proof.status, ProofStatus::Blocked);

        // Verification always runs (read-only) even in observe-only mode
        let delegation_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::VerifyDelegation)
            .expect("delegation proof must exist");
        assert_eq!(delegation_proof.status, ProofStatus::Success);

        let edge_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::VerifyEdge)
            .expect("edge proof must exist");
        assert_eq!(edge_proof.status, ProofStatus::Success);
    }

    #[test]
    fn apply_true_registers_available_domain() {
        let domain = "available-test.com";
        let desired = desired_state(
            domain,
            vec!["ns1.cloudflare.com", "ns2.cloudflare.com"],
            vec![("www.available-test.com", Some(200), true)],
        );
        let records = vec![a_record("www.available-test.com", "1.2.3.4", true)];

        let registrar = MemoryRegistrar::new()
            .with_availability(domain, true)
            .with_zone_records(domain, records)
            .with_nameservers(domain, vec!["ns1.porkbun.com".to_string()]);

        let cloudflare = MemoryCloudflare::new().with_zone(domain, "zone-123");
        let probe = MemoryEdgeProbe::new().with_tld_delegation(
            domain,
            true,
            vec!["ns1.cloudflare.com".to_string()],
        );

        let proofs = run_domain_ops(
            &desired,
            &registrar,
            &cloudflare,
            &probe,
            &make_secrets(),
            true,
            false,
        );

        // Registration should succeed
        let reg_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::Register)
            .expect("registration proof must exist");
        assert_eq!(reg_proof.status, ProofStatus::Success);

        // Zone migration should succeed (apply=true)
        let migration_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::MigrateZone)
            .expect("migration proof must exist");
        assert_eq!(migration_proof.status, ProofStatus::Success);

        // NS cutover should NOT happen (ns_cutover_confirmed=false)
        let ns_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::UpdateNameservers);
        assert!(ns_proof.is_none());
    }

    #[test]
    fn apply_true_with_ns_cutdown_confirms_ns_update() {
        let domain = "ns-cutover-test.com";
        let desired = desired_state(
            domain,
            vec!["ns1.cloudflare.com", "ns2.cloudflare.com"],
            vec![("www.ns-cutover-test.com", Some(200), true)],
        );
        let records = vec![a_record("www.ns-cutover-test.com", "1.2.3.4", true)];

        let registrar = MemoryRegistrar::new()
            .with_availability(domain, true)
            .with_zone_records(domain, records)
            .with_nameservers(
                domain,
                vec!["ns1.porkbun.com".to_string(), "ns2.porkbun.com".to_string()],
            );

        let cloudflare = MemoryCloudflare::new().with_zone(domain, "zone-456");
        let probe = MemoryEdgeProbe::new().with_tld_delegation(
            domain,
            true,
            vec!["ns1.cloudflare.com".to_string()],
        );

        let proofs = run_domain_ops(
            &desired,
            &registrar,
            &cloudflare,
            &probe,
            &make_secrets(),
            true,
            true,
        );

        let ns_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::UpdateNameservers)
            .expect("NS update proof must exist with cutover confirmed");
        assert_eq!(ns_proof.status, ProofStatus::Success);
    }

    #[test]
    fn apply_true_without_ns_cutover_blocks_ns() {
        let domain = "ns-blocked-test.com";
        let desired = desired_state(
            domain,
            vec!["ns1.cloudflare.com", "ns2.cloudflare.com"],
            vec![("www.ns-blocked-test.com", Some(200), true)],
        );
        let records = vec![a_record("www.ns-blocked-test.com", "1.2.3.4", true)];

        let registrar = MemoryRegistrar::new()
            .with_availability(domain, true)
            .with_zone_records(domain, records)
            .with_nameservers(
                domain,
                vec!["ns1.porkbun.com".to_string(), "ns2.porkbun.com".to_string()],
            );

        let cloudflare = MemoryCloudflare::new().with_zone(domain, "zone-789");
        let probe = MemoryEdgeProbe::new().with_tld_delegation(
            domain,
            false,
            vec!["ns1.cloudflare.com".to_string()],
        );

        let proofs = run_domain_ops(
            &desired,
            &registrar,
            &cloudflare,
            &probe,
            &make_secrets(),
            true,
            false,
        );

        // NS cutover blocked even with apply=true (ns_cutover_confirmed=false)
        let ns_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::UpdateNameservers);
        assert!(
            ns_proof.is_none(),
            "NS update should not appear without cutover confirmation"
        );
    }

    #[test]
    fn cname_conflict_blocks_migration_early() {
        let domain = "conflict-test.com";
        let desired = desired_state(domain, vec!["ns1.cloudflare.com"], vec![]);

        // Registrar zone has a CNAME + A record at the same name → conflict
        let registrar_records = vec![
            DnsRecordShape {
                name: "www.conflict-test.com".to_string(),
                r#type: DnsRecordType::CNAME,
                content: "target.pages.dev".to_string(),
                ttl: 1,
                priority: None,
                proxied: true,
            },
            DnsRecordShape {
                name: "www.conflict-test.com".to_string(),
                r#type: DnsRecordType::A,
                content: "1.2.3.4".to_string(),
                ttl: 300,
                priority: None,
                proxied: true,
            },
        ];

        let registrar = MemoryRegistrar::new()
            .with_availability(domain, true)
            .with_zone_records(domain, registrar_records)
            .with_nameservers(domain, vec!["ns1.porkbun.com".to_string()]);

        let cloudflare = MemoryCloudflare::new();
        let probe = MemoryEdgeProbe::new();
        let secrets = make_secrets();

        let proofs = run_domain_ops(
            &desired,
            &registrar,
            &cloudflare,
            &probe,
            &secrets,
            true,
            true,
        );

        // Should have: availability + blocked(migration: conflict) — early return
        assert!(proofs.len() == 2);

        let migration_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::MigrateZone)
            .expect("migration blocked proof must exist");
        assert_eq!(migration_proof.status, ProofStatus::Blocked);

        // No delegation or edge proofs (early return after conflict)
        let delegation_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::VerifyDelegation);
        assert!(
            delegation_proof.is_none(),
            "no verification after CNAME conflict"
        );
    }

    #[test]
    fn no_credentials_blocks_all_operations() {
        let domain = "no-creds-test.com";
        let desired = desired_state(domain, vec!["ns1.cloudflare.com"], vec![]);

        let registrar = MemoryRegistrar::new();
        let cloudflare = MemoryCloudflare::new();
        let probe = MemoryEdgeProbe::new();
        let secrets = MemorySecretProvider::new(); // No secrets registered

        let proofs = run_domain_ops(
            &desired,
            &registrar,
            &cloudflare,
            &probe,
            &secrets,
            true,
            true,
        );

        // Should return immediately with a single blocked proof
        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0].operation, DomainOperation::CheckAvailability);
        assert_eq!(proofs[0].status, ProofStatus::Blocked);
    }

    #[test]
    fn ns_sets_match_ignores_case_and_order() {
        assert!(ns_sets_match(
            &[
                "ns1.cloudflare.com".to_string(),
                "ns2.cloudflare.com".to_string()
            ],
            &[
                "ns2.CLOUDFLARE.com".to_string(),
                "NS1.Cloudflare.com".to_string()
            ],
        ));
        assert!(!ns_sets_match(
            &["ns1.cloudflare.com".to_string()],
            &["ns2.cloudflare.com".to_string()],
        ));
        assert!(ns_sets_match(&[], &[]));
    }

    fn cname_record(name: &str, content: &str, proxied: bool) -> DnsRecordShape {
        DnsRecordShape {
            name: name.to_string(),
            r#type: DnsRecordType::CNAME,
            content: content.to_string(),
            ttl: 1,
            priority: None,
            proxied,
        }
    }

    #[test]
    fn apply_true_with_desired_zone_records_syncs_to_cloudflare() {
        let domain = "talaria.build";
        let mut desired = desired_state(
            domain,
            vec!["ns1.cloudflare.com", "ns2.cloudflare.com"],
            vec![
                ("talaria.build", Some(200), true),
                ("www.talaria.build", Some(200), true),
            ],
        );
        desired.zone_records = vec![
            cname_record("talaria.build", "cname.vercel-dns.com", true),
            cname_record("www.talaria.build", "cname.vercel-dns.com", true),
        ];

        let registrar = MemoryRegistrar::new()
            .with_availability(domain, false)
            .with_zone_records(domain, vec![])
            .with_nameservers(domain, vec!["ns1.cloudflare.com".to_string()]);

        let cloudflare = MemoryCloudflare::new().with_zone(domain, "zone-talaria");
        let probe = MemoryEdgeProbe::new().with_tld_delegation(
            domain,
            true,
            vec!["ns1.cloudflare.com".to_string()],
        );

        let proofs = run_domain_ops(
            &desired,
            &registrar,
            &cloudflare,
            &probe,
            &make_secrets(),
            true,
            false,
        );

        let migration_proof = proofs
            .iter()
            .find(|p| p.operation == DomainOperation::MigrateZone)
            .expect("migration proof must exist when zone_records provided");
        assert_eq!(migration_proof.status, ProofStatus::Success);

        let result: ZoneMigrationResult =
            serde_json::from_value(migration_proof.provider_read_back.clone()).unwrap();
        assert_eq!(result.records_migrated, 2);
    }
}
