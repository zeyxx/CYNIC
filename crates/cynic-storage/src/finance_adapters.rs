//! Finance adapters for cynic-storage.
//!
//! Implements cynic-finance StorageMeter port using WAL and disk observation.

use cynic_finance::ports::{MeterError, StorageMeter};
use std::fs;
use std::path::Path;

/// Storage meter — measures disk usage and WAL size for a given agent.
pub struct StorageFinanceMeter {
    root_path: String,
}

impl StorageFinanceMeter {
    pub fn new(root_path: impl Into<String>) -> Self {
        Self {
            root_path: root_path.into(),
        }
    }
}

impl StorageMeter for StorageFinanceMeter {
    fn measure_bytes(&self, agent_id: &str) -> Result<u64, MeterError> {
        let agent_path = Path::new(&self.root_path).join(agent_id);
        if !agent_path.exists() {
            return Err(MeterError::Unknown(format!(
                "agent storage path '{}' does not exist",
                agent_path.display()
            )));
        }

        let mut total_bytes = 0u64;
        for entry in fs::read_dir(&agent_path)
            .map_err(|e| MeterError::Unavailable(format!("read {}: {}", agent_path.display(), e)))?
        {
            let entry = entry.map_err(|e| MeterError::Unavailable(format!("read entry: {}", e)))?;
            let metadata = entry
                .metadata()
                .map_err(|e| MeterError::Unavailable(format!("read metadata: {}", e)))?;
            if metadata.is_file() {
                total_bytes += metadata.len();
            }
        }

        Ok(total_bytes)
    }

    fn measure_iops(&self, _agent_id: &str) -> Result<u64, MeterError> {
        // IOPS requires time-based sampling; return unknown for now
        Err(MeterError::Unknown(
            "IOPS measurement requires time-based sampling".to_string(),
        ))
    }
}
