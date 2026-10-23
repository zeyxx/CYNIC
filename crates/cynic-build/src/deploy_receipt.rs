//! Journaled deploy receipts.
//!
//! WHY. A deploy that leaves no receipt leaves no rollback pointer and
//! no audit trail. The canary executor (couche B) writes one receipt per
//! machine step — applied or red — into an append-only JSONL journal.
//! Read-back is a pure parse: the journal is data, the CLI only renders
//! it. Nothing here touches the network or the machines.

use serde::{Deserialize, Serialize};

/// Journal schema identifier. Pinned as a const so writers and the
/// validator cannot drift apart.
pub const RECEIPT_SCHEMA: &str = "cynic.deploy_receipt.v1";

/// One journaled deploy step. The rollback pointer is the previous
/// artifact digest observed on that machine before this step ran: a red
/// step names exactly what to restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployReceipt {
    pub schema: String,
    pub commit: String,
    pub artifact_sha: String,
    pub machine: String,
    pub kind: DeployStepKind,
    pub gates: Vec<GateResult>,
    pub previous_artifact_ref: Option<String>,
    pub status: DeployStepStatus,
    pub at_unix_s: u64,
}

/// Deploy vs rollback step. Health checks are gates, not steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployStepKind {
    Deploy,
    Rollback,
}

/// Applied means every gate passed. Red means the machine was left
/// untouched (or was rolled back, which is its own receipt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployStepStatus {
    Applied,
    Red,
    RolledBack,
}

/// One per-step gate outcome (identity_match, bytes_match, smoke).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    pub name: String,
    pub pass: bool,
    pub detail: String,
}

/// Journal file name. The directory is the caller's choice (state root);
/// never hardcoded here.
pub const RECEIPT_JOURNAL_FILE: &str = "deploy-receipts.jsonl";

fn journal_path(state_dir: &str) -> std::path::PathBuf {
    std::path::Path::new(state_dir).join(RECEIPT_JOURNAL_FILE)
}

/// Validate a receipt before it touches the journal. Blank commit,
/// blank artifact, blank machine, or a wrong schema are refused — a
/// receipt that cannot identify what/where is worse than none.
pub fn validate_receipt(receipt: &DeployReceipt) -> Result<(), String> {
    if receipt.schema != RECEIPT_SCHEMA {
        return Err(format!("unknown receipt schema: {}", receipt.schema));
    }
    if receipt.commit.trim().is_empty() {
        return Err("receipt commit is blank".to_string());
    }
    if receipt.artifact_sha.trim().is_empty() {
        return Err("receipt artifact_sha is blank".to_string());
    }
    if receipt.machine.trim().is_empty() {
        return Err("receipt machine is blank".to_string());
    }
    Ok(())
}

/// Append one receipt. Creates the journal on first write.
pub fn append_receipt(state_dir: &str, receipt: &DeployReceipt) -> Result<(), String> {
    validate_receipt(receipt)?;
    let line = serde_json::to_string(receipt).map_err(|e| format!("receipt unserializable: {e}"))?;
    use std::io::Write as _;
    let path = journal_path(state_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("receipt dir unavailable: {e}"))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("receipt journal unavailable: {e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("receipt write failed: {e}"))?;
    Ok(())
}

/// Read every receipt, oldest first. Any malformed line fails the whole
/// read: a journal that silently drops steps is not a journal.
pub fn read_receipts(state_dir: &str) -> Result<Vec<DeployReceipt>, String> {
    let path = journal_path(state_dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("receipt journal unreadable: {e}")),
    };
    let mut receipts = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let receipt: DeployReceipt = serde_json::from_str(line)
            .map_err(|e| format!("receipt line {} unparsable: {e}", index + 1))?;
        validate_receipt(&receipt).map_err(|e| format!("receipt line {} invalid: {e}", index + 1))?;
        receipts.push(receipt);
    }
    Ok(receipts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(machine: &str) -> DeployReceipt {
        DeployReceipt {
            schema: RECEIPT_SCHEMA.to_string(),
            commit: "a".repeat(40),
            artifact_sha: "sha256:deadbeef".to_string(),
            machine: machine.to_string(),
            kind: DeployStepKind::Deploy,
            gates: vec![GateResult {
                name: "bytes_match".to_string(),
                pass: true,
                detail: "sha256 equals staged artifact".to_string(),
            }],
            previous_artifact_ref: Some("sha256:previous".to_string()),
            status: DeployStepStatus::Applied,
            at_unix_s: 1_791_234_567,
        }
    }

    fn dir() -> String {
        let dir = std::env::temp_dir().join(format!("cynic-receipts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        dir.display().to_string()
    }

    #[test]
    fn blank_commit_is_refused_before_write() {
        let mut bad = receipt("builder");
        bad.commit = "  ".to_string();
        let error = append_receipt(&dir(), &bad).expect_err("blank commit must not journal");
        assert!(error.contains("commit is blank"), "got: {error}");
    }

    #[test]
    fn round_trip_preserves_order_and_rollback_pointer() {
        let state = dir();
        append_receipt(&state, &receipt("builder")).unwrap();
        let mut red = receipt("cortex");
        red.status = DeployStepStatus::Red;
        red.previous_artifact_ref = None;
        append_receipt(&state, &red).unwrap();
        let back = read_receipts(&state).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].machine, "builder");
        assert_eq!(back[0].previous_artifact_ref.as_deref(), Some("sha256:previous"));
        assert_eq!(back[1].status, DeployStepStatus::Red);
        std::fs::remove_dir_all(&state).ok();
    }

    #[test]
    fn malformed_line_fails_the_whole_read() {
        let state = dir();
        append_receipt(&state, &receipt("builder")).unwrap();
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(journal_path(&state))
            .unwrap();
        writeln!(file, "not json at all").unwrap();
        let error = read_receipts(&state).expect_err("corrupt journal must fail, not skip");
        assert!(error.contains("unparsable"), "got: {error}");
        std::fs::remove_dir_all(&state).ok();
    }

    #[test]
    fn missing_journal_reads_as_empty_not_error() {
        let back = read_receipts(&dir()).unwrap();
        assert!(back.is_empty());
    }
}
