//! Sensory organ readers — the organism perceiving external data stores.
//!
//! Distinct from organ/ (InferenceOrgan: Dog backend registry).
//! Each sense reads an external data store in read-only mode.

pub mod hermes_x;
pub mod rtk;

use crate::domain::organ::OrganPort;
use std::sync::Arc;

/// Build the sense registry at startup. Best-effort: missing organs are skipped.
pub fn build_sense_registry(project_root: &str) -> Vec<Arc<dyn OrganPort>> {
    let mut senses: Vec<Arc<dyn OrganPort>> = Vec::new();

    // RTK — token metabolism (SQLite)
    let rtk_db = dirs::data_local_dir()
        .unwrap_or_default()
        .join("rtk/history.db");
    if rtk_db.exists() {
        senses.push(Arc::new(rtk::RtkReader::new(
            rtk_db,
            project_root.to_string(),
        )));
    }

    // Hermes X — social perception (JSONL + filesystem)
    let hermes_x_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".cynic/organs/hermes/x");
    if hermes_x_dir.exists() {
        senses.push(Arc::new(hermes_x::HermesXReader::new(hermes_x_dir)));
    }

    senses
}
