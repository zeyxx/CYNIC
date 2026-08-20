//! Extract auditable claims from the registry document.
//!
//! Read-only. This layer decides nothing: it turns registry JSON into
//! `ClaimToProbe` values and leaves every verdict to
//! `cynic_protocol::registry_claim_audit`.

use std::fs;
use std::path::Path;

use cynic_host::registry_divergence_probe::ClaimToProbe;
use cynic_protocol::registry_claim_audit::ClaimKind;
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
        let declared_as_candidate =
            node.get("status").and_then(Value::as_str) == Some("candidate");

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

/// Names the registry publishes as DNS records, for consistency auditing.
pub(crate) fn extract_published_names(registry_path: &Path) -> Result<Vec<String>, String> {
    let raw = fs::read_to_string(registry_path)
        .map_err(|error| format!("cannot read registry: {error}"))?;
    let document: Value =
        serde_json::from_str(&raw).map_err(|error| format!("registry is not valid JSON: {error}"))?;

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

    fn write_registry(body: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("cynic-fleet-div-{}.json", std::process::id()));
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
        let error = extract_claims(Path::new("/nonexistent/registry.json"))
            .expect_err("must not succeed");
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
}
