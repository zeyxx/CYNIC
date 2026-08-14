use super::types::{
    CapabilityEvidence, ProbeObservation, StorageCapabilitiesReport, StorageCapability,
    StorageCapabilityObservations, StorageRisk,
};
use crate::domain::shared::{BoundaryKind, LifecycleDisposition, LifecycleStatus};

pub fn build_report(
    endpoint: &str,
    cli_version: ProbeObservation,
    server_version: ProbeObservation,
    server_health: ProbeObservation,
    engine: ProbeObservation,
) -> StorageCapabilitiesReport {
    let server_reachable = server_version.is_available() || server_health.is_available();
    let version = server_version.value.as_deref().and_then(parse_version);
    let engine_id = engine.value.as_deref().unwrap_or("unknown");
    let status = overall_status(server_reachable, engine_id, version, &cli_version);
    let capabilities = capability_matrix(server_reachable, version, engine_id);
    let risks = risks(server_reachable, engine_id, version, &cli_version);
    let (
        lifecycle_status,
        lifecycle_disposition,
        next_boundary_kind,
        next_boundary_reason,
        next_command,
    ) = projection(status);

    StorageCapabilitiesReport {
        schema: "cynic.host.storage_capabilities.v1",
        status,
        mutates: false,
        lifecycle_status,
        lifecycle_disposition,
        next_boundary_kind,
        next_boundary_reason,
        next_command: next_command.clone(),
        endpoint: redact_endpoint(endpoint),
        observations: StorageCapabilityObservations {
            cli_version,
            server_version,
            server_health,
            engine,
            transport: "http_sql",
        },
        capabilities,
        risks,
        residual_unknowns: vec![
            "database runtime identity and effective permissions are not probed without credentials",
            "feature declarations by version are not runtime conformance proof",
            "backup restoration, temporal reads, and distributed consensus are not exercised by this read-only report",
            "schema deployment and index readiness require authenticated database evidence",
        ],
        next_actions: next_actions(status, next_command.as_deref()),
    }
}

fn capability_matrix(
    server_reachable: bool,
    version: Option<(u64, u64, u64)>,
    engine: &str,
) -> Vec<StorageCapability> {
    let declared = if version.is_some_and(|(major, _, _)| major >= 3) {
        CapabilityEvidence::DeclaredByVersion
    } else {
        CapabilityEvidence::Unknown
    };
    let version_proof = version
        .map(|(major, minor, patch)| format!("observed server version {major}.{minor}.{patch}"))
        .unwrap_or_else(|| "server version was not observed".into());
    let mut capabilities = vec![StorageCapability {
        id: "sql_http",
        evidence: if server_reachable {
            CapabilityEvidence::Proven
        } else {
            CapabilityEvidence::Unavailable
        },
        useful_for: "bounded query and runtime proof transport",
        proof: if server_reachable {
            "HTTP /version or /health responded successfully".into()
        } else {
            "neither HTTP /version nor /health was reachable".into()
        },
    }];

    for (id, useful_for) in [
        (
            "explicit_transactions",
            "atomic mutation, claim, and proof commits",
        ),
        ("changefeeds", "durable replay and projection recovery"),
        ("transactional_events", "atomic outbox and audit projection"),
        (
            "typed_graph_relations",
            "node, service, capability, artifact, and proof topology",
        ),
        (
            "field_constraints",
            "schemafull, assert, readonly, and computed invariants",
        ),
        (
            "explain_analyze",
            "query-plan conformance and scan detection",
        ),
        (
            "hybrid_search",
            "full-text and vector retrieval for sovereign context",
        ),
    ] {
        capabilities.push(StorageCapability {
            id,
            evidence: declared,
            useful_for,
            proof: version_proof.clone(),
        });
    }

    capabilities.push(StorageCapability {
        id: "live_queries",
        evidence: CapabilityEvidence::TransportBlocked,
        useful_for: "ephemeral local UI projections only",
        proof: "CYNIC storage adapter is HTTP SQL only; live queries require a persistent subscription transport".into(),
    });
    capabilities.push(StorageCapability {
        id: "temporal_versioning",
        evidence: CapabilityEvidence::Unknown,
        useful_for: "forensic point-in-time reconstruction",
        proof: "availability depends on storage-engine configuration and requires a runtime query proof".into(),
    });
    capabilities.push(StorageCapability {
        id: "distributed_storage",
        evidence: match engine {
            "tikv" | "surrealds" => CapabilityEvidence::DeclaredByVersion,
            "surrealkv" | "rocksdb" | "memory" => CapabilityEvidence::Unavailable,
            _ => CapabilityEvidence::Unknown,
        },
        useful_for: "multi-node durability, consensus, and horizontal scale",
        proof: format!("observed engine: {engine}"),
    });
    capabilities.push(StorageCapability {
        id: "least_privilege_runtime_identity",
        evidence: CapabilityEvidence::Unknown,
        useful_for: "separation of bootstrap authority from steady-state runtime",
        proof: "credentials are intentionally not consumed or exposed by this report".into(),
    });
    capabilities
}

fn risks(
    server_reachable: bool,
    engine: &str,
    server_version: Option<(u64, u64, u64)>,
    cli_version: &ProbeObservation,
) -> Vec<StorageRisk> {
    let mut risks = Vec::new();
    if !server_reachable {
        risks.push(StorageRisk {
            id: "storage_unreachable",
            severity: "critical",
            finding: "the configured storage endpoint did not answer read-only probes".into(),
            required_proof: "successful /health and /version observations",
        });
    }
    if engine == "surrealkv" {
        risks.push(StorageRisk {
            id: "beta_single_node_engine",
            severity: "high",
            finding: "SurrealKV is observed as the runtime storage engine; it must not be promoted without staging durability and restore proof".into(),
            required_proof: "engine benchmark, crash recovery, export/import checksum, and rollback evidence",
        });
    }
    if matches!(engine, "surrealkv" | "rocksdb" | "memory") {
        risks.push(StorageRisk {
            id: "single_node_storage",
            severity: "high",
            finding: format!("engine '{engine}' does not prove distributed storage or database high availability"),
            required_proof: "distributed storage topology, replication, consensus, and failover evidence",
        });
    }
    if engine == "unknown" {
        risks.push(StorageRisk {
            id: "engine_unknown",
            severity: "high",
            finding: "the active storage engine could not be discovered".into(),
            required_proof: "observed process argv, service ExecStart, or injected CYNIC_STORAGE_ENGINE declaration",
        });
    }
    if let (Some(server), Some(cli)) = (
        server_version,
        cli_version.value.as_deref().and_then(parse_version),
    ) && server != cli
    {
        risks.push(StorageRisk {
            id: "cli_server_version_drift",
            severity: "medium",
            finding: format!(
                "SurrealDB CLI {}.{}.{} differs from server {}.{}.{}",
                cli.0, cli.1, cli.2, server.0, server.1, server.2
            ),
            required_proof: "pinned artifact manifest and compatible upgrade proof",
        });
    }
    risks.push(StorageRisk {
        id: "http_only_transport",
        severity: "medium",
        finding: "the current adapter cannot provide persistent live-query subscriptions".into(),
        required_proof: "explicit subscription adapter or durable changefeed consumer",
    });
    risks
}

fn overall_status(
    server_reachable: bool,
    engine: &str,
    server_version: Option<(u64, u64, u64)>,
    cli_version: &ProbeObservation,
) -> &'static str {
    if !server_reachable {
        return "blocked";
    }
    let version_drift = match (
        server_version,
        cli_version.value.as_deref().and_then(parse_version),
    ) {
        (Some(server), Some(cli)) => server != cli,
        _ => false,
    };
    if engine == "unknown" || engine == "surrealkv" || version_drift {
        "degraded"
    } else {
        "ready"
    }
}

fn parse_version(raw: &str) -> Option<(u64, u64, u64)> {
    raw.split(|character: char| !(character.is_ascii_digit() || character == '.'))
        .find_map(|candidate| {
            let mut parts = candidate.split('.');
            let major = parts.next()?.parse().ok()?;
            let minor = parts.next()?.parse().ok()?;
            let patch = parts.next()?.parse().ok()?;
            Some((major, minor, patch))
        })
}

fn redact_endpoint(endpoint: &str) -> String {
    let without_query = endpoint
        .split(['?', '#'])
        .next()
        .unwrap_or(endpoint)
        .trim_end_matches('/');
    let Some((scheme, authority)) = without_query.split_once("://") else {
        return without_query.to_string();
    };
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, safe)| safe);
    format!("{scheme}://{authority}")
}

fn projection(
    status: &str,
) -> (
    LifecycleStatus,
    LifecycleDisposition,
    BoundaryKind,
    String,
    Option<String>,
) {
    match status {
        "blocked" => (
            LifecycleStatus::ConnectionRequired,
            LifecycleDisposition::RequiresConnection,
            BoundaryKind::Connect,
            "storage endpoint reachability is blocked, so runtime storage authority cannot be trusted yet"
                .to_string(),
            Some("cynic runtime storage-capabilities --json".to_string()),
        ),
        "degraded" => (
            LifecycleStatus::PrincipalMismatch,
            LifecycleDisposition::RequiresPrincipalAdoption,
            BoundaryKind::Adopt,
            "storage endpoint is reachable but engine, version drift, or capability proof remains incomplete"
                .to_string(),
            Some("cynic runtime storage-capabilities --json".to_string()),
        ),
        _ => (
            LifecycleStatus::Ready,
            LifecycleDisposition::ObservableCandidate,
            BoundaryKind::Observe,
            "storage capabilities are readable at the current endpoint and ready for deeper conformance proof"
                .to_string(),
            None,
        ),
    }
}

fn next_actions(status: &str, canonical_next_command: Option<&str>) -> Vec<String> {
    match status {
        "blocked" => {
            let mut actions = vec![
                "repair endpoint reachability before enabling storage-dependent runtime authority"
                    .to_string(),
                "do not infer database capabilities from the local CLI version".to_string(),
            ];
            if let Some(command) = canonical_next_command {
                actions.push(command.to_string());
            }
            actions.sort();
            actions.dedup();
            actions
        }
        "degraded" => {
            let mut actions = vec![
                "prove a RocksDB migration in staging before changing the runtime engine"
                    .to_string(),
                "complete fenced renew, release, and commit transactions".to_string(),
                "add authenticated schema and changefeed conformance probes without exposing credentials".to_string(),
            ];
            if let Some(command) = canonical_next_command {
                actions.push(command.to_string());
            }
            actions.sort();
            actions.dedup();
            actions
        }
        _ => vec![
            "complete fenced renew, release, and commit transactions".to_string(),
            "pilot durable changefeeds on proof_of_history and agent_tasks".to_string(),
            "capture EXPLAIN ANALYZE evidence for canonical storage queries".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed(value: &str) -> ProbeObservation {
        ProbeObservation::observed("test", value)
    }

    #[test]
    fn surrealkv_is_degraded_and_not_distributed() {
        let report = build_report(
            "http://localhost:8000",
            observed("3.0.3 for linux"),
            observed("surrealdb-3.0.3"),
            observed("healthy"),
            observed("surrealkv"),
        );
        assert_eq!(report.status, "degraded");
        assert!(report.capabilities.iter().any(|capability| {
            capability.id == "distributed_storage"
                && capability.evidence == CapabilityEvidence::Unavailable
        }));
    }

    #[test]
    fn unreachable_server_blocks_without_fabricating_features() {
        let report = build_report(
            "http://localhost:8000",
            observed("3.0.3 for linux"),
            ProbeObservation::unavailable("test", "offline"),
            ProbeObservation::unavailable("test", "offline"),
            ProbeObservation::unavailable("test", "unknown"),
        );
        assert_eq!(report.status, "blocked");
        assert!(report.capabilities.iter().any(|capability| {
            capability.id == "changefeeds" && capability.evidence == CapabilityEvidence::Unknown
        }));
    }

    #[test]
    fn endpoint_credentials_and_query_are_redacted() {
        assert_eq!(
            redact_endpoint("http://root:secret@db.internal:8000/?token=secret"),
            "http://db.internal:8000"
        );
    }
}
