//! Extract auditable claims from the registry document.
//!
//! Read-only. This layer decides nothing: it turns registry JSON into
//! `ClaimToProbe` values and leaves every verdict to
//! `cynic_protocol::registry_claim_audit`.

use std::fs;
use std::path::Path;

use cynic_host::registry_divergence_probe::ClaimToProbe;
use cynic_protocol::registry_claim_audit::{ClaimKind, DeclaredNodeView};
use serde_json::Value;

/// Read every synapse-address claim the registry declares.
///
/// `status: candidate` is carried through rather than filtered: an
/// unmaterialized entry is still worth auditing, it just grades as stale intent
/// instead of an asserted falsehood.
pub(crate) fn extract_claims(registry_path: &Path) -> Result<Vec<ClaimToProbe>, String> {
    let raw = fs::read_to_string(registry_path).map_err(|error| {
        format!(
            "cannot read registry at {}: {error}",
            registry_path.display()
        )
    })?;
    let document: Value = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "registry at {} is not valid JSON: {error}",
            registry_path.display()
        )
    })?;

    let nodes = document
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry has no `nodes` array".to_string())?;

    let mut claims = Vec::new();
    for node in nodes {
        let Some(node_id) = node.get("id").and_then(Value::as_str) else {
            continue;
        };
        let declared_as_candidate = node.get("status").and_then(Value::as_str) == Some("candidate");

        // Only the synapse plane is probed: it is the plane peers use to reach
        // each other, so a wrong address there is what breaks discovery.
        if let Some(address) = node
            .get("network")
            .and_then(|network| network.get("synapse"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            claims.push(ClaimToProbe {
                node_id: node_id.to_string(),
                kind: ClaimKind::SynapseAddress,
                declared: address.to_string(),
                declared_as_candidate,
            });
        }
    }

    Ok(claims)
}

/// Read the registry's own view of each node: role and canonical-plane address.
///
/// This is the authoritative side of a declarative comparison. Nodes with no
/// synapse address are skipped: they cannot be compared against a seed table
/// that is addressed on the synapse plane.
pub(crate) fn extract_registry_views(
    registry_path: &Path,
) -> Result<Vec<DeclaredNodeView>, String> {
    let raw = fs::read_to_string(registry_path).map_err(|error| {
        format!(
            "cannot read registry at {}: {error}",
            registry_path.display()
        )
    })?;
    let document: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("registry is not valid JSON: {error}"))?;

    let nodes = document
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry has no `nodes` array".to_string())?;

    let mut views = Vec::new();
    for node in nodes {
        let Some(node_id) = node.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(role) = node.get("role").and_then(Value::as_str) else {
            continue;
        };
        let Some(address) = node
            .get("network")
            .and_then(|network| network.get("synapse"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        else {
            continue;
        };
        views.push(DeclaredNodeView {
            node_id: node_id.to_string(),
            role: role.to_string(),
            address: address.to_string(),
        });
    }

    Ok(views)
}

/// Read the seed table the mesh resolver actually consumes.
///
/// A missing seed file is `Ok(None)`, not an error: the resolver degrades to its
/// compiled-in const, and the absence of a second source is not a divergence.
/// An unreadable-but-present file *is* an error — silently treating it as absent
/// would hide the very disagreement this compares for.
pub(crate) fn extract_seed_views(
    seed_path: &Path,
) -> Result<Option<Vec<DeclaredNodeView>>, String> {
    if !seed_path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(seed_path)
        .map_err(|error| format!("cannot read seed at {}: {error}", seed_path.display()))?;
    let document: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("seed at {} is not valid JSON: {error}", seed_path.display()))?;

    let nodes = document
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "seed has no `nodes` array".to_string())?;

    let mut views = Vec::new();
    for node in nodes {
        let (Some(id), Some(role), Some(ip)) = (
            node.get("id").and_then(Value::as_str),
            node.get("role").and_then(Value::as_str),
            node.get("ip").and_then(Value::as_str),
        ) else {
            continue;
        };
        views.push(DeclaredNodeView {
            node_id: id.to_string(),
            role: role.to_string(),
            address: ip.to_string(),
        });
    }

    Ok(Some(views))
}

#[allow(dead_code)]
pub(crate) fn extract_published_names(registry_path: &Path) -> Result<Vec<String>, String> {
    let raw = fs::read_to_string(registry_path)
        .map_err(|error| format!("cannot read registry: {error}"))?;
    let document: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("registry is not valid JSON: {error}"))?;

    let mut names = Vec::new();
    if let Some(reservations) = document
        .get("network_policy")
        .and_then(|policy| policy.get("reservations"))
        .and_then(Value::as_array)
    {
        for reservation in reservations {
            if let Some(dns) = reservation.get("dns").and_then(Value::as_array) {
                for entry in dns.iter().filter_map(Value::as_str) {
                    names.push(entry.to_string());
                }
            }
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Unique per call, not per process. Rust runs tests in parallel threads of
    /// one process, so `process::id()` alone gave every test the same path and
    /// they overwrote each other's fixtures — a real race, not a flake.
    fn unique_fixture(prefix: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{prefix}-{}-{}.json",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        path
    }

    fn write_registry(body: &str) -> std::path::PathBuf {
        let path = unique_fixture("cynic-fleet-div");
        let mut file = fs::File::create(&path).expect("create fixture");
        file.write_all(body.as_bytes()).expect("write fixture");
        path
    }

    /// The live registry shape: local-kernel declares a synapse address and is
    /// marked candidate; cortex declares only a management address.
    #[test]
    fn extracts_synapse_claims_and_preserves_candidate_status() {
        let path = write_registry(
            r#"{"nodes":[
                {"id":"local-kernel","status":"candidate","network":{"synapse":"10.10.10.100"}},
                {"id":"cynic-builder","status":"active","network":{"synapse":"10.10.10.119"}},
                {"id":"cynic-host-cortex","status":"candidate","network":{"management":"192.168.100.2"}}
            ]}"#,
        );
        let claims = extract_claims(&path).expect("claims");

        assert_eq!(claims.len(), 2, "cortex has no synapse address to probe");
        let kernel = claims
            .iter()
            .find(|c| c.node_id == "local-kernel")
            .expect("kernel claim");
        assert_eq!(kernel.declared, "10.10.10.100");
        assert!(kernel.declared_as_candidate);

        let builder = claims
            .iter()
            .find(|c| c.node_id == "cynic-builder")
            .expect("builder claim");
        assert!(!builder.declared_as_candidate);
    }

    /// A missing registry is an error naming the path, never an empty claim set:
    /// zero claims audited must not read as zero problems.
    #[test]
    fn a_missing_registry_is_an_error_not_an_empty_audit() {
        let error =
            extract_claims(Path::new("/nonexistent/registry.json")).expect_err("must not succeed");
        assert!(error.contains("cannot read registry"));
    }

    #[test]
    fn malformed_json_is_reported_as_such() {
        let path = write_registry("{not json");
        let error = extract_claims(&path).expect_err("must not succeed");
        assert!(error.contains("not valid JSON"));
    }

    #[test]
    fn a_registry_without_nodes_is_rejected() {
        let path = write_registry(r#"{"network_policy":{}}"#);
        let error = extract_claims(&path).expect_err("must not succeed");
        assert!(error.contains("no `nodes` array"));
    }

    #[test]
    fn published_names_are_collected_and_deduplicated() {
        let path = write_registry(
            r#"{"nodes":[],"network_policy":{"reservations":[
                {"name":"a","dns":["kernel.synapse.cynic","local-kernel.synapse.cynic"]},
                {"name":"b","dns":["kernel.synapse.cynic"]},
                {"name":"c","dns":[]}
            ]}}"#,
        );
        let names = extract_published_names(&path).expect("names");
        assert_eq!(
            names,
            vec![
                "kernel.synapse.cynic".to_string(),
                "local-kernel.synapse.cynic".to_string()
            ]
        );
    }

    fn write_seed(body: &str) -> std::path::PathBuf {
        let path = unique_fixture("cynic-seed");
        let mut file = fs::File::create(&path).expect("create seed");
        file.write_all(body.as_bytes()).expect("write seed");
        path
    }

    /// The divergence observed on 2026-08-20: the registry and the seed table
    /// the resolver consumes disagree about what local-kernel is.
    #[test]
    fn registry_and_seed_disagreement_is_detected() {
        use cynic_protocol::registry_claim_audit::{compare_declarative_sources, judge_claim};

        let registry = write_registry(
            r#"{"nodes":[
                {"id":"local-kernel","role":"kernel_primary","network":{"synapse":"10.10.10.100"}},
                {"id":"cynic-builder","role":"build_runner","network":{"synapse":"10.10.10.119"}}
            ]}"#,
        );
        let seed = write_seed(
            r#"{"nodes":[
                {"id":"local-kernel","role":"cortex_primary","ip":"127.0.0.1","probe_ports":[8000]},
                {"id":"cynic-builder","role":"build_runner","ip":"10.10.10.119","probe_ports":[22]}
            ]}"#,
        );

        let reg_views = extract_registry_views(&registry).expect("registry views");
        let seed_views = extract_seed_views(&seed)
            .expect("seed read")
            .expect("seed present");

        let claims = compare_declarative_sources(&reg_views, &seed_views);
        assert_eq!(claims.len(), 2, "local-kernel diverges on role and address");
        assert!(claims.iter().all(|c| judge_claim(c).is_refuted()));
    }

    /// No seed file means the resolver falls back to its compiled const. That is
    /// not a divergence, and must not be reported as one.
    #[test]
    fn a_missing_seed_is_absence_not_divergence() {
        let views = extract_seed_views(Path::new("/nonexistent/seed.json")).expect("must succeed");
        assert!(views.is_none());
    }

    /// A present-but-corrupt seed is an error: silently treating it as absent
    /// would hide the disagreement this surface exists to find.
    #[test]
    fn a_corrupt_seed_is_an_error_not_silent_absence() {
        let seed = write_seed("{not json");
        let error = extract_seed_views(&seed).expect_err("must not succeed");
        assert!(error.contains("not valid JSON"));
    }

    /// A node with no synapse address cannot be compared on that plane.
    #[test]
    fn registry_views_skip_nodes_without_a_synapse_address() {
        let path = write_registry(
            r#"{"nodes":[
                {"id":"cynic-host-cortex","role":"cortex_host","network":{"management":"192.168.100.2"}},
                {"id":"forgejo","role":"forge_primary","network":{"synapse":"10.10.10.101"}}
            ]}"#,
        );
        let views = extract_registry_views(&path).expect("views");
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].node_id, "forgejo");
    }
}
