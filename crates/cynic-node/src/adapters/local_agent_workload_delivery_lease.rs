use std::{fs, io::Write, path::PathBuf};

use cynic_protocol::{
    AgentWorkloadDeliveryLease, digest_json, validate_agent_workload_delivery_lease,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ports::agent_workload_delivery_lease_port::AgentWorkloadDeliveryLeasePort;

const LOCAL_LEASE_SCHEMA: &str = "cynic.local_agent_workload_delivery_lease.v1";

#[derive(Debug, Clone)]
pub struct LocalAgentWorkloadDeliveryLeaseStore {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LocalLeaseRecord {
    schema: String,
    workload_id: String,
    lease: AgentWorkloadDeliveryLease,
    record_digest: String,
}

impl LocalAgentWorkloadDeliveryLeaseStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self, workload_id: &str) -> Result<PathBuf, String> {
        if workload_id.trim().is_empty() {
            return Err("workload_delivery_lease_workload_id_missing".to_string());
        }
        let key = hex::encode(Sha256::digest(workload_id.as_bytes()));
        Ok(self.root.join(format!("{key}.json")))
    }

    fn record(
        workload_id: &str,
        lease: &AgentWorkloadDeliveryLease,
    ) -> Result<LocalLeaseRecord, String> {
        if !validate_agent_workload_delivery_lease(lease).is_empty() {
            return Err("workload_delivery_lease_invalid".to_string());
        }
        let record_digest = digest_json(&serde_json::json!({
            "schema": LOCAL_LEASE_SCHEMA,
            "workload_id": workload_id,
            "lease": lease,
        }))?;
        Ok(LocalLeaseRecord {
            schema: LOCAL_LEASE_SCHEMA.to_string(),
            workload_id: workload_id.to_string(),
            lease: lease.clone(),
            record_digest,
        })
    }

    fn read_record(&self, workload_id: &str) -> Result<Option<LocalLeaseRecord>, String> {
        let path = self.path(workload_id)?;
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("read workload delivery lease: {error}")),
        };
        let record: LocalLeaseRecord = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode workload delivery lease: {error}"))?;
        if record.schema != LOCAL_LEASE_SCHEMA
            || record.workload_id != workload_id
            || record != Self::record(workload_id, &record.lease)?
        {
            return Err("workload_delivery_lease_read_back_invalid".to_string());
        }
        Ok(Some(record))
    }
}

impl AgentWorkloadDeliveryLeasePort for LocalAgentWorkloadDeliveryLeaseStore {
    fn read(&self, workload_id: &str) -> Result<Option<AgentWorkloadDeliveryLease>, String> {
        Ok(self.read_record(workload_id)?.map(|record| record.lease))
    }

    fn persist(&self, workload_id: &str, lease: &AgentWorkloadDeliveryLease) -> Result<(), String> {
        let record = Self::record(workload_id, lease)?;
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("create workload delivery lease root: {error}"))?;
        let path = self.path(workload_id)?;
        let temporary = path.with_extension(format!(
            "json.tmp.{}.{}",
            std::process::id(),
            lease.fencing_token
        ));
        let bytes = serde_json::to_vec_pretty(&record)
            .map_err(|error| format!("encode workload delivery lease: {error}"))?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| format!("create workload delivery lease temporary: {error}"))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("persist workload delivery lease temporary: {error}"))?;
        fs::rename(&temporary, &path)
            .map_err(|error| format!("publish workload delivery lease: {error}"))?;
        match self.read_record(workload_id)? {
            Some(read_back) if read_back == record => Ok(()),
            _ => Err("workload_delivery_lease_read_back_mismatch".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::AGENT_WORKLOAD_DELIVERY_LEASE_SCHEMA;

    #[test]
    fn replaces_renewed_lease_and_reads_back_exact_fence() {
        let root = tempfile::tempdir().unwrap();
        let store = LocalAgentWorkloadDeliveryLeaseStore::new(root.path());
        let mut lease = AgentWorkloadDeliveryLease {
            schema: AGENT_WORKLOAD_DELIVERY_LEASE_SCHEMA.to_string(),
            lease_id: format!("sha256:{}", "1".repeat(64)),
            fencing_token: 1,
            consumer_instance_id: "incarnation-a".to_string(),
            accepted_at_ms: 1,
            expires_at_ms: 10,
            record_digest: format!("sha256:{}", "2".repeat(64)),
        };
        store.persist("workload-a", &lease).unwrap();
        lease.expires_at_ms = 20;
        store.persist("workload-a", &lease).unwrap();
        assert_eq!(store.read("workload-a").unwrap(), Some(lease));
    }
}
