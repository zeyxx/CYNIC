use std::{fs, path::PathBuf};

use cynic_workload::{
    AgentWorkloadDelivery, AgentWorkloadDeliveryStatus, validate_agent_workload_delivery,
};

use crate::ports::agent_workload_completion_receipt_port::AgentWorkloadCompletionReceiptPort;

#[derive(Debug, Clone)]
pub struct LocalAgentWorkloadCompletionReceipt {
    root: PathBuf,
}

impl LocalAgentWorkloadCompletionReceipt {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self, digest: &str) -> Result<PathBuf, String> {
        let value = digest
            .strip_prefix("sha256:")
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| "workload_completion_receipt_digest_invalid".to_string())?;
        Ok(self.root.join(format!("{value}.json")))
    }

    fn read(&self, digest: &str) -> Result<Option<AgentWorkloadDelivery>, String> {
        let path = self.path(digest)?;
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("read completion receipt: {error}")),
        };
        let delivery = serde_json::from_slice::<AgentWorkloadDelivery>(&bytes)
            .map_err(|error| format!("decode completion receipt: {error}"))?;
        if delivery.status != AgentWorkloadDeliveryStatus::Terminal
            || delivery.terminal_record_digest.as_deref() != Some(digest)
            || !validate_agent_workload_delivery(&delivery).is_empty()
        {
            return Err("workload_completion_receipt_read_back_invalid".to_string());
        }
        Ok(Some(delivery))
    }
}

impl AgentWorkloadCompletionReceiptPort for LocalAgentWorkloadCompletionReceipt {
    fn contains(&self, terminal_record_digest: &str) -> Result<bool, String> {
        Ok(self.read(terminal_record_digest)?.is_some())
    }

    fn persist(&self, delivery: &AgentWorkloadDelivery) -> Result<(), String> {
        let digest = delivery
            .terminal_record_digest
            .as_deref()
            .ok_or_else(|| "workload_completion_receipt_terminal_digest_missing".to_string())?;
        if delivery.status != AgentWorkloadDeliveryStatus::Terminal
            || !validate_agent_workload_delivery(delivery).is_empty()
        {
            return Err("workload_completion_receipt_delivery_invalid".to_string());
        }
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("create completion receipt root: {error}"))?;
        let path = self.path(digest)?;
        let bytes = serde_json::to_vec_pretty(delivery)
            .map_err(|error| format!("encode completion receipt: {error}"))?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(&bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|error| format!("persist completion receipt: {error}"))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(format!("create completion receipt: {error}")),
        }
        match self.read(digest)? {
            Some(read_back) if read_back == *delivery => Ok(()),
            _ => Err("workload_completion_receipt_read_back_mismatch".to_string()),
        }
    }
}
