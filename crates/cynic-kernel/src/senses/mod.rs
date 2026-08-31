//! Sensory readers — the organism perceiving external data stores.
//!
//! Distinct from organ/ (InferenceOrgan: Dog backend registry).
//! Each sense reads an external data store in read-only mode.

pub mod rtk;

use crate::domain::organ::OrganPort;
use std::sync::Arc;

/// Build the sense registry at startup. Best-effort: missing senses are skipped.
pub fn build_sense_registry() -> Vec<Arc<dyn OrganPort>> {
    let mut senses: Vec<Arc<dyn OrganPort>> = Vec::new();

    // RTK — token metabolism (SQLite)
    let rtk_db = dirs::data_local_dir()
        .unwrap_or_default()
        .join("rtk/history.db");
    if rtk_db.exists() {
        senses.push(Arc::new(rtk::RtkReader::new(rtk_db, None)));
    }

    senses
}
