//! Canonical Service and Path Resolver for CYNIC.
//!
//! Enforces zero-hardcoding: all service endpoints (embeddings, reranker, inference, etc.)
//! are resolved dynamically from the canonical registry SSOT (`infra/registry.json`),
//! with priority given to explicit environment variable overrides.

use std::path::PathBuf;

/// Locate the canonical registry path.
///
/// Delegates to `cynic_types::CynicPaths::find_canonical_registry_path`.
pub fn find_canonical_registry_path() -> Option<PathBuf> {
    cynic_types::CynicPaths::find_canonical_registry_path()
}

/// Dynamically resolve a service endpoint by service name from the canonical registry.
///
/// Returns `None` if the service is not declared or if it is marked as excluded.
pub fn resolve_service_endpoint_from_registry(service_name: &str) -> Option<String> {
    let registry_file = find_canonical_registry_path()?;
    let data = std::fs::read_to_string(registry_file).ok()?;
    let registry: serde_json::Value = serde_json::from_str(&data).ok()?;

    let nodes = registry.get("nodes")?.as_array()?;
    for node in nodes {
        if let Some(exclusion) = node.get("exclusion") {
            if exclusion.get("status").and_then(|s| s.as_str()) == Some("excluded") {
                continue;
            }
        }

        let Some(services) = node.get("services").and_then(|s| s.as_array()) else {
            continue;
        };

        for service in services {
            if service.get("name").and_then(|n| n.as_str()) != Some(service_name) {
                continue;
            }

            if let Some(exclusion) = service.get("exclusion") {
                if exclusion.get("status").and_then(|s| s.as_str()) == Some("excluded") {
                    continue;
                }
            }

            if let Some(ep) = service.get("endpoint").and_then(|e| e.as_str()) {
                if !ep.is_empty() {
                    return Some(ep.to_string());
                }
            }

            if let Some(bench_ep) = service.get("bench_endpoint").and_then(|e| e.as_str()) {
                if !bench_ep.is_empty() {
                    let clean = bench_ep.split('#').next().unwrap_or(bench_ep);
                    return Some(clean.to_string());
                }
            }

            let scheme = service.get("scheme").and_then(|s| s.as_str()).unwrap_or("http");
            let host = service.get("endpoint_host").and_then(|h| h.as_str())?;
            let port = service.get("port").and_then(|p| p.as_u64())?;

            return Some(format!("{scheme}://{host}:{port}"));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_existing_registry_services() {
        if let Some(reg) = find_canonical_registry_path() {
            assert!(reg.is_file());
            // embedding-server is declared on compute-01
            let embed = resolve_service_endpoint_from_registry("embedding-server");
            assert!(embed.is_some(), "embedding-server should resolve from registry");
        }
    }
}
