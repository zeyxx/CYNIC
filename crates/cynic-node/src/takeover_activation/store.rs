//! Disposable persistence adapter for takeover admission and acknowledgement.

use std::fs;
use std::path::PathBuf;

#[cfg(test)]
use std::path::Path;

pub(super) trait TakeoverWatchStore {
    fn latest(&self) -> Result<Option<cynic_node_protocol::RuntimeTakeoverWatchRecord>, String>;
    fn acknowledged(&self, admission_proof_ref: &str) -> Result<bool, String>;
    fn acknowledge(&self, ack: &cynic_node_protocol::RuntimeTakeoverActivationAck)
    -> Result<(), String>;
}

pub(super) struct JsonlTakeoverWatchStore {
    watch_path: PathBuf,
    ack_path: PathBuf,
}

impl JsonlTakeoverWatchStore {
    pub(super) fn from_env() -> Result<Self, String> {
        let watch_path = std::env::var("CYNIC_TAKEOVER_WATCH_PATH")
            .map_err(|_| "takeover activation watch path is not configured".to_string())?;
        let ack_path = std::env::var("CYNIC_TAKEOVER_ACTIVATION_ACK_PATH")
            .map_err(|_| "takeover activation ack path is not configured".to_string())?;
        Ok(Self {
            watch_path: PathBuf::from(watch_path),
            ack_path: PathBuf::from(ack_path),
        })
    }
}

impl TakeoverWatchStore for JsonlTakeoverWatchStore {
    fn latest(&self) -> Result<Option<cynic_node_protocol::RuntimeTakeoverWatchRecord>, String> {
        let content = fs::read_to_string(&self.watch_path)
            .map_err(|error| format!("read takeover watch: {error}"))?;
        content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str(line)
                    .map_err(|error| format!("decode takeover watch: {error}"))
            })
            .last()
            .transpose()
    }

    fn acknowledged(&self, admission_proof_ref: &str) -> Result<bool, String> {
        if !self.ack_path.is_file() {
            return Ok(false);
        }
        let ack: cynic_node_protocol::RuntimeTakeoverActivationAck = serde_json::from_str(
            &fs::read_to_string(&self.ack_path)
                .map_err(|error| format!("read takeover ack: {error}"))?,
        )
        .map_err(|error| format!("decode takeover ack: {error}"))?;
        Ok(
            ack.schema == cynic_node_protocol::RUNTIME_TAKEOVER_ACTIVATION_ACK_SCHEMA
                && ack.admission_proof_ref == admission_proof_ref
                && ack.status == "activated_proved",
        )
    }

    fn acknowledge(
        &self,
        ack: &cynic_node_protocol::RuntimeTakeoverActivationAck,
    ) -> Result<(), String> {
        let parent = self
            .ack_path
            .parent()
            .ok_or_else(|| "takeover ack has no parent".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("create takeover ack directory: {error}"))?;
        let temporary = self.ack_path.with_extension("tmp");
        let encoded = serde_json::to_vec_pretty(ack)
            .map_err(|error| format!("encode takeover ack: {error}"))?;
        fs::write(&temporary, encoded).map_err(|error| format!("write takeover ack: {error}"))?;
        fs::rename(&temporary, &self.ack_path)
            .map_err(|error| format!("commit takeover ack: {error}"))
    }
}

#[cfg(test)]
pub(super) fn ack_matches(path: &Path, proof_ref: &str) -> Result<bool, String> {
    let store = JsonlTakeoverWatchStore {
        watch_path: PathBuf::new(),
        ack_path: path.to_path_buf(),
    };
    store.acknowledged(proof_ref)
}
