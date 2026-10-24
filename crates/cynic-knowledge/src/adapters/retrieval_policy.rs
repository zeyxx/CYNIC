//! Retrieval Policy File Adapter (I/O Boundary)
//!
//! Loads `contracts/retrieval-policy.json` from the repository root or system locations.

use std::path::Path;
use crate::domain::retrieval_policy::{RetrievalPolicy, RETRIEVAL_POLICY_CONTRACT_PATH};

/// Loads the canonical retrieval policy from the given workspace root.
/// Returns the parsed policy on success, or falls back to `RetrievalPolicy::default_policy()`
/// with an advisory warning on parse failure.
pub fn load_canonical_retrieval_policy<P: AsRef<Path>>(root: P) -> RetrievalPolicy {
    let contract_path = if let Ok(custom) = std::env::var("CYNIC_RETRIEVAL_POLICY") {
        std::path::PathBuf::from(custom)
    } else {
        root.as_ref().join(RETRIEVAL_POLICY_CONTRACT_PATH)
    };

    if let Ok(content) = std::fs::read_to_string(&contract_path) {
        match RetrievalPolicy::from_json(&content) {
            Ok(policy) => return policy,
            Err(e) => {
                eprintln!(
                    "warning: failed to parse retrieval policy from {}: {e}; using calibrated defaults",
                    contract_path.display()
                );
            }
        }
    }

    RetrievalPolicy::default_policy()
}
