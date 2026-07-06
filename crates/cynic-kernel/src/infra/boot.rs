
use std::collections::HashMap;
use std::sync::Arc;
use crate::infra::probes::FleetTarget;

pub fn is_sovereign_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    url_lower.starts_with("http://127.") || url_lower.starts_with("http://localhost") || url_lower.starts_with("http://10.") || url_lower.starts_with("http://100.") || url_lower.starts_with("http://192.168.") || url_lower.starts_with("http://172.")
}

pub async fn seed_integrity_chain(storage_port: &Arc<dyn crate::domain::storage::StoragePort>) -> bool {
    // Quantum Integrity verification moved to core crypto. 
    // This is a stub while Inference is extracted.
    true
}

pub fn build_fleet_targets(health_urls: &HashMap<String, Option<String>>, fleet_meta: &HashMap<String, (String, u32, String, Option<String>, bool)>) -> Vec<FleetTarget> {
    vec![]
}

pub fn load_rest_api_key() -> Result<Option<String>, String> {
    Ok(None)
}
